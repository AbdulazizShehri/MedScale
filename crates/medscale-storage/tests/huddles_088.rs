//! Spec 088 AudioFlow Advanced storage + migration integration tests.
//!
//! Synthetic rows only. Additive v16 -> v17 migration, compare-and-set
//! changes with receipts, atomic media deletion (audio row, bytes and
//! transcripts), cross-row invariants and backup/restore (including
//! tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::audio::{AudioSource, AudioSourceKind, CaptureHealth, PcmFormat};
use medscale_contracts::huddles::{
    ConsentSet, HUDDLE_SCHEMA_VERSION, Huddle, HuddleAction, HuddleMedia, HuddleParticipant,
    HuddleParticipantKind, HuddleReceipt, HuddleState, MediaOrigin, MediaState,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, HUDDLE_TABLES, HuddleChange, MetaError, SqliteMetaStore,
    SyntheticVault, backup_vault, restore_vault,
};

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-088s-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn open_meta(root: &Path) -> SqliteMetaStore {
    SqliteMetaStore::open_at(&root.join("meta.sqlite3")).unwrap()
}

fn raw(root: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(root.join("meta.sqlite3")).unwrap()
}

fn h(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: HUDDLE_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

const WAV: &[u8] = b"synthetic wav bytes for storage tests";

fn audio() -> AudioSource {
    AudioSource {
        header: h("audio-source-1"),
        project_id: id("proj-1"),
        kind: AudioSourceKind::ImportedFile,
        label: "huddle".to_owned(),
        format: PcmFormat::mono_16k(),
        duration_ms: 2_000,
        byte_length: WAV.len() as u64,
        content_digest: DigestSha256::of(WAV),
        health: CaptureHealth::Ok,
        capture_session_id: None,
    }
}

fn huddle(revision: u64, state: HuddleState) -> Huddle {
    Huddle {
        header: h("huddle-1"),
        project_id: id("proj-1"),
        title: "Morning huddle".to_owned(),
        created_by: id("holder-1"),
        state,
        retention_days: 30,
        revision,
    }
}

fn participant(revision: u64, join: bool) -> HuddleParticipant {
    HuddleParticipant {
        header: h("huddle-participant-1"),
        huddle_id: id("huddle-1"),
        display_name: "Ana".to_owned(),
        kind: HuddleParticipantKind::Human,
        consents: ConsentSet {
            join,
            ..ConsentSet::default()
        },
        revision,
    }
}

fn media() -> HuddleMedia {
    HuddleMedia {
        header: h("huddle-media-1"),
        huddle_id: id("huddle-1"),
        source_id: id("audio-source-1"),
        source_digest: DigestSha256::of(WAV),
        origin: MediaOrigin::HumanRecording,
        synthetic: false,
        attached_day: 20_000,
        state: MediaState::Present,
    }
}

fn receipt(n: u32, action: HuddleAction) -> HuddleReceipt {
    HuddleReceipt {
        header: h(&format!("huddle-receipt-{n}")),
        huddle_id: id("huddle-1"),
        project_id: id("proj-1"),
        action,
        targets: Vec::new(),
        refusal: None,
        consent: None,
        deleted_digests: Vec::new(),
    }
}

fn build_state(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "p".to_owned(), None).unwrap())
        .unwrap();
    meta.insert_imported_audio_source(&audio(), WAV).unwrap();
    let hd = huddle(1, HuddleState::Open);
    meta.commit_huddle_change(&HuddleChange {
        huddle: Some((&hd, None)),
        receipt: Some(&receipt(1, HuddleAction::Create)),
        ..HuddleChange::default()
    })
    .unwrap();
    let p = participant(1, false);
    meta.commit_huddle_change(&HuddleChange {
        participant: Some((&p, None)),
        receipt: Some(&receipt(2, HuddleAction::AddParticipant)),
        ..HuddleChange::default()
    })
    .unwrap();
    let m = media();
    meta.commit_huddle_change(&HuddleChange {
        new_media: Some(&m),
        receipt: Some(&receipt(3, HuddleAction::AttachMedia)),
        ..HuddleChange::default()
    })
    .unwrap();
}

#[test]
fn migration_to_v17_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    for table in HUDDLE_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 17", [])
        .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    assert!(meta.list_huddles().unwrap().is_empty());
    assert!(meta.get_audio_source(&id("audio-source-1")).is_ok());
}

#[test]
fn changes_are_compare_and_set_and_deletion_is_atomic() {
    let root = temp_root("cas");
    let meta = open_meta(&root);
    build_state(&meta);
    meta.verify_huddle_consistency().unwrap();

    // Stale participant revision: conflict, nothing written.
    let stale = participant(2, true);
    assert!(matches!(
        meta.commit_huddle_change(&HuddleChange {
            participant: Some((&stale, Some(5))),
            receipt: Some(&receipt(4, HuddleAction::Consent)),
            ..HuddleChange::default()
        }),
        Err(MetaError::UnsupportedSchema(_) | MetaError::Conflict(_))
    ));
    assert_eq!(meta.list_huddle_receipts(None).unwrap().len(), 3);
    // Invalid consent (record without join) is refused.
    let mut bad = participant(2, false);
    bad.consents.record = true;
    assert!(
        meta.commit_huddle_change(&HuddleChange {
            participant: Some((&bad, Some(1))),
            receipt: Some(&receipt(5, HuddleAction::Consent)),
            ..HuddleChange::default()
        })
        .is_err()
    );

    // Deletion removes the audio row and marks the media deleted, with
    // its receipt, in one transaction.
    let m = media();
    let mut del = receipt(6, HuddleAction::DeleteMedia);
    del.deleted_digests = vec![DigestSha256::of(WAV)];
    meta.commit_huddle_change(&HuddleChange {
        delete_media: vec![&m],
        receipt: Some(&del),
        ..HuddleChange::default()
    })
    .unwrap();
    assert!(matches!(
        meta.get_audio_source(&id("audio-source-1")),
        Err(MetaError::NotFound)
    ));
    assert_eq!(
        meta.list_huddle_media(None).unwrap()[0].state,
        MediaState::Deleted
    );
    meta.verify_huddle_consistency().unwrap();
    // A second deletion fails and writes no receipt.
    assert!(
        meta.commit_huddle_change(&HuddleChange {
            delete_media: vec![&m],
            receipt: Some(&receipt(7, HuddleAction::DeleteMedia)),
            ..HuddleChange::default()
        })
        .is_err()
    );
    assert_eq!(meta.list_huddle_receipts(None).unwrap().len(), 4);
}

#[test]
fn unlabeled_synthetic_media_is_refused() {
    let root = temp_root("synthetic");
    let meta = open_meta(&root);
    build_state(&meta);
    let mut m = media();
    m.header = h("huddle-media-2");
    m.origin = MediaOrigin::SyntheticAgent {
        participant_id: id("huddle-participant-1"),
    };
    assert!(
        meta.commit_huddle_change(&HuddleChange {
            new_media: Some(&m),
            receipt: Some(&receipt(8, HuddleAction::AttachMedia)),
            ..HuddleChange::default()
        })
        .is_err()
    );
}

fn tamper_backup(dest: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    let snapshot_path = dest.join("metadata.snapshot");
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(&snapshot_path).unwrap()).unwrap();
    edit(&mut snapshot);
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    fs::write(&snapshot_path, &bytes).unwrap();
    let manifest_path = dest.join("manifest.json");
    let mut manifest: BackupManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.metadata_snapshot_digest = DigestSha256::of(&bytes);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}

fn rows<'a>(snapshot: &'a mut serde_json::Value, key: &str) -> &'a mut Vec<serde_json::Value> {
    snapshot
        .get_mut(key)
        .and_then(|v| v.as_array_mut())
        .unwrap_or_else(|| panic!("{key} present"))
}

fn backed_up(root: &Path) -> PathBuf {
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    build_state(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    dest
}

#[test]
fn backup_restore_round_trips_huddle_rows() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let a = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let b = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        a.meta.list_huddles().unwrap(),
        b.meta.list_huddles().unwrap()
    );
    assert_eq!(
        a.meta.list_huddle_media(None).unwrap(),
        b.meta.list_huddle_media(None).unwrap()
    );
    assert_eq!(
        a.meta.list_huddle_receipts(None).unwrap(),
        b.meta.list_huddle_receipts(None).unwrap()
    );
    b.meta.verify_huddle_consistency().unwrap();
}

#[test]
fn tampered_huddle_backups_are_refused() {
    type Edit = fn(&mut serde_json::Value);
    let edits: Vec<(&str, Edit)> = vec![
        ("huddles family missing", |s| {
            s.as_object_mut().unwrap().remove("hud_huddles");
        }),
        ("media claims deleted but audio remains", |s| {
            rows(s, "hud_media")[0]["state"] = serde_json::json!("deleted");
        }),
        ("synthetic label stripped", |s| {
            rows(s, "hud_media")[0]["origin"] = serde_json::json!({"kind":"synthetic_agent","participant_id":"huddle-participant-1"});
        }),
        ("consent without join", |s| {
            rows(s, "hud_participants")[0]["consents"]["export"] = serde_json::json!(true);
        }),
        ("media digest swapped", |s| {
            rows(s, "hud_media")[0]["source_digest"] =
                serde_json::to_value(DigestSha256::of(b"other")).unwrap();
        }),
        ("participant of a missing huddle", |s| {
            rows(s, "hud_participants")[0]["huddle_id"] = serde_json::json!("huddle-9");
        }),
    ];
    for (name, edit) in edits {
        let root = temp_root(&format!("tamper-{}", name.replace(' ', "-")));
        let dest = backed_up(&root);
        tamper_backup(&dest, edit);
        assert!(
            restore_vault(&dest, &root.join("restored")).is_err(),
            "{name} must be refused"
        );
    }
}

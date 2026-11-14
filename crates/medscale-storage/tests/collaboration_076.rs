//! Spec 076 storage + migration integration tests (076-B).
//!
//! Synthetic data only. Every claim binds to exact behavior below; nothing is
//! inferred from a green compile. Covers `migration.md` sections 7 and 8
//! (fixture and qualification sequence), `security.md` T10 (activity
//! hash-chain tamper detection) and T12 (half-committed state after crash),
//! using the same raw-storage idiom `project_graph_074.rs`/
//! `data_sources_075.rs` already established for this workstation (no local
//! linker; CI is the authoritative compile/test signal).

use std::fs;

use medscale_contracts::collaboration::{
    COLLAB_SCHEMA_VERSION, CollabEventKind, MembershipRole, ParticipantIdentity, ParticipantKind,
    Room, RoomMembership, Task,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::{Project, ProjectStatus};
use medscale_storage::{MetaError, SqliteMetaStore, SyntheticVault, backup_vault, restore_vault};

fn temp_root(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-076-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn open_meta(root: &std::path::Path) -> SqliteMetaStore {
    SqliteMetaStore::open_at(&root.join("meta.sqlite3")).unwrap()
}

fn scope() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-1")
}

fn header(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: COLLAB_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: scope(),
    }
}

fn project(id: &str) -> Project {
    Project::new(header(id), format!("project-{id}"), None).unwrap()
}

fn participant(id: &str, holder: &str) -> ParticipantIdentity {
    ParticipantIdentity::new(
        header(id),
        OpaqueId::new(holder),
        ParticipantKind::Human,
        format!("operator-{id}"),
    )
    .unwrap()
}

fn room(id: &str, project_id: &str) -> Room {
    Room::new(
        header(id),
        OpaqueId::new(project_id),
        None,
        format!("room-{id}"),
    )
    .unwrap()
}

fn membership(id: &str, room_id: &str, participant_id: &str) -> RoomMembership {
    RoomMembership::new(
        header(id),
        OpaqueId::new(room_id),
        OpaqueId::new(participant_id),
        MembershipRole::Owner,
    )
}

fn task(id: &str, room_id: &str, title: &str) -> Task {
    Task::new(
        header(id),
        OpaqueId::new(room_id),
        None,
        title.to_owned(),
        None,
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// migration.md section 7-8: fixture + qualification sequence
// ---------------------------------------------------------------------------

#[test]
fn migration_v4_to_v5_preserves_pre_076_rows_and_adds_collab_tables() {
    let root = temp_root("migrate-v5");
    let meta = open_meta(&root);

    // Pre-076 (074-era) canonical object, inserted after the v5 migration
    // already ran on open (matches how a real vault behaves: migration is a
    // one-time schema step, not a per-object gate).
    meta.insert_project(&project("proj-1")).unwrap();

    // 076 rows layered on top, additive per migration.md section 1.
    meta.insert_participant(&participant("p-1", "holder-1"), None)
        .unwrap();
    let activity = meta
        .insert_room_with_activity(
            &room("room-1", "proj-1"),
            header("act-1"),
            &OpaqueId::new("p-1"),
        )
        .unwrap();
    assert_eq!(activity.seq, 1);
    assert_eq!(activity.event_kind, CollabEventKind::RoomCreated);

    let journal = meta.migration_journal().unwrap();
    // Forward-fixed for Spec 077 (v5 -> v6): a fresh vault always migrates
    // to the live top version, not the version this spec introduced. This
    // is the same forward-fix Spec 076 itself required for Spec 075's
    // pinned finished_version == 4 assertions when v4->v5 landed.
    assert_eq!(
        journal.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );

    // Reopen must be safe (idempotent migration) and preserve every row
    // across both the pre-076 and the 076 families.
    drop(meta);
    let meta = open_meta(&root);
    let journal_again = meta.migration_journal().unwrap();
    assert_eq!(
        journal_again.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION,
        "repeat open must be a no-op, not a re-migration"
    );

    let read_project = meta.get_project(&OpaqueId::new("proj-1")).unwrap();
    assert_eq!(
        (read_project.revision, read_project.status),
        (1, ProjectStatus::Active)
    );

    let read_room = meta.get_room(&OpaqueId::new("room-1")).unwrap();
    assert_eq!(read_room.revision, 1);
    assert_eq!(read_room.project_id.as_str(), "proj-1");

    let read_participant = meta.get_participant(&OpaqueId::new("p-1")).unwrap();
    assert_eq!(read_participant.holder_id.as_str(), "holder-1");

    meta.verify_activity_chain(&OpaqueId::new("room-1"))
        .unwrap();
}

// ---------------------------------------------------------------------------
// migration.md section 11 + security.md T10: backup/restore + hash chain
// ---------------------------------------------------------------------------

#[test]
fn backup_restore_roundtrips_collab_rows_and_verifies_activity_chain() {
    let root = temp_root("collab-backup");
    let vault_root = root.join("vault");
    let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();

    vault
        .meta
        .insert_participant(&participant("p-1", "holder-1"), None)
        .unwrap();
    vault
        .meta
        .insert_room_with_activity(
            &room("room-1", "proj-1"),
            header("act-1"),
            &OpaqueId::new("p-1"),
        )
        .unwrap();
    vault
        .meta
        .insert_membership_with_activity(
            &membership("m-1", "room-1", "p-1"),
            header("act-2"),
            &OpaqueId::new("p-1"),
        )
        .unwrap();
    vault
        .meta
        .insert_task_with_activity(
            &task("t-1", "room-1", "collect baseline labs"),
            header("act-3"),
            &OpaqueId::new("p-1"),
        )
        .unwrap();

    let dest = root.join("backup");
    let manifest_out = backup_vault(&vault, &dest).unwrap();
    // Forward-fixed for Spec 077 (v5 -> v6), same discipline as above.
    assert_eq!(
        manifest_out.schema_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );

    let restore_root = root.join("restored");
    let _ = restore_vault(&dest, &restore_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restore_root).unwrap();

    let back_room = restored.meta.get_room(&OpaqueId::new("room-1")).unwrap();
    assert_eq!(back_room.name, "room-room-1");
    let back_membership = restored
        .meta
        .list_room_memberships(&OpaqueId::new("room-1"))
        .unwrap();
    assert_eq!(back_membership.len(), 1);
    let back_task = restored.meta.get_task(&OpaqueId::new("t-1")).unwrap();
    assert_eq!(back_task.title, "collect baseline labs");

    // The hash chain must still verify bit-for-bit after restore: restore
    // preserves `checkpoint_digest` verbatim (never recomputed).
    restored
        .meta
        .verify_activity_chain(&OpaqueId::new("room-1"))
        .unwrap();
    let records = restored
        .meta
        .list_activity_records(&OpaqueId::new("room-1"), 100, 0)
        .unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].seq, 1);
    assert_eq!(records[2].seq, 3);
}

/// Exact-range review finding (fixed before merge, not hidden): `restore_v5`
/// replayed every `collab_activity_records` row but never re-verified the
/// chain afterward, so a hand-edited backup -- one whose editor also
/// recomputes the outer `metadata_snapshot_digest` to match their edit,
/// exactly `security.md` T10's threat, not a random bit-flip -- would
/// restore silently instead of failing closed, contradicting
/// `migration.md` section 11. Fixed by verifying every restored room's
/// chain inside `restore_v5`.
#[test]
fn restore_rejects_hand_edited_backup_with_broken_activity_chain() {
    let root = temp_root("tamper-backup");
    let vault_root = root.join("vault");
    let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();

    vault
        .meta
        .insert_participant(&participant("p-1", "holder-1"), None)
        .unwrap();
    vault
        .meta
        .insert_room_with_activity(
            &room("room-1", "proj-1"),
            header("act-1"),
            &OpaqueId::new("p-1"),
        )
        .unwrap();
    vault
        .meta
        .insert_membership_with_activity(
            &membership("m-1", "room-1", "p-1"),
            header("act-2"),
            &OpaqueId::new("p-1"),
        )
        .unwrap();

    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();

    // Hand-edit the seq-1 activity record's target inside the raw snapshot
    // file, then recompute the outer manifest digest to match the edit --
    // the outer digest is a plain content hash, not a signature, so an
    // editor with file access can trivially keep it self-consistent. Only
    // the activity hash chain can catch this specific tamper.
    let snapshot_path = dest.join("metadata.snapshot");
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(&snapshot_path).unwrap()).unwrap();
    let records = snapshot
        .get_mut("collab_activity_records")
        .and_then(|v| v.as_array_mut())
        .expect("activity records present in snapshot");
    assert!(!records.is_empty());
    records[0]["target_object_id"] = serde_json::Value::String("tampered-target".to_owned());
    let new_snapshot_bytes = serde_json::to_vec(&snapshot).unwrap();
    fs::write(&snapshot_path, &new_snapshot_bytes).unwrap();

    let manifest_path = dest.join("manifest.json");
    let mut manifest: BackupManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.metadata_snapshot_digest = DigestSha256::of(&new_snapshot_bytes);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let restore_root = root.join("restored");
    let err = restore_vault(&dest, &restore_root).unwrap_err();
    assert!(
        err.contains("chain") || err.contains("checkpoint") || err.contains("mismatch"),
        "hand-edited backup with a broken activity chain must fail closed \
         at restore time, not merely be detectable by a separate manual \
         verify_activity_chain call afterward; got: {err}"
    );
}

// ---------------------------------------------------------------------------
// security.md T10: audit tampering / activity-log rewrite
// ---------------------------------------------------------------------------

#[test]
fn tampered_activity_row_fails_chain_verification() {
    let root = temp_root("tamper");
    let meta = open_meta(&root);
    meta.insert_participant(&participant("p-1", "holder-1"), None)
        .unwrap();
    meta.insert_room_with_activity(
        &room("room-1", "proj-1"),
        header("act-1"),
        &OpaqueId::new("p-1"),
    )
    .unwrap();
    meta.insert_membership_with_activity(
        &membership("m-1", "room-1", "p-1"),
        header("act-2"),
        &OpaqueId::new("p-1"),
    )
    .unwrap();

    // Chain verifies cleanly before tampering.
    meta.verify_activity_chain(&OpaqueId::new("room-1"))
        .unwrap();

    drop(meta);
    {
        // Direct DB tamper, not reachable through the public API: rewrite
        // the seq-1 row's target so the stored digest no longer matches the
        // recomputed chain (simulates out-of-band edit / hand-edited
        // restore, security.md T10).
        let conn = rusqlite::Connection::open(root.join("meta.sqlite3")).unwrap();
        conn.execute(
            "UPDATE collab_activity_records SET target_object_id = 'tampered-target' WHERE room_id = 'room-1' AND seq = 1",
            [],
        )
        .unwrap();
    }
    let meta = open_meta(&root);
    let err = meta
        .verify_activity_chain(&OpaqueId::new("room-1"))
        .unwrap_err();
    assert!(
        matches!(err, MetaError::CorruptObjectBody(_)),
        "tampered activity row must fail closed, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// security.md T12: half-committed state after crash
// ---------------------------------------------------------------------------

#[test]
fn room_and_activity_commit_atomically_or_neither_does() {
    let root = temp_root("atomic");
    let meta = open_meta(&root);
    meta.insert_participant(&participant("p-1", "holder-1"), None)
        .unwrap();

    // Force the activity append inside `insert_room_with_activity` to fail
    // by pre-occupying the seq slot it will try to claim: a fresh room key's
    // first activity always lands at seq = 1 (`next_collab_seq`), so a raw
    // pre-inserted row at (room_id, seq = 1) collides with the unique
    // `idx_collab_activity_room_seq` index the real insert then hits.
    {
        let conn = rusqlite::Connection::open(root.join("meta.sqlite3")).unwrap();
        let zero_digest_hex = "0".repeat(64);
        conn.execute(
            &format!(
                "INSERT INTO collab_activity_records(activity_id, room_id, realm_id, authority_scope_id, actor_participant_id, event_kind, target_object_id, checkpoint_digest_hex, seq, schema_version)
                 VALUES ('act-precollision', 'room-1', 'realm-1', 'scope-1', 'p-1', 'room_created', 'room-1', '{zero_digest_hex}', 1, 5)"
            ),
            [],
        )
        .unwrap();
    }

    let err = meta
        .insert_room_with_activity(
            &room("room-1", "proj-1"),
            header("act-real"),
            &OpaqueId::new("p-1"),
        )
        .unwrap_err();
    let _ = err; // exact error kind (Sqlite constraint) is not the contract; non-commit is.

    // The room row must NOT be visible: the primary-row insert executed
    // inside the same transaction as the failed activity append, and the
    // whole transaction must have rolled back together (migration.md
    // section 5, security.md T12) -- never a room without its activity
    // entry, and never the reverse.
    assert!(matches!(
        meta.get_room(&OpaqueId::new("room-1")),
        Err(MetaError::NotFound)
    ));
}

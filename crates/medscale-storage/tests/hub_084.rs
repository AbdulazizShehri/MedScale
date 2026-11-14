//! Spec 084 MedScale Hub foundation storage + migration integration tests.
//!
//! Synthetic data only. Additive v12 -> v13 migration, hash-chained Hub
//! events with sequence claims, client links, outbox and mirror, cross-row
//! invariants and backup/restore (including tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::hub::{
    DeviceIdentity, DeviceStatus, HUB_PROTOCOL_VERSION, HUB_SCHEMA_VERSION, HubEvent, HubEventKind,
    HubIdentity, HubInvitation, HubLink, HubOutboxEntry, InvitationStatus, OutboxState,
    SyncEnvelope, SyncEnvelopeBody, SyncIntent, SyncOutcome, SyncRefusal,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId, VaultId,
};
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, MetaError, SeqClaim, SqliteMetaStore, SyntheticVault,
    backup_vault, restore_vault,
};

const HUB_TABLES: [&str; 9] = [
    "hub_identity",
    "hub_invitations",
    "hub_devices",
    "hub_nonces",
    "hub_events",
    "hub_links",
    "hub_link_secrets",
    "hub_outbox",
    "hub_mirror",
];

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-084s-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn db_path(root: &Path) -> PathBuf {
    root.join("meta.sqlite3")
}

fn open_meta(root: &Path) -> SqliteMetaStore {
    SqliteMetaStore::open_at(&db_path(root)).unwrap()
}

fn raw(root: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(db_path(root)).unwrap()
}

fn h(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: HUB_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn hub_identity() -> HubIdentity {
    HubIdentity {
        header: h("hub-1"),
        vault_id: VaultId::new("vault-1"),
        protocol_version: HUB_PROTOCOL_VERSION,
    }
}

fn invitation(n: u32) -> HubInvitation {
    HubInvitation {
        header: h(&format!("invitation-{n}")),
        project_id: id("proj-1"),
        display_name: format!("Laptop {n}"),
        token_digest: DigestSha256::of(format!("token-{n}").as_bytes()),
        status: InvitationStatus::Open,
        device_id: None,
    }
}

fn device(n: u32) -> DeviceIdentity {
    DeviceIdentity {
        header: h(&format!("device-{n}")),
        hub_id: id("hub-1"),
        project_id: id("proj-1"),
        participant_id: id(&format!("participant-{n}")),
        holder_id: id(&format!("hub-device-{n}")),
        display_name: format!("Laptop {n}"),
        public_key_hex: format!("{n:x}").repeat(64)[..64].to_owned(),
        invitation_id: id(&format!("invitation-{n}")),
        status: DeviceStatus::Active,
    }
}

fn envelope(device: u32, seq: u64, body: &str) -> SyncEnvelope {
    SyncEnvelope {
        body: SyncEnvelopeBody {
            hub_id: id("hub-1"),
            device_id: id(&format!("device-{device}")),
            seq,
            intent: SyncIntent::MessagePost {
                thread_id: id("thread-1"),
                body: body.to_owned(),
            },
        },
        signature_hex: "ab".repeat(64),
    }
}

fn claim(e: &SyncEnvelope) -> SeqClaim {
    SeqClaim {
        device_id: e.body.device_id.clone(),
        seq: e.body.seq,
        envelope_digest: e.digest().unwrap(),
    }
}

fn applied(n: u32) -> SyncOutcome {
    SyncOutcome::Applied {
        object_id: id(&format!("message-{n}")),
        revision: 1,
    }
}

fn submission(meta: &SqliteMetaStore, e: SyncEnvelope, outcome: SyncOutcome) -> HubEvent {
    let c = claim(&e);
    let claims = !matches!(
        outcome,
        SyncOutcome::Refused {
            reason: SyncRefusal::BadSignature
        }
    );
    meta.append_hub_event(
        &id("hub-1"),
        &id("proj-1"),
        HubEventKind::Submission {
            envelope: e,
            outcome,
        },
        claims.then_some(&c),
    )
    .unwrap()
}

fn link() -> HubLink {
    HubLink {
        header: h("link-1"),
        endpoint: "medscale-hub".to_owned(),
        hub_id: id("hub-1"),
        hub_vault_id: VaultId::new("vault-1"),
        hub_realm_id: RealmId::new("realm-1"),
        hub_scope_id: AuthorityScopeId::new("scope-1"),
        device_id: id("device-1"),
        participant_id: id("participant-1"),
        project_id: id("proj-1"),
        public_key_hex: "1".repeat(64),
        last_seq: 0,
        cursor: 0,
        head_digest: None,
        revoked: false,
    }
}

/// A Hub with two enrolled devices (one revoked), an open invitation, and
/// submissions; plus a client link whose mirror holds the same events.
fn build_state(meta: &SqliteMetaStore) -> Vec<HubEvent> {
    meta.insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
    meta.insert_hub_identity(&hub_identity()).unwrap();
    for n in 1..=3 {
        meta.insert_invitation(&invitation(n)).unwrap();
    }
    meta.enroll_device(&id("invitation-1"), &device(1)).unwrap();
    meta.enroll_device(&id("invitation-2"), &device(2)).unwrap();
    submission(meta, envelope(1, 1, "hello"), applied(1));
    submission(
        meta,
        envelope(1, 2, "forged"),
        SyncOutcome::Refused {
            reason: SyncRefusal::BadSignature,
        },
    );
    submission(meta, envelope(1, 2, "second"), applied(2));
    meta.revoke_device(&id("device-2")).unwrap();
    // Client side (this vault also syncs as device-1).
    meta.insert_pending_hub_secret(&id("link-1"), &"7".repeat(64))
        .unwrap();
    meta.insert_hub_link(&link()).unwrap();
    for (seq, text) in [(1, "hello"), (2, "second"), (3, "queued")] {
        meta.queue_hub_outbox(&HubOutboxEntry {
            link_id: id("link-1"),
            envelope: envelope(1, seq, text),
            state: OutboxState::Pending,
            outcome: None,
        })
        .unwrap();
    }
    meta.record_hub_outcome(&id("link-1"), 1, &applied(1))
        .unwrap();
    meta.record_hub_outcome(&id("link-1"), 2, &applied(2))
        .unwrap();
    let events = meta.list_hub_events(&id("proj-1"), 0, 100).unwrap();
    meta.append_hub_mirror(&id("link-1"), &events).unwrap();
    events
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

fn table_exists(root: &Path, table: &str) -> bool {
    raw(root)
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
        == 1
}

#[test]
fn migration_v12_to_v13_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        meta.insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
            .unwrap();
    }
    let conn = raw(&root);
    for table in HUB_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 13", [])
        .unwrap();
    drop(conn);
    assert!(!table_exists(&root, "hub_events"));
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    for table in HUB_TABLES {
        assert!(table_exists(&root, table), "{table}");
    }
    assert!(meta.get_project(&id("proj-1")).is_ok());
    drop(meta);
    // Reopening is idempotent.
    let meta = open_meta(&root);
    assert!(meta.get_hub_identity().unwrap().is_none());
}

#[test]
fn hub_rows_hold_their_invariants() {
    let root = temp_root("invariants");
    let meta = open_meta(&root);
    let events = build_state(&meta);
    assert_eq!(events.len(), 6);
    assert!(matches!(
        meta.insert_hub_identity(&hub_identity()),
        Err(MetaError::Conflict(_))
    ));
    // Invitations: redeemed once only; a device must bind its invitation's
    // Project.
    assert!(matches!(
        meta.enroll_device(&id("invitation-1"), &device(1)),
        Err(MetaError::Conflict(_))
    ));
    let mut stray = device(3);
    stray.project_id = id("proj-2");
    assert!(matches!(
        meta.enroll_device(&id("invitation-3"), &stray),
        Err(MetaError::UnsupportedSchema(_))
    ));
    assert_eq!(
        meta.get_invitation(&id("invitation-3")).unwrap().status,
        InvitationStatus::Open
    );
    meta.revoke_invitation(&id("invitation-3")).unwrap();
    assert!(meta.revoke_invitation(&id("invitation-3")).is_err());
    // Nonces are single-use and bound to their device.
    meta.insert_hub_nonce(&id("device-1"), &"c".repeat(64))
        .unwrap();
    assert!(
        !meta
            .take_hub_nonce(&id("device-2"), &"c".repeat(64))
            .unwrap()
    );
    assert!(
        meta.take_hub_nonce(&id("device-1"), &"c".repeat(64))
            .unwrap()
    );
    assert!(
        !meta
            .take_hub_nonce(&id("device-1"), &"c".repeat(64))
            .unwrap()
    );
    // Sequence claims: one event per (device, seq); a claim must match the
    // outcome and the envelope digest.
    assert_eq!(meta.last_claimed_seq(&id("device-1")).unwrap(), 2);
    let dup = envelope(1, 2, "second");
    assert!(matches!(
        meta.append_hub_event(
            &id("hub-1"),
            &id("proj-1"),
            HubEventKind::Submission {
                envelope: dup.clone(),
                outcome: applied(2)
            },
            Some(&claim(&dup)),
        ),
        Err(MetaError::Conflict(_))
    ));
    let e3 = envelope(1, 3, "third");
    let mut wrong = claim(&e3);
    wrong.envelope_digest = DigestSha256::of(b"other");
    assert!(matches!(
        meta.append_hub_event(
            &id("hub-1"),
            &id("proj-1"),
            HubEventKind::Submission {
                envelope: e3.clone(),
                outcome: applied(3)
            },
            Some(&wrong),
        ),
        Err(MetaError::UnsupportedSchema(_))
    ));
    assert!(matches!(
        meta.append_hub_event(
            &id("hub-1"),
            &id("proj-1"),
            HubEventKind::Submission {
                envelope: e3,
                outcome: applied(3)
            },
            None,
        ),
        Err(MetaError::UnsupportedSchema(_))
    ));
    // The outbox takes only the next sequence of the link's own device.
    let mut skip = envelope(1, 9, "skip");
    skip.body.seq = 9;
    assert!(matches!(
        meta.queue_hub_outbox(&HubOutboxEntry {
            link_id: id("link-1"),
            envelope: skip,
            state: OutboxState::Pending,
            outcome: None,
        }),
        Err(MetaError::Conflict(_))
    ));
    assert!(matches!(
        meta.record_hub_outcome(&id("link-1"), 1, &applied(9)),
        Err(MetaError::Conflict(_))
    ));
    // The mirror refuses a page that does not continue its chain.
    let events_again = meta.list_hub_events(&id("proj-1"), 0, 100).unwrap();
    assert!(
        meta.append_hub_mirror(&id("link-1"), &events_again[..1])
            .is_err()
    );
    // Pages start after the cursor and are verified against it.
    let page = meta.list_hub_events(&id("proj-1"), 3, 2).unwrap();
    assert_eq!(
        page.iter().map(|e| e.cursor).collect::<Vec<_>>(),
        vec![4, 5]
    );
    meta.verify_hub_consistency().unwrap();
}

#[test]
fn edited_or_removed_event_rows_fail_closed_on_read() {
    let root = temp_root("chain");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    let body: String = conn
        .query_row(
            "SELECT body_json FROM hub_events WHERE cursor = 3",
            [],
            |row| row.get(0),
        )
        .unwrap();
    conn.execute(
        "UPDATE hub_events SET body_json = ?1 WHERE cursor = 3",
        [body.replace("hello", "HELLO")],
    )
    .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert!(meta.verify_hub_chain(&id("hub-1"), &id("proj-1")).is_err());
    assert!(meta.list_hub_events(&id("proj-1"), 0, 100).is_err());
    drop(meta);
    let conn = raw(&root);
    conn.execute(
        "UPDATE hub_events SET body_json = ?1 WHERE cursor = 3",
        [body],
    )
    .unwrap();
    conn.execute("DELETE FROM hub_events WHERE cursor = 4", [])
        .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert!(meta.verify_hub_chain(&id("hub-1"), &id("proj-1")).is_err());
    assert!(meta.list_hub_events(&id("proj-1"), 4, 10).is_err());
    assert!(meta.verify_hub_consistency().is_err());
}

#[test]
fn backup_restore_roundtrips_hub_rows_without_secrets() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    let snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(dest.join("metadata.snapshot")).unwrap()).unwrap();
    assert!(!snapshot.to_string().contains(&"7".repeat(64)));
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    assert_eq!(
        restored.meta.list_all_hub_events().unwrap(),
        original.meta.list_all_hub_events().unwrap()
    );
    assert_eq!(
        restored.meta.list_devices().unwrap(),
        original.meta.list_devices().unwrap()
    );
    assert_eq!(
        restored.meta.list_hub_links().unwrap(),
        original.meta.list_hub_links().unwrap()
    );
    assert_eq!(
        restored.meta.list_hub_outbox(&id("link-1"), false).unwrap(),
        original.meta.list_hub_outbox(&id("link-1"), false).unwrap()
    );
    assert_eq!(
        restored
            .meta
            .list_hub_mirror(&id("link-1"), 0, 100)
            .unwrap(),
        original
            .meta
            .list_hub_mirror(&id("link-1"), 0, 100)
            .unwrap()
    );
    // Secrets are not in backups; open invitations come back revoked.
    assert!(
        restored
            .meta
            .get_hub_link_secret(&id("link-1"))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        restored
            .meta
            .get_invitation(&id("invitation-3"))
            .unwrap()
            .status,
        InvitationStatus::Revoked
    );
    assert_eq!(
        restored
            .meta
            .get_invitation(&id("invitation-1"))
            .unwrap()
            .status,
        InvitationStatus::Redeemed
    );
    restored.meta.verify_hub_consistency().unwrap();
}

#[test]
fn restore_rejects_hand_edited_084_snapshots() {
    type Edit = (&'static str, fn(&mut serde_json::Value));
    let edits: [Edit; 14] = [
        ("event outcome edited", |s| {
            rows(s, "hub_events")[2]["kind"]["outcome"]["object_id"] =
                serde_json::json!("message-9");
        }),
        ("event removed", |s| {
            rows(s, "hub_events").remove(3);
        }),
        ("events reordered", |s| {
            rows(s, "hub_events").swap(2, 3);
        }),
        ("device key swapped", |s| {
            rows(s, "hub_devices")[0]["public_key_hex"] = serde_json::json!("f".repeat(64));
        }),
        ("device moved to another project", |s| {
            rows(s, "hub_devices")[0]["project_id"] = serde_json::json!("proj-2");
        }),
        ("revoked device restored active", |s| {
            rows(s, "hub_devices")[1]["status"] = serde_json::json!("active");
        }),
        ("invitation names another device", |s| {
            rows(s, "hub_invitations")[0]["device_id"] = serde_json::json!("device-2");
        }),
        ("hub events replaced by a string", |s| {
            s["hub_events"] = serde_json::json!("events");
        }),
        ("hub devices missing", |s| {
            s.as_object_mut().unwrap().remove("hub_devices");
        }),
        ("second hub identity", |s| {
            let mut other = rows(s, "hub_identity")[0].clone();
            other["header"]["id"] = serde_json::json!("hub-2");
            rows(s, "hub_identity").push(other);
        }),
        ("link cursor ahead of its mirror", |s| {
            rows(s, "hub_links")[0]["cursor"] = serde_json::json!(9);
        }),
        ("outbox entry removed", |s| {
            rows(s, "hub_outbox").remove(0);
        }),
        ("mirrored event edited", |s| {
            rows(s, "hub_mirror")[2]["event"]["cursor"] = serde_json::json!(7);
        }),
        ("claimed sequence duplicated", |s| {
            let mut dup = rows(s, "hub_events")[2].clone();
            dup["cursor"] = serde_json::json!(7);
            rows(s, "hub_events").push(dup);
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

#[test]
fn pre_084_v12_backup_restores_with_empty_hub_tables() {
    let root = temp_root("v12-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    vault
        .meta
        .insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    tamper_backup(&dest, |s| {
        let o = s.as_object_mut().unwrap();
        o.retain(|k, _| {
            !k.starts_with("hub_")
                && !k.starts_with("compute_")
                && !k.starts_with("rws_")
                && !k.starts_with("ext_")
                && !k.starts_with("hud_")
                && !k.starts_with("rp_")
                && !k.starts_with("ia_")
                && !k.starts_with("fed_")
        });
        o.insert("schema_version".to_owned(), serde_json::json!(12));
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.get_hub_identity().unwrap().is_none());
    assert!(restored.meta.list_hub_links().unwrap().is_empty());
    assert!(restored.meta.get_project(&id("proj-1")).is_ok());
}

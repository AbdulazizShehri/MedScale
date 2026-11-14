//! Spec 090 institutional adapter storage + migration integration tests.
//!
//! Synthetic rows only. Additive v18 -> v19 migration, compare-and-set
//! changes with receipts, intent validation (payload-bound idempotency,
//! never `local_phi`), consistency and backup/restore with tampering.

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::institutional::{
    AdapterAction, AdapterCapability, AdapterConfig, AdapterKind, AdapterReceipt, AdapterState,
    ExternalWriteIntent, INSTITUTIONAL_SCHEMA_VERSION, InstitutionalAdapter, idempotency_key,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, EffectState, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::privacy_gate::DataClass;
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    AdapterChange, CURRENT_META_SCHEMA_VERSION, INSTITUTIONAL_TABLES, MetaError, SqliteMetaStore,
    SyntheticVault, backup_vault, restore_vault,
};

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-090s-{name}-{}", std::process::id()));
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
        schema_version: INSTITUTIONAL_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn adapter(revision: u64) -> InstitutionalAdapter {
    InstitutionalAdapter {
        header: h("institutional-adapter-1"),
        project_id: id("proj-1"),
        name: "lab-s3".to_owned(),
        config: AdapterConfig {
            kind: AdapterKind::ObjectStorage,
            destination: "s3://lab".to_owned(),
            data_class_ceiling: DataClass::Public,
            credential_handle: Some("cred:lab".to_owned()),
            capabilities: vec![AdapterCapability::PutObject, AdapterCapability::HeadObject],
        },
        config_revision: 1,
        previous_config: None,
        state: AdapterState::Active,
        revision,
    }
}

fn intent(state: EffectState, revision: u64) -> ExternalWriteIntent {
    let payload = DigestSha256::of(b"{}");
    ExternalWriteIntent {
        header: h("write-intent-1"),
        project_id: id("proj-1"),
        adapter_id: id("institutional-adapter-1"),
        config_revision: 1,
        destination: "s3://lab".to_owned(),
        object_key: "k.json".to_owned(),
        artifact_id: id("source-1"),
        idempotency_key: idempotency_key(
            &id("institutional-adapter-1"),
            "s3://lab",
            "k.json",
            &payload,
        ),
        payload_digest: payload,
        payload_bytes: 2,
        data_class: DataClass::Public,
        state,
        attempts: 0,
        revision,
    }
}

fn receipt(n: u32, action: AdapterAction) -> AdapterReceipt {
    AdapterReceipt {
        header: h(&format!("adapter-receipt-{n}")),
        project_id: id("proj-1"),
        adapter_id: id("institutional-adapter-1"),
        intent_id: None,
        action,
        from_state: None,
        to_state: None,
        refusal: None,
        transport_outcome: None,
    }
}

fn build_state(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "p".to_owned(), None).unwrap())
        .unwrap();
    let a = adapter(1);
    meta.commit_adapter_change(&AdapterChange {
        adapter: Some((&a, None)),
        intent: None,
        receipt: Some(&receipt(1, AdapterAction::Register)),
    })
    .unwrap();
    let i = intent(EffectState::Pending, 1);
    meta.commit_adapter_change(&AdapterChange {
        adapter: None,
        intent: Some((&i, None)),
        receipt: Some(&receipt(2, AdapterAction::Intend)),
    })
    .unwrap();
}

#[test]
fn migration_to_v19_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    for table in INSTITUTIONAL_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 19", [])
        .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    assert!(meta.list_institutional_adapters().unwrap().is_empty());
}

#[test]
fn intents_are_bound_and_changes_are_compare_and_set() {
    let root = temp_root("cas");
    let meta = open_meta(&root);
    build_state(&meta);
    meta.verify_institutional_consistency().unwrap();

    // An idempotency key not bound to the payload is refused.
    let mut forged = intent(EffectState::Sent, 2);
    forged.payload_digest = DigestSha256::of(b"other");
    assert!(
        meta.commit_adapter_change(&AdapterChange {
            adapter: None,
            intent: Some((&forged, Some(1))),
            receipt: Some(&receipt(3, AdapterAction::Send)),
        })
        .is_err()
    );
    // local_phi is refused at the row level too.
    let mut phi = intent(EffectState::Sent, 2);
    phi.data_class = DataClass::LocalPhi;
    assert!(
        meta.commit_adapter_change(&AdapterChange {
            adapter: None,
            intent: Some((&phi, Some(1))),
            receipt: Some(&receipt(4, AdapterAction::Send)),
        })
        .is_err()
    );
    // Stale revision: conflict, nothing written.
    let stale = intent(EffectState::Sent, 2);
    assert!(matches!(
        meta.commit_adapter_change(&AdapterChange {
            adapter: None,
            intent: Some((&stale, Some(5))),
            receipt: Some(&receipt(5, AdapterAction::Send)),
        }),
        Err(MetaError::UnsupportedSchema(_) | MetaError::Conflict(_))
    ));
    assert_eq!(meta.list_adapter_receipts(None).unwrap().len(), 2);
    let ok = intent(EffectState::Sent, 2);
    meta.commit_adapter_change(&AdapterChange {
        adapter: None,
        intent: Some((&ok, Some(1))),
        receipt: Some(&receipt(6, AdapterAction::Send)),
    })
    .unwrap();
    assert_eq!(
        meta.get_write_intent(&id("write-intent-1")).unwrap().state,
        EffectState::Sent
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
fn backup_restore_round_trips_adapter_rows() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let a = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let b = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        a.meta.list_write_intents(None).unwrap(),
        b.meta.list_write_intents(None).unwrap()
    );
    b.meta.verify_institutional_consistency().unwrap();
}

#[test]
fn tampered_adapter_backups_are_refused() {
    type Edit = fn(&mut serde_json::Value);
    let edits: Vec<(&str, Edit)> = vec![
        ("adapters family missing", |s| {
            s.as_object_mut().unwrap().remove("ia_adapters");
        }),
        ("adapter ceiling raised to local_phi", |s| {
            rows(s, "ia_adapters")[0]["config"]["data_class_ceiling"] =
                serde_json::json!("local_phi");
        }),
        ("credential secret smuggled in", |s| {
            rows(s, "ia_adapters")[0]["config"]["credential_handle"] =
                serde_json::json!("AKIA:secret");
        }),
        ("intent re-pointed at other bytes", |s| {
            rows(s, "ia_intents")[0]["payload_digest"] =
                serde_json::to_value(DigestSha256::of(b"other")).unwrap();
        }),
        ("intent of a missing adapter", |s| {
            rows(s, "ia_adapters").remove(0);
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

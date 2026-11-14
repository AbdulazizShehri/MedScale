//! Spec 091 federation storage + migration integration tests.
//!
//! Synthetic rows only. Additive v19 -> v20 migration, the identity secret
//! never leaving through a backup, atomic imports with tombstones, and
//! backup/restore with tampering.

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::federation::{
    FEDERATION_SCHEMA_VERSION, FederationAction, FederationIdentity, FederationPeer,
    FederationReceipt, ImportedItem, ImportedState, PeerState, ProvenanceStep,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::privacy_gate::DataClass;
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, FEDERATION_TABLES, FederationChange, MetaError, SqliteMetaStore,
    SyntheticVault, backup_vault, restore_vault,
};

const ITEM: &[u8] = b"{\"n\":1}";

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-091s-{name}-{}", std::process::id()));
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
        schema_version: FEDERATION_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn receipt(n: u32, action: FederationAction) -> FederationReceipt {
    FederationReceipt {
        header: h(&format!("federation-receipt-{n}")),
        action,
        peer_institution: None,
        seq: None,
        item_digests: Vec::new(),
        tombstoned: Vec::new(),
        refusal: None,
    }
}

fn peer(revision: u64, last_received: u64) -> FederationPeer {
    FederationPeer {
        header: h("federation-peer-1"),
        institution_id: "hospital-a".to_owned(),
        public_key_hex: "ab".repeat(32),
        ceiling: DataClass::Public,
        state: PeerState::Trusted,
        last_received_seq: last_received,
        last_sent_seq: 0,
        revision,
    }
}

fn item() -> ImportedItem {
    ImportedItem {
        header: h("imported-item-1"),
        project_id: id("proj-1"),
        from_institution: "hospital-a".to_owned(),
        bundle_seq: 1,
        media_type: "application/json".to_owned(),
        digest: DigestSha256::of(ITEM),
        data_class: DataClass::Public,
        provenance: vec![ProvenanceStep {
            institution_id: "hospital-a".to_owned(),
            artifact_id: id("source-9"),
            digest: DigestSha256::of(ITEM),
        }],
        state: ImportedState::Active,
    }
}

fn build_state(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "p".to_owned(), None).unwrap())
        .unwrap();
    let identity = FederationIdentity {
        header: h("federation-identity-1"),
        institution_id: "hospital-b".to_owned(),
        public_key_hex: "cd".repeat(32),
    };
    meta.commit_federation_change(&FederationChange {
        identity: Some((&identity, "ef")),
        receipt: Some(&receipt(1, FederationAction::CreateIdentity)),
        ..FederationChange::default()
    })
    .unwrap();
    let p = peer(1, 0);
    meta.commit_federation_change(&FederationChange {
        peer: Some((&p, None)),
        receipt: Some(&receipt(2, FederationAction::TrustPeer)),
        ..FederationChange::default()
    })
    .unwrap();
    let p2 = peer(2, 1);
    let i = item();
    meta.commit_federation_change(&FederationChange {
        peer: Some((&p2, Some(1))),
        new_items: vec![(&i, ITEM)],
        receipt: Some(&receipt(3, FederationAction::Import)),
        ..FederationChange::default()
    })
    .unwrap();
}

#[test]
fn migration_to_v20_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    for table in FEDERATION_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 20", [])
        .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    assert!(meta.get_federation_identity().unwrap().is_none());
}

#[test]
fn imports_are_atomic_and_identities_are_single() {
    let root = temp_root("atomic");
    let meta = open_meta(&root);
    build_state(&meta);
    meta.verify_federation_consistency().unwrap();

    // A second identity is refused.
    let other = FederationIdentity {
        header: h("federation-identity-2"),
        institution_id: "hospital-x".to_owned(),
        public_key_hex: "cd".repeat(32),
    };
    assert!(matches!(
        meta.commit_federation_change(&FederationChange {
            identity: Some((&other, "aa")),
            receipt: Some(&receipt(4, FederationAction::CreateIdentity)),
            ..FederationChange::default()
        }),
        Err(MetaError::Conflict(_))
    ));
    // An item whose bytes do not match its digest rolls the whole import
    // back, including the peer sequence advance.
    let p3 = peer(3, 2);
    let mut bad = item();
    bad.header = h("imported-item-2");
    bad.digest = DigestSha256::of(b"different");
    assert!(
        meta.commit_federation_change(&FederationChange {
            peer: Some((&p3, Some(2))),
            new_items: vec![(&bad, ITEM)],
            receipt: Some(&receipt(5, FederationAction::Import)),
            ..FederationChange::default()
        })
        .is_err()
    );
    assert_eq!(
        meta.get_federation_peer("hospital-a")
            .unwrap()
            .last_received_seq,
        1
    );
    // A tombstone erases the bytes and keeps the record.
    let i = item();
    meta.commit_federation_change(&FederationChange {
        peer: Some((&p3, Some(2))),
        tombstone_items: vec![&i],
        receipt: Some(&receipt(6, FederationAction::Import)),
        ..FederationChange::default()
    })
    .unwrap();
    let rows = meta.list_imported_items().unwrap();
    assert_eq!(rows[0].item.state, ImportedState::Tombstoned);
    assert!(rows[0].content_hex.is_empty());
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
fn backups_never_carry_the_identity_secret() {
    let root = temp_root("secret");
    let dest = backed_up(&root);
    let snapshot = fs::read_to_string(dest.join("metadata.snapshot")).unwrap();
    assert!(!snapshot.contains("secret_hex"));
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    let (identity, secret) = restored.meta.get_federation_identity().unwrap().unwrap();
    assert_eq!(identity.institution_id, "hospital-b");
    assert!(secret.is_none());
    restored.meta.verify_federation_consistency().unwrap();
}

#[test]
fn tampered_federation_backups_are_refused() {
    type Edit = fn(&mut serde_json::Value);
    let edits: Vec<(&str, Edit)> = vec![
        ("peers family missing", |s| {
            s.as_object_mut().unwrap().remove("fed_peers");
        }),
        ("peer ceiling raised to local_phi", |s| {
            rows(s, "fed_peers")[0]["ceiling"] = serde_json::json!("local_phi");
        }),
        ("imported bytes edited", |s| {
            rows(s, "fed_imported")[0]["content_hex"] = serde_json::json!("7b7d");
        }),
        ("item from an unknown peer", |s| {
            rows(s, "fed_peers").remove(0);
        }),
        ("duplicate item", |s| {
            let mut dup = rows(s, "fed_imported")[0].clone();
            dup["item"]["header"]["id"] = serde_json::json!("imported-item-9");
            rows(s, "fed_imported").push(dup);
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

//! Spec 087 Community Extensions storage + migration integration tests.
//!
//! Synthetic rows only. Additive v15 -> v16 migration, compare-and-set
//! install changes committed with their receipts, atomic quarantine on
//! revocation, cross-row invariants and backup/restore (including
//! tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::extensions::{
    EXTENSION_SCHEMA_VERSION, EntrypointKind, ExtensionCapability, ExtensionCommand,
    ExtensionGrant, ExtensionInstallRecord, ExtensionLifecycleReceipt, ExtensionManifest,
    ExtensionPublisher, ExtensionRelease, ExtensionVersion, HostOperation, InstallState,
    LifecycleAction, PublisherState,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::privacy_gate::DataClass;
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, EXTENSION_TABLES, InstallChange, MetaError, SqliteMetaStore,
    SyntheticVault, backup_vault, restore_vault,
};

const KEY: &str = "3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29";

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-087s-{name}-{}", std::process::id()));
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
        schema_version: EXTENSION_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn publisher() -> ExtensionPublisher {
    ExtensionPublisher {
        header: h("extension-publisher-1"),
        publisher_id: "example".to_owned(),
        key_hex: KEY.to_owned(),
        state: PublisherState::Trusted,
    }
}

fn release(minor: u32) -> ExtensionRelease {
    let manifest = ExtensionManifest {
        extension_id: "org.example.peek".to_owned(),
        version: ExtensionVersion {
            major: 1,
            minor,
            patch: 0,
        },
        publisher_id: "example".to_owned(),
        publisher_key_hex: KEY.to_owned(),
        host_api_min: 1,
        host_api_max: 1,
        license: "Apache-2.0".to_owned(),
        description: String::new(),
        entrypoint: EntrypointKind::Declarative,
        capabilities: vec![ExtensionCapability::SnapshotSchemaRead],
        commands: vec![ExtensionCommand {
            name: "schema".to_owned(),
            title: "Schema".to_owned(),
            operation: HostOperation::SnapshotSchema,
            max_rows: None,
        }],
    };
    let manifest_json = String::from_utf8(manifest.canonical_bytes()).unwrap();
    ExtensionRelease {
        header: h(&format!("extension-release-{minor}")),
        digest: DigestSha256::of(manifest_json.as_bytes()),
        manifest,
        manifest_json,
        signature_hex: "ab".repeat(64),
        revoked: false,
    }
}

fn install(revision: u64, state: InstallState) -> ExtensionInstallRecord {
    ExtensionInstallRecord {
        header: h("extension-install-1"),
        project_id: id("proj-1"),
        extension_id: "org.example.peek".to_owned(),
        publisher_id: "example".to_owned(),
        active_release: release(0).digest,
        previous_release: None,
        state,
        revision,
    }
}

fn receipt(
    n: u32,
    i: &ExtensionInstallRecord,
    action: LifecycleAction,
) -> ExtensionLifecycleReceipt {
    ExtensionLifecycleReceipt {
        header: h(&format!("extension-receipt-{n}")),
        project_id: id("proj-1"),
        action,
        extension_id: i.extension_id.clone(),
        release: Some(i.active_release.clone()),
        install_id: Some(i.header.id.clone()),
        refusal: None,
        capability_expansion: Vec::new(),
        resulting_state: Some(i.state),
    }
}

fn grant() -> ExtensionGrant {
    ExtensionGrant {
        header: h("extension-grant-1"),
        install_id: id("extension-install-1"),
        project_id: id("proj-1"),
        capability: ExtensionCapability::SnapshotSchemaRead,
        data_class_ceiling: DataClass::LocalPhi,
        revoked: false,
    }
}

fn build_state(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "p".to_owned(), None).unwrap())
        .unwrap();
    meta.insert_ext_publisher(&publisher()).unwrap();
    meta.insert_ext_release(&release(0)).unwrap();
    let i = install(1, InstallState::Enabled);
    meta.commit_ext_install_change(&InstallChange {
        install: &i,
        expected_revision: None,
        new_grants: &[],
        revoke_grants: &[],
        receipt: &receipt(1, &i, LifecycleAction::Install),
    })
    .unwrap();
    let i2 = install(2, InstallState::Enabled);
    meta.commit_ext_install_change(&InstallChange {
        install: &i2,
        expected_revision: Some(1),
        new_grants: &[grant()],
        revoke_grants: &[],
        receipt: &receipt(2, &i2, LifecycleAction::Grant),
    })
    .unwrap();
}

#[test]
fn migration_to_v16_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    for table in EXTENSION_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 16", [])
        .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    assert!(meta.list_ext_installs().unwrap().is_empty());
    assert!(meta.get_project(&id("proj-1")).is_ok());
}

#[test]
fn install_changes_are_compare_and_set_and_atomic() {
    let root = temp_root("cas");
    let meta = open_meta(&root);
    build_state(&meta);
    meta.verify_extension_consistency().unwrap();

    // A stale revision is a conflict and writes nothing.
    let stale = install(2, InstallState::Disabled);
    assert!(matches!(
        meta.commit_ext_install_change(&InstallChange {
            install: &stale,
            expected_revision: Some(1),
            new_grants: &[],
            revoke_grants: &[],
            receipt: &receipt(3, &stale, LifecycleAction::Disable),
        }),
        Err(MetaError::Conflict(_)) | Err(MetaError::UnsupportedSchema(_))
    ));
    // A receipt that does not describe its install is refused.
    let next = install(3, InstallState::Disabled);
    let mut wrong = receipt(4, &next, LifecycleAction::Disable);
    wrong.resulting_state = Some(InstallState::Enabled);
    assert!(
        meta.commit_ext_install_change(&InstallChange {
            install: &next,
            expected_revision: Some(2),
            new_grants: &[],
            revoke_grants: &[],
            receipt: &wrong,
        })
        .is_err()
    );
    // A duplicate grant id rolls the whole change back.
    assert!(
        meta.commit_ext_install_change(&InstallChange {
            install: &next,
            expected_revision: Some(2),
            new_grants: &[grant()],
            revoke_grants: &[],
            receipt: &receipt(5, &next, LifecycleAction::Grant),
        })
        .is_err()
    );
    assert_eq!(meta.list_ext_installs().unwrap()[0].revision, 2);
    assert_eq!(meta.list_ext_lifecycle_receipts().unwrap().len(), 2);

    // Revocation quarantines atomically.
    let q = install(3, InstallState::Quarantined);
    meta.commit_ext_revocation(
        Some("example"),
        None,
        &[(q.clone(), 2, receipt(6, &q, LifecycleAction::Quarantine))],
    )
    .unwrap();
    assert_eq!(
        meta.get_ext_publisher("example").unwrap().state,
        PublisherState::Revoked
    );
    assert_eq!(
        meta.list_ext_installs().unwrap()[0].state,
        InstallState::Quarantined
    );
    meta.verify_extension_consistency().unwrap();
    assert!(matches!(
        meta.insert_ext_publisher(&publisher()),
        Err(MetaError::Conflict(_))
    ));
}

#[test]
fn tampered_release_rows_fail_closed() {
    let root = temp_root("rows");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    let body: String = conn
        .query_row("SELECT body_json FROM ext_releases", [], |r| r.get(0))
        .unwrap();
    let tampered = body.replacen("snapshot_schema_read", "snapshot_rows_read", 1);
    conn.execute("UPDATE ext_releases SET body_json = ?1", [tampered])
        .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert!(meta.list_ext_releases().is_err());
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
fn backup_restore_round_trips_extension_rows() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let a = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let b = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        a.meta.list_ext_installs().unwrap(),
        b.meta.list_ext_installs().unwrap()
    );
    assert_eq!(
        a.meta.list_ext_releases().unwrap(),
        b.meta.list_ext_releases().unwrap()
    );
    assert_eq!(
        a.meta.list_ext_grants(None).unwrap(),
        b.meta.list_ext_grants(None).unwrap()
    );
    b.meta.verify_extension_consistency().unwrap();
}

#[test]
fn tampered_extension_backups_are_refused() {
    type Edit = fn(&mut serde_json::Value);
    let edits: Vec<(&str, Edit)> = vec![
        ("installs family missing", |s| {
            s.as_object_mut().unwrap().remove("ext_installs");
        }),
        ("release capability widened", |s| {
            let r = &mut rows(s, "ext_releases")[0];
            let json = r["manifest_json"]
                .as_str()
                .unwrap()
                .replace("snapshot_schema_read", "snapshot_rows_read");
            r["manifest_json"] = serde_json::json!(json);
        }),
        ("release key swapped", |s| {
            rows(s, "ext_publishers")[0]["key_hex"] = serde_json::json!("cd".repeat(32));
        }),
        ("release removed under its install", |s| {
            rows(s, "ext_releases").remove(0);
        }),
        ("grant moved to another project", |s| {
            rows(s, "ext_grants")[0]["project_id"] = serde_json::json!("proj-2");
        }),
        ("publisher revoked but install enabled", |s| {
            rows(s, "ext_publishers")[0]["state"] = serde_json::json!("revoked");
        }),
        ("install revision zero", |s| {
            rows(s, "ext_installs")[0]["revision"] = serde_json::json!(0);
        }),
        ("unknown install state", |s| {
            rows(s, "ext_installs")[0]["state"] = serde_json::json!("running");
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

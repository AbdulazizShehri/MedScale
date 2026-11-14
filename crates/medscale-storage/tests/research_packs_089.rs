//! Spec 089 Research Pack storage + migration integration tests.
//!
//! Synthetic rows only. Additive v17 -> v18 migration, rows re-checked
//! against the shipped Pack version they name, compare-and-set changes
//! with receipts, cross-row invariants and backup/restore (including
//! tampered snapshots).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::Project;
use medscale_contracts::research_packs::{
    CLINICAL_RESEARCH_PACK_ID, FieldValue, PackAction, PackInstallState, PackReceipt,
    RESEARCH_PACK_SCHEMA_VERSION, ResearchArtifact, ResearchPackInstall, clinical_research_pack,
};
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, MetaError, PackChange, RESEARCH_PACK_TABLES, SqliteMetaStore,
    SyntheticVault, backup_vault, restore_vault,
};

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-089s-{name}-{}", std::process::id()));
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
        schema_version: RESEARCH_PACK_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn install(version: u32, revision: u64) -> ResearchPackInstall {
    ResearchPackInstall {
        header: h("pack-install-1"),
        project_id: id("proj-1"),
        pack_id: CLINICAL_RESEARCH_PACK_ID.to_owned(),
        version,
        manifest_digest: clinical_research_pack(version).unwrap().digest(),
        state: PackInstallState::Enabled,
        revision,
    }
}

fn artifact(version: u32, revision: u64) -> ResearchArtifact {
    let mut fields = BTreeMap::new();
    fields.insert("title".to_owned(), FieldValue::Text("LDL".to_owned()));
    fields.insert("phase".to_owned(), FieldValue::Choice("2".to_owned()));
    fields.insert("primary_outcome".to_owned(), FieldValue::Unknown);
    ResearchArtifact {
        header: h("research-artifact-1"),
        project_id: id("proj-1"),
        pack_id: CLINICAL_RESEARCH_PACK_ID.to_owned(),
        pack_version: version,
        type_id: "study_protocol".to_owned(),
        fields,
        workflow_state: "draft".to_owned(),
        assessments: Vec::new(),
        revision,
    }
}

fn receipt(n: u32, action: PackAction) -> PackReceipt {
    PackReceipt {
        header: h(&format!("pack-receipt-{n}")),
        project_id: id("proj-1"),
        pack_id: CLINICAL_RESEARCH_PACK_ID.to_owned(),
        action,
        targets: Vec::new(),
        from_version: None,
        to_version: None,
    }
}

fn build_state(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "p".to_owned(), None).unwrap())
        .unwrap();
    let i = install(1, 1);
    meta.commit_pack_change(&PackChange {
        install: Some((&i, None)),
        artifacts: Vec::new(),
        receipt: Some(&receipt(1, PackAction::Install)),
    })
    .unwrap();
    let a = artifact(1, 1);
    meta.commit_pack_change(&PackChange {
        install: None,
        artifacts: vec![(&a, None)],
        receipt: Some(&receipt(2, PackAction::Create)),
    })
    .unwrap();
}

#[test]
fn migration_to_v18_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    for table in RESEARCH_PACK_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 18", [])
        .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    assert!(meta.list_pack_installs().unwrap().is_empty());
}

#[test]
fn rows_are_checked_against_the_shipped_pack() {
    let root = temp_root("checks");
    let meta = open_meta(&root);
    build_state(&meta);
    meta.verify_research_pack_consistency().unwrap();

    // An unknown version or a forged digest is refused.
    let mut forged = install(1, 2);
    forged.manifest_digest = DigestSha256::of(b"not the pack");
    assert!(
        meta.commit_pack_change(&PackChange {
            install: Some((&forged, Some(1))),
            artifacts: Vec::new(),
            receipt: Some(&receipt(3, PackAction::Upgrade)),
        })
        .is_err()
    );
    // A field outside the schema is refused.
    let mut extra = artifact(1, 2);
    extra
        .fields
        .insert("registry_id".to_owned(), FieldValue::Unknown);
    assert!(
        meta.commit_pack_change(&PackChange {
            install: None,
            artifacts: vec![(&extra, Some(1))],
            receipt: Some(&receipt(4, PackAction::Update)),
        })
        .is_err()
    );
    // A stale revision conflicts; nothing is written.
    let stale = artifact(1, 2);
    assert!(matches!(
        meta.commit_pack_change(&PackChange {
            install: None,
            artifacts: vec![(&stale, Some(7))],
            receipt: Some(&receipt(5, PackAction::Update)),
        }),
        Err(MetaError::UnsupportedSchema(_) | MetaError::Conflict(_))
    ));
    assert_eq!(meta.list_pack_receipts().unwrap().len(), 2);

    // Upgrade install and artifact together.
    let i2 = install(2, 2);
    let mut a2 = artifact(2, 2);
    a2.fields
        .insert("registry_id".to_owned(), FieldValue::Unknown);
    meta.commit_pack_change(&PackChange {
        install: Some((&i2, Some(1))),
        artifacts: vec![(&a2, Some(1))],
        receipt: Some(&receipt(6, PackAction::Upgrade)),
    })
    .unwrap();
    meta.verify_research_pack_consistency().unwrap();
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
fn backup_restore_round_trips_pack_rows() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let a = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let b = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        a.meta.list_pack_installs().unwrap(),
        b.meta.list_pack_installs().unwrap()
    );
    assert_eq!(
        a.meta.list_research_artifacts(None).unwrap(),
        b.meta.list_research_artifacts(None).unwrap()
    );
    b.meta.verify_research_pack_consistency().unwrap();
}

#[test]
fn tampered_pack_backups_are_refused() {
    type Edit = fn(&mut serde_json::Value);
    let edits: Vec<(&str, Edit)> = vec![
        ("installs family missing", |s| {
            s.as_object_mut().unwrap().remove("rp_installs");
        }),
        ("install claims an unshipped version", |s| {
            rows(s, "rp_installs")[0]["version"] = serde_json::json!(9);
        }),
        ("artifact in an unknown workflow state", |s| {
            rows(s, "rp_artifacts")[0]["workflow_state"] = serde_json::json!("deleted");
        }),
        ("artifact of an uninstalled pack", |s| {
            rows(s, "rp_installs").remove(0);
        }),
        ("artifact at another version than its install", |s| {
            rows(s, "rp_artifacts")[0]["pack_version"] = serde_json::json!(2);
        }),
        ("artifact field of the wrong kind", |s| {
            rows(s, "rp_artifacts")[0]["fields"]["phase"] =
                serde_json::json!({"kind":"integer","value":2});
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

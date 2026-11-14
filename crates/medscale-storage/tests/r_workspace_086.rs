//! Spec 086 R Workspace storage + migration integration tests.
//!
//! Synthetic rows only. Additive v14 -> v15 migration, receipt-to-workspace
//! binding, atomic publications, cross-row invariants and backup/restore
//! (including tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::analytics::{ResultColumn, ResultTableDoc};
use medscale_contracts::data_sources::CellValue;
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::privacy_gate::DataClass;
use medscale_contracts::project_graph::Project;
use medscale_contracts::r_workspace::{
    ClassBasis, IdeKind, LaunchState, OutputPolicy, PublishRefusal, PublishState,
    R_WORKSPACE_LAYOUT_VERSION, R_WORKSPACE_SCHEMA_VERSION, README_FILE, RLaunchReceipt,
    RPublishReceipt, RPublishedTable, RRunReceipt, RRuntimeIdentity, RStagedInput,
    RVersionEvidence, RWorkspace, RWorkspaceManifest, RunRefusal, StagedFile, StagingMode,
    data_file_name, schema_file_name,
};
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, MetaError, R_WORKSPACE_TABLES, SqliteMetaStore, SyntheticVault,
    backup_vault, restore_vault,
};

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-086s-{name}-{}", std::process::id()));
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

fn h(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: R_WORKSPACE_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn d(s: &str) -> DigestSha256 {
    DigestSha256::of(s.as_bytes())
}

fn project() -> Project {
    Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap()
}

fn workspace(n: u32) -> RWorkspace {
    let snapshot = id("snapshot-1");
    let input = RStagedInput {
        data_file: data_file_name(&snapshot),
        schema_file: schema_file_name(&snapshot),
        snapshot_id: snapshot,
        content_digest: d("input bytes"),
        schema_fingerprint: d("schema"),
        row_count: 2,
        data_class: DataClass::TeamProtected,
    };
    let file = |path: &str| StagedFile {
        path: path.to_owned(),
        digest: d(path),
        bytes: 10,
    };
    let manifest = RWorkspaceManifest {
        header: h(&format!("r-workspace-{n}")),
        project_id: id("proj-1"),
        label: "labs".to_owned(),
        layout_version: R_WORKSPACE_LAYOUT_VERSION,
        stage_root: format!("/stage/labs-r-workspace-{n}"),
        staging_mode: StagingMode::CsvCopy,
        output_policy: OutputPolicy::ExplicitPublishOnly,
        data_class: DataClass::TeamProtected,
        class_basis: ClassBasis::InheritedFromInputs,
        inputs: vec![input],
        files: vec![
            file(README_FILE),
            file("data/snapshot-1.csv"),
            file("data/snapshot-1.schema.json"),
            file("labs.Rproj"),
        ],
    };
    RWorkspace {
        descriptor_digest: manifest.digest(),
        manifest,
    }
}

fn launch(ws: &RWorkspace, n: u32) -> RLaunchReceipt {
    RLaunchReceipt {
        header: h(&format!("r-launch-{n}")),
        workspace_id: ws.id().clone(),
        project_id: id("proj-1"),
        ide: IdeKind::Folder,
        program: Some("/usr/bin/xdg-open".to_owned()),
        descriptor_digest: ws.descriptor_digest.clone(),
        env_names: vec!["HOME".to_owned(), "PATH".to_owned()],
        state: LaunchState::Launched,
    }
}

fn run(ws: &RWorkspace, n: u32) -> RRunReceipt {
    RRunReceipt {
        header: h(&format!("r-run-{n}")),
        workspace_id: ws.id().clone(),
        project_id: id("proj-1"),
        descriptor_digest: ws.descriptor_digest.clone(),
        script: "analysis.R".to_owned(),
        script_digest: Some(d("print(1)")),
        runtime: RRuntimeIdentity {
            program: None,
            found: false,
            program_digest: None,
            version: RVersionEvidence::NotProbed,
        },
        lockfile: None,
        refusal: RunRefusal::ComputeDeniedPlatformUnqualified,
    }
}

fn refused(ws: &RWorkspace, n: u32) -> RPublishReceipt {
    RPublishReceipt {
        header: h(&format!("r-publish-{n}")),
        workspace_id: ws.id().clone(),
        project_id: id("proj-1"),
        descriptor_digest: ws.descriptor_digest.clone(),
        output_name: "missing.csv".to_owned(),
        source_digest: None,
        lockfile: None,
        state: PublishState::Refused,
        refusal: Some(PublishRefusal::NotFound),
        table_id: None,
        table_digest: None,
        row_count: 0,
        column_count: 0,
    }
}

fn doc() -> ResultTableDoc {
    ResultTableDoc {
        columns: vec![
            ResultColumn {
                name: "group".to_owned(),
                observed_type: "text".to_owned(),
            },
            ResultColumn {
                name: "mean".to_owned(),
                observed_type: "float".to_owned(),
            },
        ],
        rows: vec![vec![
            CellValue::Text("all".to_owned()),
            CellValue::Float(52.5),
        ]],
    }
}

fn published(ws: &RWorkspace, n: u32, t: u32) -> (RPublishReceipt, RPublishedTable) {
    let doc = doc();
    let table = RPublishedTable {
        header: h(&format!("r-table-{t}")),
        project_id: id("proj-1"),
        workspace_id: ws.id().clone(),
        receipt_id: id(&format!("r-publish-{n}")),
        output_name: "means.csv".to_owned(),
        source_digest: d("group,mean\nall,52.5\n"),
        content_digest: doc.digest(),
        row_count: 1,
        column_count: 2,
        derived_from: vec![id("snapshot-1")],
        data_class: DataClass::TeamProtected,
        reviewed: false,
    };
    let receipt = RPublishReceipt {
        output_name: "means.csv".to_owned(),
        source_digest: Some(table.source_digest.clone()),
        state: PublishState::Published,
        refusal: None,
        table_id: Some(table.header.id.clone()),
        table_digest: Some(table.content_digest.clone()),
        row_count: 1,
        column_count: 2,
        ..refused(ws, n)
    };
    (receipt, table)
}

fn build_state(meta: &SqliteMetaStore) {
    meta.insert_project(&project()).unwrap();
    let ws = workspace(1);
    meta.insert_r_workspace(&ws).unwrap();
    meta.insert_r_launch_receipt(&launch(&ws, 1)).unwrap();
    meta.insert_r_run_receipt(&run(&ws, 1)).unwrap();
    meta.insert_r_publication(&refused(&ws, 1), None).unwrap();
    let (r, t) = published(&ws, 2, 1);
    meta.insert_r_publication(&r, Some((&t, &doc()))).unwrap();
}

#[test]
fn migration_v14_to_v15_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        meta.insert_project(&project()).unwrap();
    }
    let conn = raw(&root);
    for table in R_WORKSPACE_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 15", [])
        .unwrap();
    drop(conn);
    assert!(!table_exists(&root, "rws_workspaces"));
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    for table in R_WORKSPACE_TABLES {
        assert!(table_exists(&root, table), "{table}");
    }
    assert!(meta.get_project(&id("proj-1")).is_ok());
    assert!(meta.list_all_compute_jobs().unwrap().is_empty());
    drop(meta);
    // Reopening is idempotent.
    let meta = open_meta(&root);
    assert!(meta.list_all_r_workspaces().unwrap().is_empty());
}

#[test]
fn r_workspace_rows_hold_their_invariants() {
    let root = temp_root("invariants");
    let meta = open_meta(&root);
    build_state(&meta);
    meta.verify_r_workspace_consistency().unwrap();
    let ws = workspace(1);

    assert!(matches!(
        meta.insert_r_workspace(&ws),
        Err(MetaError::Conflict(_))
    ));
    let mut forged = workspace(2);
    forged.manifest.label = "other".to_owned();
    assert!(
        meta.insert_r_workspace(&forged).is_err(),
        "descriptor digest must match"
    );

    // A receipt must describe its workspace.
    let mut stale = launch(&ws, 2);
    stale.descriptor_digest = d("other descriptor");
    assert!(meta.insert_r_launch_receipt(&stale).is_err());
    let mut orphan = run(&ws, 2);
    orphan.workspace_id = id("r-workspace-9");
    assert!(matches!(
        meta.insert_r_run_receipt(&orphan),
        Err(MetaError::NotFound)
    ));
    let mut moved = run(&ws, 3);
    moved.project_id = id("proj-2");
    assert!(meta.insert_r_run_receipt(&moved).is_err());
    let mut leaky = launch(&ws, 3);
    leaky.env_names.push("MEDSCALE_PASSPHRASE".to_owned());
    assert!(meta.insert_r_launch_receipt(&leaky).is_err());

    // A published receipt needs its exact table.
    let (r, t) = published(&ws, 3, 2);
    assert!(meta.insert_r_publication(&r, None).is_err());
    let mut weaker = t.clone();
    weaker.data_class = DataClass::Public;
    assert!(
        meta.insert_r_publication(&r, Some((&weaker, &doc())))
            .is_err()
    );
    let mut other_input = t.clone();
    other_input.derived_from = vec![id("snapshot-2")];
    assert!(
        meta.insert_r_publication(&r, Some((&other_input, &doc())))
            .is_err()
    );
    let mut reviewed = t.clone();
    reviewed.reviewed = true;
    assert!(
        meta.insert_r_publication(&r, Some((&reviewed, &doc())))
            .is_err()
    );
    let mut wrong_rows = doc();
    wrong_rows.rows.push(vec![CellValue::Null, CellValue::Null]);
    assert!(
        meta.insert_r_publication(&r, Some((&t, &wrong_rows)))
            .is_err()
    );

    // Atomic: a table that cannot be written leaves no receipt.
    let (r4, mut t4) = published(&ws, 4, 1);
    t4.receipt_id = id("r-publish-4");
    assert!(meta.insert_r_publication(&r4, Some((&t4, &doc()))).is_err());
    assert!(
        meta.list_r_publish_receipts(Some(ws.id()))
            .unwrap()
            .iter()
            .all(|p| p.header.id != id("r-publish-4"))
    );
    meta.verify_r_workspace_consistency().unwrap();
    assert_eq!(meta.list_r_workspaces(&id("proj-1")).unwrap().len(), 1);
    let (table, content) = meta.get_r_published_table(&id("r-table-1")).unwrap();
    assert_eq!(content, doc());
    assert!(!table.reviewed);
}

#[test]
fn tampered_r_workspace_rows_fail_closed_on_read() {
    let root = temp_root("rows");
    {
        let meta = open_meta(&root);
        build_state(&meta);
    }
    let conn = raw(&root);
    conn.execute(
        "UPDATE rws_workspaces SET project_id = 'proj-2' WHERE workspace_id = 'r-workspace-1'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE rws_published_tables SET content = X'00' WHERE table_id = 'r-table-1'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE rws_publish_receipts SET state = 'published' WHERE receipt_id = 'r-publish-1'",
        [],
    )
    .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert!(meta.get_r_workspace(&id("r-workspace-1")).is_err());
    assert!(meta.get_r_published_table(&id("r-table-1")).is_err());
    assert!(meta.list_r_publish_receipts(None).is_err());
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
fn backup_restore_round_trips_r_workspace_rows() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    let (a, b) = (&original.meta, &restored.meta);
    assert_eq!(
        a.list_all_r_workspaces().unwrap(),
        b.list_all_r_workspaces().unwrap()
    );
    assert_eq!(
        a.list_r_launch_receipts(None).unwrap(),
        b.list_r_launch_receipts(None).unwrap()
    );
    assert_eq!(
        a.list_r_run_receipts(None).unwrap(),
        b.list_r_run_receipts(None).unwrap()
    );
    assert_eq!(
        a.list_r_publish_receipts(None).unwrap(),
        b.list_r_publish_receipts(None).unwrap()
    );
    assert_eq!(
        a.list_all_r_published_tables().unwrap(),
        b.list_all_r_published_tables().unwrap()
    );
    b.verify_r_workspace_consistency().unwrap();
}

#[test]
fn tampered_r_workspace_backups_are_refused() {
    type Edit = fn(&mut serde_json::Value);
    let edits: Vec<(&str, Edit)> = vec![
        ("workspaces family missing", |s| {
            s.as_object_mut().unwrap().remove("rws_workspaces");
        }),
        ("tables family not an array", |s| {
            s["rws_published_tables"] = serde_json::json!({});
        }),
        ("workspace class weakened", |s| {
            rows(s, "rws_workspaces")[0]["manifest"]["data_class"] = serde_json::json!("public");
        }),
        ("workspace descriptor digest swapped", |s| {
            rows(s, "rws_workspaces")[0]["manifest"]["stage_root"] =
                serde_json::json!("/elsewhere");
        }),
        ("workspace removed under its receipts", |s| {
            rows(s, "rws_workspaces").remove(0);
        }),
        ("launch leaks a secret name", |s| {
            rows(s, "rws_launch_receipts")[0]["env_names"] =
                serde_json::json!(["MEDSCALE_PASSPHRASE"]);
        }),
        ("run claims execution", |s| {
            rows(s, "rws_run_receipts")[0]["refusal"] = serde_json::json!("completed");
        }),
        ("published receipt without its table", |s| {
            rows(s, "rws_published_tables").remove(0);
        }),
        ("table content edited", |s| {
            let hex = rows(s, "rws_published_tables")[0]["content_hex"]
                .as_str()
                .unwrap()
                .replace("35", "36");
            rows(s, "rws_published_tables")[0]["content_hex"] = serde_json::json!(hex);
        }),
        ("table content not hex", |s| {
            rows(s, "rws_published_tables")[0]["content_hex"] = serde_json::json!("7b\u{e9}");
        }),
        ("table marked reviewed", |s| {
            rows(s, "rws_published_tables")[0]["table"]["reviewed"] = serde_json::json!(true);
        }),
        ("table names another input", |s| {
            rows(s, "rws_published_tables")[0]["table"]["derived_from"] =
                serde_json::json!(["snapshot-2"]);
        }),
        ("refused receipt claims publication", |s| {
            rows(s, "rws_publish_receipts")[0]["state"] = serde_json::json!("published");
        }),
        ("duplicate workspace", |s| {
            let dup = rows(s, "rws_workspaces")[0].clone();
            rows(s, "rws_workspaces").push(dup);
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
fn pre_086_v14_backup_restores_with_empty_r_workspace_tables() {
    let root = temp_root("v14-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    vault.meta.insert_project(&project()).unwrap();
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    tamper_backup(&dest, |s| {
        let o = s.as_object_mut().unwrap();
        o.retain(|k, _| {
            !k.starts_with("rws_")
                && !k.starts_with("ext_")
                && !k.starts_with("hud_")
                && !k.starts_with("rp_")
                && !k.starts_with("ia_")
                && !k.starts_with("fed_")
        });
        o.insert("schema_version".to_owned(), serde_json::json!(14));
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.list_all_r_workspaces().unwrap().is_empty());
    assert!(restored.meta.get_project(&id("proj-1")).is_ok());
}

//! Spec 082 Analytics Gate storage + migration integration tests.
//!
//! Synthetic data only. Additive v10 -> v11 migration, crash recovery,
//! atomic receipt/result commits, digest checks on read, cross-row
//! invariants and backup/restore (including tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::analytics::{
    ANALYTICS_SCHEMA_VERSION, CohortCriterion, CohortDefinition, CohortOp, DerivedTable,
    EngineIdentity, InputReproducibility, PinnedInput, QueryDenyReason, QueryOrigin, QueryOutcome,
    QueryReceipt, ResultColumn, ResultTableDoc,
};
use medscale_contracts::data_sources::CellValue;
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, MetaError, SqliteMetaStore, SyntheticVault, backup_vault,
    restore_vault,
};

const ANALYTICS_TABLES: [&str; 3] = [
    "analytics_receipts",
    "analytics_results",
    "analytics_cohorts",
];

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-082-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
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
        schema_version: ANALYTICS_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn populate_pre_082(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
}

fn rewind_to_v10(root: &Path) {
    let conn = raw(root);
    for table in ANALYTICS_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    // Later additive versions (Spec 083 v12, ...) are absent in a v10 build too.
    for table in [
        "knowledge_manifests",
        "knowledge_chunks",
        "knowledge_receipts",
        "knowledge_canvases",
        "hub_identity",
        "hub_invitations",
        "hub_devices",
        "hub_nonces",
        "hub_events",
        "hub_links",
        "hub_link_secrets",
        "hub_outbox",
        "hub_mirror",
        "compute_jobs",
        "compute_receipts",
        "compute_outputs",
        "rws_workspaces",
        "rws_launch_receipts",
        "rws_run_receipts",
        "rws_publish_receipts",
        "rws_published_tables",
        "ext_publishers",
        "ext_releases",
        "ext_installs",
        "ext_grants",
        "ext_lifecycle_receipts",
        "ext_runtime_receipts",
        "hud_huddles",
        "hud_participants",
        "hud_media",
        "hud_proposals",
        "hud_receipts",
        "rp_installs",
        "rp_artifacts",
        "rp_receipts",
        "ia_adapters",
        "ia_intents",
        "ia_receipts",
        "fed_identity",
        "fed_peers",
        "fed_imported",
        "fed_receipts",
    ] {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {table};"))
            .unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 11", [])
        .unwrap();
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

fn pre_082_view(meta: &SqliteMetaStore) -> serde_json::Value {
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&meta.snapshot_bytes().unwrap()).unwrap();
    let object = snapshot.as_object_mut().unwrap();
    object.remove("schema_version");
    // Id sequences are allocation counters, not rows; their backup and
    // restore are covered by `id_sequences_restore`.
    object.remove("id_sequences");
    object.retain(|key, _| {
        !key.starts_with("analytics_")
            && !key.starts_with("knowledge_")
            && !key.starts_with("hub_")
            && !key.starts_with("compute_")
            && !key.starts_with("rws_")
            && !key.starts_with("ext_")
            && !key.starts_with("hud_")
            && !key.starts_with("rp_")
            && !key.starts_with("ia_")
            && !key.starts_with("fed_")
    });
    snapshot
}

fn doc() -> ResultTableDoc {
    ResultTableDoc {
        columns: vec![ResultColumn {
            name: "n".to_owned(),
            observed_type: "integer".to_owned(),
        }],
        rows: vec![vec![CellValue::Integer(4)], vec![CellValue::Integer(7)]],
    }
}

fn receipt(rid: &str, outcome: QueryOutcome) -> QueryReceipt {
    let sql = "SELECT n FROM t".to_owned();
    QueryReceipt {
        header: h(rid),
        project_id: id("proj-1"),
        origin: QueryOrigin::SqlEditor,
        sql_digest: DigestSha256::of(sql.as_bytes()),
        sql,
        inputs: vec![PinnedInput {
            alias: "t".to_owned(),
            snapshot_id: id("snap-1"),
            content_digest: DigestSha256::of(b"snap"),
            schema_fingerprint: DigestSha256::of(b"schema"),
            row_count: 2,
            complete: true,
        }],
        reproducibility: InputReproducibility::Exact,
        engine: EngineIdentity {
            engine: "sqlite".to_owned(),
            version: "3".to_owned(),
            max_rows: 1_000,
            timeout_ms: 10_000,
        },
        outcome,
        deny_reason: (outcome == QueryOutcome::Denied).then_some(QueryDenyReason::NotReadOnly),
        failure: None,
        result_id: None,
        result_digest: None,
        row_count: 0,
        column_count: 0,
        cohort_id: None,
    }
}

fn completed(rid: &str, tid: &str) -> (QueryReceipt, DerivedTable, ResultTableDoc) {
    let d = doc();
    let mut r = receipt(rid, QueryOutcome::Completed);
    r.result_id = Some(id(tid));
    r.result_digest = Some(d.digest());
    r.row_count = 2;
    r.column_count = 1;
    let t = DerivedTable {
        header: h(tid),
        project_id: id("proj-1"),
        receipt_id: id(rid),
        content_digest: d.digest(),
        row_count: 2,
        column_count: 1,
        derived_from: vec![id("snap-1")],
        truncated: false,
    };
    (r, t, d)
}

fn cohort(cid: &str) -> CohortDefinition {
    CohortDefinition {
        header: h(cid),
        project_id: id("proj-1"),
        label: "adults".to_owned(),
        snapshot_id: id("snap-1"),
        criteria: vec![CohortCriterion {
            field: "age".to_owned(),
            op: CohortOp::Ge,
            value: Some(CellValue::Integer(18)),
        }],
    }
}

fn build_state(meta: &SqliteMetaStore) {
    let (r, t, d) = completed("r1", "t1");
    meta.commit_query(&r, Some((&t, &d))).unwrap();
    meta.commit_query(&receipt("r2", QueryOutcome::Denied), None)
        .unwrap();
    meta.insert_cohort(&cohort("c1")).unwrap();
    let (mut r3, mut t3, d3) = completed("r3", "t3");
    r3.origin = QueryOrigin::CohortBuilder;
    r3.cohort_id = Some(id("c1"));
    t3.receipt_id = id("r3");
    meta.commit_query(&r3, Some((&t3, &d3))).unwrap();
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
    populate_pre_082(&vault.meta);
    build_state(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    dest
}

#[test]
fn migration_v10_to_v11_is_additive() {
    let root = temp_root("migrate");
    let before = {
        let meta = open_meta(&root);
        populate_pre_082(&meta);
        pre_082_view(&meta)
    };
    rewind_to_v10(&root);
    for t in ANALYTICS_TABLES {
        assert!(!table_exists(&root, t));
    }
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    for t in ANALYTICS_TABLES {
        assert!(table_exists(&root, t), "{t}");
    }
    assert_eq!(pre_082_view(&meta), before);
}

#[test]
fn crash_mid_v11_migration_fails_closed_and_backup_recovers() {
    let root = temp_root("crash");
    let vault_root = root.join("vault");
    {
        let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
        populate_pre_082(&vault.meta);
        backup_vault(&vault, &root.join("checkpoint")).unwrap();
    }
    rewind_to_v10(&vault_root);
    raw(&vault_root)
        .execute(
            "INSERT INTO migration_journal(version, state) VALUES (11, 'started')",
            [],
        )
        .unwrap();
    match SqliteMetaStore::open_at(&db_path(&vault_root)) {
        Err(MetaError::MigrationIncomplete(11)) => {}
        other => panic!("interrupted v11 migration must fail closed, got {other:?}"),
    }
    restore_vault(&root.join("checkpoint"), &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.get_project(&id("proj-1")).is_ok());
}

#[test]
fn receipts_and_results_commit_atomically_and_are_digest_checked() {
    let root = temp_root("atomic");
    let meta = open_meta(&root);
    populate_pre_082(&meta);
    let (r, t, d) = completed("r1", "t1");
    let mut wrong_digest = t.clone();
    wrong_digest.content_digest = DigestSha256::of(b"other");
    assert!(meta.commit_query(&r, Some((&wrong_digest, &d))).is_err());
    assert!(matches!(
        meta.get_query_receipt(&id("r1")),
        Err(MetaError::NotFound)
    ));
    assert!(
        meta.commit_query(&r, None).is_err(),
        "a completed receipt needs its table"
    );
    let mut bad = receipt("r9", QueryOutcome::Denied);
    bad.deny_reason = None;
    assert!(meta.commit_query(&bad, None).is_err(), "invalid receipt");

    meta.commit_query(&r, Some((&t, &d))).unwrap();
    assert!(matches!(
        meta.commit_query(&r, Some((&t, &d))),
        Err(MetaError::Conflict(_))
    ));
    assert_eq!(meta.get_query_receipt(&id("r1")).unwrap(), r);
    let (stored, stored_doc) = meta.get_derived_table(&id("t1")).unwrap();
    assert_eq!(stored, t);
    assert_eq!(stored_doc, d);

    raw(&root)
        .execute(
            "UPDATE analytics_results SET content = ?1 WHERE result_id = 't1'",
            [br#"{"columns":[],"rows":[]}"#.to_vec()],
        )
        .unwrap();
    assert!(matches!(
        meta.get_derived_table(&id("t1")),
        Err(MetaError::CorruptObjectBody(_))
    ));
}

#[test]
fn cohorts_are_validated_and_listed_per_project() {
    let root = temp_root("cohorts");
    let meta = open_meta(&root);
    populate_pre_082(&meta);
    let mut bad = cohort("c0");
    bad.criteria.clear();
    assert!(meta.insert_cohort(&bad).is_err());
    meta.insert_cohort(&cohort("c1")).unwrap();
    assert!(matches!(
        meta.insert_cohort(&cohort("c1")),
        Err(MetaError::Conflict(_))
    ));
    assert_eq!(meta.list_cohorts(&id("proj-1")).unwrap().len(), 1);
    assert!(meta.list_cohorts(&id("proj-2")).unwrap().is_empty());
}

#[test]
fn consistency_check_detects_invariant_breaks() {
    let root = temp_root("consistency");
    let meta = open_meta(&root);
    populate_pre_082(&meta);
    build_state(&meta);
    meta.verify_analytics_consistency().unwrap();
    raw(&root)
        .execute("DELETE FROM analytics_cohorts WHERE cohort_id = 'c1'", [])
        .unwrap();
    assert!(
        meta.verify_analytics_consistency().is_err(),
        "a cohort query names its cohort"
    );
    meta.insert_cohort(&cohort("c1")).unwrap();
    meta.verify_analytics_consistency().unwrap();
    raw(&root)
        .execute("DELETE FROM analytics_results WHERE result_id = 't1'", [])
        .unwrap();
    assert!(
        meta.verify_analytics_consistency().is_err(),
        "a completed receipt needs its table"
    );
}

#[test]
fn backup_restore_roundtrips_every_082_row_exactly() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        original.meta.list_all_query_receipts().unwrap(),
        restored.meta.list_all_query_receipts().unwrap()
    );
    assert_eq!(
        original.meta.list_all_derived_tables().unwrap(),
        restored.meta.list_all_derived_tables().unwrap()
    );
    assert_eq!(
        original.meta.list_all_cohorts().unwrap(),
        restored.meta.list_all_cohorts().unwrap()
    );
    restored.meta.verify_analytics_consistency().unwrap();
}

#[test]
fn restore_rejects_hand_edited_082_snapshots() {
    type Edit = (&'static str, fn(&mut serde_json::Value));
    let edits: [Edit; 9] = [
        ("result bytes changed", |s| {
            rows(s, "analytics_results")[0]["content_hex"] = serde_json::json!("00");
        }),
        // Four bytes whose first pair splits a UTF-8 character: this must be
        // refused as corrupt, never reach a panicking slice.
        ("result hex not ASCII", |s| {
            rows(s, "analytics_results")[0]["content_hex"] = serde_json::json!("a\u{e9}b");
        }),
        ("result hex with signs", |s| {
            rows(s, "analytics_results")[0]["content_hex"] = serde_json::json!("+f+f");
        }),
        ("receipt SQL edited", |s| {
            rows(s, "analytics_receipts")[0]["sql"] = serde_json::json!("SELECT 2 FROM t");
        }),
        ("result without receipt", |s| {
            rows(s, "analytics_receipts").remove(0);
        }),
        ("receipt names another result digest", |s| {
            let other = rows(s, "analytics_receipts")[0]["sql_digest"].clone();
            rows(s, "analytics_receipts")[0]["result_digest"] = other;
        }),
        ("cohort removed", |s| {
            rows(s, "analytics_cohorts").clear();
        }),
        ("receipt in another scope", |s| {
            rows(s, "analytics_receipts")[1]["header"]["authority_scope_id"] =
                serde_json::json!("scope-2");
        }),
        ("duplicate cohort", |s| {
            let first = rows(s, "analytics_cohorts")[0].clone();
            rows(s, "analytics_cohorts").push(first);
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
fn pre_082_v10_backup_restores_with_empty_analytics_tables() {
    let root = temp_root("v10-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_pre_082(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    tamper_backup(&dest, |s| {
        let o = s.as_object_mut().unwrap();
        o.retain(|k, _| {
            !k.starts_with("analytics_")
                && !k.starts_with("knowledge_")
                && !k.starts_with("hub_")
                && !k.starts_with("compute_")
                && !k.starts_with("rws_")
                && !k.starts_with("ext_")
                && !k.starts_with("hud_")
                && !k.starts_with("rp_")
                && !k.starts_with("ia_")
                && !k.starts_with("fed_")
        });
        o.insert("schema_version".to_owned(), serde_json::json!(10));
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.list_all_query_receipts().unwrap().is_empty());
    restored.meta.verify_analytics_consistency().unwrap();
}

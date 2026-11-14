//! Spec 085 MedScale Compute storage + migration integration tests.
//!
//! Synthetic rows only. Additive v13 -> v14 migration, single-claim state
//! transitions, atomic terminal commits, cross-row invariants and
//! backup/restore (including tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::analytics::{ResultColumn, ResultTableDoc};
use medscale_contracts::compute::{
    COMPUTE_SCHEMA_VERSION, ComputeFailure, ComputeJob, ComputeJobKind, ComputeManifest,
    ComputeOutput, ComputeParams, ComputeReceipt, ComputeState, ExecutionPolicy, OutputReview,
    ResourceCeilings, RuntimeIdentity, SandboxMechanism, SandboxReport, SandboxRequirement,
    StagedInput,
};
use medscale_contracts::data_sources::CellValue;
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    COMPUTE_TABLES, CURRENT_META_SCHEMA_VERSION, MetaError, SqliteMetaStore, SyntheticVault,
    backup_vault, restore_vault,
};

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-085s-{name}-{}", std::process::id()));
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
        schema_version: COMPUTE_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn project() -> Project {
    Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap()
}

fn input() -> StagedInput {
    StagedInput {
        snapshot_id: id("snapshot-1"),
        content_digest: DigestSha256::of(b"input bytes"),
        schema_fingerprint: DigestSha256::of(b"schema"),
        row_count: 2,
        byte_len: 100,
    }
}

fn job(n: u32) -> ComputeJob {
    let kind = ComputeJobKind::ColumnProfile;
    let manifest = ComputeManifest {
        job_id: id(&format!("compute-job-{n}")),
        project_id: id("proj-1"),
        kind,
        params: ComputeParams::ColumnProfile {},
        requested_snapshot_id: id("snapshot-1"),
        input: Some(input()),
        runtime: RuntimeIdentity::current(kind),
        policy: ExecutionPolicy::fixed(SandboxRequirement::ReadyBaseRequired),
        limits: ResourceCeilings::default(),
    };
    ComputeJob {
        header: h(&format!("compute-job-{n}")),
        manifest_digest: manifest.digest(),
        manifest,
        state: ComputeState::Queued,
    }
}

fn table() -> ResultTableDoc {
    ResultTableDoc {
        columns: vec![ResultColumn {
            name: "n".to_owned(),
            observed_type: "integer".to_owned(),
        }],
        rows: vec![vec![CellValue::Integer(2)]],
    }
}

fn sandbox() -> SandboxReport {
    SandboxReport {
        mechanism: SandboxMechanism::LinuxLandlockComposition,
        ready_base_applied: true,
        platform_qualified: false,
        env_var_count: 0,
    }
}

fn receipt(job: &ComputeJob, n: u32, failure: Option<ComputeFailure>) -> ComputeReceipt {
    let completed = failure.is_none();
    let t = table();
    ComputeReceipt {
        header: h(&format!("compute-receipt-{n}")),
        job_id: job.header.id.clone(),
        project_id: job.manifest.project_id.clone(),
        kind: job.manifest.kind,
        manifest_digest: job.manifest_digest.clone(),
        input: job.manifest.input.clone(),
        runtime: job.manifest.runtime.clone(),
        sandbox: completed.then(sandbox),
        state: failure.map_or(ComputeState::Completed, ComputeFailure::state),
        deny_reason: None,
        failure,
        output_id: completed.then(|| id(&format!("compute-output-{n}"))),
        output_digest: completed.then(|| t.digest()),
        row_count: u64::from(completed),
        column_count: u32::from(completed),
        stdout_bytes: 0,
        stderr_bytes: 0,
        stderr_prefix_digest: None,
    }
}

fn output(r: &ComputeReceipt) -> ComputeOutput {
    ComputeOutput {
        header: h(r.output_id.as_ref().unwrap().as_str()),
        project_id: r.project_id.clone(),
        job_id: r.job_id.clone(),
        receipt_id: r.header.id.clone(),
        kind: r.kind,
        content_digest: table().digest(),
        row_count: 1,
        column_count: 1,
        derived_from: id("snapshot-1"),
        input_digest: input().content_digest,
        review: OutputReview::Unreviewed,
    }
}

/// Queued job 1, running job 2, completed job 3 (with output), cancelled
/// job 4.
fn build_state(meta: &SqliteMetaStore) {
    meta.insert_project(&project()).unwrap();
    for n in 1..=4 {
        meta.insert_compute_job(&job(n), None).unwrap();
    }
    meta.claim_compute_job(&id("compute-job-2")).unwrap();
    let running3 = meta.claim_compute_job(&id("compute-job-3")).unwrap();
    let r3 = receipt(&running3, 3, None);
    meta.finish_compute_job(
        &id("compute-job-3"),
        ComputeState::Running,
        &r3,
        Some((&output(&r3), &table())),
    )
    .unwrap();
    let r4 = receipt(&job(4), 4, Some(ComputeFailure::CancelledByUser));
    meta.finish_compute_job(&id("compute-job-4"), ComputeState::Queued, &r4, None)
        .unwrap();
}

#[test]
fn migration_v13_to_v14_is_additive() {
    let root = temp_root("migrate");
    {
        let meta = open_meta(&root);
        meta.insert_project(&project()).unwrap();
    }
    let conn = raw(&root);
    for table in COMPUTE_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 14", [])
        .unwrap();
    drop(conn);
    assert!(!table_exists(&root, "compute_jobs"));
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    for table in COMPUTE_TABLES {
        assert!(table_exists(&root, table), "{table}");
    }
    assert!(meta.get_project(&id("proj-1")).is_ok());
    drop(meta);
    // Reopening is idempotent.
    let meta = open_meta(&root);
    assert!(meta.list_all_compute_jobs().unwrap().is_empty());
}

#[test]
fn compute_rows_hold_their_invariants() {
    let root = temp_root("invariants");
    let meta = open_meta(&root);
    build_state(&meta);
    meta.verify_compute_consistency().unwrap();

    // Ids are unique; a new job is queued, or terminal with its receipt.
    assert!(matches!(
        meta.insert_compute_job(&job(1), None),
        Err(MetaError::Conflict(_))
    ));
    let mut terminal = job(5);
    terminal.state = ComputeState::Cancelled;
    assert!(meta.insert_compute_job(&terminal, None).is_err());
    let mut forged = job(6);
    forged.manifest.limits.timeout_ms = 1;
    assert!(meta.insert_compute_job(&forged, None).is_err());

    // A job is claimed at most once, and never after it finished.
    assert!(matches!(
        meta.claim_compute_job(&id("compute-job-2")),
        Err(MetaError::Conflict(_))
    ));
    assert!(matches!(
        meta.claim_compute_job(&id("compute-job-3")),
        Err(MetaError::Conflict(_))
    ));
    // Finishing from the wrong state writes nothing.
    let r = receipt(&job(1), 10, Some(ComputeFailure::TimeLimit));
    assert!(matches!(
        meta.finish_compute_job(&id("compute-job-1"), ComputeState::Running, &r, None),
        Err(MetaError::Conflict(_))
    ));
    assert!(
        meta.get_compute_receipt_for_job(&id("compute-job-1"))
            .unwrap()
            .is_none()
    );
    // A second terminal commit for a finished job is refused.
    let again = receipt(&job(4), 11, Some(ComputeFailure::CancelledByUser));
    assert!(
        meta.finish_compute_job(&id("compute-job-4"), ComputeState::Queued, &again, None)
            .is_err()
    );
    // A completed receipt needs its output, with matching bytes.
    let running = meta.get_compute_job(&id("compute-job-2")).unwrap();
    let r2 = receipt(&running, 12, None);
    assert!(
        meta.finish_compute_job(&id("compute-job-2"), ComputeState::Running, &r2, None)
            .is_err()
    );
    let mut wrong = table();
    wrong.rows[0][0] = CellValue::Integer(3);
    assert!(
        meta.finish_compute_job(
            &id("compute-job-2"),
            ComputeState::Running,
            &r2,
            Some((&output(&r2), &wrong))
        )
        .is_err()
    );
    // A receipt for another job is refused.
    let other = receipt(&job(1), 13, Some(ComputeFailure::TimeLimit));
    assert!(
        meta.finish_compute_job(&id("compute-job-2"), ComputeState::Running, &other, None)
            .is_err()
    );
    assert_eq!(
        meta.get_compute_job(&id("compute-job-2")).unwrap().state,
        ComputeState::Running
    );
    meta.verify_compute_consistency().unwrap();

    let (out, doc) = meta.get_compute_output(&id("compute-output-3")).unwrap();
    assert_eq!(doc, table());
    assert_eq!(out.receipt_id, id("compute-receipt-3"));
    assert_eq!(
        meta.list_compute_jobs_in_state(ComputeState::Running)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(meta.list_compute_jobs(&id("proj-1")).unwrap().len(), 4);

    // Edited rows fail closed on read.
    drop(meta);
    let conn = raw(&root);
    conn.execute(
        "UPDATE compute_jobs SET state = 'completed' WHERE job_id = 'compute-job-1'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE compute_outputs SET content = X'00' WHERE output_id = 'compute-output-3'",
        [],
    )
    .unwrap();
    drop(conn);
    let meta = open_meta(&root);
    assert!(meta.get_compute_job(&id("compute-job-1")).is_err());
    assert!(meta.get_compute_output(&id("compute-output-3")).is_err());
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
fn backup_restore_round_trips_compute_rows() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        original.meta.list_all_compute_jobs().unwrap(),
        restored.meta.list_all_compute_jobs().unwrap()
    );
    assert_eq!(
        original.meta.list_all_compute_receipts().unwrap(),
        restored.meta.list_all_compute_receipts().unwrap()
    );
    assert_eq!(
        original.meta.list_all_compute_outputs().unwrap(),
        restored.meta.list_all_compute_outputs().unwrap()
    );
    restored.meta.verify_compute_consistency().unwrap();
    // The job backed up while `running` is still `running`; Core recovers
    // it as `interrupted` and never re-runs it.
    assert_eq!(
        restored
            .meta
            .get_compute_job(&id("compute-job-2"))
            .unwrap()
            .state,
        ComputeState::Running
    );
}

#[test]
fn tampered_compute_backups_are_refused() {
    type Edit = fn(&mut serde_json::Value);
    let edits: Vec<(&str, Edit)> = vec![
        ("compute jobs family missing", |s| {
            s.as_object_mut().unwrap().remove("compute_jobs");
        }),
        ("compute outputs family not an array", |s| {
            s["compute_outputs"] = serde_json::json!({});
        }),
        ("completed job set back to queued", |s| {
            rows(s, "compute_jobs")[2]["state"] = serde_json::json!("queued");
        }),
        ("queued job claims completion", |s| {
            rows(s, "compute_jobs")[0]["state"] = serde_json::json!("completed");
        }),
        ("manifest edited", |s| {
            rows(s, "compute_jobs")[0]["manifest"]["limits"]["timeout_ms"] =
                serde_json::json!(600_000);
        }),
        ("receipt removed", |s| {
            rows(s, "compute_receipts").remove(0);
        }),
        ("receipt moved to another job", |s| {
            rows(s, "compute_receipts")[1]["job_id"] = serde_json::json!("compute-job-1");
        }),
        ("receipt claims platform qualification", |s| {
            rows(s, "compute_receipts")[0]["sandbox"]["platform_qualified"] =
                serde_json::json!(true);
        }),
        ("output removed", |s| {
            rows(s, "compute_outputs").remove(0);
        }),
        ("output content edited", |s| {
            let hex = rows(s, "compute_outputs")[0]["content_hex"]
                .as_str()
                .unwrap()
                .replace("32", "33");
            rows(s, "compute_outputs")[0]["content_hex"] = serde_json::json!(hex);
        }),
        ("output content not hex", |s| {
            rows(s, "compute_outputs")[0]["content_hex"] = serde_json::json!("7b\u{e9}");
        }),
        ("output names another snapshot", |s| {
            rows(s, "compute_outputs")[0]["output"]["derived_from"] =
                serde_json::json!("snapshot-2");
        }),
        ("duplicate job", |s| {
            let dup = rows(s, "compute_jobs")[0].clone();
            rows(s, "compute_jobs").push(dup);
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
fn pre_085_v13_backup_restores_with_empty_compute_tables() {
    let root = temp_root("v13-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    vault.meta.insert_project(&project()).unwrap();
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    tamper_backup(&dest, |s| {
        let o = s.as_object_mut().unwrap();
        o.retain(|k, _| {
            !k.starts_with("compute_")
                && !k.starts_with("rws_")
                && !k.starts_with("ext_")
                && !k.starts_with("hud_")
                && !k.starts_with("rp_")
                && !k.starts_with("ia_")
                && !k.starts_with("fed_")
        });
        o.insert("schema_version".to_owned(), serde_json::json!(13));
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.list_all_compute_jobs().unwrap().is_empty());
    assert!(restored.meta.get_project(&id("proj-1")).is_ok());
}

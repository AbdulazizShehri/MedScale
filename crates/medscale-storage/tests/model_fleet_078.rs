//! Spec 078 Model Fleet + Compare storage + migration integration tests
//! (T078-02).
//!
//! Synthetic data only. Covers `migration.md` sections 5, 6, 7, 8, 9, 11 and
//! 12, and `security.md` T3/T4, against a genuinely pre-078 (schema v6)
//! vault: the populated fixture is built with real 074/075/076/077 rows, the
//! v7 tables and journal entry are then removed so the file is exactly what
//! a v6 build leaves on disk, and only then is the v7 migration applied.

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::collaboration::{ParticipantIdentity, ParticipantKind};
use medscale_contracts::data_sources::{
    DATA_SOURCE_SCHEMA_VERSION, DataSourceCapability, DataSourceKind, DataSourceManifest,
    LocalFileFormat, SourceLocator,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::medagent::{
    AgentCapabilityManifest, AgentIdentity, AgentProposal, AgentRun, AgentRunState,
    ContextManifest, MEDAGENT_SCHEMA_VERSION, RunReceipt, ToolKind,
};
use medscale_contracts::model_fleet::{
    AgentLane, AgentLaneStatus, ComparisonObservation, ComparisonObservationKind, ComparisonReport,
    FleetRun, FleetRunState, LanePolicy, LaneRunRef, MODEL_FLEET_SCHEMA_VERSION,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, Project,
};
use medscale_storage::{
    MetaError, SourceMeta, SqliteMetaStore, SyntheticVault, backup_vault, restore_vault,
};

/// The real admitted Pack the Spec 077 fixture identity binds to
/// (`migration.md` section 7).
const ADMITTED_PACK_ID: &str = "pack-tiny-token-classifier-v0";

const SOURCE_BYTES: &[u8] = b"synthetic source bytes";

const MODEL_FLEET_TABLES: [&str; 4] = [
    "model_fleet_lanes",
    "model_fleet_runs",
    "model_fleet_lane_run_refs",
    "model_fleet_comparison_reports",
];

// ---------------------------------------------------------------------------
// fixture helpers
// ---------------------------------------------------------------------------

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-078-{name}-{}", std::process::id()));
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

fn header_v(id: &str, schema_version: u32) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn header(id: &str) -> ObjectHeader {
    header_v(id, MEDAGENT_SCHEMA_VERSION)
}

fn fleet_header(id: &str) -> ObjectHeader {
    header_v(id, MODEL_FLEET_SCHEMA_VERSION)
}

fn id(value: &str) -> OpaqueId {
    OpaqueId::new(value)
}

fn identity(agent_id: &str) -> AgentIdentity {
    AgentIdentity::new(
        header(agent_id),
        id("proj-1"),
        id(ADMITTED_PACK_ID),
        "0.1.0".to_owned(),
        format!("agent-{agent_id}"),
    )
    .unwrap()
}

fn capabilities(agent_id: &str) -> AgentCapabilityManifest {
    AgentCapabilityManifest::new(
        id(agent_id),
        vec![
            ToolKind::ReadContextArtifact,
            ToolKind::SearchContextArtifacts,
        ],
    )
    .unwrap()
}

fn context_manifest(ctx_id: &str, artifact_id: &str) -> ContextManifest {
    ContextManifest::new(
        header(ctx_id),
        id("proj-1"),
        vec![ArtifactDescriptor {
            object_id: id(artifact_id),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        }],
    )
    .unwrap()
}

fn agent_run(run_id: &str, agent_id: &str, ctx_id: &str) -> AgentRun {
    AgentRun::new(
        header(run_id),
        id("proj-1"),
        id(agent_id),
        id(ctx_id),
        "summarize the bound context".to_owned(),
    )
    .unwrap()
}

fn run_receipt(run_id: &str, ctx_id: &str, final_state: AgentRunState) -> RunReceipt {
    let receipt = RunReceipt {
        header: header(&format!("receipt-{run_id}")),
        run_id: id(run_id),
        pack_id: id(ADMITTED_PACK_ID),
        pack_version: "0.1.0".to_owned(),
        context_manifest_id: id(ctx_id),
        context_manifest_revision: 1,
        tool_invocation_ids: Vec::new(),
        final_state,
        failure_reason: (final_state == AgentRunState::Failed)
            .then(|| "synthetic failure".to_owned()),
    };
    receipt.validate().unwrap();
    receipt
}

/// Drives an already-inserted `Pending` run to `final_state` through Spec
/// 077's own unmodified storage transitions.
fn finish_run(meta: &SqliteMetaStore, run_id: &str, ctx_id: &str, final_state: AgentRunState) {
    meta.set_agent_run_running(&id(run_id), 1).unwrap();
    meta.commit_terminal_transition_with_receipt(
        &id(run_id),
        2,
        final_state,
        &run_receipt(run_id, ctx_id, final_state),
    )
    .unwrap();
}

fn lane(lane_id: &str, agent_id: &str, ctx_id: &str) -> AgentLane {
    AgentLane::new(
        fleet_header(lane_id),
        id("proj-1"),
        id(agent_id),
        id(ctx_id),
        format!("role-{lane_id}"),
        LanePolicy::new(Some(vec![ToolKind::ReadContextArtifact]), None).unwrap(),
    )
    .unwrap()
}

fn fleet_run(fleet_id: &str) -> FleetRun {
    FleetRun::new(
        fleet_header(fleet_id),
        id("proj-1"),
        "compare both lanes".to_owned(),
    )
    .unwrap()
}

fn lane_run_ref(ref_id: &str, fleet_id: &str, lane_id: &str, run_id: &str) -> LaneRunRef {
    LaneRunRef {
        header: fleet_header(ref_id),
        fleet_run_id: id(fleet_id),
        agent_lane_id: id(lane_id),
        agent_run_id: id(run_id),
    }
}

fn partial_failure_report(report_id: &str) -> ComparisonReport {
    let report = ComparisonReport {
        header: fleet_header(report_id),
        fleet_run_id: id("fleet-1"),
        observations: vec![ComparisonObservation {
            kind: ComparisonObservationKind::SchemaValidity,
            participating_lane_ids: vec![id("lane-1")],
            detail: "lane-1 proposal parsed against the frozen schema".to_owned(),
            evidence_refs: vec![id("lrun-1")],
        }],
        participating_lane_ids: vec![id("lane-1")],
        excluded_lane_ids: vec![id("lane-2")],
    };
    report.validate().unwrap();
    report
}

/// Populated pre-078 content (`migration.md` section 7): a source binding,
/// a 074 Project, a 075 data source, a 076 agent participant, and a 077
/// identity bound to the real admitted Pack with a context manifest and a
/// completed run carrying its RunReceipt and AgentProposal. A second 077
/// identity/context exists so a two-lane fleet can bind two distinct
/// pre-existing identities.
fn populate_pre_078(meta: &SqliteMetaStore) {
    meta.insert_source(&SourceMeta {
        source_id: id("src-1"),
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
        digest: DigestSha256::of(SOURCE_BYTES),
        byte_length: SOURCE_BYTES.len() as u64,
        media_type: "application/fhir+json".to_owned(),
        visible: true,
        resource_type: "Patient".to_owned(),
    })
    .unwrap();
    meta.insert_project(&Project::new(header("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
    meta.insert_data_source(
        &DataSourceManifest::new(
            header_v("dsrc-1", DATA_SOURCE_SCHEMA_VERSION),
            id("proj-1"),
            DataSourceKind::LocalTabularFile,
            "source-dsrc-1".to_owned(),
            "csv".to_owned(),
            SourceLocator::LocalPath {
                path: "fixtures/table.csv".to_owned(),
                format: LocalFileFormat::Csv,
            },
            None,
            vec![
                DataSourceCapability::DiscoverSchema,
                DataSourceCapability::ImportSnapshot,
            ],
        )
        .unwrap(),
    )
    .unwrap();
    meta.insert_participant(
        &ParticipantIdentity::new(
            header("participant-1"),
            id("holder-1"),
            ParticipantKind::Agent,
            "agent-participant-1".to_owned(),
        )
        .unwrap(),
        None,
    )
    .unwrap();
    for (agent_id, ctx_id, artifact_id) in [
        ("agent-1", "ctx-1", "artifact-1"),
        ("agent-2", "ctx-2", "artifact-2"),
    ] {
        meta.insert_agent_identity_with_capabilities(&identity(agent_id), &capabilities(agent_id))
            .unwrap();
        meta.insert_context_manifest(&context_manifest(ctx_id, artifact_id))
            .unwrap();
    }
    meta.insert_agent_run(&agent_run("run-1", "agent-1", "ctx-1"))
        .unwrap();
    finish_run(meta, "run-1", "ctx-1", AgentRunState::Completed);
    meta.insert_agent_proposal(&AgentProposal {
        header: header("agent-proposal-1"),
        run_id: id("run-1"),
        proposal_id: id("proposal-1"),
    })
    .unwrap();
}

/// `populate_pre_078` for a full vault: the source row's blob is stored
/// too, as the backup mechanism requires every bound blob to exist.
fn populate_vault(vault: &SyntheticVault) {
    vault.blobs.put_blob(SOURCE_BYTES).unwrap();
    populate_pre_078(&vault.meta);
}

/// Rewinds a v7 file to exactly what a v6 build leaves on disk: the v7
/// tables (and their indexes) do not exist and the journal ends at v6.
fn rewind_to_v6(root: &Path) {
    let conn = raw(root);
    for table in MODEL_FLEET_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    // Later additive versions (Spec 079 v8, ...) are also absent in a v6
    // build, so their tables and journal rows go too.
    for table in LATER_VERSION_TABLES {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {table};"))
            .unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 7", [])
        .unwrap();
}

/// Tables created by migrations after v7.
const LATER_VERSION_TABLES: &[&str] = &[
    "privacy_classifications",
    "privacy_profiles",
    "privacy_deid_receipts",
    "privacy_pseudonym_maps",
    "privacy_pseudonym_entries",
    "privacy_reid_audit",
    "privacy_egress_decisions",
    "browse_allowlist",
    "browse_sessions",
    "browse_evidence",
    "browse_downloads",
    "browse_receipts",
    "audio_sources",
    "audio_capture_sessions",
    "audio_capture_chunks",
    "audio_transcripts",
    "audio_transcript_receipts",
    "analytics_receipts",
    "analytics_results",
    "analytics_cohorts",
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
];

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

/// Every snapshot family except the 078-owned ones and the version tag:
/// the complete pre-078 state as the backup mechanism itself sees it.
fn pre_078_view(meta: &SqliteMetaStore) -> serde_json::Value {
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&meta.snapshot_bytes().unwrap()).unwrap();
    let object = snapshot.as_object_mut().unwrap();
    object.remove("schema_version");
    // Id sequences are allocation counters, not rows; their backup and
    // restore are covered by `id_sequences_restore`.
    object.remove("id_sequences");
    object.retain(|key, _| {
        !key.starts_with("model_fleet_")
            && !key.starts_with("privacy_")
            && !key.starts_with("browse_")
            && !key.starts_with("audio_")
            && !key.starts_with("analytics_")
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

/// One lane's dispatch as Core performs it (`migration.md` section 5): the
/// lane's own Spec 077 `AgentRun` is created `Pending` through Spec 077's
/// unmodified insert, then bound by its `LaneRunRef`; it is only ever
/// started after the binding has committed.
fn dispatch(
    meta: &SqliteMetaStore,
    run_id: &str,
    agent_id: &str,
    ctx_id: &str,
    ref_id: &str,
    lane_id: &str,
) {
    meta.insert_agent_run(&agent_run(run_id, agent_id, ctx_id))
        .unwrap();
    meta.insert_lane_run_ref(&lane_run_ref(ref_id, "fleet-1", lane_id, run_id))
        .unwrap();
}

/// A two-lane fleet bound to the pre-existing 077 identities/contexts:
/// lane-1 completes, lane-2 fails, the fleet lands `PartiallyFailed`, and
/// one comparison report names lane-2 as excluded.
fn build_fleet(meta: &SqliteMetaStore) {
    meta.insert_agent_lane(&lane("lane-1", "agent-1", "ctx-1"))
        .unwrap();
    meta.insert_agent_lane(&lane("lane-2", "agent-2", "ctx-2"))
        .unwrap();
    meta.insert_fleet_run(&fleet_run("fleet-1")).unwrap();
    meta.transition_fleet_run(&id("fleet-1"), 1, FleetRunState::Running)
        .unwrap();
    for (n, lane_id, agent_id, ctx_id) in [
        (1, "lane-1", "agent-1", "ctx-1"),
        (2, "lane-2", "agent-2", "ctx-2"),
    ] {
        let run_id = format!("lrun-{n}");
        dispatch(
            meta,
            &run_id,
            agent_id,
            ctx_id,
            &format!("lref-{n}"),
            lane_id,
        );
    }
    finish_run(meta, "lrun-1", "ctx-1", AgentRunState::Completed);
    finish_run(meta, "lrun-2", "ctx-2", AgentRunState::Failed);
    meta.transition_fleet_run(&id("fleet-1"), 2, FleetRunState::PartiallyFailed)
        .unwrap();
    meta.insert_comparison_report(&partial_failure_report("report-1"))
        .unwrap();
}

fn assert_corrupt(result: Result<(), MetaError>, why: &str) {
    match result {
        Err(MetaError::CorruptObjectBody(_)) => {}
        other => panic!("{why}: expected CorruptObjectBody, got {other:?}"),
    }
}

/// Rewrites a backup's snapshot with `edit` and recomputes the outer
/// digest, so only the restore-time invariants can catch the edit.
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

fn snapshot_rows<'a>(
    snapshot: &'a mut serde_json::Value,
    key: &str,
) -> &'a mut Vec<serde_json::Value> {
    snapshot
        .get_mut(key)
        .and_then(|v| v.as_array_mut())
        .unwrap_or_else(|| panic!("{key} present in snapshot"))
}

// ---------------------------------------------------------------------------
// migration.md sections 7/8: populated pre-078 fixture qualification
// ---------------------------------------------------------------------------

#[test]
fn migration_v6_to_v7_preserves_populated_pre_078_vault_and_binds_to_its_077_objects() {
    let root = temp_root("migrate-v7");
    let before = {
        let meta = open_meta(&root);
        populate_pre_078(&meta);
        pre_078_view(&meta)
    };
    rewind_to_v6(&root);
    for table in MODEL_FLEET_TABLES {
        assert!(!table_exists(&root, table), "{table} must be absent at v6");
    }

    // Steps 4-5: apply the migration once and inspect version metadata.
    let meta = open_meta(&root);
    let journal = meta.migration_journal().unwrap();
    assert_eq!(
        journal.finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );
    assert_eq!(journal.started_version, None);
    for table in MODEL_FLEET_TABLES {
        assert!(table_exists(&root, table), "{table} must exist at v7");
    }
    assert_eq!(
        pre_078_view(&meta),
        before,
        "migration must not touch pre-078 rows"
    );

    // Step 6: a pre-078 regression operation (a full 077 run cycle).
    meta.insert_agent_run(&agent_run("run-2", "agent-1", "ctx-1"))
        .unwrap();
    finish_run(&meta, "run-2", "ctx-1", AgentRunState::Completed);
    meta.verify_run_receipt_consistency().unwrap();

    // Step 7: 078 operations bound to the fixture's own 077 objects.
    build_fleet(&meta);
    meta.verify_model_fleet_consistency().unwrap();
    let pre_view_after_078 = pre_078_view(&meta);
    drop(meta);

    // Steps 8-11: close, reopen, verify exact identities/revisions, and
    // prove a repeated open is a no-op.
    for _ in 0..3 {
        let meta = open_meta(&root);
        assert_eq!(
            meta.migration_journal().unwrap().finished_version,
            medscale_storage::CURRENT_META_SCHEMA_VERSION
        );
        let lane_1 = meta.get_agent_lane(&id("lane-1")).unwrap();
        assert_eq!(lane_1.agent_identity_id.as_str(), "agent-1");
        assert_eq!(lane_1.revision, 1);
        let fleet = meta.get_fleet_run(&id("fleet-1")).unwrap();
        assert_eq!(fleet.status, FleetRunState::PartiallyFailed);
        assert_eq!(fleet.revision, 3);
        let refs = meta.list_lane_run_refs(&id("fleet-1")).unwrap();
        let bound: Vec<_> = refs.iter().map(|r| r.agent_run_id.as_str()).collect();
        assert_eq!(bound, ["lrun-1", "lrun-2"]);
        let reports = meta.list_comparison_reports(&id("fleet-1")).unwrap();
        assert_eq!(reports, vec![partial_failure_report("report-1")]);
        assert_eq!(pre_078_view(&meta), pre_view_after_078);
        let identity = meta.get_agent_identity(&id("agent-1")).unwrap();
        assert_eq!(identity.pack_id.as_str(), ADMITTED_PACK_ID);
        meta.verify_model_fleet_consistency().unwrap();
        meta.verify_run_receipt_consistency().unwrap();
    }
}

#[test]
fn repeated_open_and_repeated_migration_is_safe() {
    let root = temp_root("repeat-open");
    {
        let meta = open_meta(&root);
        populate_pre_078(&meta);
        meta.insert_agent_lane(&lane("lane-1", "agent-1", "ctx-1"))
            .unwrap();
    }
    for _ in 0..5 {
        let meta = open_meta(&root);
        let journal = meta.migration_journal().unwrap();
        assert_eq!(
            journal.finished_version,
            medscale_storage::CURRENT_META_SCHEMA_VERSION
        );
        assert_eq!(meta.get_agent_lane(&id("lane-1")).unwrap().revision, 1);
        assert_eq!(meta.list_all_agent_lanes().unwrap().len(), 1);
    }
}

#[test]
fn encrypted_vault_migrates_to_v7_and_reopens_with_its_key() {
    let root = temp_root("sqlcipher");
    let key = [7_u8; 32];
    {
        let meta = SqliteMetaStore::open_at_sqlcipher(&db_path(&root), &key).unwrap();
        assert_eq!(
            meta.migration_journal().unwrap().finished_version,
            medscale_storage::CURRENT_META_SCHEMA_VERSION
        );
        populate_pre_078(&meta);
        build_fleet(&meta);
    }
    let meta = SqliteMetaStore::open_at_sqlcipher(&db_path(&root), &key).unwrap();
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );
    assert_eq!(
        meta.get_fleet_run(&id("fleet-1")).unwrap().status,
        FleetRunState::PartiallyFailed
    );
    meta.verify_model_fleet_consistency().unwrap();
    assert!(SqliteMetaStore::open_at_sqlcipher(&db_path(&root), &[8_u8; 32]).is_err());
}

// ---------------------------------------------------------------------------
// migration.md section 6: crash points
// ---------------------------------------------------------------------------

/// Crash points 1-3: before the migration begins the vault is a valid v6
/// vault; a crash after the journal records `started` but before
/// `finished` fails closed on the next open (never a silent partial v7);
/// the verified pre-migration backup restores a usable vault.
#[test]
fn crash_mid_migration_fails_closed_and_pre_migration_backup_recovers() {
    let root = temp_root("crash-migrate");
    let vault_root = root.join("vault");
    {
        let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
        populate_vault(&vault);
        backup_vault(&vault, &root.join("checkpoint")).unwrap();
    }
    rewind_to_v6(&vault_root);
    {
        let conn = raw(&vault_root);
        conn.execute(
            "INSERT INTO migration_journal(version, state) VALUES (7, 'started')",
            [],
        )
        .unwrap();
        // Half-applied DDL: one of the four v7 tables exists.
        conn.execute_batch("CREATE TABLE model_fleet_lanes (lane_id TEXT PRIMARY KEY);")
            .unwrap();
    }
    match SqliteMetaStore::open_at(&db_path(&vault_root)) {
        Err(MetaError::MigrationIncomplete(7)) => {}
        other => panic!("an interrupted v7 migration must fail closed, got {other:?}"),
    }

    let restored_root = root.join("restored");
    restore_vault(&root.join("checkpoint"), &restored_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restored_root).unwrap();
    assert_eq!(
        restored.meta.migration_journal().unwrap().finished_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );
    assert_eq!(
        restored.meta.get_agent_run(&id("run-1")).unwrap().status,
        AgentRunState::Completed
    );
    assert!(restored.meta.list_all_agent_lanes().unwrap().is_empty());
}

/// Crash points 4, 5, 7, 8: a write transaction that never commits leaves
/// nothing behind after reopen, for every 078 row family.
#[test]
fn uncommitted_078_writes_are_absent_after_reopen() {
    let root = temp_root("crash-uncommitted");
    {
        let meta = open_meta(&root);
        populate_pre_078(&meta);
        meta.insert_fleet_run(&fleet_run("fleet-1")).unwrap();
    }
    {
        let conn = raw(&root);
        conn.execute_batch(
            "BEGIN;
             INSERT INTO model_fleet_lanes VALUES ('lane-x', 'proj-1', 'realm-1', 'scope-1', 'agent-1', 'ctx-1', 'r', '{\"granted_tool_kinds\":null,\"context_artifact_ids\":null}', 'active', 1, 1);
             INSERT INTO model_fleet_runs VALUES ('fleet-x', 'proj-1', 'realm-1', 'scope-1', 'p', 'pending', 1, 1);
             UPDATE model_fleet_runs SET status = 'running', revision = 2 WHERE fleet_run_id = 'fleet-1';
             INSERT INTO model_fleet_comparison_reports VALUES ('report-x', 'fleet-1', 'realm-1', 'scope-1', '[]', '[\"lane-x\"]', '[]', 1);",
        )
        .unwrap();
        // Dropped without COMMIT: the process "crashes" mid-transaction.
    }
    let meta = open_meta(&root);
    assert!(matches!(
        meta.get_agent_lane(&id("lane-x")),
        Err(MetaError::NotFound)
    ));
    assert!(matches!(
        meta.get_fleet_run(&id("fleet-x")),
        Err(MetaError::NotFound)
    ));
    let fleet = meta.get_fleet_run(&id("fleet-1")).unwrap();
    assert_eq!((fleet.status, fleet.revision), (FleetRunState::Pending, 1));
    assert!(meta.list_all_comparison_reports().unwrap().is_empty());
    meta.verify_model_fleet_consistency().unwrap();
}

/// Crash point 6: a crash (or refusal) after a lane's `AgentRun` was
/// created but before its `LaneRunRef` committed leaves only a never-started
/// `Pending` run that no fleet claims -- a valid Spec 077 state -- while the
/// sibling lane's committed binding is untouched and the fleet is still
/// consistent. A bound lane is never missing from its fleet.
#[test]
fn failed_lane_binding_leaves_sibling_intact_and_only_an_unstarted_unbound_run() {
    let root = temp_root("dispatch-crash");
    let meta = open_meta(&root);
    populate_pre_078(&meta);
    meta.insert_agent_lane(&lane("lane-1", "agent-1", "ctx-1"))
        .unwrap();
    meta.insert_agent_lane(&lane("lane-2", "agent-2", "ctx-2"))
        .unwrap();
    meta.insert_fleet_run(&fleet_run("fleet-1")).unwrap();
    meta.transition_fleet_run(&id("fleet-1"), 1, FleetRunState::Running)
        .unwrap();
    dispatch(&meta, "lrun-1", "agent-1", "ctx-1", "lref-1", "lane-1");

    // lane-2: the run commits, then the binding collides on lref-1's key.
    meta.insert_agent_run(&agent_run("lrun-2", "agent-2", "ctx-2"))
        .unwrap();
    let err = meta
        .insert_lane_run_ref(&lane_run_ref("lref-1", "fleet-1", "lane-2", "lrun-2"))
        .unwrap_err();
    assert!(matches!(err, MetaError::Conflict(_)), "got {err:?}");
    drop(meta);

    let meta = open_meta(&root);
    let refs = meta.list_lane_run_refs(&id("fleet-1")).unwrap();
    assert_eq!(
        refs,
        vec![lane_run_ref("lref-1", "fleet-1", "lane-1", "lrun-1")]
    );
    assert_eq!(
        meta.get_agent_run(&id("lrun-2")).unwrap().status,
        AgentRunState::Pending
    );
    assert!(matches!(
        meta.get_run_receipt(&id("lrun-2")),
        Err(MetaError::NotFound)
    ));
    meta.verify_model_fleet_consistency().unwrap();
    meta.verify_run_receipt_consistency().unwrap();
}

// ---------------------------------------------------------------------------
// security.md T4 + revision preconditions
// ---------------------------------------------------------------------------

#[test]
fn an_agent_run_cannot_be_bound_to_two_lane_run_refs() {
    let root = temp_root("t4-unique");
    let meta = open_meta(&root);
    populate_pre_078(&meta);
    meta.insert_agent_lane(&lane("lane-1", "agent-1", "ctx-1"))
        .unwrap();
    meta.insert_fleet_run(&fleet_run("fleet-1")).unwrap();
    meta.insert_fleet_run(&fleet_run("fleet-2")).unwrap();
    meta.insert_lane_run_ref(&lane_run_ref("lref-1", "fleet-1", "lane-1", "run-1"))
        .unwrap();
    let err = meta
        .insert_lane_run_ref(&lane_run_ref("lref-2", "fleet-2", "lane-1", "run-1"))
        .unwrap_err();
    assert!(matches!(err, MetaError::Conflict(_)), "got {err:?}");
    assert_eq!(meta.list_all_lane_run_refs().unwrap().len(), 1);
}

#[test]
fn lane_retire_and_fleet_transition_are_stale_revision_safe() {
    let root = temp_root("cas");
    let meta = open_meta(&root);
    meta.insert_agent_lane(&lane("lane-1", "agent-1", "ctx-1"))
        .unwrap();
    meta.insert_fleet_run(&fleet_run("fleet-1")).unwrap();

    let retired = meta.retire_agent_lane(&id("lane-1"), 1).unwrap();
    assert_eq!(
        (retired.status, retired.revision),
        (AgentLaneStatus::Retired, 2)
    );
    assert!(matches!(
        meta.retire_agent_lane(&id("lane-1"), 1),
        Err(MetaError::Conflict(_))
    ));

    let running = meta
        .transition_fleet_run(&id("fleet-1"), 1, FleetRunState::Running)
        .unwrap();
    assert_eq!(running.revision, 2);
    assert!(matches!(
        meta.transition_fleet_run(&id("fleet-1"), 1, FleetRunState::Cancelled),
        Err(MetaError::Conflict(_))
    ));
    assert_eq!(
        meta.get_fleet_run(&id("fleet-1")).unwrap().status,
        FleetRunState::Running
    );
    assert!(matches!(
        meta.retire_agent_lane(&id("missing"), 1),
        Err(MetaError::NotFound)
    ));
}

#[test]
fn list_filters_are_project_and_state_scoped() {
    let root = temp_root("lists");
    let meta = open_meta(&root);
    populate_pre_078(&meta);
    build_fleet(&meta);
    meta.insert_fleet_run(&fleet_run("fleet-2")).unwrap();
    assert_eq!(
        meta.list_agent_lanes(&id("proj-1"), None, 50)
            .unwrap()
            .len(),
        2
    );
    assert!(
        meta.list_agent_lanes(&id("proj-other"), None, 50)
            .unwrap()
            .is_empty()
    );
    assert!(
        meta.list_agent_lanes(&id("proj-1"), Some(AgentLaneStatus::Retired), 50)
            .unwrap()
            .is_empty()
    );
    let pending = meta
        .list_fleet_runs(&id("proj-1"), Some(FleetRunState::Pending), 50)
        .unwrap();
    assert_eq!(pending, vec![fleet_run("fleet-2")]);
    assert_eq!(
        meta.list_fleet_runs(&id("proj-1"), None, 1).unwrap().len(),
        1
    );
}

// ---------------------------------------------------------------------------
// restore-time invariants (migration.md sections 6/11)
// ---------------------------------------------------------------------------

#[test]
fn verify_model_fleet_consistency_detects_every_invariant_break() {
    type Tamper = (&'static str, &'static str);
    let cases: [Tamper; 7] = [
        (
            "lane run ref naming a missing agent run",
            "UPDATE model_fleet_lane_run_refs SET agent_run_id = 'ghost-run' WHERE lane_run_ref_id = 'lref-1'",
        ),
        (
            "lane run ref naming a missing lane",
            "UPDATE model_fleet_lane_run_refs SET agent_lane_id = 'ghost-lane' WHERE lane_run_ref_id = 'lref-1'",
        ),
        (
            "lane run ref naming a missing fleet run",
            "UPDATE model_fleet_lane_run_refs SET fleet_run_id = 'ghost-fleet' WHERE lane_run_ref_id = 'lref-2'",
        ),
        (
            "lane run ref binding a run dispatched for another lane (T3)",
            "UPDATE model_fleet_lane_run_refs SET agent_run_id = 'run-1' WHERE lane_run_ref_id = 'lref-2'",
        ),
        (
            "fleet state disagreeing with its bound runs",
            "UPDATE model_fleet_runs SET status = 'completed' WHERE fleet_run_id = 'fleet-1'",
        ),
        (
            "terminal fleet over a non-terminal lane run",
            "UPDATE medagent_runs SET status = 'running' WHERE run_id = 'lrun-2'",
        ),
        (
            "comparison report naming a lane outside its fleet run",
            "UPDATE model_fleet_comparison_reports SET excluded_lane_ids_json = '[\"lane-9\"]' WHERE report_id = 'report-1'",
        ),
    ];
    for (n, (why, sql)) in cases.into_iter().enumerate() {
        let root = temp_root(&format!("verify-{n}"));
        let meta = open_meta(&root);
        populate_pre_078(&meta);
        build_fleet(&meta);
        meta.verify_model_fleet_consistency().unwrap();
        raw(&root).execute_batch(sql).unwrap();
        assert_corrupt(meta.verify_model_fleet_consistency(), why);
    }

    // A report attached to a fleet that never reached a comparable state.
    let root = temp_root("verify-report-state");
    let meta = open_meta(&root);
    populate_pre_078(&meta);
    build_fleet(&meta);
    raw(&root)
        .execute_batch(
            "UPDATE model_fleet_runs SET status = 'running' WHERE fleet_run_id = 'fleet-1'",
        )
        .unwrap();
    assert_corrupt(
        meta.verify_model_fleet_consistency(),
        "report over a non-comparable fleet",
    );

    // A cancelled fleet over a lane that actually completed.
    let root = temp_root("verify-cancelled-over-completed");
    let meta = open_meta(&root);
    populate_pre_078(&meta);
    build_fleet(&meta);
    raw(&root)
        .execute_batch(
            "UPDATE model_fleet_runs SET status = 'cancelled' WHERE fleet_run_id = 'fleet-1';
             DELETE FROM model_fleet_comparison_reports;",
        )
        .unwrap();
    assert_corrupt(
        meta.verify_model_fleet_consistency(),
        "cancelled fleet over completed/failed lanes",
    );

    // A terminal (non-cancelled) fleet with no dispatched lane at all.
    let root = temp_root("verify-empty-terminal");
    let meta = open_meta(&root);
    meta.insert_fleet_run(&fleet_run("fleet-1")).unwrap();
    raw(&root)
        .execute_batch(
            "UPDATE model_fleet_runs SET status = 'failed' WHERE fleet_run_id = 'fleet-1'",
        )
        .unwrap();
    assert_corrupt(
        meta.verify_model_fleet_consistency(),
        "terminal fleet without lanes",
    );
    // ...whereas a fleet cancelled before any dispatch is legitimate.
    raw(&root)
        .execute_batch(
            "UPDATE model_fleet_runs SET status = 'cancelled' WHERE fleet_run_id = 'fleet-1'",
        )
        .unwrap();
    meta.verify_model_fleet_consistency().unwrap();
}

// ---------------------------------------------------------------------------
// backup/restore (migration.md sections 8 step 12, 9, 11)
// ---------------------------------------------------------------------------

fn backed_up_fleet_vault(root: &Path) -> PathBuf {
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_vault(&vault);
    build_fleet(&vault.meta);
    let dest = root.join("backup-out");
    let manifest = backup_vault(&vault, &dest).unwrap();
    assert_eq!(
        manifest.schema_version,
        medscale_storage::CURRENT_META_SCHEMA_VERSION
    );
    dest
}

#[test]
fn backup_restore_roundtrips_every_078_row_exactly() {
    let root = temp_root("backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_vault(&vault);
    build_fleet(&vault.meta);
    let dest = root.join("backup-out");
    backup_vault(&vault, &dest).unwrap();

    let restored_root = root.join("restored");
    restore_vault(&dest, &restored_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restored_root).unwrap();
    assert_eq!(
        restored.meta.list_all_agent_lanes().unwrap(),
        vault.meta.list_all_agent_lanes().unwrap()
    );
    assert_eq!(
        restored.meta.list_all_fleet_runs().unwrap(),
        vault.meta.list_all_fleet_runs().unwrap()
    );
    assert_eq!(
        restored.meta.list_all_lane_run_refs().unwrap(),
        vault.meta.list_all_lane_run_refs().unwrap()
    );
    assert_eq!(
        restored.meta.list_all_comparison_reports().unwrap(),
        vault.meta.list_all_comparison_reports().unwrap()
    );
    assert_eq!(pre_078_view(&restored.meta), pre_078_view(&vault.meta));
    restored.meta.verify_model_fleet_consistency().unwrap();
    restored.meta.verify_run_receipt_consistency().unwrap();

    // The restored vault is live: the CAS revision continues from the
    // restored value rather than restarting.
    restored.meta.retire_agent_lane(&id("lane-2"), 1).unwrap();
}

type SnapshotEdit = Box<dyn Fn(&mut serde_json::Value)>;

#[test]
fn restore_rejects_hand_edited_078_snapshots() {
    let edits: Vec<(&str, &str, SnapshotEdit)> = vec![
        (
            "second binding of one agent run",
            "already bound",
            Box::new(|s| {
                let refs = snapshot_rows(s, "model_fleet_lane_run_refs");
                let mut dup = refs[0].clone();
                dup["header"]["id"] = "lref-dup".into();
                refs.push(dup);
            }),
        ),
        (
            "duplicate lane id",
            "duplicate agent lane",
            Box::new(|s| {
                let lanes = snapshot_rows(s, "model_fleet_lanes");
                let dup = lanes[0].clone();
                lanes.push(dup);
            }),
        ),
        (
            "fleet completed over a failed lane",
            "aggregate to",
            Box::new(|s| {
                snapshot_rows(s, "model_fleet_runs")[0]["status"] = "completed".into();
            }),
        ),
        (
            "report naming an unbound lane",
            "not bound to its fleet run",
            Box::new(|s| {
                snapshot_rows(s, "model_fleet_comparison_reports")[0]["excluded_lane_ids"] =
                    serde_json::json!(["lane-9"]);
            }),
        ),
        (
            "report shape violating its contract",
            "both participating and excluded",
            Box::new(|s| {
                snapshot_rows(s, "model_fleet_comparison_reports")[0]["excluded_lane_ids"] =
                    serde_json::json!(["lane-1"]);
            }),
        ),
        (
            "unknown fleet run state",
            "unknown variant",
            Box::new(|s| {
                snapshot_rows(s, "model_fleet_runs")[0]["status"] = "winner".into();
            }),
        ),
        (
            "oversized role label",
            "role_label",
            Box::new(|s| {
                snapshot_rows(s, "model_fleet_lanes")[0]["role_label"] = "x".repeat(4096).into();
            }),
        ),
    ];
    for (n, (why, expected, edit)) in edits.into_iter().enumerate() {
        let root = temp_root(&format!("tamper-{n}"));
        let dest = backed_up_fleet_vault(&root);
        tamper_backup(&dest, |s| edit(s));
        let err = restore_vault(&dest, &root.join("restored")).unwrap_err();
        assert!(
            err.contains(expected),
            "{why}: must fail closed on the restore-time check (expected {expected:?}), got: {err}"
        );
    }
}

/// `migration.md` sections 8 step 12 and 9: the pre-migration (v6)
/// checkpoint restores into a working vault whose 078 tables are empty --
/// never half-populated -- and whose pre-078 state is exactly the
/// checkpoint's. Post-checkpoint 078 changes are discarded by design.
#[test]
fn pre_078_v6_backup_restores_with_empty_078_tables() {
    let root = temp_root("restore-v6");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_vault(&vault);
    let expected = pre_078_view(&vault.meta);
    let dest = root.join("checkpoint");
    backup_vault(&vault, &dest).unwrap();
    // Re-express the checkpoint exactly as a v6 build writes it.
    tamper_backup(&dest, |s| {
        let object = s.as_object_mut().unwrap();
        object.retain(|key, _| {
            !key.starts_with("model_fleet_")
                && !key.starts_with("privacy_")
                && !key.starts_with("browse_")
                && !key.starts_with("audio_")
                && !key.starts_with("analytics_")
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
        object.insert("schema_version".to_owned(), 6.into());
    });
    let manifest_path = dest.join("manifest.json");
    let mut manifest: BackupManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.schema_version = 6;
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    // 078 work done after the checkpoint...
    build_fleet(&vault.meta);

    // ...is absent from the recovered vault.
    let restored_root = root.join("restored");
    restore_vault(&dest, &restored_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restored_root).unwrap();
    assert_eq!(pre_078_view(&restored.meta), expected);
    assert!(restored.meta.list_all_agent_lanes().unwrap().is_empty());
    assert!(restored.meta.list_all_fleet_runs().unwrap().is_empty());
    assert!(restored.meta.list_all_lane_run_refs().unwrap().is_empty());
    assert!(
        restored
            .meta
            .list_all_comparison_reports()
            .unwrap()
            .is_empty()
    );
    restored.meta.verify_run_receipt_consistency().unwrap();
}

// ---------------------------------------------------------------------------
// migration.md sections 4/12: no cascade into canonical or 077 objects
// ---------------------------------------------------------------------------

#[test]
fn model_fleet_tables_declare_no_foreign_keys_and_078_operations_never_touch_pre_078_rows() {
    let root = temp_root("no-cascade");
    let meta = open_meta(&root);
    populate_pre_078(&meta);
    let conn = raw(&root);
    for table in MODEL_FLEET_TABLES {
        let fk_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM pragma_foreign_key_list('{table}')"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fk_count, 0, "{table} must not declare a foreign key");
    }
    let before = pre_078_view(&meta);

    // Every 078 mutation path except dispatch (which by design inserts the
    // lane's own new AgentRun through Spec 077's insert_agent_run).
    meta.insert_agent_lane(&lane("lane-1", "agent-1", "ctx-1"))
        .unwrap();
    meta.retire_agent_lane(&id("lane-1"), 1).unwrap();
    meta.insert_fleet_run(&fleet_run("fleet-9")).unwrap();
    meta.transition_fleet_run(&id("fleet-9"), 1, FleetRunState::Cancelled)
        .unwrap();
    meta.alloc_model_fleet_id("model_fleet_lane_seq", "lane")
        .unwrap();
    assert_eq!(pre_078_view(&meta), before);

    // Removing 078 rows wholesale leaves every pre-078 row intact.
    conn.execute_batch(
        "DELETE FROM model_fleet_lanes; DELETE FROM model_fleet_runs;
         DELETE FROM model_fleet_lane_run_refs; DELETE FROM model_fleet_comparison_reports;",
    )
    .unwrap();
    assert_eq!(pre_078_view(&meta), before);
}

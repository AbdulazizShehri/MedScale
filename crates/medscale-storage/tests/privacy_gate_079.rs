//! Spec 079 Privacy Gate storage + migration integration tests (T079-02).
//!
//! Synthetic data only. Covers `migration.md` (additive v7 -> v8, crash
//! recovery, backup/restore, cross-row invariants) and `security.md` T10/T12
//! against a genuinely pre-079 (schema v7) vault: the fixture is built with
//! real 074/078 rows, the v8 tables and journal entry are removed so the file
//! is exactly what a v7 build leaves on disk, and only then is v8 applied.

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::model_fleet::{AgentLane, LanePolicy, MODEL_FLEET_SCHEMA_VERSION};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::privacy_gate::{
    ArtifactClassification, ClassificationBasis, DataClass, DeidReceipt, DeidReceiptStatus,
    EgressBoundary, EgressDecision, EgressOutcome, EgressReason, PRIVACY_GATE_SCHEMA_VERSION,
    PrivacyPolicyProfile, PrivacyProfileStatus, ProfileRule, PseudonymMapRef, PseudonymMapStatus,
    ReceiptLimitation, RecognizerFamily, RecognizerIdentity, RecognizerResult, RecognizerStatus,
    ReidentificationAudit, ReidentificationOutcome, ResidualScanResult, SensitiveSpanKind,
    TransformOp, TransformOpCount,
};
use medscale_contracts::project_graph::Project;
use medscale_keys::WrappedBlob;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, DeidTransformCommit, MetaError, PseudonymEntryRow, SourceMeta,
    SqliteMetaStore, SyntheticVault, backup_vault, restore_vault,
};

const SOURCE_BYTES: &[u8] = b"synthetic source bytes";

const PRIVACY_TABLES: [&str; 7] = [
    "privacy_classifications",
    "privacy_profiles",
    "privacy_deid_receipts",
    "privacy_pseudonym_maps",
    "privacy_pseudonym_entries",
    "privacy_reid_audit",
    "privacy_egress_decisions",
];

// ---------------------------------------------------------------------------
// fixture helpers
// ---------------------------------------------------------------------------

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-079-{name}-{}", std::process::id()));
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

fn h(id: &str) -> ObjectHeader {
    header_v(id, PRIVACY_GATE_SCHEMA_VERSION)
}

fn id(value: &str) -> OpaqueId {
    OpaqueId::new(value)
}

fn populate_pre_079(meta: &SqliteMetaStore) {
    meta.insert_source(&SourceMeta {
        source_id: id("src-1"),
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
        digest: DigestSha256::of(SOURCE_BYTES),
        byte_length: SOURCE_BYTES.len() as u64,
        media_type: "text/plain".to_owned(),
        visible: true,
        resource_type: "Note".to_owned(),
    })
    .unwrap();
    meta.insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
    meta.insert_agent_lane(
        &AgentLane::new(
            header_v("lane-1", MODEL_FLEET_SCHEMA_VERSION),
            id("proj-1"),
            id("agent-1"),
            id("ctx-1"),
            "reviewer".to_owned(),
            LanePolicy::new(None, None).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
}

/// Tables created by migrations after v8.
const LATER_VERSION_TABLES: &[&str] = &[
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

/// Rewinds the file to exactly what a v7 build leaves on disk.
fn rewind_to_v7(root: &Path) {
    let conn = raw(root);
    for table in PRIVACY_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    // Later additive versions (Spec 080 v9, ...) are absent in a v7 build too.
    for table in LATER_VERSION_TABLES {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {table};"))
            .unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 8", [])
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

/// The complete pre-079 state as the backup mechanism sees it.
fn pre_079_view(meta: &SqliteMetaStore) -> serde_json::Value {
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&meta.snapshot_bytes().unwrap()).unwrap();
    let object = snapshot.as_object_mut().unwrap();
    object.remove("schema_version");
    // Id sequences are allocation counters, not rows; their backup and
    // restore are covered by `id_sequences_restore`.
    object.remove("id_sequences");
    object.retain(|key, _| {
        !key.starts_with("privacy_")
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

fn rules(op: TransformOp) -> Vec<ProfileRule> {
    SensitiveSpanKind::ALL
        .iter()
        .map(|kind| ProfileRule { kind: *kind, op })
        .collect()
}

fn profile(profile_id: &str) -> PrivacyPolicyProfile {
    let mut r = rules(TransformOp::Redact);
    r[0].op = TransformOp::Pseudonymize;
    PrivacyPolicyProfile::new(
        h(profile_id),
        id("proj-1"),
        "export profile".to_owned(),
        DataClass::ExternalDeidentified,
        r,
        false,
    )
    .unwrap()
}

fn map(map_id: &str) -> PseudonymMapRef {
    PseudonymMapRef {
        header: h(map_id),
        revision: 1,
        project_id: id("proj-1"),
        key_account: format!("medscale.privacy.{map_id}"),
        entry_count: 0,
        status: PseudonymMapStatus::Active,
    }
}

fn recognizer(spans: u32) -> RecognizerResult {
    RecognizerResult {
        recognizer: RecognizerIdentity {
            recognizer_id: "pattern".to_owned(),
            version: "1".to_owned(),
            family: RecognizerFamily::Deterministic,
            model_pack_id: None,
        },
        status: RecognizerStatus::Completed,
        span_count: spans,
    }
}

fn receipt(receipt_id: &str, output: &str, map_id: Option<&str>) -> DeidReceipt {
    DeidReceipt {
        header: h(receipt_id),
        revision: 1,
        project_id: id("proj-1"),
        source_artifact_id: id("src-1"),
        source_digest: DigestSha256::of(SOURCE_BYTES),
        output_artifact_id: id(output),
        output_digest: DigestSha256::of(output.as_bytes()),
        output_class: DataClass::ExternalDeidentified,
        profile_id: id("prof-1"),
        profile_revision: 1,
        recognizers: vec![recognizer(2)],
        op_counts: vec![TransformOpCount {
            kind: SensitiveSpanKind::PersonName,
            op: TransformOp::Pseudonymize,
            count: 2,
        }],
        pseudonym_map_id: map_id.map(id),
        residual: ResidualScanResult::from_rescan(vec![recognizer(0)]),
        limitations: vec![ReceiptLimitation::AutomatedRecognitionIsIncomplete],
        status: DeidReceiptStatus::Valid,
    }
}

fn entry(map_id: &str, pseudonym: &str) -> PseudonymEntryRow {
    PseudonymEntryRow {
        map_id: id(map_id),
        pseudonym: pseudonym.to_owned(),
        sealed: WrappedBlob {
            nonce: vec![7; 12],
            ciphertext: vec![1, 2, 3],
        },
    }
}

fn commit(receipt_id: &str, output: &str, entries: &[&str]) -> DeidTransformCommit {
    DeidTransformCommit {
        receipt: receipt(receipt_id, output, Some("map-1")),
        output_classification: ArtifactClassification::from_receipt(
            h(&format!("cls-{output}")),
            id("proj-1"),
            id(output),
            DataClass::ExternalDeidentified,
            id(receipt_id),
        ),
        new_entries: entries.iter().map(|p| entry("map-1", p)).collect(),
    }
}

const PSN_A: &str = "PSN-aaaaaaaaaaaa";
const PSN_B: &str = "PSN-bbbbbbbbbbbb";

/// A fully populated 079 state: profile, map, two transforms, audit, decision.
fn build_privacy_state(meta: &SqliteMetaStore) {
    meta.insert_privacy_profile(&profile("prof-1")).unwrap();
    meta.insert_pseudonym_map(&map("map-1")).unwrap();
    meta.commit_deid_transform(&commit("rcpt-1", "out-1", &[PSN_A, PSN_B]))
        .unwrap();
    meta.commit_deid_transform(&commit("rcpt-2", "out-2", &[PSN_A]))
        .unwrap();
    meta.insert_classification(&ArtifactClassification::declared(
        h("cls-src"),
        id("proj-1"),
        id("src-1"),
        DataClass::LocalPhi,
    ))
    .unwrap();
    meta.insert_reid_audit(&ReidentificationAudit {
        header: h("audit-1"),
        project_id: id("proj-1"),
        map_id: id("map-1"),
        pseudonym: PSN_A.to_owned(),
        requested_by: id("holder-1"),
        reason: "synthetic follow-up".to_owned(),
        outcome: ReidentificationOutcome::Returned,
    })
    .unwrap();
    meta.insert_egress_decision(&EgressDecision {
        header: h("dec-1"),
        project_id: id("proj-1"),
        artifact_id: id("out-1"),
        boundary: EgressBoundary::Browse,
        data_class: DataClass::ExternalDeidentified,
        basis: ClassificationBasis::DeidReceipt,
        outcome: EgressOutcome::Allow,
        reason: EgressReason::AllowedDeidentified,
        deid_receipt_id: Some(id("rcpt-1")),
    })
    .unwrap();
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
        .unwrap_or_else(|| panic!("{key} present in snapshot"))
}

fn backed_up_privacy_vault(root: &Path) -> PathBuf {
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    vault.blobs.put_blob(SOURCE_BYTES).unwrap();
    populate_pre_079(&vault.meta);
    build_privacy_state(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    dest
}

// ---------------------------------------------------------------------------
// migration
// ---------------------------------------------------------------------------

#[test]
fn migration_v7_to_v8_preserves_populated_pre_079_vault() {
    let root = temp_root("migrate-v8");
    let before = {
        let meta = open_meta(&root);
        populate_pre_079(&meta);
        pre_079_view(&meta)
    };
    rewind_to_v7(&root);
    for table in PRIVACY_TABLES {
        assert!(!table_exists(&root, table), "{table} must be absent at v7");
    }
    let meta = open_meta(&root);
    let journal = meta.migration_journal().unwrap();
    assert_eq!(journal.finished_version, CURRENT_META_SCHEMA_VERSION);
    assert_eq!(journal.started_version, None);
    for table in PRIVACY_TABLES {
        assert!(table_exists(&root, table), "{table} must exist at v8");
    }
    assert_eq!(
        pre_079_view(&meta),
        before,
        "migration must not touch pre-079 rows"
    );
    assert_eq!(
        meta.get_agent_lane(&id("lane-1")).unwrap().project_id,
        id("proj-1")
    );
    // Unclassified artifacts have no row: Core reports DefaultUnclassified.
    assert!(
        meta.get_classification(&id("proj-1"), &id("src-1"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn repeated_open_is_a_no_op() {
    let root = temp_root("reopen");
    {
        let meta = open_meta(&root);
        populate_pre_079(&meta);
        build_privacy_state(&meta);
    }
    for _ in 0..3 {
        let meta = open_meta(&root);
        assert_eq!(
            meta.migration_journal().unwrap().finished_version,
            CURRENT_META_SCHEMA_VERSION
        );
        assert_eq!(meta.list_all_deid_receipts().unwrap().len(), 2);
        meta.verify_privacy_gate_consistency().unwrap();
    }
}

#[test]
fn crash_mid_v8_migration_fails_closed_and_backup_recovers() {
    let root = temp_root("crash");
    let vault_root = root.join("vault");
    {
        let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
        vault.blobs.put_blob(SOURCE_BYTES).unwrap();
        populate_pre_079(&vault.meta);
        backup_vault(&vault, &root.join("checkpoint")).unwrap();
    }
    rewind_to_v7(&vault_root);
    {
        let conn = raw(&vault_root);
        conn.execute(
            "INSERT INTO migration_journal(version, state) VALUES (8, 'started')",
            [],
        )
        .unwrap();
        conn.execute_batch("CREATE TABLE privacy_profiles (profile_id TEXT PRIMARY KEY);")
            .unwrap();
    }
    match SqliteMetaStore::open_at(&db_path(&vault_root)) {
        Err(MetaError::MigrationIncomplete(8)) => {}
        other => panic!("an interrupted v8 migration must fail closed, got {other:?}"),
    }
    let restored_root = root.join("restored");
    restore_vault(&root.join("checkpoint"), &restored_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restored_root).unwrap();
    assert_eq!(
        restored.meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    assert!(restored.meta.get_agent_lane(&id("lane-1")).is_ok());
}

// ---------------------------------------------------------------------------
// rows, revisions, atomicity
// ---------------------------------------------------------------------------

#[test]
fn classification_is_unique_per_artifact_and_revision_safe() {
    let root = temp_root("classify");
    let meta = open_meta(&root);
    let first = ArtifactClassification::declared(
        h("cls-1"),
        id("proj-1"),
        id("art-1"),
        DataClass::LocalPhi,
    );
    meta.insert_classification(&first).unwrap();
    let dup =
        ArtifactClassification::declared(h("cls-2"), id("proj-1"), id("art-1"), DataClass::Public);
    assert!(matches!(
        meta.insert_classification(&dup),
        Err(MetaError::Conflict(_))
    ));

    let mut next = first.clone();
    next.data_class = DataClass::Public;
    next.revision = 2;
    meta.update_classification(&next, 1).unwrap();
    // Replaying the same CAS (now stale) fails.
    assert!(matches!(
        meta.update_classification(&next, 1),
        Err(MetaError::Conflict(_))
    ));
    let mut skip = next.clone();
    skip.revision = 9;
    assert!(meta.update_classification(&skip, 2).is_err());
    let stored = meta
        .get_classification(&id("proj-1"), &id("art-1"))
        .unwrap()
        .unwrap();
    assert_eq!(stored.data_class, DataClass::Public);
    assert_eq!(stored.revision, 2);
    // Another Project's view of the same artifact id is separate.
    assert!(
        meta.get_classification(&id("proj-2"), &id("art-1"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn profile_and_receipt_revocation_are_terminal_and_stale_safe() {
    let root = temp_root("revoke");
    let meta = open_meta(&root);
    populate_pre_079(&meta);
    build_privacy_state(&meta);
    assert!(meta.revoke_privacy_profile(&id("prof-1"), 7).is_err());
    let revoked = meta.revoke_privacy_profile(&id("prof-1"), 1).unwrap();
    assert_eq!(revoked.status, PrivacyProfileStatus::Revoked);
    assert!(meta.revoke_privacy_profile(&id("prof-1"), 2).is_err());

    let receipt = meta.revoke_deid_receipt(&id("rcpt-1"), 1).unwrap();
    assert_eq!(receipt.status, DeidReceiptStatus::Revoked);
    assert!(meta.revoke_deid_receipt(&id("rcpt-1"), 2).is_err());

    let map_rev = meta.get_pseudonym_map(&id("map-1")).unwrap().revision;
    let revoked_map = meta.revoke_pseudonym_map(&id("map-1"), map_rev).unwrap();
    assert_eq!(revoked_map.status, PseudonymMapStatus::Revoked);
    // A revoked map accepts no further transform.
    assert!(matches!(
        meta.commit_deid_transform(&commit("rcpt-3", "out-3", &["PSN-cccccccccccc"])),
        Err(MetaError::Conflict(_))
    ));
    assert!(meta.get_deid_receipt(&id("rcpt-3")).is_err());
    meta.verify_privacy_gate_consistency().unwrap();
}

#[test]
fn transform_commit_is_atomic_and_counts_distinct_entries() {
    let root = temp_root("atomic");
    let meta = open_meta(&root);
    populate_pre_079(&meta);
    build_privacy_state(&meta);
    let map = meta.get_pseudonym_map(&id("map-1")).unwrap();
    assert_eq!(map.entry_count, 2, "a repeated pseudonym is stored once");

    // The output classification already exists for out-1: the whole commit
    // (receipt + entries) must roll back.
    let mut clash = commit("rcpt-9", "out-1", &["PSN-dddddddddddd"]);
    clash.receipt.output_artifact_id = id("out-9");
    clash.output_classification.artifact_id = id("out-9");
    meta.insert_classification(&ArtifactClassification::declared(
        h("cls-pre-9"),
        id("proj-1"),
        id("out-9"),
        DataClass::LocalPhi,
    ))
    .unwrap();
    assert!(meta.commit_deid_transform(&clash).is_err());
    assert!(meta.get_deid_receipt(&id("rcpt-9")).is_err());
    assert!(
        meta.get_pseudonym_entry(&id("map-1"), "PSN-dddddddddddd")
            .unwrap()
            .is_none()
    );
    assert_eq!(meta.get_pseudonym_map(&id("map-1")).unwrap().entry_count, 2);

    // A classification that does not match its receipt is refused up front.
    let mut mismatched = commit("rcpt-8", "out-8", &[]);
    mismatched.output_classification.deid_receipt_id = Some(id("rcpt-other"));
    assert!(meta.commit_deid_transform(&mismatched).is_err());
    meta.verify_privacy_gate_consistency().unwrap();
}

#[test]
fn edited_row_bodies_fail_closed_on_read() {
    let root = temp_root("edited");
    {
        let meta = open_meta(&root);
        populate_pre_079(&meta);
        build_privacy_state(&meta);
    }
    raw(&root)
        .execute(
            "UPDATE privacy_deid_receipts SET output_artifact_id = 'out-x' WHERE receipt_id = 'rcpt-1'",
            [],
        )
        .unwrap();
    let meta = open_meta(&root);
    assert!(matches!(
        meta.get_deid_receipt(&id("rcpt-1")),
        Err(MetaError::CorruptObjectBody(_))
    ));
}

#[test]
fn consistency_check_detects_every_invariant_break() {
    type Break = (&'static str, &'static str);
    let breaks: [Break; 5] = [
        (
            "entry count",
            "UPDATE privacy_pseudonym_entries SET map_id = 'map-1' WHERE 0; DELETE FROM privacy_pseudonym_entries WHERE pseudonym = 'PSN-bbbbbbbbbbbb'",
        ),
        (
            "orphan receipt",
            "DELETE FROM privacy_classifications WHERE artifact_id = 'out-2'",
        ),
        ("missing profile", "DELETE FROM privacy_profiles"),
        (
            "audit without map",
            "UPDATE privacy_reid_audit SET map_id = 'map-9', body_json = replace(body_json, '\"map-1\"', '\"map-9\"')",
        ),
        (
            "decision with missing receipt",
            "DELETE FROM privacy_classifications WHERE artifact_id = 'out-1'; DELETE FROM privacy_deid_receipts WHERE receipt_id = 'rcpt-1'",
        ),
    ];
    for (name, sql) in breaks {
        let root = temp_root(&format!("break-{}", name.replace(' ', "-")));
        {
            let meta = open_meta(&root);
            populate_pre_079(&meta);
            build_privacy_state(&meta);
            meta.verify_privacy_gate_consistency().unwrap();
        }
        raw(&root).execute_batch(sql).unwrap();
        let meta = open_meta(&root);
        assert!(
            matches!(
                meta.verify_privacy_gate_consistency(),
                Err(MetaError::CorruptObjectBody(_))
            ),
            "{name} must be detected"
        );
    }
}

// ---------------------------------------------------------------------------
// backup / restore
// ---------------------------------------------------------------------------

#[test]
fn backup_restore_roundtrips_every_079_row_exactly() {
    let root = temp_root("roundtrip");
    let dest = backed_up_privacy_vault(&root);
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let restored_root = root.join("restored");
    restore_vault(&dest, &restored_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restored_root).unwrap();
    let a = &original.meta;
    let b = &restored.meta;
    assert_eq!(
        a.list_all_classifications().unwrap(),
        b.list_all_classifications().unwrap()
    );
    assert_eq!(
        a.list_all_privacy_profiles().unwrap(),
        b.list_all_privacy_profiles().unwrap()
    );
    assert_eq!(
        a.list_all_deid_receipts().unwrap(),
        b.list_all_deid_receipts().unwrap()
    );
    assert_eq!(
        a.list_all_pseudonym_maps().unwrap(),
        b.list_all_pseudonym_maps().unwrap()
    );
    assert_eq!(
        a.list_all_pseudonym_entries().unwrap(),
        b.list_all_pseudonym_entries().unwrap()
    );
    assert_eq!(
        a.list_all_reid_audit().unwrap(),
        b.list_all_reid_audit().unwrap()
    );
    assert_eq!(
        a.list_all_egress_decisions().unwrap(),
        b.list_all_egress_decisions().unwrap()
    );
    let manifest: BackupManifest =
        serde_json::from_slice(&fs::read(dest.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.schema_version, CURRENT_META_SCHEMA_VERSION);
}

#[test]
fn restore_rejects_hand_edited_079_snapshots() {
    type Edit = (&'static str, fn(&mut serde_json::Value));
    let edits: [Edit; 5] = [
        ("duplicate receipt", |s| {
            let first = rows(s, "privacy_deid_receipts")[0].clone();
            rows(s, "privacy_deid_receipts").push(first);
        }),
        ("entry count mismatch", |s| {
            rows(s, "privacy_pseudonym_entries").pop();
        }),
        ("receipt without classification", |s| {
            rows(s, "privacy_classifications").retain(|c| c["artifact_id"] != "out-2");
        }),
        ("invalid profile", |s| {
            rows(s, "privacy_profiles")[0]["rules"]
                .as_array_mut()
                .unwrap()
                .pop();
        }),
        ("egress outcome mismatch", |s| {
            rows(s, "privacy_egress_decisions")[0]["outcome"] = serde_json::json!("deny");
        }),
    ];
    for (name, edit) in edits {
        let root = temp_root(&format!("tamper-{}", name.replace(' ', "-")));
        let dest = backed_up_privacy_vault(&root);
        tamper_backup(&dest, edit);
        assert!(
            restore_vault(&dest, &root.join("restored")).is_err(),
            "{name} must be refused"
        );
    }
}

#[test]
fn pre_079_v7_backup_restores_with_empty_079_tables() {
    let root = temp_root("v7-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    vault.blobs.put_blob(SOURCE_BYTES).unwrap();
    populate_pre_079(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    // Turn the snapshot into what a v7 build writes.
    tamper_backup(&dest, |s| {
        let object = s.as_object_mut().unwrap();
        object.retain(|key, _| {
            !key.starts_with("privacy_")
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
        object.insert("schema_version".to_owned(), serde_json::json!(7));
    });
    let restored_root = root.join("restored");
    restore_vault(&dest, &restored_root).unwrap();
    let restored = SyntheticVault::open("vault-1", &restored_root).unwrap();
    assert!(restored.meta.list_all_deid_receipts().unwrap().is_empty());
    assert!(restored.meta.list_all_classifications().unwrap().is_empty());
    assert!(restored.meta.get_agent_lane(&id("lane-1")).is_ok());
    restored.meta.verify_privacy_gate_consistency().unwrap();
}

#[test]
fn no_079_table_declares_a_foreign_key() {
    let root = temp_root("fk");
    let _meta = open_meta(&root);
    let conn = raw(&root);
    for table in PRIVACY_TABLES {
        let count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM pragma_foreign_key_list('{table}')"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "{table} must not declare foreign keys");
    }
}

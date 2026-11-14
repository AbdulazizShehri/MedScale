//! Spec 080 Governed Browse storage + migration integration tests.
//!
//! Synthetic data only. Additive v8 -> v9 migration, crash recovery, atomic
//! session commits, content-digest checks on read, cross-row invariants and
//! backup/restore (including tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::browse::{
    BROWSE_SCHEMA_VERSION, BrowseAllowlistEntry, BrowseDenyReason, BrowseDownloadCandidate,
    BrowseEvidenceItem, BrowseIntentKind, BrowseLimitation, BrowseNavigationStep,
    BrowsePolicyDecision, BrowseReceipt, BrowseRequest, BrowseRoute, BrowseSession,
    BrowseSessionState, DownloadCandidateStatus, HumanTakeoverReason, HumanTakeoverRequest,
    request_digest,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    BrowseSessionCommit, CURRENT_META_SCHEMA_VERSION, MetaError, SqliteMetaStore, SyntheticVault,
    backup_vault, restore_vault,
};

const BROWSE_TABLES: [&str; 5] = [
    "browse_allowlist",
    "browse_sessions",
    "browse_evidence",
    "browse_downloads",
    "browse_receipts",
];

/// Tables created by migrations after v9.
const LATER_VERSION_TABLES: &[&str] = &[
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

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-080-{name}-{}", std::process::id()));
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
        schema_version: BROWSE_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn populate_pre_080(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
}

fn rewind_to_v8(root: &Path) {
    let conn = raw(root);
    for table in BROWSE_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    // Later additive versions (Spec 081 v10, ...) are absent in a v8 build too.
    for table in LATER_VERSION_TABLES {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {table};"))
            .unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 9", [])
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

fn pre_080_view(meta: &SqliteMetaStore) -> serde_json::Value {
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&meta.snapshot_bytes().unwrap()).unwrap();
    let object = snapshot.as_object_mut().unwrap();
    object.remove("schema_version");
    // Id sequences are allocation counters, not rows; their backup and
    // restore are covered by `id_sequences_restore`.
    object.remove("id_sequences");
    object.retain(|key, _| {
        !key.starts_with("browse_")
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

fn request(url: &str) -> BrowseRequest {
    BrowseRequest {
        project_id: id("proj-1"),
        intent: BrowseIntentKind::FetchUrl,
        url: Some(url.to_owned()),
        query: None,
        context_artifact_id: None,
    }
}

const PAGE: &[u8] = b"<p>synthetic page</p>";
const PDF: &[u8] = b"%PDF-1.4 synthetic";

/// A completed session with one evidence item and one download.
fn completed_commit(session: &str) -> BrowseSessionCommit {
    let sid = id(session);
    let url = "https://example.org/a";
    let s = BrowseSession {
        header: h(session),
        revision: 1,
        project_id: id("proj-1"),
        request: request(url),
        route: BrowseRoute::HttpFetch,
        state: BrowseSessionState::Completed,
        decision: BrowsePolicyDecision::allow(None),
        steps: vec![BrowseNavigationStep {
            seq: 1,
            url: url.to_owned(),
            http_status: Some(200),
            redirect_to: None,
            decision: BrowsePolicyDecision::allow(None),
        }],
        takeover: None,
    };
    let ev = BrowseEvidenceItem {
        header: h(&format!("{session}-ev")),
        session_id: sid.clone(),
        final_url: url.to_owned(),
        content_type: "text/html".to_owned(),
        byte_length: PAGE.len() as u64,
        content_digest: DigestSha256::of(PAGE),
        excerpt: "synthetic page".to_owned(),
        instruction_like_content_flagged: false,
    };
    let dl = BrowseDownloadCandidate {
        header: h(&format!("{session}-dl")),
        session_id: sid.clone(),
        final_url: url.to_owned(),
        content_type: "application/pdf".to_owned(),
        byte_length: PDF.len() as u64,
        content_digest: DigestSha256::of(PDF),
        status: DownloadCandidateStatus::Quarantined,
    };
    let receipt = BrowseReceipt {
        header: h(&format!("{session}-rcpt")),
        session_id: sid,
        project_id: id("proj-1"),
        request_digest: request_digest(&s.request),
        route: BrowseRoute::HttpFetch,
        final_state: BrowseSessionState::Completed,
        step_count: 1,
        evidence_ids: vec![ev.header.id.clone()],
        download_ids: vec![dl.header.id.clone()],
        limitations: vec![BrowseLimitation::ContentUnverified],
    };
    BrowseSessionCommit {
        session: s,
        evidence: vec![(ev, PAGE.to_vec())],
        downloads: vec![(dl, PDF.to_vec())],
        receipt,
    }
}

/// A session awaiting human takeover (no evidence).
fn takeover_commit(session: &str) -> BrowseSessionCommit {
    let mut c = completed_commit(session);
    c.session.state = BrowseSessionState::AwaitingHumanTakeover;
    c.session.steps[0].http_status = Some(401);
    c.session.takeover = Some(HumanTakeoverRequest {
        url: "https://example.org/a".to_owned(),
        reason: HumanTakeoverReason::LoginRequired,
    });
    c.evidence.clear();
    c.downloads.clear();
    c.receipt.final_state = BrowseSessionState::AwaitingHumanTakeover;
    c.receipt.evidence_ids.clear();
    c.receipt.download_ids.clear();
    c
}

/// A session denied before any request.
fn denied_commit(session: &str) -> BrowseSessionCommit {
    let mut c = completed_commit(session);
    let deny = BrowsePolicyDecision::deny(BrowseDenyReason::EmptyAllowlist, None);
    c.session.state = BrowseSessionState::Denied;
    c.session.decision = deny.clone();
    c.session.steps[0].http_status = None;
    c.session.steps[0].decision = deny;
    c.evidence.clear();
    c.downloads.clear();
    c.receipt.final_state = BrowseSessionState::Denied;
    c.receipt.evidence_ids.clear();
    c.receipt.download_ids.clear();
    c
}

fn build_state(meta: &SqliteMetaStore) {
    meta.insert_browse_allowlist_entry(
        &BrowseAllowlistEntry::new(
            h("allow-1"),
            id("proj-1"),
            "example.org".to_owned(),
            "/".to_owned(),
        )
        .unwrap(),
    )
    .unwrap();
    meta.commit_browse_session(&completed_commit("s1")).unwrap();
    meta.commit_browse_session(&takeover_commit("s2")).unwrap();
    meta.commit_browse_session(&denied_commit("s3")).unwrap();
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
    populate_pre_080(&vault.meta);
    build_state(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    dest
}

#[test]
fn migration_v8_to_v9_is_additive() {
    let root = temp_root("migrate");
    let before = {
        let meta = open_meta(&root);
        populate_pre_080(&meta);
        pre_080_view(&meta)
    };
    rewind_to_v8(&root);
    for t in BROWSE_TABLES {
        assert!(!table_exists(&root, t));
    }
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    for t in BROWSE_TABLES {
        assert!(table_exists(&root, t), "{t}");
    }
    assert_eq!(pre_080_view(&meta), before);
}

#[test]
fn crash_mid_v9_migration_fails_closed_and_backup_recovers() {
    let root = temp_root("crash");
    let vault_root = root.join("vault");
    {
        let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
        populate_pre_080(&vault.meta);
        backup_vault(&vault, &root.join("checkpoint")).unwrap();
    }
    rewind_to_v8(&vault_root);
    raw(&vault_root)
        .execute(
            "INSERT INTO migration_journal(version, state) VALUES (9, 'started')",
            [],
        )
        .unwrap();
    match SqliteMetaStore::open_at(&db_path(&vault_root)) {
        Err(MetaError::MigrationIncomplete(9)) => {}
        other => panic!("interrupted v9 migration must fail closed, got {other:?}"),
    }
    restore_vault(&root.join("checkpoint"), &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.get_project(&id("proj-1")).is_ok());
}

#[test]
fn session_commits_are_atomic_and_content_checked() {
    let root = temp_root("atomic");
    let meta = open_meta(&root);
    populate_pre_080(&meta);
    let mut bad = completed_commit("s9");
    bad.downloads[0].1 = b"different bytes".to_vec();
    assert!(meta.commit_browse_session(&bad).is_err());
    assert!(matches!(
        meta.get_browse_session(&id("s9")),
        Err(MetaError::NotFound)
    ));
    assert!(meta.list_browse_evidence(&id("s9")).unwrap().is_empty());

    let mut mismatched = completed_commit("s8");
    mismatched.receipt.final_state = BrowseSessionState::Failed;
    assert!(meta.commit_browse_session(&mismatched).is_err());

    meta.commit_browse_session(&completed_commit("s1")).unwrap();
    assert!(matches!(
        meta.commit_browse_session(&completed_commit("s1")),
        Err(MetaError::Conflict(_))
    ));
    let (ev, bytes) = &meta.list_browse_evidence(&id("s1")).unwrap()[0];
    assert_eq!(bytes.as_slice(), PAGE);
    assert_eq!(ev.content_digest, DigestSha256::of(PAGE));
    meta.verify_browse_consistency().unwrap();
}

#[test]
fn stored_content_that_no_longer_matches_its_digest_fails_closed() {
    let root = temp_root("tamper-content");
    {
        let meta = open_meta(&root);
        populate_pre_080(&meta);
        build_state(&meta);
    }
    raw(&root)
        .execute(
            "UPDATE browse_evidence SET content = X'00' WHERE evidence_id = 's1-ev'",
            [],
        )
        .unwrap();
    let meta = open_meta(&root);
    assert!(matches!(
        meta.list_browse_evidence(&id("s1")),
        Err(MetaError::CorruptObjectBody(_))
    ));
}

#[test]
fn allowlist_is_unique_and_disable_is_revision_safe() {
    let root = temp_root("allow");
    let meta = open_meta(&root);
    populate_pre_080(&meta);
    let e = BrowseAllowlistEntry::new(
        h("a1"),
        id("proj-1"),
        "example.org".to_owned(),
        "/".to_owned(),
    )
    .unwrap();
    meta.insert_browse_allowlist_entry(&e).unwrap();
    let dup = BrowseAllowlistEntry::new(
        h("a2"),
        id("proj-1"),
        "example.org".to_owned(),
        "/".to_owned(),
    )
    .unwrap();
    assert!(matches!(
        meta.insert_browse_allowlist_entry(&dup),
        Err(MetaError::Conflict(_))
    ));
    assert!(meta.disable_browse_allowlist_entry(&id("a1"), 5).is_err());
    let off = meta.disable_browse_allowlist_entry(&id("a1"), 1).unwrap();
    assert!(!off.enabled);
    assert!(meta.disable_browse_allowlist_entry(&id("a1"), 2).is_err());
}

#[test]
fn only_takeover_sessions_can_be_cancelled() {
    let root = temp_root("cancel");
    let meta = open_meta(&root);
    populate_pre_080(&meta);
    build_state(&meta);
    assert!(
        meta.cancel_browse_session(&id("s1"), 1).is_err(),
        "completed is final"
    );
    assert!(
        meta.cancel_browse_session(&id("s2"), 9).is_err(),
        "stale revision"
    );
    let c = meta.cancel_browse_session(&id("s2"), 1).unwrap();
    assert_eq!(c.state, BrowseSessionState::Cancelled);
    assert!(meta.cancel_browse_session(&id("s2"), 2).is_err());
    meta.verify_browse_consistency().unwrap();
}

#[test]
fn consistency_check_detects_invariant_breaks() {
    let breaks: [(&str, &str); 3] = [
        (
            "missing receipt",
            "DELETE FROM browse_receipts WHERE session_id = 's3'",
        ),
        (
            "orphan evidence",
            "DELETE FROM browse_sessions WHERE session_id = 's1'; DELETE FROM browse_receipts WHERE session_id = 's1'",
        ),
        (
            "extra download",
            "DELETE FROM browse_downloads WHERE download_id = 's1-dl'",
        ),
    ];
    for (name, sql) in breaks {
        let root = temp_root(&format!("break-{}", name.replace(' ', "-")));
        {
            let meta = open_meta(&root);
            populate_pre_080(&meta);
            build_state(&meta);
            meta.verify_browse_consistency().unwrap();
        }
        raw(&root).execute_batch(sql).unwrap();
        let meta = open_meta(&root);
        assert!(
            matches!(
                meta.verify_browse_consistency(),
                Err(MetaError::CorruptObjectBody(_))
            ),
            "{name} must be detected"
        );
    }
}

#[test]
fn backup_restore_roundtrips_every_080_row_exactly() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    let (a, b) = (&original.meta, &restored.meta);
    assert_eq!(
        a.list_all_browse_allowlist().unwrap(),
        b.list_all_browse_allowlist().unwrap()
    );
    assert_eq!(
        a.list_all_browse_sessions().unwrap(),
        b.list_all_browse_sessions().unwrap()
    );
    assert_eq!(
        a.list_all_browse_evidence().unwrap(),
        b.list_all_browse_evidence().unwrap()
    );
    assert_eq!(
        a.list_all_browse_downloads().unwrap(),
        b.list_all_browse_downloads().unwrap()
    );
    assert_eq!(
        a.list_all_browse_receipts().unwrap(),
        b.list_all_browse_receipts().unwrap()
    );
    let manifest: BackupManifest =
        serde_json::from_slice(&fs::read(dest.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.schema_version, CURRENT_META_SCHEMA_VERSION);
}

#[test]
fn restore_rejects_hand_edited_080_snapshots() {
    type Edit = (&'static str, fn(&mut serde_json::Value));
    let edits: [Edit; 10] = [
        ("evidence bytes changed", |s| {
            rows(s, "browse_evidence")[0]["content_hex"] = serde_json::json!("00");
        }),
        // Four bytes whose first pair splits a UTF-8 character: refused as
        // corrupt, never a panicking slice.
        ("hex not ASCII", |s| {
            rows(s, "browse_evidence")[0]["content_hex"] = serde_json::json!("a\u{e9}b");
        }),
        ("hex with signs", |s| {
            rows(s, "browse_evidence")[0]["content_hex"] = serde_json::json!("+f+f");
        }),
        ("duplicate session", |s| {
            let first = rows(s, "browse_sessions")[0].clone();
            rows(s, "browse_sessions").push(first);
        }),
        ("receipt removed", |s| {
            rows(s, "browse_receipts").pop();
        }),
        ("allowlist ip literal", |s| {
            rows(s, "browse_allowlist")[0]["host"] = serde_json::json!("127.0.0.1");
        }),
        ("allowlist dot-segment prefix", |s| {
            rows(s, "browse_allowlist")[0]["path_prefix"] = serde_json::json!("/docs/../admin");
        }),
        ("allowlist names a missing project", |s| {
            rows(s, "browse_allowlist")[0]["project_id"] = serde_json::json!("proj-missing");
        }),
        ("session moved to another scope", |s| {
            rows(s, "browse_sessions")[0]["header"]["authority_scope_id"] =
                serde_json::json!("scope-2");
        }),
        ("receipt request digest swapped", |s| {
            let other = rows(s, "browse_evidence")[0]["evidence"]["content_digest"].clone();
            rows(s, "browse_receipts")[0]["request_digest"] = other;
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
fn pre_080_v8_backup_restores_with_empty_browse_tables() {
    let root = temp_root("v8-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_pre_080(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    tamper_backup(&dest, |s| {
        let o = s.as_object_mut().unwrap();
        o.retain(|k, _| {
            !k.starts_with("browse_")
                && !k.starts_with("audio_")
                && !k.starts_with("analytics_")
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
        o.insert("schema_version".to_owned(), serde_json::json!(8));
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.list_all_browse_sessions().unwrap().is_empty());
    restored.meta.verify_browse_consistency().unwrap();
}

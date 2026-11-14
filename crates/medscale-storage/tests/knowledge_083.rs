//! Spec 083 Knowledge + Research Canvas storage + migration integration
//! tests.
//!
//! Synthetic data only. Additive v11 -> v12 migration, crash recovery,
//! atomic index versions with digest-checked chunk reads, contiguous Canvas
//! revisions, cross-row invariants and backup/restore (including tampered
//! snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::knowledge::{
    CanvasNode, CanvasNodeContent, CanvasRevision, EvidenceSpanRef, Freshness, IndexChunk,
    IndexManifest, IndexSourceRef, IndexStatus, KNOWLEDGE_SCHEMA_VERSION, KnowledgeSourceKind,
    LEXICAL_TOKENIZER, ReceiptHit, RetrievalDenyReason, RetrievalOutcome, RetrievalPlan,
    RetrievalReceipt, RetrievalStage, SpanLocator, chunks_digest,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, MetaError, SqliteMetaStore, SyntheticVault, backup_vault,
    restore_vault,
};

const KNOWLEDGE_TABLES: [&str; 4] = [
    "knowledge_manifests",
    "knowledge_chunks",
    "knowledge_receipts",
    "knowledge_canvases",
];

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-083-{name}-{}", std::process::id()));
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
        schema_version: KNOWLEDGE_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn populate_pre_083(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
}

fn rewind_to_v11(root: &Path) {
    let conn = raw(root);
    for table in KNOWLEDGE_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    // Later additive versions (Spec 084 v13, ...) are absent in a v11 build too.
    for table in [
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
    conn.execute("DELETE FROM migration_journal WHERE version >= 12", [])
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

fn pre_083_view(meta: &SqliteMetaStore) -> serde_json::Value {
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&meta.snapshot_bytes().unwrap()).unwrap();
    let object = snapshot.as_object_mut().unwrap();
    object.remove("schema_version");
    // Id sequences are allocation counters, not rows; their backup and
    // restore are covered by `id_sequences_restore`.
    object.remove("id_sequences");
    object.retain(|key, _| {
        !key.starts_with("knowledge_")
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

fn source() -> IndexSourceRef {
    IndexSourceRef {
        kind: KnowledgeSourceKind::DataSnapshot,
        object_id: id("snap-1"),
        content_digest: DigestSha256::of(b"snap-1"),
        lineage_id: id("src-1"),
    }
}

fn span(row: u64, text: &str) -> EvidenceSpanRef {
    EvidenceSpanRef {
        source: source(),
        locator: SpanLocator::Cell {
            row,
            column: "note".to_owned(),
        },
        char_start: 0,
        char_end: text.chars().count() as u32,
    }
}

fn version(mid: &str, v: u32) -> (IndexManifest, Vec<IndexChunk>) {
    let chunks: Vec<IndexChunk> = ["myalgia after statin", "no complaints"]
        .iter()
        .enumerate()
        .map(|(i, text)| IndexChunk {
            seq: i as u32 + 1,
            span: span(i as u64, text),
            text: (*text).to_owned(),
        })
        .collect();
    (
        IndexManifest {
            header: h(mid),
            project_id: id("proj-1"),
            version: v,
            tokenizer: LEXICAL_TOKENIZER.to_owned(),
            sources: vec![source()],
            chunk_count: chunks.len() as u32,
            chunks_digest: chunks_digest(&chunks),
        },
        chunks,
    )
}

fn receipt(rid: &str, manifest: &IndexManifest) -> RetrievalReceipt {
    RetrievalReceipt {
        header: h(rid),
        project_id: id("proj-1"),
        query: "myalgia".to_owned(),
        query_digest: DigestSha256::of(b"myalgia"),
        plan: Some(RetrievalPlan {
            stages: vec![RetrievalStage::Lexical],
            terms: vec!["myalgia".to_owned()],
            max_hits: 10,
            include_stale: false,
        }),
        manifest_id: Some(manifest.header.id.clone()),
        manifest_chunks_digest: Some(manifest.chunks_digest.clone()),
        index_status: Some(IndexStatus {
            manifest_id: manifest.header.id.clone(),
            version: manifest.version,
            chunk_count: manifest.chunk_count,
            current_sources: 1,
            superseded_sources: 0,
            tombstoned_sources: 0,
            unindexed_sources: 0,
        }),
        outcome: RetrievalOutcome::Results,
        deny_reason: None,
        hits: vec![ReceiptHit {
            rank: 1,
            chunk_seq: 1,
            score: 1567,
            span: span(0, "myalgia after statin"),
            freshness: Freshness::Current,
        }],
    }
}

fn denied(rid: &str) -> RetrievalReceipt {
    RetrievalReceipt {
        header: h(rid),
        project_id: id("proj-1"),
        query: String::new(),
        query_digest: DigestSha256::of(b""),
        plan: None,
        manifest_id: None,
        manifest_chunks_digest: None,
        index_status: None,
        outcome: RetrievalOutcome::Denied,
        deny_reason: Some(RetrievalDenyReason::EmptyQuery),
        hits: Vec::new(),
    }
}

fn canvas(cid: &str, revision: u32) -> CanvasRevision {
    let mut nodes = vec![CanvasNode {
        key: "claim".to_owned(),
        content: CanvasNodeContent::Note {
            text: "LDL fell".to_owned(),
        },
    }];
    if revision > 1 {
        nodes.push(CanvasNode {
            key: "e1".to_owned(),
            content: CanvasNodeContent::Evidence {
                span: span(0, "myalgia after statin"),
            },
        });
    }
    CanvasRevision {
        header: h(cid),
        project_id: id("proj-1"),
        revision,
        title: "Statins".to_owned(),
        nodes,
        edges: Vec::new(),
    }
}

fn build_state(meta: &SqliteMetaStore) {
    let (m1, c1) = version("idx-1", 1);
    meta.commit_index_version(&m1, &c1).unwrap();
    let (m2, c2) = version("idx-2", 2);
    meta.commit_index_version(&m2, &c2).unwrap();
    meta.insert_retrieval_receipt(&receipt("kr-1", &m1))
        .unwrap();
    meta.insert_retrieval_receipt(&denied("kr-2")).unwrap();
    meta.insert_canvas_revision(&canvas("canvas-1", 1)).unwrap();
    meta.insert_canvas_revision(&canvas("canvas-1", 2)).unwrap();
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
    populate_pre_083(&vault.meta);
    build_state(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    dest
}

#[test]
fn migration_v11_to_v12_is_additive() {
    let root = temp_root("migrate");
    let before = {
        let meta = open_meta(&root);
        populate_pre_083(&meta);
        pre_083_view(&meta)
    };
    rewind_to_v11(&root);
    for t in KNOWLEDGE_TABLES {
        assert!(!table_exists(&root, t));
    }
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    for t in KNOWLEDGE_TABLES {
        assert!(table_exists(&root, t), "{t}");
    }
    assert_eq!(pre_083_view(&meta), before);
    drop(meta);
    // Reopening an already-migrated store changes nothing.
    let meta = open_meta(&root);
    assert_eq!(pre_083_view(&meta), before);
}

#[test]
fn crash_mid_v12_migration_fails_closed_and_backup_recovers() {
    let root = temp_root("crash");
    let vault_root = root.join("vault");
    {
        let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
        populate_pre_083(&vault.meta);
        backup_vault(&vault, &root.join("checkpoint")).unwrap();
    }
    rewind_to_v11(&vault_root);
    raw(&vault_root)
        .execute(
            "INSERT INTO migration_journal(version, state) VALUES (12, 'started')",
            [],
        )
        .unwrap();
    match SqliteMetaStore::open_at(&db_path(&vault_root)) {
        Err(MetaError::MigrationIncomplete(12)) => {}
        other => panic!("interrupted v12 migration must fail closed, got {other:?}"),
    }
    restore_vault(&root.join("checkpoint"), &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.get_project(&id("proj-1")).is_ok());
}

#[test]
fn index_versions_are_atomic_contiguous_and_digest_checked() {
    let root = temp_root("index");
    let meta = open_meta(&root);
    populate_pre_083(&meta);
    let (m1, c1) = version("idx-1", 1);
    // A wrong digest, a skipped version, or a chunk from an unindexed
    // source writes nothing.
    let mut wrong = m1.clone();
    wrong.chunks_digest = DigestSha256::of(b"other");
    assert!(meta.commit_index_version(&wrong, &c1).is_err());
    let (skip, cs) = version("idx-9", 2);
    assert!(meta.commit_index_version(&skip, &cs).is_err());
    let mut foreign = c1.clone();
    foreign[0].span.source.object_id = id("snap-9");
    let mut m_foreign = m1.clone();
    m_foreign.chunks_digest = chunks_digest(&foreign);
    assert!(meta.commit_index_version(&m_foreign, &foreign).is_err());
    assert_eq!(meta.latest_index_manifest(&id("proj-1")).unwrap(), None);
    let count: i64 = raw(&root)
        .query_row("SELECT COUNT(*) FROM knowledge_chunks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "no partial chunk rows");

    meta.commit_index_version(&m1, &c1).unwrap();
    assert!(matches!(
        meta.commit_index_version(&m1, &c1),
        Err(MetaError::Conflict(_))
    ));
    let (m2, c2) = version("idx-2", 2);
    meta.commit_index_version(&m2, &c2).unwrap();
    assert_eq!(
        meta.latest_index_manifest(&id("proj-1")).unwrap(),
        Some(m2.clone())
    );
    assert!(meta.latest_index_manifest(&id("proj-2")).unwrap().is_none());
    assert_eq!(meta.get_index_chunks(&m1).unwrap(), c1);

    // An edited chunk no longer matches its manifest digest.
    let mut edited = c1[0].clone();
    edited.text = "myalgia BEFORE statin".to_owned();
    raw(&root)
        .execute(
            "UPDATE knowledge_chunks SET body_json = ?1 WHERE manifest_id = 'idx-1' AND seq = 1",
            [serde_json::to_string(&edited).unwrap()],
        )
        .unwrap();
    assert!(matches!(
        meta.get_index_chunks(&m1),
        Err(MetaError::CorruptObjectBody(_))
    ));
    raw(&root)
        .execute(
            "DELETE FROM knowledge_chunks WHERE manifest_id = 'idx-2' AND seq = 2",
            [],
        )
        .unwrap();
    assert!(matches!(
        meta.get_index_chunks(&m2),
        Err(MetaError::CorruptObjectBody(_))
    ));
}

#[test]
fn receipts_and_canvas_revisions_hold_their_invariants() {
    let root = temp_root("canvas");
    let meta = open_meta(&root);
    populate_pre_083(&meta);
    let (m1, c1) = version("idx-1", 1);
    meta.commit_index_version(&m1, &c1).unwrap();
    let mut bad = receipt("kr-0", &m1);
    bad.outcome = RetrievalOutcome::InsufficientEvidence;
    assert!(meta.insert_retrieval_receipt(&bad).is_err());
    meta.insert_retrieval_receipt(&receipt("kr-1", &m1))
        .unwrap();
    assert!(matches!(
        meta.insert_retrieval_receipt(&receipt("kr-1", &m1)),
        Err(MetaError::Conflict(_))
    ));
    assert_eq!(
        meta.list_retrieval_receipts(&id("proj-1")).unwrap().len(),
        1
    );

    assert!(
        meta.insert_canvas_revision(&canvas("canvas-1", 2)).is_err(),
        "revisions start at 1"
    );
    meta.insert_canvas_revision(&canvas("canvas-1", 1)).unwrap();
    assert!(meta.insert_canvas_revision(&canvas("canvas-1", 1)).is_err());
    assert!(meta.insert_canvas_revision(&canvas("canvas-1", 3)).is_err());
    let mut moved = canvas("canvas-1", 2);
    moved.project_id = id("proj-2");
    assert!(meta.insert_canvas_revision(&moved).is_err());
    meta.insert_canvas_revision(&canvas("canvas-1", 2)).unwrap();
    assert_eq!(meta.get_canvas(&id("canvas-1"), None).unwrap().revision, 2);
    assert_eq!(
        meta.get_canvas(&id("canvas-1"), Some(1)).unwrap(),
        canvas("canvas-1", 1)
    );
    let listed = meta.list_canvases(&id("proj-1")).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].revision, 2);
}

#[test]
fn consistency_check_detects_invariant_breaks() {
    let root = temp_root("consistency");
    let meta = open_meta(&root);
    populate_pre_083(&meta);
    build_state(&meta);
    meta.verify_knowledge_consistency().unwrap();
    raw(&root)
        .execute(
            "DELETE FROM knowledge_canvases WHERE canvas_id = 'canvas-1' AND revision = 1",
            [],
        )
        .unwrap();
    assert!(
        meta.verify_knowledge_consistency().is_err(),
        "canvas revisions are contiguous"
    );
    meta.insert_canvas_revision(&canvas("canvas-2", 1)).unwrap();
    raw(&root)
        .execute(
            "DELETE FROM knowledge_canvases WHERE canvas_id = 'canvas-1'",
            [],
        )
        .unwrap();
    meta.verify_knowledge_consistency().unwrap();
    raw(&root)
        .execute(
            "DELETE FROM knowledge_manifests WHERE manifest_id = 'idx-1'",
            [],
        )
        .unwrap();
    assert!(
        meta.verify_knowledge_consistency().is_err(),
        "a receipt names its index; versions are contiguous"
    );
}

#[test]
fn backup_restore_roundtrips_every_083_row_exactly() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        original.meta.list_all_index_versions().unwrap(),
        restored.meta.list_all_index_versions().unwrap()
    );
    assert_eq!(
        original.meta.list_all_retrieval_receipts().unwrap(),
        restored.meta.list_all_retrieval_receipts().unwrap()
    );
    assert_eq!(
        original.meta.list_all_canvas_revisions().unwrap(),
        restored.meta.list_all_canvas_revisions().unwrap()
    );
    restored.meta.verify_knowledge_consistency().unwrap();
}

#[test]
fn restore_rejects_hand_edited_083_snapshots() {
    type Edit = (&'static str, fn(&mut serde_json::Value));
    let edits: [Edit; 17] = [
        ("chunk text changed", |s| {
            rows(s, "knowledge_index_versions")[0]["chunks"][0]["text"] =
                serde_json::json!("myalgia BEFORE statin");
        }),
        ("chunk dropped", |s| {
            rows(s, "knowledge_index_versions")[0]["chunks"]
                .as_array_mut()
                .unwrap()
                .pop();
        }),
        ("index version removed", |s| {
            rows(s, "knowledge_index_versions").remove(0);
        }),
        ("receipt names another digest", |s| {
            rows(s, "knowledge_receipts")[0]["manifest_chunks_digest"] =
                serde_json::to_value(DigestSha256::of(b"other")).unwrap();
        }),
        ("receipt query edited", |s| {
            rows(s, "knowledge_receipts")[0]["query"] = serde_json::json!("statin");
        }),
        ("canvas revision removed", |s| {
            rows(s, "knowledge_canvases").remove(0);
        }),
        ("canvas in another scope", |s| {
            rows(s, "knowledge_canvases")[0]["header"]["authority_scope_id"] =
                serde_json::json!("scope-2");
        }),
        ("duplicate receipt", |s| {
            let first = rows(s, "knowledge_receipts")[0].clone();
            rows(s, "knowledge_receipts").push(first);
        }),
        ("receipt family replaced by a string", |s| {
            s["knowledge_receipts"] = serde_json::json!("receipts");
        }),
        ("canvas family dropped", |s| {
            s.as_object_mut().unwrap().remove("knowledge_canvases");
        }),
        ("index versions replaced by an object", |s| {
            s["knowledge_index_versions"] = serde_json::json!({});
        }),
        ("chunk replaced by a number", |s| {
            rows(s, "knowledge_index_versions")[0]["chunks"][0] = serde_json::json!(42);
        }),
        ("receipt hit names another chunk", |s| {
            rows(s, "knowledge_receipts")[0]["hits"][0]["chunk_seq"] = serde_json::json!(2);
        }),
        ("receipt hit names no chunk", |s| {
            rows(s, "knowledge_receipts")[0]["hits"][0]["chunk_seq"] = serde_json::json!(0);
        }),
        ("receipt hit span moved", |s| {
            rows(s, "knowledge_receipts")[0]["hits"][0]["span"]["locator"]["row"] =
                serde_json::json!(7);
        }),
        ("receipt hit names another project's source", |s| {
            rows(s, "knowledge_receipts")[0]["hits"][0]["span"]["source"]["object_id"] =
                serde_json::json!("snap-other-project");
        }),
        ("receipt hit replaced by a string", |s| {
            rows(s, "knowledge_receipts")[0]["hits"][0] = serde_json::json!("hit");
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
fn pre_083_v11_backup_restores_with_empty_knowledge_tables() {
    let root = temp_root("v11-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_pre_083(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    tamper_backup(&dest, |s| {
        let o = s.as_object_mut().unwrap();
        o.retain(|k, _| {
            !k.starts_with("knowledge_")
                && !k.starts_with("hub_")
                && !k.starts_with("compute_")
                && !k.starts_with("rws_")
                && !k.starts_with("ext_")
                && !k.starts_with("hud_")
                && !k.starts_with("rp_")
                && !k.starts_with("ia_")
                && !k.starts_with("fed_")
        });
        o.insert("schema_version".to_owned(), serde_json::json!(11));
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(
        restored
            .meta
            .list_all_retrieval_receipts()
            .unwrap()
            .is_empty()
    );
    restored.meta.verify_knowledge_consistency().unwrap();
}

//! Spec 081 AudioFlow storage + migration integration tests.
//!
//! Synthetic data only (byte patterns, never real audio). Additive v9 -> v10
//! migration, crash recovery, capture chunk atomicity, immutable sources
//! with digest checks on read, transcript lineage, cross-row invariants and
//! backup/restore (including tampered snapshots).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::audio::{
    AUDIO_SCHEMA_VERSION, AsrEngineIdentity, AudioLimitation, AudioRoute, AudioRouteDecision,
    AudioRouteDenyReason, AudioSession, AudioSource, AudioSourceKind, CaptureBackendKind,
    CaptureHealth, CaptureState, DiarizationState, PcmFormat, SegmentStatus, SpeakerLabel,
    TranscriptOrigin, TranscriptReceipt, TranscriptRevision, TranscriptSegment, VadParameters,
    VoiceInputMode,
};
use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use medscale_contracts::project_graph::Project;
use medscale_storage::{
    CURRENT_META_SCHEMA_VERSION, MetaError, SqliteMetaStore, SyntheticVault, backup_vault,
    restore_vault,
};

const AUDIO_TABLES: [&str; 5] = [
    "audio_sources",
    "audio_capture_sessions",
    "audio_capture_chunks",
    "audio_transcripts",
    "audio_transcript_receipts",
];

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-081-{name}-{}", std::process::id()));
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
        schema_version: AUDIO_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-1"),
        authority_scope_id: AuthorityScopeId::new("scope-1"),
    }
}

fn id(v: &str) -> OpaqueId {
    OpaqueId::new(v)
}

fn populate_pre_081(meta: &SqliteMetaStore) {
    meta.insert_project(&Project::new(h("proj-1"), "project-1".to_owned(), None).unwrap())
        .unwrap();
}

fn rewind_to_v9(root: &Path) {
    let conn = raw(root);
    for table in AUDIO_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table};")).unwrap();
    }
    // Later additive versions (Spec 082 v11, ...) are absent in a v9 build too.
    for table in [
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
    ] {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {table};"))
            .unwrap();
    }
    conn.execute("DELETE FROM migration_journal WHERE version >= 10", [])
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

fn pre_081_view(meta: &SqliteMetaStore) -> serde_json::Value {
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&meta.snapshot_bytes().unwrap()).unwrap();
    let object = snapshot.as_object_mut().unwrap();
    object.remove("schema_version");
    // Id sequences are allocation counters, not rows; their backup and
    // restore are covered by `id_sequences_restore`.
    object.remove("id_sequences");
    object.retain(|key, _| {
        !key.starts_with("audio_")
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

/// A synthetic "WAV" payload: storage checks digests, not audio.
fn bytes(n: usize, seed: u8) -> Vec<u8> {
    (0..n).map(|i| seed.wrapping_add(i as u8)).collect()
}

fn imported(source_id: &str, content: &[u8]) -> AudioSource {
    AudioSource {
        header: h(source_id),
        project_id: id("proj-1"),
        kind: AudioSourceKind::ImportedFile,
        label: "synthetic import".to_owned(),
        format: PcmFormat::mono_16k(),
        duration_ms: 2_000,
        byte_length: content.len() as u64,
        content_digest: DigestSha256::of(content),
        health: CaptureHealth::Ok,
        capture_session_id: None,
    }
}

fn capture(session_id: &str) -> AudioSession {
    AudioSession::start(
        h(session_id),
        id("proj-1"),
        "synthetic capture".to_owned(),
        CaptureBackendKind::Scripted,
        PcmFormat::mono_16k(),
        id("holder-1"),
    )
}

fn capture_source(source_id: &str, session_id: &str, content: &[u8]) -> AudioSource {
    AudioSource {
        kind: AudioSourceKind::Capture,
        capture_session_id: Some(id(session_id)),
        ..imported(source_id, content)
    }
}

fn seg(seq: u32, start: u64, end: u64, text: Option<&str>) -> TranscriptSegment {
    TranscriptSegment {
        seq,
        start_ms: start,
        end_ms: end,
        speaker: SpeakerLabel::Unknown,
        status: if text.is_some() {
            SegmentStatus::Transcribed
        } else {
            SegmentStatus::Untranscribed
        },
        text: text.map(str::to_owned),
    }
}

fn fixture_engine() -> AsrEngineIdentity {
    AsrEngineIdentity {
        engine_id: "medscale-fixture-asr".to_owned(),
        version: "1".to_owned(),
        is_fixture: true,
    }
}

fn engine_revision(rid: &str, source: &AudioSource, no: u32) -> TranscriptRevision {
    let segments = vec![seg(1, 100, 900, Some("a")), seg(2, 1_200, 1_800, Some("b"))];
    TranscriptRevision {
        header: h(rid),
        project_id: id("proj-1"),
        source_id: source.header.id.clone(),
        source_digest: source.content_digest.clone(),
        revision_no: no,
        origin: TranscriptOrigin::Engine {
            route: AudioRoute::FixtureAsr,
            engine: Some(fixture_engine()),
            vad: VadParameters::FROZEN,
        },
        mode: VoiceInputMode::Dictation,
        language: Some("en".to_owned()),
        diarization: DiarizationState::NotRequested,
        state: TranscriptRevision::derived_state(&segments),
        segments,
    }
}

fn receipt_for(rcpt: &str, t: &TranscriptRevision) -> TranscriptReceipt {
    TranscriptReceipt {
        header: h(rcpt),
        project_id: id("proj-1"),
        source_id: t.source_id.clone(),
        source_digest: t.source_digest.clone(),
        request_digest: DigestSha256::of(b"request"),
        route: AudioRoute::FixtureAsr,
        decision: AudioRouteDecision::allow(),
        transcript_revision_id: Some(t.header.id.clone()),
        engine: Some(fixture_engine()),
        vad: VadParameters::FROZEN,
        segment_count: u32::try_from(t.segments.len()).unwrap(),
        limitations: vec![
            AudioLimitation::TranscriptNotClinicalTruth,
            AudioLimitation::FixtureEngineIsNotSpeechRecognition,
        ],
    }
}

fn denied_receipt(rcpt: &str, source: &AudioSource) -> TranscriptReceipt {
    TranscriptReceipt {
        header: h(rcpt),
        project_id: id("proj-1"),
        source_id: source.header.id.clone(),
        source_digest: source.content_digest.clone(),
        request_digest: DigestSha256::of(b"cloud"),
        route: AudioRoute::CloudAsr,
        decision: AudioRouteDecision::deny(AudioRouteDenyReason::CloudRouteForbidden),
        transcript_revision_id: None,
        engine: None,
        vad: VadParameters::FROZEN,
        segment_count: 0,
        limitations: vec![AudioLimitation::TranscriptNotClinicalTruth],
    }
}

fn correction(rid: &str, parent: &TranscriptRevision) -> TranscriptRevision {
    let mut segments = parent.segments.clone();
    segments[0].text = Some("corrected".to_owned());
    segments[0].status = SegmentStatus::Corrected;
    TranscriptRevision {
        header: h(rid),
        revision_no: parent.revision_no + 1,
        origin: TranscriptOrigin::HumanCorrection {
            parent_revision_id: parent.header.id.clone(),
            corrected_by: id("holder-1"),
            reason: "synthetic correction".to_owned(),
        },
        state: TranscriptRevision::derived_state(&segments),
        segments,
        ..parent.clone()
    }
}

/// One imported source with an engine revision, a correction and a denied
/// receipt; one stopped capture; one capture left recording with chunks.
fn build_state(meta: &SqliteMetaStore) {
    let content = bytes(4_000, 7);
    let src = imported("src-1", &content);
    meta.insert_imported_audio_source(&src, &content).unwrap();
    let t1 = engine_revision("t1", &src, 1);
    meta.commit_transcript(Some(&t1), &receipt_for("r1", &t1))
        .unwrap();
    meta.commit_transcript(None, &denied_receipt("r2", &src))
        .unwrap();
    meta.insert_transcript_correction(&correction("t2", &t1))
        .unwrap();

    let c1 = capture("cap-1");
    meta.insert_capture_session(&c1).unwrap();
    let c1 = meta
        .append_capture_chunk(&id("cap-1"), 1, &[1, 2, 3, 4])
        .unwrap();
    let wav = bytes(48, 3);
    meta.stop_capture_session(
        &id("cap-1"),
        c1.revision,
        &capture_source("src-2", "cap-1", &wav),
        &wav,
    )
    .unwrap();

    meta.insert_capture_session(&capture("cap-2")).unwrap();
    let c2 = meta.append_capture_chunk(&id("cap-2"), 1, &[9, 9]).unwrap();
    meta.append_capture_chunk(&id("cap-2"), c2.revision, &[8, 8, 8, 8])
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
        .unwrap_or_else(|| panic!("{key} present"))
}

fn backed_up(root: &Path) -> PathBuf {
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_pre_081(&vault.meta);
    build_state(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    dest
}

#[test]
fn migration_v9_to_v10_is_additive() {
    let root = temp_root("migrate");
    let before = {
        let meta = open_meta(&root);
        populate_pre_081(&meta);
        pre_081_view(&meta)
    };
    rewind_to_v9(&root);
    for t in AUDIO_TABLES {
        assert!(!table_exists(&root, t));
    }
    let meta = open_meta(&root);
    assert_eq!(
        meta.migration_journal().unwrap().finished_version,
        CURRENT_META_SCHEMA_VERSION
    );
    for t in AUDIO_TABLES {
        assert!(table_exists(&root, t), "{t}");
    }
    assert_eq!(pre_081_view(&meta), before);
    drop(meta);
    // Re-opening is idempotent.
    let again = open_meta(&root);
    assert_eq!(pre_081_view(&again), before);
}

#[test]
fn crash_mid_v10_migration_fails_closed_and_backup_recovers() {
    let root = temp_root("crash");
    let vault_root = root.join("vault");
    {
        let vault = SyntheticVault::open("vault-1", &vault_root).unwrap();
        populate_pre_081(&vault.meta);
        backup_vault(&vault, &root.join("checkpoint")).unwrap();
    }
    rewind_to_v9(&vault_root);
    raw(&vault_root)
        .execute(
            "INSERT INTO migration_journal(version, state) VALUES (10, 'started')",
            [],
        )
        .unwrap();
    match SqliteMetaStore::open_at(&db_path(&vault_root)) {
        Err(MetaError::MigrationIncomplete(10)) => {}
        other => panic!("interrupted v10 migration must fail closed, got {other:?}"),
    }
    restore_vault(&root.join("checkpoint"), &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.get_project(&id("proj-1")).is_ok());
}

#[test]
fn sources_are_immutable_and_digest_checked() {
    let root = temp_root("sources");
    let meta = open_meta(&root);
    populate_pre_081(&meta);
    let content = bytes(1_000, 1);
    let src = imported("src-1", &content);
    assert!(
        meta.insert_imported_audio_source(&src, b"other bytes")
            .is_err(),
        "digest mismatch refused"
    );
    let as_capture = capture_source("src-x", "cap-x", &content);
    assert!(
        meta.insert_imported_audio_source(&as_capture, &content)
            .is_err(),
        "capture sources come only from a stopped capture"
    );
    meta.insert_imported_audio_source(&src, &content).unwrap();
    assert!(matches!(
        meta.insert_imported_audio_source(&src, &content),
        Err(MetaError::Conflict(_))
    ));
    let (read, read_bytes) = meta.get_audio_source(&id("src-1")).unwrap();
    assert_eq!(read, src);
    assert_eq!(read_bytes, content);

    raw(&root)
        .execute(
            "UPDATE audio_sources SET content = ?1 WHERE source_id = 'src-1'",
            [bytes(1_000, 2)],
        )
        .unwrap();
    assert!(matches!(
        meta.get_audio_source(&id("src-1")),
        Err(MetaError::CorruptObjectBody(_))
    ));
}

#[test]
fn capture_chunks_are_atomic_and_leave_with_the_open_state() {
    let root = temp_root("capture");
    let meta = open_meta(&root);
    populate_pre_081(&meta);
    let mut not_empty = capture("cap-0");
    not_empty.captured_bytes = 2;
    assert!(meta.insert_capture_session(&not_empty).is_err());

    meta.insert_capture_session(&capture("cap-1")).unwrap();
    let s = meta.append_capture_chunk(&id("cap-1"), 1, &[1, 2]).unwrap();
    assert_eq!((s.revision, s.chunk_count, s.captured_bytes), (2, 1, 2));
    assert!(
        matches!(
            meta.append_capture_chunk(&id("cap-1"), 1, &[3, 4]),
            Err(MetaError::Conflict(_))
        ),
        "stale revision"
    );
    let paused = meta
        .transition_capture_session(&id("cap-1"), 2, CaptureState::Paused)
        .unwrap();
    assert!(
        meta.append_capture_chunk(&id("cap-1"), paused.revision, &[5, 6])
            .is_err(),
        "no frames while paused"
    );
    let resumed = meta
        .transition_capture_session(&id("cap-1"), paused.revision, CaptureState::Recording)
        .unwrap();
    let s = meta
        .append_capture_chunk(&id("cap-1"), resumed.revision, &[7, 8])
        .unwrap();
    assert_eq!(
        meta.capture_chunks(&id("cap-1")).unwrap(),
        vec![vec![1, 2], vec![7, 8]]
    );

    // A source that disagrees with its session is refused and changes nothing.
    let wav = bytes(48, 5);
    let mut wrong = capture_source("src-1", "cap-1", &wav);
    wrong.format.sample_rate = 8_000;
    assert!(
        meta.stop_capture_session(&id("cap-1"), s.revision, &wrong, &wav)
            .is_err()
    );
    assert_eq!(meta.capture_chunks(&id("cap-1")).unwrap().len(), 2);
    assert!(matches!(
        meta.get_audio_source(&id("src-1")),
        Err(MetaError::NotFound)
    ));

    let stopped = meta
        .stop_capture_session(
            &id("cap-1"),
            s.revision,
            &capture_source("src-1", "cap-1", &wav),
            &wav,
        )
        .unwrap();
    assert_eq!(stopped.state, CaptureState::Stopped);
    assert_eq!(stopped.source_id, Some(id("src-1")));
    assert!(meta.capture_chunks(&id("cap-1")).unwrap().is_empty());
    assert!(
        meta.transition_capture_session(&id("cap-1"), stopped.revision, CaptureState::Recording)
            .is_err(),
        "stopped is final"
    );

    meta.insert_capture_session(&capture("cap-2")).unwrap();
    meta.append_capture_chunk(&id("cap-2"), 1, &[1, 1]).unwrap();
    let cancelled = meta
        .transition_capture_session(&id("cap-2"), 2, CaptureState::Cancelled)
        .unwrap();
    assert_eq!(cancelled.state, CaptureState::Cancelled);
    assert!(meta.capture_chunks(&id("cap-2")).unwrap().is_empty());
    assert!(
        meta.transition_capture_session(&id("cap-2"), 3, CaptureState::Stopped)
            .is_err()
    );
    meta.verify_audio_consistency().unwrap();
}

#[test]
fn transcripts_form_a_contiguous_lineage_over_immutable_sources() {
    let root = temp_root("transcripts");
    let meta = open_meta(&root);
    populate_pre_081(&meta);
    let content = bytes(2_000, 4);
    let src = imported("src-1", &content);
    meta.insert_imported_audio_source(&src, &content).unwrap();

    let t1 = engine_revision("t1", &src, 1);
    let mut mismatched = receipt_for("r0", &t1);
    mismatched.segment_count = 9;
    assert!(meta.commit_transcript(Some(&t1), &mismatched).is_err());
    assert!(matches!(
        meta.get_transcript_revision(&id("t1")),
        Err(MetaError::NotFound)
    ));
    let skip = engine_revision("t5", &src, 5);
    assert!(
        meta.commit_transcript(Some(&skip), &receipt_for("r5", &skip))
            .is_err(),
        "revision numbers are contiguous"
    );
    let mut wrong_digest = engine_revision("t1", &src, 1);
    wrong_digest.source_digest = DigestSha256::of(b"other");
    assert!(
        meta.commit_transcript(Some(&wrong_digest), &receipt_for("r1", &wrong_digest))
            .is_err()
    );
    meta.commit_transcript(Some(&t1), &receipt_for("r1", &t1))
        .unwrap();
    meta.commit_transcript(None, &denied_receipt("r2", &src))
        .unwrap();
    assert!(
        meta.insert_transcript_correction(&engine_revision("t9", &src, 2))
            .is_err(),
        "engine revisions need their receipt"
    );
    let t2 = correction("t2", &t1);
    meta.insert_transcript_correction(&t2).unwrap();
    assert_eq!(
        meta.get_transcript_revision(&id("t1")).unwrap(),
        t1,
        "the parent is unchanged"
    );
    let list = meta.list_transcript_revisions(&id("src-1")).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(
        meta.list_transcript_receipts(&id("src-1")).unwrap().len(),
        2
    );
    meta.verify_audio_consistency().unwrap();

    // A tampered revision body no longer matches its source and is refused.
    let mut edited = t2.clone();
    edited.segments[1].end_ms = 99_999;
    raw(&root)
        .execute(
            "UPDATE audio_transcripts SET body_json = ?1 WHERE revision_id = 't2'",
            [serde_json::to_string(&edited).unwrap()],
        )
        .unwrap();
    assert!(matches!(
        meta.get_transcript_revision(&id("t2")),
        Err(MetaError::CorruptObjectBody(_))
    ));
}

#[test]
fn consistency_check_detects_invariant_breaks() {
    let root = temp_root("consistency");
    let meta = open_meta(&root);
    populate_pre_081(&meta);
    build_state(&meta);
    meta.verify_audio_consistency().unwrap();

    raw(&root)
        .execute(
            "INSERT INTO audio_capture_chunks(session_id, seq, content) VALUES ('cap-1', 1, x'00')",
            [],
        )
        .unwrap();
    assert!(
        meta.verify_audio_consistency().is_err(),
        "a stopped capture holds no chunks"
    );
    raw(&root)
        .execute(
            "DELETE FROM audio_capture_chunks WHERE session_id = 'cap-1'",
            [],
        )
        .unwrap();
    meta.verify_audio_consistency().unwrap();

    raw(&root)
        .execute(
            "DELETE FROM audio_transcript_receipts WHERE receipt_id = 'r1'",
            [],
        )
        .unwrap();
    assert!(
        meta.verify_audio_consistency().is_err(),
        "an engine revision needs its receipt"
    );
}

#[test]
fn backup_restore_roundtrips_every_081_row_exactly() {
    let root = temp_root("roundtrip");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        original.meta.list_all_audio_sources().unwrap(),
        restored.meta.list_all_audio_sources().unwrap()
    );
    assert_eq!(
        original.meta.list_all_capture_sessions().unwrap(),
        restored.meta.list_all_capture_sessions().unwrap()
    );
    assert_eq!(
        original.meta.list_all_capture_chunks().unwrap(),
        restored.meta.list_all_capture_chunks().unwrap()
    );
    assert_eq!(
        original.meta.list_all_transcript_revisions().unwrap(),
        restored.meta.list_all_transcript_revisions().unwrap()
    );
    assert_eq!(
        original.meta.list_all_transcript_receipts().unwrap(),
        restored.meta.list_all_transcript_receipts().unwrap()
    );
    restored.meta.verify_audio_consistency().unwrap();
    let (_, bytes) = restored.meta.get_audio_source(&id("src-1")).unwrap();
    assert_eq!(bytes.len(), 4_000);
}

#[test]
fn restore_rejects_hand_edited_081_snapshots() {
    type Edit = (&'static str, fn(&mut serde_json::Value));
    let edits: [Edit; 12] = [
        ("source bytes changed", |s| {
            rows(s, "audio_sources")[0]["content_hex"] = serde_json::json!("00");
        }),
        // Four bytes whose first pair splits a UTF-8 character: refused as
        // corrupt, never a panicking slice.
        ("hex not ASCII", |s| {
            rows(s, "audio_sources")[0]["content_hex"] = serde_json::json!("a\u{e9}b");
        }),
        ("hex with signs", |s| {
            rows(s, "audio_sources")[0]["content_hex"] = serde_json::json!("+f+f");
        }),
        ("source names a missing project", |s| {
            rows(s, "audio_sources")[0]["source"]["project_id"] = serde_json::json!("proj-x");
        }),
        ("source moved to another scope", |s| {
            rows(s, "audio_sources")[0]["source"]["header"]["authority_scope_id"] =
                serde_json::json!("scope-2");
        }),
        ("duplicate transcript", |s| {
            let first = rows(s, "audio_transcripts")[0].clone();
            rows(s, "audio_transcripts").push(first);
        }),
        ("transcript revision gap", |s| {
            rows(s, "audio_transcripts")[1]["revision_no"] = serde_json::json!(3);
        }),
        ("engine receipt removed", |s| {
            rows(s, "audio_transcript_receipts").remove(0);
        }),
        ("correction parent swapped", |s| {
            rows(s, "audio_transcripts")[1]["origin"]["parent_revision_id"] =
                serde_json::json!("t-missing");
        }),
        ("capture chunk dropped", |s| {
            rows(s, "audio_capture_chunks").pop();
        }),
        ("stopped capture loses its source", |s| {
            rows(s, "audio_sources").pop();
        }),
        ("speaker named", |s| {
            rows(s, "audio_transcripts")[0]["segments"][0]["speaker"] =
                serde_json::json!({"kind": "anonymous", "index": 1});
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
fn pre_081_v9_backup_restores_with_empty_audio_tables() {
    let root = temp_root("v9-backup");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_pre_081(&vault.meta);
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    tamper_backup(&dest, |s| {
        let o = s.as_object_mut().unwrap();
        o.retain(|k, _| {
            !k.starts_with("audio_")
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
        o.insert("schema_version".to_owned(), serde_json::json!(9));
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert!(restored.meta.list_all_audio_sources().unwrap().is_empty());
    restored.meta.verify_audio_consistency().unwrap();
}

#[test]
fn unreferenced_imported_audio_removal_preserves_other_sources_and_prior_backups() {
    let root = temp_root("remove-unreferenced");
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    populate_pre_081(&vault.meta);
    let a_bytes = bytes(128, 3);
    let b_bytes = bytes(160, 19);
    let a = imported("source-to-remove", &a_bytes);
    let b = imported("source-to-keep", &b_bytes);
    vault
        .meta
        .insert_imported_audio_source(&a, &a_bytes)
        .unwrap();
    vault
        .meta
        .insert_imported_audio_source(&b, &b_bytes)
        .unwrap();

    let prior_backup = root.join("pre-removal-backup");
    backup_vault(&vault, &prior_backup).unwrap();
    assert!(matches!(
        vault.meta.remove_unreferenced_imported_audio_source(
            &id("other-project"),
            &a.header.id,
            &a.content_digest
        ),
        Err(MetaError::NotFound)
    ));
    assert_eq!(
        vault.meta.get_audio_source(&a.header.id).unwrap().1,
        a_bytes
    );

    vault
        .meta
        .remove_unreferenced_imported_audio_source(&a.project_id, &a.header.id, &a.content_digest)
        .unwrap();
    assert!(matches!(
        vault.meta.get_audio_source(&a.header.id),
        Err(MetaError::NotFound)
    ));
    assert_eq!(
        vault.meta.get_audio_source(&b.header.id).unwrap().1,
        b_bytes
    );
    vault.meta.verify_audio_consistency().unwrap();
    assert!(matches!(
        vault.meta.remove_unreferenced_imported_audio_source(
            &a.project_id,
            &a.header.id,
            &a.content_digest
        ),
        Err(MetaError::NotFound)
    ));

    let next_backup = root.join("post-removal-backup");
    backup_vault(&vault, &next_backup).unwrap();
    restore_vault(&next_backup, &root.join("restored-current")).unwrap();
    let current = SyntheticVault::open("vault-1", &root.join("restored-current")).unwrap();
    assert!(matches!(
        current.meta.get_audio_source(&a.header.id),
        Err(MetaError::NotFound)
    ));
    assert_eq!(
        current.meta.get_audio_source(&b.header.id).unwrap().1,
        b_bytes
    );
    current.meta.verify_audio_consistency().unwrap();

    // Earlier backups remain intact; this operation must never claim to
    // retroactively erase old snapshots or all copies on physical storage.
    restore_vault(&prior_backup, &root.join("restored-previous")).unwrap();
    let prior = SyntheticVault::open("vault-1", &root.join("restored-previous")).unwrap();
    assert_eq!(
        prior.meta.get_audio_source(&a.header.id).unwrap().1,
        a_bytes
    );
    prior.meta.verify_audio_consistency().unwrap();
}

#[test]
fn audio_removal_refuses_revisions_receipts_capture_and_stale_digest() {
    let root = temp_root("remove-refusals");
    let meta = open_meta(&root);
    populate_pre_081(&meta);
    build_state(&meta);
    let (with_revision, _) = meta.get_audio_source(&id("src-1")).unwrap();
    assert!(matches!(
        meta.remove_unreferenced_imported_audio_source(
            &with_revision.project_id,
            &with_revision.header.id,
            &with_revision.content_digest
        ),
        Err(MetaError::Conflict(_))
    ));
    let (capture_recording, _) = meta.get_audio_source(&id("src-2")).unwrap();
    assert!(matches!(
        meta.remove_unreferenced_imported_audio_source(
            &capture_recording.project_id,
            &capture_recording.header.id,
            &capture_recording.content_digest
        ),
        Err(MetaError::Conflict(_))
    ));

    let audio = bytes(80, 64);
    let with_receipt = imported("source-denied-receipt", &audio);
    meta.insert_imported_audio_source(&with_receipt, &audio)
        .unwrap();
    meta.commit_transcript(None, &denied_receipt("r3", &with_receipt))
        .unwrap();
    assert!(matches!(
        meta.remove_unreferenced_imported_audio_source(
            &with_receipt.project_id,
            &with_receipt.header.id,
            &with_receipt.content_digest
        ),
        Err(MetaError::Conflict(_))
    ));

    let untouched_audio = bytes(96, 5);
    let untouched = imported("source-stale", &untouched_audio);
    meta.insert_imported_audio_source(&untouched, &untouched_audio)
        .unwrap();
    assert!(matches!(
        meta.remove_unreferenced_imported_audio_source(
            &untouched.project_id,
            &untouched.header.id,
            &DigestSha256::of(b"old-version")
        ),
        Err(MetaError::Conflict(_))
    ));
    assert_eq!(
        meta.get_audio_source(&untouched.header.id).unwrap().1,
        untouched_audio
    );
    meta.verify_audio_consistency().unwrap();
}


#[test]
fn unreferenced_audio_removal_refuses_cross_module_huddle_media() {
    let root = temp_root("remove-huddle-linked");
    let meta = open_meta(&root);
    populate_pre_081(&meta);
    let wav = bytes(144, 51);
    let source = imported("source-huddle-linked", &wav);
    meta.insert_imported_audio_source(&source, &wav).unwrap();

    // Inject only the indexed Huddle media reference to test the storage
    // layer's fail-closed behavior. No transcript or AudioFlow receipt exists.
    // A source without audio-specific references can still belong to Huddles.
    raw(&root)
        .execute(
            "INSERT INTO hud_media(media_id, huddle_id, source_id, state, body_json)
             VALUES (?1, ?2, ?3, 'present', '{}')",
            rusqlite::params!["media-huddle-linked", "huddle-linked", source.header.id.as_str()],
        )
        .unwrap();
    assert!(meta.list_transcript_revisions(&source.header.id).unwrap().is_empty());
    assert!(meta.list_transcript_receipts(&source.header.id).unwrap().is_empty());
    assert!(matches!(
        meta.remove_unreferenced_imported_audio_source(
            &source.project_id,
            &source.header.id,
            &source.content_digest,
        ),
        Err(MetaError::Conflict(_))
    ));
    assert_eq!(meta.get_audio_source(&source.header.id).unwrap().1, wav);

    // Remove the intentionally incomplete synthetic Huddle fixture to restore
    // a consistent synthetic vault; no production deletion code does this.
    raw(&root)
        .execute("DELETE FROM hud_media WHERE media_id = 'media-huddle-linked'", [])
        .unwrap();
    meta.verify_audio_consistency().unwrap();
    meta.remove_unreferenced_imported_audio_source(
        &source.project_id,
        &source.header.id,
        &source.content_digest,
    )
    .unwrap();
    assert!(matches!(
        meta.get_audio_source(&source.header.id),
        Err(MetaError::NotFound)
    ));
    meta.verify_audio_consistency().unwrap();
}

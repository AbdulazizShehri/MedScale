//! Spec 083 Knowledge + Research Canvas Core authority integration tests.
//!
//! Every call goes through `CoreFacade::dispatch`. Sources are synthetic:
//! CSV text imported as Spec 075 snapshots, a synthetic WAV transcribed by
//! the labelled fixture engine and then corrected (Spec 081), a scripted
//! socket-free web page (Spec 080), and a Spec 082 derived table.

use std::fs;
use std::path::PathBuf;

use medscale_contracts::analytics::{QueryRequest, ViewBinding};
use medscale_contracts::audio::{AudioRoute, AudioRouteRequest, PcmFormat, VoiceInputMode};
use medscale_contracts::browse::{BrowseIntentKind, BrowseRequest};
use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::knowledge::{
    CanvasOp, CanvasRelation, CanvasRevision, CanvasView, CoverageState, EvidenceOverlap,
    EvidenceOverlapKind, EvidenceSpanRef, Freshness, IndexManifest, IndexStatus,
    KnowledgeSourceKind, NoteCoverage, RetrievalDenyReason, RetrievalOutcome, RetrievalRequest,
    RetrievalResult, RetrievalStage,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::authority::FixtureAsrEngine;
use medscale_core::{CliSession, CoreFacade};
use medscale_network::ScriptedBrowseTransport;

const SCOPE: &str = "scope-a";
const NOTES: &str = "id,note\n1,myalgia after statin start\n2,no complaints\n";
const NOTES_V2: &str = "id,note\n1,fatigue only\n2,no complaints\n";
const PAGE: &[u8] = b"<html><body><p>Statin guidance: report myalgia early.</p></body></html>";
const URL: &str = "https://example.org/docs/statins";

struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    dir: PathBuf,
    next: u64,
}

fn req(scope: &str, n: u64, capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("req-{n}")),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new(scope),
        capability,
        body,
    )
}

struct Lab {
    project: OpaqueId,
    source: OpaqueId,
}

impl Harness {
    fn setup(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("medscale-083c-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self::open_at(dir)
    }

    fn open_at(dir: PathBuf) -> Self {
        let mut facade = CoreFacade::new();
        facade.set_asr_engine(Box::new(FixtureAsrEngine));
        facade.set_browse_transport(Box::new(
            ScriptedBrowseTransport::new()
                .route(URL, ScriptedBrowseTransport::ok("text/html", PAGE)),
        ));
        let mut h = Harness {
            facade,
            session: OpaqueId::new("pending"),
            dir,
            next: 0,
        };
        let ResponseBody::Lease { holder_id, .. } = h
            .raw(
                SCOPE,
                None,
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new("actor"),
                    holder_id_hint: Some(OpaqueId::new("actor")),
                },
            )
            .unwrap()
        else {
            panic!()
        };
        let ResponseBody::Session { session_id, .. } = h
            .raw(
                SCOPE,
                None,
                Capability::OpenSession,
                RequestBody::OpenSession {
                    holder_id,
                    granted: Capability::operator_grants(),
                    ttl_ticks: 1_000_000,
                },
            )
            .unwrap()
        else {
            panic!()
        };
        h.session = session_id;
        let vault_root = h.dir.display().to_string();
        h.call(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault { vault_root },
        )
        .unwrap();
        h
    }

    fn raw(
        &mut self,
        scope: &str,
        session: Option<OpaqueId>,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next += 1;
        let mut r = req(scope, self.next, capability, body);
        r.session_id = session;
        self.facade.dispatch(r).result
    }

    fn call(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        let s = Some(self.session.clone());
        self.raw(SCOPE, s, capability, body)
    }

    fn project(&mut self, name: &str) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: name.to_owned(),
                    description: None,
                },
            )
            .unwrap()
        {
            ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    /// A Project with one CSV source (file `file`) imported once.
    fn lab(&mut self, name: &str, file: &str, csv: &str) -> Lab {
        let project = self.project(name);
        fs::write(self.dir.join(file), csv).unwrap();
        let source = match self
            .call(
                Capability::DataSourceCreate,
                RequestBody::DataSourceCreate {
                    project_id: project.clone(),
                    display_name: name.to_owned(),
                    locator: SourceLocator::LocalPath {
                        path: file.to_owned(),
                        format: LocalFileFormat::Csv,
                    },
                    credential_ref: None,
                },
            )
            .unwrap()
        {
            ResponseBody::DataSource { source } => source.header.id,
            other => panic!("{other:?}"),
        };
        self.import(&source);
        Lab { project, source }
    }

    fn import(&mut self, source: &OpaqueId) -> OpaqueId {
        match self
            .call(
                Capability::SnapshotImport,
                RequestBody::SnapshotImport {
                    source_id: source.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::SnapshotImported { snapshot, .. } => snapshot.header.id,
            other => panic!("{other:?}"),
        }
    }

    /// Imports a synthetic WAV, transcribes it with the fixture engine and
    /// corrects segment 1 to `text`: returns (revision 1, revision 2).
    fn transcript(&mut self, project: &OpaqueId, text: &str) -> (OpaqueId, OpaqueId) {
        let wav = CliSession::synthetic_wav(
            PcmFormat::mono_16k(),
            &[(400, false), (800, true), (400, false)],
        );
        let source = match self
            .call(
                Capability::AudioImport,
                RequestBody::AudioImport {
                    project_id: project.clone(),
                    label: "visit".to_owned(),
                    wav,
                },
            )
            .unwrap()
        {
            ResponseBody::AudioSource { source } => source.header.id,
            other => panic!("{other:?}"),
        };
        let first = match self
            .call(
                Capability::AudioTranscribe,
                RequestBody::AudioTranscribe {
                    request: AudioRouteRequest {
                        project_id: project.clone(),
                        source_id: source,
                        route: AudioRoute::FixtureAsr,
                        mode: VoiceInputMode::Dictation,
                        language: Some("en".to_owned()),
                        diarization_required: false,
                    },
                },
            )
            .unwrap()
        {
            ResponseBody::AudioTranscript { view } => view.revision.unwrap().header.id,
            other => panic!("{other:?}"),
        };
        let second = self.correct(&first, text);
        (first, second)
    }

    fn correct(&mut self, revision: &OpaqueId, text: &str) -> OpaqueId {
        match self
            .call(
                Capability::AudioCorrect,
                RequestBody::AudioTranscriptCorrect {
                    revision_id: revision.clone(),
                    edits: vec![(1, text.to_owned())],
                    reason: "clinician review".to_owned(),
                },
            )
            .unwrap()
        {
            ResponseBody::AudioTranscriptRevision { revision } => revision.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn fetch(&mut self, project: &OpaqueId) {
        self.call(
            Capability::BrowseAllowlistManage,
            RequestBody::BrowseAllowlistAdd {
                project_id: project.clone(),
                host: "example.org".to_owned(),
                path_prefix: "/docs/".to_owned(),
            },
        )
        .unwrap();
        match self
            .call(
                Capability::BrowseRun,
                RequestBody::BrowseRun {
                    request: BrowseRequest {
                        project_id: project.clone(),
                        intent: BrowseIntentKind::FetchUrl,
                        url: Some(URL.to_owned()),
                        query: None,
                        context_artifact_id: None,
                    },
                },
            )
            .unwrap()
        {
            ResponseBody::BrowseSession { view } => assert_eq!(view.evidence.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    fn analytics(&mut self, lab: &Lab, snapshot: &OpaqueId) {
        match self
            .call(
                Capability::AnalyticsQuery,
                RequestBody::AnalyticsQuery {
                    request: QueryRequest {
                        project_id: lab.project.clone(),
                        sql: "SELECT 'cohort flagged for myalgia review' AS finding FROM notes LIMIT 1"
                            .to_owned(),
                        bindings: vec![ViewBinding {
                            alias: "notes".to_owned(),
                            snapshot_id: snapshot.clone(),
                        }],
                        max_rows: None,
                    },
                },
            )
            .unwrap()
        {
            ResponseBody::AnalyticsQuery { view } => assert!(view.table.is_some()),
            other => panic!("{other:?}"),
        }
    }

    fn build(&mut self, project: &OpaqueId) -> (IndexManifest, IndexStatus) {
        match self
            .call(
                Capability::KnowledgeIndex,
                RequestBody::KnowledgeIndexBuild {
                    project_id: project.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::KnowledgeIndexBuilt { manifest, status } => (*manifest, status),
            other => panic!("{other:?}"),
        }
    }

    fn status(&mut self, project: &OpaqueId) -> Option<IndexStatus> {
        match self
            .call(
                Capability::KnowledgeRead,
                RequestBody::KnowledgeIndexStatus {
                    project_id: project.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::KnowledgeIndexStatus { status } => status,
            other => panic!("{other:?}"),
        }
    }

    fn search_with(
        &mut self,
        project: &OpaqueId,
        query: &str,
        max_hits: Option<u32>,
        include_stale: bool,
    ) -> RetrievalResult {
        match self
            .call(
                Capability::KnowledgeSearch,
                RequestBody::KnowledgeSearch {
                    request: RetrievalRequest {
                        project_id: project.clone(),
                        query: query.to_owned(),
                        max_hits,
                        include_stale,
                    },
                },
            )
            .unwrap()
        {
            ResponseBody::KnowledgeSearch { result } => *result,
            other => panic!("{other:?}"),
        }
    }

    fn search(&mut self, project: &OpaqueId, query: &str) -> RetrievalResult {
        self.search_with(project, query, None, false)
    }

    fn canvas(&mut self, project: &OpaqueId, title: &str) -> CanvasRevision {
        match self
            .call(
                Capability::KnowledgeCanvas,
                RequestBody::CanvasCreate {
                    project_id: project.clone(),
                    title: title.to_owned(),
                },
            )
            .unwrap()
        {
            ResponseBody::Canvas { canvas } => *canvas,
            other => panic!("{other:?}"),
        }
    }

    fn edit(
        &mut self,
        canvas: &OpaqueId,
        expected: u32,
        ops: Vec<CanvasOp>,
    ) -> Result<CanvasRevision, AuthorityError> {
        match self.call(
            Capability::KnowledgeCanvas,
            RequestBody::CanvasEdit {
                canvas_id: canvas.clone(),
                expected_revision: expected,
                ops,
            },
        )? {
            ResponseBody::Canvas { canvas } => Ok(*canvas),
            other => panic!("{other:?}"),
        }
    }

    fn view(&mut self, canvas: &OpaqueId, revision: Option<u32>) -> CanvasView {
        match self
            .call(
                Capability::KnowledgeRead,
                RequestBody::CanvasGet {
                    canvas_id: canvas.clone(),
                    revision,
                },
            )
            .unwrap()
        {
            ResponseBody::CanvasView { view } => *view,
            other => panic!("{other:?}"),
        }
    }

    fn archive(&mut self, source: &OpaqueId) {
        let revision = match self
            .call(
                Capability::DataSourceRead,
                RequestBody::DataSourceGet {
                    source_id: source.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::DataSource { source } => source.revision,
            other => panic!("{other:?}"),
        };
        self.call(
            Capability::DataSourceArchive,
            RequestBody::DataSourceArchive {
                source_id: source.clone(),
                expected_revision: revision,
            },
        )
        .unwrap();
    }
}

fn span_of(result: &RetrievalResult, kind: KnowledgeSourceKind) -> EvidenceSpanRef {
    result
        .hits
        .iter()
        .find(|h| h.hit.span.source.kind == kind)
        .unwrap_or_else(|| panic!("no {kind:?} hit"))
        .hit
        .span
        .clone()
}

#[test]
fn the_index_pins_exact_sources_and_search_returns_live_spans() {
    let mut h = Harness::setup("search");
    let lab = h.lab("study", "notes.csv", NOTES);
    let snapshot = h.import(&lab.source);
    let (old_rev, new_rev) =
        h.transcript(&lab.project, "patient describes myalgia since the statin");
    h.fetch(&lab.project);
    h.analytics(&lab, &snapshot);

    assert_eq!(h.status(&lab.project), None, "no index yet");
    let (manifest, status) = h.build(&lab.project);
    assert_eq!(manifest.version, 1);
    assert!(status.is_fresh(), "{status:?}");
    let kinds: Vec<_> = manifest.sources.iter().map(|s| s.kind).collect();
    for kind in KnowledgeSourceKind::ALL {
        assert!(kinds.contains(kind), "{kind:?} indexed");
    }
    // Only the latest snapshot and transcript revision are indexed.
    assert!(manifest.sources.iter().any(|s| s.object_id == snapshot));
    assert!(manifest.sources.iter().any(|s| s.object_id == new_rev));
    assert!(!manifest.sources.iter().any(|s| s.object_id == old_rev));

    let r = h.search(&lab.project, "Myalgia");
    assert_eq!(r.receipt.outcome, RetrievalOutcome::Results);
    let plan = r.receipt.plan.clone().unwrap();
    assert_eq!(plan.stages, vec![RetrievalStage::Lexical]);
    assert_eq!(plan.terms, vec!["myalgia".to_owned()]);
    assert_eq!(r.receipt.manifest_id.as_ref(), Some(&manifest.header.id));
    assert_eq!(
        r.hits.len(),
        4,
        "snapshot, transcript, web page and derived table"
    );
    for hit in &r.hits {
        assert_eq!(hit.hit.freshness, Freshness::Current);
        let text = hit.text.as_deref().unwrap().to_lowercase();
        assert!(text.contains("myalgia"), "{text}");
    }
    // Ranks are deterministic: same request, same order and scores.
    let again = h.search(&lab.project, "Myalgia");
    assert_eq!(again.receipt.hits, r.receipt.hits);
    // The transcript hit names its exact segment and time span.
    let seg = span_of(&r, KnowledgeSourceKind::TranscriptRevision);
    assert_eq!(seg.source.object_id, new_rev);
    // The receipt is stored without text and can be read back.
    match h
        .call(
            Capability::KnowledgeRead,
            RequestBody::KnowledgeReceiptGet {
                receipt_id: r.receipt.header.id.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::KnowledgeReceipt { receipt } => assert_eq!(*receipt, r.receipt),
        other => panic!("{other:?}"),
    }

    // A rebuild of unchanged sources is identical.
    let (v2, _) = h.build(&lab.project);
    assert_eq!(v2.version, 2);
    assert_eq!(v2.chunks_digest, manifest.chunks_digest);
    assert_eq!(v2.sources, manifest.sources);

    // No match at all is insufficient evidence, never an empty success.
    let none = h.search(&lab.project, "rhabdomyolysis");
    assert_eq!(none.receipt.outcome, RetrievalOutcome::InsufficientEvidence);
    assert!(none.hits.is_empty());
}

#[test]
fn stale_and_archived_sources_are_reported_not_served() {
    let mut h = Harness::setup("stale");
    let lab = h.lab("study", "notes.csv", NOTES);
    h.build(&lab.project);
    // A new snapshot of the same source supersedes the indexed one.
    fs::write(h.dir.join("notes.csv"), NOTES_V2).unwrap();
    h.import(&lab.source);
    let status = h.status(&lab.project).unwrap();
    assert_eq!(
        (status.superseded_sources, status.unindexed_sources),
        (1, 1)
    );
    assert!(!status.is_fresh());
    // The only match is in the superseded snapshot: not served as current.
    let r = h.search(&lab.project, "myalgia");
    assert_eq!(r.receipt.outcome, RetrievalOutcome::InsufficientEvidence);
    assert!(r.hits.is_empty());
    assert!(!r.receipt.index_status.as_ref().unwrap().is_fresh());
    let stale = h.search_with(&lab.project, "myalgia", None, true);
    assert_eq!(
        stale.receipt.outcome,
        RetrievalOutcome::InsufficientEvidence
    );
    assert_eq!(stale.hits.len(), 1);
    assert_eq!(stale.hits[0].hit.freshness, Freshness::Superseded);
    assert_eq!(
        stale.hits[0].text.as_deref(),
        Some("myalgia after statin start")
    );

    // Archiving the source tombstones it: listed with include_stale, never
    // with text.
    h.archive(&lab.source);
    let status = h.status(&lab.project).unwrap();
    assert_eq!(status.tombstoned_sources, 1);
    assert_eq!(status.unindexed_sources, 0, "archived sources are not live");
    let gone = h.search_with(&lab.project, "myalgia", None, true);
    assert_eq!(gone.hits[0].hit.freshness, Freshness::Tombstoned);
    assert!(gone.hits[0].text.is_none());

    // A rebuild drops the archived source and is fresh again.
    let (v2, status) = h.build(&lab.project);
    assert_eq!(v2.version, 2);
    assert!(v2.sources.is_empty());
    assert!(status.is_fresh());
}

#[test]
fn refused_retrievals_leave_receipts() {
    let mut h = Harness::setup("deny");
    let lab = h.lab("study", "notes.csv", NOTES);
    let deny = |r: &RetrievalResult| r.receipt.deny_reason;
    assert_eq!(
        deny(&h.search(&lab.project, "myalgia")),
        Some(RetrievalDenyReason::NoIndex)
    );
    h.build(&lab.project);
    assert_eq!(
        deny(&h.search(&lab.project, " ;-- ")),
        Some(RetrievalDenyReason::EmptyQuery)
    );
    assert_eq!(
        deny(&h.search(&lab.project, &"x".repeat(600))),
        Some(RetrievalDenyReason::QueryTooLong)
    );
    let many: Vec<String> = (0..40).map(|i| format!("t{i}")).collect();
    assert_eq!(
        deny(&h.search(&lab.project, &many.join(" "))),
        Some(RetrievalDenyReason::TooManyTerms)
    );
    assert_eq!(
        deny(&h.search_with(&lab.project, "myalgia", Some(0), false)),
        Some(RetrievalDenyReason::BadHitLimit)
    );
    assert_eq!(
        deny(&h.search_with(&lab.project, "myalgia", Some(51), false)),
        Some(RetrievalDenyReason::BadHitLimit)
    );
    // The query text is data: SQL- and markup-shaped input matches words.
    let r = h.search(&lab.project, "' OR 1=1 -- <script>myalgia</script>");
    assert_eq!(r.receipt.outcome, RetrievalOutcome::Results);
    match h
        .call(
            Capability::KnowledgeRead,
            RequestBody::KnowledgeReceiptList {
                project_id: lab.project.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::KnowledgeReceipts { receipts } => {
            assert_eq!(receipts.len(), 7);
            assert_eq!(
                receipts
                    .iter()
                    .filter(|r| r.outcome == RetrievalOutcome::Denied)
                    .count(),
                6
            );
            assert!(
                receipts
                    .iter()
                    .all(|r| r.deny_reason.is_none() || r.hits.is_empty())
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn projects_and_scopes_never_leak() {
    let mut h = Harness::setup("leak");
    let a = h.lab("alpha", "a.csv", NOTES);
    let b = h.lab("beta", "b.csv", "id,note\n1,unrelated text\n");
    h.build(&a.project);
    let (bm, _) = h.build(&b.project);
    assert!(bm.sources.iter().all(|s| s.object_id.as_str() != "snap-1"));
    let rb = h.search(&b.project, "myalgia");
    assert_eq!(rb.receipt.outcome, RetrievalOutcome::InsufficientEvidence);
    let ra = h.search(&a.project, "myalgia");
    let a_span = span_of(&ra, KnowledgeSourceKind::DataSnapshot);
    // A Canvas of Project B cannot cite Project A's evidence.
    let c = h.canvas(&b.project, "beta canvas");
    assert!(matches!(
        h.edit(
            &c.header.id,
            1,
            vec![CanvasOp::AddEvidence {
                key: "e1".to_owned(),
                span: a_span,
            }]
        ),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    // Another scope reads nothing.
    let s = Some(h.session.clone());
    for body in [
        RequestBody::KnowledgeReceiptGet {
            receipt_id: ra.receipt.header.id.clone(),
        },
        RequestBody::CanvasGet {
            canvas_id: c.header.id.clone(),
            revision: None,
        },
        RequestBody::KnowledgeIndexStatus {
            project_id: a.project.clone(),
        },
    ] {
        assert!(
            h.raw("scope-b", s.clone(), Capability::KnowledgeRead, body)
                .is_err()
        );
    }
    assert!(
        h.raw(
            "scope-b",
            s,
            Capability::KnowledgeSearch,
            RequestBody::KnowledgeSearch {
                request: RetrievalRequest {
                    project_id: a.project.clone(),
                    query: "myalgia".to_owned(),
                    max_hits: None,
                    include_stale: false,
                },
            }
        )
        .is_err()
    );
}

#[test]
fn a_canvas_holds_live_references_and_inspects_its_evidence() {
    let mut h = Harness::setup("canvas");
    let lab = h.lab("study", "notes.csv", NOTES);
    let (_, rev2) = h.transcript(&lab.project, "patient describes myalgia since the statin");
    h.build(&lab.project);
    let r = h.search(&lab.project, "myalgia");
    let cell = span_of(&r, KnowledgeSourceKind::DataSnapshot);
    let seg = span_of(&r, KnowledgeSourceKind::TranscriptRevision);
    let c = h.canvas(&lab.project, "Statin myalgia");
    assert_eq!(c.revision, 1);
    let c2 = h
        .edit(
            &c.header.id,
            1,
            vec![
                CanvasOp::AddNote {
                    key: "claim".to_owned(),
                    text: "Statin start preceded myalgia".to_owned(),
                },
                CanvasOp::AddNote {
                    key: "question".to_owned(),
                    text: "Was CK measured?".to_owned(),
                },
                CanvasOp::AddEvidence {
                    key: "chart".to_owned(),
                    span: cell.clone(),
                },
                CanvasOp::AddEvidence {
                    key: "visit".to_owned(),
                    span: seg,
                },
                CanvasOp::Link {
                    from: "chart".to_owned(),
                    to: "claim".to_owned(),
                    relation: CanvasRelation::Supports,
                },
                CanvasOp::Link {
                    from: "visit".to_owned(),
                    to: "question".to_owned(),
                    relation: CanvasRelation::Contradicts,
                },
            ],
        )
        .unwrap();
    assert_eq!(c2.revision, 2);
    // The Canvas stores references, not copies of the evidence text.
    let stored = serde_json::to_string(&c2).unwrap();
    assert!(!stored.contains("myalgia after statin start"));

    let v = h.view(&c.header.id, None);
    assert_eq!(v.resolutions.len(), 2);
    assert!(
        v.resolutions
            .iter()
            .all(|n| n.freshness == Freshness::Current)
    );
    let chart = v.resolutions.iter().find(|n| n.key == "chart").unwrap();
    assert_eq!(chart.text.as_deref(), Some("myalgia after statin start"));
    assert_eq!(v.unsupported_notes, vec!["question".to_owned()]);
    assert_eq!(v.contradictions.len(), 1);
    assert!(v.superseded_evidence.is_empty() && v.missing_evidence.is_empty());
    // A cell and a transcript segment are different locations: no overlap.
    assert!(v.overlapping_evidence.is_empty());
    // Coverage: the chart cell supports the claim; the visit contradicts the
    // question. Each kind of source is its own cell.
    let cell = |note: &str, class: &str, state| NoteCoverage {
        note: note.to_owned(),
        evidence_class: class.to_owned(),
        state,
        source_count: 1,
    };
    assert_eq!(
        v.evidence_coverage,
        vec![
            cell("claim", "data_snapshot", CoverageState::Present),
            cell("question", "transcript", CoverageState::Conflicting),
        ]
    );
    assert!(v.coverage_hash.starts_with("sha256:") && v.coverage_hash.len() == 71);
    // The same revision always yields the same coverage hash.
    assert_eq!(h.view(&c.header.id, None).coverage_hash, v.coverage_hash);

    // Stale writers are refused; evidence must resolve.
    assert!(matches!(
        h.edit(
            &c.header.id,
            1,
            vec![CanvasOp::RemoveNode {
                key: "question".to_owned()
            }]
        ),
        Err(AuthorityError::Conflict { .. })
    ));
    let mut forged = cell.clone();
    forged.char_end += 500;
    assert!(matches!(
        h.edit(
            &c.header.id,
            2,
            vec![CanvasOp::AddEvidence {
                key: "forged".to_owned(),
                span: forged
            }]
        ),
        Err(AuthorityError::InvalidArgument { .. })
    ));

    // Superseding and archiving sources propagate to the live view.
    h.correct(&rev2, "patient now denies muscle pain");
    h.archive(&lab.source);
    let v = h.view(&c.header.id, None);
    assert_eq!(v.superseded_evidence, vec!["visit".to_owned()]);
    assert_eq!(v.missing_evidence, vec!["chart".to_owned()]);
    let chart = v.resolutions.iter().find(|n| n.key == "chart").unwrap();
    assert!(chart.text.is_none(), "archived evidence is not shown");
    let visit = v.resolutions.iter().find(|n| n.key == "visit").unwrap();
    assert!(
        visit.text.as_deref().unwrap().contains("myalgia"),
        "the pinned revision"
    );
    assert_eq!(
        v.unsupported_notes.len(),
        2,
        "archived support no longer counts"
    );
    // Earlier revisions stay readable exactly as written.
    assert!(h.view(&c.header.id, Some(1)).canvas.nodes.is_empty());
}

#[test]
fn knowledge_state_survives_reopen() {
    let mut h = Harness::setup("reopen");
    let lab = h.lab("study", "notes.csv", NOTES);
    let (manifest, _) = h.build(&lab.project);
    let r = h.search(&lab.project, "myalgia");
    let c = h.canvas(&lab.project, "Reopen");
    let dir = h.dir.clone();
    drop(h);
    let mut h = Harness::open_at(dir);
    assert!(h.status(&lab.project).unwrap().is_fresh());
    let again = h.search(&lab.project, "myalgia");
    assert_eq!(again.receipt.hits, r.receipt.hits);
    assert_eq!(again.receipt.manifest_id, Some(manifest.header.id));
    assert_eq!(h.view(&c.header.id, None).canvas, c);
    match h
        .call(
            Capability::KnowledgeRead,
            RequestBody::CanvasList {
                project_id: lab.project.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::Canvases { canvases } => assert_eq!(canvases.len(), 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_canvas_flags_evidence_that_cites_the_same_text_twice() {
    let mut h = Harness::setup("overlap");
    let lab = h.lab("study", "notes.csv", NOTES);
    h.build(&lab.project);
    let r = h.search(&lab.project, "myalgia");
    let cell = span_of(&r, KnowledgeSourceKind::DataSnapshot);
    // "myalgia after statin start" is 26 characters.
    assert_eq!((cell.char_start, cell.char_end), (0, 26));
    let part = |start: u32, end: u32| EvidenceSpanRef {
        char_start: start,
        char_end: end,
        ..cell.clone()
    };
    let c = h.canvas(&lab.project, "Duplicate evidence");
    let evidence = |key: &str, span: EvidenceSpanRef| CanvasOp::AddEvidence {
        key: key.to_owned(),
        span,
    };
    h.edit(
        &c.header.id,
        1,
        vec![
            evidence("whole", cell.clone()),
            evidence("whole-again", cell.clone()),
            evidence("head", part(0, 14)),
            evidence("tail", part(8, 26)),
        ],
    )
    .unwrap();
    let v = h.view(&c.header.id, None);
    let pair = |first: &str, second: &str, kind| EvidenceOverlap {
        first: first.to_owned(),
        second: second.to_owned(),
        kind,
    };
    use EvidenceOverlapKind::{Exact, Nested, Partial};
    assert_eq!(
        v.overlapping_evidence,
        vec![
            pair("whole", "whole-again", Exact),
            pair("head", "whole", Nested),
            pair("head", "whole-again", Nested),
            pair("tail", "whole", Nested),
            pair("tail", "whole-again", Nested),
            pair("head", "tail", Partial),
        ]
    );
    // A review signal only: nothing was removed or resolved.
    assert_eq!(v.resolutions.len(), 4);
}

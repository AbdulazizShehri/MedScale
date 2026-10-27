//! Spec 081 AudioFlow Foundation Core authority integration tests.
//!
//! Every call goes through `CoreFacade::dispatch`. All audio is synthetic
//! (tones and silence generated in code). The speech engine, when present,
//! is the labelled fixture wrapped in a counter, so each test can prove
//! whether an engine ran at all.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use medscale_contracts::audio::{
    AsrEngineIdentity, AudioLimitation, AudioRoute, AudioRouteDenyReason, AudioRouteOutcome,
    AudioRouteRequest, AudioSession, AudioSource, AudioSourceKind, CaptureBackendKind,
    CaptureHealth, CaptureState, PcmFormat, SegmentStatus, SpeakerLabel, TranscriptOrigin,
    TranscriptRevision, TranscriptState, TranscriptView, VoiceInputMode,
};
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId, VaultId};
use medscale_core::CliSession;
use medscale_core::CoreFacade;
use medscale_core::authority::{AsrEngine, FixtureAsrEngine, build_wav};

const SCOPE: &str = "scope-a";

/// Counts every call that reaches the engine.
struct Counting {
    calls: Arc<AtomicUsize>,
}

impl AsrEngine for Counting {
    fn identity(&self) -> AsrEngineIdentity {
        FixtureAsrEngine.identity()
    }

    fn transcribe(
        &self,
        format: PcmFormat,
        pcm: &[u8],
        segments: &[(u64, u64)],
        language: Option<&str>,
    ) -> Result<Vec<Option<String>>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        FixtureAsrEngine.transcribe(format, pcm, segments, language)
    }
}

struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    dir: PathBuf,
    calls: Arc<AtomicUsize>,
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

fn wav(parts: &[(u32, bool)]) -> Vec<u8> {
    CliSession::synthetic_wav(PcmFormat::mono_16k(), parts)
}

impl Harness {
    fn setup(name: &str, engine: bool) -> Self {
        let dir = std::env::temp_dir().join(format!("medscale-081c-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self::open_at(dir, engine)
    }

    fn open_at(dir: PathBuf, engine: bool) -> Self {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut facade = CoreFacade::new();
        if engine {
            facade.set_asr_engine(Box::new(Counting {
                calls: calls.clone(),
            }));
        }
        let mut h = Harness {
            facade,
            session: OpaqueId::new("pending"),
            dir,
            calls,
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

    fn project(&mut self) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: "audio".to_owned(),
                    description: None,
                },
            )
            .unwrap()
        {
            ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn import(
        &mut self,
        project: &OpaqueId,
        bytes: Vec<u8>,
    ) -> Result<AudioSource, AuthorityError> {
        match self.call(
            Capability::AudioImport,
            RequestBody::AudioImport {
                project_id: project.clone(),
                label: "synthetic".to_owned(),
                wav: bytes,
            },
        )? {
            ResponseBody::AudioSource { source } => Ok(*source),
            other => panic!("{other:?}"),
        }
    }

    fn start(
        &mut self,
        project: &OpaqueId,
        backend: CaptureBackendKind,
    ) -> Result<AudioSession, AuthorityError> {
        match self.call(
            Capability::AudioCapture,
            RequestBody::AudioCaptureStart {
                project_id: project.clone(),
                label: "synthetic capture".to_owned(),
                backend,
                format: PcmFormat::mono_16k(),
            },
        )? {
            ResponseBody::AudioCapture { session } => Ok(*session),
            other => panic!("{other:?}"),
        }
    }

    fn capture_op(&mut self, body: RequestBody) -> Result<AudioSession, AuthorityError> {
        let capability = match body {
            RequestBody::AudioCaptureGet { .. } => Capability::AudioRead,
            _ => Capability::AudioCapture,
        };
        match self.call(capability, body)? {
            ResponseBody::AudioCapture { session } => Ok(*session),
            other => panic!("{other:?}"),
        }
    }

    fn append(
        &mut self,
        s: &AudioSession,
        frames: Vec<u8>,
    ) -> Result<AudioSession, AuthorityError> {
        self.capture_op(RequestBody::AudioCaptureAppend {
            session_id: s.header.id.clone(),
            expected_revision: s.revision,
            frames,
        })
    }

    fn transition(
        &mut self,
        s: &AudioSession,
        to: CaptureState,
    ) -> Result<AudioSession, AuthorityError> {
        self.capture_op(RequestBody::AudioCaptureTransition {
            session_id: s.header.id.clone(),
            expected_revision: s.revision,
            to,
        })
    }

    fn stop(&mut self, s: &AudioSession) -> Result<(AudioSession, AudioSource), AuthorityError> {
        match self.call(
            Capability::AudioCapture,
            RequestBody::AudioCaptureStop {
                session_id: s.header.id.clone(),
                expected_revision: s.revision,
            },
        )? {
            ResponseBody::AudioCaptureStopped { session, source } => Ok((*session, *source)),
            other => panic!("{other:?}"),
        }
    }

    fn get_capture(&mut self, id: &OpaqueId) -> Result<AudioSession, AuthorityError> {
        self.capture_op(RequestBody::AudioCaptureGet {
            session_id: id.clone(),
        })
    }

    fn transcribe(
        &mut self,
        project: &OpaqueId,
        source: &AudioSource,
        route: AudioRoute,
        mode: VoiceInputMode,
        diarize: bool,
    ) -> Result<TranscriptView, AuthorityError> {
        match self.call(
            Capability::AudioTranscribe,
            RequestBody::AudioTranscribe {
                request: AudioRouteRequest {
                    project_id: project.clone(),
                    source_id: source.header.id.clone(),
                    route,
                    mode,
                    language: Some("en".to_owned()),
                    diarization_required: diarize,
                },
            },
        )? {
            ResponseBody::AudioTranscript { view } => Ok(*view),
            other => panic!("{other:?}"),
        }
    }

    fn correct(
        &mut self,
        revision: &OpaqueId,
        edits: Vec<(u32, String)>,
    ) -> Result<TranscriptRevision, AuthorityError> {
        match self.call(
            Capability::AudioCorrect,
            RequestBody::AudioTranscriptCorrect {
                revision_id: revision.clone(),
                edits,
                reason: "synthetic correction".to_owned(),
            },
        )? {
            ResponseBody::AudioTranscriptRevision { revision } => Ok(*revision),
            other => panic!("{other:?}"),
        }
    }

    fn revisions(&mut self, source: &OpaqueId) -> Vec<TranscriptRevision> {
        match self
            .call(
                Capability::AudioRead,
                RequestBody::AudioTranscriptList {
                    source_id: source.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::AudioTranscripts { revisions } => revisions,
            other => panic!("{other:?}"),
        }
    }

    fn engine_calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

fn pcm(parts: &[(u32, bool)]) -> Vec<u8> {
    CliSession::synthetic_pcm(PcmFormat::mono_16k(), parts)
}

#[test]
fn imports_are_validated_immutable_and_measured() {
    let mut h = Harness::setup("import", false);
    let p = h.project();
    let bytes = wav(&[(300, false), (800, true), (600, false), (400, true)]);
    let source = h.import(&p, bytes.clone()).unwrap();
    assert_eq!(source.kind, AudioSourceKind::ImportedFile);
    assert_eq!(source.duration_ms, 2_100);
    assert_eq!(source.content_digest, DigestSha256::of(&bytes));
    assert_eq!(source.health, CaptureHealth::Ok);
    let silent = h.import(&p, wav(&[(1_000, false)])).unwrap();
    assert_eq!(silent.health, CaptureHealth::Silent);

    let mut compressed = bytes.clone();
    compressed[20] = 2;
    for (name, bad) in [
        ("empty", Vec::new()),
        ("not riff", b"hello world, not audio".to_vec()),
        ("compressed", compressed),
        ("truncated", bytes[..bytes.len() - 3].to_vec()),
        ("no data", wav(&[])),
    ] {
        assert!(
            matches!(
                h.import(&p, bad),
                Err(AuthorityError::InvalidArgument { .. })
            ),
            "{name}"
        );
    }
    match h
        .call(
            Capability::AudioRead,
            RequestBody::AudioSourceList {
                project_id: p.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::AudioSources { sources } => assert_eq!(sources.len(), 2),
        other => panic!("{other:?}"),
    }
    // Another scope cannot read the source.
    let s = Some(h.session.clone());
    assert!(
        h.raw(
            "scope-b",
            s,
            Capability::AudioRead,
            RequestBody::AudioSourceGet {
                source_id: source.header.id.clone()
            }
        )
        .is_err()
    );
}

#[test]
fn capture_is_explicit_bounded_and_stops_into_exact_audio() {
    let mut h = Harness::setup("capture", false);
    let p = h.project();
    match h.start(&p, CaptureBackendKind::NativeDevice) {
        Err(AuthorityError::Unavailable { message }) => {
            assert!(message.contains("no admitted native capture backend"));
        }
        other => panic!("native capture must be unavailable, got {other:?}"),
    }
    let s = h.start(&p, CaptureBackendKind::Scripted).unwrap();
    assert_eq!(s.state, CaptureState::Recording);
    let first = pcm(&[(300, false), (500, true)]);
    let second = pcm(&[(400, true), (300, false)]);
    let s = h.append(&s, first.clone()).unwrap();
    assert!(
        matches!(
            h.append(&s, vec![1, 2, 3]),
            Err(AuthorityError::InvalidArgument { .. })
        ),
        "partial frame"
    );
    assert!(
        h.append(&s, vec![0; 2 * 1_048_576]).is_err(),
        "oversized chunk"
    );
    let paused = h.transition(&s, CaptureState::Paused).unwrap();
    assert!(
        matches!(
            h.append(&paused, second.clone()),
            Err(AuthorityError::Conflict { .. })
        ),
        "no frames while paused"
    );
    assert!(
        h.transition(&paused, CaptureState::Stopped).is_err(),
        "stop has its own request"
    );
    let resumed = h.transition(&paused, CaptureState::Recording).unwrap();
    let s = h.append(&resumed, second.clone()).unwrap();
    assert!(
        matches!(
            h.transition(&resumed, CaptureState::Paused),
            Err(AuthorityError::Conflict { .. })
        ),
        "stale revision"
    );
    let (stopped, source) = h.stop(&s).unwrap();
    assert_eq!(stopped.state, CaptureState::Stopped);
    assert_eq!(stopped.source_id.as_ref(), Some(&source.header.id));
    assert_eq!(source.kind, AudioSourceKind::Capture);
    assert_eq!(source.duration_ms, 1_500);
    let expected = build_wav(PcmFormat::mono_16k(), &[first, second].concat());
    assert_eq!(
        source.content_digest,
        DigestSha256::of(&expected),
        "the source is exactly the captured frames"
    );
    assert!(h.stop(&stopped).is_err(), "stopped is final");

    let s2 = h.start(&p, CaptureBackendKind::Scripted).unwrap();
    assert!(
        h.stop(&s2).is_err(),
        "an empty capture cannot stop into a source"
    );
    let s2 = h.append(&s2, pcm(&[(200, true)])).unwrap();
    let cancelled = h.transition(&s2, CaptureState::Cancelled).unwrap();
    assert_eq!(cancelled.state, CaptureState::Cancelled);
    assert!(cancelled.source_id.is_none());
    match h
        .call(
            Capability::AudioRead,
            RequestBody::AudioSourceList { project_id: p },
        )
        .unwrap()
    {
        ResponseBody::AudioSources { sources } => assert_eq!(sources.len(), 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_open_capture_from_an_ended_process_reopens_interrupted() {
    let mut h = Harness::setup("interrupt", false);
    let p = h.project();
    let s = h.start(&p, CaptureBackendKind::Scripted).unwrap();
    let s = h.append(&s, pcm(&[(500, true)])).unwrap();
    let paused = h.start(&p, CaptureBackendKind::Scripted).unwrap();
    let paused = h.transition(&paused, CaptureState::Paused).unwrap();
    // Still live in this process.
    assert_eq!(
        h.get_capture(&s.header.id).unwrap().state,
        CaptureState::Recording
    );
    let dir = h.dir.clone();
    drop(h);

    let mut h = Harness::open_at(dir, false);
    let recovered = h.get_capture(&s.header.id).unwrap();
    assert_eq!(recovered.state, CaptureState::Interrupted);
    assert!(recovered.source_id.is_none(), "no partial source");
    assert!(h.append(&recovered, pcm(&[(100, true)])).is_err());
    match h
        .call(
            Capability::AudioRead,
            RequestBody::AudioCaptureList {
                project_id: p.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::AudioCaptures { sessions } => {
            assert_eq!(sessions.len(), 2);
            assert!(
                sessions
                    .iter()
                    .all(|x| x.state == CaptureState::Interrupted)
            );
            assert_eq!(sessions[1].header.id, paused.header.id);
        }
        other => panic!("{other:?}"),
    }
    match h
        .call(
            Capability::AudioRead,
            RequestBody::AudioSourceList { project_id: p },
        )
        .unwrap()
    {
        ResponseBody::AudioSources { sources } => assert!(sources.is_empty()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn routes_are_local_only_and_every_request_leaves_a_receipt() {
    let mut h = Harness::setup("routes", false);
    let p = h.project();
    let source = h
        .import(
            &p,
            wav(&[(300, false), (800, true), (600, false), (400, true)]),
        )
        .unwrap();
    for (route, diarize, reason) in [
        (
            AudioRoute::CloudAsr,
            false,
            AudioRouteDenyReason::CloudRouteForbidden,
        ),
        (
            AudioRoute::LocalAsrPack,
            false,
            AudioRouteDenyReason::RouteUnavailable,
        ),
        (
            AudioRoute::FixtureAsr,
            false,
            AudioRouteDenyReason::RouteUnavailable,
        ),
    ] {
        let v = h
            .transcribe(&p, &source, route, VoiceInputMode::Dictation, diarize)
            .unwrap();
        assert_eq!(
            v.receipt.decision.outcome,
            AudioRouteOutcome::Deny,
            "{route:?}"
        );
        assert_eq!(v.receipt.decision.reason, Some(reason), "{route:?}");
        assert!(v.revision.is_none());
        assert!(v.receipt.engine.is_none());
    }
    let v = h
        .transcribe(
            &p,
            &source,
            AudioRoute::SegmentationOnly,
            VoiceInputMode::Command,
            false,
        )
        .unwrap();
    let t = v.revision.unwrap();
    assert_eq!(t.state, TranscriptState::Untranscribed);
    assert_eq!(
        t.segments
            .iter()
            .map(|s| (s.start_ms, s.end_ms))
            .collect::<Vec<_>>(),
        vec![(300, 1_100), (1_700, 2_100)]
    );
    assert!(
        t.segments
            .iter()
            .all(|s| s.speaker == SpeakerLabel::Unknown)
    );
    assert!(
        v.receipt
            .limitations
            .contains(&AudioLimitation::TranscriptNotClinicalTruth)
    );
    assert!(
        v.receipt
            .limitations
            .contains(&AudioLimitation::MedicalAccuracyUnmeasured)
    );

    // Spec 106: diarization on request is unvalidated acoustic clustering;
    // every segment gets an anonymous speaker index, never an identity.
    let d = h
        .transcribe(
            &p,
            &source,
            AudioRoute::SegmentationOnly,
            VoiceInputMode::Command,
            true,
        )
        .unwrap();
    let dt = d.revision.unwrap();
    assert_eq!(
        dt.diarization,
        medscale_contracts::audio::DiarizationState::AcousticClustering
    );
    assert!(
        dt.segments
            .iter()
            .all(|s| matches!(s.speaker, SpeakerLabel::Anonymous { index } if index >= 1))
    );
    assert!(
        d.receipt
            .limitations
            .contains(&AudioLimitation::DiarizationIsUnvalidatedClustering)
    );

    match h
        .call(Capability::AudioRead, RequestBody::AudioRouteList)
        .unwrap()
    {
        ResponseBody::AudioRoutes { routes } => {
            assert_eq!(
                routes
                    .iter()
                    .filter(|r| r.available)
                    .map(|r| r.route)
                    .collect::<Vec<_>>(),
                vec![AudioRoute::SegmentationOnly]
            );
        }
        other => panic!("{other:?}"),
    }
    match h
        .call(
            Capability::AudioRead,
            RequestBody::AudioReceiptList {
                source_id: source.header.id.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::AudioReceipts { receipts } => assert_eq!(receipts.len(), 5),
        other => panic!("{other:?}"),
    }
}

#[test]
fn fixture_transcripts_are_labelled_deterministic_and_command_text_is_inert() {
    let mut h = Harness::setup("fixture", true);
    let p = h.project();
    let source = h
        .import(
            &p,
            wav(&[(300, false), (800, true), (600, false), (400, true)]),
        )
        .unwrap();
    let denied = h
        .transcribe(
            &p,
            &source,
            AudioRoute::CloudAsr,
            VoiceInputMode::Dictation,
            false,
        )
        .unwrap();
    assert!(denied.revision.is_none());
    assert_eq!(
        h.engine_calls(),
        0,
        "a denied route never reaches an engine"
    );

    let a = h
        .transcribe(
            &p,
            &source,
            AudioRoute::FixtureAsr,
            VoiceInputMode::Command,
            false,
        )
        .unwrap();
    assert_eq!(h.engine_calls(), 1);
    let ta = a.revision.unwrap();
    assert_eq!(ta.state, TranscriptState::Complete);
    assert_eq!(ta.revision_no, 1);
    assert!(a.receipt.engine.as_ref().unwrap().is_fixture);
    assert!(
        a.receipt
            .limitations
            .contains(&AudioLimitation::FixtureEngineIsNotSpeechRecognition)
    );
    // COMMAND text is stored as text; nothing else happened.
    assert_eq!(ta.mode, VoiceInputMode::Command);
    let b = h
        .transcribe(
            &p,
            &source,
            AudioRoute::FixtureAsr,
            VoiceInputMode::Command,
            false,
        )
        .unwrap();
    let tb = b.revision.unwrap();
    assert_eq!(tb.revision_no, 2);
    assert_eq!(ta.segments, tb.segments, "deterministic");

    // Evidence resolves each segment to exact source audio.
    for seg in &tb.segments {
        match h
            .call(
                Capability::AudioRead,
                RequestBody::AudioEvidenceGet {
                    revision_id: tb.header.id.clone(),
                    segment_seq: seg.seq,
                },
            )
            .unwrap()
        {
            ResponseBody::AudioEvidence { evidence } => {
                assert_eq!(evidence.source_digest, source.content_digest);
                assert_eq!(
                    (evidence.start_ms, evidence.end_ms),
                    (seg.start_ms, seg.end_ms)
                );
                assert!(evidence.end_ms <= source.duration_ms);
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(
        h.call(
            Capability::AudioRead,
            RequestBody::AudioEvidenceGet {
                revision_id: tb.header.id.clone(),
                segment_seq: 99,
            },
        )
        .is_err()
    );
}

#[test]
fn corrections_create_new_revisions_and_never_touch_the_parent() {
    let mut h = Harness::setup("correct", true);
    let p = h.project();
    let source = h
        .import(
            &p,
            wav(&[(300, false), (800, true), (600, false), (400, true)]),
        )
        .unwrap();
    let t1 = h
        .transcribe(
            &p,
            &source,
            AudioRoute::FixtureAsr,
            VoiceInputMode::Dictation,
            false,
        )
        .unwrap()
        .revision
        .unwrap();
    for bad in [
        vec![],
        vec![(1, "  ".to_owned())],
        vec![(9, "x".to_owned())],
        vec![(1, "a".to_owned()), (1, "b".to_owned())],
    ] {
        assert!(matches!(
            h.correct(&t1.header.id, bad),
            Err(AuthorityError::InvalidArgument { .. })
        ));
    }
    let t2 = h
        .correct(&t1.header.id, vec![(2, "synthetic tone two".to_owned())])
        .unwrap();
    assert_eq!(t2.revision_no, 2);
    assert!(matches!(
        &t2.origin,
        TranscriptOrigin::HumanCorrection { parent_revision_id, .. } if *parent_revision_id == t1.header.id
    ));
    assert_eq!(t2.segments[1].status, SegmentStatus::Corrected);
    assert_eq!(t2.segments[0], t1.segments[0]);
    assert!(
        matches!(
            h.correct(&t1.header.id, vec![(1, "late".to_owned())]),
            Err(AuthorityError::Conflict { .. })
        ),
        "only the latest revision can be corrected"
    );
    let all = h.revisions(&source.header.id);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0], t1, "the parent is byte-identical");
}

#[test]
fn audio_state_survives_reopen() {
    let mut h = Harness::setup("reopen", true);
    let p = h.project();
    let source = h.import(&p, wav(&[(200, false), (600, true)])).unwrap();
    let s = h.start(&p, CaptureBackendKind::Scripted).unwrap();
    let s = h.append(&s, pcm(&[(400, true)])).unwrap();
    let (_, captured) = h.stop(&s).unwrap();
    let t = h
        .transcribe(
            &p,
            &source,
            AudioRoute::FixtureAsr,
            VoiceInputMode::Context,
            false,
        )
        .unwrap()
        .revision
        .unwrap();
    let dir = h.dir.clone();
    drop(h);
    let mut h = Harness::open_at(dir, false);
    assert_eq!(h.revisions(&source.header.id), vec![t]);
    match h
        .call(
            Capability::AudioRead,
            RequestBody::AudioSourceGet {
                source_id: captured.header.id.clone(),
            },
        )
        .unwrap()
    {
        ResponseBody::AudioSource { source } => assert_eq!(*source, captured),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        h.get_capture(&s.header.id).unwrap().state,
        CaptureState::Stopped
    );
}

#[test]
fn audio_remove_requires_distinct_write_capability_exact_confirmation_and_digest() {
    let mut h = Harness::setup("retention-authorized-remove", true);
    let project = h.project();
    let unrelated = h.project();
    let source = h
        .import(&project, wav(&[(300, false), (700, true)]))
        .unwrap();
    let other = h.import(&project, wav(&[(400, true)])).unwrap();
    let phrase = format!("REMOVE {} FROM ACTIVE VAULT", source.header.id.as_str());
    let request = || RequestBody::AudioRetentionRemove {
        project_id: project.clone(),
        source_id: source.header.id.clone(),
        expected_digest: source.content_digest.clone(),
        confirmation: phrase.clone(),
    };

    assert!(matches!(
        h.call(Capability::AudioRead, request()),
        Err(AuthorityError::Unauthorized)
    ));
    assert!(matches!(
        h.call(
            Capability::AudioRemove,
            RequestBody::AudioRetentionRemove {
                project_id: project.clone(),
                source_id: source.header.id.clone(),
                expected_digest: source.content_digest.clone(),
                confirmation: "REMOVE ANYTHING".to_owned()
            }
        ),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    assert!(matches!(
        h.call(
            Capability::AudioRemove,
            RequestBody::AudioRetentionRemove {
                project_id: project.clone(),
                source_id: source.header.id.clone(),
                expected_digest: DigestSha256::of(b"stale"),
                confirmation: phrase.clone()
            }
        ),
        Err(AuthorityError::Conflict { .. })
    ));
    assert!(matches!(
        h.call(
            Capability::AudioRemove,
            RequestBody::AudioRetentionRemove {
                project_id: unrelated,
                source_id: source.header.id.clone(),
                expected_digest: source.content_digest.clone(),
                confirmation: phrase.clone()
            }
        ),
        Err(AuthorityError::NotFound)
    ));
    let session = h.session.clone();
    assert!(matches!(
        h.raw(
            "different-scope",
            Some(session),
            Capability::AudioRemove,
            request()
        ),
        Err(AuthorityError::WrongScope)
    ));
    // Refused requests never mutate either existing source.
    assert_eq!(
        h.call(
            Capability::AudioRead,
            RequestBody::AudioSourceGet {
                source_id: source.header.id.clone(),
            }
        )
        .unwrap(),
        ResponseBody::AudioSource {
            source: Box::new(source.clone())
        }
    );

    match h.call(Capability::AudioRemove, request()).unwrap() {
        ResponseBody::AudioRetentionRemoved {
            project_id,
            source_id,
            receipt_id,
            removed_bytes,
            older_backups_may_retain_recording,
        } => {
            assert_eq!(project_id, project);
            assert_eq!(source_id, source.header.id);
            assert!(!receipt_id.as_str().is_empty());
            assert_eq!(removed_bytes, source.byte_length);
            assert!(older_backups_may_retain_recording);
        }
        other_response => panic!("{other_response:?}"),
    }
    assert!(matches!(
        h.call(
            Capability::AudioRead,
            RequestBody::AudioSourceGet {
                source_id: source.header.id.clone()
            },
        ),
        Err(AuthorityError::NotFound)
    ));
    assert_eq!(
        h.call(
            Capability::AudioRead,
            RequestBody::AudioSourceGet {
                source_id: other.header.id.clone()
            },
        )
        .unwrap(),
        ResponseBody::AudioSource {
            source: Box::new(other)
        }
    );
    assert!(matches!(
        h.call(Capability::AudioRemove, request()),
        Err(AuthorityError::NotFound)
    ));
}

#[test]
fn audio_remove_refuses_transcribed_recordings_without_losing_evidence() {
    let mut h = Harness::setup("retention-transcribed-refusal", true);
    let project = h.project();
    let source = h
        .import(&project, wav(&[(300, false), (700, true)]))
        .unwrap();
    let result = h
        .transcribe(
            &project,
            &source,
            AudioRoute::FixtureAsr,
            VoiceInputMode::Dictation,
            false,
        )
        .unwrap();
    let phrase = format!("REMOVE {} FROM ACTIVE VAULT", source.header.id.as_str());
    assert!(matches!(
        h.call(
            Capability::AudioRemove,
            RequestBody::AudioRetentionRemove {
                project_id: project,
                source_id: source.header.id.clone(),
                expected_digest: source.content_digest.clone(),
                confirmation: phrase,
            }
        ),
        Err(AuthorityError::Conflict { .. })
    ));
    assert_eq!(
        h.call(
            Capability::AudioRead,
            RequestBody::AudioSourceGet {
                source_id: source.header.id.clone()
            },
        )
        .unwrap(),
        ResponseBody::AudioSource {
            source: Box::new(source.clone())
        }
    );
    // The original transcript and route evidence cannot be orphaned.
    assert!(!h.revisions(&source.header.id).is_empty());
    let _ = result;
}

#[test]
fn audio_retention_preflight_is_read_only_scope_checked_and_link_aware() {
    let mut h = Harness::setup("retention-preflight", true);
    let p = h.project();
    let other = h.project();
    let source = h.import(&p, wav(&[(300, false), (700, true)])).unwrap();
    let query = || RequestBody::AudioRetentionPreflight {
        project_id: p.clone(),
        source_id: source.header.id.clone(),
    };
    assert!(matches!(
        h.call(Capability::AudioImport, query()),
        Err(AuthorityError::Unauthorized)
    ));
    let first = h.call(Capability::AudioRead, query()).unwrap();
    match first {
        ResponseBody::AudioRetentionPreflight {
            project_id,
            source_id,
            expected_digest,
            imported_file,
            has_transcript_revisions,
            has_audit_receipts,
            has_huddle_media_link,
            has_capture_reference,
            unreferenced_imported_file,
            delete_available,
            older_backups_may_retain_recording,
        } => {
            assert_eq!(project_id, p);
            assert_eq!(source_id, source.header.id);
            assert_eq!(expected_digest, source.content_digest);
            assert!(imported_file);
            assert!(!has_transcript_revisions);
            assert!(!has_audit_receipts);
            assert!(!has_huddle_media_link);
            assert!(!has_capture_reference);
            assert!(unreferenced_imported_file);
            assert!(delete_available);
            assert!(older_backups_may_retain_recording);
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        h.call(
            Capability::AudioRead,
            RequestBody::AudioRetentionPreflight {
                project_id: other,
                source_id: source.header.id.clone(),
            },
        ),
        Err(AuthorityError::NotFound)
    ));
    let session = h.session.clone();
    assert!(matches!(
        h.raw(
            "different-scope",
            Some(session),
            Capability::AudioRead,
            query(),
        ),
        Err(AuthorityError::WrongScope)
    ));
    // The read-only preflight cannot remove or modify the original.
    assert_eq!(
        h.call(
            Capability::AudioRead,
            RequestBody::AudioSourceGet {
                source_id: source.header.id.clone(),
            },
        )
        .unwrap(),
        ResponseBody::AudioSource {
            source: Box::new(source.clone())
        }
    );
    let _ = h
        .transcribe(
            &p,
            &source,
            AudioRoute::FixtureAsr,
            VoiceInputMode::Dictation,
            false,
        )
        .unwrap();
    match h.call(Capability::AudioRead, query()).unwrap() {
        ResponseBody::AudioRetentionPreflight {
            has_transcript_revisions,
            has_audit_receipts,
            unreferenced_imported_file,
            delete_available,
            ..
        } => {
            assert!(has_transcript_revisions);
            assert!(has_audit_receipts);
            assert!(!unreferenced_imported_file);
            assert!(!delete_available);
        }
        other => panic!("{other:?}"),
    }
}

//! Spec 106: local speech recognition from an admitted Audio Pack behind the
//! `LocalAsrPack` route. Synthetic tiny Whisper and synthetic audio only; the
//! random model recognizes nothing, so these tests prove governance and
//! wiring, not accuracy.

mod support;

use medscale_contracts::audio::{
    AudioRoute, AudioRouteDenyReason, AudioRouteOutcome, AudioRouteRequest, PcmFormat,
    VoiceInputMode,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_core::CliSession;

fn request(
    project: &medscale_contracts::objects::OpaqueId,
    source: &medscale_contracts::objects::OpaqueId,
) -> AudioRouteRequest {
    AudioRouteRequest {
        project_id: project.clone(),
        source_id: source.clone(),
        route: AudioRoute::LocalAsrPack,
        mode: VoiceInputMode::Dictation,
        language: Some("en".to_owned()),
        diarization_required: false,
    }
}

#[test]
fn local_asr_pack_route_runs_only_with_an_admitted_audio_pack() {
    let root = support::dir("asr-106");
    let pack_dir = root.join("pack");
    let pack_id = support::asr_pack(&pack_dir);
    let mut session = CliSession::connect("asr-106").unwrap();
    session
        .open_synthetic_vault(&root.join("vault").display().to_string())
        .unwrap();
    let project = session.project_create("audio".to_owned(), None).unwrap();
    let wav = CliSession::synthetic_wav(
        PcmFormat::mono_16k(),
        &[(300, false), (900, true), (300, false)],
    );
    let source = session
        .audio_import(project.header.id.clone(), "tone".to_owned(), wav)
        .unwrap();

    // Without an engine the route is listed unavailable and denied.
    let local = |s: &mut CliSession| {
        s.audio_routes()
            .unwrap()
            .into_iter()
            .find(|r| r.route == AudioRoute::LocalAsrPack)
            .unwrap()
    };
    assert!(!local(&mut session).available);
    let denied = session
        .audio_transcribe(request(&project.header.id, &source.header.id))
        .unwrap();
    assert_eq!(denied.receipt.decision.outcome, AudioRouteOutcome::Deny);
    assert_eq!(
        denied.receipt.decision.reason,
        Some(AudioRouteDenyReason::RouteUnavailable)
    );

    // A Pack that is not admitted in this vault is refused.
    let refused = session.use_local_asr_pack(&pack_dir.display().to_string());
    assert!(
        matches!(refused, Err(AuthorityError::Unauthorized)),
        "{refused:?}"
    );

    // Admitted: the engine installs, the route opens, and the transcript
    // names the non-fixture Pack engine.
    let admitted = session
        .packs_install_local(&pack_dir.display().to_string())
        .unwrap();
    assert!(admitted.admitted, "{admitted:?}");
    session
        .use_local_asr_pack(&pack_dir.display().to_string())
        .unwrap();
    assert!(local(&mut session).available);
    let view = session
        .audio_transcribe(request(&project.header.id, &source.header.id))
        .unwrap();
    assert_eq!(
        view.receipt.decision.outcome,
        AudioRouteOutcome::Allow,
        "{view:?}"
    );
    let revision = view.revision.expect("a transcript revision");
    let medscale_contracts::audio::TranscriptOrigin::Engine { route, engine, .. } =
        &revision.origin
    else {
        panic!("engine origin");
    };
    assert_eq!(*route, AudioRoute::LocalAsrPack);
    let engine = engine.as_ref().expect("engine identity");
    assert!(!engine.is_fixture);
    assert!(engine.engine_id.ends_with(&pack_id), "{engine:?}");
    assert!(!revision.segments.is_empty());
}

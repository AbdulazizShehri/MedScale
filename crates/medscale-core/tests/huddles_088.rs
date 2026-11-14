//! Spec 088 AudioFlow Advanced huddle integration tests.
//!
//! Every call goes through `CliSession` -> `CoreFacade::dispatch`. Audio is
//! synthetic tone/silence WAV (never speech) and recognition is the labeled
//! Spec 081 fixture engine, so these tests prove consent, labeling,
//! proposal, retention and deletion behavior, not speech recognition.

use medscale_contracts::audio::{AudioRoute, PcmFormat};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::huddles::{
    ConsentAct, HuddleActRequest, HuddleActResult, HuddleParticipantKind, HuddleRefusal,
    MediaState, ProposalKind, ProposalState,
};
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

struct Lab {
    s: CliSession,
    project: OpaqueId,
}

fn setup(name: &str) -> Lab {
    let dir = std::env::temp_dir().join(format!("medscale-088c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = CliSession::connect(&format!("vault-088-{name}")).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    s.use_fixture_asr();
    let project = s.project_create("ward".to_owned(), None).unwrap().header.id;
    Lab { s, project }
}

fn act(lab: &mut Lab, a: HuddleActRequest) -> HuddleActResult {
    lab.s.huddle_act(a).unwrap()
}

fn refusal(r: &HuddleActResult) -> Option<HuddleRefusal> {
    r.receipts.last().and_then(|x| x.refusal)
}

fn target(r: &HuddleActResult) -> OpaqueId {
    r.receipts.last().unwrap().targets[0].clone()
}

fn source(lab: &mut Lab, label: &str) -> OpaqueId {
    let wav = CliSession::synthetic_wav(
        PcmFormat::mono_16k(),
        &[(400, true), (400, false), (400, true)],
    );
    lab.s
        .audio_import(lab.project.clone(), label.to_owned(), wav)
        .unwrap()
        .header
        .id
}

fn huddle(lab: &mut Lab, retention_days: u32) -> OpaqueId {
    let project_id = lab.project.clone();
    act(
        lab,
        HuddleActRequest::Create {
            project_id,
            title: "Morning huddle".to_owned(),
            retention_days,
        },
    )
    .huddle
    .unwrap()
    .header
    .id
}

fn person(lab: &mut Lab, h: &OpaqueId, name: &str, kind: HuddleParticipantKind) -> OpaqueId {
    target(&act(
        lab,
        HuddleActRequest::AddParticipant {
            huddle_id: h.clone(),
            display_name: name.to_owned(),
            kind,
        },
    ))
}

fn consent(lab: &mut Lab, h: &OpaqueId, p: &OpaqueId, a: ConsentAct, v: bool) -> HuddleActResult {
    act(
        lab,
        HuddleActRequest::Consent {
            huddle_id: h.clone(),
            participant_id: p.clone(),
            consent_act: a,
            value: v,
        },
    )
}

#[test]
fn every_act_needs_every_humans_consent_for_that_act() {
    let mut lab = setup("consent");
    let h = huddle(&mut lab, 30);
    let ana = person(&mut lab, &h, "Ana", HuddleParticipantKind::Human);
    let ben = person(&mut lab, &h, "Ben", HuddleParticipantKind::Human);
    let src = source(&mut lab, "huddle-1");

    // Record consent needs joining first.
    assert_eq!(
        refusal(&consent(&mut lab, &h, &ana, ConsentAct::Record, true)),
        Some(HuddleRefusal::ConsentMissing)
    );
    for p in [&ana, &ben] {
        consent(&mut lab, &h, p, ConsentAct::Join, true);
    }
    consent(&mut lab, &h, &ana, ConsentAct::Record, true);
    let attach = |lab: &mut Lab| {
        act(
            lab,
            HuddleActRequest::Attach {
                huddle_id: h.clone(),
                source_id: src.clone(),
                agent: None,
                day: 20_000,
            },
        )
    };
    // Ben has not consented to recording.
    assert_eq!(
        refusal(&attach(&mut lab)),
        Some(HuddleRefusal::ConsentMissing)
    );
    consent(&mut lab, &h, &ben, ConsentAct::Record, true);
    let media = target(&attach(&mut lab));

    // Record consent is not transcribe consent.
    let transcribe = |lab: &mut Lab| {
        act(
            lab,
            HuddleActRequest::Transcribe {
                huddle_id: h.clone(),
                media_id: media.clone(),
                route: AudioRoute::FixtureAsr,
            },
        )
    };
    assert_eq!(
        refusal(&transcribe(&mut lab)),
        Some(HuddleRefusal::ConsentMissing)
    );
    for p in [&ana, &ben] {
        consent(&mut lab, &h, p, ConsentAct::Transcribe, true);
    }
    let transcribed = transcribe(&mut lab);
    assert_eq!(refusal(&transcribed), None);
    let revision = transcribed.receipts[0].targets[1].clone();

    // Transcribe consent is not export consent.
    let export = |lab: &mut Lab| {
        act(
            lab,
            HuddleActRequest::Export {
                huddle_id: h.clone(),
                media_id: media.clone(),
            },
        )
    };
    let denied = export(&mut lab);
    assert_eq!(refusal(&denied), Some(HuddleRefusal::ConsentMissing));
    assert!(denied.export.is_none());
    for p in [&ana, &ben] {
        consent(&mut lab, &h, p, ConsentAct::Export, true);
    }
    let exported = export(&mut lab);
    let e = exported.export.unwrap();
    assert!(!e.synthetic && e.engine_is_fixture && !e.lines.is_empty());
    assert_eq!(e.transcript_revision_id, revision);

    // Withdrawing is immediate: Ben leaves, so exporting stops.
    consent(&mut lab, &h, &ben, ConsentAct::Join, false);
    assert_eq!(
        refusal(&export(&mut lab)),
        Some(HuddleRefusal::ConsentMissing)
    );
    let view = lab.s.huddle_get(h.clone()).unwrap();
    let ben_now = view
        .participants
        .iter()
        .find(|p| p.header.id == ben)
        .unwrap();
    assert!(!ben_now.consents.record && !ben_now.consents.export);
}

#[test]
fn agents_never_consent_for_humans_and_their_audio_is_labeled() {
    let mut lab = setup("agents");
    let h = huddle(&mut lab, 30);
    let ana = person(&mut lab, &h, "Ana", HuddleParticipantKind::Human);
    let bot = person(&mut lab, &h, "Scribe agent", HuddleParticipantKind::Agent);
    consent(&mut lab, &h, &bot, ConsentAct::Join, true);
    for a in [
        ConsentAct::Record,
        ConsentAct::Transcribe,
        ConsentAct::Export,
    ] {
        assert_eq!(
            refusal(&consent(&mut lab, &h, &bot, a, true)),
            Some(HuddleRefusal::AgentCannotConsent)
        );
    }
    // An agent alone does not make a recording consented.
    let src = source(&mut lab, "agent-voice");
    let human = act(
        &mut lab,
        HuddleActRequest::Attach {
            huddle_id: h.clone(),
            source_id: src.clone(),
            agent: None,
            day: 20_000,
        },
    );
    assert_eq!(refusal(&human), Some(HuddleRefusal::ConsentMissing));
    // Attributed to the agent, it is attached and labeled synthetic.
    let synthetic = act(
        &mut lab,
        HuddleActRequest::Attach {
            huddle_id: h.clone(),
            source_id: src,
            agent: Some(bot.clone()),
            day: 20_000,
        },
    );
    assert_eq!(refusal(&synthetic), None);
    let view = lab.s.huddle_get(h.clone()).unwrap();
    assert!(view.media[0].synthetic);
    // A human cannot be passed off as an agent.
    let other = source(&mut lab, "other");
    let fake = act(
        &mut lab,
        HuddleActRequest::Attach {
            huddle_id: h,
            source_id: other,
            agent: Some(ana),
            day: 20_000,
        },
    );
    assert_eq!(refusal(&fake), Some(HuddleRefusal::NotAParticipant));
}

fn consented_media(lab: &mut Lab, retention_days: u32, day: u32) -> (OpaqueId, OpaqueId, OpaqueId) {
    let h = huddle(lab, retention_days);
    let ana = person(lab, &h, "Ana", HuddleParticipantKind::Human);
    for a in ConsentAct::ALL {
        consent(lab, &h, &ana, *a, true);
    }
    let src = source(lab, "rounds");
    let media = target(&act(
        lab,
        HuddleActRequest::Attach {
            huddle_id: h.clone(),
            source_id: src.clone(),
            agent: None,
            day,
        },
    ));
    (h, media, src)
}

#[test]
fn proposals_stay_proposals_until_a_human_reviews_them() {
    let mut lab = setup("proposals");
    let (h, media, _) = consented_media(&mut lab, 30, 20_000);
    let t = act(
        &mut lab,
        HuddleActRequest::Transcribe {
            huddle_id: h.clone(),
            media_id: media,
            route: AudioRoute::FixtureAsr,
        },
    );
    let revision = t.receipts[0].targets[1].clone();
    let bad = act(
        &mut lab,
        HuddleActRequest::Propose {
            huddle_id: h.clone(),
            transcript_revision_id: revision.clone(),
            segments: vec![999],
            kind: ProposalKind::Task,
            text: "Follow up".to_owned(),
        },
    );
    assert_eq!(refusal(&bad), Some(HuddleRefusal::SegmentNotInTranscript));
    let seq = lab
        .s
        .audio_transcript_get(revision.clone())
        .unwrap()
        .segments
        .iter()
        .find(|s| s.text.is_some())
        .unwrap()
        .seq;
    let proposed = act(
        &mut lab,
        HuddleActRequest::Propose {
            huddle_id: h.clone(),
            transcript_revision_id: revision,
            segments: vec![seq],
            kind: ProposalKind::Evidence,
            text: "Discussed repeat lipid panel".to_owned(),
        },
    );
    let proposal = target(&proposed);
    let view = lab.s.huddle_get(h.clone()).unwrap();
    assert_eq!(view.proposals[0].state, ProposalState::Proposed);
    act(
        &mut lab,
        HuddleActRequest::Review {
            huddle_id: h.clone(),
            proposal_id: proposal.clone(),
            accept: true,
        },
    );
    let view = lab.s.huddle_get(h.clone()).unwrap();
    assert_eq!(view.proposals[0].state, ProposalState::Accepted);
    assert!(view.proposals[0].reviewed_by.is_some());
    assert!(matches!(
        lab.s.huddle_act(HuddleActRequest::Review {
            huddle_id: h,
            proposal_id: proposal,
            accept: false,
        }),
        Err(AuthorityError::Conflict { .. })
    ));
}

#[test]
fn deletion_and_retention_remove_audio_and_transcripts_with_receipts() {
    let mut lab = setup("retention");
    let (h, media, src) = consented_media(&mut lab, 7, 20_000);
    act(
        &mut lab,
        HuddleActRequest::Transcribe {
            huddle_id: h.clone(),
            media_id: media.clone(),
            route: AudioRoute::FixtureAsr,
        },
    );
    assert!(!lab.s.audio_transcript_list(src.clone()).unwrap().is_empty());

    // Not yet expired.
    let early = act(&mut lab, HuddleActRequest::RetentionSweep { today: 20_006 });
    assert!(early.receipts.is_empty());
    let swept = act(&mut lab, HuddleActRequest::RetentionSweep { today: 20_007 });
    assert_eq!(swept.receipts.len(), 1);
    assert_eq!(swept.receipts[0].deleted_digests.len(), 1);
    assert!(
        lab.s.audio_source_get(src.clone()).is_err(),
        "audio bytes gone"
    );
    let view = lab.s.huddle_get(h.clone()).unwrap();
    assert_eq!(view.media[0].state, MediaState::Deleted);
    let after = act(
        &mut lab,
        HuddleActRequest::Export {
            huddle_id: h.clone(),
            media_id: media.clone(),
        },
    );
    assert_eq!(refusal(&after), Some(HuddleRefusal::MediaDeleted));

    // Explicit deletion, any time.
    let (h2, media2, src2) = consented_media(&mut lab, 3650, 20_000);
    let deleted = act(
        &mut lab,
        HuddleActRequest::DeleteMedia {
            huddle_id: h2.clone(),
            media_id: media2.clone(),
        },
    );
    assert_eq!(refusal(&deleted), None);
    assert!(lab.s.audio_source_get(src2).is_err());
    let again = act(
        &mut lab,
        HuddleActRequest::DeleteMedia {
            huddle_id: h2,
            media_id: media2,
        },
    );
    assert_eq!(refusal(&again), Some(HuddleRefusal::MediaDeleted));

    // Ending a huddle stops new media; its records remain readable.
    act(
        &mut lab,
        HuddleActRequest::End {
            huddle_id: h.clone(),
        },
    );
    let src3 = source(&mut lab, "late");
    let late = act(
        &mut lab,
        HuddleActRequest::Attach {
            huddle_id: h.clone(),
            source_id: src3,
            agent: None,
            day: 20_010,
        },
    );
    assert_eq!(refusal(&late), Some(HuddleRefusal::HuddleEnded));
    assert_eq!(lab.s.huddle_list(lab.project.clone()).unwrap().len(), 2);
}

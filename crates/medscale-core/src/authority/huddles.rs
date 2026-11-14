//! AudioFlow Advanced huddle authority (Spec 088, foundation slice).
//!
//! Built on the Spec 081 `Audio` authority. Every act on shared huddle
//! media is checked against the consent of every human participant at the
//! moment it happens (record to attach a human recording, transcribe to
//! transcribe, export to export). Agents are explicit participants, may
//! only join, and their audio is always labeled synthetic. Proposals are
//! never assertions or actions. Deletion removes the audio source bytes,
//! its transcript revisions and their receipts in one transaction and
//! records the removed digests. Every act, applied or refused, leaves a
//! receipt.

use medscale_contracts::audio::{AudioRoute, AudioRouteRequest, SpeakerLabel, VoiceInputMode};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::huddles::{
    ConsentAct, ConsentSet, HUDDLE_SCHEMA_VERSION, Huddle, HuddleActRequest, HuddleActResult,
    HuddleAction, HuddleExport, HuddleMedia, HuddleParticipant, HuddleParticipantKind,
    HuddleProposal, HuddleReceipt, HuddleRefusal, HuddleState, HuddleView, MediaOrigin, MediaState,
    PARTICIPANTS_MAX, ProposalKind, ProposalState,
};
use medscale_contracts::objects::{ObjectHeader, OpaqueId};
use medscale_storage::{HuddleChange, MetaError};

use super::audio::Audio;

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

fn invalid(message: String) -> AuthorityError {
    AuthorityError::InvalidArgument { message }
}

impl Audio<'_> {
    fn hud_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: HUDDLE_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn hud_alloc(&self, prefix: &str) -> Result<OpaqueId, AuthorityError> {
        self.meta.alloc_huddle_id(prefix).map_err(meta_err)
    }

    fn scoped_huddle(&self, id: &OpaqueId) -> Result<Huddle, AuthorityError> {
        let h = self.meta.get_huddle(id).map_err(meta_err)?;
        self.in_scope(&h.header)?;
        Ok(h)
    }

    fn hud_receipt(
        &self,
        huddle: &Huddle,
        action: HuddleAction,
    ) -> Result<HuddleReceipt, AuthorityError> {
        Ok(HuddleReceipt {
            header: self.hud_header(self.hud_alloc("huddle-receipt")?),
            huddle_id: huddle.header.id.clone(),
            project_id: huddle.project_id.clone(),
            action,
            targets: Vec::new(),
            refusal: None,
            consent: None,
            deleted_digests: Vec::new(),
        })
    }

    fn hud_refuse(
        &mut self,
        mut receipt: HuddleReceipt,
        refusal: HuddleRefusal,
    ) -> Result<HuddleReceipt, AuthorityError> {
        receipt.refusal = Some(refusal);
        self.meta
            .insert_huddle_refusal(&receipt)
            .map_err(meta_err)?;
        self.audit("huddle.refused", vec![receipt.header.id.clone()])?;
        Ok(receipt)
    }

    fn hud_commit(&mut self, change: HuddleChange<'_>, audit: &str) -> Result<(), AuthorityError> {
        let receipt_id = change
            .receipt
            .map(|r| r.header.id.clone())
            .ok_or_else(|| invalid("a change carries its receipt".to_owned()))?;
        self.meta.commit_huddle_change(&change).map_err(meta_err)?;
        self.audit(audit, vec![receipt_id])
    }

    /// Whether every human participant has joined and consented to `act`
    /// (and there is at least one human).
    fn humans_consent(
        &self,
        huddle_id: &OpaqueId,
        act: ConsentAct,
    ) -> Result<bool, AuthorityError> {
        let people = self
            .meta
            .list_huddle_participants(Some(huddle_id))
            .map_err(meta_err)?;
        let humans: Vec<&HuddleParticipant> = people
            .iter()
            .filter(|p| p.kind == HuddleParticipantKind::Human)
            .collect();
        Ok(!humans.is_empty()
            && humans
                .iter()
                .all(|p| p.consents.join && p.consents.get(act)))
    }

    fn present_media(&self, huddle_id: &OpaqueId, media_id: &OpaqueId) -> Option<HuddleMedia> {
        self.meta
            .list_huddle_media(Some(huddle_id))
            .ok()?
            .into_iter()
            .find(|m| &m.header.id == media_id)
    }

    // ----- lifecycle -----

    pub fn huddle_create(
        &mut self,
        project_id: OpaqueId,
        title: String,
        retention_days: u32,
    ) -> Result<Huddle, AuthorityError> {
        self.scoped_project(&project_id)?;
        let huddle = Huddle {
            header: self.hud_header(self.hud_alloc("huddle")?),
            project_id,
            title,
            created_by: self.actor()?,
            state: HuddleState::Open,
            retention_days,
            revision: 1,
        };
        huddle.validate().map_err(invalid)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::Create)?;
        receipt.targets = vec![huddle.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                huddle: Some((&huddle, None)),
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.create",
        )?;
        Ok(huddle)
    }

    pub fn huddle_add_participant(
        &mut self,
        huddle_id: &OpaqueId,
        display_name: String,
        kind: HuddleParticipantKind,
    ) -> Result<HuddleReceipt, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::AddParticipant)?;
        if huddle.state == HuddleState::Ended {
            return self.hud_refuse(receipt, HuddleRefusal::HuddleEnded);
        }
        let count = self
            .meta
            .list_huddle_participants(Some(huddle_id))
            .map_err(meta_err)?
            .len();
        if count >= PARTICIPANTS_MAX {
            return self.hud_refuse(receipt, HuddleRefusal::TooManyParticipants);
        }
        let participant = HuddleParticipant {
            header: self.hud_header(self.hud_alloc("huddle-participant")?),
            huddle_id: huddle_id.clone(),
            display_name,
            kind,
            consents: ConsentSet::default(),
            revision: 1,
        };
        participant.validate().map_err(invalid)?;
        receipt.targets = vec![participant.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                participant: Some((&participant, None)),
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.add_participant",
        )?;
        Ok(receipt)
    }

    /// Grants or withdraws one act for one participant. Agents may only
    /// join; they never consent to recording, transcription or export.
    pub fn huddle_consent(
        &mut self,
        huddle_id: &OpaqueId,
        participant_id: &OpaqueId,
        act: ConsentAct,
        value: bool,
    ) -> Result<HuddleReceipt, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::Consent)?;
        receipt.consent = Some((act, value));
        let Some(old) = self
            .meta
            .list_huddle_participants(Some(huddle_id))
            .map_err(meta_err)?
            .into_iter()
            .find(|p| &p.header.id == participant_id)
        else {
            return self.hud_refuse(receipt, HuddleRefusal::NotAParticipant);
        };
        if old.kind == HuddleParticipantKind::Agent && act != ConsentAct::Join && value {
            return self.hud_refuse(receipt, HuddleRefusal::AgentCannotConsent);
        }
        let mut next = old.clone();
        next.consents.set(act, value);
        if act == ConsentAct::Join && !value {
            next.consents = ConsentSet::default();
        }
        if next.consents.validate().is_err() {
            return self.hud_refuse(receipt, HuddleRefusal::ConsentMissing);
        }
        next.revision = old.revision + 1;
        receipt.targets = vec![next.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                participant: Some((&next, Some(old.revision))),
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.consent",
        )?;
        Ok(receipt)
    }

    /// Attaches a Spec 081 source as huddle media. A human recording needs
    /// every human's record consent; agent audio is labeled synthetic.
    pub fn huddle_attach(
        &mut self,
        huddle_id: &OpaqueId,
        source_id: &OpaqueId,
        agent: Option<OpaqueId>,
        day: u32,
    ) -> Result<HuddleReceipt, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::AttachMedia)?;
        if huddle.state == HuddleState::Ended {
            return self.hud_refuse(receipt, HuddleRefusal::HuddleEnded);
        }
        let source = match self.meta.get_audio_source(source_id) {
            Ok((s, _)) if s.project_id == huddle.project_id && self.in_scope(&s.header).is_ok() => {
                s
            }
            _ => return self.hud_refuse(receipt, HuddleRefusal::SourceNotInProject),
        };
        let origin = match agent {
            Some(participant_id) => {
                let is_agent = self
                    .meta
                    .list_huddle_participants(Some(huddle_id))
                    .map_err(meta_err)?
                    .iter()
                    .any(|p| {
                        p.header.id == participant_id && p.kind == HuddleParticipantKind::Agent
                    });
                if !is_agent {
                    return self.hud_refuse(receipt, HuddleRefusal::NotAParticipant);
                }
                MediaOrigin::SyntheticAgent { participant_id }
            }
            None => {
                if !self.humans_consent(huddle_id, ConsentAct::Record)? {
                    return self.hud_refuse(receipt, HuddleRefusal::ConsentMissing);
                }
                MediaOrigin::HumanRecording
            }
        };
        let synthetic = matches!(origin, MediaOrigin::SyntheticAgent { .. });
        let media = HuddleMedia {
            header: self.hud_header(self.hud_alloc("huddle-media")?),
            huddle_id: huddle_id.clone(),
            source_id: source_id.clone(),
            source_digest: source.content_digest.clone(),
            origin,
            synthetic,
            attached_day: day,
            state: MediaState::Present,
        };
        receipt.targets = vec![media.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                new_media: Some(&media),
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.attach",
        )?;
        Ok(receipt)
    }

    /// Transcribes huddle media through the Spec 081 route, only with every
    /// human's transcribe consent.
    pub fn huddle_transcribe(
        &mut self,
        huddle_id: &OpaqueId,
        media_id: &OpaqueId,
        route: AudioRoute,
    ) -> Result<HuddleReceipt, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::Transcribe)?;
        let Some(media) = self.present_media(huddle_id, media_id) else {
            return self.hud_refuse(receipt, HuddleRefusal::NotAParticipant);
        };
        if media.state == MediaState::Deleted {
            return self.hud_refuse(receipt, HuddleRefusal::MediaDeleted);
        }
        if !self.humans_consent(huddle_id, ConsentAct::Transcribe)? {
            return self.hud_refuse(receipt, HuddleRefusal::ConsentMissing);
        }
        let view = self.transcribe(AudioRouteRequest {
            project_id: huddle.project_id.clone(),
            source_id: media.source_id.clone(),
            route,
            mode: VoiceInputMode::Dictation,
            language: None,
            diarization_required: false,
        })?;
        receipt.targets = vec![view.receipt.header.id.clone()];
        if let Some(revision) = &view.revision {
            receipt.targets.push(revision.header.id.clone());
        }
        self.hud_commit(
            HuddleChange {
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.transcribe",
        )?;
        Ok(receipt)
    }

    /// Proposes a task or evidence item citing transcript segments of the
    /// huddle's present media.
    pub fn huddle_propose(
        &mut self,
        huddle_id: &OpaqueId,
        transcript_revision_id: &OpaqueId,
        segments: Vec<u32>,
        kind: ProposalKind,
        text: String,
    ) -> Result<HuddleReceipt, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::Propose)?;
        let Ok(transcript) = self.transcript(transcript_revision_id) else {
            return self.hud_refuse(receipt, HuddleRefusal::NoTranscript);
        };
        let media = self
            .meta
            .list_huddle_media(Some(huddle_id))
            .map_err(meta_err)?;
        if !media
            .iter()
            .any(|m| m.source_id == transcript.source_id && m.state == MediaState::Present)
        {
            return self.hud_refuse(receipt, HuddleRefusal::NoTranscript);
        }
        let known: Vec<u32> = transcript
            .segments
            .iter()
            .filter(|s| s.text.is_some())
            .map(|s| s.seq)
            .collect();
        if segments.is_empty() || !segments.iter().all(|s| known.contains(s)) {
            return self.hud_refuse(receipt, HuddleRefusal::SegmentNotInTranscript);
        }
        let proposal = HuddleProposal {
            header: self.hud_header(self.hud_alloc("huddle-proposal")?),
            huddle_id: huddle_id.clone(),
            transcript_revision_id: transcript_revision_id.clone(),
            segments,
            kind,
            text,
            state: ProposalState::Proposed,
            reviewed_by: None,
        };
        proposal.validate().map_err(invalid)?;
        receipt.targets = vec![proposal.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                proposal: Some((&proposal, false)),
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.propose",
        )?;
        Ok(receipt)
    }

    /// Records a human decision on a proposal. Nothing is executed.
    pub fn huddle_review(
        &mut self,
        huddle_id: &OpaqueId,
        proposal_id: &OpaqueId,
        accept: bool,
    ) -> Result<HuddleReceipt, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::Review)?;
        let Some(mut proposal) = self
            .meta
            .list_huddle_proposals(Some(huddle_id))
            .map_err(meta_err)?
            .into_iter()
            .find(|p| &p.header.id == proposal_id)
        else {
            return Err(AuthorityError::NotFound);
        };
        if proposal.state != ProposalState::Proposed {
            return Err(AuthorityError::Conflict {
                message: "proposal was already reviewed".to_owned(),
            });
        }
        proposal.state = if accept {
            ProposalState::Accepted
        } else {
            ProposalState::Rejected
        };
        proposal.reviewed_by = Some(self.actor()?);
        receipt.targets = vec![proposal.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                proposal: Some((&proposal, true)),
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.review",
        )?;
        Ok(receipt)
    }

    /// Exports the latest transcript of one media item, only with every
    /// human's export consent. Synthetic audio stays labeled.
    pub fn huddle_export(
        &mut self,
        huddle_id: &OpaqueId,
        media_id: &OpaqueId,
    ) -> Result<(HuddleReceipt, Option<HuddleExport>), AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::Export)?;
        let Some(media) = self.present_media(huddle_id, media_id) else {
            return Ok((
                self.hud_refuse(receipt, HuddleRefusal::NotAParticipant)?,
                None,
            ));
        };
        if media.state == MediaState::Deleted {
            return Ok((self.hud_refuse(receipt, HuddleRefusal::MediaDeleted)?, None));
        }
        if !self.humans_consent(huddle_id, ConsentAct::Export)? {
            return Ok((
                self.hud_refuse(receipt, HuddleRefusal::ConsentMissing)?,
                None,
            ));
        }
        let Some(latest) = self.transcripts(&media.source_id)?.into_iter().last() else {
            return Ok((self.hud_refuse(receipt, HuddleRefusal::NoTranscript)?, None));
        };
        let engine_is_fixture = match &latest.origin {
            medscale_contracts::audio::TranscriptOrigin::Engine { engine, .. } => {
                engine.as_ref().is_none_or(|e| e.is_fixture)
            }
            medscale_contracts::audio::TranscriptOrigin::HumanCorrection { .. } => false,
        };
        let label = if media.synthetic { "[SYNTHETIC] " } else { "" };
        let lines = latest
            .segments
            .iter()
            .filter_map(|s| {
                let text = s.text.as_ref()?;
                let speaker = match s.speaker {
                    SpeakerLabel::Unknown => "unknown".to_owned(),
                    SpeakerLabel::Anonymous { index } => format!("speaker {index}"),
                };
                Some(format!(
                    "{label}[{}-{} ms] {speaker}: {text}",
                    s.start_ms, s.end_ms
                ))
            })
            .collect();
        let export = HuddleExport {
            huddle_id: huddle_id.clone(),
            transcript_revision_id: latest.header.id.clone(),
            source_digest: media.source_digest.clone(),
            synthetic: media.synthetic,
            engine_is_fixture,
            lines,
        };
        receipt.targets = vec![latest.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.export",
        )?;
        Ok((receipt, Some(export)))
    }

    /// Deletes huddle media: the audio bytes, transcripts and their
    /// receipts. Always allowed; never undone.
    pub fn huddle_delete_media(
        &mut self,
        huddle_id: &OpaqueId,
        media_id: &OpaqueId,
    ) -> Result<HuddleReceipt, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&huddle, HuddleAction::DeleteMedia)?;
        let Some(media) = self.present_media(huddle_id, media_id) else {
            return Err(AuthorityError::NotFound);
        };
        if media.state == MediaState::Deleted {
            return self.hud_refuse(receipt, HuddleRefusal::MediaDeleted);
        }
        receipt.targets = vec![media.header.id.clone(), media.source_id.clone()];
        receipt.deleted_digests = vec![media.source_digest.clone()];
        self.hud_commit(
            HuddleChange {
                delete_media: vec![&media],
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.delete_media",
        )?;
        Ok(receipt)
    }

    /// Deletes every present media item older than its huddle's retention
    /// (`today` is days since the Unix epoch, supplied by the host).
    pub fn huddle_retention_sweep(
        &mut self,
        today: u32,
    ) -> Result<Vec<HuddleReceipt>, AuthorityError> {
        let mut out = Vec::new();
        for huddle in self.meta.list_huddles().map_err(meta_err)? {
            if self.in_scope(&huddle.header).is_err() {
                continue;
            }
            let expired: Vec<HuddleMedia> = self
                .meta
                .list_huddle_media(Some(&huddle.header.id))
                .map_err(meta_err)?
                .into_iter()
                .filter(|m| {
                    m.state == MediaState::Present
                        && today.saturating_sub(m.attached_day) >= huddle.retention_days
                })
                .collect();
            if expired.is_empty() {
                continue;
            }
            let mut receipt = self.hud_receipt(&huddle, HuddleAction::RetentionSweep)?;
            receipt.targets = expired.iter().map(|m| m.header.id.clone()).collect();
            receipt.deleted_digests = expired.iter().map(|m| m.source_digest.clone()).collect();
            self.hud_commit(
                HuddleChange {
                    delete_media: expired.iter().collect(),
                    receipt: Some(&receipt),
                    ..HuddleChange::default()
                },
                "huddle.retention_sweep",
            )?;
            out.push(receipt);
        }
        Ok(out)
    }

    pub fn huddle_end(&mut self, huddle_id: &OpaqueId) -> Result<HuddleReceipt, AuthorityError> {
        let old = self.scoped_huddle(huddle_id)?;
        let mut receipt = self.hud_receipt(&old, HuddleAction::End)?;
        if old.state == HuddleState::Ended {
            return self.hud_refuse(receipt, HuddleRefusal::HuddleEnded);
        }
        let next = Huddle {
            state: HuddleState::Ended,
            revision: old.revision + 1,
            ..old.clone()
        };
        receipt.targets = vec![next.header.id.clone()];
        self.hud_commit(
            HuddleChange {
                huddle: Some((&next, Some(old.revision))),
                receipt: Some(&receipt),
                ..HuddleChange::default()
            },
            "huddle.end",
        )?;
        Ok(receipt)
    }

    /// Runs one act and reports what it produced.
    pub fn huddle_act(&mut self, act: HuddleActRequest) -> Result<HuddleActResult, AuthorityError> {
        let one = |receipt: HuddleReceipt| HuddleActResult {
            huddle: None,
            receipts: vec![receipt],
            export: None,
        };
        Ok(match act {
            HuddleActRequest::Create {
                project_id,
                title,
                retention_days,
            } => {
                let huddle = self.huddle_create(project_id, title, retention_days)?;
                let receipts = self
                    .meta
                    .list_huddle_receipts(Some(&huddle.header.id))
                    .map_err(meta_err)?;
                HuddleActResult {
                    huddle: Some(huddle),
                    receipts,
                    export: None,
                }
            }
            HuddleActRequest::AddParticipant {
                huddle_id,
                display_name,
                kind,
            } => one(self.huddle_add_participant(&huddle_id, display_name, kind)?),
            HuddleActRequest::Consent {
                huddle_id,
                participant_id,
                consent_act,
                value,
            } => one(self.huddle_consent(&huddle_id, &participant_id, consent_act, value)?),
            HuddleActRequest::Attach {
                huddle_id,
                source_id,
                agent,
                day,
            } => one(self.huddle_attach(&huddle_id, &source_id, agent, day)?),
            HuddleActRequest::Transcribe {
                huddle_id,
                media_id,
                route,
            } => one(self.huddle_transcribe(&huddle_id, &media_id, route)?),
            HuddleActRequest::Propose {
                huddle_id,
                transcript_revision_id,
                segments,
                kind,
                text,
            } => one(self.huddle_propose(
                &huddle_id,
                &transcript_revision_id,
                segments,
                kind,
                text,
            )?),
            HuddleActRequest::Review {
                huddle_id,
                proposal_id,
                accept,
            } => one(self.huddle_review(&huddle_id, &proposal_id, accept)?),
            HuddleActRequest::Export {
                huddle_id,
                media_id,
            } => {
                let (receipt, export) = self.huddle_export(&huddle_id, &media_id)?;
                HuddleActResult {
                    huddle: None,
                    receipts: vec![receipt],
                    export,
                }
            }
            HuddleActRequest::DeleteMedia {
                huddle_id,
                media_id,
            } => one(self.huddle_delete_media(&huddle_id, &media_id)?),
            HuddleActRequest::RetentionSweep { today } => HuddleActResult {
                huddle: None,
                receipts: self.huddle_retention_sweep(today)?,
                export: None,
            },
            HuddleActRequest::End { huddle_id } => one(self.huddle_end(&huddle_id)?),
        })
    }

    pub fn huddle_view(&self, huddle_id: &OpaqueId) -> Result<HuddleView, AuthorityError> {
        let huddle = self.scoped_huddle(huddle_id)?;
        let id = Some(huddle_id);
        Ok(HuddleView {
            participants: self.meta.list_huddle_participants(id).map_err(meta_err)?,
            media: self.meta.list_huddle_media(id).map_err(meta_err)?,
            proposals: self.meta.list_huddle_proposals(id).map_err(meta_err)?,
            receipts: self.meta.list_huddle_receipts(id).map_err(meta_err)?,
            huddle,
        })
    }

    pub fn huddle_list(&self, project_id: &OpaqueId) -> Result<Vec<Huddle>, AuthorityError> {
        self.scoped_project(project_id)?;
        Ok(self
            .meta
            .list_huddles()
            .map_err(meta_err)?
            .into_iter()
            .filter(|h| &h.project_id == project_id && self.in_scope(&h.header).is_ok())
            .collect())
    }
}

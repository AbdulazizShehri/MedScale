//! AudioFlow Advanced contracts (Spec 088, huddle foundation slice).
//!
//! A huddle is a Project-scoped team audio session built on the Spec 081
//! AudioFlow foundation. Consent is per participant and per act: joining,
//! recording, transcribing and exporting are separate, revocable grants,
//! and an act on shared media needs the consent of every human
//! participant at the moment it happens. Agent participants are explicit,
//! never consent for humans, and any audio attributed to them is labeled
//! synthetic. Transcript-derived task and evidence items are proposals
//! until a human reviews them. Retention is declared per huddle; deletion
//! removes the media and its transcripts and leaves a deletion receipt.
//!
//! ```text
//! audio source != transcript != proposal != reviewed item
//!   != export (consented, labeled) != external effect (none)
//! ```
//!
//! No speech synthesis, voice cloning, duplex agent voice or live capture
//! backend exists in this slice; recognition is the Spec 081 engine route
//! (the fixture engine in CI). Medical ASR quality is `UNMEASURED`.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};

/// Durable schema version for every huddle object (Spec 088 v1).
pub const HUDDLE_SCHEMA_VERSION: u32 = 1;
pub const HUDDLE_TITLE_MAX_CHARS: usize = 200;
pub const PARTICIPANT_NAME_MAX_CHARS: usize = 120;
pub const PARTICIPANTS_MAX: usize = 32;
pub const PROPOSAL_TEXT_MAX_CHARS: usize = 2_000;
pub const RETENTION_DAYS_MAX: u32 = 3_650;

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

fn printable(value: &str, max: usize, what: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().count() > max || value.chars().any(char::is_control)
    {
        return Err(format!("{what} must be 1-{max} printable characters"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuddleState {
    Open,
    Ended,
}

closed_vocabulary!(HuddleState, "huddle state", {
    Open => "open",
    Ended => "ended",
});

/// A team audio session in one Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Huddle {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub title: String,
    pub created_by: OpaqueId,
    pub state: HuddleState,
    /// Media older than this many days is deleted by a retention sweep.
    pub retention_days: u32,
    pub revision: u64,
}

impl Huddle {
    pub fn validate(&self) -> Result<(), String> {
        printable(&self.title, HUDDLE_TITLE_MAX_CHARS, "huddle title")?;
        if self.retention_days == 0 || self.retention_days > RETENTION_DAYS_MAX {
            return Err("retention must be 1-3650 days".to_owned());
        }
        if self.revision == 0 {
            return Err("huddle revision starts at 1".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuddleParticipantKind {
    Human,
    /// An explicitly added, scoped agent. Its audio is synthetic; it never
    /// consents on behalf of a human.
    Agent,
}

closed_vocabulary!(HuddleParticipantKind, "huddle participant kind", {
    Human => "human",
    Agent => "agent",
});

/// The four separately granted acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentAct {
    Join,
    Record,
    Transcribe,
    Export,
}

closed_vocabulary!(ConsentAct, "consent act", {
    Join => "join",
    Record => "record",
    Transcribe => "transcribe",
    Export => "export",
});

/// Current consents of one participant. Every change is a receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentSet {
    pub join: bool,
    pub record: bool,
    pub transcribe: bool,
    pub export: bool,
}

impl ConsentSet {
    #[must_use]
    pub const fn get(&self, act: ConsentAct) -> bool {
        match act {
            ConsentAct::Join => self.join,
            ConsentAct::Record => self.record,
            ConsentAct::Transcribe => self.transcribe,
            ConsentAct::Export => self.export,
        }
    }

    pub fn set(&mut self, act: ConsentAct, value: bool) {
        match act {
            ConsentAct::Join => self.join = value,
            ConsentAct::Record => self.record = value,
            ConsentAct::Transcribe => self.transcribe = value,
            ConsentAct::Export => self.export = value,
        }
    }

    /// Recording, transcribing and exporting presuppose having joined.
    pub fn validate(&self) -> Result<(), String> {
        if !self.join && (self.record || self.transcribe || self.export) {
            return Err("a participant who has not joined consents to nothing else".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuddleParticipant {
    pub header: ObjectHeader,
    pub huddle_id: OpaqueId,
    pub display_name: String,
    pub kind: HuddleParticipantKind,
    pub consents: ConsentSet,
    pub revision: u64,
}

impl HuddleParticipant {
    pub fn validate(&self) -> Result<(), String> {
        printable(
            &self.display_name,
            PARTICIPANT_NAME_MAX_CHARS,
            "participant name",
        )?;
        self.consents.validate()?;
        if self.revision == 0 {
            return Err("participant revision starts at 1".to_owned());
        }
        Ok(())
    }
}

/// Where a huddle recording came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum MediaOrigin {
    /// Recorded from the human participants.
    HumanRecording,
    /// Audio attributed to an agent participant; always labeled synthetic.
    SyntheticAgent { participant_id: OpaqueId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaState {
    Present,
    Deleted,
}

closed_vocabulary!(MediaState, "media state", {
    Present => "present",
    Deleted => "deleted",
});

/// A Spec 081 audio source attached to a huddle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuddleMedia {
    pub header: ObjectHeader,
    pub huddle_id: OpaqueId,
    pub source_id: OpaqueId,
    pub source_digest: DigestSha256,
    pub origin: MediaOrigin,
    /// True exactly for synthetic origins.
    pub synthetic: bool,
    /// Logical day (host-supplied, days since the Unix epoch) it was attached.
    pub attached_day: u32,
    pub state: MediaState,
}

impl HuddleMedia {
    pub fn validate(&self) -> Result<(), String> {
        let synthetic_origin = matches!(self.origin, MediaOrigin::SyntheticAgent { .. });
        if synthetic_origin != self.synthetic {
            return Err("synthetic audio must be labeled, and only synthetic audio".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
    Task,
    Evidence,
}

closed_vocabulary!(ProposalKind, "proposal kind", {
    Task => "task",
    Evidence => "evidence",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Proposed,
    Accepted,
    Rejected,
}

closed_vocabulary!(ProposalState, "proposal state", {
    Proposed => "proposed",
    Accepted => "accepted",
    Rejected => "rejected",
});

/// A task or evidence item proposed from transcript segments. It is never
/// an assertion or an action; review records a human decision only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuddleProposal {
    pub header: ObjectHeader,
    pub huddle_id: OpaqueId,
    pub transcript_revision_id: OpaqueId,
    /// Segment sequence numbers the proposal cites (sorted, unique).
    pub segments: Vec<u32>,
    pub kind: ProposalKind,
    pub text: String,
    pub state: ProposalState,
    pub reviewed_by: Option<OpaqueId>,
}

impl HuddleProposal {
    pub fn validate(&self) -> Result<(), String> {
        printable(&self.text, PROPOSAL_TEXT_MAX_CHARS, "proposal text")?;
        if self.segments.is_empty() || !self.segments.windows(2).all(|w| w[0] < w[1]) {
            return Err("a proposal cites sorted, unique segments".to_owned());
        }
        if (self.state == ProposalState::Proposed) != self.reviewed_by.is_none() {
            return Err("exactly reviewed proposals name their reviewer".to_owned());
        }
        Ok(())
    }
}

/// Why an act on a huddle was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuddleRefusal {
    HuddleEnded,
    /// A human participant has not consented to this act.
    ConsentMissing,
    /// An agent cannot consent for, or stand in for, a human.
    AgentCannotConsent,
    SourceNotInProject,
    MediaDeleted,
    NoTranscript,
    SegmentNotInTranscript,
    NotAParticipant,
    TooManyParticipants,
}

closed_vocabulary!(HuddleRefusal, "huddle refusal", {
    HuddleEnded => "huddle_ended",
    ConsentMissing => "consent_missing",
    AgentCannotConsent => "agent_cannot_consent",
    SourceNotInProject => "source_not_in_project",
    MediaDeleted => "media_deleted",
    NoTranscript => "no_transcript",
    SegmentNotInTranscript => "segment_not_in_transcript",
    NotAParticipant => "not_a_participant",
    TooManyParticipants => "too_many_participants",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuddleAction {
    Create,
    AddParticipant,
    Consent,
    AttachMedia,
    Transcribe,
    Propose,
    Review,
    Export,
    DeleteMedia,
    RetentionSweep,
    End,
}

closed_vocabulary!(HuddleAction, "huddle action", {
    Create => "create",
    AddParticipant => "add_participant",
    Consent => "consent",
    AttachMedia => "attach_media",
    Transcribe => "transcribe",
    Propose => "propose",
    Review => "review",
    Export => "export",
    DeleteMedia => "delete_media",
    RetentionSweep => "retention_sweep",
    End => "end",
});

/// Every huddle act, applied or refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuddleReceipt {
    pub header: ObjectHeader,
    pub huddle_id: OpaqueId,
    pub project_id: OpaqueId,
    pub action: HuddleAction,
    /// Objects the act created or changed.
    pub targets: Vec<OpaqueId>,
    pub refusal: Option<HuddleRefusal>,
    /// For a consent change: the act and the new value.
    pub consent: Option<(ConsentAct, bool)>,
    /// For deletion: digests of the audio removed.
    pub deleted_digests: Vec<DigestSha256>,
}

/// An export: transcript text with provenance and synthetic labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuddleExport {
    pub huddle_id: OpaqueId,
    pub transcript_revision_id: OpaqueId,
    pub source_digest: DigestSha256,
    pub synthetic: bool,
    pub engine_is_fixture: bool,
    pub lines: Vec<String>,
}

/// Everything recorded about one huddle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuddleView {
    pub huddle: Huddle,
    pub participants: Vec<HuddleParticipant>,
    pub media: Vec<HuddleMedia>,
    pub proposals: Vec<HuddleProposal>,
    pub receipts: Vec<HuddleReceipt>,
}

/// One act on huddles (one request type keeps the envelope small).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "act", deny_unknown_fields)]
pub enum HuddleActRequest {
    Create {
        project_id: OpaqueId,
        title: String,
        retention_days: u32,
    },
    AddParticipant {
        huddle_id: OpaqueId,
        display_name: String,
        kind: HuddleParticipantKind,
    },
    Consent {
        huddle_id: OpaqueId,
        participant_id: OpaqueId,
        consent_act: ConsentAct,
        value: bool,
    },
    /// `agent` names the agent participant for synthetic audio.
    Attach {
        huddle_id: OpaqueId,
        source_id: OpaqueId,
        agent: Option<OpaqueId>,
        day: u32,
    },
    Transcribe {
        huddle_id: OpaqueId,
        media_id: OpaqueId,
        route: crate::audio::AudioRoute,
    },
    Propose {
        huddle_id: OpaqueId,
        transcript_revision_id: OpaqueId,
        segments: Vec<u32>,
        kind: ProposalKind,
        text: String,
    },
    Review {
        huddle_id: OpaqueId,
        proposal_id: OpaqueId,
        accept: bool,
    },
    Export {
        huddle_id: OpaqueId,
        media_id: OpaqueId,
    },
    DeleteMedia {
        huddle_id: OpaqueId,
        media_id: OpaqueId,
    },
    /// `today` is days since the Unix epoch, supplied by the host.
    RetentionSweep {
        today: u32,
    },
    End {
        huddle_id: OpaqueId,
    },
}

/// What an act produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuddleActResult {
    pub huddle: Option<Huddle>,
    pub receipts: Vec<HuddleReceipt>,
    pub export: Option<HuddleExport>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_needs_joining_first() {
        let mut c = ConsentSet::default();
        c.validate().unwrap();
        c.set(ConsentAct::Record, true);
        assert!(c.validate().is_err());
        c.set(ConsentAct::Join, true);
        c.validate().unwrap();
        assert!(c.get(ConsentAct::Record) && !c.get(ConsentAct::Export));
    }

    #[test]
    fn synthetic_audio_is_always_labeled() {
        let media = |origin, synthetic| HuddleMedia {
            header: ObjectHeader {
                id: OpaqueId::new("huddle-media-1"),
                schema_version: HUDDLE_SCHEMA_VERSION,
                realm_id: crate::objects::RealmId::new("r"),
                authority_scope_id: crate::objects::AuthorityScopeId::new("s"),
            },
            huddle_id: OpaqueId::new("huddle-1"),
            source_id: OpaqueId::new("audio-source-1"),
            source_digest: DigestSha256::of(b"wav"),
            origin,
            synthetic,
            attached_day: 20_000,
            state: MediaState::Present,
        };
        let agent = MediaOrigin::SyntheticAgent {
            participant_id: OpaqueId::new("huddle-participant-2"),
        };
        assert!(media(agent.clone(), false).validate().is_err());
        media(agent, true).validate().unwrap();
        assert!(media(MediaOrigin::HumanRecording, true).validate().is_err());
        media(MediaOrigin::HumanRecording, false)
            .validate()
            .unwrap();
    }

    #[test]
    fn proposals_cite_segments_and_reviews_name_reviewers() {
        let mut p = HuddleProposal {
            header: ObjectHeader {
                id: OpaqueId::new("huddle-proposal-1"),
                schema_version: HUDDLE_SCHEMA_VERSION,
                realm_id: crate::objects::RealmId::new("r"),
                authority_scope_id: crate::objects::AuthorityScopeId::new("s"),
            },
            huddle_id: OpaqueId::new("huddle-1"),
            transcript_revision_id: OpaqueId::new("audio-transcript-1"),
            segments: vec![1, 2],
            kind: ProposalKind::Task,
            text: "Order a lipid panel".to_owned(),
            state: ProposalState::Proposed,
            reviewed_by: None,
        };
        p.validate().unwrap();
        p.segments = vec![2, 1];
        assert!(p.validate().is_err());
        p.segments = vec![1];
        p.state = ProposalState::Accepted;
        assert!(p.validate().is_err());
        p.reviewed_by = Some(OpaqueId::new("holder-1"));
        p.validate().unwrap();
    }

    #[test]
    fn vocabularies_are_closed() {
        assert!(ConsentAct::parse("clone_voice").is_err());
        assert!(serde_json::from_str::<MediaOrigin>(r#"{"kind":"tts"}"#).is_err());
        assert!(serde_json::from_str::<ProposalState>(r#""executed""#).is_err());
    }
}

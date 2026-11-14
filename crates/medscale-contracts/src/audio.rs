//! AudioFlow Foundation contracts (Spec 081).
//!
//! Local audio only. Source audio is immutable evidence; transcripts are
//! derived revisions that never overwrite earlier revisions; speaker labels
//! never imply identity; `COMMAND`-mode text is recorded, never executed.
//! There is no cloud or remote speech route and no hidden fallback.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};
use crate::project_graph::{ProjectRevision, check_revision, initial_revision};

/// Durable schema version for every AudioFlow object (Spec 081 v1).
pub const AUDIO_SCHEMA_VERSION: u32 = 1;

/// Largest accepted WAV file (imported or produced by capture).
pub const MAX_AUDIO_BYTES: usize = 33_554_432;
pub const MAX_AUDIO_DURATION_MS: u64 = 30 * 60 * 1_000;
pub const MIN_SAMPLE_RATE: u32 = 8_000;
pub const MAX_SAMPLE_RATE: u32 = 48_000;
/// Largest single capture append (one chunk of PCM frames).
pub const MAX_CAPTURE_CHUNK_BYTES: usize = 1_048_576;
/// Above the most segments 30 minutes can yield under `VadParameters::FROZEN`
/// (200 ms speech + 300 ms gap), so segmentation never truncates.
pub const MAX_SEGMENTS: usize = 4_000;
pub const SEGMENT_TEXT_MAX_CHARS: usize = 4_000;
pub const LABEL_MAX_CHARS: usize = 200;
pub const REASON_MAX_CHARS: usize = 500;
pub const LANGUAGE_MAX_CHARS: usize = 35;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSourceKind {
    ImportedFile,
    Capture,
}

closed_vocabulary!(AudioSourceKind, "audio source kind", {
    ImportedFile => "imported_file",
    Capture => "capture",
});

/// Where capture frames come from. `NativeDevice` has no admitted backend
/// in this build (decision register Q18); `Scripted` accepts synthetic
/// frames pushed through Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureBackendKind {
    NativeDevice,
    Scripted,
}

closed_vocabulary!(CaptureBackendKind, "capture backend", {
    NativeDevice => "native_device",
    Scripted => "scripted",
});

impl CaptureBackendKind {
    #[must_use]
    pub const fn available(self) -> bool {
        matches!(self, Self::Scripted)
    }

    #[must_use]
    pub const fn unavailable_reason(self) -> Option<&'static str> {
        match self {
            Self::NativeDevice => {
                Some("no admitted native capture backend (dependency qualification pending, Q18)")
            }
            Self::Scripted => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Recording,
    Paused,
    Stopped,
    Cancelled,
    /// The process that owned the session ended while it was recording or
    /// paused. Partial frames are discarded; no source is produced.
    Interrupted,
}

closed_vocabulary!(CaptureState, "capture state", {
    Recording => "recording",
    Paused => "paused",
    Stopped => "stopped",
    Cancelled => "cancelled",
    Interrupted => "interrupted",
});

impl CaptureState {
    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Recording | Self::Paused)
    }

    /// Legal user transitions. `Interrupted` is set only by recovery.
    #[must_use]
    pub const fn can_become(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Recording, Self::Paused)
                | (Self::Paused, Self::Recording)
                | (
                    Self::Recording | Self::Paused,
                    Self::Stopped | Self::Cancelled | Self::Interrupted
                )
        )
    }
}

/// Deterministic signal health over the captured or imported samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureHealth {
    Ok,
    /// No frame reached the voice-activity threshold.
    Silent,
    /// At least 0.1% of samples sit at full scale.
    Clipping,
    /// No samples at all.
    NoAudio,
}

closed_vocabulary!(CaptureHealth, "capture health", {
    Ok => "ok",
    Silent => "silent",
    Clipping => "clipping",
    NoAudio => "no_audio",
});

/// What spoken text is for. Only recorded; `Command` is never executed by
/// this spec (voice control needs its own promotion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceInputMode {
    Command,
    Context,
    Dictation,
}

closed_vocabulary!(VoiceInputMode, "voice input mode", {
    Command => "command",
    Context => "context",
    Dictation => "dictation",
});

/// Transcription routes. Only local routes can ever run; `CloudAsr` exists
/// to be denied explicitly (no hidden fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRoute {
    /// Deterministic voice-activity segmentation; segments stay untranscribed.
    SegmentationOnly,
    /// Synthetic fixture engine for hermetic tests and demos. Not speech
    /// recognition; available only when a host installs it.
    FixtureAsr,
    /// A local ASR engine from an admitted Audio Pack (Spec 106).
    LocalAsrPack,
    CloudAsr,
}

closed_vocabulary!(AudioRoute, "audio route", {
    SegmentationOnly => "segmentation_only",
    FixtureAsr => "fixture_asr",
    LocalAsrPack => "local_asr_pack",
    CloudAsr => "cloud_asr",
});

/// Whether a route can run in this process, and why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioRouteStatus {
    pub route: AudioRoute,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRouteDenyReason {
    RouteUnavailable,
    CloudRouteForbidden,
    DiarizationUnavailable,
    SourceUnreadable,
}

closed_vocabulary!(AudioRouteDenyReason, "audio route deny reason", {
    RouteUnavailable => "route_unavailable",
    CloudRouteForbidden => "cloud_route_forbidden",
    DiarizationUnavailable => "diarization_unavailable",
    SourceUnreadable => "source_unreadable",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRouteOutcome {
    Allow,
    Deny,
}

closed_vocabulary!(AudioRouteOutcome, "audio route outcome", {
    Allow => "allow",
    Deny => "deny",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioRouteDecision {
    pub outcome: AudioRouteOutcome,
    pub reason: Option<AudioRouteDenyReason>,
}

impl AudioRouteDecision {
    #[must_use]
    pub const fn allow() -> Self {
        Self {
            outcome: AudioRouteOutcome::Allow,
            reason: None,
        }
    }

    #[must_use]
    pub const fn deny(reason: AudioRouteDenyReason) -> Self {
        Self {
            outcome: AudioRouteOutcome::Deny,
            reason: Some(reason),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match (self.outcome, self.reason) {
            (AudioRouteOutcome::Allow, None) | (AudioRouteOutcome::Deny, Some(_)) => Ok(()),
            _ => Err("allow carries no reason; deny carries exactly one".to_owned()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentStatus {
    Untranscribed,
    Transcribed,
    Corrected,
}

closed_vocabulary!(SegmentStatus, "segment status", {
    Untranscribed => "untranscribed",
    Transcribed => "transcribed",
    Corrected => "corrected",
});

/// Speaker attribution. Diarization is unavailable in this build, so every
/// segment is `Unknown`; an anonymous index never names a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SpeakerLabel {
    Unknown,
    Anonymous { index: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationState {
    NotRequested,
    Unavailable,
    /// Spec 106: segments grouped by voice features (pitch, zero-crossing
    /// rate) with unsupervised clustering. Unvalidated; labels are not
    /// identity.
    AcousticClustering,
}

closed_vocabulary!(DiarizationState, "diarization state", {
    NotRequested => "not_requested",
    Unavailable => "unavailable",
    AcousticClustering => "acoustic_clustering",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptState {
    /// Every segment has text.
    Complete,
    /// Some segments have text.
    Partial,
    /// No segment has text (segmentation only).
    Untranscribed,
}

closed_vocabulary!(TranscriptState, "transcript state", {
    Complete => "complete",
    Partial => "partial",
    Untranscribed => "untranscribed",
});

/// Fixed limitations stated on every transcript receipt that applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioLimitation {
    /// Automatic text is derived evidence, never clinical truth.
    TranscriptNotClinicalTruth,
    SpeakerLabelsAreNotIdentity,
    DiarizationUnavailable,
    /// Spec 106: speaker groups come from unvalidated acoustic clustering.
    DiarizationIsUnvalidatedClustering,
    /// The fixture engine does not recognize speech.
    FixtureEngineIsNotSpeechRecognition,
    /// Medical ASR quality (medications, doses, numbers, negation) is
    /// unmeasured for this route.
    MedicalAccuracyUnmeasured,
}

/// PCM layout. Only 16-bit little-endian PCM is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PcmFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl PcmFormat {
    #[must_use]
    pub const fn mono_16k() -> Self {
        Self {
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&self.sample_rate) {
            return Err(format!(
                "sample rate must be {MIN_SAMPLE_RATE}-{MAX_SAMPLE_RATE} Hz"
            ));
        }
        if !(1..=2).contains(&self.channels) {
            return Err("only mono or stereo audio is supported".to_owned());
        }
        if self.bits_per_sample != 16 {
            return Err("only 16-bit PCM is supported".to_owned());
        }
        Ok(())
    }

    /// Bytes per interleaved frame (all channels).
    #[must_use]
    pub const fn frame_bytes(&self) -> u64 {
        self.channels as u64 * (self.bits_per_sample as u64 / 8)
    }

    /// Duration in milliseconds of `data_bytes` of PCM (rounded down).
    #[must_use]
    pub const fn duration_ms(&self, data_bytes: u64) -> u64 {
        let frames = data_bytes / self.frame_bytes();
        frames * 1_000 / self.sample_rate as u64
    }
}

/// Parameters of the deterministic energy voice-activity segmenter. Frozen
/// for this spec and recorded on every receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VadParameters {
    pub frame_ms: u32,
    /// Frame RMS threshold, in dBFS.
    pub threshold_dbfs: i32,
    /// Shorter speech runs are dropped.
    pub min_speech_ms: u32,
    /// Silence shorter than this inside speech does not end a segment.
    pub hangover_ms: u32,
}

impl VadParameters {
    pub const FROZEN: Self = Self {
        frame_ms: 20,
        threshold_dbfs: -40,
        min_speech_ms: 200,
        hangover_ms: 300,
    };
}

/// One immutable piece of source audio (a WAV file), stored encrypted in
/// the vault. Its bytes never change after insert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSource {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub kind: AudioSourceKind,
    pub label: String,
    pub format: PcmFormat,
    pub duration_ms: u64,
    /// Length of the stored WAV file.
    pub byte_length: u64,
    /// SHA-256 of the stored WAV file.
    pub content_digest: DigestSha256,
    pub health: CaptureHealth,
    /// The capture session that produced this source (`Capture` only).
    pub capture_session_id: Option<OpaqueId>,
}

impl AudioSource {
    pub fn validate(&self) -> Result<(), String> {
        self.format.validate()?;
        check_label(&self.label)?;
        if self.byte_length == 0 || self.byte_length > MAX_AUDIO_BYTES as u64 {
            return Err("audio source size out of bounds".to_owned());
        }
        if self.duration_ms > MAX_AUDIO_DURATION_MS {
            return Err("audio source too long".to_owned());
        }
        match (self.kind, &self.capture_session_id) {
            (AudioSourceKind::Capture, Some(_)) | (AudioSourceKind::ImportedFile, None) => Ok(()),
            _ => Err("only capture sources name a capture session".to_owned()),
        }
    }
}

fn check_label(label: &str) -> Result<(), String> {
    if label.trim().is_empty()
        || label.chars().count() > LABEL_MAX_CHARS
        || label.chars().any(char::is_control)
    {
        return Err("label must be 1-200 printable characters".to_owned());
    }
    Ok(())
}

/// An explicitly started capture. Its state is always readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSession {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub label: String,
    pub backend: CaptureBackendKind,
    pub format: PcmFormat,
    pub state: CaptureState,
    /// The holder who explicitly started the capture.
    pub started_by: OpaqueId,
    /// PCM bytes captured so far (or in the final source).
    pub captured_bytes: u64,
    pub chunk_count: u32,
    /// Set when the session stops.
    pub source_id: Option<OpaqueId>,
}

impl AudioSession {
    #[must_use]
    pub fn start(
        header: ObjectHeader,
        project_id: OpaqueId,
        label: String,
        backend: CaptureBackendKind,
        format: PcmFormat,
        started_by: OpaqueId,
    ) -> Self {
        Self {
            header,
            revision: initial_revision(),
            project_id,
            label,
            backend,
            format,
            state: CaptureState::Recording,
            started_by,
            captured_bytes: 0,
            chunk_count: 0,
            source_id: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.format.validate()?;
        check_label(&self.label)?;
        if !self.backend.available() {
            return Err("capture backend is not available".to_owned());
        }
        if self.captured_bytes > MAX_AUDIO_BYTES as u64 {
            return Err("capture exceeds the audio size bound".to_owned());
        }
        if !self
            .captured_bytes
            .is_multiple_of(self.format.frame_bytes())
        {
            return Err("captured bytes are not whole PCM frames".to_owned());
        }
        match (self.state, &self.source_id) {
            (CaptureState::Stopped, Some(_)) => Ok(()),
            (CaptureState::Stopped, None) => Err("a stopped capture names its source".to_owned()),
            (_, Some(_)) => Err("only a stopped capture has a source".to_owned()),
            (_, None) => Ok(()),
        }
    }

    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

/// One timestamped piece of a transcript revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptSegment {
    pub seq: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker: SpeakerLabel,
    pub status: SegmentStatus,
    pub text: Option<String>,
}

/// Engine identity recorded on revisions and receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsrEngineIdentity {
    pub engine_id: String,
    pub version: String,
    /// True for the synthetic fixture engine, which does not recognize speech.
    pub is_fixture: bool,
}

/// Where a revision came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum TranscriptOrigin {
    Engine {
        route: AudioRoute,
        engine: Option<AsrEngineIdentity>,
        vad: VadParameters,
    },
    HumanCorrection {
        parent_revision_id: OpaqueId,
        corrected_by: OpaqueId,
        reason: String,
    },
}

/// One immutable transcript revision of one source. Corrections create a
/// new revision; earlier revisions are never modified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptRevision {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub source_id: OpaqueId,
    pub source_digest: DigestSha256,
    /// 1-based, contiguous per source.
    pub revision_no: u32,
    pub origin: TranscriptOrigin,
    pub mode: VoiceInputMode,
    pub language: Option<String>,
    pub diarization: DiarizationState,
    pub state: TranscriptState,
    pub segments: Vec<TranscriptSegment>,
}

impl TranscriptRevision {
    /// The state implied by the segments.
    #[must_use]
    pub fn derived_state(segments: &[TranscriptSegment]) -> TranscriptState {
        let with_text = segments.iter().filter(|s| s.text.is_some()).count();
        if with_text == 0 {
            TranscriptState::Untranscribed
        } else if with_text == segments.len() {
            TranscriptState::Complete
        } else {
            TranscriptState::Partial
        }
    }

    /// Checks shape against the source's duration.
    pub fn validate(&self, source_duration_ms: u64) -> Result<(), String> {
        if self.revision_no == 0 {
            return Err("revision numbers start at 1".to_owned());
        }
        if self.segments.len() > MAX_SEGMENTS {
            return Err("too many segments".to_owned());
        }
        if let Some(language) = &self.language
            && (language.is_empty()
                || language.len() > LANGUAGE_MAX_CHARS
                || !language
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'))
        {
            return Err("language must be a short BCP 47 tag".to_owned());
        }
        match &self.origin {
            TranscriptOrigin::Engine { route, engine, .. } => {
                if *route == AudioRoute::CloudAsr {
                    return Err("no revision can come from an unavailable route".to_owned());
                }
                // Spec 106: a local ASR revision names its non-fixture Pack engine.
                if *route == AudioRoute::LocalAsrPack
                    && !engine.as_ref().is_some_and(|e| !e.is_fixture)
                {
                    return Err("local ASR revisions name their Audio Pack engine".to_owned());
                }
                if (*route == AudioRoute::FixtureAsr)
                    != engine.as_ref().is_some_and(|e| e.is_fixture)
                {
                    return Err("fixture revisions name the fixture engine".to_owned());
                }
            }
            TranscriptOrigin::HumanCorrection { reason, .. } => {
                if reason.trim().is_empty()
                    || reason.chars().count() > REASON_MAX_CHARS
                    || reason.chars().any(char::is_control)
                {
                    return Err("a correction states a reason".to_owned());
                }
                if self.revision_no < 2 {
                    return Err("a correction follows an earlier revision".to_owned());
                }
            }
        }
        let mut previous_end = 0_u64;
        for (i, s) in self.segments.iter().enumerate() {
            if s.seq as usize != i + 1 {
                return Err("segments are numbered 1..n in order".to_owned());
            }
            if s.start_ms >= s.end_ms || s.start_ms < previous_end || s.end_ms > source_duration_ms
            {
                return Err(
                    "segments are ordered, non-overlapping and inside the source".to_owned(),
                );
            }
            previous_end = s.end_ms;
            // No diarization engine exists in this build.
            if s.speaker != SpeakerLabel::Unknown
                && self.diarization != DiarizationState::AcousticClustering
            {
                return Err("speaker labels need diarization".to_owned());
            }
            match (&s.text, s.status) {
                (None, SegmentStatus::Untranscribed) => {}
                (Some(text), SegmentStatus::Transcribed | SegmentStatus::Corrected) => {
                    if text.chars().count() > SEGMENT_TEXT_MAX_CHARS
                        || text.chars().any(|c| c.is_control() && c != '\n')
                    {
                        return Err("segment text out of bounds".to_owned());
                    }
                }
                _ => return Err("segment status disagrees with its text".to_owned()),
            }
            if s.status == SegmentStatus::Corrected
                && !matches!(self.origin, TranscriptOrigin::HumanCorrection { .. })
            {
                return Err("only a human correction marks segments corrected".to_owned());
            }
        }
        if self.state != Self::derived_state(&self.segments) {
            return Err("transcript state disagrees with its segments".to_owned());
        }
        Ok(())
    }

    /// Evidence reference for one segment.
    #[must_use]
    pub fn evidence_ref(&self, seq: u32) -> Option<AudioEvidenceRef> {
        self.segments
            .iter()
            .find(|s| s.seq == seq)
            .map(|s| AudioEvidenceRef {
                source_id: self.source_id.clone(),
                source_digest: self.source_digest.clone(),
                start_ms: s.start_ms,
                end_ms: s.end_ms,
                transcript_revision_id: self.header.id.clone(),
                segment_seq: s.seq,
            })
    }
}

/// Points at exact source audio: digest plus time range, through the
/// transcript revision and segment that cite it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioEvidenceRef {
    pub source_id: OpaqueId,
    pub source_digest: DigestSha256,
    pub start_ms: u64,
    pub end_ms: u64,
    pub transcript_revision_id: OpaqueId,
    pub segment_seq: u32,
}

/// A transcription request. `diarization_required` is refused in this
/// build rather than silently ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioRouteRequest {
    pub project_id: OpaqueId,
    pub source_id: OpaqueId,
    pub route: AudioRoute,
    pub mode: VoiceInputMode,
    pub language: Option<String>,
    pub diarization_required: bool,
}

/// One receipt per transcription request, allowed or denied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptReceipt {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub source_id: OpaqueId,
    pub source_digest: DigestSha256,
    pub request_digest: DigestSha256,
    pub route: AudioRoute,
    pub decision: AudioRouteDecision,
    /// Set exactly when the decision is `allow`.
    pub transcript_revision_id: Option<OpaqueId>,
    pub engine: Option<AsrEngineIdentity>,
    pub vad: VadParameters,
    pub segment_count: u32,
    pub limitations: Vec<AudioLimitation>,
}

impl TranscriptReceipt {
    pub fn validate(&self) -> Result<(), String> {
        self.decision.validate()?;
        let allowed = self.decision.outcome == AudioRouteOutcome::Allow;
        if allowed != self.transcript_revision_id.is_some() {
            return Err("only an allowed request names a transcript revision".to_owned());
        }
        if !allowed && self.segment_count != 0 {
            return Err("a denied request has no segments".to_owned());
        }
        if !self
            .limitations
            .contains(&AudioLimitation::TranscriptNotClinicalTruth)
        {
            return Err("every receipt states that transcripts are not clinical truth".to_owned());
        }
        if self.engine.as_ref().is_some_and(|e| e.is_fixture)
            && !self
                .limitations
                .contains(&AudioLimitation::FixtureEngineIsNotSpeechRecognition)
        {
            return Err("a fixture receipt says the fixture is not speech recognition".to_owned());
        }
        Ok(())
    }
}

/// Digest of a request's canonical JSON form.
#[must_use]
pub fn audio_request_digest(request: &AudioRouteRequest) -> DigestSha256 {
    let bytes = serde_json::to_vec(request).unwrap_or_default();
    DigestSha256::of(&bytes)
}

/// Everything a transcript view shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptView {
    pub revision: Option<TranscriptRevision>,
    pub receipt: TranscriptReceipt,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{AuthorityScopeId, RealmId};

    fn h(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: AUDIO_SCHEMA_VERSION,
            realm_id: RealmId::new("realm"),
            authority_scope_id: AuthorityScopeId::new("scope"),
        }
    }

    fn engine_revision(segments: Vec<TranscriptSegment>, route: AudioRoute) -> TranscriptRevision {
        let engine = (route == AudioRoute::FixtureAsr).then(|| AsrEngineIdentity {
            engine_id: "fixture".to_owned(),
            version: "1".to_owned(),
            is_fixture: true,
        });
        TranscriptRevision {
            header: h("t1"),
            project_id: OpaqueId::new("p"),
            source_id: OpaqueId::new("s"),
            source_digest: DigestSha256::of(b"wav"),
            revision_no: 1,
            origin: TranscriptOrigin::Engine {
                route,
                engine,
                vad: VadParameters::FROZEN,
            },
            mode: VoiceInputMode::Dictation,
            language: Some("en".to_owned()),
            diarization: DiarizationState::NotRequested,
            state: TranscriptRevision::derived_state(&segments),
            segments,
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

    #[test]
    fn vocabularies_round_trip_and_are_closed() {
        for v in AudioRoute::ALL {
            assert_eq!(AudioRoute::parse(v.as_str()), Ok(*v));
        }
        for v in CaptureState::ALL {
            assert_eq!(CaptureState::parse(v.as_str()), Ok(*v));
        }
        for v in VoiceInputMode::ALL {
            assert_eq!(VoiceInputMode::parse(v.as_str()), Ok(*v));
        }
        for v in CaptureHealth::ALL {
            assert_eq!(CaptureHealth::parse(v.as_str()), Ok(*v));
        }
        assert!(AudioRoute::parse("remote_asr").is_err());
        assert!(serde_json::from_str::<AudioRoute>("\"whisper_cloud\"").is_err());
        assert!(!CaptureBackendKind::NativeDevice.available());
        assert!(
            CaptureBackendKind::NativeDevice
                .unavailable_reason()
                .is_some()
        );
    }

    #[test]
    fn capture_transitions_are_explicit() {
        use CaptureState::*;
        assert!(Recording.can_become(Paused));
        assert!(Paused.can_become(Recording));
        assert!(Recording.can_become(Stopped));
        assert!(Paused.can_become(Cancelled));
        for terminal in [Stopped, Cancelled, Interrupted] {
            for next in CaptureState::ALL {
                assert!(!terminal.can_become(*next), "{terminal:?} -> {next:?}");
            }
        }
        assert!(!Recording.can_become(Recording));
    }

    #[test]
    fn formats_and_sessions_validate() {
        assert!(PcmFormat::mono_16k().validate().is_ok());
        assert_eq!(PcmFormat::mono_16k().duration_ms(32_000), 1_000);
        for bad in [
            PcmFormat {
                sample_rate: 4_000,
                ..PcmFormat::mono_16k()
            },
            PcmFormat {
                channels: 3,
                ..PcmFormat::mono_16k()
            },
            PcmFormat {
                bits_per_sample: 24,
                ..PcmFormat::mono_16k()
            },
        ] {
            assert!(bad.validate().is_err());
        }
        let mut s = AudioSession::start(
            h("c1"),
            OpaqueId::new("p"),
            "ward round".to_owned(),
            CaptureBackendKind::Scripted,
            PcmFormat::mono_16k(),
            OpaqueId::new("holder"),
        );
        assert!(s.validate().is_ok());
        s.captured_bytes = 3;
        assert!(s.validate().is_err(), "partial frame");
        s.captured_bytes = 4;
        s.state = CaptureState::Stopped;
        assert!(s.validate().is_err(), "stopped without source");
        s.source_id = Some(OpaqueId::new("src"));
        assert!(s.validate().is_ok());
        s.backend = CaptureBackendKind::NativeDevice;
        assert!(s.validate().is_err(), "native backend is unavailable");
    }

    #[test]
    fn transcript_revisions_hold_their_invariants() {
        let ok = engine_revision(
            vec![seg(1, 0, 500, Some("a")), seg(2, 600, 900, Some("b"))],
            AudioRoute::FixtureAsr,
        );
        assert!(ok.validate(1_000).is_ok());
        assert_eq!(ok.state, TranscriptState::Complete);
        let seg_only = engine_revision(vec![seg(1, 0, 500, None)], AudioRoute::SegmentationOnly);
        assert!(seg_only.validate(1_000).is_ok());
        assert_eq!(seg_only.state, TranscriptState::Untranscribed);

        let overlap = engine_revision(
            vec![seg(1, 0, 500, None), seg(2, 400, 900, None)],
            AudioRoute::SegmentationOnly,
        );
        assert!(overlap.validate(1_000).is_err());
        let outside = engine_revision(vec![seg(1, 0, 1_500, None)], AudioRoute::SegmentationOnly);
        assert!(outside.validate(1_000).is_err());
        let cloud = engine_revision(vec![], AudioRoute::CloudAsr);
        assert!(cloud.validate(1_000).is_err());
        let mut unlabeled_fixture = engine_revision(vec![], AudioRoute::SegmentationOnly);
        unlabeled_fixture.origin = TranscriptOrigin::Engine {
            route: AudioRoute::FixtureAsr,
            engine: None,
            vad: VadParameters::FROZEN,
        };
        assert!(unlabeled_fixture.validate(1_000).is_err());
        let mut wrong_state = ok.clone();
        wrong_state.state = TranscriptState::Partial;
        assert!(wrong_state.validate(1_000).is_err());
        let mut corrected_by_engine = ok.clone();
        corrected_by_engine.segments[0].status = SegmentStatus::Corrected;
        assert!(corrected_by_engine.validate(1_000).is_err());

        let mut correction = ok.clone();
        correction.revision_no = 2;
        correction.origin = TranscriptOrigin::HumanCorrection {
            parent_revision_id: OpaqueId::new("t1"),
            corrected_by: OpaqueId::new("holder"),
            reason: "drug name".to_owned(),
        };
        correction.segments[0].status = SegmentStatus::Corrected;
        assert!(correction.validate(1_000).is_ok());
        let mut first_correction = correction.clone();
        first_correction.revision_no = 1;
        assert!(first_correction.validate(1_000).is_err());

        let evidence = ok.evidence_ref(2).unwrap();
        assert_eq!((evidence.start_ms, evidence.end_ms), (600, 900));
        assert_eq!(evidence.source_digest, ok.source_digest);
        assert!(ok.evidence_ref(3).is_none());
    }

    #[test]
    fn receipts_state_their_limits() {
        let mut r = TranscriptReceipt {
            header: h("r1"),
            project_id: OpaqueId::new("p"),
            source_id: OpaqueId::new("s"),
            source_digest: DigestSha256::of(b"wav"),
            request_digest: DigestSha256::of(b"req"),
            route: AudioRoute::CloudAsr,
            decision: AudioRouteDecision::deny(AudioRouteDenyReason::CloudRouteForbidden),
            transcript_revision_id: None,
            engine: None,
            vad: VadParameters::FROZEN,
            segment_count: 0,
            limitations: vec![AudioLimitation::TranscriptNotClinicalTruth],
        };
        assert!(r.validate().is_ok());
        r.transcript_revision_id = Some(OpaqueId::new("t"));
        assert!(r.validate().is_err(), "denied request with a revision");
        r.transcript_revision_id = None;
        r.limitations.clear();
        assert!(r.validate().is_err(), "missing clinical-truth limitation");
        r.limitations = vec![AudioLimitation::TranscriptNotClinicalTruth];
        r.decision = AudioRouteDecision::allow();
        r.transcript_revision_id = Some(OpaqueId::new("t"));
        r.engine = Some(AsrEngineIdentity {
            engine_id: "fixture".to_owned(),
            version: "1".to_owned(),
            is_fixture: true,
        });
        assert!(r.validate().is_err(), "fixture without its limitation");
        r.limitations
            .push(AudioLimitation::FixtureEngineIsNotSpeechRecognition);
        assert!(r.validate().is_ok());
        assert!(
            AudioRouteDecision {
                outcome: AudioRouteOutcome::Allow,
                reason: Some(AudioRouteDenyReason::RouteUnavailable)
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn speaker_labels_serialize_without_identity() {
        let json = serde_json::to_string(&SpeakerLabel::Anonymous { index: 1 }).unwrap();
        assert_eq!(json, r#"{"kind":"anonymous","index":1}"#);
        assert!(serde_json::from_str::<SpeakerLabel>(r#"{"kind":"named","name":"x"}"#).is_err());
    }
}

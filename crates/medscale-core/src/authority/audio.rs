//! AudioFlow Foundation Core authority paths (Spec 081).
//!
//! Local only: no function here reaches the network, and no route can
//! fall back to a remote engine. Source audio is immutable evidence;
//! transcripts are derived revisions; corrections create new revisions.
//! `COMMAND`-mode text is recorded and never executed.
//!
//! Deterministic signal processing (WAV parsing, voice-activity
//! segmentation, health) lives here as pure functions; speech engines sit
//! behind `AsrEngine`, of which this build ships only a labelled fixture.

use std::collections::HashSet;
use std::sync::Mutex;

use medscale_contracts::audio::{
    AUDIO_SCHEMA_VERSION, AsrEngineIdentity, AudioEvidenceRef, AudioLimitation, AudioRoute,
    AudioRouteDecision, AudioRouteDenyReason, AudioRouteRequest, AudioRouteStatus, AudioSession,
    AudioSource, AudioSourceKind, CaptureBackendKind, CaptureHealth, CaptureState,
    DiarizationState, MAX_AUDIO_BYTES, MAX_AUDIO_DURATION_MS, MAX_CAPTURE_CHUNK_BYTES,
    MAX_SEGMENTS, PcmFormat, SEGMENT_TEXT_MAX_CHARS, SegmentStatus, SpeakerLabel, TranscriptOrigin,
    TranscriptReceipt, TranscriptRevision, TranscriptSegment, TranscriptView, VadParameters,
    audio_request_digest,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId, VaultId};
use medscale_storage::{MetaError, SqliteMetaStore};

use super::store::{InMemoryAuthorityStore, StoredObject};
use crate::process::{LeaseRegistry, SessionRegistry};

const WAV_HEADER_BYTES: usize = 44;

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

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

// ----- deterministic signal processing ---------------------------------------

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *b.get(at)?,
        *b.get(at + 1)?,
        *b.get(at + 2)?,
        *b.get(at + 3)?,
    ]))
}

/// Parses a RIFF/WAVE file holding 16-bit PCM. Returns the format and the
/// PCM data. Anything else (compressed, extensible, truncated, trailing
/// garbage inside the RIFF size) is refused with a reason.
pub fn parse_wav(bytes: &[u8]) -> Result<(PcmFormat, &[u8]), String> {
    if bytes.len() > MAX_AUDIO_BYTES {
        return Err("audio file exceeds the 32 MiB bound".to_owned());
    }
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".to_owned());
    }
    let riff_end = read_u32(bytes, 4)
        .and_then(|n| usize::try_from(n).ok())
        .and_then(|n| n.checked_add(8))
        .ok_or("bad RIFF size")?;
    if riff_end > bytes.len() {
        return Err("truncated RIFF file".to_owned());
    }
    let mut at = 12;
    let mut format: Option<PcmFormat> = None;
    let mut data: Option<&[u8]> = None;
    while at + 8 <= riff_end {
        let id = &bytes[at..at + 4];
        let size = read_u32(bytes, at + 4)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or("bad chunk size")?;
        let body_start = at + 8;
        let body_end = body_start.checked_add(size).ok_or("bad chunk size")?;
        if body_end > riff_end {
            return Err("chunk runs past the end of the file".to_owned());
        }
        let body = &bytes[body_start..body_end];
        match id {
            b"fmt " => {
                if format.is_some() {
                    return Err("duplicate fmt chunk".to_owned());
                }
                if body.len() < 16 {
                    return Err("short fmt chunk".to_owned());
                }
                let audio_format = read_u16(body, 0).ok_or("short fmt chunk")?;
                if audio_format != 1 {
                    return Err("only uncompressed PCM is supported".to_owned());
                }
                let f = PcmFormat {
                    channels: read_u16(body, 2).ok_or("short fmt chunk")?,
                    sample_rate: read_u32(body, 4).ok_or("short fmt chunk")?,
                    bits_per_sample: read_u16(body, 14).ok_or("short fmt chunk")?,
                };
                f.validate()?;
                let block_align = read_u16(body, 12).ok_or("short fmt chunk")?;
                if u64::from(block_align) != f.frame_bytes() {
                    return Err("inconsistent block alignment".to_owned());
                }
                format = Some(f);
            }
            b"data" => {
                if data.is_some() {
                    return Err("duplicate data chunk".to_owned());
                }
                data = Some(body);
            }
            _ => {}
        }
        // Chunks are padded to an even length.
        at = body_end + (size & 1);
    }
    let format = format.ok_or("missing fmt chunk")?;
    let data = data.ok_or("missing data chunk")?;
    if data.is_empty() {
        return Err("empty audio".to_owned());
    }
    if !(data.len() as u64).is_multiple_of(format.frame_bytes()) {
        return Err("data is not whole PCM frames".to_owned());
    }
    if format.duration_ms(data.len() as u64) > MAX_AUDIO_DURATION_MS {
        return Err("audio exceeds 30 minutes".to_owned());
    }
    Ok((format, data))
}

/// Canonical 44-byte-header WAV file for PCM data.
#[must_use]
pub fn build_wav(format: PcmFormat, pcm: &[u8]) -> Vec<u8> {
    let data_len = u32::try_from(pcm.len()).unwrap_or(u32::MAX);
    let block_align = format.channels * (format.bits_per_sample / 8);
    let byte_rate = format.sample_rate * u32::from(block_align);
    let mut out = Vec::with_capacity(WAV_HEADER_BYTES + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&format.channels.to_le_bytes());
    out.extend_from_slice(&format.sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&format.bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

/// Synthetic PCM for fixtures and demos: each `(duration_ms, tone)` part is
/// a 440 Hz tone at about -13 dBFS or digital silence. Never speech.
#[must_use]
pub fn synthetic_pcm(format: PcmFormat, parts: &[(u32, bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offset: u64 = 0;
    for &(ms, tone) in parts {
        let frames = u64::from(ms) * u64::from(format.sample_rate) / 1_000;
        for i in 0..frames {
            let sample = if tone {
                let t = (offset + i) as f64 / f64::from(format.sample_rate);
                (0.3 * 32_767.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) as i16
            } else {
                0
            };
            for _ in 0..format.channels {
                out.extend_from_slice(&sample.to_le_bytes());
            }
        }
        offset += frames;
    }
    out
}

/// Segments and health of PCM audio under the frozen VAD parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalAnalysis {
    pub segments: Vec<(u64, u64)>,
    pub health: CaptureHealth,
}

/// Deterministic energy voice-activity segmentation plus health. Channels
/// are averaged; each frame's RMS is compared with the threshold; runs of
/// active frames joined across gaps shorter than the hangover become
/// segments, and runs shorter than the minimum are dropped.
#[must_use]
pub fn analyze_pcm(format: PcmFormat, pcm: &[u8], vad: VadParameters) -> SignalAnalysis {
    let channels = usize::from(format.channels.max(1));
    let samples: Vec<i32> = pcm
        .chunks_exact(2 * channels)
        .map(|frame| {
            let sum: i32 = frame
                .chunks_exact(2)
                .map(|s| i32::from(i16::from_le_bytes([s[0], s[1]])))
                .sum();
            sum / channels as i32
        })
        .collect();
    if samples.is_empty() {
        return SignalAnalysis {
            segments: Vec::new(),
            health: CaptureHealth::NoAudio,
        };
    }
    let clipped = samples.iter().filter(|s| s.abs() >= 32_767).count();
    let per_frame = (u64::from(vad.frame_ms) * u64::from(format.sample_rate) / 1_000).max(1);
    let per_frame = usize::try_from(per_frame).unwrap_or(usize::MAX);
    let threshold = 32_768.0 * 10_f64.powf(f64::from(vad.threshold_dbfs) / 20.0);
    let active: Vec<bool> = samples
        .chunks(per_frame)
        .map(|frame| {
            let energy: f64 = frame.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
            (energy / frame.len() as f64).sqrt() >= threshold
        })
        .collect();
    let frame_ms = u64::from(vad.frame_ms);
    let total_ms = format.duration_ms(pcm.len() as u64);
    let hangover_frames = (u64::from(vad.hangover_ms) / frame_ms) as usize;
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < active.len() {
        if !active[i] {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i + 1;
        let mut j = end;
        while j < active.len() {
            if active[j] {
                end = j + 1;
                j += 1;
            } else if j + 1 - end < hangover_frames {
                j += 1;
            } else {
                break;
            }
        }
        runs.push((start, end));
        i = end;
    }
    let segments: Vec<(u64, u64)> = runs
        .into_iter()
        .map(|(s, e)| (s as u64 * frame_ms, (e as u64 * frame_ms).min(total_ms)))
        .filter(|(s, e)| e > s && e - s >= u64::from(vad.min_speech_ms))
        .take(MAX_SEGMENTS)
        .collect();
    let health = if clipped * 1_000 >= samples.len() {
        CaptureHealth::Clipping
    } else if !active.iter().any(|a| *a) {
        CaptureHealth::Silent
    } else {
        CaptureHealth::Ok
    };
    SignalAnalysis { segments, health }
}

// ----- engines ----------------------------------------------------------------

/// A local speech engine. It receives decoded PCM and the segment spans and
/// returns text per segment (`None` when it produced nothing).
pub trait AsrEngine: Send + Sync {
    fn identity(&self) -> AsrEngineIdentity;
    fn transcribe(
        &self,
        format: PcmFormat,
        pcm: &[u8],
        segments: &[(u64, u64)],
        language: Option<&str>,
    ) -> Result<Vec<Option<String>>, String>;
}

/// Synthetic fixture engine. It does not recognize speech: each segment
/// gets a fixed label naming its position and span.
#[derive(Debug, Default)]
pub struct FixtureAsrEngine;

impl AsrEngine for FixtureAsrEngine {
    fn identity(&self) -> AsrEngineIdentity {
        AsrEngineIdentity {
            engine_id: "medscale-fixture-asr".to_owned(),
            version: "1".to_owned(),
            is_fixture: true,
        }
    }

    fn transcribe(
        &self,
        _format: PcmFormat,
        _pcm: &[u8],
        segments: &[(u64, u64)],
        _language: Option<&str>,
    ) -> Result<Vec<Option<String>>, String> {
        Ok(segments
            .iter()
            .enumerate()
            .map(|(i, (s, e))| Some(format!("[fixture segment {} {s}-{e} ms]", i + 1)))
            .collect())
    }
}

/// Local speech recognition from an admitted Audio Pack (Spec 106): a candle
/// Whisper model. Real recognition, but medical accuracy is unmeasured and a
/// transcript stays a proposal for review.
pub struct PackAsrEngine {
    asr: medscale_pack::PreparedWhisperAsr,
    pack_id: String,
    version: String,
}

impl PackAsrEngine {
    #[must_use]
    pub fn new(asr: medscale_pack::PreparedWhisperAsr, pack_id: String, version: String) -> Self {
        Self {
            asr,
            pack_id,
            version,
        }
    }
}

impl AsrEngine for PackAsrEngine {
    fn identity(&self) -> AsrEngineIdentity {
        AsrEngineIdentity {
            engine_id: format!(
                "{}:{}",
                medscale_pack::CANDLE_WHISPER_ASR_RUNTIME_ID,
                self.pack_id
            ),
            version: format!(
                "{}+{}",
                self.version,
                &self.asr.content_digest().to_hex()[..12]
            ),
            is_fixture: false,
        }
    }

    fn transcribe(
        &self,
        format: PcmFormat,
        pcm: &[u8],
        segments: &[(u64, u64)],
        _language: Option<&str>,
    ) -> Result<Vec<Option<String>>, String> {
        if format.sample_rate != 16_000 || format.bits_per_sample != 16 {
            return Err("local ASR needs 16 kHz 16-bit PCM".to_owned());
        }
        let samples = medscale_pack::pcm16_to_mono_f32(pcm, format.channels)
            .ok_or_else(|| "PCM layout is not 16-bit frames".to_owned())?;
        segments
            .iter()
            .map(|&(start_ms, end_ms)| {
                let to_index =
                    |ms: u64| usize::try_from(ms.saturating_mul(16)).unwrap_or(usize::MAX);
                let (a, b) = (
                    to_index(start_ms).min(samples.len()),
                    to_index(end_ms).min(samples.len()),
                );
                if b <= a {
                    return Ok(None);
                }
                let text = self
                    .asr
                    .transcribe(&samples[a..b])
                    .map_err(|e| e.to_string())?;
                let text = text.trim().to_owned();
                Ok(if text.is_empty() { None } else { Some(text) })
            })
            .collect()
    }
}

/// Route availability given the engines installed in this process.
#[must_use]
pub fn route_statuses(fixture_installed: bool, local_installed: bool) -> Vec<AudioRouteStatus> {
    AudioRoute::ALL
        .iter()
        .map(|route| {
            let (available, reason) = match route {
                AudioRoute::SegmentationOnly => (true, None),
                AudioRoute::FixtureAsr if fixture_installed => (true, None),
                AudioRoute::FixtureAsr => (false, Some("no fixture engine installed")),
                AudioRoute::LocalAsrPack if local_installed => (true, None),
                AudioRoute::LocalAsrPack => (false, Some("no admitted local ASR Audio Pack")),
                AudioRoute::CloudAsr => (false, Some("audio never leaves the device")),
            };
            AudioRouteStatus {
                route: *route,
                available,
                reason: reason.map(str::to_owned),
            }
        })
        .collect()
}

/// A correction to one segment of the latest revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentEdit {
    pub seq: u32,
    pub text: String,
}

// ----- authority --------------------------------------------------------------

/// Authenticated AudioFlow view for one request.
pub struct Audio<'a> {
    pub store: &'a mut InMemoryAuthorityStore,
    pub meta: &'a SqliteMetaStore,
    pub engine: Option<&'a dyn AsrEngine>,
    /// Spec 106: the Audio Pack engine behind `LocalAsrPack`, if installed.
    pub local_engine: Option<&'a dyn AsrEngine>,
    /// Capture sessions started by this process. Any open session not in
    /// this set belongs to a process that ended: it becomes `interrupted`.
    pub live_captures: &'a Mutex<HashSet<String>>,
    pub sessions: &'a SessionRegistry,
    pub leases: &'a LeaseRegistry,
    pub vault_id: &'a VaultId,
    pub realm: RealmId,
    pub scope: AuthorityScopeId,
    pub session_id: Option<OpaqueId>,
}

impl Audio<'_> {
    pub(super) fn actor(&self) -> Result<OpaqueId, AuthorityError> {
        if let Some(holder) = self
            .session_id
            .as_ref()
            .and_then(|session_id| self.sessions.holder_of(session_id))
        {
            return Ok(holder);
        }
        self.leases
            .holder(self.vault_id)
            .ok_or(AuthorityError::LeaseRequired)
    }

    pub(super) fn audit(
        &mut self,
        action: &str,
        targets: Vec<OpaqueId>,
    ) -> Result<(), AuthorityError> {
        let actor = self.actor()?;
        let id = self.store.alloc_id("audit");
        let record = medscale_contracts::objects::ActionAuditRecord {
            header: ObjectHeader {
                id,
                schema_version: medscale_contracts::AUTHORITY_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            kind: medscale_contracts::objects::ActionAuditKind::Audit,
            actor,
            action: action.to_owned(),
            target_refs: targets,
            effect_state: None,
            payload_digest: None,
            detail: None,
        };
        self.store.insert(StoredObject::Audit(record));
        Ok(())
    }

    fn header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: AUDIO_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    pub(super) fn in_scope(&self, header: &ObjectHeader) -> Result<(), AuthorityError> {
        if header.realm_id != self.realm || header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(())
    }

    pub(super) fn scoped_project(&self, id: &OpaqueId) -> Result<(), AuthorityError> {
        let project = self.meta.get_project(id).map_err(meta_err)?;
        self.in_scope(&project.header)
    }

    fn alloc(&self, prefix: &str) -> Result<OpaqueId, AuthorityError> {
        self.meta.alloc_audio_id(prefix).map_err(meta_err)
    }

    fn live(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.live_captures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    // ----- sources -----

    /// Imports a WAV file as an immutable source.
    pub fn import(
        &mut self,
        project_id: OpaqueId,
        label: String,
        wav: Vec<u8>,
    ) -> Result<AudioSource, AuthorityError> {
        self.scoped_project(&project_id)?;
        let (format, pcm) = parse_wav(&wav).map_err(invalid)?;
        let analysis = analyze_pcm(format, pcm, VadParameters::FROZEN);
        let duration_ms = format.duration_ms(pcm.len() as u64);
        let mut source = AudioSource {
            header: self.header(OpaqueId::new("pending")),
            project_id,
            kind: AudioSourceKind::ImportedFile,
            label,
            format,
            duration_ms,
            byte_length: wav.len() as u64,
            content_digest: medscale_contracts::objects::DigestSha256::of(&wav),
            health: analysis.health,
            capture_session_id: None,
        };
        source.validate().map_err(invalid)?;
        source.header = self.header(self.alloc("audio-source")?);
        self.meta
            .insert_imported_audio_source(&source, &wav)
            .map_err(meta_err)?;
        self.audit("audio.import", vec![source.header.id.clone()])?;
        Ok(source)
    }

    pub fn get_source(&self, id: &OpaqueId) -> Result<AudioSource, AuthorityError> {
        let (source, _) = self.meta.get_audio_source(id).map_err(meta_err)?;
        self.in_scope(&source.header)?;
        Ok(source)
    }

    pub fn list_sources(&self, project_id: &OpaqueId) -> Result<Vec<AudioSource>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta.list_audio_sources(project_id).map_err(meta_err)
    }

    // ----- capture -----

    /// Starts a capture on an explicit request. The native backend is not
    /// admitted in this build and is refused with its reason.
    pub fn capture_start(
        &mut self,
        project_id: OpaqueId,
        label: String,
        backend: CaptureBackendKind,
        format: PcmFormat,
    ) -> Result<AudioSession, AuthorityError> {
        self.scoped_project(&project_id)?;
        if let Some(reason) = backend.unavailable_reason() {
            return Err(AuthorityError::Unavailable {
                message: reason.to_owned(),
            });
        }
        let started_by = self.actor()?;
        let probe = AudioSession::start(
            self.header(OpaqueId::new("pending")),
            project_id.clone(),
            label.clone(),
            backend,
            format,
            started_by.clone(),
        );
        probe.validate().map_err(invalid)?;
        let id = self.alloc("audio-capture")?;
        let session = AudioSession::start(
            self.header(id.clone()),
            project_id,
            label,
            backend,
            format,
            started_by,
        );
        self.meta
            .insert_capture_session(&session)
            .map_err(meta_err)?;
        self.live().insert(id.as_str().to_owned());
        self.audit("audio.capture.start", vec![id])?;
        Ok(session)
    }

    /// Reads a session, first marking it `interrupted` when it is open but
    /// was not started by this process (its owner ended mid-capture).
    fn scoped_capture(&mut self, id: &OpaqueId) -> Result<AudioSession, AuthorityError> {
        let session = self.meta.get_capture_session(id).map_err(meta_err)?;
        self.in_scope(&session.header)?;
        if session.state.is_open() && !self.live().contains(id.as_str()) {
            let recovered = self
                .meta
                .transition_capture_session(id, session.revision, CaptureState::Interrupted)
                .map_err(meta_err)?;
            self.audit("audio.capture.interrupted", vec![id.clone()])?;
            return Ok(recovered);
        }
        Ok(session)
    }

    pub fn capture_get(&mut self, id: &OpaqueId) -> Result<AudioSession, AuthorityError> {
        self.scoped_capture(id)
    }

    pub fn capture_list(
        &mut self,
        project_id: &OpaqueId,
    ) -> Result<Vec<AudioSession>, AuthorityError> {
        self.scoped_project(project_id)?;
        let ids: Vec<OpaqueId> = self
            .meta
            .list_capture_sessions(project_id)
            .map_err(meta_err)?
            .into_iter()
            .map(|s| s.header.id)
            .collect();
        ids.iter().map(|id| self.scoped_capture(id)).collect()
    }

    /// Appends PCM frames to a recording session (scripted backend only).
    pub fn capture_append(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
        frames: Vec<u8>,
    ) -> Result<AudioSession, AuthorityError> {
        let session = self.scoped_capture(id)?;
        if session.backend != CaptureBackendKind::Scripted {
            return Err(invalid("only scripted captures accept pushed frames"));
        }
        if frames.is_empty() || frames.len() > MAX_CAPTURE_CHUNK_BYTES {
            return Err(invalid("a chunk holds 1 byte to 1 MiB of PCM"));
        }
        if !(frames.len() as u64).is_multiple_of(session.format.frame_bytes()) {
            return Err(invalid("a chunk holds whole PCM frames"));
        }
        let total = session.captured_bytes + frames.len() as u64;
        if total + WAV_HEADER_BYTES as u64 > MAX_AUDIO_BYTES as u64
            || session.format.duration_ms(total) > MAX_AUDIO_DURATION_MS
        {
            return Err(invalid(
                "capture would exceed the audio size or length bound",
            ));
        }
        self.meta
            .append_capture_chunk(id, expected_revision, &frames)
            .map_err(meta_err)
    }

    /// Pause, resume or cancel.
    pub fn capture_transition(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
        to: CaptureState,
    ) -> Result<AudioSession, AuthorityError> {
        if !matches!(
            to,
            CaptureState::Paused | CaptureState::Recording | CaptureState::Cancelled
        ) {
            return Err(invalid("use stop to finish a capture"));
        }
        self.scoped_capture(id)?;
        let next = self
            .meta
            .transition_capture_session(id, expected_revision, to)
            .map_err(meta_err)?;
        if !to.is_open() {
            self.live().remove(id.as_str());
        }
        self.audit(&format!("audio.capture.{}", to.as_str()), vec![id.clone()])?;
        Ok(next)
    }

    /// Stops a capture and stores exactly the captured frames as a source.
    pub fn capture_stop(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<(AudioSession, AudioSource), AuthorityError> {
        let session = self.scoped_capture(id)?;
        if !session.state.can_become(CaptureState::Stopped) {
            return Err(AuthorityError::Conflict {
                message: format!("capture is {}", session.state.as_str()),
            });
        }
        let chunks = self.meta.capture_chunks(id).map_err(meta_err)?;
        let pcm: Vec<u8> = chunks.concat();
        if pcm.is_empty() {
            return Err(invalid("nothing was captured; cancel the capture instead"));
        }
        if pcm.len() as u64 != session.captured_bytes {
            return Err(AuthorityError::Corrupt {
                message: "captured chunks disagree with the session".to_owned(),
            });
        }
        let wav = build_wav(session.format, &pcm);
        let analysis = analyze_pcm(session.format, &pcm, VadParameters::FROZEN);
        let source = AudioSource {
            header: self.header(self.alloc("audio-source")?),
            project_id: session.project_id.clone(),
            kind: AudioSourceKind::Capture,
            label: session.label.clone(),
            format: session.format,
            duration_ms: session.format.duration_ms(pcm.len() as u64),
            byte_length: wav.len() as u64,
            content_digest: medscale_contracts::objects::DigestSha256::of(&wav),
            health: analysis.health,
            capture_session_id: Some(id.clone()),
        };
        let stopped = self
            .meta
            .stop_capture_session(id, expected_revision, &source, &wav)
            .map_err(meta_err)?;
        self.live().remove(id.as_str());
        self.audit(
            "audio.capture.stop",
            vec![id.clone(), source.header.id.clone()],
        )?;
        Ok((stopped, source))
    }

    // ----- transcripts -----

    /// Runs one transcription request. Every request, allowed or denied,
    /// leaves a receipt; an allowed one also leaves a new revision.
    pub fn transcribe(
        &mut self,
        request: AudioRouteRequest,
    ) -> Result<TranscriptView, AuthorityError> {
        self.scoped_project(&request.project_id)?;
        let (source, wav) = self
            .meta
            .get_audio_source(&request.source_id)
            .map_err(meta_err)?;
        self.in_scope(&source.header)?;
        if source.project_id != request.project_id {
            return Err(AuthorityError::WrongScope);
        }
        if let Some(language) = &request.language
            && (language.is_empty()
                || language.len() > medscale_contracts::audio::LANGUAGE_MAX_CHARS
                || !language
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'))
        {
            return Err(invalid("language must be a short BCP 47 tag"));
        }
        let vad = VadParameters::FROZEN;
        let deny = match request.route {
            _ if request.diarization_required => Some(AudioRouteDenyReason::DiarizationUnavailable),
            AudioRoute::CloudAsr => Some(AudioRouteDenyReason::CloudRouteForbidden),
            AudioRoute::LocalAsrPack if self.local_engine.is_none() => {
                Some(AudioRouteDenyReason::RouteUnavailable)
            }
            AudioRoute::LocalAsrPack => None,
            AudioRoute::FixtureAsr if self.engine.is_none() => {
                Some(AudioRouteDenyReason::RouteUnavailable)
            }
            AudioRoute::FixtureAsr | AudioRoute::SegmentationOnly => None,
        };
        let engine_identity = match request.route {
            AudioRoute::FixtureAsr if deny.is_none() => self.engine.map(|e| e.identity()),
            AudioRoute::LocalAsrPack if deny.is_none() => self.local_engine.map(|e| e.identity()),
            _ => None,
        };
        let mut limitations = vec![
            AudioLimitation::TranscriptNotClinicalTruth,
            AudioLimitation::SpeakerLabelsAreNotIdentity,
            AudioLimitation::DiarizationUnavailable,
            AudioLimitation::MedicalAccuracyUnmeasured,
        ];
        if engine_identity.as_ref().is_some_and(|e| e.is_fixture) {
            limitations.push(AudioLimitation::FixtureEngineIsNotSpeechRecognition);
        }

        let (decision, revision) = if let Some(reason) = deny {
            (AudioRouteDecision::deny(reason), None)
        } else {
            let pcm = match parse_wav(&wav) {
                Ok((_, pcm)) => pcm,
                Err(_) => {
                    return self.commit_transcription(
                        &request,
                        &source,
                        AudioRouteDecision::deny(AudioRouteDenyReason::SourceUnreadable),
                        None,
                        None,
                        limitations,
                    );
                }
            };
            let spans = analyze_pcm(source.format, pcm, vad).segments;
            let engine = match request.route {
                AudioRoute::FixtureAsr => self.engine,
                AudioRoute::LocalAsrPack => self.local_engine,
                _ => None,
            };
            let texts: Vec<Option<String>> = match (request.route, engine) {
                (AudioRoute::FixtureAsr | AudioRoute::LocalAsrPack, Some(engine)) => {
                    let out = engine
                        .transcribe(source.format, pcm, &spans, request.language.as_deref())
                        .map_err(|message| AuthorityError::Unavailable { message })?;
                    if out.len() != spans.len() {
                        return Err(AuthorityError::Internal {
                            message: "engine returned the wrong number of segments".to_owned(),
                        });
                    }
                    out.into_iter()
                        .map(|t| t.map(|t| t.chars().take(SEGMENT_TEXT_MAX_CHARS).collect()))
                        .collect()
                }
                _ => vec![None; spans.len()],
            };
            let segments: Vec<TranscriptSegment> = spans
                .iter()
                .zip(texts)
                .enumerate()
                .map(|(i, (&(start_ms, end_ms), text))| TranscriptSegment {
                    seq: u32::try_from(i + 1).unwrap_or(u32::MAX),
                    start_ms,
                    end_ms,
                    speaker: SpeakerLabel::Unknown,
                    status: if text.is_some() {
                        SegmentStatus::Transcribed
                    } else {
                        SegmentStatus::Untranscribed
                    },
                    text,
                })
                .collect();
            let latest = self
                .meta
                .list_transcript_revisions(&source.header.id)
                .map_err(meta_err)?
                .last()
                .map_or(0, |t| t.revision_no);
            let revision = TranscriptRevision {
                header: self.header(self.alloc("audio-transcript")?),
                project_id: source.project_id.clone(),
                source_id: source.header.id.clone(),
                source_digest: source.content_digest.clone(),
                revision_no: latest + 1,
                origin: TranscriptOrigin::Engine {
                    route: request.route,
                    engine: engine_identity.clone(),
                    vad,
                },
                mode: request.mode,
                language: request.language.clone(),
                diarization: DiarizationState::NotRequested,
                state: TranscriptRevision::derived_state(&segments),
                segments,
            };
            (AudioRouteDecision::allow(), Some(revision))
        };
        self.commit_transcription(
            &request,
            &source,
            decision,
            revision,
            engine_identity,
            limitations,
        )
    }

    fn commit_transcription(
        &mut self,
        request: &AudioRouteRequest,
        source: &AudioSource,
        decision: AudioRouteDecision,
        revision: Option<TranscriptRevision>,
        engine: Option<AsrEngineIdentity>,
        limitations: Vec<AudioLimitation>,
    ) -> Result<TranscriptView, AuthorityError> {
        let receipt = TranscriptReceipt {
            header: self.header(self.alloc("audio-receipt")?),
            project_id: source.project_id.clone(),
            source_id: source.header.id.clone(),
            source_digest: source.content_digest.clone(),
            request_digest: audio_request_digest(request),
            route: request.route,
            decision,
            transcript_revision_id: revision.as_ref().map(|t| t.header.id.clone()),
            engine: if revision.is_some() { engine } else { None },
            vad: VadParameters::FROZEN,
            segment_count: revision
                .as_ref()
                .map_or(0, |t| u32::try_from(t.segments.len()).unwrap_or(u32::MAX)),
            limitations,
        };
        receipt.validate().map_err(|e| AuthorityError::Internal {
            message: format!("transcript receipt invariant: {e}"),
        })?;
        self.meta
            .commit_transcript(revision.as_ref(), &receipt)
            .map_err(meta_err)?;
        let mut targets = vec![receipt.header.id.clone()];
        if let Some(t) = &revision {
            targets.push(t.header.id.clone());
        }
        self.audit("audio.transcribe", targets)?;
        Ok(TranscriptView { revision, receipt })
    }

    pub fn transcripts(
        &self,
        source_id: &OpaqueId,
    ) -> Result<Vec<TranscriptRevision>, AuthorityError> {
        self.get_source(source_id)?;
        self.meta
            .list_transcript_revisions(source_id)
            .map_err(meta_err)
    }

    pub fn receipts(&self, source_id: &OpaqueId) -> Result<Vec<TranscriptReceipt>, AuthorityError> {
        self.get_source(source_id)?;
        self.meta
            .list_transcript_receipts(source_id)
            .map_err(meta_err)
    }

    pub fn transcript(&self, id: &OpaqueId) -> Result<TranscriptRevision, AuthorityError> {
        let t = self.meta.get_transcript_revision(id).map_err(meta_err)?;
        self.in_scope(&t.header)?;
        Ok(t)
    }

    /// Creates a corrected revision of the latest revision. The parent is
    /// never modified.
    pub fn correct(
        &mut self,
        parent_id: &OpaqueId,
        edits: Vec<SegmentEdit>,
        reason: String,
    ) -> Result<TranscriptRevision, AuthorityError> {
        let parent = self.transcript(parent_id)?;
        let latest = self
            .meta
            .list_transcript_revisions(&parent.source_id)
            .map_err(meta_err)?
            .last()
            .map_or(0, |t| t.revision_no);
        if parent.revision_no != latest {
            return Err(AuthorityError::Conflict {
                message: format!(
                    "revision {} is not the latest (latest is {latest})",
                    parent.revision_no
                ),
            });
        }
        if edits.is_empty() {
            return Err(invalid("a correction changes at least one segment"));
        }
        let mut segments = parent.segments.clone();
        let mut seen = HashSet::new();
        for edit in edits {
            if !seen.insert(edit.seq) {
                return Err(invalid("each segment is corrected once"));
            }
            let text = edit.text.trim().to_owned();
            if text.is_empty() {
                return Err(invalid("corrected text must not be empty"));
            }
            let segment = segments
                .iter_mut()
                .find(|s| s.seq == edit.seq)
                .ok_or_else(|| invalid(format!("no segment {}", edit.seq)))?;
            segment.text = Some(text);
            segment.status = SegmentStatus::Corrected;
        }
        let corrected_by = self.actor()?;
        let revision = TranscriptRevision {
            header: self.header(self.alloc("audio-transcript")?),
            project_id: parent.project_id.clone(),
            source_id: parent.source_id.clone(),
            source_digest: parent.source_digest.clone(),
            revision_no: parent.revision_no + 1,
            origin: TranscriptOrigin::HumanCorrection {
                parent_revision_id: parent.header.id.clone(),
                corrected_by,
                reason,
            },
            mode: parent.mode,
            language: parent.language.clone(),
            diarization: parent.diarization,
            state: TranscriptRevision::derived_state(&segments),
            segments,
        };
        let source = self.get_source(&parent.source_id)?;
        revision.validate(source.duration_ms).map_err(invalid)?;
        self.meta
            .insert_transcript_correction(&revision)
            .map_err(meta_err)?;
        self.audit(
            "audio.transcript.correct",
            vec![revision.header.id.clone(), parent.header.id],
        )?;
        Ok(revision)
    }

    /// Evidence reference for one segment, checked against stored audio.
    pub fn evidence(
        &self,
        revision_id: &OpaqueId,
        seq: u32,
    ) -> Result<AudioEvidenceRef, AuthorityError> {
        let t = self.transcript(revision_id)?;
        let source = self.get_source(&t.source_id)?;
        let evidence = t.evidence_ref(seq).ok_or(AuthorityError::NotFound)?;
        if evidence.source_digest != source.content_digest || evidence.end_ms > source.duration_ms {
            return Err(AuthorityError::Corrupt {
                message: "evidence does not resolve to its source audio".to_owned(),
            });
        }
        Ok(evidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt() -> PcmFormat {
        PcmFormat::mono_16k()
    }

    #[test]
    fn wav_round_trips_and_malformed_files_are_refused() {
        let pcm = synthetic_pcm(fmt(), &[(500, true)]);
        let wav = build_wav(fmt(), &pcm);
        let (f, data) = parse_wav(&wav).unwrap();
        assert_eq!(f, fmt());
        assert_eq!(data, &pcm[..]);

        let mut bad_magic = wav.clone();
        bad_magic[0] = b'X';
        assert!(parse_wav(&bad_magic).is_err());
        assert!(parse_wav(&wav[..wav.len() - 1]).is_err(), "truncated");
        let mut float = wav.clone();
        float[20] = 3; // IEEE float
        assert!(parse_wav(&float).is_err());
        let mut bits24 = wav.clone();
        bits24[34] = 24;
        assert!(parse_wav(&bits24).is_err());
        let mut rate = wav.clone();
        rate[24..28].copy_from_slice(&96_000_u32.to_le_bytes());
        assert!(parse_wav(&rate).is_err());
        let mut huge_chunk = wav.clone();
        huge_chunk[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_wav(&huge_chunk).is_err());
        let odd = build_wav(fmt(), &[1, 2, 3]);
        assert!(parse_wav(&odd).is_err(), "partial frame");
        assert!(parse_wav(&build_wav(fmt(), &[])).is_err(), "empty");
        assert!(parse_wav(b"").is_err());
        let long = vec![0_u8; MAX_AUDIO_BYTES + 1];
        assert!(parse_wav(&long).is_err());
    }

    #[test]
    fn segmentation_is_deterministic_and_bounded() {
        let pcm = synthetic_pcm(
            fmt(),
            &[
                (300, false),
                (800, true),
                (200, false), // shorter than the hangover: same segment
                (400, true),
                (1_000, false),
                (100, true), // shorter than min speech: dropped
                (500, false),
                (600, true),
            ],
        );
        let a = analyze_pcm(fmt(), &pcm, VadParameters::FROZEN);
        let b = analyze_pcm(fmt(), &pcm, VadParameters::FROZEN);
        assert_eq!(a, b);
        assert_eq!(a.health, CaptureHealth::Ok);
        assert_eq!(a.segments, vec![(300, 1_700), (3_300, 3_900)]);

        let silent = synthetic_pcm(fmt(), &[(1_000, false)]);
        let s = analyze_pcm(fmt(), &silent, VadParameters::FROZEN);
        assert_eq!(s.health, CaptureHealth::Silent);
        assert!(s.segments.is_empty());

        let clipped: Vec<u8> = std::iter::repeat_n(i16::MAX.to_le_bytes(), 16_000)
            .flatten()
            .collect();
        assert_eq!(
            analyze_pcm(fmt(), &clipped, VadParameters::FROZEN).health,
            CaptureHealth::Clipping
        );
        assert_eq!(
            analyze_pcm(fmt(), &[], VadParameters::FROZEN).health,
            CaptureHealth::NoAudio
        );
        let stereo = PcmFormat {
            channels: 2,
            ..fmt()
        };
        let st = synthetic_pcm(stereo, &[(300, false), (800, true)]);
        assert_eq!(
            analyze_pcm(stereo, &st, VadParameters::FROZEN).segments,
            vec![(300, 1_100)]
        );
    }

    #[test]
    fn routes_never_offer_a_remote_engine() {
        for fixture in [false, true] {
            let routes = route_statuses(fixture, false);
            let cloud = routes
                .iter()
                .find(|r| r.route == AudioRoute::CloudAsr)
                .unwrap();
            assert!(!cloud.available);
            let pack = routes
                .iter()
                .find(|r| r.route == AudioRoute::LocalAsrPack)
                .unwrap();
            assert!(!pack.available && pack.reason.is_some());
            let fx = routes
                .iter()
                .find(|r| r.route == AudioRoute::FixtureAsr)
                .unwrap();
            assert_eq!(fx.available, fixture);
        }
        let engine = FixtureAsrEngine;
        assert!(engine.identity().is_fixture);
        let out = engine
            .transcribe(fmt(), &[], &[(0, 400), (500, 900)], None)
            .unwrap();
        assert_eq!(out.len(), 2);
        assert!(out[0].as_deref().unwrap().starts_with("[fixture segment 1"));
    }
}

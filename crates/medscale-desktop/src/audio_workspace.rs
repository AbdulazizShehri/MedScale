//! AudioFlow view-models (Spec 081).
//!
//! Desktop reaches AudioFlow only through the Core-owned `CliSession`; it
//! holds no audio device, engine or storage code. Every function maps one
//! typed Core result to plain rows plus an explicit status. Capture state is
//! always shown; the native device backend is shown as unavailable with its
//! reason rather than hidden.

use medscale_contracts::audio::{
    AudioRoute, AudioRouteRequest, CaptureBackendKind, TranscriptRevision, VoiceInputMode,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceRowVm {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub detail: String,
    pub health: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaptureRowVm {
    pub id: String,
    pub label: String,
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioOverviewVm {
    pub sources: Vec<SourceRowVm>,
    pub captures: Vec<CaptureRowVm>,
    pub routes: String,
    /// Why live device capture is not offered in this build.
    pub native_capture: String,
}

#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::Conflict { .. } => "Conflict: stale revision or already final",
        AuthorityError::InvalidArgument { .. } => "Invalid: rejected by Core",
        AuthorityError::Corrupt { .. } => "Corrupt: integrity check failed",
        _ => "Unavailable: vault or Core not ready",
    }
}

pub fn refresh(
    session: &mut CliSession,
    project_id: &str,
) -> Result<AudioOverviewVm, AuthorityError> {
    let project = OpaqueId::new(project_id);
    let sources = session
        .audio_source_list(project.clone())?
        .iter()
        .rev()
        .map(|s| SourceRowVm {
            id: s.header.id.as_str().to_owned(),
            label: s.label.clone(),
            kind: s.kind.as_str().to_owned(),
            detail: format!(
                "{} ms · {} Hz · {} ch",
                s.duration_ms, s.format.sample_rate, s.format.channels
            ),
            health: s.health.as_str().to_owned(),
        })
        .collect();
    let captures = session
        .audio_capture_list(project)?
        .iter()
        .rev()
        .map(|c| CaptureRowVm {
            id: c.header.id.as_str().to_owned(),
            label: c.label.clone(),
            state: c.state.as_str().to_owned(),
            detail: format!("{} · {} bytes", c.backend.as_str(), c.captured_bytes),
        })
        .collect();
    let routes = session
        .audio_routes()?
        .iter()
        .map(|r| {
            format!(
                "{} {}",
                r.route.as_str(),
                if r.available {
                    "available"
                } else {
                    "unavailable"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(AudioOverviewVm {
        sources,
        captures,
        routes,
        native_capture: CaptureBackendKind::NativeDevice
            .unavailable_reason()
            .unwrap_or_default()
            .to_owned(),
    })
}

/// Transcript lines for display (`[seq] start-end ms status: text`).
#[must_use]
pub fn transcript_lines(t: &TranscriptRevision) -> Vec<String> {
    t.segments
        .iter()
        .map(|s| {
            format!(
                "[{}] {}-{} ms · {} · speaker unknown: {}",
                s.seq,
                s.start_ms,
                s.end_ms,
                s.status.as_str(),
                s.text.as_deref().unwrap_or("(untranscribed)")
            )
        })
        .collect()
}

/// Runs deterministic segmentation (no speech engine) on one source.
pub fn segment(
    session: &mut CliSession,
    project_id: &str,
    source_id: &str,
) -> Result<(String, Vec<String>), AuthorityError> {
    let view = session.audio_transcribe(AudioRouteRequest {
        project_id: OpaqueId::new(project_id),
        source_id: OpaqueId::new(source_id),
        route: AudioRoute::SegmentationOnly,
        mode: VoiceInputMode::Dictation,
        language: None,
        diarization_required: false,
    })?;
    let lines = view
        .revision
        .as_ref()
        .map(transcript_lines)
        .unwrap_or_default();
    let summary = format!(
        "{} · {} segment{} · not clinical truth",
        view.receipt.decision.outcome.as_str(),
        view.receipt.segment_count,
        if view.receipt.segment_count == 1 {
            ""
        } else {
            "s"
        }
    );
    Ok((summary, lines))
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::audio::PcmFormat;

    #[test]
    fn audio_view_models_flow_through_a_real_core_session() {
        let root =
            std::env::temp_dir().join(format!("medscale-081-desktop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut s = CliSession::connect("vault-081-desktop").unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let project = s
            .project_create("audio".to_owned(), None)
            .unwrap()
            .header
            .id;
        let pid = project.as_str().to_owned();

        let empty = refresh(&mut s, &pid).unwrap();
        assert!(empty.sources.is_empty() && empty.captures.is_empty());
        assert!(empty.routes.contains("cloud_asr unavailable"));
        assert!(
            empty
                .native_capture
                .contains("no admitted native capture backend")
        );

        let wav = CliSession::synthetic_wav(
            PcmFormat::mono_16k(),
            &[(300, false), (800, true), (600, false), (400, true)],
        );
        let source = s
            .audio_import(project.clone(), "import".to_owned(), wav)
            .unwrap();
        let capture = s
            .audio_capture_start(
                project.clone(),
                "left open".to_owned(),
                CaptureBackendKind::Scripted,
                PcmFormat::mono_16k(),
            )
            .unwrap();

        let overview = refresh(&mut s, &pid).unwrap();
        assert_eq!(overview.sources.len(), 1);
        assert_eq!(overview.sources[0].health, "ok");
        assert_eq!(overview.captures[0].id, capture.header.id.as_str());
        assert_eq!(overview.captures[0].state, "recording");

        let (summary, lines) = segment(&mut s, &pid, source.header.id.as_str()).unwrap();
        assert!(summary.starts_with("allow · 2 segments"));
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("300-1100 ms") && lines[0].contains("(untranscribed)"));
        assert!(segment(&mut s, &pid, "audio-source-missing").is_err());
    }
}

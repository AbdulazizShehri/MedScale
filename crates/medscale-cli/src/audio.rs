//! Spec 081 AudioFlow Foundation commands (CLI vertical slice through Core).
//!
//! Every command dispatches typed Core requests via `CliSession` and renders
//! typed results as human lines or stable JSON. Audio never leaves the
//! device. Each CLI invocation is its own process: a capture left open when
//! the command ends is reported as `interrupted` by the next command, so
//! `capture` starts, fills and stops a scripted capture in one invocation.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::audio::{
    AudioRoute, AudioRouteRequest, AudioSession, AudioSource, CaptureBackendKind, PcmFormat,
    TranscriptRevision, TranscriptView, VoiceInputMode,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

use super::{fail_json, print_json_or_debug};

fn audio_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => ("denied", debug),
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::Unavailable { message } => ("unavailable", message.clone()),
        AuthorityError::LeaseRequired | AuthorityError::VaultRequired => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn open_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    fixture_engine: bool,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open_configured(vault_id, vault_root, |session| {
        if fixture_engine {
            session.use_fixture_asr();
        }
    })
    .map_err(|err| audio_fail(&err, json))
}

fn print_source(s: &AudioSource, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(s, true);
    }
    println!(
        "{}\t{}\t{}\t{} Hz x{}\t{} ms\t{}\tsha256:{}",
        s.header.id.as_str(),
        s.kind.as_str(),
        s.label.escape_debug(),
        s.format.sample_rate,
        s.format.channels,
        s.duration_ms,
        s.health.as_str(),
        s.content_digest.to_hex()
    );
    Ok(())
}

fn print_capture(s: &AudioSession, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(s, true);
    }
    println!(
        "{}\t{}\t{}\t{} bytes\trev {}\tsource {}",
        s.header.id.as_str(),
        s.backend.as_str(),
        s.state.as_str(),
        s.captured_bytes,
        s.revision,
        s.source_id.as_ref().map_or("-", OpaqueId::as_str)
    );
    Ok(())
}

fn print_revision(t: &TranscriptRevision) {
    println!(
        "revision {} ({}) mode={} state={} diarization={}",
        t.revision_no,
        t.header.id.as_str(),
        t.mode.as_str(),
        t.state.as_str(),
        t.diarization.as_str()
    );
    for s in &t.segments {
        println!(
            "  [{}] {}-{} ms speaker=unknown {}: {}",
            s.seq,
            s.start_ms,
            s.end_ms,
            s.status.as_str(),
            s.text.as_deref().unwrap_or("-").escape_debug()
        );
    }
}

fn print_view(v: &TranscriptView, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(v, true);
    }
    let r = &v.receipt;
    println!("receipt_id: {}", r.header.id.as_str());
    println!("route: {}", r.route.as_str());
    println!("decision: {}", r.decision.outcome.as_str());
    if let Some(reason) = r.decision.reason {
        println!("reason: {}", reason.as_str());
    }
    if let Some(engine) = &r.engine {
        println!(
            "engine: {} {}{}",
            engine.engine_id,
            engine.version,
            if engine.is_fixture {
                " (fixture; not speech recognition)"
            } else {
                ""
            }
        );
    }
    for l in &r.limitations {
        println!(
            "limitation: {}",
            serde_json::to_string(l).unwrap_or_default()
        );
    }
    if let Some(t) = &v.revision {
        print_revision(t);
    }
    Ok(())
}

/// Spec 081 AudioFlow commands. Local only.
#[derive(Debug, Subcommand)]
pub enum AudioCmd {
    /// Import a 16-bit PCM WAV file as an immutable source.
    Import {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// List the Project's audio sources.
    Sources {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Explicitly start a capture, feed it synthetic tone/silence, and stop
    /// it into a source. `--native` asks for the device backend, which is
    /// not admitted in this build.
    Capture {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        label: String,
        /// Synthetic parts as `ms:tone` or `ms:silence`, comma separated.
        #[arg(long, default_value = "300:silence,800:tone,400:silence")]
        synthetic: String,
        #[arg(long)]
        native: bool,
        /// Leave the capture recording when this process ends (the next
        /// command reports it as interrupted).
        #[arg(long)]
        leave_open: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show one capture session (an orphaned open capture becomes interrupted).
    CaptureShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Show which transcription routes this process can run.
    Routes {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        /// Install the labelled fixture engine first.
        #[arg(long)]
        fixture_engine: bool,
        #[arg(long)]
        json: bool,
    },
    /// Request a transcript revision for one source.
    Transcribe {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        source_id: String,
        /// segmentation_only | fixture_asr | local_asr_pack | cloud_asr
        #[arg(long, default_value = "segmentation_only")]
        route: String,
        /// command | context | dictation
        #[arg(long, default_value = "dictation")]
        mode: String,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        diarize: bool,
        #[arg(long)]
        fixture_engine: bool,
        #[arg(long)]
        json: bool,
    },
    /// List a source's transcript revisions.
    Transcripts {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Correct one segment of the latest revision into a new revision.
    Correct {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        revision_id: String,
        #[arg(long)]
        segment: u32,
        #[arg(long)]
        text: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        json: bool,
    },
    /// Resolve one segment to its source audio span.
    Evidence {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        revision_id: String,
        #[arg(long)]
        segment: u32,
        #[arg(long)]
        json: bool,
    },
}

fn parse_parts(spec: &str) -> Result<Vec<(u32, bool)>, String> {
    spec.split(',')
        .map(|part| {
            let (ms, kind) = part
                .trim()
                .split_once(':')
                .ok_or_else(|| format!("bad part {part}"))?;
            let ms: u32 = ms.parse().map_err(|_| format!("bad duration {ms}"))?;
            if ms == 0 || ms > 600_000 {
                return Err("each part is 1 ms to 10 minutes".to_owned());
            }
            match kind {
                "tone" => Ok((ms, true)),
                "silence" => Ok((ms, false)),
                other => Err(format!("unknown part kind {other}")),
            }
        })
        .collect()
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

#[allow(clippy::too_many_lines)]
pub fn run_audio(cmd: AudioCmd) -> anyhow::Result<()> {
    match cmd {
        AudioCmd::Import {
            vault_id,
            vault_root,
            project_id,
            label,
            file,
            json,
        } => {
            let wav = std::fs::read(&file)
                .map_err(|e| invalid(format!("cannot read {}: {e}", file.display()), json))?;
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let source = s
                .audio_import(OpaqueId::new(project_id), label, wav)
                .map_err(|err| audio_fail(&err, json))?;
            print_source(&source, json)
        }
        AudioCmd::Sources {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let sources = s
                .audio_source_list(OpaqueId::new(project_id))
                .map_err(|err| audio_fail(&err, json))?;
            if json {
                return print_json_or_debug(&sources, true);
            }
            for source in &sources {
                print_source(source, false)?;
            }
            Ok(())
        }
        AudioCmd::Capture {
            vault_id,
            vault_root,
            project_id,
            label,
            synthetic,
            native,
            leave_open,
            json,
        } => {
            let parts = parse_parts(&synthetic).map_err(|e| invalid(e, json))?;
            let format = PcmFormat::mono_16k();
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let backend = if native {
                CaptureBackendKind::NativeDevice
            } else {
                CaptureBackendKind::Scripted
            };
            let started = s
                .audio_capture_start(OpaqueId::new(project_id), label, backend, format)
                .map_err(|err| audio_fail(&err, json))?;
            let appended = s
                .audio_capture_append(
                    started.header.id.clone(),
                    started.revision,
                    CliSession::synthetic_pcm(format, &parts),
                )
                .map_err(|err| audio_fail(&err, json))?;
            if leave_open {
                return print_capture(&appended, json);
            }
            let (session, source) = s
                .audio_capture_stop(appended.header.id.clone(), appended.revision)
                .map_err(|err| audio_fail(&err, json))?;
            if json {
                return print_json_or_debug(
                    &serde_json::json!({ "session": session, "source": source }),
                    true,
                );
            }
            print_capture(&session, false)?;
            print_source(&source, false)
        }
        AudioCmd::CaptureShow {
            vault_id,
            vault_root,
            session_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let session = s
                .audio_capture_get(OpaqueId::new(session_id))
                .map_err(|err| audio_fail(&err, json))?;
            print_capture(&session, json)
        }
        AudioCmd::Routes {
            vault_id,
            vault_root,
            fixture_engine,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, fixture_engine, json)?;
            let routes = s.audio_routes().map_err(|err| audio_fail(&err, json))?;
            if json {
                return print_json_or_debug(&routes, true);
            }
            for r in &routes {
                println!(
                    "{}\t{}\t{}",
                    r.route.as_str(),
                    if r.available {
                        "available"
                    } else {
                        "unavailable"
                    },
                    r.reason.as_deref().unwrap_or("-")
                );
            }
            Ok(())
        }
        AudioCmd::Transcribe {
            vault_id,
            vault_root,
            project_id,
            source_id,
            route,
            mode,
            language,
            diarize,
            fixture_engine,
            json,
        } => {
            let route = AudioRoute::parse(&route).map_err(|e| invalid(e, json))?;
            let mode = VoiceInputMode::parse(&mode).map_err(|e| invalid(e, json))?;
            let mut s = open_session(&vault_id, &vault_root, fixture_engine, json)?;
            let view = s
                .audio_transcribe(AudioRouteRequest {
                    project_id: OpaqueId::new(project_id),
                    source_id: OpaqueId::new(source_id),
                    route,
                    mode,
                    language,
                    diarization_required: diarize,
                })
                .map_err(|err| audio_fail(&err, json))?;
            print_view(&view, json)
        }
        AudioCmd::Transcripts {
            vault_id,
            vault_root,
            source_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let revisions = s
                .audio_transcript_list(OpaqueId::new(source_id))
                .map_err(|err| audio_fail(&err, json))?;
            if json {
                return print_json_or_debug(&revisions, true);
            }
            for t in &revisions {
                print_revision(t);
            }
            Ok(())
        }
        AudioCmd::Correct {
            vault_id,
            vault_root,
            revision_id,
            segment,
            text,
            reason,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let t = s
                .audio_transcript_correct(OpaqueId::new(revision_id), vec![(segment, text)], reason)
                .map_err(|err| audio_fail(&err, json))?;
            if json {
                return print_json_or_debug(&t, true);
            }
            print_revision(&t);
            Ok(())
        }
        AudioCmd::Evidence {
            vault_id,
            vault_root,
            revision_id,
            segment,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let e = s
                .audio_evidence(OpaqueId::new(revision_id), segment)
                .map_err(|err| audio_fail(&err, json))?;
            if json {
                return print_json_or_debug(&e, true);
            }
            println!(
                "source {} sha256:{} {}-{} ms (revision {}, segment {})",
                e.source_id.as_str(),
                e.source_digest.to_hex(),
                e.start_ms,
                e.end_ms,
                e.transcript_revision_id.as_str(),
                e.segment_seq
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::audio::{AudioRouteDenyReason, CaptureState, SegmentStatus};

    const VAULT: &str = "vault-081-cli";

    fn base(root: &std::path::Path) -> (String, PathBuf) {
        (VAULT.to_owned(), root.to_path_buf())
    }

    #[test]
    fn audio_commands_run_through_core_across_fresh_sessions() {
        let root = std::env::temp_dir().join(format!("medscale-081-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let project = session
            .project_create("audio".to_owned(), None)
            .unwrap()
            .header
            .id;
        drop(session);
        let pid = project.as_str().to_owned();
        let (vault_id, vault_root) = base(&root);

        let wav_path = root.join("synthetic.wav");
        std::fs::write(
            &wav_path,
            CliSession::synthetic_wav(PcmFormat::mono_16k(), &[(300, false), (800, true)]),
        )
        .unwrap();
        let bad_path = root.join("not-audio.wav");
        std::fs::write(&bad_path, b"RIFF....WAVEjunk").unwrap();
        assert!(
            run_audio(AudioCmd::Import {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                label: "bad".to_owned(),
                file: bad_path,
                json: true,
            })
            .is_err()
        );
        for json in [true, false] {
            run_audio(AudioCmd::Import {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                label: format!("import {json}"),
                file: wav_path.clone(),
                json,
            })
            .expect("import");
            run_audio(AudioCmd::Capture {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                label: format!("capture {json}"),
                synthetic: "200:silence,600:tone".to_owned(),
                native: false,
                leave_open: false,
                json,
            })
            .expect("scripted capture");
        }
        assert!(
            run_audio(AudioCmd::Capture {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                label: "native".to_owned(),
                synthetic: "100:tone".to_owned(),
                native: true,
                leave_open: false,
                json: true,
            })
            .is_err(),
            "native capture is not admitted"
        );
        run_audio(AudioCmd::Capture {
            vault_id: vault_id.clone(),
            vault_root: vault_root.clone(),
            project_id: pid.clone(),
            label: "left open".to_owned(),
            synthetic: "400:tone".to_owned(),
            native: false,
            leave_open: true,
            json: true,
        })
        .expect("open capture");

        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let sources = s.audio_source_list(project.clone()).unwrap();
        assert_eq!(sources.len(), 4);
        let captures = s.audio_capture_list(project.clone()).unwrap();
        assert_eq!(captures.len(), 3);
        assert_eq!(captures[2].state, CaptureState::Interrupted);
        assert!(captures[2].source_id.is_none());
        let source_id = sources[0].header.id.as_str().to_owned();
        drop(s);

        for (route, fixture) in [
            ("cloud_asr", false),
            ("local_asr_pack", false),
            ("fixture_asr", false),
            ("segmentation_only", false),
            ("fixture_asr", true),
        ] {
            run_audio(AudioCmd::Transcribe {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                source_id: source_id.clone(),
                route: route.to_owned(),
                mode: "dictation".to_owned(),
                language: Some("en".to_owned()),
                diarize: false,
                fixture_engine: fixture,
                json: route == "cloud_asr",
            })
            .expect("every request returns a receipt");
        }
        for json in [true, false] {
            for cmd in [
                AudioCmd::Sources {
                    vault_id: vault_id.clone(),
                    vault_root: vault_root.clone(),
                    project_id: pid.clone(),
                    json,
                },
                AudioCmd::Routes {
                    vault_id: vault_id.clone(),
                    vault_root: vault_root.clone(),
                    fixture_engine: json,
                    json,
                },
                AudioCmd::Transcripts {
                    vault_id: vault_id.clone(),
                    vault_root: vault_root.clone(),
                    source_id: source_id.clone(),
                    json,
                },
            ] {
                run_audio(cmd).expect("read command");
            }
        }

        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let receipts = s
            .audio_receipt_list(OpaqueId::new(source_id.clone()))
            .unwrap();
        let reasons: Vec<_> = receipts.iter().map(|r| r.decision.reason).collect();
        assert_eq!(
            reasons,
            vec![
                Some(AudioRouteDenyReason::CloudRouteForbidden),
                Some(AudioRouteDenyReason::RouteUnavailable),
                Some(AudioRouteDenyReason::RouteUnavailable),
                None,
                None,
            ]
        );
        let revisions = s
            .audio_transcript_list(OpaqueId::new(source_id.clone()))
            .unwrap();
        assert_eq!(revisions.len(), 2);
        let latest = revisions[1].header.id.as_str().to_owned();
        drop(s);
        run_audio(AudioCmd::Correct {
            vault_id: vault_id.clone(),
            vault_root: vault_root.clone(),
            revision_id: latest.clone(),
            segment: 1,
            text: "synthetic tone".to_owned(),
            reason: "fixture label replaced".to_owned(),
            json: false,
        })
        .expect("correct");
        assert!(
            run_audio(AudioCmd::Correct {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                revision_id: latest.clone(),
                segment: 1,
                text: "again".to_owned(),
                reason: "stale".to_owned(),
                json: true,
            })
            .is_err(),
            "only the latest revision can be corrected"
        );
        run_audio(AudioCmd::Evidence {
            vault_id: vault_id.clone(),
            vault_root: vault_root.clone(),
            revision_id: latest.clone(),
            segment: 1,
            json: false,
        })
        .expect("evidence");

        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let revisions = s.audio_transcript_list(OpaqueId::new(source_id)).unwrap();
        assert_eq!(revisions.len(), 3);
        assert_eq!(revisions[2].segments[0].status, SegmentStatus::Corrected);
        assert_eq!(
            revisions[1].segments[0].status,
            SegmentStatus::Transcribed,
            "the parent revision is unchanged"
        );
    }
}

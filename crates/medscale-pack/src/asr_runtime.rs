//! Local speech recognition on candle Whisper (Spec 106, AudioFlow).
//!
//! Runs an admitted, signed Audio Pack (data-only safetensors, `config.json`,
//! `tokenizer.json`, provenance) through candle-transformers' Whisper model on
//! the CPU: no remote code, no network. The log-mel front end is candle's;
//! the mel filter bank is computed here (Slaney scale and norm, matching
//! OpenAI's `mel_filters.npz`). Decoding is greedy and bounded, without
//! timestamps. A transcript is a proposal for review, never clinical truth,
//! and medical accuracy is unmeasured until a qualified evaluation exists.

use std::fmt;
use std::path::Path;
use std::sync::Mutex;

use candle_core::{D, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self, Config, audio, model::Whisper};
use medscale_contracts::objects::DigestSha256;
use medscale_contracts::packs::{ModelSourceProvenance, PackArtifactKind, PackManifestV0};
use thiserror::Error;
use tokenizers::Tokenizer;

use crate::onnx_runtime::{OnnxRuntimeError, select_artifact};

/// Runtime id declared by Audio Packs.
pub const CANDLE_WHISPER_ASR_RUNTIME_ID: &str = "candle_whisper_asr_v1";
/// Whisper checkpoints admitted as Audio Pack sources, each at one pinned
/// revision (`hf_snapshot::build_asr_pack` refuses anything else).
pub const ASR_SOURCES: &[(&str, &str)] = &[
    (
        "openai/whisper-tiny.en",
        "87c7102498dcde7456f24cfd30239ca606ed9063",
    ),
    (
        "openai/whisper-tiny",
        "169d4a4341b33bc18d8881c4b69c2e104e1cc0af",
    ),
    (
        "openai/whisper-base.en",
        "911407f4214e0e1d82085af863093ec0b66f9cd6",
    ),
];
/// Longest audio one call transcribes (10 minutes at 16 kHz).
pub const MAX_ASR_SAMPLES: usize = 10 * 60 * whisper::SAMPLE_RATE;
/// Largest number of tokens decoded for one 30-second window.
const MAX_WINDOW_TOKENS: usize = 224;
/// Token allowance per second of audio in a window (speech runs at about
/// 3–4 tokens per second), plus a fixed margin. Bounds runaway decoding.
const TOKENS_PER_SECOND: usize = 8;
const TOKEN_MARGIN: usize = 16;
/// A token repeated this many times in a row ends the window (greedy
/// Whisper can loop on silence or unclear audio); the repeats are dropped.
const MAX_TOKEN_REPEAT: usize = 4;
// A full 30-second window may still use the whole window budget.
const _: () = assert!(TOKEN_MARGIN + TOKENS_PER_SECOND * 31 >= MAX_WINDOW_TOKENS);

#[derive(Debug, Error)]
pub enum AsrRuntimeError {
    #[error("pack runtime requirement does not admit the candle Whisper ASR runtime")]
    RuntimeNotAdmitted,
    #[error("artifact: {0}")]
    Artifact(#[from] OnnxRuntimeError),
    #[error("config.json is invalid: {0}")]
    InvalidConfig(String),
    #[error("model provenance metadata is invalid")]
    InvalidProvenance,
    #[error("tokenizer failed or lacks a Whisper control token: {0}")]
    Tokenizer(String),
    #[error("model preparation failed: {0}")]
    Prepare(String),
    #[error("model execution failed: {0}")]
    Execution(String),
    #[error("audio is empty or longer than the admitted bound")]
    InputBound,
}

fn prep(e: impl fmt::Display) -> AsrRuntimeError {
    AsrRuntimeError::Prepare(format!("{e}").chars().take(600).collect())
}

fn exec(e: impl fmt::Display) -> AsrRuntimeError {
    AsrRuntimeError::Execution(format!("{e}").chars().take(600).collect())
}

fn hz_to_mel(f: f64) -> f64 {
    // Slaney scale: linear below 1 kHz, logarithmic above.
    let f_sp = 200.0 / 3.0;
    let min_log_hz = 1000.0;
    let min_log_mel = min_log_hz / f_sp;
    let logstep = (6.4f64).ln() / 27.0;
    if f >= min_log_hz {
        min_log_mel + (f / min_log_hz).ln() / logstep
    } else {
        f / f_sp
    }
}

fn mel_to_hz(m: f64) -> f64 {
    let f_sp = 200.0 / 3.0;
    let min_log_hz = 1000.0;
    let min_log_mel = min_log_hz / f_sp;
    let logstep = (6.4f64).ln() / 27.0;
    if m >= min_log_mel {
        min_log_hz * (logstep * (m - min_log_mel)).exp()
    } else {
        f_sp * m
    }
}

/// Mel filter bank `[n_mels][1 + n_fft / 2]` for 16 kHz audio, equivalent to
/// `librosa.filters.mel(sr=16000, n_fft=400, n_mels=n_mels)` (Slaney scale
/// and area normalization), which is what Whisper was trained with.
#[must_use]
pub fn mel_filters(n_mels: usize) -> Vec<f32> {
    let sr = whisper::SAMPLE_RATE as f64;
    let n_bins = 1 + whisper::N_FFT / 2;
    let fft_freqs: Vec<f64> = (0..n_bins)
        .map(|i| i as f64 * (sr / 2.0) / (n_bins - 1) as f64)
        .collect();
    let (lo, hi) = (hz_to_mel(0.0), hz_to_mel(sr / 2.0));
    let mel_f: Vec<f64> = (0..n_mels + 2)
        .map(|i| mel_to_hz(lo + (hi - lo) * i as f64 / (n_mels + 1) as f64))
        .collect();
    let mut weights = vec![0f32; n_mels * n_bins];
    for m in 0..n_mels {
        let (left, center, right) = (mel_f[m], mel_f[m + 1], mel_f[m + 2]);
        let enorm = 2.0 / (right - left);
        for (k, &f) in fft_freqs.iter().enumerate() {
            let lower = (f - left) / (center - left);
            let upper = (right - f) / (right - center);
            let w = lower.min(upper).max(0.0) * enorm;
            weights[m * n_bins + k] = w as f32;
        }
    }
    weights
}

/// Converts little-endian 16-bit PCM (any channel count) to mono f32 in
/// [-1, 1]. Returns `None` for an unsupported layout.
#[must_use]
pub fn pcm16_to_mono_f32(pcm: &[u8], channels: u16) -> Option<Vec<f32>> {
    let channels = usize::from(channels);
    if channels == 0 || !pcm.len().is_multiple_of(2 * channels) {
        return None;
    }
    Some(
        pcm.chunks_exact(2 * channels)
            .map(|frame| {
                let sum: f32 = frame
                    .chunks_exact(2)
                    .map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0)
                    .sum();
                sum / channels as f32
            })
            .collect(),
    )
}

struct ControlTokens {
    sot: u32,
    transcribe: u32,
    no_timestamps: u32,
    eot: u32,
    /// Present for multilingual checkpoints only.
    english: Option<u32>,
    suppress: Vec<u32>,
}

/// A prepared Whisper model, ready to transcribe 16 kHz mono audio.
pub struct PreparedWhisperAsr {
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    config: Config,
    filters: Vec<f32>,
    tokenizer: Tokenizer,
    tokens: ControlTokens,
    model: Mutex<Whisper>,
    device: Device,
}

impl fmt::Debug for PreparedWhisperAsr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedWhisperAsr")
            .field("repository", &self.provenance.repository)
            .field("revision", &self.provenance.revision)
            .finish_non_exhaustive()
    }
}

fn token(tokenizer: &Tokenizer, name: &str) -> Result<u32, AsrRuntimeError> {
    tokenizer
        .token_to_id(name)
        .ok_or_else(|| AsrRuntimeError::Tokenizer(name.to_owned()))
}

/// Builds a Whisper runtime from verified bytes (CPU).
pub fn prepare_whisper_from_bytes(
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    config_bytes: &[u8],
    weights: Vec<u8>,
    tokenizer_bytes: &[u8],
) -> Result<PreparedWhisperAsr, AsrRuntimeError> {
    let config: Config = serde_json::from_slice(config_bytes)
        .map_err(|e| AsrRuntimeError::InvalidConfig(e.to_string()))?;
    if config.num_mel_bins != 80 && config.num_mel_bins != 128 {
        return Err(AsrRuntimeError::InvalidConfig(format!(
            "num_mel_bins {} is not 80 or 128",
            config.num_mel_bins
        )));
    }
    let tokenizer = Tokenizer::from_bytes(tokenizer_bytes)
        .map_err(|e| AsrRuntimeError::Tokenizer(e.to_string()))?;
    let english = tokenizer.token_to_id("<|en|>");
    let multilingual = config.vocab_size >= 51865;
    let mut suppress = config.suppress_tokens.clone();
    // Without timestamps, timestamp tokens are never valid output.
    if let Some(first_ts) = tokenizer.token_to_id("<|0.00|>") {
        suppress.extend(first_ts..u32::try_from(config.vocab_size).unwrap_or(u32::MAX));
    }
    let tokens = ControlTokens {
        sot: token(&tokenizer, whisper::SOT_TOKEN)?,
        transcribe: token(&tokenizer, whisper::TRANSCRIBE_TOKEN)?,
        no_timestamps: token(&tokenizer, whisper::NO_TIMESTAMPS_TOKEN)?,
        eot: token(&tokenizer, whisper::EOT_TOKEN)?,
        english: if multilingual { english } else { None },
        suppress,
    };
    let device = Device::Cpu;
    let vb =
        VarBuilder::from_buffered_safetensors(weights, whisper::DTYPE, &device).map_err(prep)?;
    let model = Whisper::load(&vb, config.clone()).map_err(prep)?;
    Ok(PreparedWhisperAsr {
        content_digest,
        provenance,
        filters: mel_filters(config.num_mel_bins),
        config,
        tokenizer,
        tokens,
        model: Mutex::new(model),
        device,
    })
}

/// Prepares an admitted Audio Pack (CPU). Artifact digests are verified by
/// `select_artifact` against the signed manifest.
pub fn prepare_whisper_asr(
    pack_dir: &Path,
    pack: &PackManifestV0,
) -> Result<PreparedWhisperAsr, AsrRuntimeError> {
    if !pack
        .runtime_requirements
        .split(';')
        .any(|r| r == CANDLE_WHISPER_ASR_RUNTIME_ID)
    {
        return Err(AsrRuntimeError::RuntimeNotAdmitted);
    }
    let weights = select_artifact(pack_dir, pack, PackArtifactKind::SafetensorsModel, None)?;
    let config = select_artifact(
        pack_dir,
        pack,
        PackArtifactKind::ModelMetadata,
        Some("config.json"),
    )?;
    let tokenizer = select_artifact(
        pack_dir,
        pack,
        PackArtifactKind::TokenizerMeta,
        Some("tokenizer.json"),
    )?;
    let provenance_bytes = select_artifact(
        pack_dir,
        pack,
        PackArtifactKind::ModelMetadata,
        Some("model.meta.json"),
    )?;
    let provenance: ModelSourceProvenance = serde_json::from_slice(&provenance_bytes)
        .map_err(|_| AsrRuntimeError::InvalidProvenance)?;
    if provenance.runtime_family != CANDLE_WHISPER_ASR_RUNTIME_ID {
        return Err(AsrRuntimeError::RuntimeNotAdmitted);
    }
    prepare_whisper_from_bytes(
        pack.content_digest.clone(),
        provenance,
        &config,
        weights,
        &tokenizer,
    )
}

impl PreparedWhisperAsr {
    #[must_use]
    pub fn provenance(&self) -> &ModelSourceProvenance {
        &self.provenance
    }

    #[must_use]
    pub fn content_digest(&self) -> &DigestSha256 {
        &self.content_digest
    }

    /// Transcribes 16 kHz mono samples in 30-second windows (greedy, no
    /// timestamps). Multilingual checkpoints are prompted for English.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String, AsrRuntimeError> {
        if samples.is_empty() || samples.len() > MAX_ASR_SAMPLES {
            return Err(AsrRuntimeError::InputBound);
        }
        let n_mels = self.config.num_mel_bins;
        let mel = audio::pcm_to_mel(&self.config, samples, &self.filters);
        let n_frames = mel.len() / n_mels;
        let mel = Tensor::from_vec(mel, (1, n_mels, n_frames), &self.device).map_err(exec)?;
        // Frames that carry audio (the front end pads with silence).
        let content_frames = samples.len().div_ceil(whisper::HOP_LENGTH);
        let mut model = self.model.lock().map_err(|_| exec("model lock poisoned"))?;
        let mut text = String::new();
        let mut seek = 0;
        while seek < content_frames {
            let size = whisper::N_FRAMES.min(n_frames - seek);
            let window = mel.narrow(2, seek, size).map_err(exec)?;
            let window = if size < whisper::N_FRAMES {
                window
                    .pad_with_zeros(2, 0, whisper::N_FRAMES - size)
                    .map_err(exec)?
            } else {
                window
            };
            let seconds = (content_frames - seek).min(whisper::N_FRAMES) * whisper::HOP_LENGTH
                / whisper::SAMPLE_RATE;
            let budget = TOKEN_MARGIN + TOKENS_PER_SECOND * (seconds + 1);
            let piece = self.decode_window(&mut model, &window, budget)?;
            if !piece.is_empty() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(piece.trim());
            }
            seek += whisper::N_FRAMES;
        }
        Ok(text)
    }

    fn decode_window(
        &self,
        model: &mut Whisper,
        mel: &Tensor,
        audio_budget: usize,
    ) -> Result<String, AsrRuntimeError> {
        let features = model.encoder.forward(mel, true).map_err(exec)?;
        let mut tokens = vec![self.tokens.sot];
        if let Some(en) = self.tokens.english {
            tokens.push(en);
        }
        tokens.push(self.tokens.transcribe);
        tokens.push(self.tokens.no_timestamps);
        let prompt_len = tokens.len();
        let vocab = self.config.vocab_size;
        let mut mask = vec![0f32; vocab];
        for &t in &self.tokens.suppress {
            if let Some(m) = mask.get_mut(t as usize) {
                *m = f32::NEG_INFINITY;
            }
        }
        let mask = Tensor::new(mask.as_slice(), &self.device).map_err(exec)?;
        let budget = MAX_WINDOW_TOKENS
            .min(self.config.max_target_positions / 2)
            .min(audio_budget);
        for i in 0..budget {
            let input = Tensor::new(tokens.as_slice(), &self.device)
                .and_then(|t| t.unsqueeze(0))
                .map_err(exec)?;
            let ys = model
                .decoder
                .forward(&input, &features, i == 0)
                .map_err(exec)?;
            let (_, seq_len, _) = ys.dims3().map_err(exec)?;
            let logits = model
                .decoder
                .final_linear(&ys.i((..1, seq_len - 1..)).map_err(exec)?)
                .and_then(|l| l.i(0))
                .and_then(|l| l.i(0))
                .map_err(exec)?;
            let logits = logits.broadcast_add(&mask).map_err(exec)?;
            let next = logits
                .argmax(D::Minus1)
                .and_then(|t| t.to_scalar::<u32>())
                .map_err(exec)?;
            if next == self.tokens.eot {
                break;
            }
            tokens.push(next);
            let tail = &tokens[prompt_len..];
            if tail.len() >= MAX_TOKEN_REPEAT
                && tail[tail.len() - MAX_TOKEN_REPEAT..]
                    .iter()
                    .all(|&t| t == next)
            {
                // Keep one occurrence; drop the loop.
                tokens.truncate(tokens.len() - (MAX_TOKEN_REPEAT - 1));
                break;
            }
        }
        model.decoder.reset_kv_cache();
        self.tokenizer
            .decode(&tokens[prompt_len..], true)
            .map_err(|e| AsrRuntimeError::Tokenizer(e.to_string()))
    }
}

/// A tiny random-weight Whisper (synthetic fixtures only) for this crate's
/// and downstream tests. It exercises the pipeline; it recognizes nothing.
#[cfg(any(test, feature = "synthetic-fixtures"))]
#[doc(hidden)]
pub mod fixtures {
    use candle_core::{DType, Device};
    use candle_nn::{VarBuilder, VarMap};
    use candle_transformers::models::whisper::{Config, model::Whisper};
    use serde_json::{Value, json};

    const VOCAB: &[&str] = &[
        "<|endoftext|>",
        "<|startoftranscript|>",
        "<|en|>",
        "<|transcribe|>",
        "<|notimestamps|>",
        "patient",
        "has",
        "fever",
        "and",
        "cough",
        "today",
        ".",
    ];

    pub fn tiny_config() -> Value {
        json!({
            "num_mel_bins": 80, "max_source_positions": 1500, "d_model": 8,
            "encoder_attention_heads": 2, "encoder_layers": 1, "vocab_size": VOCAB.len(),
            "max_target_positions": 16, "decoder_attention_heads": 2, "decoder_layers": 1,
            "suppress_tokens": []
        })
    }

    pub fn tiny_tokenizer() -> String {
        let vocab: serde_json::Map<String, Value> = VOCAB
            .iter()
            .enumerate()
            .map(|(i, t)| ((*t).to_owned(), json!(i)))
            .collect();
        let added: Vec<Value> = VOCAB[..5]
            .iter()
            .enumerate()
            .map(|(i, t)| json!({"id": i, "content": t, "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true}))
            .collect();
        json!({
            "version": "1.0", "truncation": null, "padding": null,
            "added_tokens": added,
            "normalizer": null,
            "pre_tokenizer": {"type": "WhitespaceSplit"},
            "post_processor": null,
            "decoder": {"type": "WordPiece", "prefix": "##", "cleanup": false},
            "model": {"type": "WordLevel", "vocab": vocab, "unk_token": "<|endoftext|>"}
        })
        .to_string()
    }

    /// Writes a signed synthetic Audio Pack into `out` and returns its id.
    /// `build_asr_pack` admits only pinned sources, so the metadata names the
    /// pinned whisper-tiny.en revision while every sibling digest is the
    /// synthetic file's own.
    pub fn synthetic_audio_pack(out: &std::path::Path) -> String {
        use sha1::Sha1;
        use sha2::{Digest, Sha256};
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        let blob = |b: &[u8]| {
            let mut h = Sha1::new();
            h.update(format!("blob {} ", b.len()).as_bytes());
            h.update(b);
            hex(&h.finalize())
        };
        let (repo, sha) = super::ASR_SOURCES[0];
        let config = serde_json::to_vec(&tiny_config()).unwrap();
        let tokenizer = tiny_tokenizer().into_bytes();
        let weights = tiny_weights();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let snapshot = std::env::temp_dir().join(format!(
            "medscale-106-asr-snapshot-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&snapshot).unwrap();
        std::fs::write(snapshot.join("config.json"), &config).unwrap();
        std::fs::write(snapshot.join("tokenizer.json"), &tokenizer).unwrap();
        std::fs::write(snapshot.join("model.safetensors"), &weights).unwrap();
        let meta: crate::HfRepoMetadata = serde_json::from_value(json!({
            "id": repo, "sha": sha, "gated": false, "private": false,
            "siblings": [
                {"rfilename": "config.json", "size": config.len(), "blobId": blob(&config)},
                {"rfilename": "model.safetensors", "size": weights.len(),
                 "lfs": {"sha256": hex(&Sha256::digest(&weights)), "size": weights.len()}},
                {"rfilename": "tokenizer.json", "size": tokenizer.len(), "blobId": blob(&tokenizer)}
            ]
        }))
        .unwrap();
        let pack_id = crate::build_asr_pack(&meta, &snapshot, out).unwrap();
        let _ = std::fs::remove_dir_all(snapshot);
        pack_id
    }

    pub fn tiny_weights() -> Vec<u8> {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
        let cfg: Config = serde_json::from_value(tiny_config()).unwrap();
        Whisper::load(&vb, cfg).unwrap();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "medscale-106-whisper-{}-{}.safetensors",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        varmap.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> PreparedWhisperAsr {
        let provenance = ModelSourceProvenance {
            source_kind: "synthetic_fixture".into(),
            repository: "medscale://synthetic/106-tiny-whisper".into(),
            revision: "v0".into(),
            original_file: "model.safetensors".into(),
            source_sha256: "0".repeat(64),
            transform: "none".into(),
            fixed_sequence_length: 16,
            task: "automatic-speech-recognition".into(),
            export_format: "safetensors".into(),
            runtime_family: CANDLE_WHISPER_ASR_RUNTIME_ID.into(),
            license_id: "Apache-2.0".into(),
            upstream_rights_uri: "medscale://synthetic".into(),
        };
        prepare_whisper_from_bytes(
            DigestSha256::of(b"tiny"),
            provenance,
            fixtures::tiny_config().to_string().as_bytes(),
            fixtures::tiny_weights(),
            fixtures::tiny_tokenizer().as_bytes(),
        )
        .expect("tiny whisper prepares")
    }

    #[test]
    fn tiny_whisper_transcribes_deterministically_within_bounds() {
        let asr = tiny();
        // 1.5 s of a 440 Hz tone: not speech; the pipeline must still run.
        let tone: Vec<f32> = (0..24_000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 16_000.0).sin() * 0.3)
            .collect();
        let a = asr.transcribe(&tone).expect("transcribes");
        let b = asr.transcribe(&tone).expect("transcribes again");
        assert_eq!(
            a, b,
            "greedy decoding is deterministic and the KV cache resets"
        );
        assert!(matches!(
            asr.transcribe(&[]),
            Err(AsrRuntimeError::InputBound)
        ));
        assert!(matches!(
            asr.transcribe(&vec![0.0; MAX_ASR_SAMPLES + 1]),
            Err(AsrRuntimeError::InputBound)
        ));
        // Longer than one window: two windows decode.
        let long = vec![0.0f32; 31 * 16_000];
        asr.transcribe(&long).expect("two windows");
    }

    #[test]
    fn synthetic_audio_pack_admits_and_prepares() {
        let out =
            std::env::temp_dir().join(format!("medscale-106-unit-pack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let pack_id = fixtures::synthetic_audio_pack(&out);
        let manifest = crate::admit_pack_dir(&out).expect("signed Audio Pack admits");
        assert_eq!(manifest.pack_id.as_str(), pack_id);
        assert!(
            manifest
                .runtime_requirements
                .starts_with(CANDLE_WHISPER_ASR_RUNTIME_ID)
        );
        let asr = prepare_whisper_asr(&out, &manifest).expect("prepares from the Pack");
        assert_eq!(
            asr.provenance().runtime_family,
            CANDLE_WHISPER_ASR_RUNTIME_ID
        );
        asr.transcribe(&[0.0; 16_000]).expect("transcribes");
        let _ = std::fs::remove_dir_all(out);
    }

    #[test]
    fn missing_control_token_is_refused() {
        let tok = fixtures::tiny_tokenizer().replace("<|notimestamps|>", "<|other|>");
        let err = prepare_whisper_from_bytes(
            DigestSha256::of(b"t"),
            tiny().provenance().clone(),
            fixtures::tiny_config().to_string().as_bytes(),
            fixtures::tiny_weights(),
            tok.as_bytes(),
        )
        .unwrap_err();
        assert!(matches!(err, AsrRuntimeError::Tokenizer(_)), "{err}");
    }

    #[test]
    fn mel_filters_match_slaney_shape() {
        let f = mel_filters(80);
        assert_eq!(f.len(), 80 * 201);
        assert!(f.iter().all(|w| *w >= 0.0 && w.is_finite()));
        // Every filter has support, and filters are ordered by frequency.
        let peak = |m: usize| {
            (0..201)
                .max_by(|&a, &b| f[m * 201 + a].total_cmp(&f[m * 201 + b]))
                .unwrap()
        };
        for m in 0..80 {
            assert!(
                f[m * 201..(m + 1) * 201].iter().any(|w| *w > 0.0),
                "filter {m}"
            );
        }
        assert!(peak(0) < peak(40) && peak(40) < peak(79));
        // The lowest filter sits in the linear region (bins are 40 Hz wide).
        assert!(peak(0) <= 2);
        assert_eq!(mel_filters(128).len(), 128 * 201);
    }

    #[test]
    fn mel_scale_round_trips() {
        for hz in [0.0, 200.0, 999.0, 1000.0, 4000.0, 8000.0] {
            assert!((mel_to_hz(hz_to_mel(hz)) - hz).abs() < 1e-6, "{hz}");
        }
    }

    #[test]
    fn pcm16_converts_and_mixes_channels() {
        let mono = [0i16, 16384, -32768]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect::<Vec<_>>();
        assert_eq!(pcm16_to_mono_f32(&mono, 1).unwrap(), vec![0.0, 0.5, -1.0]);
        let stereo = [16384i16, 0]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect::<Vec<_>>();
        assert_eq!(pcm16_to_mono_f32(&stereo, 2).unwrap(), vec![0.25]);
        assert!(pcm16_to_mono_f32(&[0u8; 3], 1).is_none());
        assert!(pcm16_to_mono_f32(&[0u8; 4], 0).is_none());
    }
}

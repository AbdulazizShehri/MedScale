//! Bounded local text generation on candle (Spec 105, phase A).
//!
//! Runs causal language models from data-only safetensors through the same
//! governed lifecycle as the token classifiers: admitted signed Pack, digest
//! verification at prepare, a fixed `model_type` allowlist, no remote code,
//! no network. Decoding is greedy (deterministic), bounded by a token budget,
//! and stops at the model's end-of-sequence ids. Output is a proposal for
//! evaluation only, never a `ClinicalAssertion`.
//!
//! Every OpenMed v3.0.0 generative or vision row is 1.31–20.6 GB, over the
//! unchanged 1 GiB Pack artifact bound, so this phase is qualified with
//! synthetic models only; the catalog rows stay blocked until the founder
//! decides on the bound.

use std::fmt;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use candle_core::{D, DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::{llama, mistral, qwen2};
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::packs::{ModelSourceProvenance, PackArtifactKind, PackManifestV0};
use serde_json::{Value, json};
use thiserror::Error;
use tokenizers::Tokenizer;

use crate::RuntimeOutput;
use crate::onnx_runtime::{OnnxRuntimeError, select_artifact};

/// Runtime id declared by generation Packs.
pub const CANDLE_TEXT_GENERATION_RUNTIME_ID: &str = "candle_text_generation_v1";
/// Architectures this runtime builds (`config.json` `model_type`).
pub const GENERATION_ARCHITECTURES: &[&str] = &["qwen2", "mistral", "llama"];
const MAX_PROMPT_BYTES: usize = 16_384;
/// Largest number of tokens one call may generate.
pub const MAX_NEW_TOKENS: usize = 256;
/// Upper bound on prompt plus generated tokens, whatever the model allows.
const MAX_CONTEXT: usize = 4_096;

#[derive(Debug, Error)]
pub enum GenerationRuntimeError {
    #[error("pack runtime requirement does not admit the candle text-generation runtime")]
    RuntimeNotAdmitted,
    #[error("artifact: {0}")]
    Artifact(#[from] OnnxRuntimeError),
    #[error("config.json is invalid: {0}")]
    InvalidConfig(String),
    #[error("model_type {0:?} is not supported by this runtime")]
    UnsupportedArchitecture(String),
    #[error("model provenance metadata is invalid")]
    InvalidProvenance,
    #[error("tokenizer failed")]
    Tokenizer,
    #[error("model preparation failed: {0}")]
    Prepare(String),
    #[error("model execution failed")]
    ModelExecution,
    #[error("prompt is empty or outside the admitted bound")]
    InputBound,
    #[error("max_new_tokens must be between 1 and {MAX_NEW_TOKENS}")]
    InvalidBudget,
    #[error("prompt and budget exceed the context limit")]
    TooLong,
    #[error("generation cancelled by the caller")]
    Cancelled,
}

fn prep(e: impl fmt::Display) -> GenerationRuntimeError {
    GenerationRuntimeError::Prepare(format!("{e}").chars().take(600).collect())
}

enum Decoder {
    Qwen2(Box<qwen2::ModelForCausalLM>),
    Mistral(Box<mistral::Model>),
    /// candle's Llama keeps its KV cache outside the model; `fresh` is the
    /// empty cache each call starts from.
    Llama {
        model: Box<llama::Llama>,
        cache: Box<llama::Cache>,
        fresh: Box<llama::Cache>,
    },
}

impl Decoder {
    /// Logits of the last position, as a vector over the vocabulary.
    fn forward(&mut self, ids: &Tensor, offset: usize) -> candle_core::Result<Tensor> {
        let logits = match self {
            Self::Qwen2(m) => m.forward(ids, offset),
            Self::Mistral(m) => m.forward(ids, offset),
            Self::Llama { model, cache, .. } => model.forward(ids, offset, cache),
        }?;
        logits.flatten_all()
    }

    fn clear(&mut self) {
        match self {
            Self::Qwen2(m) => m.clear_kv_cache(),
            Self::Mistral(m) => m.clear_kv_cache(),
            Self::Llama { cache, fresh, .. } => cache.clone_from(fresh),
        }
    }
}

/// candle's `Llama::load` panics (an `unwrap`) when a decoder layer tensor
/// is missing or misshapen, so every layer tensor is checked here first and a
/// bad checkpoint is refused as an error instead.
fn check_llama_layers(vb: &VarBuilder, cfg: &llama::Config) -> Result<(), GenerationRuntimeError> {
    let hidden = cfg.hidden_size;
    if cfg.num_attention_heads == 0 || !hidden.is_multiple_of(cfg.num_attention_heads) {
        return Err(GenerationRuntimeError::InvalidConfig(
            "hidden_size must be a multiple of num_attention_heads".into(),
        ));
    }
    let head_dim = hidden / cfg.num_attention_heads;
    let kv = head_dim * cfg.num_key_value_heads;
    let inter = cfg.intermediate_size;
    for i in 0..cfg.num_hidden_layers {
        let layer = vb.pp(format!("model.layers.{i}"));
        let shapes: [(&str, &[usize]); 9] = [
            ("self_attn.q_proj.weight", &[hidden, hidden]),
            ("self_attn.k_proj.weight", &[kv, hidden]),
            ("self_attn.v_proj.weight", &[kv, hidden]),
            ("self_attn.o_proj.weight", &[hidden, hidden]),
            ("mlp.gate_proj.weight", &[inter, hidden]),
            ("mlp.up_proj.weight", &[inter, hidden]),
            ("mlp.down_proj.weight", &[hidden, inter]),
            ("input_layernorm.weight", &[hidden]),
            ("post_attention_layernorm.weight", &[hidden]),
        ];
        for (name, shape) in shapes {
            layer.get(shape, name).map_err(prep)?;
        }
    }
    Ok(())
}

/// A prepared generator. One call runs at a time (the KV cache is reset at
/// the start of each call); no prompt is retained between calls.
pub struct PreparedTextGenerator {
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    model_type: String,
    tokenizer: Tokenizer,
    decoder: Mutex<Decoder>,
    eos: Vec<u32>,
    context: usize,
    device: Device,
}

impl fmt::Debug for PreparedTextGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedTextGenerator")
            .field("content_digest", &self.content_digest)
            .field("model_type", &self.model_type)
            .field("context", &self.context)
            .finish_non_exhaustive()
    }
}

/// Why generation stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndOfSequence,
    TokenBudget,
}

fn eos_ids(config: &Value) -> Vec<u32> {
    let ids = match &config["eos_token_id"] {
        Value::Number(n) => vec![n.as_u64()],
        Value::Array(a) => a.iter().map(Value::as_u64).collect(),
        _ => Vec::new(),
    };
    ids.into_iter()
        .flatten()
        .filter_map(|v| u32::try_from(v).ok())
        .collect()
}

/// Prepares from verified bytes on `device`.
pub(crate) fn prepare_generator_from_bytes(
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    config_bytes: &[u8],
    weights: Vec<u8>,
    tokenizer_bytes: &[u8],
    device: Device,
) -> Result<PreparedTextGenerator, GenerationRuntimeError> {
    let config: Value = serde_json::from_slice(config_bytes)
        .map_err(|e| GenerationRuntimeError::InvalidConfig(e.to_string()))?;
    let model_type = config["model_type"]
        .as_str()
        .ok_or_else(|| GenerationRuntimeError::InvalidConfig("model_type is missing".into()))?
        .to_ascii_lowercase();
    if !GENERATION_ARCHITECTURES.contains(&model_type.as_str()) {
        return Err(GenerationRuntimeError::UnsupportedArchitecture(model_type));
    }
    let max_positions = config["max_position_embeddings"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| {
            GenerationRuntimeError::InvalidConfig("max_position_embeddings is missing".into())
        })?;
    let eos = eos_ids(&config);
    let mut tokenizer =
        Tokenizer::from_bytes(tokenizer_bytes).map_err(|_| GenerationRuntimeError::Tokenizer)?;
    tokenizer.with_padding(None);
    tokenizer
        .with_truncation(None)
        .map_err(|_| GenerationRuntimeError::Tokenizer)?;
    // Architecture and config are checked before any weight is parsed.
    let parse = |e: serde_json::Error| GenerationRuntimeError::InvalidConfig(e.to_string());
    let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device).map_err(prep)?;
    let decoder = match model_type.as_str() {
        "qwen2" => {
            let cfg: qwen2::Config = serde_json::from_value(config.clone()).map_err(parse)?;
            Decoder::Qwen2(Box::new(
                qwen2::ModelForCausalLM::new(&cfg, vb).map_err(prep)?,
            ))
        }
        "llama" => {
            let raw: llama::LlamaConfig = serde_json::from_value(config.clone()).map_err(parse)?;
            let cfg = raw.into_config(false);
            check_llama_layers(&vb, &cfg)?;
            let model = llama::Llama::load(vb, &cfg).map_err(prep)?;
            // The cache precomputes rotary tables for every position; only
            // the runtime's context bound is ever used.
            let mut cache_cfg = cfg.clone();
            cache_cfg.max_position_embeddings = cfg.max_position_embeddings.min(MAX_CONTEXT);
            let fresh = llama::Cache::new(true, DType::F32, &cache_cfg, &device).map_err(prep)?;
            Decoder::Llama {
                model: Box::new(model),
                cache: Box::new(fresh.clone()),
                fresh: Box::new(fresh),
            }
        }
        _ => {
            let cfg: mistral::Config = serde_json::from_value(config.clone()).map_err(parse)?;
            Decoder::Mistral(Box::new(mistral::Model::new(&cfg, vb).map_err(prep)?))
        }
    };
    Ok(PreparedTextGenerator {
        content_digest,
        provenance,
        model_type,
        tokenizer,
        decoder: Mutex::new(decoder),
        eos,
        context: max_positions.min(MAX_CONTEXT),
        device,
    })
}

/// Prepares an admitted generation Pack (CPU).
pub fn prepare_text_generator(
    pack_dir: &Path,
    pack: &PackManifestV0,
) -> Result<PreparedTextGenerator, GenerationRuntimeError> {
    if !pack
        .runtime_requirements
        .split(';')
        .any(|r| r == CANDLE_TEXT_GENERATION_RUNTIME_ID)
    {
        return Err(GenerationRuntimeError::RuntimeNotAdmitted);
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
        .map_err(|_| GenerationRuntimeError::InvalidProvenance)?;
    if provenance.runtime_family != CANDLE_TEXT_GENERATION_RUNTIME_ID {
        return Err(GenerationRuntimeError::RuntimeNotAdmitted);
    }
    prepare_generator_from_bytes(
        pack.content_digest.clone(),
        provenance,
        &config,
        weights,
        &tokenizer,
        Device::Cpu,
    )
}

impl PreparedTextGenerator {
    #[must_use]
    pub fn provenance(&self) -> &ModelSourceProvenance {
        &self.provenance
    }

    #[must_use]
    pub fn context_limit(&self) -> usize {
        self.context
    }

    /// Greedy generation of at most `max_new_tokens` tokens after `prompt`.
    pub fn generate(
        &self,
        pack_id: &OpaqueId,
        prompt: &str,
        max_new_tokens: usize,
    ) -> Result<RuntimeOutput, GenerationRuntimeError> {
        self.generate_cancellable(pack_id, prompt, max_new_tokens, &AtomicBool::new(false))
    }

    /// [`Self::generate`] that stops before the next token once `cancel` is
    /// set, returning [`GenerationRuntimeError::Cancelled`] (no partial
    /// output; the KV cache is reset either way).
    pub fn generate_cancellable(
        &self,
        pack_id: &OpaqueId,
        prompt: &str,
        max_new_tokens: usize,
        cancel: &AtomicBool,
    ) -> Result<RuntimeOutput, GenerationRuntimeError> {
        if prompt.trim().is_empty() || prompt.len() > MAX_PROMPT_BYTES {
            return Err(GenerationRuntimeError::InputBound);
        }
        if !(1..=MAX_NEW_TOKENS).contains(&max_new_tokens) {
            return Err(GenerationRuntimeError::InvalidBudget);
        }
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|_| GenerationRuntimeError::Tokenizer)?;
        let prompt_ids = encoding.get_ids().to_vec();
        if prompt_ids.is_empty() || prompt_ids.len() + max_new_tokens > self.context {
            return Err(GenerationRuntimeError::TooLong);
        }
        let mut decoder = self
            .decoder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        decoder.clear();
        let decoded = self.decode_greedy(&mut decoder, &prompt_ids, max_new_tokens, cancel);
        decoder.clear();
        drop(decoder);
        let (generated, stop) = decoded?;
        let text = self
            .tokenizer
            .decode(&generated, true)
            .map_err(|_| GenerationRuntimeError::Tokenizer)?;
        Ok(RuntimeOutput {
            proposal_payload: json!({
                "kind": "candle_text_generation_v1",
                "runtime": CANDLE_TEXT_GENERATION_RUNTIME_ID,
                "model_type": self.model_type,
                "content_digest": self.content_digest.to_hex(),
                "source_kind": self.provenance.source_kind,
                "source_repository": self.provenance.repository,
                "source_revision": self.provenance.revision,
                "decoding": "greedy",
                "prompt_tokens": prompt_ids.len(),
                "generated_tokens": generated.len(),
                "max_new_tokens": max_new_tokens,
                "stop_reason": stop,
                "generated_text": text,
                "note": "Proposal/evaluation evidence only; never ClinicalAssertion",
            }),
            evidence_only: true,
            pack_id: pack_id.clone(),
        })
    }
}

impl PreparedTextGenerator {
    fn decode_greedy(
        &self,
        decoder: &mut Decoder,
        prompt_ids: &[u32],
        max_new_tokens: usize,
        cancel: &AtomicBool,
    ) -> Result<(Vec<u32>, StopReason), GenerationRuntimeError> {
        let exec = |_| GenerationRuntimeError::ModelExecution;
        let mut generated: Vec<u32> = Vec::with_capacity(max_new_tokens);
        let mut input = prompt_ids.to_vec();
        let mut offset = 0;
        for _ in 0..max_new_tokens {
            if cancel.load(Ordering::Relaxed) {
                return Err(GenerationRuntimeError::Cancelled);
            }
            let ids = Tensor::new(input.as_slice(), &self.device)
                .and_then(|t| t.unsqueeze(0))
                .map_err(exec)?;
            let next = decoder
                .forward(&ids, offset)
                .and_then(|l| l.argmax(D::Minus1))
                .and_then(|t| t.to_scalar::<u32>())
                .map_err(exec)?;
            offset += input.len();
            if self.eos.contains(&next) {
                return Ok((generated, StopReason::EndOfSequence));
            }
            generated.push(next);
            input = vec![next];
        }
        Ok((generated, StopReason::TokenBudget))
    }
}

/// Deterministic synthetic generation fixtures (random weights, synthetic
/// only) for this crate's and downstream tests. Never a real model.
#[cfg(any(test, feature = "synthetic-fixtures"))]
#[doc(hidden)]
pub mod fixtures {
    use candle_core::{DType, Device};
    use candle_nn::{VarBuilder, VarMap};
    use candle_transformers::models::{llama, mistral, qwen2};
    use serde_json::{Value, json};

    pub fn tiny_config(model_type: &str) -> Value {
        json!({
            "model_type": model_type,
            "vocab_size": 12, "hidden_size": 16, "intermediate_size": 32,
            "num_hidden_layers": 1, "num_attention_heads": 2, "num_key_value_heads": 1,
            "max_position_embeddings": 64, "sliding_window": 64, "max_window_layers": 1,
            "tie_word_embeddings": false, "rope_theta": 10000.0, "rms_norm_eps": 1e-6,
            "use_sliding_window": false, "hidden_act": "silu", "eos_token_id": 2
        })
    }

    pub fn tiny_tokenizer() -> String {
        let vocab = [
            "<pad>", "<s>", "</s>", "<unk>", "patient", "has", "fever", "and", "cough", "today",
            ".", "no",
        ];
        let vocab: serde_json::Map<String, Value> = vocab
            .iter()
            .enumerate()
            .map(|(i, t)| ((*t).to_owned(), json!(i)))
            .collect();
        json!({
            "version": "1.0", "truncation": null, "padding": null,
            "added_tokens": [
                {"id": 0, "content": "<pad>", "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true},
                {"id": 1, "content": "<s>", "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true},
                {"id": 2, "content": "</s>", "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true}
            ],
            "normalizer": null,
            "pre_tokenizer": {"type": "WhitespaceSplit"},
            "post_processor": null,
            "decoder": {"type": "WordPiece", "prefix": "##", "cleanup": false},
            "model": {"type": "WordLevel", "vocab": vocab, "unk_token": "<unk>"}
        })
        .to_string()
    }

    pub fn tiny_weights(model_type: &str) -> Vec<u8> {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
        let config = tiny_config(model_type);
        if model_type == "qwen2" {
            let cfg: qwen2::Config = serde_json::from_value(config).unwrap();
            qwen2::ModelForCausalLM::new(&cfg, vb).unwrap();
        } else if model_type == "llama" {
            let cfg: llama::LlamaConfig = serde_json::from_value(config).unwrap();
            llama::Llama::load(vb, &cfg.into_config(false)).unwrap();
        } else {
            let cfg: mistral::Config = serde_json::from_value(config).unwrap();
            mistral::Model::new(&cfg, vb).unwrap();
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "medscale-105-gen-{model_type}-{}-{}.safetensors",
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
    use super::fixtures::{tiny_config, tiny_tokenizer, tiny_weights};
    use super::*;

    fn provenance() -> ModelSourceProvenance {
        serde_json::from_value(json!({
            "source_kind": "synthetic_fixture", "repository": "synthetic/tiny-generator",
            "revision": "0000000000000000000000000000000000000000",
            "original_file": "model.safetensors", "source_sha256": "0".repeat(64),
            "transform": "none", "fixed_sequence_length": 64, "task": "text-generation",
            "export_format": "safetensors", "runtime_family": CANDLE_TEXT_GENERATION_RUNTIME_ID,
            "license_id": "Apache-2.0", "upstream_rights_uri": "https://example.invalid/synthetic"
        }))
        .unwrap()
    }

    fn generator(model_type: &str) -> PreparedTextGenerator {
        prepare_generator_from_bytes(
            DigestSha256::of(model_type.as_bytes()),
            provenance(),
            tiny_config(model_type).to_string().as_bytes(),
            tiny_weights(model_type),
            tiny_tokenizer().as_bytes(),
            Device::Cpu,
        )
        .unwrap()
    }

    #[test]
    fn generation_is_greedy_bounded_and_deterministic() {
        for model_type in GENERATION_ARCHITECTURES {
            let g = generator(model_type);
            let pid = OpaqueId::new("p");
            let out = g.generate(&pid, "patient has fever", 8).unwrap();
            let p = &out.proposal_payload;
            assert!(out.evidence_only);
            assert_eq!(p["kind"], "candle_text_generation_v1");
            assert_eq!(p["model_type"], *model_type);
            assert_eq!(p["prompt_tokens"], 3);
            let n = p["generated_tokens"].as_u64().unwrap();
            assert!(n <= 8, "{model_type}");
            match p["stop_reason"].as_str().unwrap() {
                "token_budget" => assert_eq!(n, 8),
                "end_of_sequence" => assert!(n < 8),
                other => panic!("{other}"),
            }
            // Greedy decoding with a reset cache: identical on repeat.
            assert_eq!(g.generate(&pid, "patient has fever", 8).unwrap(), out);
        }
    }

    #[test]
    fn cancellation_stops_generation_and_leaves_the_generator_usable() {
        for model_type in GENERATION_ARCHITECTURES {
            let g = generator(model_type);
            let pid = OpaqueId::new("p");
            let before = g.generate(&pid, "patient has fever", 8).unwrap();
            let cancelled = AtomicBool::new(true);
            assert!(matches!(
                g.generate_cancellable(&pid, "patient has fever", 8, &cancelled),
                Err(GenerationRuntimeError::Cancelled)
            ));
            // The cache was reset: the next call equals the one before.
            assert_eq!(g.generate(&pid, "patient has fever", 8).unwrap(), before);
        }
    }

    #[test]
    fn llama_checkpoints_missing_layer_tensors_are_refused_without_a_panic() {
        let mut tensors =
            candle_core::safetensors::load_buffer(&tiny_weights("llama"), &Device::Cpu).unwrap();
        tensors.remove("model.layers.0.mlp.down_proj.weight");
        let path = std::env::temp_dir().join(format!(
            "medscale-105-llama-missing-{}.safetensors",
            std::process::id()
        ));
        candle_core::safetensors::save(&tensors, &path).unwrap();
        let weights = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        let err = prepare_generator_from_bytes(
            DigestSha256::of(b"llama"),
            provenance(),
            tiny_config("llama").to_string().as_bytes(),
            weights,
            tiny_tokenizer().as_bytes(),
            Device::Cpu,
        )
        .unwrap_err();
        assert!(matches!(err, GenerationRuntimeError::Prepare(_)), "{err}");
    }

    #[test]
    fn bounds_and_unknown_architectures_are_refused() {
        let g = generator("qwen2");
        let pid = OpaqueId::new("p");
        assert!(matches!(
            g.generate(&pid, "  ", 4),
            Err(GenerationRuntimeError::InputBound)
        ));
        assert!(matches!(
            g.generate(&pid, &"a".repeat(MAX_PROMPT_BYTES + 1), 4),
            Err(GenerationRuntimeError::InputBound)
        ));
        assert!(matches!(
            g.generate(&pid, "fever", 0),
            Err(GenerationRuntimeError::InvalidBudget)
        ));
        assert!(matches!(
            g.generate(&pid, "fever", MAX_NEW_TOKENS + 1),
            Err(GenerationRuntimeError::InvalidBudget)
        ));
        // 60 prompt tokens + 8 > the 64-position context.
        assert!(matches!(
            g.generate(&pid, &"fever ".repeat(60), 8),
            Err(GenerationRuntimeError::TooLong)
        ));
        let mut gpt2 = tiny_config("qwen2");
        gpt2["model_type"] = json!("gpt2");
        let err = prepare_generator_from_bytes(
            DigestSha256::of(b"x"),
            provenance(),
            gpt2.to_string().as_bytes(),
            Vec::new(),
            tiny_tokenizer().as_bytes(),
            Device::Cpu,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            GenerationRuntimeError::UnsupportedArchitecture(_)
        ));
        assert_eq!(eos_ids(&json!({"eos_token_id": [2, 7]})), [2, 7]);
        assert!(eos_ids(&json!({})).is_empty());
    }
}

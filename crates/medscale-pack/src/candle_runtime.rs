//! Native safetensors token-classification runtime (Spec 104).
//!
//! Runs Hugging Face token-classification checkpoints stored as
//! `model.safetensors` in pure Rust with `candle` 0.9.1 (CPU only): no Python,
//! no pickle, no executable model code. The architecture comes from
//! `config.json` `model_type` and must be on a fixed allowlist; the encoder is a
//! `candle-transformers` implementation and the token-classification head is
//! built here (a linear layer with bias at `classifier`, as in Transformers'
//! `*ForTokenClassification`; `candle`'s own DeBERTa NER head omits the bias).
//!
//! Whole documents run through the Spec 103 windowing, best-context vote and
//! entity decoding (`token_windows`). Output is proposal/evaluation evidence
//! only. Every artifact is read through the admitted manifest with digest
//! verification, and weights are loaded from those verified bytes.

use std::fmt;
use std::path::Path;

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{LayerNorm, Linear, VarBuilder};
use candle_transformers::models::{bert, debertav2, distilbert, modernbert, xlm_roberta};
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::packs::{ModelSourceProvenance, PackArtifactKind, PackManifestV0};
use serde_json::{Value, json};
use thiserror::Error;
use tokenizers::Tokenizer;

use crate::RuntimeOutput;
use crate::onnx_runtime::{OnnxRuntimeError, select_artifact};
use crate::token_windows::{
    BestContextVotes, WindowError, decode_entities, default_stride, plan_windows,
};

pub const CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID: &str = "candle_safetensors_token_classification_v1";
const MAX_LABELS: usize = 512;
const MAX_INPUT_BYTES: usize = 65_536;
const MAX_WINDOW: usize = 512;

/// Architectures this runtime builds (`config.json` `model_type`).
pub const CANDLE_ARCHITECTURES: &[&str] = &[
    "bert",
    "distilbert",
    "roberta",
    "xlm-roberta",
    "deberta-v2",
    "modernbert",
];

/// Fail-closed errors. Input text is never embedded in an error.
#[derive(Debug, Error)]
pub enum CandleRuntimeError {
    #[error("pack runtime requirement does not admit the candle safetensors runtime")]
    RuntimeNotAdmitted,
    #[error("artifact: {0}")]
    Artifact(#[from] OnnxRuntimeError),
    #[error("config.json is invalid: {0}")]
    InvalidConfig(String),
    #[error("model_type {0:?} is not supported by this runtime")]
    UnsupportedArchitecture(String),
    #[error("tokenizer failed")]
    Tokenizer,
    #[error("label metadata is invalid")]
    InvalidLabels,
    #[error("model provenance metadata is invalid")]
    InvalidProvenance,
    #[error("model preparation failed: {0}")]
    Prepare(String),
    #[error("model execution failed")]
    ModelExecution,
    #[error("model output shape is outside the token-classification contract")]
    OutputShape,
    #[error("model output contained a non-finite logit")]
    NonFiniteLogit,
    #[error("input is outside the admitted bound")]
    InputBound,
    #[error("input produced no tokens")]
    EmptyInput,
    #[error("score threshold must be between 0 and 1")]
    InvalidThreshold,
    #[error("document windowing failed: {0}")]
    Windowing(#[from] WindowError),
}

fn parse(what: &'static str) -> impl Fn(serde_json::Error) -> CandleRuntimeError {
    move |e| CandleRuntimeError::InvalidConfig(format!("{what}: {e}"))
}

fn prepare_err(e: impl fmt::Display) -> CandleRuntimeError {
    CandleRuntimeError::Prepare(format!("{e}").chars().take(600).collect())
}

/// The encoder families, each with its token-classification head.
enum Encoder {
    Bert(Box<bert::BertModel>),
    DistilBert(Box<distilbert::DistilBertModel>),
    Roberta(Box<xlm_roberta::XLMRobertaModel>),
    DebertaV2(Box<debertav2::DebertaV2Model>),
    ModernBert(Box<ModernBertTokenClassifier>),
}

/// ModernBERT encoder plus the Transformers prediction head
/// (`dense` → GELU → `norm`); `candle`'s own head loader is private.
struct ModernBertTokenClassifier {
    model: modernbert::ModernBert,
    head_dense: Linear,
    head_norm: LayerNorm,
}

/// A prepared local safetensors model. Weights are reusable; no user input is
/// retained between runs.
pub struct PreparedCandleTokenClassifier {
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    model_type: String,
    tokenizer: Tokenizer,
    labels: Vec<String>,
    window: usize,
    encoder: Encoder,
    classifier: Linear,
    device: Device,
    device_name: &'static str,
    device_fallback: Option<String>,
}

impl fmt::Debug for PreparedCandleTokenClassifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedCandleTokenClassifier")
            .field("content_digest", &self.content_digest)
            .field("model_type", &self.model_type)
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

/// Where a candle model runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CandleDevicePreference {
    /// Apple GPU (Metal) when this build and machine provide one, else CPU.
    #[default]
    Auto,
    /// Always the CPU.
    Cpu,
}

/// Resolves the device; returns it, its name and why Metal was not used.
fn select_device(preference: CandleDevicePreference) -> (Device, &'static str, Option<String>) {
    if preference == CandleDevicePreference::Cpu {
        return (Device::Cpu, "cpu", Some("cpu requested".into()));
    }
    select_metal()
}

/// The Apple GPU when present and working; a tiny operation proves it executes.
#[cfg(target_os = "macos")]
fn select_metal() -> (Device, &'static str, Option<String>) {
    match Device::new_metal(0) {
        Ok(device) => match Tensor::ones(4, DType::F32, &device)
            .and_then(|t| t.sum_all())
            .and_then(|t| t.to_scalar::<f32>())
        {
            Ok(v) if (v - 4.0).abs() < f32::EPSILON => (device, "metal", None),
            Ok(_) => (Device::Cpu, "cpu", Some("metal self-test mismatch".into())),
            Err(e) => (
                Device::Cpu,
                "cpu",
                Some(format!("metal self-test failed: {e}")),
            ),
        },
        Err(e) => (Device::Cpu, "cpu", Some(format!("no metal device: {e}"))),
    }
}

#[cfg(not(target_os = "macos"))]
fn select_metal() -> (Device, &'static str, Option<String>) {
    (
        Device::Cpu,
        "cpu",
        Some("metal is available on macOS only".into()),
    )
}

/// Options for [`PreparedCandleTokenClassifier::run_document`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleRunOptions {
    pub threshold: f32,
    pub stride: Option<usize>,
}

impl Default for CandleRunOptions {
    fn default() -> Self {
        Self {
            threshold: 0.0,
            stride: None,
        }
    }
}

/// Checks the `config.json` architecture against the allowlist.
pub fn candle_model_type(config: &Value) -> Result<String, CandleRuntimeError> {
    let model_type = config["model_type"]
        .as_str()
        .ok_or_else(|| CandleRuntimeError::InvalidConfig("model_type is missing".into()))?
        .to_ascii_lowercase();
    if CANDLE_ARCHITECTURES.contains(&model_type.as_str()) {
        Ok(model_type)
    } else {
        Err(CandleRuntimeError::UnsupportedArchitecture(model_type))
    }
}

fn usize_field(config: &Value, names: &[&str]) -> Result<usize, CandleRuntimeError> {
    names
        .iter()
        .find_map(|n| config[*n].as_u64())
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| CandleRuntimeError::InvalidConfig(format!("{} is missing", names[0])))
}

/// Builds encoder and head from verified `config.json` and weight bytes.
fn build(
    config: &Value,
    weights: Vec<u8>,
    num_labels: usize,
    device: &Device,
) -> Result<(String, usize, Encoder, Linear), CandleRuntimeError> {
    let model_type = candle_model_type(config)?;
    let mlx = crate::mlx_layout::is_mlx_export(config);
    if mlx && crate::mlx_layout::is_quantized(config) {
        return Err(CandleRuntimeError::InvalidConfig(
            "quantized MLX exports are not supported".into(),
        ));
    }
    let vb =
        VarBuilder::from_buffered_safetensors(weights, DType::F32, device).map_err(prepare_err)?;
    // OpenMed MLX exports store the same weights under renamed keys; the
    // encoders ask for Hugging Face names (see `mlx_layout`).
    let vb = if mlx {
        let layout_type = model_type.clone();
        vb.rename_f(move |name| crate::mlx_layout::mlx_key(name, &layout_type))
    } else {
        vb
    };
    let hidden = usize_field(config, &["hidden_size", "dim"])?;
    let max_positions = usize_field(config, &["max_position_embeddings"]).unwrap_or(MAX_WINDOW);
    let (encoder, window) = match model_type.as_str() {
        "bert" => {
            let cfg: bert::Config =
                serde_json::from_value(config.clone()).map_err(parse("bert"))?;
            (
                Encoder::Bert(Box::new(
                    bert::BertModel::load(vb.pp("bert"), &cfg).map_err(prepare_err)?,
                )),
                max_positions,
            )
        }
        "distilbert" => {
            let cfg: distilbert::Config =
                serde_json::from_value(config.clone()).map_err(parse("distilbert"))?;
            (
                Encoder::DistilBert(Box::new(
                    distilbert::DistilBertModel::load(vb.pp("distilbert"), &cfg)
                        .map_err(prepare_err)?,
                )),
                max_positions,
            )
        }
        "roberta" | "xlm-roberta" => {
            let cfg: xlm_roberta::Config =
                serde_json::from_value(config.clone()).map_err(parse("roberta"))?;
            // RoBERTa positions start after the padding index (offset 2).
            (
                Encoder::Roberta(Box::new(
                    xlm_roberta::XLMRobertaModel::new(&cfg, vb.pp("roberta"))
                        .map_err(prepare_err)?,
                )),
                max_positions.saturating_sub(2),
            )
        }
        "deberta-v2" => {
            let cfg: debertav2::Config =
                serde_json::from_value(config.clone()).map_err(parse("deberta-v2"))?;
            (
                Encoder::DebertaV2(Box::new(
                    debertav2::DebertaV2Model::load(vb.pp("deberta"), &cfg).map_err(prepare_err)?,
                )),
                max_positions,
            )
        }
        "modernbert" => {
            let cfg: modernbert::Config =
                serde_json::from_value(config.clone()).map_err(parse("modernbert"))?;
            let eps = config["norm_eps"].as_f64().unwrap_or(1e-5);
            let dense_bias = config["classifier_bias"].as_bool().unwrap_or(false);
            let head_dense = if dense_bias {
                candle_nn::linear(hidden, hidden, vb.pp("head.dense"))
            } else {
                candle_nn::linear_no_bias(hidden, hidden, vb.pp("head.dense"))
            }
            .map_err(prepare_err)?;
            let head_norm = candle_nn::layer_norm_no_bias(hidden, eps, vb.pp("head.norm"))
                .map_err(prepare_err)?;
            (
                Encoder::ModernBert(Box::new(ModernBertTokenClassifier {
                    model: modernbert::ModernBert::load(vb.clone(), &cfg).map_err(prepare_err)?,
                    head_dense,
                    head_norm,
                })),
                max_positions,
            )
        }
        other => {
            return Err(CandleRuntimeError::UnsupportedArchitecture(
                other.to_owned(),
            ));
        }
    };
    let classifier =
        candle_nn::linear(hidden, num_labels, vb.pp("classifier")).map_err(prepare_err)?;
    Ok((model_type, window.clamp(8, MAX_WINDOW), encoder, classifier))
}

/// Prepares an admitted Pack that declares the candle runtime, on the Apple
/// GPU when available (macOS, Metal) and otherwise on the CPU.
pub fn prepare_candle_token_classifier(
    pack_dir: &Path,
    pack: &PackManifestV0,
) -> Result<PreparedCandleTokenClassifier, CandleRuntimeError> {
    prepare_candle_token_classifier_with(pack_dir, pack, CandleDevicePreference::Auto)
}

/// As [`prepare_candle_token_classifier`] with an explicit device preference.
pub fn prepare_candle_token_classifier_with(
    pack_dir: &Path,
    pack: &PackManifestV0,
    preference: CandleDevicePreference,
) -> Result<PreparedCandleTokenClassifier, CandleRuntimeError> {
    let declared = pack
        .runtime_requirements
        .split(';')
        .any(|r| r == CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID);
    if !declared {
        return Err(CandleRuntimeError::RuntimeNotAdmitted);
    }
    let weights = select_artifact(pack_dir, pack, PackArtifactKind::SafetensorsModel, None)?;
    let config_bytes = select_artifact(
        pack_dir,
        pack,
        PackArtifactKind::ModelMetadata,
        Some("config.json"),
    )?;
    let tokenizer_bytes = select_artifact(
        pack_dir,
        pack,
        PackArtifactKind::TokenizerMeta,
        Some("tokenizer.json"),
    )?;
    let labels_bytes = select_artifact(
        pack_dir,
        pack,
        PackArtifactKind::TokenizerMeta,
        Some("labels.json"),
    )?;
    let provenance_bytes = select_artifact(
        pack_dir,
        pack,
        PackArtifactKind::ModelMetadata,
        Some("model.meta.json"),
    )?;
    let provenance: ModelSourceProvenance = serde_json::from_slice(&provenance_bytes)
        .map_err(|_| CandleRuntimeError::InvalidProvenance)?;
    if provenance.runtime_family != CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID {
        return Err(CandleRuntimeError::RuntimeNotAdmitted);
    }
    prepare_from_bytes(
        pack.content_digest.clone(),
        provenance,
        &config_bytes,
        weights,
        &tokenizer_bytes,
        &labels_bytes,
        preference,
    )
}

/// Prepares from already verified bytes.
pub(crate) fn prepare_from_bytes(
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    config_bytes: &[u8],
    weights: Vec<u8>,
    tokenizer_bytes: &[u8],
    labels_bytes: &[u8],
    preference: CandleDevicePreference,
) -> Result<PreparedCandleTokenClassifier, CandleRuntimeError> {
    let config: Value = serde_json::from_slice(config_bytes)
        .map_err(|e| CandleRuntimeError::InvalidConfig(e.to_string()))?;
    // Architecture is checked before any weight is parsed.
    candle_model_type(&config)?;
    let labels: Vec<String> =
        serde_json::from_slice(labels_bytes).map_err(|_| CandleRuntimeError::InvalidLabels)?;
    if labels.is_empty() || labels.len() > MAX_LABELS {
        return Err(CandleRuntimeError::InvalidLabels);
    }
    let mut tokenizer =
        Tokenizer::from_bytes(tokenizer_bytes).map_err(|_| CandleRuntimeError::Tokenizer)?;
    tokenizer.with_padding(None);
    tokenizer
        .with_truncation(None)
        .map_err(|_| CandleRuntimeError::Tokenizer)?;
    let (device, device_name, device_fallback) = select_device(preference);
    let (model_type, window, encoder, classifier) = build(&config, weights, labels.len(), &device)?;
    Ok(PreparedCandleTokenClassifier {
        content_digest,
        provenance,
        model_type,
        tokenizer,
        labels,
        window,
        encoder,
        classifier,
        device,
        device_name,
        device_fallback,
    })
}

impl PreparedCandleTokenClassifier {
    #[must_use]
    pub fn model_type(&self) -> &str {
        &self.model_type
    }

    #[must_use]
    pub const fn window(&self) -> usize {
        self.window
    }

    /// `metal` or `cpu`.
    #[must_use]
    pub const fn device_name(&self) -> &'static str {
        self.device_name
    }

    /// Why the Apple GPU was not used, when it was not.
    #[must_use]
    pub fn device_fallback(&self) -> Option<&str> {
        self.device_fallback.as_deref()
    }

    #[must_use]
    pub fn provenance(&self) -> &ModelSourceProvenance {
        &self.provenance
    }

    /// Logits `[len * labels]` for one window (no padding: dynamic length).
    fn infer(&self, ids: &[u32], type_ids: &[u32]) -> Result<Vec<f32>, CandleRuntimeError> {
        let exec = |_| CandleRuntimeError::ModelExecution;
        let len = ids.len();
        let device = &self.device;
        let input = Tensor::from_slice(ids, (1, len), device).map_err(exec)?;
        let types = Tensor::from_slice(type_ids, (1, len), device).map_err(exec)?;
        let ones = Tensor::ones((1, len), DType::U32, device).map_err(exec)?;
        let hidden = match &self.encoder {
            Encoder::Bert(m) => m.forward(&input, &types, Some(&ones)),
            Encoder::DistilBert(m) => {
                // DistilBERT masks where the mask is nonzero: nothing is masked here.
                let none = Tensor::zeros((1, 1, 1, len), DType::U8, device).map_err(exec)?;
                m.forward(&input, &none)
            }
            Encoder::Roberta(m) => m.forward(&input, &ones, &types, None, None, None),
            Encoder::DebertaV2(m) => m.forward(&input, Some(types.clone()), Some(ones.clone())),
            Encoder::ModernBert(m) => m
                .model
                .forward(&input, &ones)
                .and_then(|h| h.apply(&m.head_dense)?.gelu_erf()?.apply(&m.head_norm)),
        }
        .map_err(exec)?;
        let logits = self.classifier.forward(&hidden).map_err(exec)?;
        if logits.dims() != [1, len, self.labels.len()] {
            return Err(CandleRuntimeError::OutputShape);
        }
        let flat: Vec<f32> = logits
            .flatten_all()
            .and_then(|t| t.to_vec1())
            .map_err(exec)?;
        if flat.iter().any(|v| !v.is_finite()) {
            return Err(CandleRuntimeError::NonFiniteLogit);
        }
        Ok(flat)
    }

    /// Runs a whole document (windows, best-context vote, entity decoding).
    pub fn run_document(
        &self,
        pack_id: &OpaqueId,
        input: &str,
        options: CandleRunOptions,
    ) -> Result<RuntimeOutput, CandleRuntimeError> {
        if input.len() > MAX_INPUT_BYTES {
            return Err(CandleRuntimeError::InputBound);
        }
        if !(0.0..=1.0).contains(&options.threshold) {
            return Err(CandleRuntimeError::InvalidThreshold);
        }
        let encoding = self
            .tokenizer
            .encode_char_offsets(input, true)
            .map_err(|_| CandleRuntimeError::Tokenizer)?;
        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();
        let special = encoding.get_special_tokens_mask();
        let type_ids = encoding.get_type_ids();
        let offsets = encoding.get_offsets();
        let content: Vec<usize> = (0..ids.len())
            .filter(|&i| mask[i] != 0 && special[i] == 0)
            .collect();
        if content.is_empty() {
            return Err(CandleRuntimeError::EmptyInput);
        }
        let stride = options
            .stride
            .unwrap_or_else(|| default_stride(self.window));
        let windows = plan_windows(ids.len(), &content, self.window, stride)?;
        let width = self.labels.len();
        let mut votes = BestContextVotes::new(content.len());
        for w in &windows {
            let window_ids: Vec<u32> = w.positions.iter().map(|&p| ids[p]).collect();
            let window_types: Vec<u32> = w.positions.iter().map(|&p| type_ids[p]).collect();
            let logits = self.infer(&window_ids, &window_types)?;
            votes.offer(
                w,
                |slot| &logits[slot * width..(slot + 1) * width],
                |slot| offsets[w.positions[slot]],
            )?;
        }
        let tokens = votes.finish()?;
        let entities = decode_entities(&tokens, &self.labels, input, options.threshold);
        Ok(RuntimeOutput {
            proposal_payload: json!({
                "kind": "candle_token_classification_document_v1",
                "runtime": CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID,
                "model_type": self.model_type,
                "device": self.device_name,
                "device_fallback": self.device_fallback,
                "content_digest": self.content_digest.to_hex(),
                "source_kind": self.provenance.source_kind,
                "source_repository": self.provenance.repository,
                "source_revision": self.provenance.revision,
                "token_count": content.len(),
                "covered_tokens": tokens.len(),
                "window_count": windows.len(),
                "window_tokens": self.window,
                "stride": stride,
                "threshold": options.threshold,
                "offsets": "unicode_scalar",
                "entities": entities,
                "note": "Proposal/evaluation evidence only; never ClinicalAssertion",
            }),
            evidence_only: true,
            pack_id: pack_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_nn::VarMap;

    const TOKENIZER: &str = include_str!(
        "../../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/tokenizer.json"
    );

    fn tiny_bert_config() -> Value {
        json!({
            "model_type": "bert",
            "vocab_size": 8,
            "hidden_size": 8,
            "num_hidden_layers": 1,
            "num_attention_heads": 2,
            "intermediate_size": 16,
            "hidden_act": "gelu",
            "hidden_dropout_prob": 0.0,
            "max_position_embeddings": 16,
            "type_vocab_size": 2,
            "initializer_range": 0.02,
            "layer_norm_eps": 1e-12,
            "pad_token_id": 0,
            "position_embedding_type": "absolute",
            "use_cache": false,
            "classifier_dropout": null
        })
    }

    /// A randomly initialised tiny BERT token classifier saved as safetensors.
    fn tiny_bert_weights(config: &Value) -> Vec<u8> {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
        let cfg: bert::Config = serde_json::from_value(config.clone()).unwrap();
        bert::BertModel::load(vb.pp("bert"), &cfg).unwrap();
        candle_nn::linear(8, 2, vb.pp("classifier")).unwrap();
        // Unique per call: tests run in parallel in one process.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "medscale-104-tiny-{}-{}.safetensors",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        varmap.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        bytes
    }

    fn provenance() -> ModelSourceProvenance {
        serde_json::from_value(json!({
            "source_kind": "synthetic_fixture",
            "repository": "synthetic/tiny-bert",
            "revision": "0000000000000000000000000000000000000000",
            "original_file": "model.safetensors",
            "source_sha256": "0".repeat(64),
            "transform": "none",
            "fixed_sequence_length": 16,
            "task": "token-classification",
            "export_format": "safetensors",
            "runtime_family": CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID,
            "license_id": "Apache-2.0",
            "upstream_rights_uri": "https://example.invalid/synthetic"
        }))
        .unwrap()
    }

    #[test]
    fn tiny_bert_runs_a_whole_document_with_full_coverage() {
        let config = tiny_bert_config();
        let prepared = prepare_from_bytes(
            DigestSha256::of(b"tiny"),
            provenance(),
            config.to_string().as_bytes(),
            tiny_bert_weights(&config),
            TOKENIZER.as_bytes(),
            br#"["O","ENTITY"]"#,
            CandleDevicePreference::Cpu,
        )
        .unwrap();
        assert_eq!(prepared.model_type(), "bert");
        assert_eq!(prepared.window(), 16);
        let text = "alice visited clinic today bob called alice visited ".repeat(3);
        let out = prepared
            .run_document(&OpaqueId::new("p"), &text, CandleRunOptions::default())
            .unwrap();
        let p = &out.proposal_payload;
        assert!(out.evidence_only);
        assert_eq!(p["kind"], "candle_token_classification_document_v1");
        assert_eq!(p["token_count"], 24);
        assert_eq!(p["covered_tokens"], 24);
        assert_eq!(p["window_count"], 2);
    }

    #[test]
    fn device_selection_records_the_choice_and_any_fallback() {
        let (_, name, why) = select_device(CandleDevicePreference::Cpu);
        assert_eq!(name, "cpu");
        assert_eq!(why.as_deref(), Some("cpu requested"));
        let (device, name, why) = select_device(CandleDevicePreference::Auto);
        match name {
            "metal" => assert!(device.is_metal() && why.is_none()),
            _ => assert!(!device.is_metal() && why.is_some()),
        }
    }

    #[test]
    fn architecture_allowlist_and_input_bounds_fail_closed() {
        assert!(matches!(
            candle_model_type(&json!({"model_type": "gpt2"})),
            Err(CandleRuntimeError::UnsupportedArchitecture(t)) if t == "gpt2"
        ));
        assert!(matches!(
            candle_model_type(&json!({})),
            Err(CandleRuntimeError::InvalidConfig(_))
        ));
        assert_eq!(
            candle_model_type(&json!({"model_type": "XLM-RoBERTa"})).unwrap(),
            "xlm-roberta"
        );
        // An unsupported architecture is refused before weights are parsed.
        let err = prepare_from_bytes(
            DigestSha256::of(b"x"),
            provenance(),
            br#"{"model_type":"llama"}"#,
            b"not safetensors".to_vec(),
            TOKENIZER.as_bytes(),
            br#"["O"]"#,
            CandleDevicePreference::Cpu,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            CandleRuntimeError::UnsupportedArchitecture(_)
        ));
        let config = tiny_bert_config();
        let prepared = prepare_from_bytes(
            DigestSha256::of(b"tiny"),
            provenance(),
            config.to_string().as_bytes(),
            tiny_bert_weights(&config),
            TOKENIZER.as_bytes(),
            br#"["O","ENTITY"]"#,
            CandleDevicePreference::Cpu,
        )
        .unwrap();
        let pid = OpaqueId::new("p");
        assert!(matches!(
            prepared.run_document(&pid, "", CandleRunOptions::default()),
            Err(CandleRuntimeError::EmptyInput)
        ));
        assert!(matches!(
            prepared.run_document(&pid, &"a ".repeat(40_000), CandleRunOptions::default()),
            Err(CandleRuntimeError::InputBound)
        ));
        assert!(matches!(
            prepared.run_document(
                &pid,
                "alice",
                CandleRunOptions {
                    threshold: 2.0,
                    stride: None
                }
            ),
            Err(CandleRuntimeError::InvalidThreshold)
        ));
    }
}

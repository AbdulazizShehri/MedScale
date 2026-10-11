//! GLiNER zero-shot NER on candle (Spec 104).
//!
//! Runs OpenMed's uni-encoder span GLiNER checkpoints from their safetensors
//! MLX exports (`OpenMed-ZeroShot-NER-*-mlx`, family
//! `gliner-uni-encoder-span`). The PyTorch originals ship only pickle weights,
//! which MedScale refuses; the MLX exports hold the same tensors as plain F32
//! safetensors.
//!
//! Ported from OpenMed v3.0.0 (`maziyarpanahi/openmed@ea920f36`, Apache-2.0):
//! - `openmed/mlx/models/gliner_span.py`: `GLiNERSpanModel`, `SpanMarkerV0`;
//! - `openmed/mlx/models/gliner_common.py`: `ProjectionMLP`,
//!   `BidirectionalLSTM`, `extract_marker_embeddings`,
//!   `extract_word_embeddings`, `build_candidate_span_indices`;
//! - `openmed/mlx/inference.py`: `GLiNERMLXPipeline.predict_entities`
//!   (prompt `<<ENT>> label … <<SEP>> words`), `_split_words_with_offsets`,
//!   `_prepare_word_mask`, `_suppress_overlaps`.
//!
//! The DeBERTa-v2 backbone is `candle-transformers`' encoder, loaded through
//! the MLX key renaming in `mlx_layout`. Output is evidence only.

use std::fmt;
use std::path::Path;

use candle_core::{D, DType, Device, IndexOp, Module, Tensor};
use candle_nn::{Linear, VarBuilder};
use candle_transformers::models::debertav2;
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::packs::{ModelSourceProvenance, PackArtifactKind, PackManifestV0};
use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;
use tokenizers::Tokenizer;

use crate::RuntimeOutput;
use crate::candle_runtime::CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID;
use crate::onnx_runtime::{OnnxRuntimeError, select_artifact};

/// OpenMed family of the supported zero-shot exports.
pub const GLINER_SPAN_FAMILY: &str = "gliner-uni-encoder-span";
const MAX_INPUT_BYTES: usize = 65_536;
const MAX_TOKENS: usize = 512;
const MAX_LABELS: usize = 64;
const MAX_LABEL_BYTES: usize = 128;

#[derive(Debug, Error)]
pub enum GlinerRuntimeError {
    #[error("pack runtime requirement does not admit the candle runtime")]
    RuntimeNotAdmitted,
    #[error("artifact: {0}")]
    Artifact(#[from] OnnxRuntimeError),
    #[error("config.json is not an OpenMed GLiNER span export: {0}")]
    NotGliner(String),
    #[error("config.json is invalid: {0}")]
    InvalidConfig(String),
    #[error("model provenance metadata is invalid")]
    InvalidProvenance,
    #[error("tokenizer failed")]
    Tokenizer,
    #[error("model preparation failed: {0}")]
    Prepare(String),
    #[error("model execution failed")]
    ModelExecution,
    #[error("input is outside the admitted bound")]
    InputBound,
    #[error("labels are empty, too many, or too long")]
    InvalidLabels,
    #[error("text and prompt exceed {MAX_TOKENS} tokens; split the text")]
    TooLong,
    #[error("score threshold must be between 0 and 1")]
    InvalidThreshold,
}

fn prep(e: impl fmt::Display) -> GlinerRuntimeError {
    GlinerRuntimeError::Prepare(format!("{e}").chars().take(600).collect())
}

fn exec(_: candle_core::Error) -> GlinerRuntimeError {
    GlinerRuntimeError::ModelExecution
}

/// Two-layer projection block (`ProjectionMLP`): linear, ReLU, linear.
struct ProjectionMlp {
    linear1: Linear,
    linear2: Linear,
}

impl ProjectionMlp {
    fn load(vb: &VarBuilder, input: usize, output: usize) -> candle_core::Result<Self> {
        Ok(Self {
            linear1: candle_nn::linear(input, output * 4, vb.pp("linear1"))?,
            linear2: candle_nn::linear(output * 4, output, vb.pp("linear2"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        self.linear2.forward(&self.linear1.forward(x)?.relu()?)
    }
}

/// One MLX-layout LSTM direction (`Wx`, `Wh`, `bias`; gates i, f, g, o).
struct Lstm {
    wx: Tensor,
    wh: Tensor,
    bias: Tensor,
    hidden: usize,
}

impl Lstm {
    fn load(vb: &VarBuilder, input: usize, hidden: usize) -> candle_core::Result<Self> {
        Ok(Self {
            wx: vb.get((4 * hidden, input), "Wx")?,
            wh: vb.get((4 * hidden, hidden), "Wh")?,
            bias: vb.get(4 * hidden, "bias")?,
            hidden,
        })
    }

    /// `xs`: (n, input) -> (n, hidden).
    fn forward(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
        let n = xs.dim(0)?;
        let projected = xs.matmul(&self.wx.t()?)?.broadcast_add(&self.bias)?;
        let wh_t = self.wh.t()?;
        let mut h = Tensor::zeros((1, self.hidden), DType::F32, xs.device())?;
        let mut c = h.clone();
        let mut outputs = Vec::with_capacity(n);
        for step in 0..n {
            let gates = projected.narrow(0, step, 1)?.add(&h.matmul(&wh_t)?)?;
            let chunks = gates.chunk(4, 1)?;
            let i = candle_nn::ops::sigmoid(&chunks[0])?;
            let f = candle_nn::ops::sigmoid(&chunks[1])?;
            let g = chunks[2].tanh()?;
            let o = candle_nn::ops::sigmoid(&chunks[3])?;
            c = f.mul(&c)?.add(&i.mul(&g)?)?;
            h = o.mul(&c.tanh()?)?;
            outputs.push(h.clone());
        }
        Tensor::cat(&outputs, 0)
    }
}

/// A prepared GLiNER span model. No input is retained between calls.
pub struct PreparedGlinerSpanModel {
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    tokenizer: Tokenizer,
    deberta: debertav2::DebertaV2Model,
    token_projection: Linear,
    forward_lstm: Lstm,
    backward_lstm: Lstm,
    project_start: ProjectionMlp,
    project_end: ProjectionMlp,
    out_project: ProjectionMlp,
    prompt_rep: ProjectionMlp,
    class_token_index: u32,
    entity_token: String,
    separator_token: String,
    max_width: usize,
    device: Device,
}

impl fmt::Debug for PreparedGlinerSpanModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedGlinerSpanModel")
            .field("content_digest", &self.content_digest)
            .field("max_width", &self.max_width)
            .finish_non_exhaustive()
    }
}

/// One zero-shot entity with char offsets into the input text.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ZeroShotEntity {
    pub label: String,
    pub score: f32,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// True when a verified `config.json` describes an OpenMed GLiNER span export.
#[must_use]
pub fn is_gliner_span_export(config: &Value) -> bool {
    config["_mlx_family"] == GLINER_SPAN_FAMILY
}

/// Words and char offsets, as OpenMed `_split_words_with_offsets`
/// (`\w+(?:[-_]\w+)*|\S`).
fn split_words(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if !is_word(chars[i]) {
            spans.push((i, i + 1));
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && is_word(chars[i]) {
            i += 1;
        }
        // `(?:[-_]\w+)*`: a hyphen or underscore joined to more word characters.
        while i + 1 < chars.len() && (chars[i] == '-' || chars[i] == '_') && is_word(chars[i + 1]) {
            i += 1;
            while i < chars.len() && is_word(chars[i]) {
                i += 1;
            }
        }
        spans.push((start, i));
    }
    spans
}

/// `_suppress_overlaps`: keep the highest-scoring non-overlapping spans.
fn suppress_overlaps(mut entities: Vec<ZeroShotEntity>) -> Vec<ZeroShotEntity> {
    entities.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.start.cmp(&b.start))
            .then(a.end.cmp(&b.end))
    });
    let mut selected: Vec<ZeroShotEntity> = Vec::new();
    for e in entities {
        if selected
            .iter()
            .all(|s| e.end <= s.start || e.start >= s.end)
        {
            selected.push(e);
        }
    }
    selected.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.end.cmp(&b.end))
            .then(a.label.cmp(&b.label))
    });
    selected
}

fn backbone_config(config: &Value) -> Result<debertav2::Config, GlinerRuntimeError> {
    let mut backbone = config.clone();
    let encoder_hidden = config["encoder_hidden_size"]
        .as_u64()
        .or_else(|| config["hidden_size"].as_u64())
        .ok_or_else(|| GlinerRuntimeError::InvalidConfig("hidden size missing".into()))?;
    backbone["hidden_size"] = json!(encoder_hidden);
    serde_json::from_value(backbone).map_err(|e| GlinerRuntimeError::InvalidConfig(e.to_string()))
}

/// Prepares from verified bytes on `device`.
pub(crate) fn prepare_gliner_from_bytes(
    content_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    config_bytes: &[u8],
    weights: Vec<u8>,
    tokenizer_bytes: &[u8],
    device: Device,
) -> Result<PreparedGlinerSpanModel, GlinerRuntimeError> {
    let config: Value = serde_json::from_slice(config_bytes)
        .map_err(|e| GlinerRuntimeError::InvalidConfig(e.to_string()))?;
    if !is_gliner_span_export(&config) {
        return Err(GlinerRuntimeError::NotGliner(
            config["_mlx_family"].as_str().unwrap_or("none").to_owned(),
        ));
    }
    if config["model_type"] != "deberta-v2" {
        return Err(GlinerRuntimeError::NotGliner(
            "backbone is not deberta-v2".into(),
        ));
    }
    if crate::mlx_layout::is_quantized(&config) {
        return Err(GlinerRuntimeError::InvalidConfig("quantized export".into()));
    }
    let spec = &config["_mlx_prompt_spec"];
    let class_token_index = spec["class_token_index"]
        .as_u64()
        .or_else(|| config["class_token_index"].as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| GlinerRuntimeError::InvalidConfig("class_token_index missing".into()))?;
    let entity_token = spec["entity_token"]
        .as_str()
        .unwrap_or("<<ENT>>")
        .to_owned();
    let separator_token = spec["separator_token"]
        .as_str()
        .unwrap_or("<<SEP>>")
        .to_owned();
    let max_width = config["max_width"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(12)
        .clamp(1, 32);
    let hidden = config["hidden_size"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| GlinerRuntimeError::InvalidConfig("hidden_size missing".into()))?;
    let backbone = backbone_config(&config)?;
    let encoder_hidden = backbone.hidden_size;

    let mut tokenizer =
        Tokenizer::from_bytes(tokenizer_bytes).map_err(|_| GlinerRuntimeError::Tokenizer)?;
    tokenizer.with_padding(None);
    tokenizer
        .with_truncation(None)
        .map_err(|_| GlinerRuntimeError::Tokenizer)?;
    if tokenizer.token_to_id(&entity_token) != Some(class_token_index) {
        return Err(GlinerRuntimeError::InvalidConfig(
            "entity token does not map to class_token_index".into(),
        ));
    }

    let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device).map_err(prep)?;
    let deberta_vb = vb
        .clone()
        .rename_f(|name| crate::mlx_layout::mlx_key(name, "deberta-v2"));
    let deberta =
        debertav2::DebertaV2Model::load(deberta_vb.pp("deberta"), &backbone).map_err(prep)?;
    let half = hidden / 2;
    Ok(PreparedGlinerSpanModel {
        content_digest,
        provenance,
        tokenizer,
        deberta,
        token_projection: candle_nn::linear(encoder_hidden, hidden, vb.pp("token_projection"))
            .map_err(prep)?,
        forward_lstm: Lstm::load(&vb.pp("rnn.forward_lstm"), hidden, half).map_err(prep)?,
        backward_lstm: Lstm::load(&vb.pp("rnn.backward_lstm"), hidden, half).map_err(prep)?,
        project_start: ProjectionMlp::load(&vb.pp("span_rep_layer.project_start"), hidden, hidden)
            .map_err(prep)?,
        project_end: ProjectionMlp::load(&vb.pp("span_rep_layer.project_end"), hidden, hidden)
            .map_err(prep)?,
        out_project: ProjectionMlp::load(&vb.pp("span_rep_layer.out_project"), hidden * 2, hidden)
            .map_err(prep)?,
        prompt_rep: ProjectionMlp::load(&vb.pp("prompt_rep_layer"), hidden, hidden)
            .map_err(prep)?,
        class_token_index,
        entity_token,
        separator_token,
        max_width,
        device,
    })
}

/// Prepares an admitted candle Pack whose `config.json` is a GLiNER span
/// export (CPU; the candle device policy is the token-classification one).
pub fn prepare_gliner_span_model(
    pack_dir: &Path,
    pack: &PackManifestV0,
) -> Result<PreparedGlinerSpanModel, GlinerRuntimeError> {
    if !pack
        .runtime_requirements
        .split(';')
        .any(|r| r == CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID)
    {
        return Err(GlinerRuntimeError::RuntimeNotAdmitted);
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
        .map_err(|_| GlinerRuntimeError::InvalidProvenance)?;
    prepare_gliner_from_bytes(
        pack.content_digest.clone(),
        provenance,
        &config,
        weights,
        &tokenizer,
        Device::Cpu,
    )
}

impl PreparedGlinerSpanModel {
    #[must_use]
    pub fn provenance(&self) -> &ModelSourceProvenance {
        &self.provenance
    }

    /// Zero-shot entities for `labels` in `text` (`predict_entities`).
    pub fn predict(
        &self,
        text: &str,
        labels: &[&str],
        threshold: f32,
        flat_ner: bool,
    ) -> Result<Vec<ZeroShotEntity>, GlinerRuntimeError> {
        if text.len() > MAX_INPUT_BYTES {
            return Err(GlinerRuntimeError::InputBound);
        }
        if labels.is_empty()
            || labels.len() > MAX_LABELS
            || labels
                .iter()
                .any(|l| l.trim().is_empty() || l.len() > MAX_LABEL_BYTES)
        {
            return Err(GlinerRuntimeError::InvalidLabels);
        }
        if !(0.0..=1.0).contains(&threshold) {
            return Err(GlinerRuntimeError::InvalidThreshold);
        }
        let word_spans = split_words(text);
        if word_spans.is_empty() {
            return Ok(Vec::new());
        }
        let chars: Vec<usize> = text
            .char_indices()
            .map(|(b, _)| b)
            .chain(std::iter::once(text.len()))
            .collect();
        let words: Vec<&str> = word_spans
            .iter()
            .map(|(s, e)| &text[chars[*s]..chars[*e]])
            .collect();
        let mut prompt: Vec<&str> = Vec::with_capacity(labels.len() * 2 + 1);
        for label in labels {
            prompt.push(&self.entity_token);
            prompt.push(label);
        }
        prompt.push(&self.separator_token);
        let sequence: Vec<&str> = prompt.iter().chain(words.iter()).copied().collect();
        let encoding = self
            .tokenizer
            .encode(sequence.as_slice(), true)
            .map_err(|_| GlinerRuntimeError::Tokenizer)?;
        let ids = encoding.get_ids();
        if ids.len() > MAX_TOKENS {
            return Err(GlinerRuntimeError::TooLong);
        }
        // `_prepare_word_mask`: first sub-token of each text word. The word
        // index is kept with the position, so a word the tokenizer drops
        // (no tokens) cannot shift the offsets of the words after it.
        let mut word_positions = Vec::with_capacity(words.len());
        let mut word_index = Vec::with_capacity(words.len());
        let mut marker_positions = Vec::with_capacity(labels.len());
        let mut previous = None;
        for (position, (word_id, id)) in encoding.get_word_ids().iter().zip(ids).enumerate() {
            let Some(w) = *word_id else {
                previous = None;
                continue;
            };
            let w = w as usize;
            if w < prompt.len() {
                // Class markers count only inside the label prompt.
                if *id == self.class_token_index {
                    marker_positions.push(position as u32);
                }
            } else if previous != Some(w) {
                word_positions.push(position as u32);
                word_index.push(w - prompt.len());
            }
            previous = Some(w);
        }
        // One marker per label; a label that contains the entity token (or
        // tokenizes to it) would misalign labels and prompts.
        if marker_positions.len() != labels.len() {
            return Err(GlinerRuntimeError::InvalidLabels);
        }
        let n_words = word_positions.len();
        if n_words == 0 {
            return Ok(Vec::new());
        }
        let device = &self.device;
        let input = Tensor::from_slice(ids, (1, ids.len()), device).map_err(exec)?;
        let mask = Tensor::ones((1, ids.len()), DType::U32, device).map_err(exec)?;
        let hidden = self
            .deberta
            .forward(&input, None, Some(mask))
            .and_then(|h| h.i(0))
            .map_err(exec)?;
        let take = |positions: &[u32]| -> candle_core::Result<Tensor> {
            let index = Tensor::from_slice(positions, positions.len(), device)?;
            hidden.index_select(&index, 0)
        };
        let words_rep = take(&word_positions)
            .and_then(|w| self.token_projection.forward(&w))
            .map_err(exec)?;
        let prompts_rep = take(&marker_positions)
            .and_then(|p| self.token_projection.forward(&p))
            .and_then(|p| self.prompt_rep.forward(&p))
            .map_err(exec)?;
        // `BidirectionalLSTM` over the text words (single example, no padding).
        let forward = self.forward_lstm.forward(&words_rep).map_err(exec)?;
        let reversed_index: Vec<u32> = (0..n_words as u32).rev().collect();
        let reverse = Tensor::from_slice(&reversed_index, n_words, device).map_err(exec)?;
        let backward = words_rep
            .index_select(&reverse, 0)
            .and_then(|r| self.backward_lstm.forward(&r))
            .and_then(|b| b.index_select(&reverse, 0))
            .map_err(exec)?;
        let words_rep = Tensor::cat(&[forward, backward], D::Minus1).map_err(exec)?;
        // `SpanMarkerV0` over every candidate span up to `max_width` words.
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        for start in 0..n_words {
            for width in 0..self.max_width {
                let end = start + width;
                if end < n_words {
                    starts.push(start as u32);
                    ends.push(end as u32);
                }
            }
        }
        let start_rep = self.project_start.forward(&words_rep).map_err(exec)?;
        let end_rep = self.project_end.forward(&words_rep).map_err(exec)?;
        let pick = |rep: &Tensor, index: &[u32]| -> candle_core::Result<Tensor> {
            rep.index_select(&Tensor::from_slice(index, index.len(), device)?, 0)
        };
        let span_rep = Tensor::cat(
            &[
                pick(&start_rep, &starts).map_err(exec)?,
                pick(&end_rep, &ends).map_err(exec)?,
            ],
            D::Minus1,
        )
        .and_then(|t| t.relu())
        .and_then(|t| self.out_project.forward(&t))
        .map_err(exec)?;
        let scores = span_rep
            .matmul(&prompts_rep.t().map_err(exec)?)
            .and_then(|l| candle_nn::ops::sigmoid(&l))
            .and_then(|s| s.to_vec2::<f32>())
            .map_err(exec)?;
        let mut entities = Vec::new();
        for (span, row) in scores.iter().enumerate() {
            let (start_word, end_word) = (starts[span] as usize, ends[span] as usize);
            let (start, end) = (
                word_spans[word_index[start_word]].0,
                word_spans[word_index[end_word]].1,
            );
            for (label_index, score) in row.iter().enumerate() {
                if *score >= threshold {
                    entities.push(ZeroShotEntity {
                        label: labels[label_index].to_owned(),
                        score: *score,
                        start,
                        end,
                        text: text[chars[start]..chars[end]].to_owned(),
                    });
                }
            }
        }
        if flat_ner {
            return Ok(suppress_overlaps(entities));
        }
        entities.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then(a.end.cmp(&b.end))
                .then(a.label.cmp(&b.label))
        });
        Ok(entities)
    }

    /// Evidence-only proposal payload for one zero-shot run.
    pub fn run(
        &self,
        pack_id: &OpaqueId,
        text: &str,
        labels: &[&str],
        threshold: f32,
    ) -> Result<RuntimeOutput, GlinerRuntimeError> {
        let entities = self.predict(text, labels, threshold, true)?;
        Ok(RuntimeOutput {
            proposal_payload: json!({
                "kind": "gliner_zero_shot_ner_v1",
                "family": GLINER_SPAN_FAMILY,
                "content_digest": self.content_digest.to_hex(),
                "source_repository": self.provenance.repository,
                "source_revision": self.provenance.revision,
                "labels": labels,
                "threshold": threshold,
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
    use candle_nn::{Init, VarMap};

    #[test]
    fn words_split_like_openmed() {
        let text = "Type-2 diabetes, e.g. in snake_case; Émile";
        let words: Vec<String> = split_words(text)
            .into_iter()
            .map(|(s, e)| text.chars().skip(s).take(e - s).collect())
            .collect();
        assert_eq!(
            words,
            [
                "Type-2",
                "diabetes",
                ",",
                "e",
                ".",
                "g",
                ".",
                "in",
                "snake_case",
                ";",
                "Émile"
            ]
        );
        assert!(split_words("   ").is_empty());
    }

    #[test]
    fn overlaps_keep_the_best_span() {
        let e = |label: &str, score: f32, start: usize, end: usize| ZeroShotEntity {
            label: label.into(),
            score,
            start,
            end,
            text: String::new(),
        };
        let kept = suppress_overlaps(vec![
            e("a", 0.6, 0, 10),
            e("b", 0.9, 5, 8),
            e("c", 0.7, 10, 12),
        ]);
        let labels: Vec<&str> = kept.iter().map(|x| x.label.as_str()).collect();
        assert_eq!(labels, ["b", "c"]);
    }

    const H: usize = 8;
    const E: usize = 8;

    fn tiny_config() -> Value {
        json!({
            "model_type": "deberta-v2",
            "_mlx_family": GLINER_SPAN_FAMILY,
            "_mlx_task": "zero-shot-ner",
            "_mlx_weights_format": "safetensors",
            "_mlx_prompt_spec": {"entity_token": "<<ENT>>", "separator_token": "<<SEP>>", "class_token_index": 4},
            "vocab_size": 16, "hidden_size": H, "encoder_hidden_size": E,
            "num_hidden_layers": 1, "num_attention_heads": 2, "intermediate_size": 16,
            "hidden_act": "gelu", "hidden_dropout_prob": 0.0, "attention_probs_dropout_prob": 0.0,
            "max_position_embeddings": 64, "type_vocab_size": 0, "initializer_range": 0.02,
            "layer_norm_eps": 1e-7, "relative_attention": true, "max_relative_positions": -1,
            "pad_token_id": 0, "position_biased_input": false, "pos_att_type": ["p2c", "c2p"],
            "position_buckets": 8, "share_att_key": true, "norm_rel_ebd": "layer_norm",
            "max_width": 3
        })
    }

    fn tiny_tokenizer() -> String {
        let vocab = [
            "[PAD]", "[UNK]", "[CLS]", "[SEP]", "<<ENT>>", "<<SEP>>", "anatomy", "drug", "pain",
            "in", "the", "left", "knee", "aspirin", ".", "today",
        ];
        let vocab: serde_json::Map<String, Value> = vocab
            .iter()
            .enumerate()
            .map(|(i, t)| ((*t).to_owned(), json!(i)))
            .collect();
        json!({
            "version": "1.0", "truncation": null, "padding": null, "added_tokens": [],
            "normalizer": null,
            "pre_tokenizer": {"type": "WhitespaceSplit"},
            "post_processor": {
                "type": "TemplateProcessing",
                "single": [{"SpecialToken": {"id": "[CLS]", "type_id": 0}}, {"Sequence": {"id": "A", "type_id": 0}}, {"SpecialToken": {"id": "[SEP]", "type_id": 0}}],
                "pair": [{"Sequence": {"id": "A", "type_id": 0}}, {"Sequence": {"id": "B", "type_id": 1}}],
                "special_tokens": {
                    "[CLS]": {"id": "[CLS]", "ids": [2], "tokens": ["[CLS]"]},
                    "[SEP]": {"id": "[SEP]", "ids": [3], "tokens": ["[SEP]"]}
                }
            },
            "decoder": null,
            "model": {"type": "WordLevel", "vocab": vocab, "unk_token": "[UNK]"}
        })
        .to_string()
    }

    /// Random tiny GLiNER weights written in the OpenMed MLX layout.
    fn tiny_weights(config: &Value) -> Vec<u8> {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
        let backbone = backbone_config(config).unwrap();
        let deberta_vb = vb
            .clone()
            .rename_f(|name| crate::mlx_layout::mlx_key(name, "deberta-v2"));
        debertav2::DebertaV2Model::load(deberta_vb.pp("deberta"), &backbone).unwrap();
        let init = Init::Randn {
            mean: 0.0,
            stdev: 0.3,
        };
        let get = |shape: (usize, usize), name: &str| {
            vb.get_with_hints(shape, name, init).unwrap();
        };
        get((H, E), "token_projection.weight");
        for dir in ["forward_lstm", "backward_lstm"] {
            get((4 * (H / 2), H), &format!("rnn.{dir}.Wx"));
            get((4 * (H / 2), H / 2), &format!("rnn.{dir}.Wh"));
        }
        for (prefix, input) in [
            ("span_rep_layer.project_start", H),
            ("span_rep_layer.project_end", H),
            ("span_rep_layer.out_project", 2 * H),
            ("prompt_rep_layer", H),
        ] {
            get((4 * H, input), &format!("{prefix}.linear1.weight"));
            get((H, 4 * H), &format!("{prefix}.linear2.weight"));
        }
        let bias = |n: usize, name: &str| {
            vb.get_with_hints(n, name, init).unwrap();
        };
        bias(H, "token_projection.bias");
        for dir in ["forward_lstm", "backward_lstm"] {
            bias(4 * (H / 2), &format!("rnn.{dir}.bias"));
        }
        for prefix in [
            "span_rep_layer.project_start",
            "span_rep_layer.project_end",
            "span_rep_layer.out_project",
            "prompt_rep_layer",
        ] {
            bias(4 * H, &format!("{prefix}.linear1.bias"));
            bias(H, &format!("{prefix}.linear2.bias"));
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "medscale-104-gliner-{}-{}.safetensors",
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
            "source_kind": "synthetic_fixture", "repository": "synthetic/tiny-gliner",
            "revision": "0000000000000000000000000000000000000000",
            "original_file": "weights.safetensors", "source_sha256": "0".repeat(64),
            "transform": "none", "fixed_sequence_length": 64, "task": "zero-shot-ner",
            "export_format": "safetensors", "runtime_family": CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID,
            "license_id": "Apache-2.0", "upstream_rights_uri": "https://example.invalid/synthetic"
        }))
        .unwrap()
    }

    fn tiny_model() -> PreparedGlinerSpanModel {
        let config = tiny_config();
        prepare_gliner_from_bytes(
            DigestSha256::of(b"tiny-gliner"),
            provenance(),
            config.to_string().as_bytes(),
            tiny_weights(&config),
            tiny_tokenizer().as_bytes(),
            Device::Cpu,
        )
        .unwrap()
    }

    #[test]
    fn synthetic_gliner_scores_every_candidate_span_and_suppresses_overlaps() {
        let model = tiny_model();
        let text = "pain in the left knee today .";
        // Threshold 0: every candidate span scores; flat decoding keeps a
        // non-overlapping subset, each with valid offsets.
        let all = model
            .predict(text, &["anatomy", "drug"], 0.0, false)
            .unwrap();
        // 7 words, widths 1..=3: 7 + 6 + 5 candidate spans, two labels each.
        assert_eq!(all.len(), (7 + 6 + 5) * 2);
        let flat = model
            .predict(text, &["anatomy", "drug"], 0.0, true)
            .unwrap();
        assert!(!flat.is_empty());
        for pair in flat.windows(2) {
            assert!(pair[0].end <= pair[1].start, "flat spans do not overlap");
        }
        for e in &flat {
            assert!((0.0..=1.0).contains(&e.score));
            assert_eq!(
                e.text,
                text.chars()
                    .skip(e.start)
                    .take(e.end - e.start)
                    .collect::<String>()
            );
        }
        // Deterministic for the same input.
        assert_eq!(
            flat,
            model
                .predict(text, &["anatomy", "drug"], 0.0, true)
                .unwrap()
        );
        // A threshold above every sigmoid score returns nothing.
        assert!(
            model
                .predict(text, &["anatomy"], 1.0, true)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn invalid_inputs_and_non_gliner_configs_are_refused() {
        let model = tiny_model();
        assert!(matches!(
            model.predict("pain", &[], 0.5, true),
            Err(GlinerRuntimeError::InvalidLabels)
        ));
        assert!(matches!(
            model.predict("pain", &["anatomy"], 1.5, true),
            Err(GlinerRuntimeError::InvalidThreshold)
        ));
        assert!(matches!(
            model.predict(&"a ".repeat(40_000), &["anatomy"], 0.5, true),
            Err(GlinerRuntimeError::InputBound)
        ));
        assert!(
            model
                .predict("   ", &["anatomy"], 0.5, true)
                .unwrap()
                .is_empty()
        );
        // A label carrying the entity token would add a prompt marker.
        assert!(matches!(
            model.predict("pain", &["<<ENT>>"], 0.5, true),
            Err(GlinerRuntimeError::InvalidLabels)
        ));
        // The entity token inside the text is split into ordinary words
        // (`<`, `<`, `ENT`, `>`, `>`) and never acts as a label marker:
        // 6 words, widths 1..=3, one label.
        assert_eq!(
            model
                .predict("<<ENT>> pain", &["anatomy"], 0.0, false)
                .unwrap()
                .len(),
            6 + 5 + 4
        );
        let mut token_classifier = tiny_config();
        token_classifier["_mlx_family"] = json!("bert");
        let err = prepare_gliner_from_bytes(
            DigestSha256::of(b"x"),
            provenance(),
            token_classifier.to_string().as_bytes(),
            Vec::new(),
            tiny_tokenizer().as_bytes(),
            Device::Cpu,
        )
        .unwrap_err();
        assert!(matches!(err, GlinerRuntimeError::NotGliner(_)));
    }
}

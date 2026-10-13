//! Bounded local ONNX token-classification runtime (Spec 069).
//!
//! Runtime code consumes only an already-admitted local MedScale Pack. It never
//! performs network I/O and its output remains proposal/evaluation evidence.

use std::fmt;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::packs::{ModelSourceProvenance, PackArtifactKind, PackManifestV0};
use serde_json::json;
use thiserror::Error;
use tokenizers::{PaddingDirection, Tokenizer};
use tract_hir::internal::ElementWiseIntoHir as _;
use tract_hir::tract_core::ops::element_wise::{ElementWiseMiniOp, ElementWiseOp};
use tract_onnx::prelude::*;
use tract_onnx::tract_hir;

use crate::RuntimeOutput;
use crate::format::artifact_size_limit;
use crate::token_windows::{
    BestContextVotes, EntityAggregation, WindowError, decode_entities_by_word, default_stride,
    plan_windows,
};

pub const ONNX_TOKEN_CLASSIFIER_RUNTIME_ID: &str = "tract_onnx_token_classification_v1";
const MAX_LABELS: usize = 512;
const MAX_INPUT_BYTES: usize = 65_536;
/// Layout tag of the on-disk plan cache. Any change to the tract version or to
/// the graph rewrites applied before caching must change this tag.
const PLAN_CACHE_FORMAT: &str = "medscale-tract-plan-cache-v2;tract=0.22.4;pad-optional-inputs";
/// A cached plan larger than a model artifact may be is never written or read.
const PLAN_CACHE_MAX_BYTES: u64 = 1 << 30;
const PLAN_CACHE_SIDECAR_MAX_BYTES: u64 = 65_536;

/// What [`OnnxTokenClassifierRuntime::prepare_with_plan_cache`] did with the cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanCacheOutcome {
    /// A verified cached plan was loaded; the slow ONNX-to-typed step was skipped.
    Hit,
    /// No usable cached plan; the plan was built and written to the cache.
    Written,
    /// No usable cached plan, and the built plan could not be cached
    /// (serialization refused, over the size bound, or the write failed).
    NotCached,
}

/// Fail-closed runtime errors. Input text is never embedded in an error.
#[derive(Debug, Error)]
pub enum OnnxRuntimeError {
    #[error("pack runtime requirement does not admit tract ONNX token classification")]
    RuntimeNotAdmitted,
    #[error("required pack artifact is missing or ambiguous: {0}")]
    ArtifactSelection(&'static str),
    #[error("unsafe pack artifact path")]
    UnsafeArtifactPath,
    #[error("pack artifact digest changed after admission")]
    ArtifactDigestMismatch,
    #[error("pack artifact exceeds the admitted byte bound")]
    ArtifactBound,
    #[error("tokenizer failed")]
    Tokenizer,
    #[error("input is outside the admitted bound")]
    InputBound,
    #[error("token count is outside the admitted bound")]
    TokenBound,
    #[error("unsupported required model input: {0}")]
    UnsupportedModelInput(String),
    #[error("tokenizer is a BPE model without merges: text would be split into single characters")]
    DegenerateTokenizer,
    #[error("model execution failed")]
    ModelExecution,
    #[error("run cancelled by the caller")]
    Cancelled,
    /// Preparing the model failed. Preparation happens before any input is
    /// seen, so the detail (tract's message) never contains user text.
    #[error("model preparation failed at {stage}: {detail}")]
    Prepare { stage: &'static str, detail: String },
    #[error("model output shape is outside the token-classification contract")]
    OutputShape,
    #[error("model output contained a non-finite logit")]
    NonFiniteLogit,
    #[error("label metadata is invalid")]
    InvalidLabels,
    #[error("model provenance metadata is invalid")]
    InvalidProvenance,
    #[error("document windowing failed: {0}")]
    Windowing(#[from] WindowError),
    #[error("score threshold must be between 0 and 1")]
    InvalidThreshold,
}

/// Options for [`PreparedOnnxTokenClassifier::run_document`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DocumentRunOptions {
    /// Minimum mean entity score, from 0.0 to 1.0 (OpenMed default 0.0).
    pub threshold: f32,
    /// Overlapping content tokens between windows; `None` uses OpenMed's
    /// default (at most 96, a quarter of the window).
    pub stride: Option<usize>,
    /// Also report OpenMed's semantic-unit PII merge of the entities
    /// (`pii_merged_entities`; see [`crate::merge_entities_with_semantic_units`]).
    pub pii_semantic_merge: bool,
    /// Sub-token to entity aggregation; the default reproduces OpenMed's
    /// ONNX decoder (see [`EntityAggregation`]).
    pub aggregation: EntityAggregation,
    /// With `pii_semantic_merge`: match the PII patterns on detection-
    /// normalized text (NFKC, native digits folded; see
    /// [`crate::normalize_for_detection`]).
    pub pii_normalize_for_detection: bool,
}

impl Default for DocumentRunOptions {
    fn default() -> Self {
        Self {
            threshold: 0.0,
            stride: None,
            pii_semantic_merge: false,
            aggregation: EntityAggregation::OpenMedOnnx,
            pii_normalize_for_detection: false,
        }
    }
}

/// One bounded runtime evaluation and the signed source provenance that was executed.
#[derive(Debug, Clone, PartialEq)]
pub struct OnnxRuntimeEvaluation {
    pub output: RuntimeOutput,
    pub provenance: ModelSourceProvenance,
}

/// Portable CPU runtime for HF-exported ONNX token-classification models.
#[derive(Debug, Clone, Copy)]
pub struct OnnxTokenClassifierRuntime {
    max_tokens: usize,
}

/// Prepared local model session. Model weights and execution plan are reusable;
/// no user input is retained between evaluations.
/// Wall-clock milliseconds spent in each tract preparation stage. Load,
/// typing and decluttering are zero on a plan-cache hit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrepareStageTimings {
    pub load_ms: u64,
    pub typed_and_decluttered_ms: u64,
    pub optimized_ms: u64,
    pub runnable_ms: u64,
}

fn elapsed_ms(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub struct PreparedOnnxTokenClassifier {
    content_digest: DigestSha256,
    runtime_contract_digest: DigestSha256,
    provenance: ModelSourceProvenance,
    tokenizer: Tokenizer,
    /// Same tokenizer with truncation and padding disabled, for documents
    /// that span several windows (`run_document`).
    document_tokenizer: Tokenizer,
    pad_token: String,
    pad_id: u32,
    labels: Vec<String>,
    input_names: Vec<String>,
    fixed_sequence_length: usize,
    runnable: TypedSimplePlan<TypedModel>,
    stage_timings: PrepareStageTimings,
}

impl fmt::Debug for PreparedOnnxTokenClassifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedOnnxTokenClassifier")
            .field("content_digest", &self.content_digest)
            .field("runtime_contract_digest", &self.runtime_contract_digest)
            .field("provenance", &self.provenance)
            .field("input_names", &self.input_names)
            .field("fixed_sequence_length", &self.fixed_sequence_length)
            .finish_non_exhaustive()
    }
}

impl OnnxRuntimeError {
    fn prepare(stage: &'static str, error: &impl fmt::Display) -> Self {
        let detail: String = format!("{error:#}").chars().take(600).collect();
        Self::Prepare { stage, detail }
    }
}

impl OnnxTokenClassifierRuntime {
    /// Construct a runtime with an explicit token ceiling.
    pub fn new(max_tokens: usize) -> Result<Self, OnnxRuntimeError> {
        if !(1..=4096).contains(&max_tokens) {
            return Err(OnnxRuntimeError::TokenBound);
        }
        Ok(Self { max_tokens })
    }

    /// Verify artifacts and prepare an optimized reusable execution plan.
    pub fn prepare(
        &self,
        pack_dir: &Path,
        pack: &PackManifestV0,
    ) -> Result<PreparedOnnxTokenClassifier, OnnxRuntimeError> {
        self.prepare_inner(pack_dir, pack, None)
            .map(|(prepared, _)| prepared)
    }

    /// Like [`Self::prepare`], but keeps the decluttered tract plan as NNEF in
    /// `cache_dir`, so later prepares skip the ONNX-to-typed analysis (minutes
    /// for DeBERTa-v2 exports). Pack artifacts are still verified first. The
    /// cache key binds the Pack content digest, its runtime contract and
    /// [`PLAN_CACHE_FORMAT`]; a cached plan is used only when its SHA-256
    /// matches the sidecar written after it. Any cache problem falls back to
    /// the full build. The cache shares the trust of the local Pack store: the
    /// digest detects corruption and stale entries, not a local writer.
    pub fn prepare_with_plan_cache(
        &self,
        pack_dir: &Path,
        pack: &PackManifestV0,
        cache_dir: &Path,
    ) -> Result<(PreparedOnnxTokenClassifier, PlanCacheOutcome), OnnxRuntimeError> {
        self.prepare_inner(pack_dir, pack, Some(cache_dir))
    }

    fn prepare_inner(
        &self,
        pack_dir: &Path,
        pack: &PackManifestV0,
        cache_dir: Option<&Path>,
    ) -> Result<(PreparedOnnxTokenClassifier, PlanCacheOutcome), OnnxRuntimeError> {
        let model_bytes = select_artifact(pack_dir, pack, PackArtifactKind::OnnxModel, None)?;
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
            .map_err(|_| OnnxRuntimeError::InvalidProvenance)?;
        validate_provenance(&provenance)?;
        let declared_runtime = pack
            .runtime_requirements
            .split(';')
            .any(|requirement| requirement == ONNX_TOKEN_CLASSIFIER_RUNTIME_ID);
        if !declared_runtime || provenance.runtime_family != ONNX_TOKEN_CLASSIFIER_RUNTIME_ID {
            return Err(OnnxRuntimeError::RuntimeNotAdmitted);
        }
        let fixed_sequence_length = usize::try_from(provenance.fixed_sequence_length)
            .map_err(|_| OnnxRuntimeError::TokenBound)?;
        if fixed_sequence_length == 0 || fixed_sequence_length > self.max_tokens {
            return Err(OnnxRuntimeError::TokenBound);
        }

        let mut tokenizer =
            Tokenizer::from_bytes(&tokenizer_bytes).map_err(|_| OnnxRuntimeError::Tokenizer)?;
        reject_degenerate_tokenizer(&tokenizer_bytes)?;
        // The runtime owns padding to the fixed sequence length. Exported
        // tokenizers may embed their own fixed padding (OpenMed: 512), which
        // would break that contract, so padding is disabled here. The
        // tokenizer's own truncation is kept as configured (existing Packs rely
        // on it); an encoding that is still longer than the fixed sequence
        // length is refused below (TokenBound). The pad token is taken
        // from the tokenizer's own padding settings when present, else the
        // BERT (`[PAD]`) or RoBERTa/XLM-R (`<pad>`) convention.
        let (pad_token, pad_id) = tokenizer
            .get_padding()
            .map(|p| (p.pad_token.clone(), p.pad_id))
            .filter(|(token, id)| tokenizer.token_to_id(token) == Some(*id))
            .or_else(|| {
                ["[PAD]", "<pad>"]
                    .into_iter()
                    .find_map(|t| tokenizer.token_to_id(t).map(|id| (t.to_string(), id)))
            })
            .ok_or(OnnxRuntimeError::Tokenizer)?;
        tokenizer.with_padding(None);
        let mut document_tokenizer = tokenizer.clone();
        document_tokenizer
            .with_truncation(None)
            .map_err(|_| OnnxRuntimeError::Tokenizer)?;
        let labels: Vec<String> =
            serde_json::from_slice(&labels_bytes).map_err(|_| OnnxRuntimeError::InvalidLabels)?;
        if labels.is_empty() || labels.len() > MAX_LABELS {
            return Err(OnnxRuntimeError::InvalidLabels);
        }

        let runtime_contract_digest = runtime_contract_digest(pack)?;
        let cache = cache_dir.map(|dir| {
            let key = plan_cache_key(pack, &runtime_contract_digest, fixed_sequence_length);
            (dir, key)
        });
        let cached = cache
            .as_ref()
            .and_then(|(dir, key)| load_cached_plan(dir, key));
        let hit = cached.is_some();
        let mut stage_timings = PrepareStageTimings::default();
        let (decluttered, input_names) = if let Some(plan) = cached {
            plan
        } else {
            let load_started = Instant::now();
            let mut model_reader = Cursor::new(model_bytes);
            let onnx = tract_onnx::onnx();
            let mut proto = onnx
                .proto_model_for_read(&mut model_reader)
                .map_err(|e| OnnxRuntimeError::prepare("load", &e))?;
            drop(model_reader);
            // Load-time graph rewrites: fp16 exports are widened to fp32 in
            // memory (see `fp16_widen`), and omitted optional `Pad` inputs are
            // dropped (see `onnx_optional_inputs`).
            if let Some(graph) = proto.graph.as_mut() {
                crate::fp16_widen::widen_fp16(graph);
                crate::onnx_optional_inputs::drop_omitted_pad_inputs(graph);
            }
            let mut model = onnx
                .model_for_proto_model(&proto)
                .map_err(|e| OnnxRuntimeError::prepare("load", &e))?;
            drop(proto);
            let input_outlets = model
                .input_outlets()
                .map_err(|e| OnnxRuntimeError::prepare("inputs", &e))?
                .to_vec();
            let input_names: Vec<String> = input_outlets
                .iter()
                .map(|outlet| {
                    model
                        .outlet_label(*outlet)
                        .unwrap_or(&model.node(outlet.node).name)
                        .to_owned()
                })
                .collect();
            for (index, name) in input_names.iter().enumerate() {
                if !matches!(
                    name.as_str(),
                    "input_ids" | "attention_mask" | "token_type_ids"
                ) {
                    return Err(OnnxRuntimeError::UnsupportedModelInput(name.clone()));
                }
                model
                    .set_input_fact(
                        index,
                        InferenceFact::dt_shape(i64::datum_type(), tvec!(1, fixed_sequence_length)),
                    )
                    .map_err(|e| OnnxRuntimeError::prepare("input_fact", &e))?;
            }
            relax_symbolic_value_info(&mut model);
            normalize_static_shape_ops(&mut model);
            stage_timings.load_ms = elapsed_ms(load_started);
            let typed_started = Instant::now();
            let decluttered = model
                .into_typed()
                .and_then(TypedModel::into_decluttered)
                .map_err(|e| OnnxRuntimeError::prepare("optimize", &e))?;
            stage_timings.typed_and_decluttered_ms = elapsed_ms(typed_started);
            (decluttered, input_names)
        };
        let outcome = match (&cache, hit) {
            (_, true) => PlanCacheOutcome::Hit,
            (Some((dir, key)), false) => store_cached_plan(dir, key, &decluttered, &input_names),
            (None, false) => PlanCacheOutcome::NotCached,
        };
        let optimize_started = Instant::now();
        let optimized = decluttered
            .into_optimized()
            .map_err(|e| OnnxRuntimeError::prepare("optimize", &e))?;
        stage_timings.optimized_ms = elapsed_ms(optimize_started);
        if optimized
            .output_outlets()
            .map_err(|_| OnnxRuntimeError::OutputShape)?
            .len()
            != 1
        {
            return Err(OnnxRuntimeError::OutputShape);
        }
        let output_fact = optimized
            .output_fact(0)
            .map_err(|_| OnnxRuntimeError::OutputShape)?;
        let expected_output_shape = [1, fixed_sequence_length, labels.len()];
        if output_fact.datum_type != f32::datum_type()
            || output_fact.shape.as_concrete() != Some(expected_output_shape.as_slice())
        {
            return Err(OnnxRuntimeError::OutputShape);
        }
        let runnable_started = Instant::now();
        let runnable = optimized
            .into_runnable()
            .map_err(|e| OnnxRuntimeError::prepare("runnable", &e))?;
        stage_timings.runnable_ms = elapsed_ms(runnable_started);

        let prepared = PreparedOnnxTokenClassifier {
            content_digest: pack.content_digest.clone(),
            runtime_contract_digest,
            provenance,
            tokenizer,
            document_tokenizer,
            labels,
            input_names,
            fixed_sequence_length,
            runnable,
            pad_token,
            pad_id,
            stage_timings,
        };
        Ok((prepared, outcome))
    }

    /// Convenience one-shot path; callers handling repeated traffic should keep
    /// the prepared session returned by [`Self::prepare`].
    pub fn run(
        &self,
        pack_dir: &Path,
        pack: &PackManifestV0,
        input: &str,
    ) -> Result<OnnxRuntimeEvaluation, OnnxRuntimeError> {
        self.prepare(pack_dir, pack)?.run(&pack.pack_id, input)
    }
}

impl PreparedOnnxTokenClassifier {
    /// Time spent in each preparation stage (qualification evidence).
    #[must_use]
    pub fn stage_timings(&self) -> PrepareStageTimings {
        self.stage_timings
    }

    #[must_use]
    pub const fn fixed_sequence_length(&self) -> usize {
        self.fixed_sequence_length
    }

    /// True only when the current admitted manifest has the same executable contract.
    #[must_use]
    pub fn matches_runtime_contract(&self, pack: &PackManifestV0) -> bool {
        runtime_contract_digest(pack)
            .map(|digest| digest == self.runtime_contract_digest)
            .unwrap_or(false)
    }

    #[must_use]
    pub fn provenance(&self) -> &ModelSourceProvenance {
        &self.provenance
    }

    /// Run one bounded evaluation using the already-prepared local model.
    pub fn run(
        &self,
        pack_id: &OpaqueId,
        input: &str,
    ) -> Result<OnnxRuntimeEvaluation, OnnxRuntimeError> {
        if input.len() > MAX_INPUT_BYTES {
            return Err(OnnxRuntimeError::InputBound);
        }
        let mut encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|_| OnnxRuntimeError::Tokenizer)?;
        if encoding.get_ids().is_empty() || encoding.get_ids().len() > self.fixed_sequence_length {
            return Err(OnnxRuntimeError::TokenBound);
        }
        encoding.pad(
            self.fixed_sequence_length,
            self.pad_id,
            0,
            &self.pad_token,
            PaddingDirection::Right,
        );
        let ids = encoding.get_ids();
        let ids_i64: Vec<i64> = ids.iter().map(|id| i64::from(*id)).collect();
        let mask_i64: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|value| i64::from(*value))
            .collect();
        let type_ids_i64: Vec<i64> = encoding
            .get_type_ids()
            .iter()
            .map(|value| i64::from(*value))
            .collect();
        let logits = self.infer(&ids_i64, &mask_i64, &type_ids_i64)?;
        let mut predictions = Vec::new();
        let token_strings = encoding.get_tokens();
        for (token_index, token) in token_strings.iter().enumerate().take(ids.len()) {
            if encoding.get_attention_mask()[token_index] == 0 {
                continue;
            }
            let offset = token_index * self.labels.len();
            let row = &logits[offset..offset + self.labels.len()];
            if row.iter().any(|value| !value.is_finite()) {
                return Err(OnnxRuntimeError::NonFiniteLogit);
            }
            let (label_index, _) = row
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .ok_or(OnnxRuntimeError::OutputShape)?;
            predictions.push(json!({
                "token": token,
                "label": self.labels[label_index],
                "label_index": label_index,
            }));
        }
        Ok(OnnxRuntimeEvaluation {
            output: RuntimeOutput {
                proposal_payload: json!({
                    "kind": "onnx_token_classification_v1",
                    "runtime": ONNX_TOKEN_CLASSIFIER_RUNTIME_ID,
                    "content_digest": self.content_digest.to_hex(),
                    "source_kind": self.provenance.source_kind,
                    "source_repository": self.provenance.repository,
                    "source_revision": self.provenance.revision,
                    "token_count": predictions.len(),
                    "predictions": predictions,
                    "note": "Proposal/evaluation evidence only; never ClinicalAssertion",
                }),
                evidence_only: true,
                pack_id: pack_id.clone(),
            },
            provenance: self.provenance.clone(),
        })
    }
}

impl PreparedOnnxTokenClassifier {
    /// One static-shape forward pass; returns `fixed_sequence_length * labels`
    /// finite logits.
    fn infer(
        &self,
        ids: &[i64],
        mask: &[i64],
        type_ids: &[i64],
    ) -> Result<Vec<f32>, OnnxRuntimeError> {
        let len = ids.len();
        let mut model_inputs = tvec![];
        for name in &self.input_names {
            let values = match name.as_str() {
                "input_ids" => ids,
                "attention_mask" => mask,
                "token_type_ids" => type_ids,
                other => return Err(OnnxRuntimeError::UnsupportedModelInput(other.to_owned())),
            };
            let tensor = Tensor::from_shape(&[1, len], values)
                .map_err(|_| OnnxRuntimeError::ModelExecution)?;
            model_inputs.push(tensor.into_tvalue());
        }
        // tract can panic on graphs it prepared but cannot execute (an
        // out-of-range `Gather` in the Longformer exports, run 38170322676).
        // The panic is contained and reported as a model execution failure.
        let outputs = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.runnable.run(model_inputs)
        }))
        .map_err(|_| OnnxRuntimeError::ModelExecution)?
        .map_err(|_| OnnxRuntimeError::ModelExecution)?;
        if outputs.len() != 1 {
            return Err(OnnxRuntimeError::OutputShape);
        }
        let output = &outputs[0];
        if output.shape() != [1, len, self.labels.len()] {
            return Err(OnnxRuntimeError::OutputShape);
        }
        let logits = output
            .as_slice::<f32>()
            .map_err(|_| OnnxRuntimeError::OutputShape)?;
        Ok(logits.to_vec())
    }

    /// Runs a whole document of any length up to the input bound, without
    /// truncation: overlapping windows of `fixed_sequence_length` tokens, a
    /// best-context vote per token, then entity spans with char offsets and
    /// mean scores (ported from OpenMed v3.0.0; see `token_windows`). Every
    /// token must be covered, or the run fails with no partial result.
    pub fn run_document(
        &self,
        pack_id: &OpaqueId,
        input: &str,
        options: DocumentRunOptions,
    ) -> Result<OnnxRuntimeEvaluation, OnnxRuntimeError> {
        self.run_document_cancellable(pack_id, input, options, &AtomicBool::new(false))
    }

    /// [`Self::run_document`] that stops between windows once `cancel` is
    /// set, returning [`OnnxRuntimeError::Cancelled`] (no partial output).
    pub fn run_document_cancellable(
        &self,
        pack_id: &OpaqueId,
        input: &str,
        options: DocumentRunOptions,
        cancel: &AtomicBool,
    ) -> Result<OnnxRuntimeEvaluation, OnnxRuntimeError> {
        if input.len() > MAX_INPUT_BYTES {
            return Err(OnnxRuntimeError::InputBound);
        }
        if !(0.0..=1.0).contains(&options.threshold) {
            return Err(OnnxRuntimeError::InvalidThreshold);
        }
        let encoding = self
            .document_tokenizer
            .encode_char_offsets(input, true)
            .map_err(|_| OnnxRuntimeError::Tokenizer)?;
        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();
        let special = encoding.get_special_tokens_mask();
        let type_ids = encoding.get_type_ids();
        let offsets = encoding.get_offsets();
        let content: Vec<usize> = (0..ids.len())
            .filter(|&i| mask[i] != 0 && special[i] == 0)
            .collect();
        if content.is_empty() {
            return Err(OnnxRuntimeError::TokenBound);
        }
        let window = self.fixed_sequence_length;
        let stride = options.stride.unwrap_or_else(|| default_stride(window));
        let windows = plan_windows(ids.len(), &content, window, stride)?;
        let width = self.labels.len();
        let mut votes = BestContextVotes::new(content.len());
        for w in &windows {
            if cancel.load(Ordering::Relaxed) {
                return Err(OnnxRuntimeError::Cancelled);
            }
            let mut ids_i64 = vec![i64::from(self.pad_id); window];
            let mut mask_i64 = vec![0i64; window];
            let mut types_i64 = vec![0i64; window];
            for (slot, &pos) in w.positions.iter().enumerate() {
                ids_i64[slot] = i64::from(ids[pos]);
                mask_i64[slot] = i64::from(mask[pos]);
                types_i64[slot] = i64::from(type_ids[pos]);
            }
            let logits = self.infer(&ids_i64, &mask_i64, &types_i64)?;
            if logits.iter().any(|value| !value.is_finite()) {
                return Err(OnnxRuntimeError::NonFiniteLogit);
            }
            votes.offer(
                w,
                |slot| &logits[slot * width..(slot + 1) * width],
                |slot| offsets[w.positions[slot]],
            )?;
        }
        let tokens = votes.finish()?;
        let word_ids: Vec<Option<u32>> = content
            .iter()
            .map(|&i| encoding.get_word_ids()[i])
            .collect();
        let entities = decode_entities_by_word(
            &tokens,
            &word_ids,
            &self.labels,
            input,
            options.threshold,
            options.aggregation,
        );
        let pii_merged = options.pii_semantic_merge.then(|| {
            crate::pii_merge::merge_decoded(&entities, input, options.pii_normalize_for_detection)
        });
        let aggregation =
            (options.aggregation != EntityAggregation::OpenMedOnnx).then_some(options.aggregation);
        Ok(OnnxRuntimeEvaluation {
            output: RuntimeOutput {
                proposal_payload: crate::pii_merge::with_merged(
                    json!({
                        "kind": "onnx_token_classification_document_v1",
                        "runtime": ONNX_TOKEN_CLASSIFIER_RUNTIME_ID,
                        "content_digest": self.content_digest.to_hex(),
                        "source_kind": self.provenance.source_kind,
                        "source_repository": self.provenance.repository,
                        "source_revision": self.provenance.revision,
                        "token_count": content.len(),
                        "covered_tokens": tokens.len(),
                        "window_count": windows.len(),
                        "window_tokens": window,
                        "stride": stride,
                        "threshold": options.threshold,
                        "offsets": "unicode_scalar",
                        "entities": entities,
                        "note": "Proposal/evaluation evidence only; never ClinicalAssertion",
                    }),
                    pii_merged,
                    aggregation,
                ),
                evidence_only: true,
                pack_id: pack_id.clone(),
            },
            provenance: self.provenance.clone(),
        })
    }
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, OnnxRuntimeError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(OnnxRuntimeError::UnsafeArtifactPath);
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|_| OnnxRuntimeError::ArtifactDigestMismatch)?;
    let candidate = root.join(path);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| OnnxRuntimeError::ArtifactDigestMismatch)?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(OnnxRuntimeError::UnsafeArtifactPath);
    }
    Ok(canonical_candidate)
}

fn normalized_artifact_bytes(
    path: &Path,
    kind: PackArtifactKind,
) -> Result<Vec<u8>, OnnxRuntimeError> {
    let limit = artifact_size_limit(kind);
    let metadata = fs::metadata(path).map_err(|_| OnnxRuntimeError::ArtifactDigestMismatch)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(OnnxRuntimeError::ArtifactBound);
    }
    let file = fs::File::open(path).map_err(|_| OnnxRuntimeError::ArtifactDigestMismatch)?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| OnnxRuntimeError::ArtifactDigestMismatch)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(OnnxRuntimeError::ArtifactBound);
    }
    if !matches!(
        kind,
        PackArtifactKind::FixtureBytes
            | PackArtifactKind::TokenizerMeta
            | PackArtifactKind::ModelMetadata
    ) {
        return Ok(bytes);
    }
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' {
            if index + 1 < bytes.len() && bytes[index + 1] == b'\n' {
                normalized.push(b'\n');
                index += 2;
                continue;
            }
            normalized.push(b'\n');
            index += 1;
            continue;
        }
        normalized.push(bytes[index]);
        index += 1;
    }
    Ok(normalized)
}

pub(crate) fn select_artifact(
    root: &Path,
    pack: &PackManifestV0,
    kind: PackArtifactKind,
    required_name: Option<&str>,
) -> Result<Vec<u8>, OnnxRuntimeError> {
    let mut selected = None;
    for artifact in &pack.artifacts {
        let path = safe_join(root, &artifact.relative_path)?;
        let bytes = normalized_artifact_bytes(&path, artifact.kind)?;
        let digest = medscale_contracts::objects::DigestSha256::of(&bytes);
        if digest != artifact.digest {
            return Err(OnnxRuntimeError::ArtifactDigestMismatch);
        }
        let name_matches = required_name
            .map(|name| artifact.relative_path == name)
            .unwrap_or(true);
        if artifact.kind == kind && name_matches {
            if selected.is_some() {
                return Err(OnnxRuntimeError::ArtifactSelection("ambiguous artifact"));
            }
            selected = Some(bytes);
        }
    }
    selected.ok_or(OnnxRuntimeError::ArtifactSelection(match required_name {
        Some("tokenizer.json") => "tokenizer.json",
        Some("labels.json") => "labels.json",
        Some("model.meta.json") => "model.meta.json",
        _ => "onnx_model",
    }))
}

fn plan_cache_key(
    pack: &PackManifestV0,
    runtime_contract_digest: &DigestSha256,
    fixed_sequence_length: usize,
) -> String {
    let material = format!(
        "{PLAN_CACHE_FORMAT}\n{}\n{}\n{fixed_sequence_length}",
        pack.content_digest.to_hex(),
        runtime_contract_digest.to_hex()
    );
    DigestSha256::of(material.as_bytes()).to_hex()
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PlanCacheSidecar {
    format: String,
    plan_sha256: String,
    input_names: Vec<String>,
}

fn plan_cache_nnef() -> tract_nnef::internal::Nnef {
    use tract_onnx::WithOnnx as _;
    tract_nnef::nnef().with_tract_core().with_onnx()
}

fn read_bounded(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

fn load_cached_plan(dir: &Path, key: &str) -> Option<(TypedModel, Vec<String>)> {
    let sidecar = read_bounded(
        &dir.join(format!("{key}.json")),
        PLAN_CACHE_SIDECAR_MAX_BYTES,
    )?;
    let sidecar: PlanCacheSidecar = serde_json::from_slice(&sidecar).ok()?;
    if sidecar.format != PLAN_CACHE_FORMAT
        || sidecar.input_names.is_empty()
        || !sidecar.input_names.iter().all(|name| {
            matches!(
                name.as_str(),
                "input_ids" | "attention_mask" | "token_type_ids"
            )
        })
    {
        return None;
    }
    let plan = read_bounded(&dir.join(format!("{key}.nnef.tar")), PLAN_CACHE_MAX_BYTES)?;
    if DigestSha256::of(&plan).to_hex() != sidecar.plan_sha256 {
        return None;
    }
    let model = plan_cache_nnef()
        .model_for_read(&mut Cursor::new(plan))
        .ok()?;
    (model.inputs.len() == sidecar.input_names.len()).then_some((model, sidecar.input_names))
}

fn store_cached_plan(
    dir: &Path,
    key: &str,
    model: &TypedModel,
    input_names: &[String],
) -> PlanCacheOutcome {
    let Ok(plan) = plan_cache_nnef().write_to_tar(model, Vec::new()) else {
        return PlanCacheOutcome::NotCached;
    };
    if plan.len() as u64 > PLAN_CACHE_MAX_BYTES {
        return PlanCacheOutcome::NotCached;
    }
    let sidecar = PlanCacheSidecar {
        format: PLAN_CACHE_FORMAT.to_owned(),
        plan_sha256: DigestSha256::of(&plan).to_hex(),
        input_names: input_names.to_vec(),
    };
    let Ok(sidecar) = serde_json::to_vec(&sidecar) else {
        return PlanCacheOutcome::NotCached;
    };
    // The plan is renamed into place before its sidecar, so a reader never
    // pairs a sidecar with a partially written plan. Temporary names carry the
    // process id so concurrent writers do not interleave; a sidecar paired
    // with another writer's plan fails the digest check and is rebuilt.
    let write = |name: String, bytes: &[u8]| -> std::io::Result<()> {
        let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, dir.join(name))
    };
    let stored = fs::create_dir_all(dir)
        .and_then(|()| write(format!("{key}.nnef.tar"), &plan))
        .and_then(|()| write(format!("{key}.json"), &sidecar));
    if stored.is_ok() {
        PlanCacheOutcome::Written
    } else {
        PlanCacheOutcome::NotCached
    }
}

fn runtime_contract_digest(pack: &PackManifestV0) -> Result<DigestSha256, OnnxRuntimeError> {
    let bytes = serde_json::to_vec(&(pack.runtime_requirements.as_str(), &pack.artifacts))
        .map_err(|_| OnnxRuntimeError::RuntimeNotAdmitted)?;
    Ok(DigestSha256::of(&bytes))
}

fn validate_provenance(provenance: &ModelSourceProvenance) -> Result<(), OnnxRuntimeError> {
    let source_ok = matches!(
        provenance.source_kind.as_str(),
        "hugging_face" | "synthetic_fixture"
    );
    if !source_ok
        || provenance.repository.trim().is_empty()
        || provenance.revision.trim().is_empty()
        || provenance.original_file.trim().is_empty()
        || provenance.source_sha256.len() != 64
        || !provenance
            .source_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || provenance.transform.trim().is_empty()
        || provenance.fixed_sequence_length == 0
        || provenance.fixed_sequence_length > 4096
        || provenance.task != "token-classification"
        || provenance.export_format != "onnx"
        || provenance.runtime_family != ONNX_TOKEN_CLASSIFIER_RUNTIME_ID
        || provenance.license_id.trim().is_empty()
        || provenance.upstream_rights_uri.trim().is_empty()
    {
        return Err(OnnxRuntimeError::InvalidProvenance);
    }
    Ok(())
}

/// Stateless cast to `i64`, usable during inference-time constant folding.
#[derive(Debug, Clone)]
struct CastToI64;

impl ElementWiseMiniOp for CastToI64 {
    fn name(&self) -> String {
        "medscale.CastToI64".into()
    }

    fn output_type(&self, _input_type: DatumType) -> Option<DatumType> {
        Some(i64::datum_type())
    }

    fn eval_out_of_place(&self, t: &Tensor, _out_dt: Option<DatumType>) -> TractResult<Tensor> {
        Ok(t.cast_to::<i64>()?.into_owned())
    }
}

/// Spec 103: some exports (for example the XLM-R `model_fp16.onnx` files)
/// declare intermediate `value_info` shapes with symbolic dimensions
/// (`batch`, `sequence`). `tract-onnx` always imports them as facts, and they
/// then conflict with the concrete `[1, fixed_sequence_length]` inputs this
/// runtime sets ("Impossible to unify Sym(batch) with Val(1)"). Symbolic
/// dimensions are relaxed to unknown, keeping the rank, the concrete
/// dimensions and the datum type, so analysis derives them from the inputs.
/// Returns the number of relaxed facts.
fn relax_symbolic_value_info(model: &mut InferenceModel) -> usize {
    use tract_hir::infer::{GenericFactoid, ShapeFactoid};
    let mut relaxed = 0;
    for node in 0..model.nodes().len() {
        for slot in 0..model.nodes()[node].outputs.len() {
            let outlet = OutletId::new(node, slot);
            let Ok(fact) = model.outlet_fact(outlet) else {
                continue;
            };
            let symbolic = |d: &GenericFactoid<TDim>| matches!(d, GenericFactoid::Only(dim) if dim.to_i64().is_err());
            if !fact.shape.dims().any(symbolic) {
                continue;
            }
            let dims = fact
                .shape
                .dims()
                .map(|d| {
                    if symbolic(d) {
                        GenericFactoid::Any
                    } else {
                        d.clone()
                    }
                })
                .collect();
            let shape = if fact.shape.is_open() {
                ShapeFactoid::open(dims)
            } else {
                ShapeFactoid::closed(dims)
            };
            let relaxed_fact = fact.clone().with_shape(shape);
            if model.set_outlet_fact(outlet, relaxed_fact).is_ok() {
                relaxed += 1;
            }
        }
    }
    relaxed
}

/// Spec 103: a BPE tokenizer with a large vocabulary but no merges can only
/// emit single characters, so the model sees input it was never trained on
/// and returns plausible-looking but meaningless labels. Observed in the
/// published OpenMed v3.0.0 XLM-R NER exports (`OpenMed-NER-*-BigMed-278M`:
/// 250,002-entry BPE vocabulary, zero merges; qualification run
/// 37974743874 labelled every character `O`). Such tokenizers are refused.
fn reject_degenerate_tokenizer(bytes: &[u8]) -> Result<(), OnnxRuntimeError> {
    let json: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| OnnxRuntimeError::Tokenizer)?;
    let model = &json["model"];
    if model["type"] == "BPE" {
        let merges = model["merges"].as_array().map_or(0, Vec::len);
        let vocab = model["vocab"].as_object().map_or(0, serde_json::Map::len);
        if merges == 0 && vocab > 1024 {
            return Err(OnnxRuntimeError::DegenerateTokenizer);
        }
    }
    Ok(())
}

/// ONNX `Sign` for every numeric type. `tract` evaluates `Sign` only on
/// floats (and quantized ints); DeBERTa-v2 exports apply it to `int64`
/// relative positions (`log_bucket_position`). Integers map to -1, 0 or 1
/// (unsigned: 0 or 1); floats keep zero and NaN and otherwise take the sign.
#[derive(Debug, Clone)]
struct SignAnyNumeric;

impl ElementWiseMiniOp for SignAnyNumeric {
    fn name(&self) -> String {
        "medscale.SignAnyNumeric".into()
    }

    fn output_type(&self, input_type: DatumType) -> Option<DatumType> {
        Some(input_type)
    }

    fn eval_out_of_place(&self, t: &Tensor, _out_dt: Option<DatumType>) -> TractResult<Tensor> {
        fn ints<T: Datum + Copy + PartialOrd + From<i8>>(t: &mut Tensor) -> TractResult<()> {
            let (zero, one, minus) = (T::from(0), T::from(1), T::from(-1));
            for x in t.as_slice_mut::<T>()? {
                *x = if *x > zero {
                    one
                } else if *x < zero {
                    minus
                } else {
                    zero
                };
            }
            Ok(())
        }
        fn unsigned<T: Datum + Copy + PartialOrd + From<u8>>(t: &mut Tensor) -> TractResult<()> {
            for x in t.as_slice_mut::<T>()? {
                *x = if *x > T::from(0) {
                    T::from(1)
                } else {
                    T::from(0)
                };
            }
            Ok(())
        }
        let mut out = t.clone();
        match t.datum_type() {
            DatumType::I64 => ints::<i64>(&mut out)?,
            DatumType::I32 => ints::<i32>(&mut out)?,
            DatumType::I16 => ints::<i16>(&mut out)?,
            DatumType::I8 => ints::<i8>(&mut out)?,
            DatumType::U64 => unsigned::<u64>(&mut out)?,
            DatumType::U32 => unsigned::<u32>(&mut out)?,
            DatumType::U16 => unsigned::<u16>(&mut out)?,
            DatumType::U8 => unsigned::<u8>(&mut out)?,
            DatumType::F64 => {
                for x in out.as_slice_mut::<f64>()? {
                    if *x != 0.0 && !x.is_nan() {
                        *x = x.signum();
                    }
                }
            }
            DatumType::F32 => {
                for x in out.as_slice_mut::<f32>()? {
                    if *x != 0.0 && !x.is_nan() {
                        *x = x.signum();
                    }
                }
            }
            other => {
                return Err(TractError::msg(format!(
                    "Sign is not defined for {other:?}"
                )));
            }
        }
        Ok(out)
    }
}

/// Spec 103: `tract-onnx` loads ONNX `Shape` and `Cast(to=INT64)` as symbolic
/// `TDim` operations. In Transformer exports (for example OpenMed BERT token
/// classifiers) shape arithmetic then meets `i64` constants and analysis fails
/// ("Impossible to unify TDim with I64" at the position-id `Range`). This
/// runtime always fixes every input to a concrete `[1, fixed_sequence_length]`
/// shape, so those values are concrete integers and can be typed as `i64`
/// without changing semantics. Only full-range `Shape` nodes and casts to
/// `TDim` are rewritten. `Sign` becomes [`SignAnyNumeric`]. The rewrite also
/// descends into both branches of ONNX `If` (DeBERTa-v2 computes relative
/// position buckets inside one). Returns the number of rewritten nodes.
fn normalize_static_shape_ops(model: &mut InferenceModel) -> usize {
    let mut rewritten = 0;
    for node in model.nodes_mut() {
        if let Some(branch) = node.op_as_mut::<tract_onnx::ops::logic::If>() {
            rewritten += normalize_static_shape_ops(&mut branch.then_body);
            rewritten += normalize_static_shape_ops(&mut branch.else_body);
            continue;
        }
        let op = format!("{:?}", node.op);
        if node.op.name() == "Sign" || op == "ElementWiseOp(Sign)" {
            node.op = ElementWiseOp(Box::new(SignAnyNumeric), None).into_hir();
            rewritten += 1;
        } else if node.op.name() == "Shape" && op.contains("start: 0, end: None") {
            node.op = tract_hir::ops::expandable::expand(tract_hir::ops::array::Shape::new(
                i64::datum_type(),
            ));
            rewritten += 1;
        } else if op == "ElementWiseOp(Cast { to: TDim })" {
            node.op = ElementWiseOp(Box::new(CastToI64), None).into_hir();
            rewritten += 1;
        }
    }
    rewritten
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpe_without_merges_and_a_large_vocabulary_is_refused() {
        let vocab: serde_json::Map<String, serde_json::Value> = (0..2000)
            .map(|i| (format!("t{i}"), serde_json::json!(i)))
            .collect();
        let degenerate =
            serde_json::json!({"model": {"type": "BPE", "vocab": vocab, "merges": []}});
        assert!(matches!(
            reject_degenerate_tokenizer(degenerate.to_string().as_bytes()),
            Err(OnnxRuntimeError::DegenerateTokenizer)
        ));
        let merged =
            serde_json::json!({"model": {"type": "BPE", "vocab": vocab, "merges": ["t 1"]}});
        assert!(reject_degenerate_tokenizer(merged.to_string().as_bytes()).is_ok());
        let unigram = serde_json::json!({"model": {"type": "Unigram", "vocab": []}});
        assert!(reject_degenerate_tokenizer(unigram.to_string().as_bytes()).is_ok());
        let tiny = serde_json::json!({"model": {"type": "WordLevel", "vocab": {"a": 0}}});
        assert!(reject_degenerate_tokenizer(tiny.to_string().as_bytes()).is_ok());
    }

    #[test]
    fn sign_covers_signed_unsigned_and_float_tensors() {
        let op = SignAnyNumeric;
        let i = op
            .eval_out_of_place(&tensor1(&[-7i64, 0, 3, i64::MIN]), None)
            .unwrap();
        assert_eq!(i.as_slice::<i64>().unwrap(), &[-1, 0, 1, -1]);
        let u = op.eval_out_of_place(&tensor1(&[0u8, 9]), None).unwrap();
        assert_eq!(u.as_slice::<u8>().unwrap(), &[0, 1]);
        let f = op
            .eval_out_of_place(&tensor1(&[-2.5f32, 0.0, 4.0]), None)
            .unwrap();
        assert_eq!(f.as_slice::<f32>().unwrap(), &[-1.0, 0.0, 1.0]);
        assert_eq!(op.output_type(DatumType::I64), Some(DatumType::I64));
        assert!(op.eval_out_of_place(&tensor1(&[true]), None).is_err());
    }
}

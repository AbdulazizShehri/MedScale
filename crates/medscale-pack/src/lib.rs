//! Offline Pack v0 admission (Spec 008) + MESC synthetic verifier (Spec 036).

mod asr_runtime;
#[cfg(feature = "synthetic-fixtures")]
#[doc(hidden)]
pub use asr_runtime::fixtures as asr_fixtures;
mod candle_runtime;
mod catalog;
mod compatibility;
mod distilbert;
pub mod entity_eval;
mod format;
mod fp16_widen;
mod generation_runtime;
mod gliner_runtime;
#[cfg(feature = "synthetic-fixtures")]
#[doc(hidden)]
pub use gliner_runtime::fixtures as gliner_fixtures;
mod hf_snapshot;
mod hir_const_matmul;
pub mod identifier_validators;
mod mesc_verify;
mod mlx_layout;
mod onnx_optional_inputs;
mod onnx_runtime;
mod pack_runtime;
mod pii_merge;
mod residency;
mod runtime;
mod store;
mod token_windows;
mod variant_selection;

pub use asr_runtime::{
    ASR_SOURCES, AsrRuntimeError, CANDLE_WHISPER_ASR_RUNTIME_ID, MAX_ASR_SAMPLES,
    PreparedWhisperAsr, mel_filters, pcm16_to_mono_f32, prepare_whisper_asr,
    prepare_whisper_from_bytes,
};
pub use candle_runtime::{
    CANDLE_ARCHITECTURES, CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID, CandleDevicePreference,
    CandleRunOptions, CandleRuntimeError, PreparedCandleTokenClassifier, candle_model_type,
    prepare_candle_token_classifier, prepare_candle_token_classifier_with,
};
pub use catalog::{
    CatalogError, CatalogFilter, CatalogPage, CatalogRow, CatalogSource, CatalogStatus, DeviceFit,
    ModelCatalog, SizeMb,
};
pub use compatibility::{
    DEGENERATE_TOKENIZER_REASON, EVIDENCED_ARCHITECTURES, EXECUTED_MODELS, ExecutedModel,
    KNOWN_DEFECTIVE_EXPORTS, KNOWN_UNSUPPORTED, KNOWN_UNSUPPORTED_FAMILIES, RuntimeExpectation,
    SAFETENSORS_EVIDENCED_ARCHITECTURES, SAFETENSORS_KNOWN_UNSUPPORTED, executed_model,
    runtime_expectation,
};
pub use format::{admit_pack_dir, forbidden_reason};
#[cfg(feature = "synthetic-fixtures")]
#[doc(hidden)]
pub use generation_runtime::fixtures as generation_fixtures;
pub use generation_runtime::{
    CANDLE_TEXT_GENERATION_RUNTIME_ID, GENERATION_ARCHITECTURES, GenerationRuntimeError,
    MAX_NEW_TOKENS, PreparedTextGenerator, StopReason, prepare_text_generator,
};
pub use gliner_runtime::{
    GLINER_DEFAULT_THRESHOLD, GLINER_SPAN_FAMILY, GLINER_ZERO_SHOT_RUNTIME_ID, GlinerRuntimeError,
    PreparedGlinerSpanModel, ZeroShotEntity, is_gliner_span_export, pack_is_gliner_span,
    prepare_gliner_span_model,
};
pub use hf_snapshot::{
    HfRepoMetadata, HfSibling, SnapshotError, TextGenerationSnapshot, TokenClassifierSnapshot,
    TokenizerSource, build_asr_pack, build_text_generation_pack, build_token_classifier_pack,
    labels_from_config, reproducibility_hash, snapshot_tokenizer_source, tokenizer_source_repo,
    verify_against_catalog, verify_file,
};
pub use mesc_verify::{MescEpochStore, MescVerifyError, verify_mesc_release_dir};
pub use mlx_layout::{is_mlx_export, mlx_key};
pub use onnx_runtime::{
    DocumentRunOptions, ONNX_TOKEN_CLASSIFIER_RUNTIME_ID, OnnxRuntimeError,
    OnnxTokenClassifierRuntime, PlanCacheOutcome, PrepareStageTimings, PreparedOnnxTokenClassifier,
};
pub use pack_runtime::{
    PackEvaluation, PackRuntimeKind, evaluate_admitted_pack, pack_runtime_kind,
};
pub use pii_merge::{
    ClinicalNormalization, INDIAN_CLINICAL_ABBREVIATIONS, INDIAN_CLINICAL_NORMALIZATION_VERSION,
    IndiaClinicalMerge, PiiEntity, PiiMergeOptions, SemanticUnit, find_semantic_units,
    is_more_specific, merge_entities_with_semantic_units, merge_india_code_mixed_spans,
    normalize_for_detection, normalize_indian_clinical_entities, normalize_indian_clinical_surface,
    normalize_label,
};
pub use residency::{EvictionPlan, ResidencyError, ResidencyPool};
pub use runtime::{FixtureRuntime, PackRuntimeAdapter, RuntimeOutput};
pub use store::PackStore;
pub use token_windows::{
    BestContextVotes, CharSpan, DecodedEntity, EntityAggregation, MAX_WINDOWS, TokenLogits,
    TokenWindow, WindowError, decode_entities, decode_entities_by_word, default_stride,
    plan_windows,
};
pub use variant_selection::{
    HostPlatform, VariantChoice, VariantRuntime, rank_variants, variant_family,
};

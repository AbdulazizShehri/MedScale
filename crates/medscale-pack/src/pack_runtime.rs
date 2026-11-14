//! Runtime selection for admitted token-classification Packs (Spec 104).
//!
//! A Pack declares its runtime in `runtime_requirements`. Callers that run a
//! Pack once (for example MedAgent / Model Fleet lanes) use
//! [`evaluate_admitted_pack`], which dispatches to the tract ONNX runtime
//! (Spec 069/103) or the candle safetensors runtime (Spec 104). Nothing is
//! selected by guessing from file names: an unknown declaration is refused.

use std::path::Path;

use medscale_contracts::packs::{ModelSourceProvenance, PackManifestV0};

use crate::RuntimeOutput;
use crate::candle_runtime::{
    CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID, CandleRunOptions, prepare_candle_token_classifier,
};
use crate::onnx_runtime::{ONNX_TOKEN_CLASSIFIER_RUNTIME_ID, OnnxTokenClassifierRuntime};

/// The runtime a Pack declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackRuntimeKind {
    TractOnnx,
    CandleSafetensors,
}

impl PackRuntimeKind {
    #[must_use]
    pub const fn runtime_id(self) -> &'static str {
        match self {
            Self::TractOnnx => ONNX_TOKEN_CLASSIFIER_RUNTIME_ID,
            Self::CandleSafetensors => CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID,
        }
    }
}

/// Reads the runtime declaration; exactly one known runtime must be declared.
pub fn pack_runtime_kind(manifest: &PackManifestV0) -> Option<PackRuntimeKind> {
    let declared = |id: &str| manifest.runtime_requirements.split(';').any(|r| r == id);
    match (
        declared(ONNX_TOKEN_CLASSIFIER_RUNTIME_ID),
        declared(CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID),
    ) {
        (true, false) => Some(PackRuntimeKind::TractOnnx),
        (false, true) => Some(PackRuntimeKind::CandleSafetensors),
        _ => None,
    }
}

/// One evaluation of an admitted Pack.
#[derive(Debug, Clone)]
pub struct PackEvaluation {
    pub output: RuntimeOutput,
    pub provenance: ModelSourceProvenance,
    pub runtime: PackRuntimeKind,
}

/// Prepares and runs an admitted Pack once with its declared runtime.
/// `max_tokens` bounds the tract static window; the candle runtime runs whole
/// documents through its own windowing.
pub fn evaluate_admitted_pack(
    pack_dir: &Path,
    manifest: &PackManifestV0,
    input: &str,
    max_tokens: usize,
) -> Result<PackEvaluation, String> {
    match pack_runtime_kind(manifest) {
        Some(PackRuntimeKind::TractOnnx) => {
            let runtime = OnnxTokenClassifierRuntime::new(max_tokens)
                .map_err(|e| format!("pack runtime configuration denied: {e}"))?;
            let evaluation = runtime
                .run(pack_dir, manifest, input)
                .map_err(|e| format!("pack runtime evaluation failed: {e}"))?;
            Ok(PackEvaluation {
                output: evaluation.output,
                provenance: evaluation.provenance,
                runtime: PackRuntimeKind::TractOnnx,
            })
        }
        Some(PackRuntimeKind::CandleSafetensors) => {
            let prepared = prepare_candle_token_classifier(pack_dir, manifest)
                .map_err(|e| format!("pack runtime preparation failed: {e}"))?;
            let output = prepared
                .run_document(&manifest.pack_id, input, CandleRunOptions::default())
                .map_err(|e| format!("pack runtime evaluation failed: {e}"))?;
            Ok(PackEvaluation {
                output,
                provenance: prepared.provenance().clone(),
                runtime: PackRuntimeKind::CandleSafetensors,
            })
        }
        None => Err("pack declares no single admitted runtime".to_owned()),
    }
}

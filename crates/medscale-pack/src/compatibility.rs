//! Pre-download runtime compatibility for catalog rows (Spec 103, T103-09).
//!
//! This is an **expectation** derived from recorded execution evidence for the
//! local `tract_onnx_token_classification_v1` runtime
//! (`specs/103-local-model-catalog/research.md` sections 9–10). It is never a
//! status promotion: a row only becomes `RUNTIME_COMPATIBLE` / `EXECUTED_TESTED`
//! through its own evidence.

use serde::Serialize;

use crate::catalog::CatalogRow;

/// Architectures with at least one OpenMed v3.0.0 model executed end to end
/// (catalog → verified snapshot → signed Pack → admission → run). DeBERTa-v2
/// and XLM-R were added with qualification run 37977100903 (XLM-R through its
/// fp16 export, widened to fp32 at load).
pub const EVIDENCED_ARCHITECTURES: &[&str] = &[
    "bert",
    "distilbert",
    "roberta",
    "modernbert",
    "deberta-v2",
    "xlm-roberta",
    "eurobert",
];

/// One catalog row executed end to end on a real model: acquisition at the
/// catalog-pinned revision, verification, signed Pack, admission and run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExecutedModel {
    pub repo_id: &'static str,
    /// Hub commit the evidence ran against.
    pub revision: &'static str,
    /// `tract_onnx`, `candle_safetensors`, `candle_mlx` or `candle_gliner`.
    pub runtime: &'static str,
    pub weight_file: &'static str,
    /// GitHub Actions run id of the qualification evidence.
    pub evidence_run: &'static str,
}

/// Per-model execution evidence (rows, not architectures). Synthetic text
/// only: this records that the model runs, not task quality.
pub const EXECUTED_MODELS: &[ExecutedModel] = &[
    // Clinical Longformer on the exact candle local-window path; entities
    // match Hugging Face LongformerForTokenClassification (run 38300309054).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-ClinicalLongformer-Base-149M-v1",
        revision: "ade4412daadbae377eda99f73e107557d15d31c0",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38300309054",
    },
    // The remaining QwenMed PII rows, int8 export (run 38260759126).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "b23b67ba6797bd57fe43ab5c74dbb02370261970",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-French-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "ab281a758ae9713c53e1b230342ef52e5ab4e053",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-German-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "236025090d28429f45664edd8b43bc8fd4903511",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Hindi-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "8eb77f446e812328a8c461526e03a2f195f7216e",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Italian-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "cb463fd8f72a1340fbf81f49be9adb4c50c17ebf",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Japanese-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "2652ad2ff21b8ff9bed70028617a5e034dd618d9",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Korean-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "d15e599e481265938fd8e49a8a10b4ae4ca91f16",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Portuguese-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "b9b80f6b92aa18d5726de026b0555c6bdf34daa3",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "e47526af75653006cb2ae0200dd271c733dacba0",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "e4c4c8aa8c55089bfa26b337dc8257356031e58f",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Telugu-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "19ad9f710bb9d442a745abbfea7030115c7fd187",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Turkish-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "d796a5e3366f65fa26e4a857bbfc010f800b7213",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38260759126",
    },
    // Catalog rows without an `architecture` field; their `config.json`
    // declares bert, roberta, deberta-v2 or modernbert (run 38163293053).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M",
        revision: "49bedb870db7008773880b643733c78100737e12",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ChemicalDetect-SuperMedical-355M",
        revision: "ee4f7b5d9e41a005f35a9ed033a77de71c68e4e5",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-GenomicDetect-SuperClinical-184M",
        revision: "3d01b67e46890c10540daa00b57693e6238ebb46",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-OncologyDetect-BioPatient-108M",
        revision: "6ffc399d133a28adc6b9a835f351bc2e6f8fc73c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-ProteinDetect-PubMed-v2-109M",
        revision: "f3877242a44ac750054eafc097bc92e55368b6a4",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-SpeciesDetect-ModernMed-395M",
        revision: "8e126e50c965925235b26401a44e865d6c2fea18",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "38163293053",
    },
    // GLiNER zero-shot MLX exports (Spec 104), run with synthetic labels.
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-ZeroShot-NER-Anatomy-Small-166M-mlx",
        revision: "02a3c8ba849c2ddcd5f994ce882df2418abbd02e",
        runtime: "candle_gliner",
        weight_file: "weights.safetensors",
        evidence_run: "38161626788",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-ZeroShot-NER-Disease-Medium-209M-mlx",
        revision: "0c3d802dcba6c88afa3fd6b6451216b4d56c1b1a",
        runtime: "candle_gliner",
        weight_file: "weights.safetensors",
        evidence_run: "38163323960",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-EuroMed-Large-210M-v1-onnx-android",
        revision: "6f15663a6e6f863792f9ad7ef1725552c703dabb",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "38152456058",
    },
    // Only the int8 export fits the 1 GiB artifact bound (model.onnx keeps
    // its weights in external data; model_fp16.onnx is 1.19 GB).
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Chinese-QwenMed-XLarge-600M-v1-onnx-android",
        revision: "0f9550e49fbf1d90d9136879be1a635b5e9728f2",
        runtime: "tract_onnx",
        weight_file: "model_int8.onnx",
        evidence_run: "38152456058",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-DiseaseDetect-ElectraMed-33M-v1-onnx-android",
        revision: "54c9cc119d325ddad23300a1e9108bafa9643a06",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-v1-onnx-android",
        revision: "ad5e79cb87caf49d9ecc28da4de008856527ab63",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M-v1-onnx-android",
        revision: "95f0bf742c8c31d24b66190e17c1add82664482c",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-149M-v1-onnx-android",
        revision: "66525717fb59d0ff6e1396b7c2d1ab8c1e4a93c8",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1-onnx-android",
        revision: "e41c8009026fdd98dae146cb67ae9b16673faf49",
        runtime: "tract_onnx",
        weight_file: "model.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Spanish-BigMed-Large-278M-v1-onnx-android",
        revision: "da3fec609614401bba850912065c832a74586d5a",
        runtime: "tract_onnx",
        weight_file: "model_fp16.onnx",
        evidence_run: "37984399447",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M",
        revision: "3c914174cf099fda326b43084f679ad31425f88c",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M",
        revision: "1ab9359dada59aafc76bf63879c4984a5fa3490e",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M",
        revision: "724a3702f1fcc80ab0bd695f1a3e3ddf2ac854f1",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-PII-Dutch-SuperClinical-Small-44M-v1",
        revision: "ee9cafadbe5689d779f4ae6971d30eb348274b82",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ModernClinical-149M",
        revision: "3c0bdcc332af0fc54b8f945bcfbebdd375eb69dd",
        runtime: "candle_safetensors",
        weight_file: "model.safetensors",
        evidence_run: "37986892863",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M-mlx",
        revision: "70bcf3d3c4e37781723795830772d4a4ad8f2f1c",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38039625442",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-mlx",
        revision: "1be81258e117a95f41600bd2b0b5684581a5714a",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38039625442",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-82M-mlx",
        revision: "30ae97175384e04e2c961ab6dc6af5bc0342d3ce",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38039625442",
    },
    ExecutedModel {
        repo_id: "OpenMed/OpenMed-NER-AnatomyDetect-SuperClinical-141M-mlx",
        revision: "8d9d9946e0b291bffb505a59b7f73c15ef4a92be",
        runtime: "candle_mlx",
        weight_file: "weights.safetensors",
        evidence_run: "38041300631",
    },
];

/// Recorded execution evidence for one catalog row, if any.
#[must_use]
pub fn executed_model(repo_id: &str) -> Option<&'static ExecutedModel> {
    EXECUTED_MODELS.iter().find(|m| m.repo_id == repo_id)
}

/// Architectures recorded as not supported by the current runtime, with the cause.
pub const KNOWN_UNSUPPORTED: &[(&str, &str)] = &[];

/// Architectures executed end to end through the native safetensors runtime
/// (Spec 104, qualification run 37986892863) from PyTorch-format rows.
pub const SAFETENSORS_EVIDENCED_ARCHITECTURES: &[&str] =
    &["bert", "distilbert", "roberta", "deberta-v2", "modernbert"];

/// Architectures executed from OpenMed MLX `mlx-fp` exports (renamed-key
/// safetensors, `mlx_layout.rs`): qualification runs 38039625442 (bert,
/// distilbert, roberta) and 38041300631 (deberta-v2). Repositories that ship
/// no `tokenizer.json` are refused at acquisition.
pub const MLX_EVIDENCED_ARCHITECTURES: &[&str] = &["bert", "distilbert", "roberta", "deberta-v2"];

/// PyTorch-format architectures known not to fit the safetensors path.
pub const SAFETENSORS_KNOWN_UNSUPPORTED: &[(&str, &str)] = &[
    (
        "xlm-roberta",
        "PyTorch-format XLM-R weights (1.11 GB for the 278M-parameter models) exceed the 1 GiB Pack bound",
    ),
    (
        "gliner",
        "PyTorch GLiNER repositories ship pickle weights only (pytorch_model.bin), which are refused; the ZeroShot MLX exports run",
    ),
    (
        "eurobert",
        "the candle runtime has no EuroBERT encoder; the ONNX exports run on tract",
    ),
    (
        "qwen",
        "the candle runtime has no Qwen token-classification encoder; the int8 ONNX export runs on tract",
    ),
];

/// Architecture families known not to run on any admitted runtime, matched by
/// a substring of the catalog `architecture` (OpenMed names some families per
/// repository, for example `openmed-pii-clinicallongformer-base-149m-v1`).
pub const KNOWN_UNSUPPORTED_FAMILIES: &[(&str, &str)] = &[
    (
        "longformer",
        "Longformer ONNX export: tract 0.22.4 fails in Gather at 512 and 1024 tokens, after the omitted Pad input is dropped (run 38172472172); the PyTorch-format rows run on candle (exact local-window path, run 38300309054)",
    ),
    (
        "privacy-filter",
        "privacy-filter weights are 1.49 GB (8-bit MLX, quantized) to 2.80 GB, over the 1 GiB Pack bound",
    ),
];

/// Published exports known to be defective, matched by architecture and a
/// repository-id prefix, with the cause.
pub const KNOWN_DEFECTIVE_EXPORTS: &[(&str, &str, &str)] = &[(
    "xlm-roberta",
    "OpenMed/OpenMed-NER-",
    "the published tokenizer.json is a BPE model with zero merges, so text is split into single characters; the runtime refuses it",
)];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeExpectation {
    /// This exact row was executed end to end on its real weights (see
    /// [`EXECUTED_MODELS`]); evidence, not task qualification.
    ExecutedTested {
        runtime: String,
        revision: String,
        weight_file: String,
        evidence_run: String,
    },
    /// Same task, an ONNX file, and an architecture with recorded execution
    /// evidence. Expected to run; still unverified for this row.
    ExpectedRunnable,
    /// Known not to run on the current runtime.
    KnownUnsupported { reason: String },
    /// No ONNX file, but PyTorch-format weights for an architecture with
    /// recorded native safetensors execution evidence (Spec 104). Expected to
    /// run; still unverified for this row (its weights may be pickle-only,
    /// which acquisition refuses).
    ExpectedRunnableSafetensors,
    /// OpenMed zero-shot MLX export (GLiNER uni-encoder span, Spec 104) in a
    /// tier whose weights fit the 1 GiB Pack bound. Expected to run through
    /// the GLiNER runtime with caller-supplied labels; still unverified for
    /// this row.
    ExpectedRunnableZeroShot,
    /// No runnable artifact is listed (for example MLX only).
    NoOnnxArtifact,
    /// The task is not token classification.
    UnsupportedTask { task: String },
    /// ONNX token classifier with an architecture not yet tried.
    Untested,
}

/// Expectation for `row` on the local ONNX token-classification runtime.
/// Expectation for an OpenMed MLX repository (no ONNX, MLX weights only).
fn mlx_expectation(row: &CatalogRow, arch: &str) -> RuntimeExpectation {
    let full_precision = row.formats.iter().any(|f| f.eq_ignore_ascii_case("mlx-fp"));
    if !full_precision {
        return RuntimeExpectation::KnownUnsupported {
            reason: "quantized MLX exports change tensor contents and are not supported".into(),
        };
    }
    // Zero-shot repositories use GLiNER span heads (Spec 104). In OpenMed
    // v3.0.0 the Small (611 MB) and Medium (781 MB) exports fit the Pack
    // bound; the untiered Base (1.16 GB) and Large (1.78 GB) ones do not.
    if row.repo_id.contains("ZeroShot") {
        if arch != "deberta-v2" {
            return RuntimeExpectation::Untested;
        }
        return match row.tier.as_deref() {
            Some(t) if t.eq_ignore_ascii_case("small") || t.eq_ignore_ascii_case("medium") => {
                RuntimeExpectation::ExpectedRunnableZeroShot
            }
            _ => RuntimeExpectation::KnownUnsupported {
                reason: "zero-shot MLX exports outside the Small and Medium tiers are 1.16-1.78 GB, over the 1 GiB Pack bound".into(),
            },
        };
    }
    if let Some((_, reason)) = SAFETENSORS_KNOWN_UNSUPPORTED
        .iter()
        .find(|(a, _)| *a == arch)
    {
        return RuntimeExpectation::KnownUnsupported {
            reason: (*reason).to_string(),
        };
    }
    if MLX_EVIDENCED_ARCHITECTURES.contains(&arch) {
        return RuntimeExpectation::ExpectedRunnableSafetensors;
    }
    RuntimeExpectation::Untested
}

pub fn runtime_expectation(row: &CatalogRow) -> RuntimeExpectation {
    if row.task != "token-classification" {
        return RuntimeExpectation::UnsupportedTask {
            task: row.task.clone(),
        };
    }
    if let Some(model) = executed_model(&row.repo_id) {
        return RuntimeExpectation::ExecutedTested {
            runtime: model.runtime.to_owned(),
            revision: model.revision.to_owned(),
            weight_file: model.weight_file.to_owned(),
            evidence_run: model.evidence_run.to_owned(),
        };
    }
    let arch = row.architecture.to_ascii_lowercase();
    let has_onnx = row.formats.iter().any(|f| f.eq_ignore_ascii_case("onnx"));
    // Longformer: the ONNX export is blocked on tract, but PyTorch-format
    // rows run on the exact candle local-window path (Spec 103 T103-19).
    if arch.contains("longformer") && !has_onnx {
        return RuntimeExpectation::ExpectedRunnableSafetensors;
    }
    if let Some((_, reason)) = KNOWN_UNSUPPORTED_FAMILIES
        .iter()
        .find(|(family, _)| arch.contains(family))
    {
        return RuntimeExpectation::KnownUnsupported {
            reason: (*reason).to_string(),
        };
    }
    if !row.formats.iter().any(|f| f.eq_ignore_ascii_case("onnx")) {
        // OpenMed v3.0.0 lists MLX repositories (`*-mlx`) with formats such as
        // `["mlx-fp", "pytorch"]`; they publish MLX weights only
        // (`weights.safetensors` with renamed keys, see `mlx_layout`).
        let mlx = row
            .formats
            .iter()
            .any(|f| f.to_ascii_lowercase().starts_with("mlx"));
        if mlx {
            return mlx_expectation(row, &arch);
        }
        if row
            .formats
            .iter()
            .any(|f| f.eq_ignore_ascii_case("pytorch"))
        {
            if let Some((_, reason)) = SAFETENSORS_KNOWN_UNSUPPORTED
                .iter()
                .find(|(a, _)| *a == arch)
            {
                return RuntimeExpectation::KnownUnsupported {
                    reason: (*reason).to_string(),
                };
            }
            if SAFETENSORS_EVIDENCED_ARCHITECTURES.contains(&arch.as_str()) {
                return RuntimeExpectation::ExpectedRunnableSafetensors;
            }
            return RuntimeExpectation::Untested;
        }
        return RuntimeExpectation::NoOnnxArtifact;
    }
    if let Some((_, _, reason)) = KNOWN_DEFECTIVE_EXPORTS
        .iter()
        .find(|(a, prefix, _)| *a == arch && row.repo_id.starts_with(prefix))
    {
        return RuntimeExpectation::KnownUnsupported {
            reason: (*reason).to_string(),
        };
    }
    if let Some((_, reason)) = KNOWN_UNSUPPORTED.iter().find(|(a, _)| *a == arch) {
        return RuntimeExpectation::KnownUnsupported {
            reason: (*reason).to_string(),
        };
    }
    if EVIDENCED_ARCHITECTURES.contains(&arch.as_str()) {
        return RuntimeExpectation::ExpectedRunnable;
    }
    RuntimeExpectation::Untested
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ModelCatalog;

    const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";

    fn row(task: &str, arch: &str, formats: &str) -> CatalogRow {
        named("Org/m", task, arch, formats)
    }

    fn named(repo: &str, task: &str, arch: &str, formats: &str) -> CatalogRow {
        let line = format!(
            r#"{{"repo_id":"{repo}","family":"NER","task":"{task}","architecture":"{arch}","formats":{formats},"license":"apache-2.0"}}"#
        );
        ModelCatalog::import("s/f", COMMIT, line.as_bytes())
            .unwrap()
            .get(repo)
            .unwrap()
            .clone()
    }

    #[test]
    fn expectations_follow_recorded_evidence() {
        assert_eq!(
            runtime_expectation(&row("token-classification", "bert", r#"["onnx"]"#)),
            RuntimeExpectation::ExpectedRunnable
        );
        assert_eq!(
            runtime_expectation(&row(
                "token-classification",
                "ModernBERT",
                r#"["pytorch","onnx"]"#
            )),
            RuntimeExpectation::ExpectedRunnable
        );
        assert_eq!(
            runtime_expectation(&row("token-classification", "deberta-v2", r#"["onnx"]"#)),
            RuntimeExpectation::ExpectedRunnable
        );
        assert_eq!(
            runtime_expectation(&named(
                "OpenMed/OpenMed-PII-French-BigMed-Large-278M-v1-onnx-android",
                "token-classification",
                "xlm-roberta",
                r#"["onnx"]"#
            )),
            RuntimeExpectation::ExpectedRunnable
        );
        assert!(matches!(
            runtime_expectation(&named(
                "OpenMed/OpenMed-NER-DiseaseDetect-BigMed-278M-v1-onnx-android",
                "token-classification",
                "xlm-roberta",
                r#"["onnx"]"#
            )),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        assert_eq!(
            runtime_expectation(&row("token-classification", "bert", r#"["mlx-fp"]"#)),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert_eq!(
            runtime_expectation(&row("token-classification", "deberta-v2", r#"["pytorch"]"#)),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert!(matches!(
            runtime_expectation(&row(
                "token-classification",
                "xlm-roberta",
                r#"["pytorch"]"#
            )),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        // Pickle-only GLiNER, and encoders the candle runtime lacks.
        for arch in ["gliner", "eurobert", "qwen"] {
            assert!(
                matches!(
                    runtime_expectation(&row("token-classification", arch, r#"["pytorch"]"#)),
                    RuntimeExpectation::KnownUnsupported { .. }
                ),
                "{arch}"
            );
        }
        assert_eq!(
            runtime_expectation(&row(
                "token-classification",
                "bert",
                r#"["mlx-fp","pytorch"]"#
            )),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert!(matches!(
            runtime_expectation(&row("token-classification", "bert", r#"["mlx-8bit"]"#)),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        assert!(matches!(
            runtime_expectation(&row("token-classification", "xlm-roberta", r#"["mlx-fp"]"#)),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
        assert_eq!(
            runtime_expectation(&row("token-classification", "modernbert", r#"["mlx-fp"]"#)),
            RuntimeExpectation::Untested
        );
        assert_eq!(
            runtime_expectation(&row("token-classification", "gliner", r#"["onnx"]"#)),
            RuntimeExpectation::Untested
        );
        assert_eq!(
            runtime_expectation(&row("text-generation", "qwen", r#"["onnx"]"#)),
            RuntimeExpectation::UnsupportedTask {
                task: "text-generation".into()
            }
        );
    }

    #[test]
    fn longformer_pytorch_rows_run_on_candle_while_onnx_rows_stay_blocked() {
        assert_eq!(
            runtime_expectation(&row(
                "token-classification",
                "openmed-pii-dutch-clinicallongformer-base-149m-v1",
                r#"["pytorch"]"#
            )),
            RuntimeExpectation::ExpectedRunnableSafetensors
        );
        assert!(matches!(
            runtime_expectation(&row(
                "token-classification",
                "openmed-pii-dutch-clinicallongformer-base-149m-v1",
                r#"["onnx"]"#
            )),
            RuntimeExpectation::KnownUnsupported { .. }
        ));
    }

    #[test]
    fn known_unsupported_families_match_per_repository_architecture_names() {
        for (arch, formats) in [
            ("openmed-pii-clinicallongformer-base-149m-v1", r#"["onnx"]"#),
            ("clinical-longformer", r#"["onnx"]"#),
            ("privacy-filter-multilingual-v2", r#"["mlx-fp","pytorch"]"#),
        ] {
            assert!(
                matches!(
                    runtime_expectation(&row("token-classification", arch, formats)),
                    RuntimeExpectation::KnownUnsupported { .. }
                ),
                "{arch}"
            );
        }
        // A catalog row without an architecture that was executed reports it.
        let r = named(
            "OpenMed/OpenMed-NER-AnatomyDetect-BioMed-109M",
            "token-classification",
            "",
            r#"["pytorch"]"#,
        );
        assert!(matches!(
            runtime_expectation(&r),
            RuntimeExpectation::ExecutedTested { .. }
        ));
    }

    #[test]
    fn zero_shot_mlx_rows_follow_tier_size_evidence() {
        let zs = |repo: &str, tier: &str| {
            let tier = if tier.is_empty() {
                "null".to_owned()
            } else {
                format!("\"{tier}\"")
            };
            let line = format!(
                r#"{{"repo_id":"{repo}","family":"ZeroShot","task":"token-classification","tier":{tier},"architecture":"deberta-v2","formats":["mlx-fp","pytorch"],"license":"apache-2.0"}}"#
            );
            let catalog = ModelCatalog::import("s/f", COMMIT, line.as_bytes()).unwrap();
            runtime_expectation(catalog.get(repo).unwrap())
        };
        assert_eq!(
            zs(
                "OpenMed/OpenMed-ZeroShot-NER-Pharma-Small-166M-mlx",
                "Small"
            ),
            RuntimeExpectation::ExpectedRunnableZeroShot
        );
        assert_eq!(
            zs(
                "OpenMed/OpenMed-ZeroShot-NER-Pharma-Medium-209M-mlx",
                "Medium"
            ),
            RuntimeExpectation::ExpectedRunnableZeroShot
        );
        for tier in ["Large", ""] {
            assert!(matches!(
                zs("OpenMed/OpenMed-ZeroShot-NER-Pharma-X-mlx", tier),
                RuntimeExpectation::KnownUnsupported { .. }
            ));
        }
        assert!(matches!(
            zs(
                "OpenMed/OpenMed-ZeroShot-NER-Disease-Medium-209M-mlx",
                "Medium"
            ),
            RuntimeExpectation::ExecutedTested { .. }
        ));
    }

    #[test]
    fn executed_rows_report_their_own_evidence_first() {
        let r = named(
            "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-33M-mlx",
            "token-classification",
            "bert",
            r#"["mlx-fp","pytorch"]"#,
        );
        match runtime_expectation(&r) {
            RuntimeExpectation::ExecutedTested {
                runtime,
                revision,
                evidence_run,
                ..
            } => {
                assert_eq!(runtime, "candle_mlx");
                assert_eq!(revision.len(), 40);
                assert_eq!(evidence_run, "38039625442");
            }
            other => panic!("{other:?}"),
        }
        let mut repos: Vec<&str> = EXECUTED_MODELS.iter().map(|m| m.repo_id).collect();
        repos.sort_unstable();
        repos.dedup();
        assert_eq!(repos.len(), EXECUTED_MODELS.len(), "one entry per row");
        for m in EXECUTED_MODELS {
            assert_eq!(m.revision.len(), 40, "{}", m.repo_id);
            assert!(m.revision.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(m.evidence_run.bytes().all(|b| b.is_ascii_digit()));
        }
    }

    #[test]
    fn expectation_never_changes_the_row_status() {
        let r = row("token-classification", "bert", r#"["onnx"]"#);
        let before = r.status;
        let _ = runtime_expectation(&r);
        assert_eq!(r.status, before);
        assert!(before <= crate::catalog::CatalogStatus::RightsPending);
    }
}

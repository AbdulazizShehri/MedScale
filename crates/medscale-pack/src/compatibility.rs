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
];

/// One catalog row executed end to end on a real model: acquisition at the
/// catalog-pinned revision, verification, signed Pack, admission and run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExecutedModel {
    pub repo_id: &'static str,
    /// Hub commit the evidence ran against.
    pub revision: &'static str,
    /// `tract_onnx`, `candle_safetensors` or `candle_mlx`.
    pub runtime: &'static str,
    pub weight_file: &'static str,
    /// GitHub Actions run id of the qualification evidence.
    pub evidence_run: &'static str,
}

/// Per-model execution evidence (rows, not architectures). Synthetic text
/// only: this records that the model runs, not task quality.
pub const EXECUTED_MODELS: &[ExecutedModel] = &[
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
pub const SAFETENSORS_KNOWN_UNSUPPORTED: &[(&str, &str)] = &[(
    "xlm-roberta",
    "PyTorch-format XLM-R weights (1.11 GB for the 278M-parameter models) exceed the 1 GiB Pack bound",
)];

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
    // Zero-shot repositories use span heads, not token-classification heads.
    if row.repo_id.contains("ZeroShot") {
        return RuntimeExpectation::Untested;
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
        assert_eq!(
            runtime_expectation(&row("token-classification", "gliner", r#"["pytorch"]"#)),
            RuntimeExpectation::Untested
        );
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

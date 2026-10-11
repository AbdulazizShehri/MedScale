//! Runtime-aware choice among a model's catalog variants (Spec 103).
//!
//! OpenMed v3.0.0 picks a backend at load time by availability
//! (`core/backends.py` `get_backend`: MLX, then Hugging Face, then ONNX). Each
//! OpenMed model is listed as sibling repositories that differ only in format
//! (`…-65M` PyTorch, `…-65M-mlx`, `…-65M-v1-onnx-android`). MedScale chooses
//! *before* any download, from catalog metadata and recorded evidence:
//! - variants are grouped by their repository id with the format suffix and a
//!   trailing `-v1` removed (`-v2` stays a different model);
//! - only variants whose runtime expectation says they can run are offered;
//! - Apple silicon prefers MLX `mlx-fp` (candle on Metal), then safetensors,
//!   then ONNX; other hosts prefer safetensors (candle prepares in seconds),
//!   then ONNX (tract), then MLX (candle on CPU);
//! - within a format, a variant with recorded execution evidence comes first.
//!
//! The result is advice for acquisition. It never downloads, and the chosen
//! Pack still passes verification and admission.

use crate::catalog::{CatalogRow, ModelCatalog};
use crate::compatibility::{RuntimeExpectation, runtime_expectation};

/// Host class for the preference order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostPlatform {
    /// macOS on Apple silicon (candle Metal).
    AppleSilicon,
    /// Any other host (CPU).
    Other,
}

impl HostPlatform {
    /// The platform this binary runs on.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Self::AppleSilicon
        } else {
            Self::Other
        }
    }
}

/// The runtime path a variant would use.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantRuntime {
    /// OpenMed MLX `mlx-fp` export on candle (Metal on Apple silicon).
    CandleMlx,
    /// PyTorch-format safetensors on candle.
    CandleSafetensors,
    /// ONNX on tract.
    TractOnnx,
}

/// One runnable variant, best first.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct VariantChoice {
    pub repo_id: String,
    pub runtime: VariantRuntime,
    /// The variant row has recorded execution evidence.
    pub executed: bool,
    pub expectation: RuntimeExpectation,
}

const ONNX_SUFFIXES: &[&str] = &["-onnx-android", "-onnx-webgpu", "-onnx"];

/// The model-family key shared by a model's format variants.
#[must_use]
pub fn variant_family(repo_id: &str) -> String {
    let mut key = repo_id;
    // `-mlx`, and quantized `-mlx-8bit` / `-mlx-q8` exports.
    if let Some(i) = key.rfind("-mlx") {
        let tail = &key[i + 4..];
        if tail.is_empty() || (tail.starts_with('-') && !tail[1..].contains('-')) {
            key = &key[..i];
        }
    }
    for suffix in ONNX_SUFFIXES {
        if let Some(stripped) = key.strip_suffix(suffix) {
            key = stripped;
            break;
        }
    }
    key.strip_suffix("-v1").unwrap_or(key).to_owned()
}

fn runtime_of(row: &CatalogRow, expectation: &RuntimeExpectation) -> Option<VariantRuntime> {
    match expectation {
        RuntimeExpectation::ExecutedTested { runtime, .. } => match runtime.as_str() {
            "tract_onnx" => Some(VariantRuntime::TractOnnx),
            "candle_mlx" => Some(VariantRuntime::CandleMlx),
            "candle_safetensors" => Some(VariantRuntime::CandleSafetensors),
            _ => None,
        },
        RuntimeExpectation::ExpectedRunnable => Some(VariantRuntime::TractOnnx),
        RuntimeExpectation::ExpectedRunnableSafetensors => {
            if row.formats.iter().any(|f| f.eq_ignore_ascii_case("mlx-fp")) {
                Some(VariantRuntime::CandleMlx)
            } else {
                Some(VariantRuntime::CandleSafetensors)
            }
        }
        _ => None,
    }
}

const fn preference(host: HostPlatform, runtime: VariantRuntime) -> u8 {
    match (host, runtime) {
        (HostPlatform::AppleSilicon, VariantRuntime::CandleMlx)
        | (HostPlatform::Other, VariantRuntime::CandleSafetensors) => 0,
        (HostPlatform::AppleSilicon, VariantRuntime::CandleSafetensors)
        | (HostPlatform::Other, VariantRuntime::TractOnnx) => 1,
        (HostPlatform::AppleSilicon, VariantRuntime::TractOnnx)
        | (HostPlatform::Other, VariantRuntime::CandleMlx) => 2,
    }
}

/// Runnable token-classification variants of `repo_id`'s model family, best
/// first for `host`. Zero-shot rows (GLiNER runtime) are not ranked here.
#[must_use]
pub fn rank_variants(
    catalog: &ModelCatalog,
    repo_id: &str,
    host: HostPlatform,
) -> Vec<VariantChoice> {
    let family = variant_family(repo_id);
    let mut choices: Vec<VariantChoice> = catalog
        .rows()
        .iter()
        .filter(|row| variant_family(&row.repo_id) == family)
        .filter_map(|row| {
            let expectation = runtime_expectation(row);
            let runtime = runtime_of(row, &expectation)?;
            Some(VariantChoice {
                repo_id: row.repo_id.clone(),
                runtime,
                executed: matches!(expectation, RuntimeExpectation::ExecutedTested { .. }),
                expectation,
            })
        })
        .collect();
    choices.sort_by(|a, b| {
        preference(host, a.runtime)
            .cmp(&preference(host, b.runtime))
            .then(b.executed.cmp(&a.executed))
            .then_with(|| a.repo_id.cmp(&b.repo_id))
    });
    choices
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";

    fn catalog(rows: &[(&str, &str, &str)]) -> ModelCatalog {
        let lines: Vec<String> = rows
            .iter()
            .map(|(repo, arch, formats)| {
                format!(
                    r#"{{"repo_id":"{repo}","family":"NER","task":"token-classification","architecture":"{arch}","formats":{formats},"license":"apache-2.0"}}"#
                )
            })
            .collect();
        ModelCatalog::import("s/f", COMMIT, lines.join("\n").as_bytes()).unwrap()
    }

    #[test]
    fn families_group_format_variants_but_keep_v2_models_apart() {
        for repo in [
            "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M",
            "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-mlx",
            "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-v1-onnx-android",
            "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-v1-onnx-webgpu",
        ] {
            assert_eq!(
                variant_family(repo),
                "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M"
            );
        }
        assert_eq!(
            variant_family("OpenMed/OpenMed-PII-ClinicalE5-Small-33M-v1-mlx-q8"),
            "OpenMed/OpenMed-PII-ClinicalE5-Small-33M"
        );
        assert_ne!(
            variant_family("OpenMed/privacy-filter-multilingual-v2"),
            variant_family("OpenMed/privacy-filter-multilingual")
        );
    }

    #[test]
    fn variants_are_ranked_by_host_then_evidence_and_unrunnable_ones_dropped() {
        let c = catalog(&[
            ("OpenMed/OpenMed-NER-X-65M", "distilbert", r#"["pytorch"]"#),
            (
                "OpenMed/OpenMed-NER-X-65M-mlx",
                "distilbert",
                r#"["mlx-fp","pytorch"]"#,
            ),
            (
                "OpenMed/OpenMed-NER-X-65M-mlx-8bit",
                "distilbert",
                r#"["mlx-8bit","pytorch"]"#,
            ),
            (
                "OpenMed/OpenMed-NER-X-65M-v1-onnx-android",
                "distilbert",
                r#"["onnx"]"#,
            ),
            (
                "OpenMed/OpenMed-NER-Other-65M",
                "distilbert",
                r#"["pytorch"]"#,
            ),
        ]);
        let other: Vec<VariantRuntime> =
            rank_variants(&c, "OpenMed/OpenMed-NER-X-65M-mlx", HostPlatform::Other)
                .iter()
                .map(|v| v.runtime)
                .collect();
        assert_eq!(
            other,
            [
                VariantRuntime::CandleSafetensors,
                VariantRuntime::TractOnnx,
                VariantRuntime::CandleMlx
            ],
            "quantized MLX is not runnable and is dropped; other families excluded"
        );
        let apple = rank_variants(&c, "OpenMed/OpenMed-NER-X-65M", HostPlatform::AppleSilicon);
        assert_eq!(apple[0].repo_id, "OpenMed/OpenMed-NER-X-65M-mlx");
        assert_eq!(apple[0].runtime, VariantRuntime::CandleMlx);
    }

    #[test]
    fn recorded_execution_is_reported_and_unknown_families_yield_nothing() {
        let c = catalog(&[
            (
                "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M",
                "distilbert",
                r#"["pytorch"]"#,
            ),
            (
                "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M-v1-onnx-android",
                "distilbert",
                r#"["onnx"]"#,
            ),
        ]);
        let ranked = rank_variants(
            &c,
            "OpenMed/OpenMed-NER-AnatomyDetect-TinyMed-65M",
            HostPlatform::Other,
        );
        assert_eq!(ranked.len(), 2);
        assert!(
            ranked.iter().all(|v| v.executed),
            "both rows carry recorded runs"
        );
        assert!(rank_variants(&c, "OpenMed/None", HostPlatform::Other).is_empty());
    }
}

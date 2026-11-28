//! OpenMed v4.0.0 model ecosystem capabilities: device-tier recommendations,
//! manifest diffing, MedScale model cards, RFC 8785 I-JSON JCS canonicalization,
//! fixed-point decision calibration validation, RAM/disk preflight, and ordered
//! batch evaluation helpers.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use medscale_contracts::objects::DigestSha256;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::catalog::{CatalogError, CatalogRow, ModelCatalog};

pub const MIB: u64 = 1_048_576;
pub const MIN_BUDGET_BYTES: u64 = 16 * MIB;
pub const MAX_BUDGET_BYTES: u64 = 1_048_576 * MIB; // 1 TiB
pub const MAX_SAFE_IJSON_INT: i64 = (1_i64 << 53) - 1;
pub const PPM_SCALE: u64 = 1_000_000;
pub const MAX_BATCH_ITEMS: usize = 64;
pub const DECISION_CALIBRATION_EXTENSION: &str = "org.medscale.decision_calibration";

pub const DIFF_FIELDS: &[&str] = &["tier", "param_count", "formats", "license", "benchmark"];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EcosystemV4Error {
    #[error("unknown device tier '{0}'; expected phone, laptop, workstation, or server")]
    UnknownDeviceTier(String),
    #[error("JCS canonicalization forbids non-integer finite or non-finite floats")]
    NonIntegerJsonNumber,
    #[error("JCS integer outside IEEE-754 I-JSON safe range")]
    UnsafeIntegerRange,
    #[error("calibration profile validation failed: {0}")]
    Calibration(#[from] DecisionCalibrationReason),
    #[error("batch size {0} exceeds maximum of {MAX_BATCH_ITEMS}")]
    BatchLimitExceeded(usize),
    #[error("batch input list cannot be empty")]
    EmptyBatch,
    #[error("catalog error: {0}")]
    Catalog(#[from] CatalogError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTier {
    Phone,
    Laptop,
    Workstation,
    Server,
}

impl DeviceTier {
    pub fn parse(raw: &str) -> Result<Self, EcosystemV4Error> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "phone" => Ok(Self::Phone),
            "laptop" => Ok(Self::Laptop),
            "workstation" => Ok(Self::Workstation),
            "server" => Ok(Self::Server),
            other => Err(EcosystemV4Error::UnknownDeviceTier(other.to_owned())),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Laptop => "laptop",
            Self::Workstation => "workstation",
            Self::Server => "server",
        }
    }

    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Phone => 0,
            Self::Laptop => 1,
            Self::Workstation => 2,
            Self::Server => 3,
        }
    }

    #[must_use]
    pub const fn ram_budget_mb(self) -> f64 {
        match self {
            Self::Phone => 6_000.0,
            Self::Laptop => 16_000.0,
            Self::Workstation => 64_000.0,
            Self::Server => 512_000.0,
        }
    }
}

/// Power-of-two MiB bucketing matching `openmed.core.device_capabilities.budget_bucket`.
#[must_use]
pub fn budget_bucket_bytes(value: u64) -> Option<u64> {
    if value < MIN_BUDGET_BYTES {
        return Some(0);
    }
    if value >= MAX_BUDGET_BYTES {
        return Some(MAX_BUDGET_BYTES);
    }
    let highest_pow2 = 1_u64 << (63 - value.leading_zeros());
    Some(highest_pow2.max(MIN_BUDGET_BYTES))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflightAssessment {
    pub repo_id: String,
    pub medscale_id: String,
    pub fits_ram: bool,
    pub fits_disk: bool,
    pub required_ram_mb: Option<f64>,
    pub required_disk_mb: Option<f64>,
    pub available_ram_mb: f64,
    pub free_disk_mb: f64,
    pub bucketed_ram_bytes: Option<u64>,
    pub recommended_tier: Option<String>,
}

/// Evaluates whether host RAM and free disk fit a catalog model row cleanly.
#[must_use]
pub fn preflight_model_resources(
    row: &CatalogRow,
    tier_hint: DeviceTier,
    available_ram_mb: f64,
    free_disk_mb: f64,
) -> PreflightAssessment {
    let required_ram_mb = peak_ram_mb_for_tier(row, tier_hint);
    let required_disk_mb = row.download_mb.or(match row.disk {
        crate::catalog::SizeMb::Stated(v) | crate::catalog::SizeMb::Estimated(v) => Some(v),
        crate::catalog::SizeMb::Unknown => None,
    });
    let fits_ram = match required_ram_mb {
        Some(req) => req <= available_ram_mb && req <= tier_hint.ram_budget_mb(),
        None => {
            if let Some(rec) = row
                .recommended_tier
                .as_deref()
                .and_then(|s| DeviceTier::parse(s).ok())
            {
                rec.rank() <= tier_hint.rank()
            } else {
                true
            }
        }
    };
    let fits_disk = match required_disk_mb {
        Some(req) => req <= free_disk_mb,
        None => true,
    };
    let available_bytes = (available_ram_mb.max(0.0) * MIB as f64) as u64;
    PreflightAssessment {
        repo_id: row.repo_id.clone(),
        medscale_id: row.medscale_id.clone(),
        fits_ram,
        fits_disk,
        required_ram_mb,
        required_disk_mb,
        available_ram_mb,
        free_disk_mb,
        bucketed_ram_bytes: budget_bucket_bytes(available_bytes),
        recommended_tier: row.recommended_tier.clone(),
    }
}

fn peak_ram_mb_for_tier(row: &CatalogRow, tier: DeviceTier) -> Option<f64> {
    let v = row.peak_ram_mb.as_ref()?;
    match v {
        Value::Number(n) => n.as_f64().filter(|x| x.is_finite()),
        Value::Object(map) => map
            .get(tier.as_str())
            .and_then(Value::as_f64)
            .filter(|x| x.is_finite()),
        _ => None,
    }
}

fn has_tier_enrichment(row: &CatalogRow, tier: DeviceTier) -> bool {
    row.recommended_tier.is_some() || peak_ram_mb_for_tier(row, tier).is_some()
}

fn benchmark_score(row: &CatalogRow) -> f64 {
    let Some(Value::Object(map)) = row.benchmark.as_ref() else {
        return 0.0;
    };
    let mut best = 0.0_f64;
    for (k, v) in map {
        let low = k.to_ascii_lowercase();
        if (low.contains("recall") || low.contains("f1"))
            && let Some(num) = v.as_f64()
            && num.is_finite()
            && num > best
        {
            best = num;
        }
    }
    best
}

/// Filters and ranks catalog models for `device_tier` identically to
/// `openmed.core.model_search.recommend_models`.
#[must_use]
pub fn recommend_catalog_models<'a>(
    catalog: &'a ModelCatalog,
    device_tier: DeviceTier,
    task: Option<&str>,
    language: Option<&str>,
) -> Vec<&'a CatalogRow> {
    let norm_task = task.map(|t| t.trim().to_ascii_lowercase());
    let norm_lang = language.map(|l| l.trim().to_ascii_lowercase());
    let budget = device_tier.ram_budget_mb();

    let mut matches: Vec<&'a CatalogRow> = catalog
        .rows()
        .iter()
        .filter(|row| {
            if let Some(ref t) = norm_task
                && !t.is_empty()
                && row.task.to_ascii_lowercase() != *t
            {
                return false;
            }
            if let Some(ref l) = norm_lang
                && !l.is_empty()
                && !row
                    .languages
                    .iter()
                    .any(|lang| lang.eq_ignore_ascii_case(l))
            {
                return false;
            }
            if let Some(rec_str) = row.recommended_tier.as_deref()
                && let Ok(rec) = DeviceTier::parse(rec_str)
                && rec.rank() > device_tier.rank()
            {
                return false;
            }
            if let Some(ram) = peak_ram_mb_for_tier(row, device_tier)
                && ram > budget
            {
                return false;
            }
            true
        })
        .collect();

    matches.sort_by(|a, b| {
        let enrich_a = u8::from(!has_tier_enrichment(a, device_tier));
        let enrich_b = u8::from(!has_tier_enrichment(b, device_tier));
        let size_a = peak_ram_mb_for_tier(a, device_tier)
            .or_else(|| a.param_count.map(|p| p as f64))
            .unwrap_or(f64::INFINITY);
        let size_b = peak_ram_mb_for_tier(b, device_tier)
            .or_else(|| b.param_count.map(|p| p as f64))
            .unwrap_or(f64::INFINITY);
        let score_a = -benchmark_score(a);
        let score_b = -benchmark_score(b);
        enrich_a
            .cmp(&enrich_b)
            .then_with(|| size_a.total_cmp(&size_b))
            .then_with(|| score_a.total_cmp(&score_b))
            .then_with(|| {
                a.repo_id
                    .to_ascii_lowercase()
                    .cmp(&b.repo_id.to_ascii_lowercase())
            })
            .then_with(|| a.repo_id.cmp(&b.repo_id))
    });
    matches
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestFieldChange {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestRepoChange {
    pub repo_id: String,
    pub changes: Vec<ManifestFieldChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogManifestDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<ManifestRepoChange>,
}

fn manifest_row_field_value(row: &CatalogRow, field: &str) -> Value {
    match field {
        "tier" => row.tier.clone().map(Value::String).unwrap_or(Value::Null),
        "param_count" => row
            .param_count
            .map(|n| Value::Number(n.into()))
            .unwrap_or(Value::Null),
        "formats" => {
            let mut fmts = row.formats.clone();
            fmts.sort();
            Value::Array(fmts.into_iter().map(Value::String).collect())
        }
        "license" => row
            .license_claim
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
        "benchmark" => row.benchmark.clone().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// Computes structural diff across `tier`, `param_count`, `formats`, `license`,
/// and `benchmark` between two JSONL catalog snapshots.
pub fn diff_catalog_manifests(
    old_jsonl: &[u8],
    new_jsonl: &[u8],
) -> Result<CatalogManifestDiff, CatalogError> {
    let old_cat = ModelCatalog::import(
        "diff/old",
        "0000000000000000000000000000000000000000",
        old_jsonl,
    )?;
    let new_cat = ModelCatalog::import(
        "diff/new",
        "0000000000000000000000000000000000000000",
        new_jsonl,
    )?;

    let old_map: BTreeMap<&str, &CatalogRow> = old_cat
        .rows()
        .iter()
        .map(|r| (r.repo_id.as_str(), r))
        .collect();
    let new_map: BTreeMap<&str, &CatalogRow> = new_cat
        .rows()
        .iter()
        .map(|r| (r.repo_id.as_str(), r))
        .collect();

    let added: Vec<String> = new_map
        .keys()
        .filter(|k| !old_map.contains_key(*k))
        .map(|s| (*s).to_owned())
        .collect();
    let removed: Vec<String> = old_map
        .keys()
        .filter(|k| !new_map.contains_key(*k))
        .map(|s| (*s).to_owned())
        .collect();

    let mut changed = Vec::new();
    for (repo_id, old_row) in &old_map {
        let Some(new_row) = new_map.get(repo_id) else {
            continue;
        };
        let mut field_changes = Vec::new();
        for &field in DIFF_FIELDS {
            let before = manifest_row_field_value(old_row, field);
            let after = manifest_row_field_value(new_row, field);
            if before != after {
                field_changes.push(ManifestFieldChange {
                    field: field.to_owned(),
                    before,
                    after,
                });
            }
        }
        if !field_changes.is_empty() {
            changed.push(ManifestRepoChange {
                repo_id: (*repo_id).to_owned(),
                changes: field_changes,
            });
        }
    }

    Ok(CatalogManifestDiff {
        added,
        removed,
        changed,
    })
}

/// Renders a MedScale Model Card in Markdown preserving immutable source
/// `repo_id` provenance and honest qualification boundaries.
#[must_use]
pub fn render_medscale_model_card(row: &CatalogRow) -> String {
    let params = row
        .param_count
        .map(|p| format!("{p}"))
        .unwrap_or_else(|| "unknown".to_owned());
    let formats = if row.formats.is_empty() {
        "unspecified".to_owned()
    } else {
        row.formats.join(", ")
    };
    let languages = if row.languages.is_empty() {
        "unspecified".to_owned()
    } else {
        row.languages.join(", ")
    };
    let license = row
        .license_claim
        .as_deref()
        .unwrap_or("unspecified (rights pending)");
    let tier = row.tier.as_deref().unwrap_or("unspecified");
    let rec_tier = row.recommended_tier.as_deref().unwrap_or("unspecified");
    let download = row
        .download_mb
        .map(|mb| format!("{mb:.1} MB"))
        .unwrap_or_else(|| "unspecified".to_owned());

    let mut lines = vec![
        format!("# MedScale Model Card — {}", row.medscale_id),
        String::new(),
        format!("- **Source Repository ID:** `{}`", row.repo_id),
        format!("- **Family / Task:** `{}` / `{}`", row.family, row.task),
        format!("- **Architecture:** `{}`", row.architecture),
        format!("- **Parameters / Tier:** {} (`{}`)", params, tier),
        format!("- **Formats:** {}", formats),
        format!("- **Languages:** {}", languages),
        format!("- **Recommended Hardware Tier:** `{}`", rec_tier),
        format!("- **Estimated Download:** {}", download),
        format!("- **License:** `{}`", license),
        format!("- **Catalog Qualification Status:** `{:?}`", row.status),
    ];
    if let Some(ref arxiv) = row.arxiv {
        lines.push(format!("- **Reference Paper (arXiv):** `{arxiv}`"));
    }
    if let Some(ref hash) = row.reproducibility_hash {
        lines.push(format!("- **Reproducibility Hash:** `{hash}`"));
    }
    if !row.canonical_labels.is_empty() {
        lines.push(format!(
            "- **Canonical Labels ({}):** {}",
            row.canonical_labels.len(),
            row.canonical_labels.join(", ")
        ));
    }
    if !row.script_coverage.is_empty() {
        let summary: Vec<String> = row
            .script_coverage
            .iter()
            .map(|(k, v)| {
                if let Some(score) = v.as_f64() {
                    format!("{k}: {score:.2}")
                } else {
                    let verdict = v
                        .get("verdict")
                        .and_then(Value::as_str)
                        .unwrap_or("present");
                    format!("{k} ({verdict})")
                }
            })
            .collect();
        lines.push(format!("- **Script Coverage:** {}", summary.join(", ")));
    }
    if let Some(ref bm) = row.benchmark {
        lines.push(format!("- **Benchmark Metadata:** `{bm}`"));
    }
    lines.push(String::new());
    lines.push(
        "> **Qualification Notice:** Catalog discovery does not imply local installation, execution capability, clinical validation, or permission to process live PHI."
            .to_owned(),
    );
    lines.join("\n")
}

/// RFC 8785 integer-only I-JSON canonical serialization matching
/// `openmed.core.portable_bundle.canonical_json_bytes`.
pub fn jcs_canonical_bytes(value: &Value) -> Result<Vec<u8>, EcosystemV4Error> {
    let mut out = String::new();
    write_jcs(value, &mut out)?;
    Ok(out.into_bytes())
}

pub fn jcs_content_id(value: &Value) -> Result<String, EcosystemV4Error> {
    let bytes = jcs_canonical_bytes(value)?;
    Ok(format!("sha256:{}", DigestSha256::of(&bytes).to_hex()))
}

fn write_jcs(value: &Value, out: &mut String) -> Result<(), EcosystemV4Error> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => {
            if n.is_f64() && !n.is_i64() && !n.is_u64() {
                return Err(EcosystemV4Error::NonIntegerJsonNumber);
            }
            if let Some(i) = n.as_i64() {
                if !(-MAX_SAFE_IJSON_INT..=MAX_SAFE_IJSON_INT).contains(&i) {
                    return Err(EcosystemV4Error::UnsafeIntegerRange);
                }
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                if u > MAX_SAFE_IJSON_INT as u64 {
                    return Err(EcosystemV4Error::UnsafeIntegerRange);
                }
                out.push_str(&u.to_string());
            } else {
                return Err(EcosystemV4Error::NonIntegerJsonNumber);
            }
        }
        Value::String(s) => {
            let encoded =
                serde_json::to_string(s).map_err(|_| EcosystemV4Error::NonIntegerJsonNumber)?;
            out.push_str(&encoded);
        }
        Value::Array(arr) => {
            out.push('[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_jcs(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            // Sort by UTF-16 code units per RFC 8785 §3.2.3
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|(ka, _), (kb, _)| ka.encode_utf16().cmp(kb.encode_utf16()));
            out.push('{');
            for (i, (k, v)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let ek =
                    serde_json::to_string(k).map_err(|_| EcosystemV4Error::NonIntegerJsonNumber)?;
                out.push_str(&ek);
                out.push(':');
                write_jcs(v, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationMethod {
    DeterministicMargin,
    IsotonicRegression,
    PlattScaling,
    TemperatureScaling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCalibrationReason {
    #[error("calibration_bundle_unverified")]
    CalibrationBundleUnverified,
    #[error("calibration_extension_missing")]
    CalibrationExtensionMissing,
    #[error("calibration_malformed")]
    CalibrationMalformed,
    #[error("calibration_version_unsupported")]
    CalibrationVersionUnsupported,
    #[error("calibration_digest_mismatch")]
    CalibrationDigestMismatch,
    #[error("calibration_model_mismatch")]
    CalibrationModelMismatch,
    #[error("calibration_runtime_mismatch")]
    CalibrationRuntimeMismatch,
    #[error("calibration_scope_mismatch")]
    CalibrationScopeMismatch,
    #[error("calibration_evidence_mismatch")]
    CalibrationEvidenceMismatch,
    #[error("calibration_profile_missing")]
    CalibrationProfileMissing,
    #[error("calibration_runtime_unbound")]
    CalibrationRuntimeUnbound,
    #[error("calibration_schema_incompatible")]
    CalibrationSchemaIncompatible,
    #[error("calibration_pin_mismatch")]
    CalibrationPinMismatch,
    #[error("calibration_request_invalid")]
    CalibrationRequestInvalid,
}

impl DecisionCalibrationReason {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::CalibrationBundleUnverified => "calibration_bundle_unverified",
            Self::CalibrationExtensionMissing => "calibration_extension_missing",
            Self::CalibrationMalformed => "calibration_malformed",
            Self::CalibrationVersionUnsupported => "calibration_version_unsupported",
            Self::CalibrationDigestMismatch => "calibration_digest_mismatch",
            Self::CalibrationModelMismatch => "calibration_model_mismatch",
            Self::CalibrationRuntimeMismatch => "calibration_runtime_mismatch",
            Self::CalibrationScopeMismatch => "calibration_scope_mismatch",
            Self::CalibrationEvidenceMismatch => "calibration_evidence_mismatch",
            Self::CalibrationProfileMissing => "calibration_profile_missing",
            Self::CalibrationRuntimeUnbound => "calibration_runtime_unbound",
            Self::CalibrationSchemaIncompatible => "calibration_schema_incompatible",
            Self::CalibrationPinMismatch => "calibration_pin_mismatch",
            Self::CalibrationRequestInvalid => "calibration_request_invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCalibrationProfile {
    pub calibration_id: String,
    pub version: u32,
    pub method: CalibrationMethod,
    pub min_margin_ppm: u64,
    pub abstention_threshold_ppm: u64,
    pub model_id: String,
    pub runtime_digests: Vec<String>,
}

/// Validates integer-only fixed-point PPM calibration bounds cleanly.
pub fn verify_calibration_profile_bounds(
    profile: &BundleCalibrationProfile,
) -> Result<(), DecisionCalibrationReason> {
    if profile.calibration_id.trim().is_empty() || profile.model_id.trim().is_empty() {
        return Err(DecisionCalibrationReason::CalibrationMalformed);
    }
    if profile.version != 1 {
        return Err(DecisionCalibrationReason::CalibrationVersionUnsupported);
    }
    if profile.min_margin_ppm > PPM_SCALE || profile.abstention_threshold_ppm > PPM_SCALE {
        return Err(DecisionCalibrationReason::CalibrationMalformed);
    }
    if profile.runtime_digests.is_empty() {
        return Err(DecisionCalibrationReason::CalibrationRuntimeUnbound);
    }
    let unique: BTreeSet<&str> = profile.runtime_digests.iter().map(String::as_str).collect();
    if unique.len() != profile.runtime_digests.len() {
        return Err(DecisionCalibrationReason::CalibrationMalformed);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchEvaluationItem {
    pub index: usize,
    pub input_len: usize,
    pub proposal_payload: Value,
}

/// Utility helper to run an ordered slice of inputs up to [`MAX_BATCH_ITEMS`]
/// with cancellation checks between items (no partial results on cancellation).
pub fn evaluate_texts_ordered_cancellable<E, F>(
    inputs: &[String],
    cancel: &std::sync::atomic::AtomicBool,
    cancelled_err: E,
    mut eval_one: F,
) -> Result<Vec<BatchEvaluationItem>, Result<E, EcosystemV4Error>>
where
    F: FnMut(&str) -> Result<Value, E>,
{
    if inputs.is_empty() {
        return Err(Err(EcosystemV4Error::EmptyBatch));
    }
    if inputs.len() > MAX_BATCH_ITEMS {
        return Err(Err(EcosystemV4Error::BatchLimitExceeded(inputs.len())));
    }
    let mut out = Vec::with_capacity(inputs.len());
    for (idx, text) in inputs.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Ok(cancelled_err));
        }
        let payload = eval_one(text).map_err(Ok)?;
        out.push(BatchEvaluationItem {
            index: idx,
            input_len: text.len(),
            proposal_payload: payload,
        });
    }
    Ok(out)
}

pub fn build_jcs_object(pairs: &[(&str, Value)]) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert((*k).to_owned(), v.clone());
    }
    Value::Object(m)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceLabelScore {
    pub label: String,
    pub probability: f32,
}

/// Computes numerically stable softmax scores paired with labels (highest first).
pub fn softmax_label_scores(logits: &[f32], labels: &[String]) -> Option<Vec<SequenceLabelScore>> {
    if logits.is_empty() || logits.len() != labels.len() || logits.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|&v| (v - max_logit).exp()).collect();
    let sum: f32 = exps.iter().sum();
    if !sum.is_finite() || sum <= 0.0 {
        return None;
    }
    let mut out: Vec<SequenceLabelScore> = labels
        .iter()
        .zip(exps)
        .map(|(label, e)| SequenceLabelScore {
            label: label.clone(),
            probability: e / sum,
        })
        .collect();
    out.sort_by(|a, b| {
        b.probability
            .total_cmp(&a.probability)
            .then_with(|| a.label.cmp(&b.label))
    });
    Some(out)
}

impl fmt::Display for DeviceTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

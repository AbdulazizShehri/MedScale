//! OpenMed v4.0.0 Wave 2 authority contracts (privacy presets, calendar/RTL
//! de-identification, table re-identification risk, clinical NLP & UCUM v4).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Query filter for `PrivacyV4Presets`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyV4PresetsRequest {
    #[serde(default)]
    pub filter: Option<String>,
}

/// Summary of one of the 25 privacy policy presets (24 OpenMed + `saudi_pdpl`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyPresetDto {
    pub name: String,
    pub schema_version: u32,
    pub posture: String,
    pub default_action: String,
    pub strict_no_leak: bool,
    pub safety_sweep_mandatory: bool,
    pub keep_mapping: bool,
    pub reversible_id: bool,
    pub actions: BTreeMap<String, String>,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyV4PresetsResponse {
    pub presets: Vec<PrivacyPresetDto>,
}

/// Request for bounded local synthetic text de-identification using Wave 2 controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyV4DeidentifyRequest {
    pub text: String,
    #[serde(default)]
    pub policy_name: Option<String>,
    #[serde(default)]
    pub calendar: Option<String>,
    #[serde(default)]
    pub date_shift_days: Option<i64>,
    #[serde(default)]
    pub patient_key: Option<String>,
    #[serde(default)]
    pub shift_secret: Option<String>,
    #[serde(default)]
    pub max_shift_days: Option<i64>,
    #[serde(default)]
    pub rtl_isolate: bool,
    #[serde(default = "default_true")]
    pub clinical_protect: bool,
    #[serde(default = "default_true")]
    pub safety_sweep: bool,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacySpanDto {
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShiftedDateDto {
    pub original: String,
    pub shifted: String,
    pub calendar: String,
    pub offset_days: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyComplianceReceiptDto {
    pub schema_version: u32,
    pub policy_name: String,
    pub policy_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub base_direction: String,
    pub rtl_isolated: bool,
    pub suppressed_clinical_terms: usize,
    pub sweep_added_spans: usize,
    pub total_redacted_spans: usize,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyV4DeidentifyResponse {
    pub redacted_text: String,
    pub policy_name: String,
    pub policy_sha256: String,
    pub base_direction: String,
    pub rtl_isolated: bool,
    pub detected_spans: Vec<PrivacySpanDto>,
    pub suppressed_clinical_terms: usize,
    pub sweep_added_spans: usize,
    pub shifted_dates: Vec<ShiftedDateDto>,
    pub receipt: PrivacyComplianceReceiptDto,
    pub receipt_verified: bool,
}

/// Request for k-anonymity, l-diversity, t-closeness & attacker risk analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacyV4RiskRequest {
    pub records: Vec<BTreeMap<String, Value>>,
    pub quasi_identifiers: Vec<String>,
    #[serde(default)]
    pub sensitive_attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacyV4RiskResponse {
    pub record_count: usize,
    pub k_min: usize,
    pub class_count: usize,
    pub singleton_count: usize,
    pub l_diversity: BTreeMap<String, usize>,
    pub t_closeness: BTreeMap<String, f64>,
    pub prosecutor_risk: f64,
    pub journalist_risk: f64,
    pub marketer_risk: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MedMentionDto {
    pub medication: String,
    #[serde(default)]
    pub dose: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoseCheckDto {
    pub drug: String,
    pub route: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoseRangeDto {
    pub drug: String,
    pub route: String,
    #[serde(default)]
    pub low: Option<f64>,
    #[serde(default)]
    pub high: Option<f64>,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimDto {
    pub claim_id: String,
    pub claim_key: String,
    pub value: String,
}

/// Request for deterministic clinical NLP, ConText, Sig, med-rec, guarded dosing, SDOH & NLI analysis.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClinicalV4AnalyzeRequest {
    pub text: String,
    #[serde(default)]
    pub specialty: Option<String>,
    #[serde(default)]
    pub sig_lines: Vec<String>,
    #[serde(default)]
    pub med_mentions: Vec<MedMentionDto>,
    #[serde(default)]
    pub dose_checks: Vec<DoseCheckDto>,
    #[serde(default)]
    pub dose_ranges: Vec<DoseRangeDto>,
    #[serde(default)]
    pub nli_scores: Option<(f64, f64, f64)>,
    #[serde(default)]
    pub claims: Vec<ClaimDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionDto {
    pub label: String,
    pub header: String,
    pub start: usize,
    pub end: usize,
    pub content_start: usize,
    pub loinc_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegationScopeDto {
    pub cue: String,
    pub direction: String,
    pub scope_start: usize,
    pub scope_end: usize,
    pub governed: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextAssertionDto {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub negation: String,
    pub temporality: String,
    pub certainty: String,
    pub experiencer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSigDto {
    pub raw: String,
    pub dose: Option<f64>,
    pub unit: Option<String>,
    pub form: Option<String>,
    pub route: Option<String>,
    pub frequency_per_day: Option<f64>,
    pub frequency_period: Option<u32>,
    pub frequency_period_unit: Option<String>,
    pub as_needed: bool,
    pub condition: Option<String>,
    pub duration_days: Option<u32>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciledMedDto {
    pub ingredient: String,
    pub current_status: String,
    pub current_dose: Option<String>,
    pub current_route: Option<String>,
    pub history_len: usize,
    pub has_conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoseAdvisoryDto {
    pub drug: String,
    pub route: String,
    pub status: String,
    pub requires_clinician_review: bool,
    pub autonomous_decision: bool,
    pub disclaimer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationDto {
    pub relation_type: String,
    pub source_text: String,
    pub target_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDto {
    pub event_type: String,
    pub subject: String,
    pub temporality: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdohDto {
    pub category: String,
    pub value: String,
    pub status: String,
    pub temporality: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NliDecisionDto {
    pub outcome: String,
    pub selected_probability: f64,
    pub margin: f64,
    pub requires_human_review: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClinicalV4AnalyzeResponse {
    pub disclaimer: String,
    pub shac_notice: String,
    pub sections: Vec<SectionDto>,
    pub negation_scopes: Vec<NegationScopeDto>,
    pub context_assertions: Vec<ContextAssertionDto>,
    pub parsed_sigs: Vec<ParsedSigDto>,
    pub reconciled_meds: Vec<ReconciledMedDto>,
    pub dose_advisories: Vec<DoseAdvisoryDto>,
    pub relations: Vec<RelationDto>,
    pub events: Vec<EventDto>,
    pub sdoh: Vec<SdohDto>,
    pub specialty_result: Option<Value>,
    pub nli_decision: Option<NliDecisionDto>,
    pub claim_conflict_count: usize,
}

/// Request for compositional UCUM v4 parsing and normalization / analyte conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitsV4ConvertRequest {
    #[serde(default)]
    pub value: Option<f64>,
    pub from_unit: String,
    #[serde(default)]
    pub to_unit: Option<String>,
    #[serde(default)]
    pub analyte: Option<String>,
    #[serde(default)]
    pub molar_mass_g_per_mol: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitsV4ConvertResponse {
    pub parsed: Value,
    pub conversion: Option<Value>,
}

//! H0-B presentation / coverage contract types (non-authoritative views).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::objects::{DigestSha256, MedicalTime, OpaqueId};
use crate::text::TextSpan;

/// Closed presentation rules pin for golden rebuild.
pub const PRESENTATION_RULES_VERSION: &str = "presentation.rules.v1";

/// Admitted UCUM subset pin id (hand table; no FHIRPath).
pub const UCUM_SUBSET_ID: &str = "medscale.ucum.subset.v1";

/// SHA-256 of the canonical admitted UCUM code list (newline-joined sorted codes + trailing newline).
pub const UCUM_SUBSET_DIGEST_HEX: &str =
    "947866ded10e36783aa1f15d5eb8e90b79513f1fec1a9e27528bd1c7b1904303";

/// Projection kind constants.
pub const KIND_SUBJECT_TIMELINE_V1: &str = "SubjectTimelineV1";
pub const KIND_SUBJECT_BRIEF_V1: &str = "SubjectBriefV1";
pub const KIND_SUBJECT_COVERAGE_V1: &str = "SubjectCoverageV1";

/// Coverage honesty status for a field or concept slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Present,
    Absent,
    Unknown,
    Conflict,
    IncomparableUnits,
    UnhealthyEvidence,
    UnsupportedResourceType,
}

/// Unit comparability within the admitted UCUM subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitComparability {
    Comparable,
    Incomparable,
    Unrecognized,
}

/// Result of unit semantic evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitSemanticResult {
    pub raw_unit: String,
    pub admitted: bool,
    pub canonical_unit: Option<String>,
    pub comparability: UnitComparability,
}

/// Whole-resource citation (structural path note is not executed FHIRPath).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WholeResourceCitation {
    pub source_id: OpaqueId,
    pub content_digest: DigestSha256,
    pub structural_path_note: Option<String>,
}

/// Span or whole-resource citation for drill-down.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpanCitation {
    TextSpan { source_id: OpaqueId, span: TextSpan },
    WholeResource { citation: WholeResourceCitation },
}

/// Typed extracted field with coverage status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractedField {
    pub field_key: String,
    pub value: Option<Value>,
    pub status: CoverageStatus,
    pub evidence_refs: Vec<OpaqueId>,
    pub span_citations: Vec<SpanCitation>,
    pub unit: Option<String>,
    pub unit_semantic: Option<UnitSemanticResult>,
}

/// Conflict members retained unresolved in H0-B.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictSet {
    pub claim_key: String,
    pub members: Vec<ExtractedField>,
    /// Always `Unresolved` in H0-B.
    pub resolution: String,
}

/// Coverage slot for a closed concept key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageSlot {
    pub concept_key: String,
    pub status: CoverageStatus,
    pub values: Vec<ExtractedField>,
    pub conflict: Option<ConflictSet>,
    pub notes: Vec<String>,
}

/// Deterministic timeline event from a ClinicalAssertion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineEvent {
    pub event_id: String,
    pub assertion_id: OpaqueId,
    pub subject_ref: OpaqueId,
    pub claim_kind: String,
    pub summary: Value,
    pub effective_time: Option<MedicalTime>,
    pub recorded_time: MedicalTime,
    pub sort_key: String,
    pub evidence_refs: Vec<OpaqueId>,
    pub span_citations: Vec<SpanCitation>,
}

/// Narrow LLM-free Brief sections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefSections {
    pub identity: Vec<ExtractedField>,
    pub vitals: Vec<ExtractedField>,
    pub conditions: Vec<ExtractedField>,
    pub coverage_summary: CoverageSummary,
}

/// Rollup of coverage statuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageSummary {
    pub present: u32,
    pub absent: u32,
    pub unknown: u32,
    pub conflict: u32,
    pub incomparable_units: u32,
    pub unhealthy_evidence: u32,
    pub unsupported_resource_type: u32,
}

/// Presentation rules version embedded in Projection bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationRulesVersion {
    pub extractors: Vec<String>,
    pub ucum_subset: String,
    pub ucum_subset_digest_hex: String,
    pub order_rules: String,
    pub brief_schema: String,
    pub rules_id: String,
}

impl PresentationRulesVersion {
    /// Canonical H0-B rules pin.
    #[must_use]
    pub fn v1() -> Self {
        Self {
            extractors: vec![
                "patient.v1".to_owned(),
                "observation.v1".to_owned(),
                "condition.v1".to_owned(),
            ],
            ucum_subset: UCUM_SUBSET_ID.to_owned(),
            ucum_subset_digest_hex: UCUM_SUBSET_DIGEST_HEX.to_owned(),
            order_rules: "timeline.order.effective_then_recorded_then_id.v1".to_owned(),
            brief_schema: KIND_SUBJECT_BRIEF_V1.to_owned(),
            rules_id: PRESENTATION_RULES_VERSION.to_owned(),
        }
    }
}

/// SubjectTimelineV1 Projection body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectTimelineV1 {
    pub subject_ref: OpaqueId,
    pub events: Vec<TimelineEvent>,
    pub built_rules_version: PresentationRulesVersion,
}

/// SubjectBriefV1 Projection body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectBriefV1 {
    pub subject_ref: OpaqueId,
    pub sections: BriefSections,
    pub built_rules_version: PresentationRulesVersion,
}

/// SubjectCoverageV1 Projection body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectCoverageV1 {
    pub subject_ref: OpaqueId,
    pub slots: Vec<CoverageSlot>,
    pub built_rules_version: PresentationRulesVersion,
}

/// Drill-down result for a presentation field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrillDownResult {
    pub field_key: String,
    pub source_id: OpaqueId,
    pub content_digest: DigestSha256,
    pub media_type: String,
    pub span: Option<TextSpan>,
    pub citation: Option<WholeResourceCitation>,
    pub excerpt_lossy: Option<String>,
    pub status: CoverageStatus,
}

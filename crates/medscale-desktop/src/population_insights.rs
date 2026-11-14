//! Read-only Population Insights + contextual evidence assistant boundary for Spec 062.
//!
//! This module aggregates existing trusted presentation contracts and admitted
//! evidence-retrieval metadata into Desktop display state. It does not create a
//! clinical risk score, open storage/network clients, or mutate authority.

use medscale_contracts::evidence::{LexicalRetrieveResult, RetrievalHit};
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::presentation::{
    BriefSections, CoverageSlot, CoverageStatus, CoverageSummary, ExtractedField,
    PresentationRulesVersion, SubjectBriefV1, SubjectCoverageV1,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionRowVm {
    pub label: String,
    pub value: String,
    pub detail: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortRowVm {
    pub subject_ref: String,
    pub display_name: String,
    pub condition: String,
    pub evidence_state: String,
    pub review_attention: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantEvidenceVm {
    pub source: String,
    pub title: String,
    pub snippet: String,
    pub metadata: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationInsightsVm {
    pub synthetic_only: bool,
    pub cohort_size: usize,
    pub review_attention_count: usize,
    pub evidence_gap_count: u32,
    pub present_count: u32,
    pub condition_distribution: Vec<DistributionRowVm>,
    pub coverage_distribution: Vec<DistributionRowVm>,
    pub cohorts: Vec<CohortRowVm>,
    pub evidence: Vec<AssistantEvidenceVm>,
    pub risk_state: String,
    pub trend_state: String,
    pub gap_state: String,
    pub recommendation_state: String,
    pub assistant_default: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopulationInsightsError {
    EmptyPopulation,
    SubjectMismatch,
    RetrievalAuthorityBoundaryViolated,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CoverageTally {
    present: u32,
    absent: u32,
    unknown: u32,
    conflict: u32,
    incomparable_units: u32,
    unhealthy_evidence: u32,
    unsupported_resource_type: u32,
}

impl CoverageTally {
    fn add(&mut self, status: CoverageStatus) {
        match status {
            CoverageStatus::Present => self.present += 1,
            CoverageStatus::Absent => self.absent += 1,
            CoverageStatus::Unknown => self.unknown += 1,
            CoverageStatus::Conflict => self.conflict += 1,
            CoverageStatus::IncomparableUnits => self.incomparable_units += 1,
            CoverageStatus::UnhealthyEvidence => self.unhealthy_evidence += 1,
            CoverageStatus::UnsupportedResourceType => self.unsupported_resource_type += 1,
        }
    }

    fn evidence_gap_count(self) -> u32 {
        self.absent + self.unknown + self.unsupported_resource_type
    }
}

impl PopulationInsightsVm {
    pub fn from_trusted(
        subjects: &[(SubjectBriefV1, SubjectCoverageV1)],
        retrieval: &LexicalRetrieveResult,
    ) -> Result<Self, PopulationInsightsError> {
        if subjects.is_empty() {
            return Err(PopulationInsightsError::EmptyPopulation);
        }
        if !retrieval.evidence_only
            || !retrieval.relevance_is_not_authority
            || retrieval.hits.iter().any(|hit| !hit.relevance_only)
        {
            return Err(PopulationInsightsError::RetrievalAuthorityBoundaryViolated);
        }

        let mut tally = CoverageTally::default();
        let mut conditions = BTreeMap::<String, u32>::new();
        let mut cohorts = Vec::with_capacity(subjects.len());
        let mut review_attention_count = 0usize;

        for (brief, coverage) in subjects {
            if brief.subject_ref != coverage.subject_ref {
                return Err(PopulationInsightsError::SubjectMismatch);
            }

            for slot in &coverage.slots {
                tally.add(slot.status);
            }

            for field in &brief.sections.conditions {
                if field.status == CoverageStatus::Present
                    && let Some(value) = field.value.as_ref().map(value_text)
                {
                    *conditions.entry(value).or_default() += 1;
                }
            }

            let needs_review = coverage.slots.iter().any(|slot| {
                matches!(
                    slot.status,
                    CoverageStatus::Unknown
                        | CoverageStatus::Conflict
                        | CoverageStatus::IncomparableUnits
                        | CoverageStatus::UnhealthyEvidence
                )
            });
            if needs_review {
                review_attention_count += 1;
            }

            cohorts.push(CohortRowVm {
                subject_ref: brief.subject_ref.as_str().to_owned(),
                display_name: display_name(brief),
                condition: condition_label(brief),
                evidence_state: coverage_state(coverage),
                review_attention: if needs_review {
                    "Review evidence".to_owned()
                } else {
                    "No unresolved evidence".to_owned()
                },
            });
        }

        let condition_distribution = conditions
            .into_iter()
            .map(|(label, count)| DistributionRowVm {
                label,
                value: count.to_string(),
                detail: "Present condition facts only".to_owned(),
                status: "Trusted presentation".to_owned(),
            })
            .collect();
        let coverage_distribution = coverage_rows(tally);
        let evidence = retrieval
            .hits
            .iter()
            .map(|hit| evidence_card(retrieval, hit))
            .collect::<Vec<_>>();
        let evidence_gap_count = tally.evidence_gap_count();

        Ok(Self {
            synthetic_only: true,
            cohort_size: subjects.len(),
            review_attention_count,
            evidence_gap_count,
            present_count: tally.present,
            condition_distribution,
            coverage_distribution,
            cohorts,
            evidence,
            risk_state: "Clinical risk score unavailable — no qualified risk contract. Review attention reflects evidence state only; patients are not clinically ranked."
                .to_owned(),
            trend_state: "Population trend withheld — this static synthetic fixture does not establish a qualified longitudinal cohort-trend contract."
                .to_owned(),
            gap_state: format!(
                "{evidence_gap_count} absent, unknown, or unsupported coverage slots. These are evidence gaps, not clinical care-gap claims."
            ),
            recommendation_state: format!(
                "Review {review_attention_count} subject(s) with unknown, conflicting, incomparable, or unhealthy evidence. Navigation cue only; no action is committed."
            ),
            assistant_default: format!(
                "Synthetic cohort: {} subjects; {} need evidence review. Evidence context only — retrieval relevance is not clinical authority.",
                subjects.len(),
                review_attention_count
            ),
        })
    }

    pub fn answer_query(&self, query: &str) -> String {
        let normalized = query.trim().to_lowercase();
        let body = if normalized.is_empty() {
            self.assistant_default.clone()
        } else if normalized.contains("risk") {
            self.risk_state.clone()
        } else if normalized.contains("gap")
            || normalized.contains("coverage")
            || normalized.contains("unknown")
        {
            self.gap_state.clone()
        } else if normalized.contains("condition") || normalized.contains("cohort") {
            let top = self
                .condition_distribution
                .first()
                .map(|row| format!("{}: {} subject(s)", row.label, row.value))
                .unwrap_or_else(|| {
                    "No present condition facts in this synthetic cohort".to_owned()
                });
            format!("Cohort size: {}. {top}.", self.cohort_size)
        } else if normalized.contains("recommend") || normalized.contains("next") {
            self.recommendation_state.clone()
        } else {
            format!(
                "I can explain trusted cohort coverage, present condition distribution, evidence-review attention, or retrieval provenance. {}",
                self.assistant_default
            )
        };
        format!("{body} Evidence-context only; relevance is not clinical authority.")
    }

    pub fn synthetic_demo() -> Self {
        let subjects = synthetic_subjects();
        let retrieval = synthetic_retrieval();
        Self::from_trusted(&subjects, &retrieval)
            .expect("synthetic population inputs preserve authority boundaries")
    }
}

fn coverage_rows(tally: CoverageTally) -> Vec<DistributionRowVm> {
    [
        ("Present", tally.present, "Supported trusted presentation"),
        ("Absent", tally.absent, "Explicitly absent evidence"),
        ("Unknown", tally.unknown, "Unknown — not inferred"),
        (
            "Conflict",
            tally.conflict,
            "Unresolved conflicting evidence",
        ),
        (
            "Incomparable units",
            tally.incomparable_units,
            "No silent unit comparison",
        ),
        (
            "Evidence unhealthy",
            tally.unhealthy_evidence,
            "Evidence requires review",
        ),
        (
            "Unsupported resource",
            tally.unsupported_resource_type,
            "Outside trusted extractor scope",
        ),
    ]
    .into_iter()
    .filter(|(_, count, _)| *count > 0)
    .map(|(label, count, detail)| DistributionRowVm {
        label: label.to_owned(),
        value: count.to_string(),
        detail: detail.to_owned(),
        status: if matches!(label, "Present" | "Absent") {
            "Stable state".to_owned()
        } else {
            "Review / limitation".to_owned()
        },
    })
    .collect()
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "—".to_owned(),
        Value::Array(values) => values.iter().map(value_text).collect::<Vec<_>>().join(", "),
        Value::Object(map) => map
            .get("display")
            .or_else(|| map.get("text"))
            .map(value_text)
            .unwrap_or_else(|| serde_json::to_string(value).unwrap_or_else(|_| "—".to_owned())),
    }
}

fn first_present(fields: &[ExtractedField], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|field| field.field_key == key && field.status == CoverageStatus::Present)
        .and_then(|field| field.value.as_ref())
        .map(value_text)
}

fn display_name(brief: &SubjectBriefV1) -> String {
    let given = first_present(&brief.sections.identity, "patient.name.given");
    let family = first_present(&brief.sections.identity, "patient.name.family");
    match (given, family) {
        (Some(given), Some(family)) => format!("{given} {family}"),
        (Some(name), None) | (None, Some(name)) => name,
        (None, None) => format!("Subject {}", brief.subject_ref.as_str()),
    }
}

fn condition_label(brief: &SubjectBriefV1) -> String {
    let Some(field) = brief.sections.conditions.first() else {
        return "No supported condition fact".to_owned();
    };
    let value = field
        .value
        .as_ref()
        .map(value_text)
        .unwrap_or_else(|| "No value".to_owned());
    if field.status == CoverageStatus::Present {
        value
    } else {
        format!("{value} ({})", coverage_status_label(field.status))
    }
}

fn coverage_state(coverage: &SubjectCoverageV1) -> String {
    let mut states = coverage
        .slots
        .iter()
        .filter(|slot| slot.status != CoverageStatus::Present)
        .map(|slot| coverage_status_label(slot.status))
        .collect::<Vec<_>>();
    states.sort_unstable();
    states.dedup();
    if states.is_empty() {
        "Supported evidence".to_owned()
    } else {
        states.join(" · ")
    }
}

fn coverage_status_label(status: CoverageStatus) -> &'static str {
    match status {
        CoverageStatus::Present => "Present",
        CoverageStatus::Absent => "Absent",
        CoverageStatus::Unknown => "Unknown",
        CoverageStatus::Conflict => "Conflict — unresolved",
        CoverageStatus::IncomparableUnits => "Incomparable units",
        CoverageStatus::UnhealthyEvidence => "Evidence unhealthy",
        CoverageStatus::UnsupportedResourceType => "Unsupported resource",
    }
}

fn evidence_card(retrieval: &LexicalRetrieveResult, hit: &RetrievalHit) -> AssistantEvidenceVm {
    let mut metadata = vec![format!("score {:.2}", hit.score)];
    if hit.retracted {
        metadata.push("RETRACTED".to_owned());
    }
    if let Some(freshness) = &hit.freshness_marker {
        metadata.push(format!("freshness: {freshness}"));
    }
    if let Some(conflict) = &hit.conflict_marker {
        metadata.push(format!("conflict: {conflict}"));
    }
    metadata.push("relevance only".to_owned());
    AssistantEvidenceVm {
        source: retrieval.corpus_source_identity.clone(),
        title: hit.doc_id.as_str().to_owned(),
        snippet: hit.snippet.clone(),
        metadata: metadata.join(" · "),
    }
}

fn field(key: &str, value: Option<Value>, status: CoverageStatus, source: &str) -> ExtractedField {
    ExtractedField {
        field_key: key.to_owned(),
        value,
        status,
        evidence_refs: if status == CoverageStatus::Present {
            vec![OpaqueId::new(source)]
        } else {
            vec![]
        },
        span_citations: vec![],
        unit: None,
        unit_semantic: None,
    }
}

fn summary_for(statuses: &[CoverageStatus]) -> CoverageSummary {
    let mut tally = CoverageTally::default();
    for status in statuses {
        tally.add(*status);
    }
    CoverageSummary {
        present: tally.present,
        absent: tally.absent,
        unknown: tally.unknown,
        conflict: tally.conflict,
        incomparable_units: tally.incomparable_units,
        unhealthy_evidence: tally.unhealthy_evidence,
        unsupported_resource_type: tally.unsupported_resource_type,
    }
}

fn synthetic_subject(
    id: &str,
    given: &str,
    family: &str,
    condition: &str,
    condition_status: CoverageStatus,
    observation_status: CoverageStatus,
    medication_status: CoverageStatus,
) -> (SubjectBriefV1, SubjectCoverageV1) {
    let subject_ref = OpaqueId::new(id);
    let source = format!("synthetic-source-{id}");
    let given_field = field(
        "patient.name.given",
        Some(json!(given)),
        CoverageStatus::Present,
        &source,
    );
    let family_field = field(
        "patient.name.family",
        Some(json!(family)),
        CoverageStatus::Present,
        &source,
    );
    let condition_field = field(
        "condition.code",
        Some(json!(condition)),
        condition_status,
        &source,
    );
    let observation_field = field(
        "observation.valueQuantity",
        (observation_status == CoverageStatus::Present).then(|| json!("synthetic value")),
        observation_status,
        &source,
    );
    let statuses = [condition_status, observation_status, medication_status];
    let brief = SubjectBriefV1 {
        subject_ref: subject_ref.clone(),
        sections: BriefSections {
            identity: vec![given_field, family_field],
            vitals: vec![observation_field.clone()],
            conditions: vec![condition_field.clone()],
            coverage_summary: summary_for(&statuses),
        },
        built_rules_version: PresentationRulesVersion::v1(),
    };
    let coverage = SubjectCoverageV1 {
        subject_ref,
        slots: vec![
            CoverageSlot {
                concept_key: "condition.code".to_owned(),
                status: condition_status,
                values: vec![condition_field],
                conflict: None,
                notes: vec![],
            },
            CoverageSlot {
                concept_key: "observation.valueQuantity".to_owned(),
                status: observation_status,
                values: vec![observation_field],
                conflict: None,
                notes: vec![],
            },
            CoverageSlot {
                concept_key: "MedicationRequest".to_owned(),
                status: medication_status,
                values: vec![],
                conflict: None,
                notes: vec!["Medication projection is intentionally not inferred.".to_owned()],
            },
        ],
        built_rules_version: PresentationRulesVersion::v1(),
    };
    (brief, coverage)
}

fn synthetic_subjects() -> Vec<(SubjectBriefV1, SubjectCoverageV1)> {
    vec![
        synthetic_subject(
            "synthetic-patient-sarah-chen",
            "Sarah",
            "Chen",
            "Type 2 diabetes mellitus",
            CoverageStatus::Present,
            CoverageStatus::Present,
            CoverageStatus::UnsupportedResourceType,
        ),
        synthetic_subject(
            "synthetic-patient-michael-torres",
            "Michael",
            "Torres",
            "Hypertension",
            CoverageStatus::Present,
            CoverageStatus::Unknown,
            CoverageStatus::UnsupportedResourceType,
        ),
        synthetic_subject(
            "synthetic-patient-emily-watson",
            "Emily",
            "Watson",
            "Asthma",
            CoverageStatus::Conflict,
            CoverageStatus::IncomparableUnits,
            CoverageStatus::UnsupportedResourceType,
        ),
        synthetic_subject(
            "synthetic-patient-james-miller",
            "James",
            "Miller",
            "Heart failure",
            CoverageStatus::Present,
            CoverageStatus::UnhealthyEvidence,
            CoverageStatus::Absent,
        ),
    ]
}

fn synthetic_retrieval() -> LexicalRetrieveResult {
    LexicalRetrieveResult {
        corpus_id: "synthetic-lexical".to_owned(),
        corpus_version: "1.0.0".to_owned(),
        corpus_source_identity: "synthetic-lexical@1.0.0".to_owned(),
        query: "population evidence review".to_owned(),
        hits: vec![
            RetrievalHit {
                doc_id: OpaqueId::new("synthetic-evidence-active"),
                score: 1.0,
                snippet: "Synthetic evidence note about reviewing unresolved coverage before consequential action."
                    .to_owned(),
                relevance_only: true,
                retracted: false,
                freshness_marker: Some("synthetic-current".to_owned()),
                conflict_marker: None,
                corpus_version: "1.0.0".to_owned(),
            },
            RetrievalHit {
                doc_id: OpaqueId::new("synthetic-evidence-retracted"),
                score: 0.5,
                snippet: "Synthetic retracted evidence retained only to demonstrate visible lifecycle metadata."
                    .to_owned(),
                relevance_only: true,
                retracted: true,
                freshness_marker: Some("synthetic-historical".to_owned()),
                conflict_marker: Some("retracted-example".to_owned()),
                corpus_version: "1.0.0".to_owned(),
            },
        ],
        evaluation_id: OpaqueId::new("eval-synthetic-population"),
        evidence_only: true,
        relevance_is_not_authority: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_population_preserves_coverage_and_no_risk_claim() {
        let vm = PopulationInsightsVm::synthetic_demo();
        assert!(vm.synthetic_only);
        assert_eq!(vm.cohort_size, 4);
        assert_eq!(vm.review_attention_count, 3);
        for status in [
            "Present",
            "Absent",
            "Unknown",
            "Conflict",
            "Incomparable units",
            "Evidence unhealthy",
            "Unsupported resource",
        ] {
            assert!(
                vm.coverage_distribution
                    .iter()
                    .any(|row| row.label == status)
            );
        }
        assert!(vm.risk_state.contains("no qualified risk contract"));
        assert!(vm.gap_state.contains("not clinical care-gap claims"));
    }

    #[test]
    fn assistant_is_bounded_and_relevance_never_becomes_authority() {
        let vm = PopulationInsightsVm::synthetic_demo();
        let risk = vm.answer_query("Who is highest risk?");
        assert!(risk.contains("no qualified risk contract"));
        assert!(risk.contains("relevance is not clinical authority"));
        assert!(
            vm.evidence
                .iter()
                .any(|card| card.metadata.contains("RETRACTED"))
        );
        assert!(
            vm.evidence
                .iter()
                .all(|card| card.metadata.contains("relevance only"))
        );
    }

    #[test]
    fn mismatched_subjects_fail_closed() {
        let mut subjects = synthetic_subjects();
        subjects[0].1.subject_ref = OpaqueId::new("different-subject");
        assert_eq!(
            PopulationInsightsVm::from_trusted(&subjects, &synthetic_retrieval()),
            Err(PopulationInsightsError::SubjectMismatch)
        );
    }

    #[test]
    fn dishonest_retrieval_envelope_fails_closed() {
        let subjects = synthetic_subjects();
        let mut retrieval = synthetic_retrieval();
        retrieval.relevance_is_not_authority = false;
        assert_eq!(
            PopulationInsightsVm::from_trusted(&subjects, &retrieval),
            Err(PopulationInsightsError::RetrievalAuthorityBoundaryViolated)
        );
    }
}

//! Read-only patient workspace presentation boundary for Spec 061.
//!
//! This module maps existing trusted presentation contracts into Desktop display state.
//! It never opens storage, keys, network clients, or commits authority-changing actions.

use medscale_contracts::objects::{MedicalTime, OpaqueId, TimePrecision};
use medscale_contracts::presentation::{
    BriefSections, CoverageSlot, CoverageStatus, CoverageSummary, ExtractedField,
    PresentationRulesVersion, SubjectBriefV1, SubjectCoverageV1, SubjectTimelineV1, TimelineEvent,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRowVm {
    pub date: String,
    pub title: String,
    pub detail: String,
    pub source: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabRowVm {
    pub label: String,
    pub value: String,
    pub unit: String,
    pub status: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRowVm {
    pub source_id: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatientWorkspaceVm {
    pub synthetic_only: bool,
    pub subject_ref: String,
    pub display_name: String,
    pub birth_date: String,
    pub condition_summary: String,
    pub coverage_summary: String,
    pub coverage_state: String,
    pub timeline: Vec<TimelineRowVm>,
    pub labs: Vec<LabRowVm>,
    pub sources: Vec<SourceRowVm>,
    pub medications_state: String,
    pub documents_state: String,
    pub care_plan_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatientWorkspaceError {
    SubjectMismatch,
}

impl PatientWorkspaceVm {
    pub fn from_trusted(
        brief: &SubjectBriefV1,
        timeline: &SubjectTimelineV1,
        coverage: &SubjectCoverageV1,
    ) -> Result<Self, PatientWorkspaceError> {
        if brief.subject_ref != timeline.subject_ref || brief.subject_ref != coverage.subject_ref {
            return Err(PatientWorkspaceError::SubjectMismatch);
        }

        let given = first_field_value(&brief.sections.identity, "patient.name.given");
        let family = first_field_value(&brief.sections.identity, "patient.name.family");
        let display_name = match (given, family) {
            (Some(g), Some(f)) => format!("{g} {f}"),
            (Some(g), None) => g,
            (None, Some(f)) => f,
            (None, None) => format!("Subject {}", brief.subject_ref.as_str()),
        };
        let birth_date = first_field_value(&brief.sections.identity, "patient.birthDate")
            .unwrap_or_else(|| "Unknown — not inferred".to_owned());

        let conditions: Vec<String> = brief
            .sections
            .conditions
            .iter()
            .map(format_field_with_status)
            .collect();
        let condition_summary = if conditions.is_empty() {
            "No supported condition facts in this projection".to_owned()
        } else {
            conditions.join(" · ")
        };

        let labs = brief
            .sections
            .vitals
            .iter()
            .map(|field| LabRowVm {
                label: humanize_field_key(&field.field_key),
                value: field
                    .value
                    .as_ref()
                    .map(value_text)
                    .unwrap_or_else(|| "—".to_owned()),
                unit: field.unit.clone().unwrap_or_else(|| "No unit".to_owned()),
                status: status_label(field.status).to_owned(),
                source: evidence_label(&field.evidence_refs),
            })
            .collect();

        let timeline_rows = timeline
            .events
            .iter()
            .map(|event| TimelineRowVm {
                date: medical_time_label(
                    event
                        .effective_time
                        .as_ref()
                        .unwrap_or(&event.recorded_time),
                ),
                title: event.claim_kind.clone(),
                detail: summary_text(&event.summary),
                source: evidence_label(&event.evidence_refs),
                status: if event.evidence_refs.is_empty() {
                    "No source reference".to_owned()
                } else {
                    "Source linked".to_owned()
                },
            })
            .collect::<Vec<_>>();

        let mut source_ids = BTreeSet::new();
        for field in brief
            .sections
            .identity
            .iter()
            .chain(brief.sections.vitals.iter())
            .chain(brief.sections.conditions.iter())
        {
            for source in &field.evidence_refs {
                source_ids.insert(source.as_str().to_owned());
            }
        }
        for event in &timeline.events {
            for source in &event.evidence_refs {
                source_ids.insert(source.as_str().to_owned());
            }
        }
        let sources = source_ids
            .into_iter()
            .map(|source_id| SourceRowVm {
                source_id,
                note: "Synthetic fixture evidence; provenance is visible but does not create clinical authority."
                    .to_owned(),
            })
            .collect();

        let summary = &brief.sections.coverage_summary;
        let coverage_summary = format!(
            "{} present · {} absent · {} unknown · {} conflict · {} unsupported",
            summary.present,
            summary.absent,
            summary.unknown,
            summary.conflict + summary.incomparable_units + summary.unhealthy_evidence,
            summary.unsupported_resource_type
        );
        let coverage_state = if summary.conflict
            + summary.incomparable_units
            + summary.unhealthy_evidence
            + summary.unsupported_resource_type
            > 0
        {
            "Needs review"
        } else {
            "Supported evidence"
        }
        .to_owned();

        Ok(Self {
            synthetic_only: true,
            subject_ref: brief.subject_ref.as_str().to_owned(),
            display_name,
            birth_date,
            condition_summary,
            coverage_summary,
            coverage_state,
            timeline: timeline_rows,
            labs,
            sources,
            medications_state: "Unsupported trusted projection — no medication regimen is inferred or displayed as patient truth."
                .to_owned(),
            documents_state: "No admitted document-understanding projection in this demo. Source custody does not imply OCR/ASR completeness."
                .to_owned(),
            care_plan_state: "Review-first only — prepare a proposal in Workflows; no authority-changing action is committed here."
                .to_owned(),
        })
    }

    pub fn synthetic_demo() -> Self {
        let (brief, timeline, coverage) = synthetic_contracts();
        Self::from_trusted(&brief, &timeline, &coverage)
            .expect("synthetic patient projections share one subject")
    }
}

fn first_field_value(fields: &[ExtractedField], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|field| field.field_key == key && field.status == CoverageStatus::Present)
        .and_then(|field| field.value.as_ref())
        .map(value_text)
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

fn format_field_with_status(field: &ExtractedField) -> String {
    let value = field
        .value
        .as_ref()
        .map(value_text)
        .unwrap_or_else(|| "—".to_owned());
    if field.status == CoverageStatus::Present {
        value
    } else {
        format!("{value} ({})", status_label(field.status))
    }
}

fn humanize_field_key(key: &str) -> String {
    match key {
        "observation.valueQuantity" => "Observation".to_owned(),
        _ => key.rsplit('.').next().unwrap_or(key).replace('_', " "),
    }
}

fn status_label(status: CoverageStatus) -> &'static str {
    match status {
        CoverageStatus::Present => "Present",
        CoverageStatus::Absent => "Absent",
        CoverageStatus::Unknown => "Unknown",
        CoverageStatus::Conflict => "Conflict — unresolved",
        CoverageStatus::IncomparableUnits => "Incomparable units",
        CoverageStatus::UnhealthyEvidence => "Evidence unhealthy",
        CoverageStatus::UnsupportedResourceType => "Unsupported resource type",
    }
}

fn evidence_label(evidence_refs: &[OpaqueId]) -> String {
    match evidence_refs {
        [] => "No source reference".to_owned(),
        [one] => format!("Source {}", one.as_str()),
        many => format!("{} source references", many.len()),
    }
}

fn medical_time_label(time: &MedicalTime) -> String {
    if time.approximate {
        format!("~{}", time.value)
    } else {
        time.value.clone()
    }
}

fn summary_text(summary: &Value) -> String {
    summary
        .get("display")
        .or_else(|| summary.get("text"))
        .map(value_text)
        .unwrap_or_else(|| value_text(summary))
}

fn synthetic_field(key: &str, value: Value, source: &str, unit: Option<&str>) -> ExtractedField {
    ExtractedField {
        field_key: key.to_owned(),
        value: Some(value),
        status: CoverageStatus::Present,
        evidence_refs: vec![OpaqueId::new(source)],
        span_citations: vec![],
        unit: unit.map(str::to_owned),
        unit_semantic: None,
    }
}

fn synthetic_contracts() -> (SubjectBriefV1, SubjectTimelineV1, SubjectCoverageV1) {
    let subject = OpaqueId::new("synthetic-patient-sarah-chen");
    let given = synthetic_field(
        "patient.name.given",
        json!("Sarah"),
        "synthetic-fhir-patient",
        None,
    );
    let family = synthetic_field(
        "patient.name.family",
        json!("Chen"),
        "synthetic-fhir-patient",
        None,
    );
    let birth_date = synthetic_field(
        "patient.birthDate",
        json!("1984-05-21"),
        "synthetic-fhir-patient",
        None,
    );
    let lab = synthetic_field(
        "observation.valueQuantity",
        json!(7.4),
        "synthetic-fhir-observation",
        Some("%"),
    );
    let condition = synthetic_field(
        "condition.code",
        json!("Type 2 diabetes mellitus"),
        "synthetic-fhir-condition",
        None,
    );
    let coverage_summary = CoverageSummary {
        present: 3,
        absent: 0,
        unknown: 0,
        conflict: 0,
        incomparable_units: 0,
        unhealthy_evidence: 0,
        unsupported_resource_type: 1,
    };
    let brief = SubjectBriefV1 {
        subject_ref: subject.clone(),
        sections: BriefSections {
            identity: vec![given.clone(), family.clone(), birth_date.clone()],
            vitals: vec![lab.clone()],
            conditions: vec![condition.clone()],
            coverage_summary,
        },
        built_rules_version: PresentationRulesVersion::v1(),
    };
    let timeline = SubjectTimelineV1 {
        subject_ref: subject.clone(),
        events: vec![
            TimelineEvent {
                event_id: "evt:synthetic-condition".to_owned(),
                assertion_id: OpaqueId::new("assertion-synthetic-condition"),
                subject_ref: subject.clone(),
                claim_kind: "Condition".to_owned(),
                summary: json!({"display":"Type 2 diabetes mellitus reviewed"}),
                effective_time: Some(MedicalTime::new("2026-08-19", TimePrecision::Day, false)),
                recorded_time: MedicalTime::new("2026-08-19", TimePrecision::Day, false),
                sort_key: "2026-08-19:condition".to_owned(),
                evidence_refs: vec![OpaqueId::new("synthetic-fhir-condition")],
                span_citations: vec![],
            },
            TimelineEvent {
                event_id: "evt:synthetic-lab".to_owned(),
                assertion_id: OpaqueId::new("assertion-synthetic-lab"),
                subject_ref: subject.clone(),
                claim_kind: "Observation".to_owned(),
                summary: json!({"display":"HbA1c-style synthetic observation: 7.4%"}),
                effective_time: Some(MedicalTime::new("2026-09-03", TimePrecision::Day, false)),
                recorded_time: MedicalTime::new("2026-09-03", TimePrecision::Day, false),
                sort_key: "2026-09-03:observation".to_owned(),
                evidence_refs: vec![OpaqueId::new("synthetic-fhir-observation")],
                span_citations: vec![],
            },
            TimelineEvent {
                event_id: "evt:synthetic-follow-up".to_owned(),
                assertion_id: OpaqueId::new("assertion-synthetic-follow-up"),
                subject_ref: subject.clone(),
                claim_kind: "Patient".to_owned(),
                summary: json!({"display":"Synthetic longitudinal record refreshed"}),
                effective_time: Some(MedicalTime::new("2026-09-12", TimePrecision::Day, false)),
                recorded_time: MedicalTime::new("2026-09-12", TimePrecision::Day, false),
                sort_key: "2026-09-12:patient".to_owned(),
                evidence_refs: vec![OpaqueId::new("synthetic-fhir-patient")],
                span_citations: vec![],
            },
        ],
        built_rules_version: PresentationRulesVersion::v1(),
    };
    let coverage = SubjectCoverageV1 {
        subject_ref: subject,
        slots: vec![
            CoverageSlot {
                concept_key: "patient.birthDate".to_owned(),
                status: CoverageStatus::Present,
                values: vec![birth_date],
                conflict: None,
                notes: vec![],
            },
            CoverageSlot {
                concept_key: "observation.valueQuantity".to_owned(),
                status: CoverageStatus::Present,
                values: vec![lab],
                conflict: None,
                notes: vec![],
            },
            CoverageSlot {
                concept_key: "condition.code".to_owned(),
                status: CoverageStatus::Present,
                values: vec![condition],
                conflict: None,
                notes: vec![],
            },
            CoverageSlot {
                concept_key: "MedicationRequest".to_owned(),
                status: CoverageStatus::UnsupportedResourceType,
                values: vec![],
                conflict: None,
                notes: vec![
                    "MedicationRequest is not a supported trusted presentation extractor."
                        .to_owned(),
                ],
            },
        ],
        built_rules_version: PresentationRulesVersion::v1(),
    };
    (brief, timeline, coverage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_demo_is_explicit_and_source_aware() {
        let vm = PatientWorkspaceVm::synthetic_demo();
        assert!(vm.synthetic_only);
        assert_eq!(vm.display_name, "Sarah Chen");
        assert_eq!(vm.timeline.len(), 3);
        assert_eq!(vm.labs.len(), 1);
        assert_eq!(vm.sources.len(), 3);
        assert!(vm.coverage_state.contains("review"));
        assert!(
            vm.medications_state
                .contains("Unsupported trusted projection")
        );
        assert!(
            vm.documents_state
                .contains("does not imply OCR/ASR completeness")
        );
        assert!(vm.care_plan_state.contains("no authority-changing action"));
    }

    #[test]
    fn mismatched_subjects_fail_closed() {
        let (brief, mut timeline, coverage) = synthetic_contracts();
        timeline.subject_ref = OpaqueId::new("other-subject");
        assert_eq!(
            PatientWorkspaceVm::from_trusted(&brief, &timeline, &coverage),
            Err(PatientWorkspaceError::SubjectMismatch)
        );
    }

    #[test]
    fn conflict_and_unknown_are_not_flattened_to_present() {
        let (mut brief, timeline, coverage) = synthetic_contracts();
        brief.sections.conditions[0].status = CoverageStatus::Conflict;
        let vm = PatientWorkspaceVm::from_trusted(&brief, &timeline, &coverage).unwrap();
        assert!(vm.condition_summary.contains("Conflict — unresolved"));
    }
}

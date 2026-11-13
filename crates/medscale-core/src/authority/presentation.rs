//! H0-B presentation orchestration (timeline / brief / coverage / drill-down).

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::objects::{
    ClinicalAssertion, DigestSha256, MedicalTime, ObjectHeader, OpaqueId, Projection, Proposal,
    SourceRecord, TimePrecision,
};
use medscale_contracts::presentation::{
    BriefSections, ConflictSet, CoverageSlot, CoverageStatus, CoverageSummary, DrillDownResult,
    ExtractedField, KIND_SUBJECT_BRIEF_V1, KIND_SUBJECT_COVERAGE_V1, KIND_SUBJECT_TIMELINE_V1,
    PresentationRulesVersion, SpanCitation, SubjectBriefV1, SubjectCoverageV1, SubjectTimelineV1,
    TimelineEvent, WholeResourceCitation,
};
use medscale_contracts::text::{CoordinateSystem, TextRepresentation, TextSpan};
use medscale_fhir::{ExtractContext, extract_resource};
use serde_json::Value;

use super::store::{InMemoryAuthorityStore, StoredObject};

/// Blob bytes + health flag from vault or memory fallback.
pub type BlobLookupFn<'a> = dyn Fn(&DigestSha256) -> Option<(Vec<u8>, bool)> + 'a;

/// Resolve FHIR resource Value from assertion payload.
fn resource_from_payload(payload: &Value) -> &Value {
    payload
        .get("resource")
        .filter(|v| v.is_object())
        .unwrap_or(payload)
}

fn proposal_for(store: &InMemoryAuthorityStore, assertion: &ClinicalAssertion) -> Option<Proposal> {
    let pid = assertion.promoted_from_proposal_id.as_ref()?;
    match store.objects_raw().get(pid.as_str()) {
        Some(StoredObject::Proposal(p)) => Some(p.clone()),
        _ => None,
    }
}

fn source_for(store: &InMemoryAuthorityStore, id: &OpaqueId) -> Option<SourceRecord> {
    match store.objects_raw().get(id.as_str()) {
        Some(StoredObject::Source(s)) => Some(s.clone()),
        _ => None,
    }
}

fn build_ctx(
    store: &InMemoryAuthorityStore,
    assertion: &ClinicalAssertion,
    blob_bytes: Option<&[u8]>,
    blob_ok: bool,
    resource: &Value,
) -> ExtractContext {
    let proposal = proposal_for(store, assertion);
    let source_id = proposal
        .as_ref()
        .and_then(|p| p.evidence_refs.first().cloned());
    let source = source_id.as_ref().and_then(|id| source_for(store, id));
    ExtractContext {
        source_id: source_id.clone(),
        content_digest: source.as_ref().map(|s| s.content_digest.clone()),
        source_bytes: blob_bytes.map(|b| b.to_vec()),
        evidence_healthy: source_id.is_none() || blob_ok,
        resource_id: resource
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned),
    }
}

fn effective_for_timeline(assertion: &ClinicalAssertion, resource: &Value) -> Option<MedicalTime> {
    if let Some(t) = &assertion.effective_time {
        return Some(t.clone());
    }
    let s = resource
        .get("effectiveDateTime")
        .or_else(|| resource.get("effectiveInstant"))
        .or_else(|| resource.get("onsetDateTime"))
        .and_then(|v| v.as_str())?;
    Some(MedicalTime::new(
        s.to_owned(),
        TimePrecision::Instant,
        false,
    ))
}

fn sort_key(effective: &Option<MedicalTime>, recorded: &MedicalTime, assertion_id: &str) -> String {
    // Precision-aware: Year/Month/Unknown sort as ranges, never as false Instant.
    let eff = effective
        .as_ref()
        .map(MedicalTime::timeline_sort_key)
        .unwrap_or_else(|| "~|p00|~|~|_".to_owned());
    format!("{eff}|{}|{assertion_id}", recorded.timeline_sort_key())
}

/// Build SubjectTimelineV1 from promoted ClinicalAssertions for a subject.
pub fn build_timeline(
    store: &InMemoryAuthorityStore,
    subject_ref: &OpaqueId,
    blob_lookup: &BlobLookupFn<'_>,
) -> SubjectTimelineV1 {
    let superseded = store.superseded_assertion_ids();
    let mut events = Vec::new();
    for assertion in store.assertions_for_subject(subject_ref) {
        if superseded.contains(assertion.header.id.as_str()) {
            continue;
        }
        let resource = resource_from_payload(&assertion.payload);
        let (bytes, ok) = resolve_blob(store, &assertion, blob_lookup);
        let ctx = build_ctx(store, &assertion, bytes.as_deref(), ok, resource);
        let extraction = extract_resource(resource, &ctx);
        let evidence_refs = ctx.source_id.iter().cloned().collect::<Vec<_>>();
        let span_citations = extraction
            .fields
            .iter()
            .flat_map(|f| f.span_citations.clone())
            .collect();
        let effective = effective_for_timeline(&assertion, resource);
        let key = sort_key(
            &effective,
            &assertion.recorded_time,
            assertion.header.id.as_str(),
        );
        events.push(TimelineEvent {
            event_id: format!("evt:{}", assertion.header.id.as_str()),
            assertion_id: assertion.header.id.clone(),
            subject_ref: assertion.subject_ref.clone(),
            claim_kind: assertion.claim_kind.clone(),
            summary: extraction.summary,
            effective_time: effective,
            recorded_time: assertion.recorded_time.clone(),
            sort_key: key,
            evidence_refs,
            span_citations,
        });
    }
    events.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    SubjectTimelineV1 {
        subject_ref: subject_ref.clone(),
        events,
        built_rules_version: PresentationRulesVersion::v1(),
    }
}

fn resolve_blob(
    store: &InMemoryAuthorityStore,
    assertion: &ClinicalAssertion,
    blob_lookup: &BlobLookupFn<'_>,
) -> (Option<Vec<u8>>, bool) {
    let Some(proposal) = proposal_for(store, assertion) else {
        return (None, true);
    };
    let Some(sid) = proposal.evidence_refs.first() else {
        return (None, true);
    };
    let Some(source) = source_for(store, sid) else {
        return (None, false);
    };
    if let Some((bytes, ok)) = blob_lookup(&source.content_digest) {
        return (Some(bytes), ok);
    }
    // Fall back to in-memory SourceRecord bytes (Spec 003 also keeps them).
    if source.digest_valid() && !source.bytes.is_empty() {
        return (Some(source.bytes.clone()), true);
    }
    (None, false)
}

/// Two present values agree when they state the same thing: equal as JSON, or
/// equal numbers (`5` and `5.0`), or the same quantity in two units of one
/// group of the admitted UCUM subset (`5 kg` and `5000 g`). Anything else,
/// including units that need an analyte-specific conversion, still counts as
/// a disagreement and is kept for review.
fn fields_equal_value(a: &ExtractedField, b: &ExtractedField) -> bool {
    if a.value == b.value && a.unit == b.unit {
        return true;
    }
    // Decimals arrive as their exact text (B2), integers as numbers.
    let number = |f: &ExtractedField| {
        f.value
            .as_ref()
            .and_then(|v| v.as_f64().or_else(|| v.as_str()?.trim().parse().ok()))
    };
    let (Some(x), Some(y)) = (number(a), number(b)) else {
        return false;
    };
    match (a.unit.as_deref(), b.unit.as_deref()) {
        (None, None) => x == y,
        (Some(ua), Some(ub)) if ua.trim() == ub.trim() => x == y,
        (Some(ua), Some(ub)) => medscale_fhir::same_quantity(x, ua, y, ub).unwrap_or(false),
        _ => false,
    }
}

/// Aggregate coverage slots from extractions.
pub fn build_coverage(
    store: &InMemoryAuthorityStore,
    subject_ref: &OpaqueId,
    blob_lookup: &BlobLookupFn<'_>,
) -> SubjectCoverageV1 {
    let mut by_key: std::collections::BTreeMap<String, Vec<ExtractedField>> =
        std::collections::BTreeMap::new();
    let mut unsupported_slots = Vec::new();

    for assertion in store.assertions_for_subject(subject_ref) {
        let resource = resource_from_payload(&assertion.payload);
        let (bytes, ok) = resolve_blob(store, &assertion, blob_lookup);
        let ctx = build_ctx(store, &assertion, bytes.as_deref(), ok, resource);
        let extraction = extract_resource(resource, &ctx);
        if extraction.unsupported {
            unsupported_slots.push(CoverageSlot {
                concept_key: format!("unsupported.{}", extraction.resource_type),
                status: CoverageStatus::UnsupportedResourceType,
                values: extraction.fields.clone(),
                conflict: None,
                notes: vec!["unsupported_resource_type".to_owned()],
            });
            continue;
        }
        for field in extraction.fields {
            by_key
                .entry(field.field_key.clone())
                .or_default()
                .push(field);
        }
    }

    let mut slots = Vec::new();
    for (concept_key, values) in by_key {
        let present_vals: Vec<&ExtractedField> = values
            .iter()
            .filter(|f| {
                f.status == CoverageStatus::Present || f.status == CoverageStatus::UnhealthyEvidence
            })
            .collect();
        let all_absent = values.iter().all(|f| f.status == CoverageStatus::Absent);
        let any_unhealthy = values
            .iter()
            .any(|f| f.status == CoverageStatus::UnhealthyEvidence);

        let (status, conflict) = if any_unhealthy && present_vals.is_empty() {
            (CoverageStatus::UnhealthyEvidence, None)
        } else if all_absent {
            (CoverageStatus::Absent, None)
        } else if let Some(unit_st) = medscale_fhir::unit_conflict_status(&present_vals) {
            (unit_st, None)
        } else if present_vals.len() >= 2 {
            let first = present_vals[0];
            let disagree = present_vals.iter().any(|f| !fields_equal_value(first, f));
            if disagree {
                (
                    CoverageStatus::Conflict,
                    Some(ConflictSet {
                        claim_key: concept_key.clone(),
                        members: present_vals.iter().map(|f| (*f).clone()).collect(),
                        resolution: "Unresolved".to_owned(),
                    }),
                )
            } else {
                (CoverageStatus::Present, None)
            }
        } else if present_vals.len() == 1 {
            (present_vals[0].status, None)
        } else {
            (CoverageStatus::Unknown, None)
        };

        slots.push(CoverageSlot {
            concept_key,
            status,
            values,
            conflict,
            notes: vec![],
        });
    }
    slots.extend(unsupported_slots);
    slots.sort_by(|a, b| a.concept_key.cmp(&b.concept_key));

    SubjectCoverageV1 {
        subject_ref: subject_ref.clone(),
        slots,
        built_rules_version: PresentationRulesVersion::v1(),
    }
}

fn summarize_coverage(slots: &[CoverageSlot]) -> CoverageSummary {
    let mut s = CoverageSummary {
        present: 0,
        absent: 0,
        unknown: 0,
        conflict: 0,
        incomparable_units: 0,
        unhealthy_evidence: 0,
        unsupported_resource_type: 0,
    };
    for slot in slots {
        match slot.status {
            CoverageStatus::Present => s.present += 1,
            CoverageStatus::Absent => s.absent += 1,
            CoverageStatus::Unknown => s.unknown += 1,
            CoverageStatus::Conflict => s.conflict += 1,
            CoverageStatus::IncomparableUnits => s.incomparable_units += 1,
            CoverageStatus::UnhealthyEvidence => s.unhealthy_evidence += 1,
            CoverageStatus::UnsupportedResourceType => s.unsupported_resource_type += 1,
        }
    }
    s
}

/// Build narrow LLM-free Brief.
pub fn build_brief(
    store: &InMemoryAuthorityStore,
    subject_ref: &OpaqueId,
    blob_lookup: &BlobLookupFn<'_>,
) -> SubjectBriefV1 {
    let coverage = build_coverage(store, subject_ref, blob_lookup);
    let mut identity = Vec::new();
    let mut vitals = Vec::new();
    let mut conditions = Vec::new();

    for slot in &coverage.slots {
        if slot.concept_key.starts_with("patient.") {
            if let Some(f) = slot.values.first() {
                identity.push(f.clone());
            }
        } else if slot.concept_key.starts_with("observation.") {
            if let Some(f) = slot.values.first() {
                vitals.push(f.clone());
            }
        } else if slot.concept_key.starts_with("condition.")
            && let Some(f) = slot.values.first()
        {
            let mut f = f.clone();
            if slot.status == CoverageStatus::Conflict {
                f.status = CoverageStatus::Conflict;
            }
            conditions.push(f);
        }
    }

    SubjectBriefV1 {
        subject_ref: subject_ref.clone(),
        sections: BriefSections {
            identity,
            vitals,
            conditions,
            coverage_summary: summarize_coverage(&coverage.slots),
        },
        built_rules_version: PresentationRulesVersion::v1(),
    }
}

/// Drill down a field to SourceRecord citation.
pub fn drill_down(
    store: &InMemoryAuthorityStore,
    subject_ref: &OpaqueId,
    field_key: &str,
    assertion_id: Option<&OpaqueId>,
    blob_lookup: &BlobLookupFn<'_>,
) -> Result<DrillDownResult, DrillDownError> {
    let assertions = store.assertions_for_subject(subject_ref);
    let assertion = if let Some(aid) = assertion_id {
        assertions
            .into_iter()
            .find(|a| &a.header.id == aid)
            .ok_or(DrillDownError::NotFound)?
    } else {
        assertions
            .into_iter()
            .find(|a| {
                let resource = resource_from_payload(&a.payload);
                let (bytes, ok) = resolve_blob(store, a, blob_lookup);
                let ctx = build_ctx(store, a, bytes.as_deref(), ok, resource);
                extract_resource(resource, &ctx)
                    .fields
                    .iter()
                    .any(|f| f.field_key == field_key && f.status == CoverageStatus::Present)
            })
            .ok_or(DrillDownError::NotFound)?
    };

    let resource = resource_from_payload(&assertion.payload);
    let (bytes, ok) = resolve_blob(store, &assertion, blob_lookup);
    let ctx = build_ctx(store, &assertion, bytes.as_deref(), ok, resource);
    let extraction = extract_resource(resource, &ctx);
    let field = extraction
        .fields
        .iter()
        .find(|f| f.field_key == field_key)
        .ok_or(DrillDownError::NotFound)?;

    if !ok || field.status == CoverageStatus::UnhealthyEvidence {
        return Err(DrillDownError::UnhealthyEvidence);
    }

    let source_id = ctx.source_id.clone().ok_or(DrillDownError::NotFound)?;
    let digest = ctx.content_digest.clone().ok_or(DrillDownError::NotFound)?;
    let source = source_for(store, &source_id).ok_or(DrillDownError::NotFound)?;

    let span = field.span_citations.iter().find_map(|c| match c {
        SpanCitation::TextSpan { span, .. } => Some(span.clone()),
        _ => None,
    });
    let citation = Some(WholeResourceCitation {
        source_id: source_id.clone(),
        content_digest: digest.clone(),
        structural_path_note: Some(field_key.to_owned()),
    });

    Ok(DrillDownResult {
        field_key: field_key.to_owned(),
        source_id,
        content_digest: digest,
        media_type: source.media_type,
        span: span.or(Some(TextSpan {
            representation: TextRepresentation::SourceBytes,
            coordinate_system: CoordinateSystem::RawByte,
            start: 0,
            end: (source.bytes.len() as u64).min(64),
        })),
        citation,
        excerpt_lossy: None,
        status: field.status,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrillDownError {
    NotFound,
    UnhealthyEvidence,
}

/// Persist a presentation Projection body.
#[allow(dead_code)]
pub fn persist_projection(
    store: &mut InMemoryAuthorityStore,
    realm_id: medscale_contracts::objects::RealmId,
    scope_id: medscale_contracts::objects::AuthorityScopeId,
    kind: &str,
    built_from: Vec<OpaqueId>,
    body: Value,
) -> Projection {
    let id = store.alloc_id("proj");
    let proj = Projection {
        header: ObjectHeader {
            id: id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id: scope_id,
        },
        projection_kind: kind.to_owned(),
        built_from,
        built_at: MedicalTime::new("1970-01-01T00:00:00Z", TimePrecision::Instant, false),
        body,
        authoritative: false,
    };
    store.insert(StoredObject::Projection(proj.clone()));
    proj
}

/// Canonical JSON for equality (serde_json Value already normalizes object key order on serialize).
#[must_use]
#[allow(dead_code)]
pub fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

pub fn body_timeline(t: &SubjectTimelineV1) -> Value {
    serde_json::to_value(t).unwrap_or(Value::Null)
}

pub fn body_brief(b: &SubjectBriefV1) -> Value {
    serde_json::to_value(b).unwrap_or(Value::Null)
}

pub fn body_coverage(c: &SubjectCoverageV1) -> Value {
    serde_json::to_value(c).unwrap_or(Value::Null)
}

pub fn is_presentation_kind(kind: &str) -> bool {
    matches!(
        kind,
        KIND_SUBJECT_TIMELINE_V1 | KIND_SUBJECT_BRIEF_V1 | KIND_SUBJECT_COVERAGE_V1
    )
}

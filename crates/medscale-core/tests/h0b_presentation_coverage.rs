//! Spec 004 H0-B presentation + coverage integration tests.

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::presentation::{
    CoverageStatus, KIND_SUBJECT_BRIEF_V1, KIND_SUBJECT_COVERAGE_V1, KIND_SUBJECT_TIMELINE_V1,
};
use medscale_core::CoreFacade;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

fn realm() -> RealmId {
    RealmId::new("realm-004")
}
fn scope() -> AuthorityScopeId {
    AuthorityScopeId::new("scope-004")
}
fn vault() -> VaultId {
    VaultId::new("vault-004")
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("req-{}", uuid_like())),
        vault(),
        realm(),
        scope(),
        cap,
        body,
    )
}

fn uuid_like() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn fixture(name: &str) -> Vec<u8> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("fixtures/synthetic/fhir/r4/presentation");
    p.push(name);
    fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn open_and_lease(facade: &CoreFacade) -> OpaqueId {
    let root = std::env::temp_dir().join(format!("medscale-004-{}", uuid_like()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let lease = facade
        .dispatch(req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("cli-004"),
                holder_id_hint: None,
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Lease { holder_id, .. } = lease else {
        panic!("lease");
    };
    facade
        .dispatch(req(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: root.to_string_lossy().into_owned(),
            },
        ))
        .result
        .unwrap();
    holder_id
}

fn ingest_promote(
    facade: &CoreFacade,
    bytes: Vec<u8>,
    claim_kind: &str,
    subject: &OpaqueId,
    effective: Option<&str>,
) -> OpaqueId {
    let ingested = facade
        .dispatch(req(
            Capability::IngestFhirSynthetic,
            RequestBody::IngestFhirSynthetic {
                media_type: "application/fhir+json".to_owned(),
                bytes: bytes.clone(),
                fhir_version_hint: Some("4.0.1".to_owned()),
                attach_validator_fixture_id: None,
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Ingested { receipt } = ingested else {
        panic!("ingest");
    };
    let source_id = receipt.source_id.expect("source");
    let resource: Value = serde_json::from_slice(&bytes).unwrap();
    let mut payload = json!({ "resource": resource });
    if let Some(eff) = effective {
        payload["effective_hint"] = json!(eff);
    }
    let created = facade
        .dispatch(req(
            Capability::CreateProposal,
            RequestBody::CreateProposal {
                subject_ref: Some(subject.clone()),
                claim_kind: claim_kind.to_owned(),
                payload,
                evidence_refs: vec![source_id],
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Created {
        object_id: proposal_id,
    } = created
    else {
        panic!("proposal");
    };
    let promoted = facade
        .dispatch(req(
            Capability::PromoteProposal,
            RequestBody::PromoteProposal {
                proposal_id,
                authorized_by: OpaqueId::new("clinician-004"),
                subject_ref: subject.clone(),
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Promoted { assertion_id, .. } = promoted else {
        panic!("promote");
    };
    assertion_id
}

#[test]
fn timeline_golden_rebuild() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let _lease = open_and_lease(&facade);
    let subject = OpaqueId::new("subject-timeline");

    ingest_promote(
        &facade,
        fixture("observation-weight.json"),
        "observation",
        &subject,
        Some("2020-01-01T09:00:00Z"),
    );
    ingest_promote(
        &facade,
        fixture("observation-hr.json"),
        "observation",
        &subject,
        Some("2020-01-02T10:00:00Z"),
    );
    ingest_promote(
        &facade,
        fixture("condition-uri.json"),
        "condition",
        &subject,
        None,
    );

    // Proposal-only must not appear on timeline
    facade
        .dispatch(req(
            Capability::CreateProposal,
            RequestBody::CreateProposal {
                subject_ref: Some(subject.clone()),
                claim_kind: "observation".to_owned(),
                payload: json!({"resource": {"resourceType": "Observation", "id": "prop-only"}}),
                evidence_refs: vec![],
            },
        ))
        .result
        .unwrap();

    let tl = facade
        .dispatch(req(
            Capability::GetTimeline,
            RequestBody::GetTimeline {
                subject_ref: subject.clone(),
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Timeline { body, .. } = tl else {
        panic!("timeline");
    };
    assert!(
        body.events.len() >= 3,
        "expected >=3 events, got {}",
        body.events.len()
    );
    let keys: Vec<_> = body.events.iter().map(|e| e.sort_key.clone()).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "events must be deterministically ordered");

    let a_ids: Vec<_> = body.events.iter().map(|e| e.assertion_id.clone()).collect();
    let rebuilt1 = facade
        .dispatch(req(
            Capability::RebuildProjection,
            RequestBody::RebuildProjection {
                kind: KIND_SUBJECT_TIMELINE_V1.to_owned(),
                built_from: a_ids.clone(),
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Created { object_id: p1 } = rebuilt1 else {
        panic!("rebuild1");
    };
    let rebuilt2 = facade
        .dispatch(req(
            Capability::RebuildProjection,
            RequestBody::RebuildProjection {
                kind: KIND_SUBJECT_TIMELINE_V1.to_owned(),
                built_from: a_ids,
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Created { object_id: p2 } = rebuilt2 else {
        panic!("rebuild2");
    };
    let o1 = facade
        .dispatch(req(
            Capability::ReadObject,
            RequestBody::ReadObject { object_id: p1 },
        ))
        .result
        .unwrap();
    let o2 = facade
        .dispatch(req(
            Capability::ReadObject,
            RequestBody::ReadObject { object_id: p2 },
        ))
        .result
        .unwrap();
    let ResponseBody::Object { value: v1 } = o1 else {
        panic!();
    };
    let ResponseBody::Object { value: v2 } = o2 else {
        panic!();
    };
    assert_eq!(v1["body"], v2["body"]);
    assert_eq!(v1["authoritative"], false);
}

#[test]
fn coverage_absence_conflict() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let _lease = open_and_lease(&facade);
    let subject = OpaqueId::new("subject-coverage");

    ingest_promote(
        &facade,
        fixture("patient-absence.json"),
        "patient",
        &subject,
        None,
    );
    ingest_promote(
        &facade,
        fixture("condition-uri.json"),
        "condition",
        &subject,
        None,
    );
    ingest_promote(
        &facade,
        fixture("condition-conflict-b.json"),
        "condition",
        &subject,
        None,
    );
    ingest_promote(
        &facade,
        fixture("unsupported-medicationrequest.json"),
        "medication",
        &subject,
        None,
    );

    let cov = facade
        .dispatch(req(
            Capability::GetCoverage,
            RequestBody::GetCoverage {
                subject_ref: subject,
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Coverage { body, .. } = cov else {
        panic!("coverage");
    };

    let birth = body
        .slots
        .iter()
        .find(|s| s.concept_key == "patient.birthDate")
        .expect("birthDate slot");
    assert_eq!(birth.status, CoverageStatus::Absent);

    let code = body
        .slots
        .iter()
        .find(|s| s.concept_key == "condition.code")
        .expect("condition.code");
    assert_eq!(code.status, CoverageStatus::Conflict);
    let conflict = code.conflict.as_ref().expect("conflict set");
    assert!(conflict.members.len() >= 2);
    assert_eq!(conflict.resolution, "Unresolved");

    assert!(body.slots.iter().any(|s| {
        s.status == CoverageStatus::UnsupportedResourceType
            || s.concept_key.contains("MedicationRequest")
    }));
}

#[test]
fn brief_golden() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let _lease = open_and_lease(&facade);
    let subject = OpaqueId::new("subject-brief");
    ingest_promote(
        &facade,
        fixture("patient-golden.json"),
        "patient",
        &subject,
        None,
    );
    ingest_promote(
        &facade,
        fixture("observation-hr.json"),
        "observation",
        &subject,
        None,
    );

    let br = facade
        .dispatch(req(
            Capability::GetBrief,
            RequestBody::GetBrief {
                subject_ref: subject.clone(),
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Brief { body, .. } = br else {
        panic!("brief");
    };
    assert!(!body.sections.identity.is_empty());
    assert!(!body.sections.vitals.is_empty());
    assert_eq!(body.built_rules_version.brief_schema, KIND_SUBJECT_BRIEF_V1);
    // No LLM surface: rules pin is deterministic and extractors-only.
    assert!(
        body.built_rules_version
            .extractors
            .iter()
            .all(|e| e.ends_with(".v1"))
    );

    let _ = facade
        .dispatch(req(
            Capability::RebuildProjection,
            RequestBody::RebuildProjection {
                kind: KIND_SUBJECT_BRIEF_V1.to_owned(),
                built_from: vec![subject],
            },
        ))
        .result
        .unwrap();
}

#[test]
fn presentation_drilldown() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let _lease = open_and_lease(&facade);
    let subject = OpaqueId::new("subject-drill");
    ingest_promote(
        &facade,
        fixture("patient-golden.json"),
        "patient",
        &subject,
        None,
    );

    let dd = facade
        .dispatch(req(
            Capability::DrillDownPresentation,
            RequestBody::DrillDownPresentation {
                subject_ref: subject.clone(),
                field_key: "patient.birthDate".to_owned(),
                assertion_id: None,
            },
        ))
        .result
        .unwrap();
    let ResponseBody::DrillDown { result } = dd else {
        panic!("drilldown");
    };
    assert_eq!(result.field_key, "patient.birthDate");
    assert_eq!(result.status, CoverageStatus::Present);
    assert!(result.citation.is_some());
}

#[test]
fn presentation_golden_rebuild() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let _lease = open_and_lease(&facade);
    let subject = OpaqueId::new("subject-rebuild");
    ingest_promote(
        &facade,
        fixture("patient-golden.json"),
        "patient",
        &subject,
        None,
    );

    let mk = |kind: &str| {
        let r = facade
            .dispatch(req(
                Capability::RebuildProjection,
                RequestBody::RebuildProjection {
                    kind: kind.to_owned(),
                    built_from: vec![subject.clone()],
                },
            ))
            .result
            .unwrap();
        let ResponseBody::Created { object_id } = r else {
            panic!("created");
        };
        let o = facade
            .dispatch(req(
                Capability::ReadObject,
                RequestBody::ReadObject { object_id },
            ))
            .result
            .unwrap();
        let ResponseBody::Object { value } = o else {
            panic!("object");
        };
        value["body"].clone()
    };

    assert_eq!(mk(KIND_SUBJECT_COVERAGE_V1), mk(KIND_SUBJECT_COVERAGE_V1));
    assert_eq!(mk(KIND_SUBJECT_BRIEF_V1), mk(KIND_SUBJECT_BRIEF_V1));
    assert_eq!(mk(KIND_SUBJECT_TIMELINE_V1), mk(KIND_SUBJECT_TIMELINE_V1));

    // Delta after new assertion
    let before = mk(KIND_SUBJECT_TIMELINE_V1);
    ingest_promote(
        &facade,
        fixture("observation-hr.json"),
        "observation",
        &subject,
        None,
    );
    let after = mk(KIND_SUBJECT_TIMELINE_V1);
    assert_ne!(before, after);
}

#[test]
fn no_fhirpath_engine_dependency() {
    let manifest =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"))
            .expect("Cargo.lock");
    for banned in ["fhirpath", "fhir_path", "octofhir", "helios-fhirpath"] {
        assert!(
            !manifest.to_lowercase().contains(banned),
            "banned FHIRPath-related crate mention: {banned}"
        );
    }
}

/// The same quantity written in two units of one admitted group, or as `72`
/// and `72.0`, is not a conflict. A real difference still is.
#[test]
fn coverage_compares_quantities_not_text() {
    let obs = |id: &str, value: &str, code: &str| {
        format!(
            r#"{{"resourceType":"Observation","id":"{id}","status":"final","code":{{"text":"Body weight"}},"valueQuantity":{{"value":{value},"unit":"{code}","system":"http://unitsofmeasure.org","code":"{code}"}}}}"#
        )
        .into_bytes()
    };
    let status_for = |pairs: [(&str, &str, &str); 2], subject: &str| {
        let facade = CoreFacade::new_legacy_lease_only_engineering();
        let _lease = open_and_lease(&facade);
        let subject = OpaqueId::new(subject);
        for (id, value, code) in pairs {
            ingest_promote(&facade, obs(id, value, code), "observation", &subject, None);
        }
        let cov = facade
            .dispatch(req(
                Capability::GetCoverage,
                RequestBody::GetCoverage {
                    subject_ref: subject,
                },
            ))
            .result
            .unwrap();
        let ResponseBody::Coverage { body, .. } = cov else {
            panic!("coverage");
        };
        body.slots
            .iter()
            .find(|s| s.concept_key == "observation.valueQuantity")
            .expect("valueQuantity slot")
            .status
    };
    assert_ne!(
        status_for([("w1", "5", "kg"), ("w2", "5000", "g")], "subject-units-1"),
        CoverageStatus::Conflict,
        "5 kg and 5000 g are the same quantity"
    );
    assert_ne!(
        status_for(
            [("w3", "72", "kg"), ("w4", "72.0", "kg")],
            "subject-units-2"
        ),
        CoverageStatus::Conflict,
        "72 and 72.0 are the same number"
    );
    assert_eq!(
        status_for([("w5", "5", "kg"), ("w6", "6", "kg")], "subject-units-3"),
        CoverageStatus::Conflict,
        "a real difference is still a conflict"
    );
}

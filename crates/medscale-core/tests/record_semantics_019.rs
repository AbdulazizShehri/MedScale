//! Spec 019 record semantics (Trusted V1 Q06).

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{
    AmendmentKind, AuthorityScopeId, IdentityAssertion, MedicalTime, MissingnessKind, ObjectHeader,
    OpaqueId, RealmId, TimePrecision, VaultId, find_unresolved_identity_candidates,
};
use medscale_core::{CoreFacade, build_doctor_report};
use serde_json::json;

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new("scope-a"),
        capability,
        body,
    )
}

fn acquire_lease(facade: &CoreFacade) -> OpaqueId {
    let acquired = facade.dispatch(req(
        Capability::AcquireLease,
        RequestBody::AcquireLease {
            client_id: OpaqueId::new("client-a"),
            holder_id_hint: None,
        },
    ));
    match acquired.result.expect("lease") {
        ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn partial_date_precision_preserved() {
    let t = MedicalTime::parse_validated("2019", TimePrecision::Year, true, None).unwrap();
    assert_eq!(t.precision, TimePrecision::Year);
    assert!(t.approximate);
    let (start, end) = t.precision_bounds();
    assert_eq!(start, "2019-01-01");
    assert_eq!(end, "2019-12-31");
    assert!(!t.timeline_sort_key().contains("Instant"));
    assert!(t.timeline_sort_key().contains("|p01|"));
}

#[test]
fn refuse_false_instant_upgrade() {
    let t = MedicalTime::new("2019-06", TimePrecision::Month, false);
    let err = t.try_with_precision(TimePrecision::Instant).unwrap_err();
    assert!(err.to_string().contains("refusing false"));
}

#[test]
fn missingness_roundtrip() {
    let kinds = [
        MissingnessKind::Unknown,
        MissingnessKind::ExplicitAbsence,
        MissingnessKind::NotObserved,
        MissingnessKind::NotApplicable,
        MissingnessKind::Withheld,
        MissingnessKind::Conflict,
    ];
    for kind in kinds {
        let v = serde_json::to_value(kind).unwrap();
        let back: MissingnessKind = serde_json::from_value(v).unwrap();
        assert_eq!(back, kind);
    }
}

#[test]
fn identity_unresolved_candidates() {
    let a = IdentityAssertion {
        header: ObjectHeader {
            id: OpaqueId::new("ia-1"),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: RealmId::new("realm-a"),
            authority_scope_id: AuthorityScopeId::new("scope-a"),
        },
        subject_id: OpaqueId::new("subj-a"),
        identifier_system: "mrn".to_owned(),
        identifier_value: "SYN-100".to_owned(),
        confidence: None,
        evidence_refs: vec![],
    };
    let b = IdentityAssertion {
        header: ObjectHeader {
            id: OpaqueId::new("ia-2"),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: RealmId::new("realm-a"),
            authority_scope_id: AuthorityScopeId::new("scope-a"),
        },
        subject_id: OpaqueId::new("subj-b"),
        identifier_system: "mrn".to_owned(),
        identifier_value: "SYN-100".to_owned(),
        confidence: None,
        evidence_refs: vec![],
    };
    let set = find_unresolved_identity_candidates(&[a, b], &[]);
    assert_eq!(set.candidates.len(), 1);
    assert_eq!(set.candidates[0].subject_a.as_str(), "subj-a");
    assert_eq!(set.candidates[0].subject_b.as_str(), "subj-b");
}

#[test]
fn amendment_lineage_append_only() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let _holder = acquire_lease(&facade);

    let created = facade.dispatch(req(
        Capability::CreateProposal,
        RequestBody::CreateProposal {
            subject_ref: Some(OpaqueId::new("subj-1")),
            claim_kind: "allergy".to_owned(),
            payload: json!({"code": "peanut"}),
            evidence_refs: vec![],
        },
    ));
    let proposal_id = match created.result.expect("proposal") {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };

    let promoted = facade.dispatch(req(
        Capability::PromoteProposal,
        RequestBody::PromoteProposal {
            proposal_id,
            authorized_by: OpaqueId::new("actor-1"),
            subject_ref: OpaqueId::new("subj-1"),
        },
    ));
    let prior_id = match promoted.result.expect("promote") {
        ResponseBody::Promoted { assertion_id, .. } => assertion_id,
        other => panic!("{other:?}"),
    };

    let amended = facade.dispatch(req(
        Capability::AmendAssertion,
        RequestBody::AmendAssertion {
            prior_assertion_id: prior_id.clone(),
            authorized_by: OpaqueId::new("actor-1"),
            payload: json!({"code": "tree_nut"}),
            effective_time: Some(
                MedicalTime::parse_validated("2020", TimePrecision::Year, false, None).unwrap(),
            ),
            kind: AmendmentKind::Supersession,
            rationale: "corrected allergen class".to_owned(),
        },
    ));
    let (new_id, amend_id, _) = match amended.result.expect("amend") {
        ResponseBody::Amended {
            assertion_id,
            amendment_id,
            audit_id,
        } => (assertion_id, amendment_id, audit_id),
        other => panic!("{other:?}"),
    };
    assert_ne!(prior_id, new_id);
    assert!(amend_id.as_str().starts_with("amend-"));

    // Prior object still readable (append-only; never overwritten).
    let prior = facade.dispatch(req(
        Capability::ReadObject,
        RequestBody::ReadObject {
            object_id: prior_id.clone(),
        },
    ));
    match prior.result.expect("prior") {
        ResponseBody::Object { value } => {
            assert_eq!(value["payload"]["code"], "peanut");
        }
        other => panic!("{other:?}"),
    }

    let amendment = facade.dispatch(req(
        Capability::ReadObject,
        RequestBody::ReadObject {
            object_id: amend_id,
        },
    ));
    match amendment.result.expect("amendment") {
        ResponseBody::Object { value } => {
            assert_eq!(value["prior_assertion_id"], prior_id.as_str());
            assert_eq!(value["superseding_assertion_id"], new_id.as_str());
            assert_eq!(value["kind"], "supersession");
        }
        other => panic!("{other:?}"),
    }

    // Second amend of already-superseded prior fails closed.
    let again = facade.dispatch(req(
        Capability::AmendAssertion,
        RequestBody::AmendAssertion {
            prior_assertion_id: prior_id,
            authorized_by: OpaqueId::new("actor-1"),
            payload: json!({"code": "x"}),
            effective_time: None,
            kind: AmendmentKind::Supersession,
            rationale: "retry".to_owned(),
        },
    ));
    assert!(matches!(
        again.result,
        Err(medscale_contracts::envelopes::AuthorityError::IllegalTransition)
    ));
}

#[test]
fn doctor_record_semantics_axis() {
    let report = build_doctor_report(None, false, false);
    assert!(report.record_semantics.ready_base);
    assert!(report.record_semantics.precision_aware_time);
    assert!(!report.record_semantics.release_ready);
}

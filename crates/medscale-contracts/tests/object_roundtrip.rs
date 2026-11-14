use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::objects::{
    ActionAuditKind, ActionAuditRecord, AuthorityScopeId, ClinicalAssertion, DerivedSourceArtifact,
    DigestSha256, EvaluationRecord, IdentityAssertion, IdentityMergeDecision, LossClass,
    MedicalTime, ObjectHeader, OpaqueId, ProducerKind, Projection, Proposal, RealmId,
    RepresentationKind, SourceRecord, TimePrecision,
};
use serde_json::json;

fn header(id: &str) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: AUTHORITY_SCHEMA_VERSION,
        realm_id: RealmId::new("realm-a"),
        authority_scope_id: AuthorityScopeId::new("scope-a"),
    }
}

fn instant() -> MedicalTime {
    MedicalTime::new("1970-01-01T00:00:00Z", TimePrecision::Instant, false)
}

#[test]
fn object_roundtrip_all_classes() {
    let source = SourceRecord {
        header: header("src-1"),
        bytes: b"hello".to_vec(),
        content_digest: DigestSha256::of(b"hello"),
        media_type: "text/plain".to_owned(),
        acquired_at: None,
        provenance_note: None,
    };
    let derived = DerivedSourceArtifact {
        header: header("der-1"),
        source_id: OpaqueId::new("src-1"),
        transform_id: "normalize.nfc.v1".to_owned(),
        transform_version: "1".to_owned(),
        loss_class: LossClass::Lossless,
        representation: RepresentationKind::NormalizedText,
        bytes: b"hello".to_vec(),
        content_digest: DigestSha256::of(b"hello"),
        parent_span_map_ref: None,
    };
    let proposal = Proposal {
        header: header("prop-1"),
        subject_ref: Some(OpaqueId::new("subj-1")),
        claim_kind: "allergy".to_owned(),
        payload: json!({"code": "x"}),
        confidence: Some(0.5),
        evidence_refs: vec![OpaqueId::new("src-1")],
        producer: ProducerKind::Rule,
    };
    let assertion = ClinicalAssertion {
        header: header("assert-1"),
        subject_ref: OpaqueId::new("subj-1"),
        claim_kind: "allergy".to_owned(),
        payload: json!({"code": "x"}),
        promoted_from_proposal_id: Some(OpaqueId::new("prop-1")),
        authorized_by: OpaqueId::new("actor-1"),
        effective_time: None,
        recorded_time: instant(),
    };
    let evaluation = EvaluationRecord {
        header: header("eval-1"),
        target_refs: vec![OpaqueId::new("assert-1")],
        evaluator: "rule.v1".to_owned(),
        result: json!({"ok": true}),
        evidence_only: true,
    };
    let projection = Projection {
        header: header("proj-1"),
        projection_kind: "SubjectIndexStub".to_owned(),
        built_from: vec![OpaqueId::new("assert-1")],
        built_at: instant(),
        body: json!({}),
        authoritative: false,
    };
    let audit = ActionAuditRecord {
        header: header("audit-1"),
        kind: ActionAuditKind::Audit,
        actor: OpaqueId::new("actor-1"),
        action: "test".to_owned(),
        target_refs: vec![],
        effect_state: None,
        payload_digest: None,
        detail: None,
    };
    let identity = IdentityAssertion {
        header: header("ident-1"),
        subject_id: OpaqueId::new("subj-1"),
        identifier_system: "mrn".to_owned(),
        identifier_value: "1".to_owned(),
        confidence: None,
        evidence_refs: vec![],
    };
    let merge = IdentityMergeDecision {
        header: header("merge-1"),
        surviving_subject_id: OpaqueId::new("subj-1"),
        merged_subject_ids: vec![OpaqueId::new("subj-2")],
        authorized_by: OpaqueId::new("actor-1"),
        rationale: "duplicate".to_owned(),
        evidence_refs: vec![],
    };

    for (name, value) in [
        ("source", serde_json::to_value(&source).unwrap()),
        ("derived", serde_json::to_value(&derived).unwrap()),
        ("proposal", serde_json::to_value(&proposal).unwrap()),
        ("assertion", serde_json::to_value(&assertion).unwrap()),
        ("evaluation", serde_json::to_value(&evaluation).unwrap()),
        ("projection", serde_json::to_value(&projection).unwrap()),
        ("audit", serde_json::to_value(&audit).unwrap()),
        ("identity", serde_json::to_value(&identity).unwrap()),
        ("merge", serde_json::to_value(&merge).unwrap()),
    ] {
        let round: serde_json::Value = serde_json::from_str(&value.to_string()).unwrap();
        assert_eq!(value, round, "{name} roundtrip");
    }

    assert_eq!(
        source,
        serde_json::from_value(serde_json::to_value(&source).unwrap()).unwrap()
    );
    assert_eq!(
        proposal,
        serde_json::from_value(serde_json::to_value(&proposal).unwrap()).unwrap()
    );
}

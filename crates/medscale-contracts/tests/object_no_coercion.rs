use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId, SourceRecord,
};

#[test]
fn wrong_class_fields_do_not_decode_as_source() {
    let proposal_shaped = serde_json::json!({
        "header": {
            "id": "prop-1",
            "schema_version": AUTHORITY_SCHEMA_VERSION,
            "realm_id": "realm-a",
            "authority_scope_id": "scope-a"
        },
        "subject_ref": "subj-1",
        "claim_kind": "allergy",
        "payload": {"code": "x"},
        "confidence": 0.9,
        "evidence_refs": [],
        "producer": "rule"
    });
    let decoded: Result<SourceRecord, _> = serde_json::from_value(proposal_shaped);
    assert!(
        decoded.is_err(),
        "proposal JSON must not coerce into SourceRecord"
    );
}

#[test]
fn source_requires_bytes_and_digest() {
    let incomplete = serde_json::json!({
        "header": {
            "id": "src-1",
            "schema_version": 1,
            "realm_id": "realm-a",
            "authority_scope_id": "scope-a"
        },
        "media_type": "text/plain"
    });
    assert!(serde_json::from_value::<SourceRecord>(incomplete).is_err());
}

#[test]
fn valid_source_decodes() {
    let bytes = b"abc".to_vec();
    let source = SourceRecord {
        header: ObjectHeader {
            id: OpaqueId::new("src-1"),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: RealmId::new("realm-a"),
            authority_scope_id: AuthorityScopeId::new("scope-a"),
        },
        content_digest: DigestSha256::of(&bytes),
        bytes,
        media_type: "text/plain".to_owned(),
        acquired_at: None,
        provenance_note: None,
    };
    let value = serde_json::to_value(&source).unwrap();
    let back: SourceRecord = serde_json::from_value(value).unwrap();
    assert_eq!(back, source);
}

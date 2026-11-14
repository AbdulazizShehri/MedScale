//! Spec 011 lexical retrieval tests (updated for Spec 025 corpus).

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::evidence::LexicalRetrieveRequest;
use medscale_contracts::objects::{AuthorityScopeId, ObjectClass, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use std::sync::atomic::{AtomicU64, Ordering};

fn uid() -> u64 {
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-011"),
        RealmId::new("realm-011"),
        AuthorityScopeId::new("scope-011"),
        cap,
        body,
    )
}

fn lease(facade: &CoreFacade) {
    facade
        .dispatch(req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("cli"),
                holder_id_hint: None,
            },
        ))
        .result
        .unwrap();
}

#[test]
fn lexical_retrieve_ranks_and_evidence_only() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let out = facade
        .dispatch(req(
            Capability::RetrieveLexical,
            RequestBody::RetrieveLexical {
                request: LexicalRetrieveRequest {
                    query: "diabetes metformin".into(),
                    corpus_id: "synthetic-lexical".into(),
                    max_hits: 5,
                    include_retracted: false,
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::LexicalRetrieve { result } = out else {
        panic!("expected lexical");
    };
    assert!(result.evidence_only);
    assert!(result.relevance_is_not_authority);
    assert_eq!(result.corpus_version, "1.0.0");
    assert_eq!(result.corpus_source_identity, "synthetic-lexical@1.0.0");
    assert!(!result.hits.is_empty());
    assert!(result.hits[0].relevance_only);
    assert!(result.hits.iter().any(|h| h.doc_id.as_str() == "doc-dm"));
    // Top hit should be diabetes doc
    assert_eq!(result.hits[0].doc_id.as_str(), "doc-dm");

    let eval = facade
        .dispatch(req(
            Capability::ReadObject,
            RequestBody::ReadObject {
                object_id: result.evaluation_id.clone(),
            },
        ))
        .result
        .unwrap();
    let ResponseBody::Object { value } = eval else {
        panic!("expected object");
    };
    assert_eq!(value.get("evidence_only"), Some(&serde_json::json!(true)));
    let _ = ObjectClass::EvaluationRecord;
}

#[test]
fn unknown_corpus_denied() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let err = facade
        .dispatch(req(
            Capability::RetrieveLexical,
            RequestBody::RetrieveLexical {
                request: LexicalRetrieveRequest {
                    query: "x".into(),
                    corpus_id: "nope".into(),
                    max_hits: 1,
                    include_retracted: false,
                },
            },
        ))
        .result
        .unwrap_err();
    assert!(matches!(
        err,
        medscale_contracts::envelopes::AuthorityError::InvalidArgument { .. }
    ));
}

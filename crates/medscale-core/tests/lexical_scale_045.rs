//! Spec 045 procedural lexical scale corpus.

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::evidence::LexicalRetrieveRequest;
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::{
    CoreFacade, SCALE_CORPUS_DOC_COUNT_10K, SCALE_CORPUS_ID, SCALE_CORPUS_VERSION_10K,
    build_doctor_report, build_synthetic_lexical_scale_corpus, scale_synthetic_corpus_10k,
};
use std::sync::atomic::{AtomicU64, Ordering};

fn uid() -> u64 {
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-045"),
        RealmId::new("realm-045"),
        AuthorityScopeId::new("scope-045"),
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
fn scale_10k_corpus_retrievable_and_honest() {
    let c = scale_synthetic_corpus_10k();
    assert_eq!(c.documents.len(), SCALE_CORPUS_DOC_COUNT_10K);
    assert_eq!(c.corpus_id, SCALE_CORPUS_ID);
    assert_eq!(c.version, SCALE_CORPUS_VERSION_10K);

    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let out = facade
        .dispatch(req(
            Capability::RetrieveLexical,
            RequestBody::RetrieveLexical {
                request: LexicalRetrieveRequest {
                    query: "hypertension blood pressure".to_owned(),
                    corpus_id: SCALE_CORPUS_ID.to_owned(),
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
    assert!(!result.hits.is_empty());
    assert_eq!(result.corpus_id, SCALE_CORPUS_ID);
    assert!(result.relevance_is_not_authority);
    assert!(result.evidence_only);

    let doctor = build_doctor_report(None, false, false);
    assert!(doctor.evidence_corpus.scale_corpus_generator_present);
    assert!(!doctor.evidence_corpus.clinical_quality_claimed);
    assert!(!doctor.evidence_corpus.release_ready);
}

#[test]
fn small_scale_builder_is_deterministic() {
    let a = build_synthetic_lexical_scale_corpus(50);
    let b = build_synthetic_lexical_scale_corpus(50);
    assert_eq!(a.content_digest, b.content_digest);
    assert_eq!(a.documents.len(), 50);
}

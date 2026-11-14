//! Spec 025 evidence corpus lifecycle tests.

use std::path::PathBuf;

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::evidence::{EvidenceCorpusRights, LexicalRetrieveRequest};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::{CoreFacade, admit_corpus_dir, build_doctor_report, default_synthetic_corpus};
use std::sync::atomic::{AtomicU64, Ordering};

fn uid() -> u64 {
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/025-evidence-corpus-lifecycle/fixtures")
        .join(name)
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-025"),
        RealmId::new("realm-025"),
        AuthorityScopeId::new("scope-025"),
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
fn load_corpus_fixture_and_builtin() {
    let v1 = admit_corpus_dir(&fixture("synthetic-lexical-v1")).expect("admit v1");
    assert_eq!(v1.corpus_id, "synthetic-lexical");
    assert_eq!(v1.version, "1.0.0");
    assert_eq!(v1.source_identity(), "synthetic-lexical@1.0.0");
    assert!(matches!(v1.rights, EvidenceCorpusRights::SyntheticOwned));
    assert_ne!(v1.source_identity(), v1.content_digest.to_hex());

    let builtin = default_synthetic_corpus();
    assert_eq!(builtin.content_digest, v1.content_digest);
    assert_eq!(builtin.source_identity(), v1.source_identity());
}

#[test]
fn lexical_search_hits_over_versioned_corpus() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let out = facade
        .dispatch(req(
            Capability::RetrieveLexical,
            RequestBody::RetrieveLexical {
                request: LexicalRetrieveRequest {
                    query: "hypertension blood pressure".into(),
                    corpus_id: "synthetic-lexical@1.0.0".into(),
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
    assert!(result.hits.iter().any(|h| h.doc_id.as_str() == "doc-htn"));
    assert!(result.hits.iter().all(|h| h.relevance_only));
    assert!(result.relevance_is_not_authority);
    assert!(result.evidence_only);
}

#[test]
fn retracted_doc_excluded_by_default_marked_when_included() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);

    let excluded = facade
        .dispatch(req(
            Capability::RetrieveLexical,
            RequestBody::RetrieveLexical {
                request: LexicalRetrieveRequest {
                    query: "statin myopathy retracted".into(),
                    corpus_id: "synthetic-lexical".into(),
                    max_hits: 10,
                    include_retracted: false,
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::LexicalRetrieve { result } = excluded else {
        panic!("expected lexical");
    };
    assert!(
        result
            .hits
            .iter()
            .all(|h| h.doc_id.as_str() != "doc-statin-retracted")
    );

    let included = facade
        .dispatch(req(
            Capability::RetrieveLexical,
            RequestBody::RetrieveLexical {
                request: LexicalRetrieveRequest {
                    query: "statin myopathy retracted".into(),
                    corpus_id: "synthetic-lexical".into(),
                    max_hits: 10,
                    include_retracted: true,
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::LexicalRetrieve { result } = included else {
        panic!("expected lexical");
    };
    let hit = result
        .hits
        .iter()
        .find(|h| h.doc_id.as_str() == "doc-statin-retracted")
        .expect("retracted hit when include_retracted");
    assert!(hit.retracted);
    assert!(hit.relevance_only);
}

#[test]
fn version_bump_changes_source_identity_and_digest() {
    let v1 = admit_corpus_dir(&fixture("synthetic-lexical-v1")).unwrap();
    let v11 = admit_corpus_dir(&fixture("synthetic-lexical-v1.1")).unwrap();
    assert_eq!(v1.corpus_id, v11.corpus_id);
    assert_ne!(v1.version, v11.version);
    assert_ne!(v1.source_identity(), v11.source_identity());
    assert_ne!(v1.content_digest, v11.content_digest);
    assert_eq!(v11.source_identity(), "synthetic-lexical@1.1.0");
}

#[test]
fn conflict_and_freshness_markers_are_evidence_metadata() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let out = facade
        .dispatch(req(
            Capability::RetrieveLexical,
            RequestBody::RetrieveLexical {
                request: LexicalRetrieveRequest {
                    query: "conflicting guidance pathway".into(),
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
    let hit = result
        .hits
        .iter()
        .find(|h| h.doc_id.as_str() == "doc-conflict")
        .expect("conflict doc");
    assert_eq!(hit.conflict_marker.as_deref(), Some("duplicate_pathway"));
    assert_eq!(hit.freshness_marker.as_deref(), Some("fixture_current"));
    assert!(hit.relevance_only);
    assert!(result.relevance_is_not_authority);
}

#[test]
fn doctor_evidence_corpus_honesty() {
    let report = build_doctor_report(None, false, false);
    assert!(report.evidence_corpus.ready_base);
    assert!(report.evidence_corpus.versioned_corpus);
    assert!(report.evidence_corpus.synthetic_owned_only);
    assert!(!report.evidence_corpus.clinical_quality_claimed);
    assert!(report.evidence_corpus.scale_corpus_generator_present);
    assert!(!report.evidence_corpus.release_ready);
    assert_eq!(
        report.evidence_corpus.current_corpus_id.as_deref(),
        Some("synthetic-lexical")
    );
}

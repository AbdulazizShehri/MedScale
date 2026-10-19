//! Spec 104: a candle safetensors Pack runs through Core authority
//! (`PacksInstallLocal` -> `PacksEvaluateLocal`), with the prepared model kept
//! in its own residency pool. The tiny BERT checkpoint is written here in the
//! safetensors format (deterministic weights, synthetic only); no network.

mod support;

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::PackEvaluationRequest;
use medscale_core::CoreFacade;
use medscale_pack::CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID;
use support::{candle_pack, dir, uid};

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-104"),
        RealmId::new("realm-104"),
        AuthorityScopeId::new("scope-104"),
        capability,
        body,
    )
}

#[test]
fn candle_pack_evaluates_through_core_with_a_resident_cache() {
    let out = dir("pack");
    let pack_id = candle_pack(&out);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("spec104-client"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease");
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: out.display().to_string(),
            },
        ))
        .result
        .expect("candle Pack admits through Core");
    let evaluate = || {
        facade
            .dispatch(request(
                Capability::PacksEvaluateLocal,
                RequestBody::PacksEvaluateLocal {
                    request: PackEvaluationRequest {
                        pack_id: OpaqueId::new(&pack_id),
                        local_path: out.display().to_string(),
                        input: "alice visited clinic today".to_owned(),
                        max_tokens: 16,
                        synthetic_only: true,
                        zero_shot_labels: None,
                        decoding: None,
                    },
                },
            ))
            .result
            .expect("candle Pack evaluates")
    };
    let ResponseBody::PackEvaluation { result: first } = evaluate() else {
        panic!("evaluation");
    };
    assert_eq!(first.runtime_id, CANDLE_TOKEN_CLASSIFIER_RUNTIME_ID);
    assert!(first.evidence_only);
    assert!(!first.prepared_cache_hit);
    assert_eq!(first.provenance.export_format, "safetensors");
    assert_eq!(
        first.proposal_payload["kind"],
        "candle_token_classification_document_v1"
    );
    assert_eq!(first.proposal_payload["covered_tokens"], 4);
    let ResponseBody::PackEvaluation { result: second } = evaluate() else {
        panic!("evaluation");
    };
    assert!(
        second.prepared_cache_hit,
        "prepared candle model stays resident"
    );

    // An explicit decoding is applied and recorded in the payload.
    let decoded = facade
        .dispatch(request(
            Capability::PacksEvaluateLocal,
            RequestBody::PacksEvaluateLocal {
                request: PackEvaluationRequest {
                    pack_id: OpaqueId::new(&pack_id),
                    local_path: out.display().to_string(),
                    input: "alice visited clinic today".to_owned(),
                    max_tokens: 16,
                    synthetic_only: true,
                    zero_shot_labels: None,
                    decoding: Some(medscale_contracts::packs::EntityDecoding::WordFirstPiiMerge),
                },
            },
        ))
        .result
        .expect("candle Pack evaluates with a decoding");
    let ResponseBody::PackEvaluation { result: decoded } = decoded else {
        panic!("evaluation");
    };
    assert_eq!(decoded.proposal_payload["aggregation"], "word_first");
    assert!(decoded.proposal_payload["pii_merged_entities"].is_array());

    // `recommended` is task-aware: this synthetic Pack is not a PII-family
    // model, so it decodes word-first without the PII merger.
    let recommended = facade
        .dispatch(request(
            Capability::PacksEvaluateLocal,
            RequestBody::PacksEvaluateLocal {
                request: PackEvaluationRequest {
                    pack_id: OpaqueId::new(&pack_id),
                    local_path: out.display().to_string(),
                    input: "alice visited clinic today".to_owned(),
                    max_tokens: 16,
                    synthetic_only: true,
                    zero_shot_labels: None,
                    decoding: Some(medscale_contracts::packs::EntityDecoding::Recommended),
                },
            },
        ))
        .result
        .expect("candle Pack evaluates with the recommended decoding");
    let ResponseBody::PackEvaluation {
        result: recommended,
    } = recommended
    else {
        panic!("evaluation");
    };
    assert_eq!(recommended.proposal_payload["aggregation"], "word_first");
    assert!(recommended.proposal_payload["pii_merged_entities"].is_null());

    let real_phi = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: PackEvaluationRequest {
                pack_id: OpaqueId::new(&pack_id),
                local_path: out.display().to_string(),
                input: "alice".to_owned(),
                max_tokens: 16,
                synthetic_only: false,
                zero_shot_labels: None,
                decoding: None,
            },
        },
    ));
    assert!(real_phi.result.is_err(), "non-synthetic input stays gated");
    let _ = std::fs::remove_dir_all(out);
}

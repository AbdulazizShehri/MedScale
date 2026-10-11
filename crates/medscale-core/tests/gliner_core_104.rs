//! Spec 104: a GLiNER zero-shot Pack runs through Core authority
//! (`PacksInstallLocal` -> `PacksEvaluateLocal` with `zero_shot_labels`),
//! with its prepared model resident. Synthetic random weights; no network.

mod support;

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::PackEvaluationRequest;
use medscale_core::CoreFacade;
use medscale_pack::GLINER_ZERO_SHOT_RUNTIME_ID;
use support::{candle_pack, dir, gliner_pack, uid};

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-104z"),
        RealmId::new("realm-104z"),
        AuthorityScopeId::new("scope-104z"),
        capability,
        body,
    )
}

fn install(facade: &CoreFacade, path: &std::path::Path) {
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: path.display().to_string(),
            },
        ))
        .result
        .expect("Pack admits through Core");
}

fn evaluate(
    facade: &CoreFacade,
    pack_id: &str,
    path: &std::path::Path,
    labels: Option<&[&str]>,
    synthetic_only: bool,
) -> Result<ResponseBody, String> {
    facade
        .dispatch(request(
            Capability::PacksEvaluateLocal,
            RequestBody::PacksEvaluateLocal {
                request: PackEvaluationRequest {
                    pack_id: OpaqueId::new(pack_id),
                    local_path: path.display().to_string(),
                    input: "pain in the left knee today .".to_owned(),
                    max_tokens: 64,
                    synthetic_only,
                    zero_shot_labels: labels.map(|l| l.iter().map(|x| (*x).to_owned()).collect()),
                },
            },
        ))
        .result
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn gliner_pack_evaluates_zero_shot_through_core() {
    let out = dir("gliner-pack");
    let pack_id = gliner_pack(&out);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("spec104z-client"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease");
    install(&facade, &out);

    let labels: &[&str] = &["anatomy", "drug"];
    let Ok(ResponseBody::PackEvaluation { result: first }) =
        evaluate(&facade, &pack_id, &out, Some(labels), true)
    else {
        panic!("zero-shot evaluation");
    };
    assert_eq!(first.runtime_id, GLINER_ZERO_SHOT_RUNTIME_ID);
    assert!(first.evidence_only);
    assert!(!first.prepared_cache_hit);
    assert_eq!(first.proposal_payload["kind"], "gliner_zero_shot_ner_v1");
    assert_eq!(first.proposal_payload["labels"], serde_json::json!(labels));
    for e in first.proposal_payload["entities"].as_array().unwrap() {
        assert!(labels.contains(&e["label"].as_str().unwrap()));
    }
    let Ok(ResponseBody::PackEvaluation { result: second }) =
        evaluate(&facade, &pack_id, &out, Some(labels), true)
    else {
        panic!("zero-shot evaluation");
    };
    assert!(
        second.prepared_cache_hit,
        "prepared GLiNER model stays resident"
    );
    assert_eq!(second.proposal_payload, first.proposal_payload);

    let missing = evaluate(&facade, &pack_id, &out, None, true).unwrap_err();
    assert!(missing.contains("requires zero_shot_labels"), "{missing}");
    let empty = evaluate(&facade, &pack_id, &out, Some(&[]), true).unwrap_err();
    assert!(empty.contains("evaluation failed"), "{empty}");
    assert!(
        evaluate(&facade, &pack_id, &out, Some(labels), false).is_err(),
        "non-synthetic input stays gated"
    );

    // Labels are refused for a token-classification Pack.
    let tc = dir("tc-pack");
    let tc_id = candle_pack(&tc);
    install(&facade, &tc);
    let refused = evaluate(&facade, &tc_id, &tc, Some(labels), true).unwrap_err();
    assert!(refused.contains("only to zero-shot Packs"), "{refused}");
    let _ = std::fs::remove_dir_all(out);
    let _ = std::fs::remove_dir_all(tc);
}

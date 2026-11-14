//! Spec 105: a generation Pack runs through Core authority
//! (`PacksInstallLocal` -> `PacksGenerateLocal`), resident between calls,
//! listed and unloaded with the other pools. Synthetic tiny Qwen2; no network.

mod support;

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::PackGenerationRequest;
use medscale_core::CoreFacade;
use medscale_pack::CANDLE_TEXT_GENERATION_RUNTIME_ID;
use support::{dir, generation_pack, uid};

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-105"),
        RealmId::new("realm-105"),
        AuthorityScopeId::new("scope-105"),
        capability,
        body,
    )
}

fn generate(
    facade: &CoreFacade,
    capability: Capability,
    pack_id: &str,
    path: &std::path::Path,
    max_new_tokens: u32,
    synthetic_only: bool,
) -> Result<ResponseBody, String> {
    facade
        .dispatch(request(
            capability,
            RequestBody::PacksGenerateLocal {
                request: PackGenerationRequest {
                    pack_id: OpaqueId::new(pack_id),
                    local_path: path.display().to_string(),
                    prompt: "patient has fever".to_owned(),
                    max_new_tokens,
                    synthetic_only,
                },
            },
        ))
        .result
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn generation_packs_run_through_core_with_residency_and_gates() {
    let out = dir("generation-pack");
    let pack_id = generation_pack(&out);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("spec105-client"),
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
        .expect("generation Pack admits through Core");

    let cap = Capability::PacksEvaluateLocal;
    let Ok(ResponseBody::PackEvaluation { result: first }) =
        generate(&facade, cap, &pack_id, &out, 6, true)
    else {
        panic!("generation");
    };
    assert_eq!(first.runtime_id, CANDLE_TEXT_GENERATION_RUNTIME_ID);
    assert!(first.evidence_only && !first.prepared_cache_hit);
    assert_eq!(first.proposal_payload["kind"], "candle_text_generation_v1");
    assert!(first.proposal_payload["generated_tokens"].as_u64().unwrap() <= 6);
    let Ok(ResponseBody::PackEvaluation { result: second }) =
        generate(&facade, cap, &pack_id, &out, 6, true)
    else {
        panic!("generation");
    };
    assert!(second.prepared_cache_hit);
    assert_eq!(
        second.proposal_payload, first.proposal_payload,
        "greedy and deterministic"
    );

    // Gates and bounds.
    assert!(
        generate(&facade, cap, &pack_id, &out, 6, false).is_err(),
        "synthetic only"
    );
    assert!(
        generate(&facade, cap, &pack_id, &out, 0, true).is_err(),
        "token budget"
    );
    assert!(
        generate(&facade, Capability::PackRead, &pack_id, &out, 6, true).is_err(),
        "needs the evaluate capability"
    );

    // The generator is listed and unloaded with the other pools.
    let Ok(ResponseBody::PackResidency { view }) = facade
        .dispatch(request(Capability::PackRead, RequestBody::PacksResidency))
        .result
    else {
        panic!("residency");
    };
    assert_eq!(view.models.len(), 1);
    assert_eq!(view.models[0].runtime, "candle_text_generation");
    let Ok(ResponseBody::PackResidency { view }) = facade
        .dispatch(request(
            cap,
            RequestBody::PacksUnloadLocal { pack_id: None },
        ))
        .result
    else {
        panic!("unload");
    };
    assert_eq!(view.unloaded.len(), 1);
    let _ = std::fs::remove_dir_all(out);
}

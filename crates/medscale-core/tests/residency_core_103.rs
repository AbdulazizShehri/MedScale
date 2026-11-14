//! Spec 103: prepared models resident in Core can be listed
//! (`PacksResidency`) and unloaded (`PacksUnloadLocal`), the counterpart of
//! OpenMed `ModelLoader.unload_model`. Synthetic Packs only; no network.

mod support;

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::{PackEvaluationRequest, PackResidencyView};
use medscale_core::CoreFacade;
use support::{candle_pack, dir, gliner_pack, uid};

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-103r"),
        RealmId::new("realm-103r"),
        AuthorityScopeId::new("scope-103r"),
        capability,
        body,
    )
}

fn residency(facade: &CoreFacade, capability: Capability, body: RequestBody) -> PackResidencyView {
    match facade.dispatch(request(capability, body)).result {
        Ok(ResponseBody::PackResidency { view }) => *view,
        other => panic!("{other:?}"),
    }
}

fn evaluate(
    facade: &CoreFacade,
    pack_id: &str,
    path: &std::path::Path,
    labels: Option<Vec<String>>,
) -> bool {
    match facade
        .dispatch(request(
            Capability::PacksEvaluateLocal,
            RequestBody::PacksEvaluateLocal {
                request: PackEvaluationRequest {
                    pack_id: OpaqueId::new(pack_id),
                    local_path: path.display().to_string(),
                    input: "pain in the left knee today .".to_owned(),
                    max_tokens: 64,
                    synthetic_only: true,
                    zero_shot_labels: labels,
                    decoding: None,
                },
            },
        ))
        .result
    {
        Ok(ResponseBody::PackEvaluation { result }) => result.prepared_cache_hit,
        other => panic!("{other:?}"),
    }
}

#[test]
fn resident_models_are_listed_and_unloaded_through_core() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("spec103r-client"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease");
    let tc = dir("res-tc");
    let tc_id = candle_pack(&tc);
    let zs = dir("res-zs");
    let zs_id = gliner_pack(&zs);
    for path in [&tc, &zs] {
        facade
            .dispatch(request(
                Capability::PacksInstallLocal,
                RequestBody::PacksInstallLocal {
                    local_path: path.display().to_string(),
                },
            ))
            .result
            .expect("install");
    }
    assert!(
        residency(&facade, Capability::PackRead, RequestBody::PacksResidency)
            .models
            .is_empty()
    );

    assert!(!evaluate(&facade, &tc_id, &tc, None));
    assert!(!evaluate(
        &facade,
        &zs_id,
        &zs,
        Some(vec!["anatomy".to_owned()])
    ));
    let listed = residency(&facade, Capability::PackRead, RequestBody::PacksResidency);
    let mut runtimes: Vec<&str> = listed.models.iter().map(|m| m.runtime.as_str()).collect();
    runtimes.sort_unstable();
    assert_eq!(runtimes, ["candle_gliner", "candle_token_classifier"]);
    let tc_model = listed
        .models
        .iter()
        .find(|m| m.runtime == "candle_token_classifier")
        .unwrap();
    assert_eq!(tc_model.pack_ids, [OpaqueId::new(&tc_id)]);
    assert!(tc_model.resident_bytes > 0);
    assert!(listed.unloaded.is_empty());

    // Unload one Pack: only its model leaves, and the next run prepares again.
    let after = residency(
        &facade,
        Capability::PacksEvaluateLocal,
        RequestBody::PacksUnloadLocal {
            pack_id: Some(OpaqueId::new(&tc_id)),
        },
    );
    assert_eq!(
        after.unloaded,
        std::slice::from_ref(&tc_model.content_digest)
    );
    assert_eq!(after.models.len(), 1);
    assert_eq!(after.models[0].runtime, "candle_gliner");
    assert!(
        !evaluate(&facade, &tc_id, &tc, None),
        "prepared again after unload"
    );

    // Unload everything unpinned.
    let cleared = residency(
        &facade,
        Capability::PacksEvaluateLocal,
        RequestBody::PacksUnloadLocal { pack_id: None },
    );
    assert_eq!(cleared.unloaded.len(), 2);
    assert!(cleared.models.is_empty());

    // Unknown Packs and the wrong capability are refused.
    assert!(
        facade
            .dispatch(request(
                Capability::PacksEvaluateLocal,
                RequestBody::PacksUnloadLocal {
                    pack_id: Some(OpaqueId::new("no-such-pack")),
                },
            ))
            .result
            .is_err()
    );
    assert!(
        facade
            .dispatch(request(
                Capability::PackRead,
                RequestBody::PacksUnloadLocal { pack_id: None },
            ))
            .result
            .is_err(),
        "unload needs the evaluate capability"
    );
    let _ = std::fs::remove_dir_all(tc);
    let _ = std::fs::remove_dir_all(zs);
}

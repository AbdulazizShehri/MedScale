//! Spec 069 real local ONNX runtime + Core authority boundary regression.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::PackEvaluationRequest;
use medscale_core::CoreFacade;

fn uid() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-069"),
        RealmId::new("realm-069"),
        AuthorityScopeId::new("scope-069"),
        capability,
        body,
    )
}

fn pack_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0",
    )
}

fn pack_copy_with_id(pack_id: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "medscale-069-core-pack-{}-{}",
        std::process::id(),
        uid()
    ));
    std::fs::create_dir_all(&root).expect("create copied pack");
    for name in [
        "model.onnx",
        "tokenizer.json",
        "labels.json",
        "model.meta.json",
        "pack.manifest.json",
    ] {
        std::fs::copy(pack_dir().join(name), root.join(name)).expect("copy pack artifact");
    }
    let manifest_path = root.join("pack.manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).expect("manifest bytes"))
            .expect("manifest json");
    let version = manifest["version"].as_str().expect("version").to_owned();
    let epoch = manifest["pack_epoch"].as_u64().expect("epoch");
    let content_digest = manifest["content_digest"]
        .as_str()
        .expect("content digest")
        .to_owned();
    let rights_uri = manifest["rights_uri"]
        .as_str()
        .expect("rights uri")
        .to_owned();
    let sbom_ref = manifest["sbom_ref"].as_str().expect("sbom ref").to_owned();
    let payload = medscale_keys::pack_signing_payload(
        pack_id,
        &version,
        epoch,
        &content_digest,
        &rights_uri,
        &sbom_ref,
    );
    manifest["pack_id"] = serde_json::Value::String(pack_id.to_owned());
    manifest["signature_hex"] =
        serde_json::Value::String(medscale_keys::sign_pack_payload(&payload));
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n",
    )
    .expect("write copied manifest");
    root
}

fn acquire_lease(facade: &CoreFacade) {
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("spec069-client"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease");
}

fn evaluation_request(synthetic_only: bool) -> PackEvaluationRequest {
    PackEvaluationRequest {
        pack_id: OpaqueId::new("pack-tiny-token-classifier-v0"),
        local_path: pack_dir().display().to_string(),
        input: "alice visited clinic today".to_owned(),
        max_tokens: 4,
        synthetic_only,
        zero_shot_labels: None,
    }
}

#[test]
fn core_runs_real_local_onnx_as_evidence_only() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    acquire_lease(&facade);
    let install = facade.dispatch(request(
        Capability::PacksInstallLocal,
        RequestBody::PacksInstallLocal {
            local_path: pack_dir().display().to_string(),
        },
    ));
    assert!(
        install.result.is_ok(),
        "pack install must admit signed fixture"
    );

    let response = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: evaluation_request(true),
        },
    ));
    let ResponseBody::PackEvaluation { result } = response.result.expect("evaluation") else {
        panic!("expected pack evaluation response");
    };
    assert!(result.evidence_only);
    assert!(!result.prepared_cache_hit);
    assert_eq!(result.runtime_id, "tract_onnx_token_classification_v1");
    assert_eq!(result.provenance.source_kind, "synthetic_fixture");
    assert_eq!(result.provenance.export_format, "onnx");
    assert_eq!(result.proposal_payload["token_count"], 4);
    assert_eq!(
        result.proposal_payload["kind"],
        "onnx_token_classification_v1"
    );

    let second = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: evaluation_request(true),
        },
    ));
    let ResponseBody::PackEvaluation { result: second } = second.result.expect("second evaluation")
    else {
        panic!("expected second pack evaluation response");
    };
    assert!(second.prepared_cache_hit);
    assert!(second.evidence_only);
}

#[test]
fn real_phi_runtime_remains_explicitly_gated() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    acquire_lease(&facade);
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: pack_dir().display().to_string(),
            },
        ))
        .result
        .expect("install");
    let denied = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: evaluation_request(false),
        },
    ));
    assert_eq!(
        denied.result,
        Err(AuthorityError::ExternalGateRequired {
            gate: "REAL_PHI_MODEL_RUNTIME".to_owned(),
        })
    );
}

#[test]
fn strict_mode_requires_client_session_for_model_execution() {
    let facade = CoreFacade::new();
    acquire_lease(&facade);
    let denied = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: evaluation_request(true),
        },
    ));
    assert_eq!(denied.result, Err(AuthorityError::SessionRequired));
}

#[test]
fn prepared_cache_reuse_never_leaks_the_first_pack_identity() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    acquire_lease(&facade);
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: pack_dir().display().to_string(),
            },
        ))
        .result
        .expect("install first pack");
    let first = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: evaluation_request(true),
        },
    ));
    let ResponseBody::PackEvaluation { result: first } = first.result.expect("first evaluation")
    else {
        panic!("expected first evaluation");
    };
    assert!(!first.prepared_cache_hit);

    let alias_id = OpaqueId::new("pack-tiny-token-classifier-v0-alias");
    let alias_dir = pack_copy_with_id(alias_id.as_str());
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: alias_dir.display().to_string(),
            },
        ))
        .result
        .expect("install alias pack");
    let alias = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: PackEvaluationRequest {
                pack_id: alias_id.clone(),
                local_path: alias_dir.display().to_string(),
                input: "alice visited clinic today".to_owned(),
                max_tokens: 4,
                synthetic_only: true,
                zero_shot_labels: None,
            },
        },
    ));
    let ResponseBody::PackEvaluation { result: alias } = alias.result.expect("alias evaluation")
    else {
        panic!("expected alias evaluation");
    };
    assert!(alias.prepared_cache_hit);
    assert_eq!(alias.pack_id, alias_id);
    let _ = std::fs::remove_dir_all(alias_dir);
}

#[test]
fn prepared_cache_reuse_requires_the_same_runtime_contract() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    acquire_lease(&facade);
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: pack_dir().display().to_string(),
            },
        ))
        .result
        .expect("install first pack");
    let first = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: evaluation_request(true),
        },
    ));
    let ResponseBody::PackEvaluation { result: first } = first.result.expect("first evaluation")
    else {
        panic!("expected first evaluation");
    };
    assert!(!first.prepared_cache_hit);

    let alias_id = OpaqueId::new("pack-tiny-token-classifier-v0-contract-alias");
    let alias_dir = pack_copy_with_id(alias_id.as_str());
    let manifest_path = alias_dir.join("pack.manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).expect("manifest bytes"))
            .expect("manifest json");
    manifest["runtime_requirements"] = serde_json::Value::String(
        "tract_onnx_token_classification_v1;synthetic_only=true;fixed_sequence_length=4;contract_variant=alias".to_owned(),
    );
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n",
    )
    .expect("write altered runtime contract");
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: alias_dir.display().to_string(),
            },
        ))
        .result
        .expect("install altered-contract alias");
    let alias = facade.dispatch(request(
        Capability::PacksEvaluateLocal,
        RequestBody::PacksEvaluateLocal {
            request: PackEvaluationRequest {
                pack_id: alias_id,
                local_path: alias_dir.display().to_string(),
                input: "alice visited clinic today".to_owned(),
                max_tokens: 4,
                synthetic_only: true,
                zero_shot_labels: None,
            },
        },
    ));
    let ResponseBody::PackEvaluation { result: alias } = alias.result.expect("alias evaluation")
    else {
        panic!("expected alias evaluation");
    };
    assert!(!alias.prepared_cache_hit);
    let _ = std::fs::remove_dir_all(alias_dir);
}

#[test]
fn desktop_and_cli_still_do_not_own_runtime_dependencies() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    for crate_name in ["medscale-desktop", "medscale-cli"] {
        let manifest = std::fs::read_to_string(root.join(crate_name).join("Cargo.toml"))
            .expect("client manifest");
        assert!(!manifest.contains("tract-onnx"));
        assert!(!manifest.contains("tokenizers"));
        assert!(!manifest.contains("medscale-pack"));
    }
}

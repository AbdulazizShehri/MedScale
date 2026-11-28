//! Core authority + CLI session tests for Wave 1 OpenMed v4.0.0 Model Ecosystem
//! parity: hardware-tier recommendation, MedScale Model Cards, RAM/disk
//! preflight, manifest diffing, and ordered batch evaluation.

mod support;

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::model_catalog::{
    ModelCatalogCardRequest, ModelCatalogManifestDiffRequest, ModelCatalogPreflightRequest,
    ModelCatalogRecommendRequest,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::PackBatchEvaluationRequest;
use medscale_core::{CliSession, CoreFacade};
use support::{candle_pack, dir, uid};

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-v4"),
        RealmId::new("realm-v4"),
        AuthorityScopeId::new("scope-v4"),
        capability,
        body,
    )
}

#[test]
fn core_recommends_renders_cards_checks_preflight_and_diffs_manifests() {
    let mut session = CliSession::connect("v-v4-cat").unwrap();

    let rec = session
        .model_catalog_recommend(ModelCatalogRecommendRequest {
            snapshot_path: None,
            repository: None,
            commit: None,
            device_tier: "phone".to_owned(),
            task: Some("token-classification".to_owned()),
            language: Some("en".to_owned()),
            limit: 10,
        })
        .unwrap();
    assert_eq!(rec.device_tier, "phone");
    assert!(rec.total_matches > 0);
    assert!(!rec.items.is_empty());

    let first_repo_id = rec.items[0]["repo_id"]
        .as_str()
        .expect("recommended item has repo_id")
        .to_owned();
    let card = session
        .model_catalog_card(ModelCatalogCardRequest {
            snapshot_path: None,
            repository: None,
            commit: None,
            repo_id: first_repo_id.clone(),
        })
        .unwrap();
    assert_eq!(card.repo_id, first_repo_id);
    assert!(card.markdown.contains(&card.medscale_id));
    assert!(card.markdown.contains(&first_repo_id));

    let preflight = session
        .model_catalog_preflight(ModelCatalogPreflightRequest {
            snapshot_path: None,
            repository: None,
            commit: None,
            repo_id: first_repo_id.clone(),
            device_tier: "phone".to_owned(),
            available_ram_mb: 16_384.0,
            free_disk_mb: 20_480.0,
        })
        .unwrap();
    assert_eq!(preflight.repo_id, first_repo_id);
    assert!(preflight.fits_ram);
    assert!(preflight.fits_disk);

    let tmp = dir("diff-v4");
    let old_path = tmp.join("old.jsonl");
    let new_path = tmp.join("new.jsonl");
    std::fs::write(
        &old_path,
        concat!(
            r#"{"repo_id":"OpenMed/Synthetic-A","family":"NER","task":"token-classification","formats":["pytorch"],"param_count":1000,"license":"apache-2.0"}"#,
            "\n"
        ),
    )
    .unwrap();
    std::fs::write(
        &new_path,
        concat!(
            r#"{"repo_id":"OpenMed/Synthetic-A","family":"NER","task":"token-classification","formats":["pytorch","onnx"],"param_count":2000,"license":"apache-2.0"}"#,
            "\n",
            r#"{"repo_id":"OpenMed/Synthetic-B","family":"NER","task":"token-classification","formats":["onnx"],"param_count":500,"license":"mit"}"#,
            "\n"
        ),
    )
    .unwrap();
    let diff = session
        .model_catalog_manifest_diff(ModelCatalogManifestDiffRequest {
            old_snapshot_path: old_path.display().to_string(),
            new_snapshot_path: new_path.display().to_string(),
        })
        .unwrap();
    assert_eq!(diff.added, vec!["OpenMed/Synthetic-B"]);
    assert!(diff.removed.is_empty());
    assert_eq!(diff.changed.len(), 1);
}

#[test]
fn packs_evaluate_batch_preserves_order_enforces_synthetic_only_and_uses_residency() {
    let out = dir("batch-pack");
    let pack_id = candle_pack(&out);
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("v4-batch-client"),
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
        .expect("install");

    // Fail closed when synthetic_only = false.
    let err = facade
        .dispatch(request(
            Capability::PacksEvaluateLocal,
            RequestBody::PacksEvaluateBatch {
                request: PackBatchEvaluationRequest {
                    pack_id: OpaqueId::new(&pack_id),
                    local_path: out.display().to_string(),
                    inputs: vec!["alice visited clinic today".to_owned()],
                    max_tokens: 16,
                    decoding: None,
                    zero_shot_labels: None,
                    synthetic_only: false,
                },
            },
        ))
        .result
        .unwrap_err();
    assert!(matches!(err, AuthorityError::ExternalGateRequired { .. }));

    // Cold batch run followed by warm cached batch run.
    let run_batch = || {
        facade
            .dispatch(request(
                Capability::PacksEvaluateLocal,
                RequestBody::PacksEvaluateBatch {
                    request: PackBatchEvaluationRequest {
                        pack_id: OpaqueId::new(&pack_id),
                        local_path: out.display().to_string(),
                        inputs: vec![
                            "alice visited clinic today".to_owned(),
                            "bob called alice".to_owned(),
                        ],
                        max_tokens: 16,
                        decoding: None,
                        zero_shot_labels: None,
                        synthetic_only: true,
                    },
                },
            ))
            .result
            .unwrap()
    };

    let ResponseBody::PackBatchEvaluation { result: cold } = run_batch() else {
        panic!("expected PackBatchEvaluation");
    };
    assert!(!cold.prepared_cache_hit);
    assert!(cold.evidence_only);
    assert_eq!(cold.items.len(), 2);
    assert_eq!(cold.items[0]["index"], 0);
    assert_eq!(cold.items[1]["index"], 1);

    let ResponseBody::PackBatchEvaluation { result: warm } = run_batch() else {
        panic!("expected PackBatchEvaluation");
    };
    assert!(warm.prepared_cache_hit);
    assert_eq!(warm.items.len(), 2);
}

//! Spec 103 end-to-end model lifecycle through Core, hermetic:
//! DISCOVER (catalog query) -> INSPECT (runtime expectation, variants) ->
//! ACQUIRE (governed transport, scripted; interrupted then resumed) ->
//! VERIFY + ADMIT (catalog hash, per-file digests, signed Pack) -> LOAD + RUN
//! (Core evaluation, resident cache) -> COMPARE (Core result equals the direct
//! runtime) -> UNLOAD (residency list and unload), plus the failure paths:
//! consent bound, tampered metadata, privacy gate, weights tampered after
//! admission and an unsupported architecture.
//!
//! The model is a synthetic tiny BERT (deterministic weights); this is a
//! lifecycle test, not real-model qualification.

mod support;

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::model_catalog::ModelCatalogQueryRequest;
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::packs::PackEvaluationRequest;
use medscale_core::CoreFacade;
use medscale_core::model_acquisition::{AcquisitionConsent, AcquisitionError, acquire_snapshot};
use medscale_network::ScriptedBrowseTransport;
use medscale_pack::{
    HfRepoMetadata, ModelCatalog, TokenClassifierSnapshot, build_token_classifier_pack,
    reproducibility_hash,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use support::{dir, git_blob, hex, tiny_bert, uid};

const COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";
const SHA: &str = "3c914174cf099fda326b43084f679ad31425f88c";
const REPO: &str = "OpenMed/Synthetic-E2E-NER";

struct Snapshot {
    config: Vec<u8>,
    tokenizer: Vec<u8>,
    weights: Vec<u8>,
    meta: String,
    catalog_line: String,
}

fn snapshot(model_type: &str) -> Snapshot {
    let config = serde_json::to_vec(&json!({
        "model_type": model_type, "vocab_size": 8, "hidden_size": 8, "num_hidden_layers": 1,
        "num_attention_heads": 2, "intermediate_size": 16, "hidden_act": "gelu",
        "hidden_dropout_prob": 0.0, "max_position_embeddings": 16, "type_vocab_size": 2,
        "initializer_range": 0.02, "layer_norm_eps": 1e-12, "pad_token_id": 0,
        "classifier_dropout": null, "id2label": {"0": "O", "1": "B-X"}
    }))
    .unwrap();
    let tokenizer = std::fs::read(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0/tokenizer.json",
    ))
    .unwrap();
    let weights = tiny_bert();
    let meta = json!({
        "id": REPO, "sha": SHA, "lastModified": "2026-07-13T07:43:16.000Z",
        "gated": false, "private": false,
        "siblings": [
            {"rfilename": "config.json", "size": config.len(), "blobId": git_blob(&config)},
            {"rfilename": "model.safetensors", "size": weights.len(),
             "lfs": {"sha256": hex(&Sha256::digest(&weights)), "size": weights.len()}},
            {"rfilename": "tokenizer.json", "size": tokenizer.len(), "blobId": git_blob(&tokenizer)}
        ]
    })
    .to_string();
    let parsed: HfRepoMetadata = serde_json::from_str(&meta).unwrap();
    let catalog_line = format!(
        r#"{{"repo_id":"{REPO}","family":"NER","task":"token-classification","architecture":"bert","formats":["pytorch"],"license":"apache-2.0","param_count":1000,"reproducibility_hash":"{}"}}"#,
        reproducibility_hash(&parsed)
    );
    Snapshot {
        config,
        tokenizer,
        weights,
        meta,
        catalog_line,
    }
}

fn transport(s: &Snapshot, meta: &str) -> ScriptedBrowseTransport {
    let base = format!("https://huggingface.co/{REPO}/resolve/{SHA}");
    ScriptedBrowseTransport::new()
        .route(
            &format!("https://huggingface.co/api/models/{REPO}?blobs=true"),
            ScriptedBrowseTransport::ok("application/json", meta.as_bytes()),
        )
        .route(
            &format!("{base}/config.json"),
            ScriptedBrowseTransport::ok("application/json", &s.config),
        )
        .route(
            &format!("{base}/tokenizer.json"),
            ScriptedBrowseTransport::ok("application/json", &s.tokenizer),
        )
        .route(
            &format!("{base}/model.safetensors"),
            ScriptedBrowseTransport::ok("application/octet-stream", &s.weights),
        )
}

fn consent(bytes: u64) -> AcquisitionConsent {
    AcquisitionConsent {
        repo_id: REPO.into(),
        onnx_file: "model.safetensors".into(),
        approved_bytes: bytes,
    }
}

fn request(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-e2e"),
        RealmId::new("realm-e2e"),
        AuthorityScopeId::new("scope-e2e"),
        capability,
        body,
    )
}

fn evaluate(
    facade: &CoreFacade,
    pack_id: &str,
    path: &std::path::Path,
    synthetic_only: bool,
) -> Result<ResponseBody, String> {
    facade
        .dispatch(request(
            Capability::PacksEvaluateLocal,
            RequestBody::PacksEvaluateLocal {
                request: PackEvaluationRequest {
                    pack_id: OpaqueId::new(pack_id),
                    local_path: path.display().to_string(),
                    input: "alice visited clinic today".to_owned(),
                    max_tokens: 16,
                    synthetic_only,
                    zero_shot_labels: None,
                },
            },
        ))
        .result
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn model_lifecycle_runs_end_to_end_through_core() {
    let s = snapshot("bert");
    let catalog_path = dir("e2e-catalog").join("models.jsonl");
    std::fs::write(&catalog_path, &s.catalog_line).unwrap();

    // DISCOVER: the row is listed by the read-only Core catalog query.
    let page = medscale_core::model_catalog::catalog_page(&ModelCatalogQueryRequest {
        snapshot_path: catalog_path.display().to_string(),
        repository: "maziyarpanahi/openmed".into(),
        commit: COMMIT.into(),
        text: Some("E2E".into()),
        limit: 10,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(page.total_matching, 1);

    // INSPECT: an expected-runnable safetensors row with one variant.
    let variants = medscale_core::model_catalog::catalog_variants(
        &catalog_path,
        "maziyarpanahi/openmed",
        COMMIT,
        REPO,
        Some("other"),
    )
    .unwrap();
    assert_eq!(variants.len(), 1);
    assert_eq!(
        variants[0].runtime,
        medscale_pack::VariantRuntime::CandleSafetensors
    );

    // ACQUIRE: an interrupted download keeps verified staging and resumes.
    let catalog =
        ModelCatalog::import("maziyarpanahi/openmed", COMMIT, s.catalog_line.as_bytes()).unwrap();
    let weights_url = format!("https://huggingface.co/{REPO}/resolve/{SHA}/model.safetensors");
    let snap_dir = dir("e2e-snapshot").join("snapshot");
    let interrupted = transport(&s, &s.meta).interrupt_after(&weights_url, 512);
    let err = acquire_snapshot(&interrupted, &catalog, &consent(1_000_000), &snap_dir).unwrap_err();
    assert!(matches!(err, AcquisitionError::Transport(_)), "{err}");
    let report = acquire_snapshot(
        &transport(&s, &s.meta),
        &catalog,
        &consent(1_000_000),
        &snap_dir,
    )
    .unwrap();
    assert_eq!(report.revision, SHA);
    assert!(report.resumed_bytes > 0, "resumed from verified staging");

    // VERIFY + ADMIT.
    let meta: HfRepoMetadata =
        serde_json::from_slice(&std::fs::read(snap_dir.join("metadata.json")).unwrap()).unwrap();
    let pack_dir = dir("e2e-pack");
    let pack_id = build_token_classifier_pack(
        &TokenClassifierSnapshot {
            meta: &meta,
            row: catalog.get(REPO).unwrap(),
            snapshot_dir: &snap_dir,
            onnx_file: "model.safetensors",
            fixed_sequence_length: 16,
        },
        &pack_dir,
    )
    .unwrap();
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    facade
        .dispatch(request(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("e2e-client"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease");
    facade
        .dispatch(request(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: pack_dir.display().to_string(),
            },
        ))
        .result
        .expect("admitted");

    // LOAD + RUN: first evaluation prepares, the second is resident.
    let Ok(ResponseBody::PackEvaluation { result: first }) =
        evaluate(&facade, &pack_id, &pack_dir, true)
    else {
        panic!("evaluation");
    };
    assert!(first.evidence_only && !first.prepared_cache_hit);
    let Ok(ResponseBody::PackEvaluation { result: second }) =
        evaluate(&facade, &pack_id, &pack_dir, true)
    else {
        panic!("evaluation");
    };
    assert!(second.prepared_cache_hit);

    // COMPARE: Core's result equals the direct runtime on the same Pack.
    let manifest = medscale_pack::admit_pack_dir(&pack_dir).unwrap();
    let direct = medscale_pack::evaluate_admitted_pack(
        &pack_dir,
        &manifest,
        "alice visited clinic today",
        16,
    )
    .unwrap();
    assert_eq!(
        direct.output.proposal_payload["entities"],
        first.proposal_payload["entities"]
    );

    // Privacy gate: non-synthetic input is refused.
    assert!(evaluate(&facade, &pack_id, &pack_dir, false).is_err());

    // UNLOAD.
    let Ok(ResponseBody::PackResidency { view }) = facade
        .dispatch(request(
            Capability::PacksEvaluateLocal,
            RequestBody::PacksUnloadLocal {
                pack_id: Some(OpaqueId::new(&pack_id)),
            },
        ))
        .result
    else {
        panic!("unload");
    };
    assert_eq!(view.unloaded.len(), 1);
    assert!(view.models.is_empty());

    // Weights tampered after admission: evaluation fails closed.
    std::fs::write(pack_dir.join("model.safetensors"), b"tampered").unwrap();
    assert!(evaluate(&facade, &pack_id, &pack_dir, true).is_err());
}

#[test]
fn acquisition_refuses_over_consent_and_tampered_metadata() {
    let s = snapshot("bert");
    let catalog =
        ModelCatalog::import("maziyarpanahi/openmed", COMMIT, s.catalog_line.as_bytes()).unwrap();
    let d = dir("e2e-refuse").join("snapshot");
    // Consent bound below the snapshot size: nothing is downloaded.
    let err = acquire_snapshot(&transport(&s, &s.meta), &catalog, &consent(100), &d).unwrap_err();
    assert!(
        matches!(err, AcquisitionError::ExceedsConsent { .. }),
        "{err}"
    );
    // Metadata that no longer matches the catalog reproducibility hash: the
    // catalog-pinned revision is searched in the commit history, the only
    // commit served is the tampered one, and acquisition is refused.
    let tampered = s.meta.replace("2026-07-13", "2026-07-14");
    let base = format!("https://huggingface.co/api/models/{REPO}");
    let history = transport(&s, &tampered)
        .route(
            &format!("{base}/commits/main"),
            ScriptedBrowseTransport::ok(
                "application/json",
                format!(r#"[{{"id":"{SHA}"}}]"#).as_bytes(),
            ),
        )
        .route(
            &format!("{base}/revision/{SHA}?blobs=true"),
            ScriptedBrowseTransport::ok("application/json", tampered.as_bytes()),
        );
    let err = acquire_snapshot(&history, &catalog, &consent(1_000_000), &d).unwrap_err();
    assert!(matches!(err, AcquisitionError::Snapshot(_)), "{err}");
    assert!(!d.exists());
}

#[test]
fn an_unsupported_architecture_is_refused_before_execution() {
    let s = snapshot("gpt2");
    let catalog =
        ModelCatalog::import("maziyarpanahi/openmed", COMMIT, s.catalog_line.as_bytes()).unwrap();
    let snap_dir = dir("e2e-gpt2").join("snapshot");
    acquire_snapshot(
        &transport(&s, &s.meta),
        &catalog,
        &consent(1_000_000),
        &snap_dir,
    )
    .unwrap();
    let meta: HfRepoMetadata =
        serde_json::from_slice(&std::fs::read(snap_dir.join("metadata.json")).unwrap()).unwrap();
    let pack_dir = dir("e2e-gpt2-pack");
    build_token_classifier_pack(
        &TokenClassifierSnapshot {
            meta: &meta,
            row: catalog.get(REPO).unwrap(),
            snapshot_dir: &snap_dir,
            onnx_file: "model.safetensors",
            fixed_sequence_length: 16,
        },
        &pack_dir,
    )
    .unwrap();
    let manifest = medscale_pack::admit_pack_dir(&pack_dir).unwrap();
    let err = medscale_pack::prepare_candle_token_classifier(&pack_dir, &manifest).unwrap_err();
    assert!(
        matches!(
            err,
            medscale_pack::CandleRuntimeError::UnsupportedArchitecture(_)
        ),
        "{err}"
    );
}

//! Spec 104 external qualification: catalog row -> verified Hub snapshot with
//! `model.safetensors` -> signed candle Pack -> admission -> local execution.
//! Weights are not vendored; the caller supplies them (pinned qualification
//! acquisition, see `.github/workflows/model-qualification.yml`).
//!
//! MEDSCALE_CATALOG_SNAPSHOT, MEDSCALE_CATALOG_COMMIT, MEDSCALE_HF_SNAPSHOT_DIR,
//! MEDSCALE_SAMPLE_TEXT as for `openmed_snapshot_external_103`; the weight file
//! is `model.safetensors`.
//! cargo test --release -p medscale-pack --test safetensors_external_104 -- --ignored --nocapture

use std::path::PathBuf;
use std::time::Instant;

use medscale_pack::{
    CandleRunOptions, HfRepoMetadata, ModelCatalog, TokenClassifierSnapshot, admit_pack_dir,
    build_token_classifier_pack, prepare_candle_token_classifier,
};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

#[test]
#[ignore = "requires a pinned, locally supplied Hugging Face snapshot; weights are not vendored"]
fn safetensors_catalog_row_to_local_execution() {
    let catalog = ModelCatalog::import(
        "maziyarpanahi/openmed",
        &env("MEDSCALE_CATALOG_COMMIT"),
        &std::fs::read(env("MEDSCALE_CATALOG_SNAPSHOT")).unwrap(),
    )
    .unwrap();
    let snapshot = PathBuf::from(env("MEDSCALE_HF_SNAPSHOT_DIR"));
    let meta: HfRepoMetadata =
        serde_json::from_slice(&std::fs::read(snapshot.join("metadata.json")).unwrap()).unwrap();
    let row = catalog
        .get(&meta.id)
        .expect("repository must be in the catalog");
    // `model.safetensors` (Transformers layout) or `weights.safetensors`
    // (OpenMed MLX export, renamed keys).
    let weight_file =
        std::env::var("MEDSCALE_HF_ONNX_FILE").unwrap_or_else(|_| "model.safetensors".into());
    let out = std::env::temp_dir().join(format!("medscale-104-pack-{}", std::process::id()));
    let pack_id = build_token_classifier_pack(
        &TokenClassifierSnapshot {
            meta: &meta,
            row,
            snapshot_dir: &snapshot,
            onnx_file: &weight_file,
            fixed_sequence_length: 512,
        },
        &out,
    )
    .expect("verified snapshot must build a Pack");
    let manifest = admit_pack_dir(&out).expect("signed Pack must admit");
    assert_eq!(manifest.pack_id.as_str(), pack_id);
    let t0 = Instant::now();
    let prepared = prepare_candle_token_classifier(&out, &manifest).expect("model must prepare");
    let prepare_ms = t0.elapsed().as_millis();
    let text = env("MEDSCALE_SAMPLE_TEXT");
    let _ = prepared
        .run_document(&manifest.pack_id, &text, CandleRunOptions::default())
        .unwrap();
    let t1 = Instant::now();
    let output = prepared
        .run_document(&manifest.pack_id, &text, CandleRunOptions::default())
        .expect("model must run");
    let run_ms = t1.elapsed().as_millis();
    assert!(output.evidence_only);
    assert_eq!(prepared.provenance().revision, meta.sha);
    println!(
        "SNAPSHOT_EVIDENCE runtime=candle device={} device_fallback={:?} repo={} revision={} model_type={} window={} prepare_ms={} warm_run_ms={}",
        prepared.device_name(),
        prepared.device_fallback(),
        meta.id,
        meta.sha,
        prepared.model_type(),
        prepared.window(),
        prepare_ms,
        run_ms
    );
    println!(
        "SNAPSHOT_DOCUMENT {}",
        serde_json::to_string(&output.proposal_payload["entities"]).unwrap()
    );
    let _ = std::fs::remove_dir_all(&out);
}

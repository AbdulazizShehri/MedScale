//! Spec 104 external qualification of GLiNER zero-shot NER: catalog row ->
//! verified Hub snapshot of an OpenMed `ZeroShot-NER-*-mlx` export (safetensors)
//! -> signed Pack -> admission -> local zero-shot run. Weights are not
//! vendored; the caller supplies the pinned snapshot.
//!
//! MEDSCALE_CATALOG_SNAPSHOT, MEDSCALE_CATALOG_COMMIT, MEDSCALE_HF_SNAPSHOT_DIR,
//! MEDSCALE_HF_ONNX_FILE (weights.safetensors), MEDSCALE_SAMPLE_TEXT,
//! MEDSCALE_ZS_LABELS (comma-separated, default "anatomy").
//! cargo test --release -p medscale-pack --test gliner_external_104 -- --ignored --nocapture

use std::path::PathBuf;
use std::time::Instant;

use medscale_pack::{
    HfRepoMetadata, ModelCatalog, TokenClassifierSnapshot, admit_pack_dir,
    build_token_classifier_pack, prepare_gliner_span_model,
};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

#[test]
#[ignore = "requires a pinned, locally supplied Hugging Face snapshot; weights are not vendored"]
fn gliner_catalog_row_to_local_zero_shot_run() {
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
    let weight_file =
        std::env::var("MEDSCALE_HF_ONNX_FILE").unwrap_or_else(|_| "weights.safetensors".into());
    let out = std::env::temp_dir().join(format!("medscale-104-gliner-{}", std::process::id()));
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
    let model = prepare_gliner_span_model(&out, &manifest).expect("GLiNER must prepare");
    let prepare_ms = t0.elapsed().as_millis();
    let text = env("MEDSCALE_SAMPLE_TEXT");
    let labels_raw = std::env::var("MEDSCALE_ZS_LABELS").unwrap_or_else(|_| "anatomy".into());
    let labels: Vec<&str> = labels_raw.split(',').map(str::trim).collect();
    let _ = model.predict(&text, &labels, 0.5, true).unwrap();
    let t1 = Instant::now();
    let output = model
        .run(&manifest.pack_id, &text, &labels, 0.5)
        .expect("GLiNER must run");
    let run_ms = t1.elapsed().as_millis();
    assert!(output.evidence_only);
    assert_eq!(model.provenance().revision, meta.sha);
    println!(
        "SNAPSHOT_EVIDENCE runtime=candle_gliner repo={} revision={} labels={labels:?} prepare_ms={prepare_ms} warm_run_ms={run_ms}",
        meta.id, meta.sha
    );
    println!(
        "SNAPSHOT_DOCUMENT {}",
        serde_json::to_string(&output.proposal_payload["entities"]).unwrap()
    );
    let _ = std::fs::remove_dir_all(&out);
}

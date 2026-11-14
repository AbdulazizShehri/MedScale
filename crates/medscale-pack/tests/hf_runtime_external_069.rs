use std::path::PathBuf;
use std::time::Instant;

use medscale_pack::{OnnxTokenClassifierRuntime, admit_pack_dir};

fn external_pack() -> PathBuf {
    PathBuf::from(
        std::env::var("MEDSCALE_HF_PACK_DIR")
            .expect("MEDSCALE_HF_PACK_DIR must point at the pinned external HF pack"),
    )
}

#[test]
#[ignore = "requires pinned external Hugging Face model bytes; weights are not vendored"]
fn pinned_hugging_face_onnx_model_executes_locally() {
    let root = external_pack();
    let manifest = admit_pack_dir(&root).expect("external signed pack must admit");
    let runtime = OnnxTokenClassifierRuntime::new(128).expect("runtime");
    let prepare_started = Instant::now();
    let prepared = runtime
        .prepare(&root, &manifest)
        .expect("HF-hosted ONNX model must prepare locally");
    let prepare_ms = prepare_started.elapsed().as_secs_f64() * 1000.0;

    let evaluation = prepared
        .run(&manifest.pack_id, "John Smith lives in London.")
        .expect("HF-hosted ONNX model must execute locally");
    assert!(evaluation.output.evidence_only);
    assert_eq!(evaluation.provenance.source_kind, "hugging_face");
    assert_eq!(
        evaluation.provenance.repository,
        "onnx-community/bert-base-NER-ONNX"
    );
    assert_eq!(
        evaluation.provenance.revision,
        "9faa2f4a2d59b396888b318f596ff719cc893f1e"
    );
    assert_eq!(evaluation.provenance.license_id, "MIT");
    assert_eq!(evaluation.provenance.fixed_sequence_length, 128);
    assert_eq!(
        evaluation.output.proposal_payload["kind"],
        "onnx_token_classification_v1"
    );
    let predictions = evaluation.output.proposal_payload["predictions"]
        .as_array()
        .expect("predictions");
    assert!(
        predictions
            .iter()
            .any(|p| p["token"] == "John" && p["label"] == "B-PER")
    );
    assert!(
        predictions
            .iter()
            .any(|p| p["token"] == "London" && p["label"] == "B-LOC")
    );

    let mut warm_ms = Vec::with_capacity(30);
    for _ in 0..30 {
        let started = Instant::now();
        let warm = prepared
            .run(&manifest.pack_id, "John Smith lives in London.")
            .expect("warm inference");
        assert!(warm.output.evidence_only);
        warm_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    warm_ms.sort_by(f64::total_cmp);
    let p50 = warm_ms[14];
    let p95 = warm_ms[28];
    eprintln!("HF_069 prepare_ms={prepare_ms:.3} warm_p50_ms={p50:.3} warm_p95_ms={p95:.3}");
}

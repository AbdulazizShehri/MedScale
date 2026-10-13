use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use medscale_pack::{
    OnnxRuntimeError, OnnxTokenClassifierRuntime, PlanCacheOutcome, admit_pack_dir,
};

fn fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0",
    )
}

fn temp_copy() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let root = std::env::temp_dir().join(format!(
        "medscale-069-runtime-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("create temp pack");
    for name in [
        "model.onnx",
        "tokenizer.json",
        "labels.json",
        "model.meta.json",
        "pack.manifest.json",
    ] {
        fs::copy(fixture_pack().join(name), root.join(name)).expect("copy fixture");
    }
    root
}

#[test]
fn real_onnx_fixture_runs_evidence_only() {
    let root = fixture_pack();
    let manifest = admit_pack_dir(&root).expect("signed pack admits");
    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    let output = runtime
        .run(&root, &manifest, "alice visited clinic today")
        .expect("real ONNX inference");

    assert!(output.output.evidence_only);
    assert_eq!(output.output.pack_id, manifest.pack_id);
    assert_eq!(output.provenance.source_kind, "synthetic_fixture");
    assert_eq!(
        output.output.proposal_payload["kind"],
        "onnx_token_classification_v1"
    );
    assert_eq!(output.output.proposal_payload["token_count"], 4);
    let predictions = output.output.proposal_payload["predictions"]
        .as_array()
        .unwrap();
    assert!(predictions.iter().all(|item| item["label"] == "ENTITY"));
}

#[test]
fn plan_cache_round_trip_matches_the_uncached_plan() {
    let root = fixture_pack();
    let manifest = admit_pack_dir(&root).expect("signed pack admits");
    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    let text = "alice visited clinic today";
    let payload = |prepared: &medscale_pack::PreparedOnnxTokenClassifier| {
        prepared
            .run(&manifest.pack_id, text)
            .expect("inference")
            .output
            .proposal_payload
    };
    let expected = payload(&runtime.prepare(&root, &manifest).expect("prepare"));

    let cache = temp_copy().join("plan-cache");
    let (cold, outcome) = runtime
        .prepare_with_plan_cache(&root, &manifest, &cache)
        .expect("cold prepare");
    assert_eq!(outcome, PlanCacheOutcome::Written);
    assert_eq!(payload(&cold), expected);
    let (warm, outcome) = runtime
        .prepare_with_plan_cache(&root, &manifest, &cache)
        .expect("warm prepare");
    assert_eq!(outcome, PlanCacheOutcome::Hit);
    assert_eq!(payload(&warm), expected);
    // A hit skips loading, typing and decluttering.
    let stages = warm.stage_timings();
    assert_eq!((stages.load_ms, stages.typed_and_decluttered_ms), (0, 0));

    // A corrupted plan fails its digest: the cache is rebuilt, never trusted.
    let plan = fs::read_dir(&cache)
        .expect("cache dir")
        .map(|entry| entry.expect("entry").path())
        .find(|path| path.to_string_lossy().ends_with(".nnef.tar"))
        .expect("cached plan");
    let mut bytes = fs::read(&plan).expect("read plan");
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0x55;
    fs::write(&plan, bytes).expect("corrupt plan");
    let (rebuilt, outcome) = runtime
        .prepare_with_plan_cache(&root, &manifest, &cache)
        .expect("prepare after corruption");
    assert_eq!(outcome, PlanCacheOutcome::Written);
    assert_eq!(payload(&rebuilt), expected);
    let (_, outcome) = runtime
        .prepare_with_plan_cache(&root, &manifest, &cache)
        .expect("prepare after rebuild");
    assert_eq!(outcome, PlanCacheOutcome::Hit);
}

#[test]
fn plan_cache_still_verifies_pack_artifacts() {
    let root = temp_copy();
    let manifest = admit_pack_dir(&root).expect("initial admission");
    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    let cache = root.join("plan-cache");
    let (_, outcome) = runtime
        .prepare_with_plan_cache(&root, &manifest, &cache)
        .expect("cold prepare");
    assert_eq!(outcome, PlanCacheOutcome::Written);
    fs::write(root.join("labels.json"), b"[\"O\"]\n").expect("tamper");
    assert!(matches!(
        runtime.prepare_with_plan_cache(&root, &manifest, &cache),
        Err(OnnxRuntimeError::ArtifactDigestMismatch)
    ));
}

#[test]
fn post_admission_artifact_swap_fails_closed() {
    let root = temp_copy();
    let manifest = admit_pack_dir(&root).expect("initial admission");
    fs::write(root.join("labels.json"), b"[\"O\"]\n").expect("tamper");
    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    assert!(matches!(
        runtime.run(&root, &manifest, "alice visited clinic today"),
        Err(OnnxRuntimeError::ArtifactDigestMismatch)
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn runtime_never_accepts_unsafe_artifact_paths() {
    let root = fixture_pack();
    let mut manifest = admit_pack_dir(&root).expect("admission");
    manifest.artifacts[0].relative_path = "../outside".to_owned();
    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    assert!(matches!(
        runtime.run(Path::new(&root), &manifest, "alice visited clinic today"),
        Err(OnnxRuntimeError::UnsafeArtifactPath)
    ));
}

#[cfg(unix)]
#[test]
fn runtime_rejects_post_admission_symlink_escape_even_when_bytes_match() {
    use std::os::unix::fs::symlink;

    let root = temp_copy();
    let manifest = admit_pack_dir(&root).expect("initial admission");
    let outside = std::env::temp_dir().join(format!(
        "medscale-069-runtime-outside-{}-{}",
        std::process::id(),
        uid_for_test()
    ));
    fs::copy(root.join("model.onnx"), &outside).expect("copy outside model");
    fs::remove_file(root.join("model.onnx")).expect("remove in-pack model");
    symlink(&outside, root.join("model.onnx")).expect("create escaping symlink");
    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    assert!(matches!(
        runtime.run(&root, &manifest, "alice visited clinic today"),
        Err(OnnxRuntimeError::UnsafeArtifactPath)
    ));
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_file(outside);
}

#[test]
fn admission_and_runtime_reject_oversized_model_before_loading() {
    let root = temp_copy();
    let manifest = admit_pack_dir(&root).expect("initial admission");
    let model_path = root.join("model.onnx");
    fs::OpenOptions::new()
        .write(true)
        .open(&model_path)
        .expect("open model")
        .set_len(1_073_741_825)
        .expect("create sparse oversized model");

    let admission_error = admit_pack_dir(&root).expect_err("oversized model must be refused");
    assert!(admission_error.to_string().contains("byte bound"));

    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    assert!(matches!(
        runtime.run(&root, &manifest, "alice visited clinic today"),
        Err(OnnxRuntimeError::ArtifactBound)
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn pack_admission_rejects_oversized_manifest_before_json_parse() {
    let root = temp_copy();
    fs::OpenOptions::new()
        .write(true)
        .open(root.join("pack.manifest.json"))
        .expect("open manifest")
        .set_len(1_048_577)
        .expect("create sparse oversized manifest");
    let err = admit_pack_dir(&root).expect_err("oversized manifest must be refused");
    assert!(
        err.to_string()
            .contains("manifest exceeds admitted byte bound")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn prepare_rejects_static_output_shape_before_execution() {
    let root = temp_copy();
    let mut manifest = admit_pack_dir(&root).expect("initial admission");
    let labels = b"[\n  \"O\"\n]\n";
    fs::write(root.join("labels.json"), labels).expect("write one-label metadata");
    let labels_artifact = manifest
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.relative_path == "labels.json")
        .expect("labels artifact");
    labels_artifact.digest = medscale_contracts::objects::DigestSha256::of(labels);

    let runtime = OnnxTokenClassifierRuntime::new(4).expect("runtime");
    assert!(matches!(
        runtime.prepare(&root, &manifest),
        Err(OnnxRuntimeError::OutputShape)
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn pack_admission_rejects_parent_traversal_before_reading_artifact() {
    let root = temp_copy();
    let manifest_path = root.join("pack.manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest bytes"))
            .expect("manifest json");
    manifest["artifacts"][0]["relative_path"] = serde_json::Value::String("../outside".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n",
    )
    .expect("write traversal manifest");
    let err = admit_pack_dir(&root).expect_err("parent traversal must be refused");
    assert!(err.to_string().contains("escapes pack root"));
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn pack_admission_rejects_symlink_escape_even_when_bytes_match() {
    use std::os::unix::fs::symlink;

    let root = temp_copy();
    let outside = std::env::temp_dir().join(format!(
        "medscale-069-outside-{}-{}",
        std::process::id(),
        uid_for_test()
    ));
    fs::copy(root.join("model.onnx"), &outside).expect("copy outside model");
    fs::remove_file(root.join("model.onnx")).expect("remove in-pack model");
    symlink(&outside, root.join("model.onnx")).expect("create escaping symlink");
    let err = admit_pack_dir(&root).expect_err("symlink escape must be refused");
    assert!(err.to_string().contains("escapes pack root"));
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_file(outside);
}

#[cfg(unix)]
fn uid_for_test() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(10_000);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

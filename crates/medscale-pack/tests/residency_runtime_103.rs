//! Spec 103 LOAD/RUN/UNLOAD: prepared ONNX runtimes held in a budgeted
//! residency pool, using the signed synthetic fixture Pack from Spec 069.

use std::path::PathBuf;

use medscale_pack::{
    OnnxTokenClassifierRuntime, PreparedOnnxTokenClassifier, ResidencyError, ResidencyPool,
    admit_pack_dir,
};

fn fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0")
}

/// Estimated resident size: the ONNX weights (the dominant term for these
/// models). Labelled an estimate; not a measurement of process memory.
fn estimated_bytes(dir: &std::path::Path) -> u64 {
    std::fs::metadata(dir.join("model.onnx")).unwrap().len()
}

#[test]
fn load_run_evict_and_unload_prepared_models_under_a_budget() {
    let dir = fixture_pack();
    let manifest = admit_pack_dir(&dir).expect("signed fixture admits");
    let runtime = OnnxTokenClassifierRuntime::new(4).unwrap();
    let bytes = estimated_bytes(&dir);
    // Budget fits exactly one prepared model.
    let mut pool: ResidencyPool<PreparedOnnxTokenClassifier> = ResidencyPool::new(bytes);

    let first = runtime.prepare(&dir, &manifest).unwrap();
    assert!(pool.load("lane-a", bytes, first).unwrap().is_empty());
    let out = pool
        .get("lane-a")
        .unwrap()
        .run(&manifest.pack_id, "alice visited clinic today")
        .unwrap();
    assert!(out.output.evidence_only);

    // Loading a second model evicts the least recently used one.
    let second = runtime.prepare(&dir, &manifest).unwrap();
    assert_eq!(pool.load("lane-b", bytes, second).unwrap(), ["lane-a"]);
    assert!(!pool.contains("lane-a"));
    assert!(
        pool.get("lane-b")
            .unwrap()
            .run(&manifest.pack_id, "alice visited clinic today")
            .is_ok()
    );

    // A pinned model cannot be displaced or unloaded until unpinned.
    pool.set_pinned("lane-b", true).unwrap();
    let third = runtime.prepare(&dir, &manifest).unwrap();
    assert!(matches!(
        pool.load("lane-c", bytes, third),
        Err(ResidencyError::BudgetHeldByPinned { .. })
    ));
    assert!(pool.contains("lane-b") && !pool.contains("lane-c"));
    assert_eq!(
        pool.unload("lane-b").err(),
        Some(ResidencyError::Pinned("lane-b".into()))
    );
    pool.set_pinned("lane-b", false).unwrap();
    assert!(pool.unload("lane-b").is_ok());
    assert_eq!(pool.used_bytes(), 0);
}

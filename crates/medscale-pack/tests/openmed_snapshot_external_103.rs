//! Spec 103 external qualification: catalog row -> verified Hub snapshot ->
//! signed Pack -> admission -> local ONNX execution. Weights are not vendored;
//! the caller supplies them (explicit, pinned qualification acquisition).
//!
//! MEDSCALE_CATALOG_SNAPSHOT=<models.jsonl at the catalog commit>
//! MEDSCALE_CATALOG_COMMIT=<40-hex catalog commit>
//! MEDSCALE_HF_SNAPSHOT_DIR=<dir with metadata.json (Hub API ?blobs=true), config.json, tokenizer.json, <onnx file>>
//! MEDSCALE_HF_ONNX_FILE=model.onnx
//! MEDSCALE_SAMPLE_TEXT="<synthetic sentence>"
//! MEDSCALE_PLAN_CACHE=1 (optional: also qualify the tract plan cache, cold then warm)
//! cargo test --release -p medscale-pack --test openmed_snapshot_external_103 -- --ignored --nocapture

use std::path::PathBuf;
use std::time::Instant;

use medscale_pack::{
    DocumentRunOptions, HfRepoMetadata, ModelCatalog, OnnxTokenClassifierRuntime, PlanCacheOutcome,
    TokenClassifierSnapshot, admit_pack_dir, build_token_classifier_pack,
};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

#[test]
#[ignore = "requires a pinned, locally supplied Hugging Face snapshot; weights are not vendored"]
fn catalog_row_to_verified_pack_to_local_execution() {
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
    let onnx_file = env("MEDSCALE_HF_ONNX_FILE");
    let out = std::env::temp_dir().join(format!("medscale-103-pack-{}", std::process::id()));

    let pack_id = build_token_classifier_pack(
        &TokenClassifierSnapshot {
            meta: &meta,
            row,
            snapshot_dir: &snapshot,
            onnx_file: &onnx_file,
            fixed_sequence_length: 128,
            tokenizer_source: None,
        },
        &out,
    )
    .expect("verified snapshot must build a Pack");
    let manifest = admit_pack_dir(&out).expect("signed Pack must admit");
    assert_eq!(manifest.pack_id.as_str(), pack_id);

    let runtime = OnnxTokenClassifierRuntime::new(128).unwrap();
    let t0 = Instant::now();
    let prepared = runtime
        .prepare(&out, &manifest)
        .expect("model must prepare locally");
    let prepare_ms = t0.elapsed().as_millis();
    let text = env("MEDSCALE_SAMPLE_TEXT");
    // The product's whole-document path: text longer than one window (for
    // example Telugu under a byte-level tokenizer, batch 10) runs in
    // overlapping windows instead of failing the single-window bound.
    let run = || prepared.run_document(&manifest.pack_id, &text, DocumentRunOptions::default());
    let _ = run().unwrap();
    let t1 = Instant::now();
    let evaluation = run().expect("model must run locally");
    let run_ms = t1.elapsed().as_millis();

    assert!(evaluation.output.evidence_only);
    assert_eq!(evaluation.provenance.repository, meta.id);
    assert_eq!(evaluation.provenance.revision, meta.sha);
    println!(
        "SNAPSHOT_EVIDENCE repo={} revision={} pack={} prepare_ms={} warm_run_ms={}",
        meta.id, meta.sha, pack_id, prepare_ms, run_ms
    );
    let stages = prepared.stage_timings();
    println!(
        "PREPARE_STAGES load_ms={} typed_and_decluttered_ms={} optimized_ms={} runnable_ms={}",
        stages.load_ms, stages.typed_and_decluttered_ms, stages.optimized_ms, stages.runnable_ms
    );
    println!(
        "SNAPSHOT_OUTPUT {}",
        serde_json::to_string(&evaluation.output.proposal_payload).unwrap()
    );
    let t2 = Instant::now();
    let document = prepared
        .run_document(&manifest.pack_id, &text, DocumentRunOptions::default())
        .expect("whole-document run");
    println!(
        "SNAPSHOT_DOCUMENT document_ms={} {}",
        t2.elapsed().as_millis(),
        serde_json::to_string(&document.output.proposal_payload["entities"]).unwrap()
    );
    if std::env::var("MEDSCALE_PLAN_CACHE").as_deref() == Ok("1") {
        let cache = out.with_extension("plan-cache");
        let mut timings = Vec::new();
        let mut outcomes = Vec::new();
        for _ in 0..2 {
            let t = Instant::now();
            let (cached, outcome) = runtime
                .prepare_with_plan_cache(&out, &manifest, &cache)
                .expect("plan-cache prepare");
            timings.push(t.elapsed().as_millis());
            outcomes.push(outcome);
            let again = cached
                .run_document(&manifest.pack_id, &text, DocumentRunOptions::default())
                .expect("plan-cache document run");
            assert_eq!(
                again.output.proposal_payload, document.output.proposal_payload,
                "a cached plan must reproduce the uncached output exactly"
            );
        }
        // Some graphs hold ops NNEF cannot serialize; they fall back to an
        // uncached build (NotCached both times). A written plan must hit next.
        assert!(
            matches!(
                outcomes.as_slice(),
                [PlanCacheOutcome::Written, PlanCacheOutcome::Hit]
                    | [PlanCacheOutcome::NotCached, PlanCacheOutcome::NotCached]
            ),
            "{outcomes:?}"
        );
        let bytes: u64 = std::fs::read_dir(&cache)
            .map(|dir| {
                dir.map(|entry| entry.unwrap().metadata().unwrap().len())
                    .sum()
            })
            .unwrap_or(0);
        println!(
            "PLAN_CACHE_EVIDENCE outcomes={outcomes:?} cold_prepare_ms={} warm_prepare_ms={} cache_bytes={bytes} outputs_identical=true",
            timings[0], timings[1]
        );
        let _ = std::fs::remove_dir_all(&cache);
    }
    // Optional deterministic accuracy evaluation on a gold set
    // (`MEDSCALE_EVAL_SET`, for example the synthetic PII set). Evaluation
    // evidence only; never clinical validation.
    if let Some(path) = std::env::var("MEDSCALE_EVAL_SET")
        .ok()
        .filter(|p| !p.is_empty())
    {
        use medscale_pack::entity_eval::{EvalSet, evaluate, predicted_spans_at};
        // Repository-relative paths resolve against the workspace root
        // (`cargo test` runs in the crate directory).
        let path = std::path::Path::new(&path);
        let path = if path.is_relative() {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path)
        } else {
            path.to_path_buf()
        };
        let set = EvalSet::from_json(&std::fs::read(&path).unwrap()).expect("valid gold set");
        // The OpenMed-compatible default and MedScale's optional decoders.
        let configs = [
            ("openmed_default", DocumentRunOptions::default(), "entities"),
            (
                "word_first",
                DocumentRunOptions {
                    aggregation: medscale_pack::EntityAggregation::WordFirst,
                    ..DocumentRunOptions::default()
                },
                "entities",
            ),
            (
                "word_average",
                DocumentRunOptions {
                    aggregation: medscale_pack::EntityAggregation::WordAverage,
                    ..DocumentRunOptions::default()
                },
                "entities",
            ),
            (
                "pii_merge",
                DocumentRunOptions {
                    pii_semantic_merge: true,
                    ..DocumentRunOptions::default()
                },
                "pii_merged_entities",
            ),
            (
                "word_first_pii_merge",
                DocumentRunOptions {
                    aggregation: medscale_pack::EntityAggregation::WordFirst,
                    pii_semantic_merge: true,
                    ..DocumentRunOptions::default()
                },
                "pii_merged_entities",
            ),
        ];
        for (config, options, field) in configs {
            let mut predictions = std::collections::BTreeMap::new();
            for doc in &set.documents {
                let payload = prepared
                    .run_document(&manifest.pack_id, &doc.text, options)
                    .expect("gold document run")
                    .output
                    .proposal_payload;
                predictions.insert(doc.id.clone(), predicted_spans_at(&payload, field));
            }
            let report = evaluate(&set, &predictions);
            println!(
                "EVAL_METRICS {}",
                serde_json::json!({"repo": meta.id, "revision": meta.sha, "config": config, "report": report})
            );
        }
    }
    let _ = std::fs::remove_dir_all(&out);
}

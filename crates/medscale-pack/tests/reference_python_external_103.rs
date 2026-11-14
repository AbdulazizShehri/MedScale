//! Spec 103 head-to-head: scores spans that pinned OpenMed (PyPI
//! `openmed==3.0.0`, `scripts/evaluation/reference_predict.py`) predicted on a
//! MedScale gold set, with the same scorer MedScale's own runs use. Ignored:
//! it needs `MEDSCALE_REFERENCE_PREDICTIONS` and `MEDSCALE_EVAL_SET` from the
//! qualification workflow. Synthetic text; evaluation evidence only.

use medscale_pack::entity_eval::{EvalSet, PredictedSpan, evaluate, latency_summary};

fn resolve(path: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(path);
    if path.is_relative() {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    } else {
        path.to_path_buf()
    }
}

#[test]
#[ignore = "needs MEDSCALE_REFERENCE_PREDICTIONS and MEDSCALE_EVAL_SET"]
fn reference_python_predictions_score_on_gold_set() {
    let predictions_path = std::env::var("MEDSCALE_REFERENCE_PREDICTIONS").expect("predictions");
    let set_path = std::env::var("MEDSCALE_EVAL_SET").expect("gold set");
    let set = EvalSet::from_json(&std::fs::read(resolve(&set_path)).unwrap()).expect("gold set");
    let run: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&predictions_path).unwrap()).expect("JSON");

    let mut predictions = std::collections::BTreeMap::new();
    for doc in &set.documents {
        let spans = run["predictions"][&doc.id]
            .as_array()
            .unwrap_or_else(|| panic!("no predictions for {}", doc.id))
            .iter()
            .map(|e| PredictedSpan {
                start: usize::try_from(e["start"].as_u64().unwrap()).unwrap(),
                end: usize::try_from(e["end"].as_u64().unwrap()).unwrap(),
                model_label: e["label"].as_str().unwrap().to_owned(),
            })
            .collect();
        predictions.insert(doc.id.clone(), spans);
    }
    let report = evaluate(&set, &predictions);
    let samples: Vec<f64> = run["doc_latency_ms"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .collect();
    assert_eq!(samples.len(), set.documents.len());
    println!(
        "EVAL_METRICS {}",
        serde_json::json!({
            "repo": run["repo"], "revision": run["revision"],
            "config": "reference_python_default", "report": report,
            "runtime": format!("openmed-python-{}", run["openmed_version"].as_str().unwrap_or("unknown")),
            "settings": run["settings"],
            "cold_start_ms": run["cold_start_ms"],
            "doc_latency_ms": latency_summary(&samples),
            "peak_rss_kib": run["peak_rss_kib"],
        })
    );
}

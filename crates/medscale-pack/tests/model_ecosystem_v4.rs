//! Consolidated Wave 1 OpenMed v4.0.0 Model Ecosystem conformance & runtime tests.

use std::sync::atomic::AtomicBool;

use medscale_pack::{
    BundleCalibrationProfile, CalibrationMethod, DECISION_CALIBRATION_EXTENSION,
    DecisionCalibrationReason, DeviceTier, ModelCatalog, PPM_SCALE, budget_bucket_bytes,
    diff_catalog_manifests, evaluate_texts_ordered_cancellable, jcs_canonical_bytes,
    jcs_content_id, preflight_model_resources, recommend_catalog_models,
    render_medscale_model_card, verify_calibration_profile_bounds,
};
use serde_json::Value;

const VECTORS: &str = include_str!("vectors/model_ecosystem_v4.json");
const COMMIT: &str = "5b79cae5df6bc22b0fcdc799be480ee4ffc23c57";

fn vectors() -> Value {
    serde_json::from_str(VECTORS).expect("parse v4 vectors")
}

#[test]
fn conformance_recommendations_diff_card_jcs_calibration_and_buckets() {
    let v = vectors();
    let rows = v["manifest_rows"].as_array().unwrap();
    let jsonl: String = rows
        .iter()
        .map(|r| format!("{}\n", serde_json::to_string(r).unwrap()))
        .collect();
    let catalog = ModelCatalog::import("openmed/v4", COMMIT, jsonl.as_bytes()).unwrap();

    // Rich fields must be preserved on CatalogRow (no longer dropped)
    let phone_row = catalog.get("OpenMed/Synthetic-Phone-NER").unwrap();
    assert_eq!(phone_row.arxiv.as_deref(), Some("2501.00001"));
    assert_eq!(phone_row.canonical_labels, vec!["DISEASE", "DRUG"]);
    assert_eq!(phone_row.download_mb, Some(110.0));
    assert_eq!(phone_row.recommended_tier.as_deref(), Some("phone"));
    assert_eq!(
        phone_row
            .script_coverage
            .get("Arabic")
            .and_then(Value::as_f64),
        Some(0.98)
    );

    // Device-tier recommendations match OpenMed Python v4 order identically
    for tier_name in ["phone", "laptop", "workstation", "server"] {
        let tier = DeviceTier::parse(tier_name).unwrap();
        let actual: Vec<&str> =
            recommend_catalog_models(&catalog, tier, Some("token-classification"), Some("en"))
                .into_iter()
                .map(|r| r.repo_id.as_str())
                .collect();
        let expected: Vec<&str> = v["recommendations_by_tier"][tier_name]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        assert_eq!(actual, expected, "tier mismatch: {tier_name}");
    }

    // Manifest diff matches v4 conformance vector
    let mut rows_v2 = rows[1..].to_vec();
    let r0 = rows_v2[0].as_object_mut().unwrap();
    r0.insert("param_count".into(), Value::from(115_000_000));
    r0.insert("formats".into(), serde_json::json!(["pytorch", "onnx"]));
    r0.insert(
        "benchmark".into(),
        serde_json::json!({"recall": 0.97, "f1": 0.95}),
    );
    let mut added_row = rows[0].clone();
    added_row.as_object_mut().unwrap().insert(
        "repo_id".into(),
        Value::from("OpenMed/Synthetic-Added-Tiny"),
    );
    rows_v2.push(added_row);
    let jsonl_v2: String = rows_v2
        .iter()
        .map(|r| format!("{}\n", serde_json::to_string(r).unwrap()))
        .collect();
    let diff = diff_catalog_manifests(jsonl.as_bytes(), jsonl_v2.as_bytes()).unwrap();
    let exp_added: Vec<&str> = v["manifest_diff"]["added"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    let exp_removed: Vec<&str> = v["manifest_diff"]["removed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    let exp_changed: Vec<&str> = v["manifest_diff"]["changed_repos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert_eq!(diff.added, exp_added);
    assert_eq!(diff.removed, exp_removed);
    assert_eq!(
        diff.changed
            .iter()
            .map(|c| c.repo_id.as_str())
            .collect::<Vec<_>>(),
        exp_changed
    );

    // Model card renders MedScale provenance and safety notice
    let card = render_medscale_model_card(phone_row);
    assert!(card.contains(&phone_row.medscale_id));
    assert!(card.contains("OpenMed/Synthetic-Phone-NER"));
    assert!(card.contains("2501.00001"));
    assert!(card.contains("apache-2.0"));

    // RFC 8785 I-JSON JCS canonical bytes & SHA-256 content_id match Python reference
    for case in v["jcs_vectors"].as_array().unwrap() {
        let canonical = String::from_utf8(jcs_canonical_bytes(&case["input"]).unwrap()).unwrap();
        assert_eq!(canonical, case["canonical_utf8"].as_str().unwrap());
        assert_eq!(
            jcs_content_id(&case["input"]).unwrap(),
            case["content_id"].as_str().unwrap()
        );
    }
    // Non-integer float is refused fail-closed under integer-only I-JSON JCS
    assert!(jcs_canonical_bytes(&serde_json::json!({"score": 0.5})).is_err());

    // Calibration constants & bounds
    assert_eq!(
        DECISION_CALIBRATION_EXTENSION,
        v["calibration_constants"]["extension"].as_str().unwrap()
    );
    assert_eq!(
        PPM_SCALE,
        v["calibration_constants"]["ppm_scale"].as_u64().unwrap()
    );
    let valid_cal = BundleCalibrationProfile {
        calibration_id: "cal-v1".into(),
        version: 1,
        method: CalibrationMethod::DeterministicMargin,
        min_margin_ppm: 100_000,
        abstention_threshold_ppm: 800_000,
        model_id: "medscale-ner".into(),
        runtime_digests: vec!["sha256:aaa".into()],
    };
    verify_calibration_profile_bounds(&valid_cal).unwrap();
    let bad_cal = BundleCalibrationProfile {
        version: 9,
        ..valid_cal
    };
    assert_eq!(
        verify_calibration_profile_bounds(&bad_cal).unwrap_err(),
        DecisionCalibrationReason::CalibrationVersionUnsupported
    );

    // Device capability MiB power-of-2 buckets match Python reference
    for case in v["device_capability_buckets"]["cases"].as_array().unwrap() {
        let input = case["input_bytes"].as_u64().unwrap();
        let expected = case["bucket_bytes"].as_u64().unwrap();
        assert_eq!(budget_bucket_bytes(input), Some(expected), "input {input}");
    }

    // RAM & Disk preflight check
    let ok_preflight = preflight_model_resources(phone_row, DeviceTier::Phone, 4096.0, 1000.0);
    assert!(ok_preflight.fits_ram && ok_preflight.fits_disk);
    let low_ram = preflight_model_resources(phone_row, DeviceTier::Phone, 512.0, 1000.0);
    assert!(!low_ram.fits_ram);
}

#[test]
fn real_catalog_snapshot_recommends_across_tiers_and_batch_cancels() {
    let manifest = include_bytes!("../../../third_party/medscale-model-catalog-v3/models.jsonl");
    let catalog = ModelCatalog::import("medscale/catalog", COMMIT, manifest).unwrap();
    assert_eq!(catalog.len(), 2266);

    let phone = recommend_catalog_models(
        &catalog,
        DeviceTier::Phone,
        Some("token-classification"),
        Some("en"),
    );
    assert!(!phone.is_empty());

    let inputs = vec!["first note".to_owned(), "second note".to_owned()];
    let cancelled = AtomicBool::new(true);
    let err = evaluate_texts_ordered_cancellable(&inputs, &cancelled, "cancelled", |s| {
        Ok(serde_json::json!({ "text": s }))
    })
    .unwrap_err();
    assert_eq!(err, Ok("cancelled"));

    let active = AtomicBool::new(false);
    let ok = evaluate_texts_ordered_cancellable(&inputs, &active, "cancelled", |s| {
        Ok(serde_json::json!({ "len": s.len() }))
    })
    .unwrap();
    assert_eq!(ok.len(), 2);
    assert_eq!(ok[0].index, 0);
    assert_eq!(ok[1].index, 1);
}

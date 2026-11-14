//! Spec 103: whole-document token classification on the real tiny ONNX
//! fixture (static sequence length 4), using the windowing and entity decoding
//! ported from OpenMed v3.0.0. Synthetic text only.

use std::path::PathBuf;

use medscale_pack::{
    DocumentRunOptions, OnnxRuntimeError, OnnxTokenClassifierRuntime, WindowError, admit_pack_dir,
};

fn fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0",
    )
}

const DOC: &str = "alice visited clinic today bob called alice visited";

#[test]
fn long_document_is_fully_covered_instead_of_truncated() {
    let root = fixture_pack();
    let manifest = admit_pack_dir(&root).expect("signed pack admits");
    let prepared = OnnxTokenClassifierRuntime::new(4)
        .unwrap()
        .prepare(&root, &manifest)
        .expect("prepare");

    // The single-window path keeps the tokenizer's truncation: 4 of 8 tokens.
    let single = prepared.run(&manifest.pack_id, DOC).expect("single window");
    assert_eq!(single.output.proposal_payload["token_count"], 4);

    let doc = prepared
        .run_document(&manifest.pack_id, DOC, DocumentRunOptions::default())
        .expect("document run");
    let p = &doc.output.proposal_payload;
    assert!(doc.output.evidence_only);
    assert_eq!(p["kind"], "onnx_token_classification_document_v1");
    assert_eq!(p["token_count"], 8);
    assert_eq!(p["covered_tokens"], 8);
    assert_eq!(p["window_tokens"], 4);
    assert_eq!(p["stride"], 0);
    assert_eq!(p["window_count"], 2);
    let chars: Vec<char> = DOC.chars().collect();
    for e in p["entities"].as_array().unwrap() {
        let (start, end) = (
            usize::try_from(e["start"].as_u64().unwrap()).unwrap(),
            usize::try_from(e["end"].as_u64().unwrap()).unwrap(),
        );
        let slice: String = chars[start..end].iter().collect();
        assert_eq!(e["text"], slice.as_str());
        assert!(!slice.contains(' '), "unprefixed labels split on gaps");
        let score = e["score"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&score));
    }

    let overlapped = prepared
        .run_document(
            &manifest.pack_id,
            DOC,
            DocumentRunOptions {
                threshold: 0.0,
                stride: Some(1),
                ..Default::default()
            },
        )
        .expect("overlapping windows");
    assert_eq!(overlapped.output.proposal_payload["window_count"], 3);
    assert_eq!(overlapped.output.proposal_payload["covered_tokens"], 8);

    assert!(
        overlapped
            .output
            .proposal_payload
            .get("pii_merged_entities")
            .is_none(),
        "merge is opt-in"
    );

    let merged = prepared
        .run_document(
            &manifest.pack_id,
            DOC,
            DocumentRunOptions {
                pii_semantic_merge: true,
                ..Default::default()
            },
        )
        .expect("document run with PII merge");
    let p = &merged.output.proposal_payload;
    assert!(p["pii_merged_entities"].is_array());
    assert!(
        p.get("aggregation").is_none(),
        "default aggregation is not reported"
    );

    let word = prepared
        .run_document(
            &manifest.pack_id,
            DOC,
            DocumentRunOptions {
                aggregation: medscale_pack::EntityAggregation::WordFirst,
                ..Default::default()
            },
        )
        .expect("document run with word aggregation");
    let wp = &word.output.proposal_payload;
    assert_eq!(wp["aggregation"], "word_first");
    let chars: Vec<char> = DOC.chars().collect();
    for e in wp["entities"].as_array().unwrap() {
        let (start, end) = (
            usize::try_from(e["start"].as_u64().unwrap()).unwrap(),
            usize::try_from(e["end"].as_u64().unwrap()).unwrap(),
        );
        assert_eq!(
            e["text"],
            chars[start..end].iter().collect::<String>().as_str()
        );
    }
    assert_eq!(
        p["entities"],
        overlapped_default_entities(&prepared, &manifest)
    );
}

fn overlapped_default_entities(
    prepared: &medscale_pack::PreparedOnnxTokenClassifier,
    manifest: &medscale_contracts::packs::PackManifestV0,
) -> serde_json::Value {
    prepared
        .run_document(&manifest.pack_id, DOC, DocumentRunOptions::default())
        .unwrap()
        .output
        .proposal_payload["entities"]
        .clone()
}

#[test]
fn a_cancelled_document_run_stops_without_output() {
    let root = fixture_pack();
    let manifest = admit_pack_dir(&root).unwrap();
    let prepared = OnnxTokenClassifierRuntime::new(4)
        .unwrap()
        .prepare(&root, &manifest)
        .unwrap();
    let cancelled = std::sync::atomic::AtomicBool::new(true);
    assert!(matches!(
        prepared.run_document_cancellable(
            &manifest.pack_id,
            DOC,
            DocumentRunOptions::default(),
            &cancelled
        ),
        Err(OnnxRuntimeError::Cancelled)
    ));
}

#[test]
fn document_run_refuses_invalid_options() {
    let root = fixture_pack();
    let manifest = admit_pack_dir(&root).unwrap();
    let prepared = OnnxTokenClassifierRuntime::new(4)
        .unwrap()
        .prepare(&root, &manifest)
        .unwrap();
    assert!(matches!(
        prepared.run_document(
            &manifest.pack_id,
            DOC,
            DocumentRunOptions {
                threshold: 1.5,
                stride: None,
                ..Default::default()
            }
        ),
        Err(OnnxRuntimeError::InvalidThreshold)
    ));
    assert!(matches!(
        prepared.run_document(
            &manifest.pack_id,
            DOC,
            DocumentRunOptions {
                threshold: 0.0,
                stride: Some(4),
                ..Default::default()
            }
        ),
        Err(OnnxRuntimeError::Windowing(WindowError::StrideTooLarge))
    ));
    assert!(matches!(
        prepared.run_document(&manifest.pack_id, "", DocumentRunOptions::default()),
        Err(OnnxRuntimeError::TokenBound)
    ));
    let oversized = "a ".repeat(40_000);
    assert!(matches!(
        prepared.run_document(&manifest.pack_id, &oversized, DocumentRunOptions::default()),
        Err(OnnxRuntimeError::InputBound)
    ));
}

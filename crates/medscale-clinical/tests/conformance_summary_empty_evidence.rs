//! The Rust empty-evidence gate selects and refuses exactly like the
//! reference package. Vectors come from
//! `integrations/clinical-evidence-python/conformance/generate_vectors.py`;
//! each record's approval is classified by the reference itself.

use medscale_clinical::summary_empty_evidence::{Approval, require_approved_evidence};
use serde_json::Value;

#[test]
fn selection_and_refusals_match_the_reference_byte_for_byte() {
    let vectors: Value =
        serde_json::from_str(include_str!("vectors/summary_empty_evidence.json")).expect("parse");
    let cases = vectors["cases"].as_array().expect("cases");
    assert!(cases.len() >= 6);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let candidates: Vec<(usize, Approval)> = case["approvals"]
            .as_array()
            .expect("approvals")
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let approval = match a.as_str().expect("approval") {
                    "approved" => Approval::Approved,
                    "unapproved" => Approval::Unapproved,
                    "invalid" => Approval::Invalid,
                    other => panic!("{name}: unknown approval {other}"),
                };
                (i, approval)
            })
            .collect();
        match (require_approved_evidence(candidates), &case["refusal_json"]) {
            (Ok(selected), Value::Null) => {
                let expected: Vec<usize> = case["selected"]
                    .as_array()
                    .expect("selected")
                    .iter()
                    .map(|v| usize::try_from(v.as_u64().expect("index")).expect("fits"))
                    .collect();
                assert_eq!(selected, expected, "{name}");
            }
            (Err(refusal), Value::String(json)) => {
                assert_eq!(&refusal.to_json(), json, "{name}");
            }
            (got, want) => panic!("{name}: got {got:?}, reference {want}"),
        }
    }
}

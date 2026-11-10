//! The Rust evidence coverage matrix reproduces the reference package's
//! reports exactly. Vectors come from
//! `integrations/clinical-evidence-python/conformance/generate_vectors.py`.

use medscale_clinical::evidence_coverage::{
    CoverageError, CoverageMatrix, CoverageStatus, Requirement, build_coverage_matrix,
};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!("vectors/evidence_coverage.json")).expect("parse")
}

fn run(case: &Value) -> Result<CoverageMatrix, CoverageError> {
    let requirements = case["requirements"]
        .as_array()
        .expect("requirements")
        .iter()
        .map(|row| Requirement {
            claim_id: row[0].as_str().expect("claim").to_owned(),
            evidence_class: row[1].as_str().expect("class").to_owned(),
            status: match row[2].as_str().expect("status") {
                "present" => CoverageStatus::Present,
                "missing" => CoverageStatus::Missing,
                "conflicting" => CoverageStatus::Conflicting,
                "unreviewed" => CoverageStatus::Unreviewed,
                other => panic!("unknown status {other}"),
            },
            source_fingerprints: row[3]
                .as_array()
                .expect("fingerprints")
                .iter()
                .map(|f| f.as_str().expect("digest").to_owned())
                .collect(),
        })
        .collect();
    build_coverage_matrix(requirements)
}

#[test]
fn matrices_match_the_reference_byte_for_byte() {
    let vectors = vectors();
    let cases = vectors["cases"].as_array().expect("cases");
    assert!(cases.len() >= 8);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let matrix = run(case).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(matrix.to_value(), case["expected"], "{name}");
        assert_eq!(
            matrix.to_json(),
            case["expected_json"].as_str().expect("json"),
            "{name}: canonical JSON and hashes"
        );
    }
}

#[test]
fn invalid_requirements_fail_like_the_reference() {
    let vectors = vectors();
    for case in vectors["errors"].as_array().expect("errors") {
        let name = case["name"].as_str().expect("name");
        let err = run(case).expect_err(name);
        assert_eq!(
            err.to_string(),
            case["error"].as_str().expect("error"),
            "{name}"
        );
    }
}

#[test]
fn raw_fingerprints_are_refused_rather_than_hashed() {
    let err = build_coverage_matrix(vec![Requirement {
        claim_id: "claim-a".to_owned(),
        evidence_class: "local_record".to_owned(),
        status: CoverageStatus::Present,
        source_fingerprints: vec!["Patient Jane Doe".to_owned()],
    }])
    .expect_err("not a digest");
    assert_eq!(err, CoverageError::Fingerprint);
}

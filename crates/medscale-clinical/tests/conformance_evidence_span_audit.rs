//! The Rust evidence-span audit reproduces the reference package's reports
//! exactly. Vectors come from
//! `integrations/clinical-evidence-python/conformance/generate_vectors.py`.

use medscale_clinical::evidence_span_audit::{
    EvidenceSpan, MAX_AUDIT_SPANS, OverlapKind, SpanAuditError, audit_evidence_spans, audit_records,
};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!("vectors/evidence_span_audit.json")).expect("vectors parse")
}

fn rows(case: &Value) -> Vec<(String, String, u64, u64)> {
    case["spans"]
        .as_array()
        .expect("spans")
        .iter()
        .map(|row| {
            (
                row[0].as_str().expect("source").to_owned(),
                row[1].as_str().expect("evidence").to_owned(),
                row[2].as_u64().expect("start"),
                row[3].as_u64().expect("end"),
            )
        })
        .collect()
}

fn run(
    case: &Value,
) -> Result<medscale_clinical::evidence_span_audit::EvidenceSpanAudit, SpanAuditError> {
    let owned = rows(case);
    let borrowed: Vec<(&str, &str, u64, u64)> = owned
        .iter()
        .map(|(s, e, a, b)| (s.as_str(), e.as_str(), *a, *b))
        .collect();
    audit_records(&borrowed)
}

#[test]
fn reports_match_the_reference_byte_for_byte() {
    let vectors = vectors();
    let cases = vectors["cases"].as_array().expect("cases");
    assert!(cases.len() >= 10);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let audit = run(case).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(audit.to_value(), case["expected"], "{name}: report");
        assert_eq!(
            audit.to_json(),
            case["expected_json"].as_str().expect("json"),
            "{name}: canonical JSON and fingerprint"
        );
    }
}

#[test]
fn invalid_spans_fail_like_the_reference() {
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
fn input_order_does_not_change_the_report() {
    let vectors = vectors();
    let case = vectors["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "mixed_unordered")
        .expect("case");
    let mut owned = rows(case);
    let forward = run(case).expect("audit").to_json();
    owned.reverse();
    let spans = owned
        .iter()
        .map(|(s, e, a, b)| EvidenceSpan::new(s, e, *a, *b).expect("span"))
        .collect();
    assert_eq!(
        audit_evidence_spans(spans).expect("audit").to_json(),
        forward
    );
}

#[test]
fn reports_never_contain_raw_identifiers() {
    let audit = audit_records(&[
        ("Patient Jane Doe note", "MRN-004512", 0, 10),
        ("Patient Jane Doe note", "MRN-004513", 5, 15),
    ])
    .expect("audit");
    let json = audit.to_json();
    assert!(!json.contains("Jane") && !json.contains("MRN"), "{json}");
    assert_eq!(audit.count(OverlapKind::Partial), 1);
}

#[test]
fn oversized_input_is_refused() {
    let spans = (0..=MAX_AUDIT_SPANS as u64)
        .map(|i| EvidenceSpan::new("s", &format!("e{i}"), i, i + 1).expect("span"))
        .collect();
    assert_eq!(audit_evidence_spans(spans), Err(SpanAuditError::TooMany));
}

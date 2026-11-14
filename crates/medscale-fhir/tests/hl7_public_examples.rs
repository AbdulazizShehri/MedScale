//! B2: MedScale's FHIR import against public HL7 examples (US Core and IPS,
//! CC0, fictional people; see fixtures/public/hl7/NOTICE.md). Every file must
//! pass the gate, keep each decimal exactly as written, split into its entries
//! by byte position, and cite the exact member a field came from.

use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::presentation::SpanCitation;
use medscale_fhir::{
    ExtractContext, bundle_entry_spans, extract_resource, gate_fhir_json, parse_exact_decimals,
};
use serde_json::Value;
use std::path::PathBuf;

const FILES: [&str; 5] = [
    "Bundle-IPS-examples-Bundle-01.json",
    "Bundle-66c8856b-ba11-4876-8aa8-467aad8c11a2.json",
    "Patient-example.json",
    "Observation-blood-pressure.json",
    "Observation-serum-potassium.json",
];

fn read(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/public/hl7")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every number token written with a fraction or an exponent, as text.
fn decimal_tokens(bytes: &[u8]) -> Vec<String> {
    let text = std::str::from_utf8(bytes).unwrap();
    let mut out = Vec::new();
    let mut in_string = false;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if in_string {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        if c == '"' {
            in_string = true;
        } else if c == '-' || c.is_ascii_digit() {
            let mut end = i + c.len_utf8();
            while let Some(&(j, d)) = chars.peek() {
                if d.is_ascii_digit() || matches!(d, '.' | 'e' | 'E' | '+' | '-') {
                    end = j + d.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let token = &text[i..end];
            if token.contains(['.', 'e', 'E']) {
                out.push(token.to_owned());
            }
        }
    }
    out
}

fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

#[test]
fn every_example_passes_the_gate_and_keeps_its_decimals_exactly() {
    for name in FILES {
        let bytes = read(name);
        let report = gate_fhir_json("application/fhir+json", &bytes, None)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let parsed = parse_exact_decimals(&bytes).unwrap();
        assert_eq!(
            parsed["resourceType"],
            report.resource_type.as_str(),
            "{name}"
        );
        // A decimal parses to the exact text it was written as.
        let mut texts = Vec::new();
        strings(&parsed, &mut texts);
        for token in decimal_tokens(&bytes) {
            assert!(
                texts.contains(&token),
                "{name}: decimal {token} was not kept exactly"
            );
        }
    }
}

#[test]
fn every_bundle_splits_into_its_entries_by_byte_position() {
    for name in FILES.iter().filter(|n| n.starts_with("Bundle-")) {
        let bytes = read(name);
        let parsed = parse_exact_decimals(&bytes).unwrap();
        let entries = parsed["entry"].as_array().unwrap();
        let spans = bundle_entry_spans(&bytes).unwrap();
        assert_eq!(spans.len(), entries.len(), "{name}");
        for ((start, end), entry) in spans.iter().zip(entries) {
            let piece = parse_exact_decimals(&bytes[*start..*end]).unwrap();
            assert_eq!(
                &piece, &entry["resource"],
                "{name}: entry bytes parse to the same resource"
            );
            gate_fhir_json("application/fhir+json", &bytes[*start..*end], None)
                .unwrap_or_else(|e| panic!("{name}: an entry fails the gate: {e}"));
        }
    }
}

/// Each text citation covers `"key": value` for the field it backs, inside
/// the cited resource, so it can be shown to a person as the source.
fn assert_citations(name: &str, bytes: &[u8], resource: &Value, within: (usize, usize)) -> usize {
    let mut spans = 0;
    let ctx = ExtractContext {
        source_id: Some(OpaqueId::new("hl7-example")),
        content_digest: Some(DigestSha256::of(bytes)),
        source_bytes: Some(bytes.to_vec()),
        evidence_healthy: true,
        resource_id: resource["id"].as_str().map(str::to_owned),
    };
    let extraction = extract_resource(resource, &ctx);
    for field in &extraction.fields {
        // The JSON key the field came from is the last segment of its path note.
        let key = field
            .span_citations
            .iter()
            .find_map(|c| match c {
                SpanCitation::WholeResource { citation } => citation.structural_path_note.clone(),
                SpanCitation::TextSpan { .. } => None,
            })
            .and_then(|note| note.rsplit('.').next().map(str::to_owned));
        for cite in &field.span_citations {
            let SpanCitation::TextSpan { span, .. } = cite else {
                continue;
            };
            spans += 1;
            let (start, end) = (span.start as usize, span.end as usize);
            assert!(
                start >= within.0 && end <= within.1,
                "{name}: {} cites outside its resource",
                field.field_key
            );
            let text = std::str::from_utf8(&bytes[start..end]).unwrap();
            let key = key.as_deref().expect("a text citation has a path note");
            assert!(
                text.starts_with(&format!("\"{key}\"")),
                "{name}: {} cites {text:.60}",
                field.field_key
            );
            let (_, value) = text.split_once(':').unwrap();
            serde_json::from_str::<Value>(value.trim()).unwrap_or_else(|e| {
                panic!(
                    "{name}: {} span is not a whole member: {e}",
                    field.field_key
                )
            });
        }
    }
    spans
}

#[test]
fn citations_cover_the_exact_member_of_the_cited_resource() {
    let mut cited = 0;
    let mut text_spans = 0;
    for name in FILES {
        let bytes = read(name);
        let parsed = parse_exact_decimals(&bytes).unwrap();
        if parsed["resourceType"] == "Bundle" {
            let spans = bundle_entry_spans(&bytes).unwrap();
            for ((start, end), entry) in spans.into_iter().zip(parsed["entry"].as_array().unwrap())
            {
                text_spans += assert_citations(name, &bytes, &entry["resource"], (start, end));
                cited += 1;
            }
        } else {
            text_spans += assert_citations(name, &bytes, &parsed, (0, bytes.len()));
            cited += 1;
        }
    }
    assert!(cited >= 26, "every resource was checked: {cited}");
    assert!(
        text_spans >= 10,
        "text citations were found and checked: {text_spans}"
    );
}

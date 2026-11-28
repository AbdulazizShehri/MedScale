use std::collections::BTreeMap;

use medscale_fhir::{
    UcumV4Status, convert_analyte_concentration, normalize_ucum_v4, parse_ucum_v4,
};
use serde_json::Value;

fn status_from_str(s: &str) -> UcumV4Status {
    match s {
        "ok" => UcumV4Status::Ok,
        "unknown" => UcumV4Status::Unknown,
        "ambiguous" => UcumV4Status::Ambiguous,
        "incommensurable" => UcumV4Status::Incommensurable,
        "analyte_context_required" => UcumV4Status::AnalyteContextRequired,
        other => panic!("unexpected vector status: {other}"),
    }
}

#[test]
fn ucum_v4_matches_python_reference_vectors() {
    let raw: Value =
        serde_json::from_str(include_str!("vectors/ucum_v4.json")).expect("valid json");
    for case in raw["parsed"].as_array().expect("parsed array") {
        let input = case["input"].as_str().expect("input");
        let parsed = parse_ucum_v4(input);
        assert_eq!(
            parsed.status,
            status_from_str(case["status"].as_str().expect("status")),
            "parse status mismatch for `{input}`"
        );
        assert_eq!(
            parsed.canonical_unit.as_deref(),
            case["canonical_unit"].as_str(),
            "canonical_unit mismatch for `{input}`"
        );
        let expected_dim: BTreeMap<String, i32> = case["dimension"]
            .as_object()
            .expect("dim obj")
            .iter()
            .map(|(k, v)| (k.clone(), v.as_i64().expect("int") as i32))
            .collect();
        assert_eq!(
            parsed.dimension, expected_dim,
            "dimension mismatch for `{input}`"
        );
    }

    for case in raw["conversions"].as_array().expect("conversions array") {
        let value = case["value"].as_f64().expect("value");
        let from_unit = case["from_unit"].as_str().expect("from");
        let to_unit = case["to_unit"].as_str().expect("to");
        let got = normalize_ucum_v4(value, from_unit, to_unit);
        assert_eq!(
            got.status,
            status_from_str(case["status"].as_str().expect("status")),
            "conversion status mismatch for {value} {from_unit} -> {to_unit}"
        );
        if got.status == UcumV4Status::Ok {
            let exp_mag = case["magnitude"].as_f64().expect("mag");
            let got_mag = got.magnitude.expect("got mag");
            let tol = 1e-9 * exp_mag.abs().max(1.0);
            assert!(
                (got_mag - exp_mag).abs() <= tol,
                "magnitude mismatch for {value} {from_unit} -> {to_unit}: got {got_mag} vs {exp_mag}"
            );
        }
    }
}

#[test]
fn ucum_v4_analyte_glucose_and_ambiguity_guards() {
    let amb = parse_ucum_v4("units");
    assert_eq!(amb.status, UcumV4Status::Ambiguous);

    let glu = convert_analyte_concentration(180.156, "mg/dL", "mmol/L", "glucose", None);
    assert_eq!(glu.status, UcumV4Status::Ok);
    let mag = glu.magnitude.expect("glucose mmol/L");
    assert!((mag - 10.0).abs() < 1e-6, "got {mag}");

    let unk_analyte = convert_analyte_concentration(100.0, "mg/dL", "mmol/L", "mystery", None);
    assert_eq!(unk_analyte.status, UcumV4Status::AnalyteContextRequired);
    assert!(unk_analyte.magnitude.is_none());
}

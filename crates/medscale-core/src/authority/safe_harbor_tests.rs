use super::*;
use serde_json::json;

/// A synthetic Bundle with a planted marker in every identifier field,
/// tagged with the Safe Harbor category it belongs to.
fn synthetic_bundle() -> (Value, Vec<(SafeHarborCategory, &'static str)>) {
    use SafeHarborCategory as C;
    let markers = vec![
        (C::Names, "Zebulon"),
        (C::Names, "Quillfeather"),
        (C::Names, "Ottoline Brackwater"),
        (C::Names, "Dr Mallory Fenwick"),
        (C::Geographic, "77 Larkspur Lane"),
        (C::Geographic, "Thistlebury"),
        (C::Geographic, "Wrenford County"),
        (C::Geographic, "02134"),
        (C::Dates, "1931-04-17"),
        (C::Dates, "2025-03-09"),
        (C::Dates, "2025-03-09T08:15:00Z"),
        (C::Telephone, "+1-617-555-0142"),
        (C::Fax, "+1-617-555-0199"),
        (C::Email, "zebulon.q@example.org"),
        (C::SocialSecurity, "078-05-1120"),
        (C::MedicalRecord, "MRN-88231"),
        (C::HealthPlan, "PLAN-55102"),
        (C::Account, "ACCT-30017"),
        (C::CertificateLicense, "DL-K4471"),
        (C::Device, "UDI-(01)00844588003288"),
        (C::Device, "SER-99812"),
        (C::Url, "https://zebulon.example.org/home"),
        (C::Photo, "aGVsbG8tcGhvdG8="),
        (C::OtherUnique, "pat-zq-001"),
        (C::OtherUnique, "obs-zq-002"),
    ];
    let ident = |code: &str, value: &str| {
        json!({
            "type": {"coding": [{"system": "http://terminology.hl7.org/CodeSystem/v2-0203", "code": code}]},
            "system": "urn:oid:1.2.3",
            "value": value
        })
    };
    let bundle = json!({
        "resourceType": "Bundle",
        "type": "collection",
        "timestamp": "2025-03-09T08:15:00Z",
        "entry": [
            {
                "fullUrl": "urn:uuid:pat-zq-001",
                "resource": {
                    "resourceType": "Patient",
                    "id": "pat-zq-001",
                    "meta": {"versionId": "1", "lastUpdated": "2025-03-09T08:15:00Z", "source": "https://zebulon.example.org/home"},
                    "text": {"status": "generated", "div": "<div>Zebulon Quillfeather</div>"},
                    "extension": [{"url": "http://example.org/x", "valueString": "Thistlebury"}],
                    "identifier": [
                        ident("SS", "078-05-1120"),
                        ident("MR", "MRN-88231"),
                        ident("MB", "PLAN-55102"),
                        ident("AN", "ACCT-30017"),
                        ident("DL", "DL-K4471")
                    ],
                    "name": [{"family": "Quillfeather", "given": ["Zebulon"], "text": "Zebulon Quillfeather"}],
                    "telecom": [
                        {"system": "phone", "value": "+1-617-555-0142"},
                        {"system": "fax", "value": "+1-617-555-0199"},
                        {"system": "email", "value": "zebulon.q@example.org"},
                        {"system": "url", "value": "https://zebulon.example.org/home"}
                    ],
                    "gender": "male",
                    "birthDate": "1931-04-17",
                    "address": [{
                        "use": "home",
                        "line": ["77 Larkspur Lane"],
                        "city": "Thistlebury",
                        "district": "Wrenford County",
                        "state": "MA",
                        "postalCode": "02134",
                        "country": "US"
                    }],
                    "photo": [{"contentType": "image/png", "data": "aGVsbG8tcGhvdG8="}],
                    "contact": [{"name": {"text": "Ottoline Brackwater"}}],
                    "generalPractitioner": [{"reference": "Practitioner/prac-1", "display": "Dr Mallory Fenwick"}]
                }
            },
            {
                "fullUrl": "urn:uuid:obs-zq-002",
                "resource": {
                    "resourceType": "Observation",
                    "id": "obs-zq-002",
                    "status": "final",
                    "category": [{"coding": [{"system": "http://terminology.hl7.org/CodeSystem/observation-category", "code": "vital-signs", "display": "Vital Signs"}]}],
                    "code": {"coding": [{"system": "http://loinc.org", "code": "29463-7", "display": "Body weight"}], "text": "Weight for Zebulon"},
                    "subject": {"reference": "urn:uuid:pat-zq-001", "display": "Zebulon Quillfeather"},
                    "effectiveDateTime": "2025-03-09",
                    "issued": "2025-03-09T08:15:00Z",
                    "valueQuantity": {"value": 71.5, "unit": "kg", "system": "http://unitsofmeasure.org", "code": "kg"},
                    "note": [{"text": "Zebulon called from +1-617-555-0142"}]
                }
            },
            {
                "resource": {
                    "resourceType": "Practitioner",
                    "id": "prac-1",
                    "name": [{"text": "Dr Mallory Fenwick"}]
                }
            },
            {
                "resource": {
                    "resourceType": "Immunization",
                    "id": "imm-1",
                    "status": "completed",
                    "vaccineCode": {"coding": [{"system": "http://hl7.org/fhir/sid/cvx", "code": "207"}]},
                    "patient": {"reference": "Patient/pat-zq-001"},
                    "occurrenceDateTime": "2025-03-09",
                    "lotNumber": "SER-99812"
                }
            },
            {
                "resource": {
                    "resourceType": "Procedure",
                    "id": "proc-1",
                    "status": "completed",
                    "code": {"coding": [{"system": "http://snomed.info/sct", "code": "80146002"}]},
                    "subject": {"reference": "Patient/pat-zq-001"},
                    "focalDevice": [{"manipulated": {"reference": "Device/dev-1"}}],
                    "usedReference": [{"identifier": {"value": "UDI-(01)00844588003288"}}]
                }
            }
        ]
    });
    (bundle, markers)
}

fn run(value: &Value) -> Result<SafeHarborOutput, SafeHarborError> {
    deidentify_fhir_json(&value.to_string(), 2026)
}

fn parse(out: &SafeHarborOutput) -> Value {
    serde_json::from_str(&out.json).expect("output is JSON")
}

#[test]
fn no_planted_identifier_survives_in_any_category() {
    let (bundle, markers) = synthetic_bundle();
    let out = run(&bundle).expect("synthetic bundle de-identifies");
    let mut per_category: BTreeMap<SafeHarborCategory, (u32, u32)> = BTreeMap::new();
    for (category, marker) in &markers {
        let entry = per_category.entry(*category).or_default();
        entry.0 += 1;
        if out.json.contains(marker) {
            entry.1 += 1;
        }
    }
    for (category, (total, leaked)) in &per_category {
        assert_eq!(
            *leaked,
            0,
            "category {} leaked {leaked} of {total} planted values",
            category.rule_letter()
        );
    }
    assert!(!out.json.contains("Zebulon"), "free text must not survive");
}

#[test]
fn dates_become_years_and_zip_keeps_three_digits() {
    let (bundle, _) = synthetic_bundle();
    let v = parse(&run(&bundle).expect("de-identifies"));
    let obs = &v["entry"][1]["resource"];
    assert_eq!(obs["effectiveDateTime"], "2025");
    assert_eq!(obs["issued"], "2025");
    assert_eq!(v["timestamp"], "2025");
    let address = &v["entry"][0]["resource"]["address"][0];
    assert_eq!(address["postalCode"], "021");
    assert_eq!(address["state"], "MA");
    assert!(address.get("city").is_none() && address.get("line").is_none());
    assert_eq!(obs["valueQuantity"]["value"], 71.5);
    assert_eq!(obs["code"]["coding"][0]["code"], "29463-7");
}

#[test]
fn a_birth_year_showing_90_or_older_is_removed() {
    let (bundle, _) = synthetic_bundle();
    let out = run(&bundle).expect("de-identifies");
    assert!(
        parse(&out)["entry"][0]["resource"]
            .get("birthDate")
            .is_none()
    );
    assert_eq!(out.report.ages_aggregated_90_or_older, 1);

    let young = json!({"resourceType": "Patient", "birthDate": "1980-06-02"});
    let out = run(&young).expect("de-identifies");
    assert!(out.json.contains("\"birthDate\":\"1980\""));
    assert_eq!(out.report.ages_aggregated_90_or_older, 0);
}

#[test]
fn ages_over_89_become_90_or_older() {
    let condition = json!({
        "resourceType": "Condition",
        "onsetAge": {"value": 93, "unit": "years", "system": "http://unitsofmeasure.org", "code": "a"}
    });
    let v = parse(&run(&condition).expect("de-identifies"));
    assert_eq!(v["onsetAge"]["value"], 90);
    assert_eq!(v["onsetAge"]["comparator"], ">=");

    let obs = json!({
        "resourceType": "Observation",
        "status": "final",
        "code": {"coding": [{"system": "http://loinc.org", "code": "30525-0"}]},
        "valueQuantity": {"value": 1150, "unit": "months", "code": "mo"}
    });
    let v = parse(&run(&obs).expect("de-identifies"));
    assert_eq!(v["valueQuantity"]["value"], 90);
    assert_eq!(v["valueQuantity"]["code"], "a");

    let child = json!({"resourceType": "Condition", "onsetAge": {"value": 4, "code": "a"}});
    let out = run(&child).expect("de-identifies");
    assert!(out.json.contains("\"value\":4"));
}

#[test]
fn restricted_zip3_becomes_000_and_non_us_postcodes_are_removed() {
    let restricted =
        json!({"resourceType": "Patient", "address": [{"postalCode": "05901", "country": "US"}]});
    let out = run(&restricted).expect("de-identifies");
    assert!(out.json.contains("\"postalCode\":\"000\""));
    let foreign = json!({"resourceType": "Patient", "address": [{"postalCode": "SW1A 1AA", "country": "GB"}]});
    let out = run(&foreign).expect("de-identifies");
    assert!(!out.json.contains("postalCode"));
    let saudi =
        json!({"resourceType": "Patient", "address": [{"postalCode": "12345", "country": "SA"}]});
    let out = run(&saudi).expect("de-identifies");
    assert!(!out.json.contains("12345"));
}

#[test]
fn references_stay_consistent_without_revealing_ids() {
    let (bundle, _) = synthetic_bundle();
    let v = parse(&run(&bundle).expect("de-identifies"));
    let patient_id = v["entry"][0]["resource"]["id"]
        .as_str()
        .expect("id")
        .to_owned();
    assert_eq!(v["entry"][0]["fullUrl"], format!("urn:uuid:{patient_id}"));
    assert_eq!(
        v["entry"][1]["resource"]["subject"]["reference"],
        format!("urn:uuid:{patient_id}")
    );
    assert_eq!(
        v["entry"][3]["resource"]["patient"]["reference"],
        format!("Patient/{patient_id}")
    );
    assert!(patient_id.starts_with("00000000-0000-4000-8000-"));
    let prac = &v["entry"][2]["resource"];
    assert_eq!(
        prac.as_object().map(Map::len),
        Some(2),
        "reduced to type and id"
    );
}

#[test]
fn the_report_counts_every_category_and_lists_limitations() {
    use SafeHarborCategory as C;
    let (bundle, _) = synthetic_bundle();
    let report = run(&bundle).expect("de-identifies").report;
    assert_eq!(report.categories.len(), 18);
    for (category, letter) in SafeHarborCategory::ALL.iter().zip('A'..='R') {
        assert_eq!(category.rule_letter(), letter);
    }
    assert!(report.category(C::Names).removed >= 3);
    assert_eq!(report.category(C::Telephone).removed, 1);
    assert_eq!(report.category(C::Fax).removed, 1);
    assert_eq!(report.category(C::Email).removed, 1);
    assert_eq!(report.category(C::SocialSecurity).removed, 1);
    assert_eq!(report.category(C::MedicalRecord).removed, 1);
    assert_eq!(report.category(C::HealthPlan).removed, 1);
    assert_eq!(report.category(C::Account).removed, 1);
    assert_eq!(report.category(C::CertificateLicense).removed, 1);
    assert!(report.category(C::Device).removed >= 1);
    assert!(report.category(C::Url).removed >= 2);
    assert_eq!(report.category(C::Photo).removed, 1);
    assert_eq!(report.category(C::Geographic).generalized, 1);
    assert!(report.category(C::Dates).generalized >= 4);
    assert!(report.free_text_fields_removed >= 2);
    assert_eq!(report.narratives_removed, 1);
    assert_eq!(report.extensions_removed, 1);
    assert_eq!(report.limitations, SafeHarborLimitation::ALL.to_vec());
    let text = serde_json::to_string(&report)
        .expect("report serializes")
        .to_lowercase();
    for word in ["compliant", "certified", "anonymi", "zebulon", "02134"] {
        assert!(!text.contains(word), "{word}");
    }
}

#[test]
fn unknown_fields_and_resources_are_refused() {
    let unknown = json!({"resourceType": "Patient", "nickname": "Zeb"});
    assert_eq!(
        run(&unknown),
        Err(SafeHarborError::UnknownField {
            path: "nickname".to_owned()
        })
    );
    let modifier = json!({"resourceType": "Patient", "modifierExtension": [{"url": "x"}]});
    assert!(matches!(
        run(&modifier),
        Err(SafeHarborError::ModifierExtension { .. })
    ));
    let other = json!({"resourceType": "Claim"});
    assert_eq!(
        run(&other),
        Err(SafeHarborError::UnsupportedResource {
            resource_type: "Claim".to_owned()
        })
    );
    let odd_key = json!({"resourceType": "Patient", "Zebulon Q": "x"});
    let Err(SafeHarborError::UnknownField { path }) = run(&odd_key) else {
        panic!("expected refusal");
    };
    assert_eq!(path, "?", "keys that are not plain identifiers are masked");
    assert_eq!(run(&json!({"x": 1})), Err(SafeHarborError::NotFhir));
    assert_eq!(
        deidentify_fhir_json("{", 2026),
        Err(SafeHarborError::Malformed)
    );
    assert_eq!(
        deidentify_fhir_json("{\"resourceType\":\"Patient\"}", 10),
        Err(SafeHarborError::BadReferenceYear)
    );
}

#[test]
fn identifiers_left_in_kept_text_block_the_output() {
    let leaky = json!({
        "resourceType": "Observation",
        "status": "final",
        "code": {"coding": [{"system": "http://loinc.org", "code": "1-1", "display": "Call zebulon.q@example.org"}]}
    });
    assert!(matches!(
        run(&leaky),
        Err(SafeHarborError::ResidualFinding {
            category: SafeHarborCategory::Email,
            ..
        })
    ));
}

#[test]
fn unreadable_references_are_refused() {
    let bad = json!({"resourceType": "Observation", "subject": {"reference": "zebulon"}});
    assert!(matches!(
        run(&bad),
        Err(SafeHarborError::UnreadableReference { .. })
    ));
}

#[test]
fn errors_never_echo_values() {
    let bad = json!({"resourceType": "Patient", "nickname": "Zebulon"});
    let message = run(&bad).expect_err("refused").to_string();
    assert!(!message.contains("Zebulon"));
}

#[test]
fn date_shapes() {
    assert_eq!(fhir_date_year("2025"), Some("2025"));
    assert_eq!(fhir_date_year("2025-03"), Some("2025"));
    assert_eq!(fhir_date_year("2025-03-09"), Some("2025"));
    assert_eq!(fhir_date_year("2025-03-09T08:15:00+03:00"), Some("2025"));
    assert_eq!(fhir_date_year("20250309"), None);
    assert_eq!(fhir_date_year("2025-3-9"), None);
    assert_eq!(fhir_date_year("kg"), None);
}

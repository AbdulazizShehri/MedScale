//! Wave 3 conformance & adversarial safety tests for `medscale_fhir::interop_v4`.

use medscale_fhir::interop_v4::{
    InteropErrorKind, audit_smart_scopes, check_external_bridge_license,
    check_us_core_and_bundle_integrity, fhir_deidentify_resource, is_cda_document,
    parse_smart_scope, project_fhir_bundle_to_omop_v54, redact_cda_xml, redact_hl7v2,
    redact_x12_edi,
};
use serde_json::{Value, json};

fn synth_scrub(text: &str) -> String {
    text.replace("Jane Example", "[REDACTED] [REDACTED]")
        .replace("Jane", "[REDACTED]")
        .replace("Example", "[REDACTED]")
        .replace("jane@example.org", "[EMAIL]")
}

fn vec_json() -> Value {
    let raw = include_str!("vectors/wave3_interop_v4.json");
    serde_json::from_str(raw).expect("wave3 interop JSON vector parses")
}

#[test]
fn conformance_fhir_deid_integrity_smart_hl7_cda_x12_and_omop() {
    let v = vec_json();

    let bundle = json!({
        "resourceType": "Bundle",
        "type": "collection",
        "entry": [
            {
                "fullUrl": "urn:uuid:11111111-1111-4111-8111-111111111111",
                "resource": {
                    "resourceType": "Patient",
                    "id": "pat-1",
                    "gender": "female",
                    "birthDate": "1930-01-02",
                    "name": [{"family": "Example", "given": ["Jane"]}],
                    "identifier": [{"system": "urn:example:mrn", "value": "MRN-998877"}]
                }
            },
            {
                "fullUrl": "urn:uuid:22222222-2222-4222-8222-222222222222",
                "resource": {
                    "resourceType": "Observation",
                    "id": "obs-1",
                    "status": "final",
                    "code": {"coding": [{"system": "http://loinc.org", "code": "2345-7"}]},
                    "effectiveDateTime": "2026-11-28",
                    "valueQuantity": {"value": 95.0, "unit": "mg/dL"},
                    "subject": {"reference": "urn:uuid:11111111-1111-4111-8111-111111111111"},
                    "note": [{"text": "Contact Jane Example at jane@example.org"}]
                }
            },
            {
                "fullUrl": "urn:uuid:33333333-3333-4333-8333-333333333333",
                "resource": {
                    "resourceType": "DocumentReference",
                    "id": "doc-1",
                    "description": "Discharge summary for Jane Example"
                }
            }
        ]
    });

    let deid = fhir_deidentify_resource(&bundle, 14, &synth_scrub).expect("fhir de-identify");
    let dumped = serde_json::to_string(&deid.resource).unwrap();
    assert!(!dumped.contains("Jane"));
    assert!(!dumped.contains("MRN-998877"));
    assert!(dumped.contains("2345-7"));
    assert_eq!(deid.resource["resourceType"], "Bundle");

    let int_ok = check_us_core_and_bundle_integrity(&bundle);
    assert_eq!(
        int_ok.valid,
        v["fhir_reference_integrity"]["valid_is_valid"]
            .as_bool()
            .unwrap()
    );
    let broken = json!({
        "resourceType": "Bundle",
        "type": "collection",
        "entry": [{
            "resource": {
                "resourceType": "Observation",
                "id": "obs-bad",
                "status": "final",
                "code": {"coding": [{"system": "http://loinc.org", "code": "2345-7"}]},
                "subject": {"reference": "Patient/missing-pat"}
            }
        }]
    });
    let int_bad = check_us_core_and_bundle_integrity(&broken);
    assert_eq!(
        int_bad.valid,
        v["fhir_reference_integrity"]["broken_is_valid"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        int_bad.dangling_paths.len(),
        v["fhir_reference_integrity"]["broken_finding_count"]
            .as_u64()
            .unwrap() as usize
    );

    // SMART scope grammar + audit
    let parsed_v1 = parse_smart_scope("patient/Patient.read").expect("smart v1 read");
    assert_eq!(parsed_v1.operations, "rs");
    let ok_rep = audit_smart_scopes(
        "patient_summary",
        &[
            "patient/Patient.rs".to_owned(),
            "patient/Observation.rs".to_owned(),
        ],
        &[
            "patient/Patient.rs".to_owned(),
            "patient/Observation.rs".to_owned(),
        ],
    );
    assert_eq!(
        ok_rep.status,
        v["smart_scopes"]["ok_status"].as_str().unwrap()
    );

    let ex_rep = audit_smart_scopes(
        "patient_summary",
        &["patient/Patient.r".to_owned()],
        &["patient/Patient.r".to_owned(), "system/*.cruds".to_owned()],
    );
    assert_eq!(
        ex_rep.status,
        v["smart_scopes"]["excess_status"].as_str().unwrap()
    );
    assert_eq!(
        ex_rep.excessive_scopes.len(),
        v["smart_scopes"]["excess_count"].as_u64().unwrap() as usize
    );

    // HL7v2
    let hl7_msg = "MSH|^~\\&|SEND|FAC|RECV|FAC|202611281200||ORU^R01|MSG001|P|2.5\r\
        PID|1||MRN12345^^^FAC^MR||Example^Jane||19800102|F|||123 Main St^^Riyadh^^11564||555-0199\r\
        OBR|1||ORD1|2345-7^Glucose\r\
        OBX|1|NM|2345-7^Glucose||95|mg/dL\r\
        NTE|1||Call Jane Example at jane@example.org\r";
    let hl7_rep = redact_hl7v2(hl7_msg, 7, &synth_scrub).expect("hl7v2");
    assert!(!hl7_rep.redacted_message.contains("Example^Jane"));
    assert!(hl7_rep.redacted_message.contains("19800109"));
    assert!(hl7_rep.redacted_message.contains("[EMAIL]"));
    assert_eq!(
        hl7_rep.segment_names,
        vec!["MSH", "PID", "OBR", "OBX", "NTE"]
    );

    // C-CDA XML
    let cda_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <ClinicalDocument xmlns=\"urn:hl7-org:v3\">\
        <recordTarget><patientRole><id root=\"2.16.840.1.113883.4.1\" extension=\"1098765439\"/>\
        <patient><name><given>Jane</given><family>Example</family></name><birthTime value=\"19800102\"/></patient>\
        </patientRole></recordTarget><component><structuredBody><component><section>\
        <text>Synthetic note for Jane Example</text>\
        </section></component></structuredBody></component></ClinicalDocument>";
    assert!(is_cda_document(cda_xml));
    let cda_rep = redact_cda_xml(cda_xml, 7, &synth_scrub).expect("cda");
    assert!(!cda_rep.redacted_xml.contains("Jane"));
    assert!(cda_rep.redacted_xml.contains("19800109"));

    // X12 837
    let x12_msg = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *261128*1200*^*00501*000000001*0*P*:~\
        GS*HC*SENDER*RECEIVER*20261128*1200*1*X*005010X222A1~\
        ST*837*0001*005010X222A1~\
        NM1*IL*1*Example*Jane****MI*MEM998877~\
        N3*123 Main St~N4*Riyadh*RI*11564~DMG*D8*19800102*F~\
        SE*6*0001~GE*1*1~IEA*1*000000001~";
    let x12_rep = redact_x12_edi(x12_msg).expect("x12 837");
    assert!(!x12_rep.deidentified_text.contains("MEM998877"));
    assert!(x12_rep.redaction_count >= 8);

    // OMOP CDM v5.4 projection w/ Safe Harbor age 90+ cap
    let omop = project_fhir_bundle_to_omop_v54(&bundle, 14, 2026, &synth_scrub).expect("omop");
    assert_eq!(omop.cdm_version, "5.4");
    assert_eq!(omop.person[0]["year_of_birth"], 1936); // 2026 - 1930 = 96 >= 90 -> capped to 2026 - 90 = 1936
    assert!(omop.person[0]["month_of_birth"].is_null());
    assert!(!omop.note[0]["note_text"].as_str().unwrap().contains("Jane"));
}

#[test]
fn external_bridge_license_quarantines_gpl_and_elastic() {
    let e1 = check_external_bridge_license("sdcMicro").expect_err("gpl bridge must refuse");
    assert_eq!(e1.kind, InteropErrorKind::BlockedExternalRights);
    let e2 = check_external_bridge_license("MedCAT").expect_err("elastic bridge must refuse");
    assert_eq!(e2.kind, InteropErrorKind::BlockedExternalRights);
    check_external_bridge_license("medscale_native").expect("apache native permitted");
}

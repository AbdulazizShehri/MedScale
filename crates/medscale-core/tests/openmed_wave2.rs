use std::collections::BTreeMap;

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::openmed_wave2::{
    ClaimDto, ClinicalV4AnalyzeRequest, DoseCheckDto, DoseRangeDto, MedMentionDto,
    PrivacyV4DeidentifyRequest, PrivacyV4PresetsRequest, PrivacyV4RiskRequest,
    UnitsV4ConvertRequest,
};
use medscale_core::CliSession;
use serde_json::json;

#[test]
fn wave2_privacy_presets_deidentify_and_risk_roundtrip() {
    let mut session = CliSession::connect_wave2_studio("wave2-core-test").expect("session");

    let presets = session
        .privacy_v4_presets(PrivacyV4PresetsRequest { filter: None })
        .expect("presets");
    assert_eq!(presets.presets.len(), 25);
    assert!(presets.presets.iter().any(|p| p.name == "saudi_pdpl"));
    assert!(
        presets
            .presets
            .iter()
            .any(|p| p.name == "hipaa_safe_harbor")
    );

    let deid = session
        .privacy_v4_deidentify(PrivacyV4DeidentifyRequest {
            text: "المريض أحمد حضر إلى العيادة الطبية بتاريخ 2026-11-28، Diabetes mellitus، SSN 123-45-6789."
                .to_owned(),
            policy_name: Some("hipaa_safe_harbor".to_owned()),
            calendar: Some("gregorian".to_owned()),
            date_shift_days: Some(14),
            patient_key: None,
            shift_secret: None,
            max_shift_days: None,
            rtl_isolate: true,
            clinical_protect: true,
            safety_sweep: true,
        })
        .expect("deidentify");
    assert!(deid.receipt_verified);
    assert!(deid.rtl_isolated);
    assert_eq!(deid.base_direction, "rtl");
    assert!(deid.redacted_text.contains("2026-12-12"));
    assert!(deid.redacted_text.contains("Diabetes mellitus"));
    assert!(!deid.redacted_text.contains("123-45-6789"));

    let mut r1 = BTreeMap::new();
    r1.insert("age".to_owned(), json!("30-39"));
    r1.insert("zip".to_owned(), json!("100**"));
    r1.insert("dx".to_owned(), json!("asthma"));
    let mut r2 = BTreeMap::new();
    r2.insert("age".to_owned(), json!("30-39"));
    r2.insert("zip".to_owned(), json!("100**"));
    r2.insert("dx".to_owned(), json!("diabetes"));
    let mut r3 = BTreeMap::new();
    r3.insert("age".to_owned(), json!("40-49"));
    r3.insert("zip".to_owned(), json!("200**"));
    r3.insert("dx".to_owned(), json!("asthma"));

    let risk = session
        .privacy_v4_risk(PrivacyV4RiskRequest {
            records: vec![r1, r2, r3],
            quasi_identifiers: vec!["age".to_owned(), "zip".to_owned()],
            sensitive_attributes: vec!["dx".to_owned()],
        })
        .expect("risk");
    assert_eq!(risk.record_count, 3);
    assert_eq!(risk.k_min, 1);
    assert_eq!(risk.singleton_count, 1);
    assert_eq!(risk.l_diversity.get("dx"), Some(&1));
}

#[test]
fn wave2_clinical_nlp_ucum_and_auth_boundaries() {
    let mut session = CliSession::connect_wave2_studio("wave2-core-test-2").expect("session");

    let analysis = session
        .clinical_v4_analyze(ClinicalV4AnalyzeRequest {
            text: "Assessment:\nDenies chest pain.\nMedications:\nMetformin 500 mg tablet by mouth twice daily for 30 days\nSocial History:\nUnemployed and lives alone.\n"
                .to_owned(),
            specialty: None,
            sig_lines: Vec::new(),
            med_mentions: vec![
                MedMentionDto {
                    medication: "Metformin".to_owned(),
                    dose: Some("500 mg".to_owned()),
                    route: Some("oral".to_owned()),
                    status: Some("continued".to_owned()),
                    timestamp: Some("2026-01-01".to_owned()),
                },
                MedMentionDto {
                    medication: "Metformin".to_owned(),
                    dose: Some("1000 mg".to_owned()),
                    route: Some("oral".to_owned()),
                    status: Some("continued".to_owned()),
                    timestamp: Some("2026-02-01".to_owned()),
                },
            ],
            dose_checks: vec![DoseCheckDto {
                drug: "amoxicillin".to_owned(),
                route: "oral".to_owned(),
                value: 2.0,
                unit: "g".to_owned(),
            }],
            dose_ranges: vec![DoseRangeSpecDtoFixture::amox()],
            nli_scores: Some((0.75, 0.20, 0.05)),
            claims: vec![
                ClaimDto {
                    claim_id: "c1".to_owned(),
                    claim_key: "a1c".to_owned(),
                    value: "6.4%".to_owned(),
                },
                ClaimDto {
                    claim_id: "c2".to_owned(),
                    claim_key: "a1c".to_owned(),
                    value: "7.8%".to_owned(),
                },
            ],
        })
        .expect("clinical");
    assert!(!analysis.sections.is_empty());
    assert!(
        analysis
            .context_assertions
            .iter()
            .any(|c| c.text.contains("chest pain") && c.negation == "negated")
    );
    assert_eq!(analysis.parsed_sigs.len(), 1);
    assert_eq!(analysis.parsed_sigs[0].frequency_per_day, Some(2.0));
    assert!(analysis.reconciled_meds[0].has_conflict);
    assert_eq!(analysis.dose_advisories[0].status, "above_range");
    assert!(analysis.dose_advisories[0].requires_clinician_review);
    assert!(!analysis.dose_advisories[0].autonomous_decision);
    assert_eq!(analysis.nli_decision.unwrap().outcome, "abstain");
    assert_eq!(analysis.claim_conflict_count, 1);

    let units = session
        .units_v4_convert(UnitsV4ConvertRequest {
            value: Some(90.0),
            from_unit: "mg/dL".to_owned(),
            to_unit: Some("mmol/L".to_owned()),
            analyte: Some("glucose".to_owned()),
            molar_mass_g_per_mol: None,
        })
        .expect("units");
    let mag = units.conversion.unwrap()["magnitude"].as_f64().unwrap();
    assert!((mag - 4.99567).abs() < 0.01, "got {mag}");

    // Least-privilege separation: pack operator session must be denied Wave 2 ops,
    // and Wave 2 studio session must be denied pack admission/list ops.
    let mut pack_only = CliSession::connect_pack_operator("pack-only").expect("pack session");
    let denied = pack_only.privacy_v4_presets(PrivacyV4PresetsRequest { filter: None });
    assert!(matches!(denied, Err(AuthorityError::SessionDenied)));
    let denied_pack = session.packs_list();
    assert!(matches!(denied_pack, Err(AuthorityError::SessionDenied)));
}

struct DoseRangeSpecDtoFixture;
impl DoseRangeSpecDtoFixture {
    fn amox() -> DoseRangeDto {
        DoseRangeDto {
            drug: "amoxicillin".to_owned(),
            route: "oral".to_owned(),
            low: Some(250.0),
            high: Some(1500.0),
            unit: "mg".to_owned(),
        }
    }
}

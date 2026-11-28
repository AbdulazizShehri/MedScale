use std::collections::BTreeMap;

use medscale_clinical::clinical_nlp_v4::{
    ClaimInput, DoseCheckInput, DoseRangeSpec, MedicationMentionInput, SpanInput,
    assert_context_axes, check_dose_ranges, count_claim_conflicts, detect_negation_scopes,
    detect_sections, evaluate_nli_gate, extract_discharge_summary, extract_pathology_profile,
    extract_relations_and_events, extract_sdoh, parse_radiology_report, parse_sig,
    reconcile_medications, refine_experiencer,
};
use medscale_clinical::privacy_v4::{
    CalendarCode, DetectedSpanV4, build_compliance_receipt, calendar_to_ordinal,
    evaluate_table_risk, filter_clinical_protected_spans, get_policy_preset, list_policy_presets,
    normalize_arabic_indic_digits, ordinal_to_calendar, render_redacted_rtl, run_safety_sweep,
    shift_calendar_date, stable_date_shift_from_seed, stable_date_shift_offset,
    strip_bidi_controls, verify_compliance_receipt,
};
use serde_json::{Value, json};

#[test]
fn wave2_privacy_risk_matches_python_vectors() {
    let vec: Value =
        serde_json::from_str(include_str!("vectors/wave2_privacy_risk_v4.json")).expect("json");

    assert_eq!(
        normalize_arabic_indic_digits(
            "\u{0661}\u{0660}\u{0669}\u{0668}\u{0667}\u{0666}\u{0665}\u{0664}\u{0663}\u{0669}"
        ),
        vec["arabic_digit_norm"]["eastern"].as_str().unwrap()
    );
    assert_eq!(
        normalize_arabic_indic_digits(
            "\u{06F2}\u{06F0}\u{06F3}\u{06F4}\u{06F5}\u{06F6}\u{06F7}\u{06F8}\u{06F9}\u{06F7}"
        ),
        vec["arabic_digit_norm"]["extended"].as_str().unwrap()
    );

    for c in vec["calendars"].as_array().unwrap() {
        let code = CalendarCode::parse(c["calendar"].as_str().unwrap()).unwrap();
        let ordinal = c["ordinal"].as_i64().unwrap();
        let (ny, nm, nd) = ordinal_to_calendar(code, ordinal).unwrap();
        let exp = c["native"].as_array().unwrap();
        assert_eq!(
            (ny, nm, nd),
            (
                exp[0].as_i64().unwrap() as i32,
                exp[1].as_u64().unwrap() as u8,
                exp[2].as_u64().unwrap() as u8
            ),
            "calendar {} ordinal {ordinal}",
            code.as_str()
        );
        assert_eq!(calendar_to_ordinal(code, ny, nm, nd).unwrap(), ordinal);
        let (sy, sm, sd) = shift_calendar_date(code, ny, nm, nd, 14).unwrap();
        let exp_s = c["shifted_plus14_native"].as_array().unwrap();
        assert_eq!(
            (sy, sm, sd),
            (
                exp_s[0].as_i64().unwrap() as i32,
                exp_s[1].as_u64().unwrap() as u8,
                exp_s[2].as_u64().unwrap() as u8
            )
        );
    }

    for ds in vec["date_shifts"].as_array().unwrap() {
        let got = if ds["kind"] == "hmac" {
            stable_date_shift_offset(
                ds["patient_key"].as_str().unwrap().as_bytes(),
                ds["max_days"].as_i64().unwrap(),
                ds["secret"].as_str().unwrap().as_bytes(),
            )
            .unwrap()
        } else {
            stable_date_shift_from_seed(
                ds["seed"].as_i64().unwrap(),
                ds["max_days"].as_i64().unwrap(),
            )
            .unwrap()
        };
        assert_eq!(got, ds["offset"].as_i64().unwrap());
    }

    for rc in vec["rtl"].as_array().unwrap() {
        let spans: Vec<(usize, usize)> = rc["spans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["start"].as_u64().unwrap() as usize,
                    s["end"].as_u64().unwrap() as usize,
                )
            })
            .collect();
        let res = render_redacted_rtl(rc["text"].as_str().unwrap(), &spans);
        assert_eq!(res.text, rc["rendered_text"].as_str().unwrap());
        assert_eq!(res.base_direction, rc["base_direction"].as_str().unwrap());
        assert_eq!(res.isolated, rc["isolated"].as_bool().unwrap());
        assert_eq!(
            strip_bidi_controls(&res.text),
            rc["stripped"].as_str().unwrap()
        );
    }

    // Policy presets (24 OpenMed + Saudi PDPL = 25)
    assert_eq!(list_policy_presets().len(), 25);
    assert!(get_policy_preset("saudi_pdpl").is_some());
    assert!(get_policy_preset("hipaa_safe_harbor").is_some());

    // Clinical protect
    let p_text = vec["clinical_protect"]["text"].as_str().unwrap();
    let prot_in = vec![
        DetectedSpanV4 {
            label: "PERSON".to_owned(),
            start: p_text.find("Parkinson").unwrap(),
            end: p_text.find("Parkinson").unwrap() + "Parkinson".len(),
            text: "Parkinson".to_owned(),
            source: "model".to_owned(),
        },
        DetectedSpanV4 {
            label: "PERSON".to_owned(),
            start: p_text.find("Jane Roe").unwrap(),
            end: p_text.find("Jane Roe").unwrap() + "Jane Roe".len(),
            text: "Jane Roe".to_owned(),
            source: "model".to_owned(),
        },
        DetectedSpanV4 {
            label: "PERSON".to_owned(),
            start: p_text.find("metformin").unwrap(),
            end: p_text.find("metformin").unwrap() + "metformin".len(),
            text: "metformin".to_owned(),
            source: "model".to_owned(),
        },
    ];
    let prot = filter_clinical_protected_spans(&prot_in, p_text);
    assert_eq!(prot.retained.len(), 1);
    assert_eq!(prot.retained[0].text, "Jane Roe");

    // Safety sweep
    let sw_text = vec["safety_sweep"]["text"].as_str().unwrap();
    let sw = run_safety_sweep(sw_text, &[]);
    assert_eq!(
        sw.len(),
        vec["safety_sweep"]["hits"].as_array().unwrap().len()
    );

    // k-anonymity & l-diversity & t-closeness
    let recs: Vec<BTreeMap<String, Value>> = vec![
        BTreeMap::from([
            ("age_band".into(), json!("40-49")),
            ("zip3".into(), json!("123")),
            ("sex".into(), json!("F")),
            ("dx".into(), json!("asthma")),
        ]),
        BTreeMap::from([
            ("age_band".into(), json!("40-49")),
            ("zip3".into(), json!("123")),
            ("sex".into(), json!("F")),
            ("dx".into(), json!("diabetes")),
        ]),
        BTreeMap::from([
            ("age_band".into(), json!("50-59")),
            ("zip3".into(), json!("456")),
            ("sex".into(), json!("M")),
            ("dx".into(), json!("hypertension")),
        ]),
        BTreeMap::from([
            ("age_band".into(), json!("50-59")),
            ("zip3".into(), json!("456")),
            ("sex".into(), json!("M")),
            ("dx".into(), json!("hypertension")),
        ]),
    ];
    let risk = evaluate_table_risk(
        &recs,
        &["age_band".into(), "zip3".into(), "sex".into()],
        &["dx".into()],
    );
    assert_eq!(risk.k_min, 2);
    assert_eq!(risk.l_diversity.get("dx"), Some(&1));
    assert!((risk.t_closeness.get("dx").copied().unwrap() - 0.5).abs() < 1e-9);

    // Compliance receipt tamper check
    let mut rec =
        build_compliance_receipt("saudi_pdpl", "sha256:00", "in", "out", "rtl", true, 2, 1, 3);
    assert!(verify_compliance_receipt(&rec));
    rec.total_redacted_spans = 999;
    assert!(!verify_compliance_receipt(&rec));
}

#[test]
fn wave2_clinical_nlp_matches_python_vectors() {
    let vec: Value =
        serde_json::from_str(include_str!("vectors/wave2_clinical_nlp_v4.json")).expect("json");

    let note = vec["sections"]["note"].as_str().unwrap();
    let secs = detect_sections(note);
    let exp_secs = vec["sections"]["detected"].as_array().unwrap();
    assert_eq!(secs.len(), exp_secs.len());
    for (g, e) in secs.iter().zip(exp_secs.iter()) {
        assert_eq!(g.label, e["label"].as_str().unwrap());
        assert_eq!(g.loinc_code.as_deref(), e["loinc_code"].as_str());
    }

    let neg_t = vec["negation_scope"]["text"].as_str().unwrap();
    let cp_start = neg_t.find("chest pain").unwrap();
    let fv_start = neg_t.find("fever").unwrap();
    let scopes = detect_negation_scopes(
        neg_t,
        &[
            SpanInput {
                start: cp_start,
                end: cp_start + 10,
                text: "chest pain".into(),
            },
            SpanInput {
                start: fv_start,
                end: fv_start + 5,
                text: "fever".into(),
            },
        ],
    );
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].governed, vec![(cp_start, cp_start + 10)]);

    let exp_t = vec["experiencer"]["text"].as_str().unwrap();
    let d_start = exp_t.find("type 2 diabetes").unwrap();
    let h_start = exp_t.find("hypertension").unwrap();
    let exps = refine_experiencer(
        exp_t,
        &[
            SpanInput {
                start: d_start,
                end: d_start + 15,
                text: "type 2 diabetes".into(),
            },
            SpanInput {
                start: h_start,
                end: h_start + 12,
                text: "hypertension".into(),
            },
        ],
    );
    assert_eq!(exps[0].experiencer, "family");
    assert_eq!(exps[1].experiencer, "patient");

    let ctx = assert_context_axes(
        "History of asthma; rule out pulmonary embolism.",
        &[
            SpanInput {
                start: 11,
                end: 17,
                text: "asthma".into(),
            },
            SpanInput {
                start: 28,
                end: 46,
                text: "pulmonary embolism".into(),
            },
        ],
    );
    assert_eq!(ctx[0].temporality, "historical");
    assert_eq!(ctx[1].certainty, "uncertain");

    for s in vec["sigs"].as_array().unwrap() {
        let got = parse_sig(s["raw"].as_str().unwrap());
        assert_eq!(got.dose, s["dose"].as_f64());
        assert_eq!(got.route.as_deref(), s["route"].as_str());
        assert_eq!(got.frequency_per_day, s["frequency_per_day"].as_f64());
        assert_eq!(got.as_needed, s["as_needed"].as_bool().unwrap());
        assert_eq!(
            got.duration_days.map(u64::from),
            s["duration_days"].as_u64()
        );
    }

    let medrec = reconcile_medications(&[
        MedicationMentionInput {
            medication: "metformin".into(),
            dose: Some("500 mg".into()),
            route: None,
            status: Some("started".into()),
            timestamp: Some("2026-10-01".into()),
        },
        MedicationMentionInput {
            medication: "metformin".into(),
            dose: Some("1000 mg".into()),
            route: None,
            status: Some("changed".into()),
            timestamp: Some("2026-11-01".into()),
        },
        MedicationMentionInput {
            medication: "lisinopril".into(),
            dose: Some("10 mg".into()),
            route: None,
            status: Some("continued".into()),
            timestamp: Some("2026-11-01".into()),
        },
        MedicationMentionInput {
            medication: "aspirin".into(),
            dose: Some("81 mg".into()),
            route: None,
            status: Some("stopped".into()),
            timestamp: Some("2026-11-01".into()),
        },
    ]);
    assert_eq!(medrec.len(), 3);
    assert_eq!(medrec[0].current_status, "changed");
    assert_eq!(medrec[1].current_status, "continued");
    assert_eq!(medrec[2].current_status, "stopped");

    let dosing = check_dose_ranges(
        &[DoseCheckInput {
            drug: "metformin".into(),
            route: "oral".into(),
            value: 2500.0,
            unit: "mg".into(),
        }],
        &[DoseRangeSpec {
            drug: "metformin".into(),
            route: "oral".into(),
            low: Some(500.0),
            high: Some(2000.0),
            unit: "mg".into(),
        }],
    );
    assert_eq!(dosing[0].status, "above_range");
    assert!(dosing[0].requires_clinician_review);
    assert!(!dosing[0].autonomous_decision);

    let sd = extract_sdoh(vec["sdoh"]["text"].as_str().unwrap());
    assert_eq!(sd.len(), 3);

    let rad = parse_radiology_report(
        "EXAM: Chest X-Ray\nINDICATION: Cough.\nFINDINGS: Lungs are clear without consolidation.\nIMPRESSION: Normal chest radiograph.",
    );
    assert_eq!(
        rad.impression_text,
        vec["radiology"]["impression_text"].as_str().unwrap()
    );

    let path = extract_pathology_profile(
        "SPECIMEN: Colon polyp\nDIAGNOSIS: Tubular adenoma, negative for dysplasia. Margins negative.",
    );
    assert_eq!(path.specimens.len(), 1);
    assert_eq!(
        path.diagnoses[0],
        vec["pathology"]["diagnosis_text"].as_str().unwrap()
    );

    let disch = extract_discharge_summary(
        "DISCHARGE DIAGNOSIS:\nCommunity-acquired pneumonia.\nDISCHARGE MEDICATIONS:\nAmoxicillin 500 mg PO TID x7 days.",
    );
    assert_eq!(disch.discharge_diagnoses.len(), 1);
    assert_eq!(disch.discharge_medications[0].dose, Some(500.0));

    let (rels, evs) = extract_relations_and_events("Amoxicillin 500 mg PO TID x7 days.");
    assert_eq!(rels.len(), 2);
    assert_eq!(evs.len(), 1);

    let nli = evaluate_nli_gate(0.91, 0.04, 0.05);
    assert_eq!(nli.outcome, vec["nli"]["outcome"].as_str().unwrap());

    let c_count = count_claim_conflicts(&[
        ClaimInput {
            claim_id: "1".into(),
            claim_key: "smoking_status".into(),
            value: "never".into(),
        },
        ClaimInput {
            claim_id: "2".into(),
            claim_key: "smoking_status".into(),
            value: "former".into(),
        },
    ]);
    assert_eq!(c_count, 1);
}

//! Authority execution handlers for OpenMed v4.0.0 Wave 2 (privacy presets,
//! calendar/RTL de-identification, table risk metrics, clinical NLP & UCUM v4).

use std::collections::{BTreeMap, BTreeSet};

use medscale_clinical::clinical_nlp_v4::{
    CLINICIAN_REVIEW_DISCLAIMER, ClaimInput, DoseCheckInput, DoseRangeSpec, MedicationMentionInput,
    SHAC_NON_BUNDLING_NOTICE, SpanInput, assert_context_axes, check_dose_ranges,
    count_claim_conflicts, detect_negation_scopes, detect_sections, evaluate_nli_gate,
    extract_discharge_summary, extract_pathology_profile, extract_relations_and_events,
    extract_sdoh, parse_radiology_report, parse_sig, reconcile_medications,
};
use medscale_clinical::privacy_v4::{
    CalendarCode, DetectedSpanV4, base_direction, build_compliance_receipt, evaluate_table_risk,
    filter_clinical_protected_spans, get_policy_preset, list_policy_presets,
    normalize_arabic_indic_digits, render_redacted_rtl, run_safety_sweep, shift_calendar_date,
    stable_date_shift_offset, verify_compliance_receipt,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::openmed_wave2::{
    ClinicalV4AnalyzeRequest, ClinicalV4AnalyzeResponse, ContextAssertionDto, DoseAdvisoryDto,
    EventDto, NegationScopeDto, NliDecisionDto, ParsedSigDto, PrivacyComplianceReceiptDto,
    PrivacyPresetDto, PrivacySpanDto, PrivacyV4DeidentifyRequest, PrivacyV4DeidentifyResponse,
    PrivacyV4PresetsRequest, PrivacyV4PresetsResponse, PrivacyV4RiskRequest, PrivacyV4RiskResponse,
    ReconciledMedDto, RelationDto, SdohDto, SectionDto, ShiftedDateDto, UnitsV4ConvertRequest,
    UnitsV4ConvertResponse,
};
use medscale_contracts::privacy_gate::SensitiveSpanKind;
use medscale_fhir::{
    UcumV4Status, convert_analyte_concentration, normalize_ucum_v4, parse_ucum_v4,
};
use serde_json::json;

use super::privacy_recognizers;

const MAX_WAVE2_TEXT_BYTES: usize = 512 * 1024;
const MAX_WAVE2_RECORDS: usize = 10_000;

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

fn span_kind_label(kind: SensitiveSpanKind) -> &'static str {
    match kind {
        SensitiveSpanKind::PersonName => "PERSON",
        SensitiveSpanKind::Email => "EMAIL",
        SensitiveSpanKind::Url => "URL",
        SensitiveSpanKind::IpAddress => "IP_ADDRESS",
        SensitiveSpanKind::Date => "DATE",
        SensitiveSpanKind::Phone => "PHONE",
        SensitiveSpanKind::Identifier => "ID_NUM",
        SensitiveSpanKind::PostalCode => "ZIPCODE",
        SensitiveSpanKind::PostalAddress => "STREET_ADDRESS",
        SensitiveSpanKind::ModelEntity => "MODEL_ENTITY",
    }
}

fn parse_iso_ymd(raw: &str) -> Option<(i32, u8, u8)> {
    let norm = normalize_arabic_indic_digits(raw.trim());
    let mut parts = norm.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u8 = parts.next()?.parse().ok()?;
    let d: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn format_date_shift(
    raw: &str,
    cal: CalendarCode,
    offset_days: i64,
) -> Option<(String, ShiftedDateDto)> {
    let (y, m, d) = parse_iso_ymd(raw)?;
    let (sy, sm, sd) = shift_calendar_date(cal, y, m, d, offset_days).ok()?;
    let formatted = format!("{sy:04}-{sm:02}-{sd:02}");
    Some((
        formatted.clone(),
        ShiftedDateDto {
            original: raw.to_owned(),
            shifted: formatted,
            calendar: cal.as_str().to_owned(),
            offset_days,
        },
    ))
}

/// Deterministic calendar-aware date shift (`gregorian`, `hijri`, `solar_hijri`, `bikram_sambat`)
/// via explicit `days` or HMAC(`patient_key`, `shift_secret`).
pub fn shift_calendar_date_v4(
    calendar: &str,
    date_ymd: &str,
    days: Option<i64>,
    patient_key: Option<&str>,
    shift_secret: Option<&str>,
    max_days: Option<i64>,
) -> Result<ShiftedDateDto, AuthorityError> {
    let cal = CalendarCode::parse(calendar).map_err(invalid)?;
    let offset = if let Some(d) = days {
        d
    } else if let (Some(key), Some(secret)) = (patient_key, shift_secret) {
        stable_date_shift_offset(key.as_bytes(), max_days.unwrap_or(365), secret.as_bytes())
            .map_err(invalid)?
    } else {
        return Err(invalid(
            "provide --days or both --patient-key and --shift-secret",
        ));
    };
    let (y, m, d) = parse_iso_ymd(date_ymd).ok_or_else(|| invalid("invalid YYYY-MM-DD date"))?;
    let (sy, sm, sd) = shift_calendar_date(cal, y, m, d, offset).map_err(invalid)?;
    Ok(ShiftedDateDto {
        original: date_ymd.to_owned(),
        shifted: format!("{sy:04}-{sm:02}-{sd:02}"),
        calendar: cal.as_str().to_owned(),
        offset_days: offset,
    })
}

pub fn handle_presets(
    req: &PrivacyV4PresetsRequest,
) -> Result<PrivacyV4PresetsResponse, AuthorityError> {
    let filter = req.filter.as_deref().map(str::trim).unwrap_or("");
    let needle = filter.to_lowercase();
    let presets = list_policy_presets()
        .iter()
        .filter(|p| {
            needle.is_empty()
                || p.name.to_lowercase().contains(&needle)
                || p.posture.to_lowercase().contains(&needle)
        })
        .map(|p| PrivacyPresetDto {
            name: p.name.clone(),
            schema_version: p.schema_version,
            posture: p.posture.clone(),
            default_action: p.default_action.clone(),
            strict_no_leak: p.strict_no_leak,
            safety_sweep_mandatory: p.safety_sweep_mandatory,
            keep_mapping: p.keep_mapping,
            reversible_id: p.reversible_id,
            actions: p.actions.clone(),
            sha256: p.sha256.clone(),
        })
        .collect();
    Ok(PrivacyV4PresetsResponse { presets })
}

pub fn handle_deidentify(
    req: &PrivacyV4DeidentifyRequest,
) -> Result<PrivacyV4DeidentifyResponse, AuthorityError> {
    if req.text.len() > MAX_WAVE2_TEXT_BYTES {
        return Err(invalid(format!(
            "text exceeds maximum {MAX_WAVE2_TEXT_BYTES} bytes"
        )));
    }
    let policy_id = req.policy_name.as_deref().unwrap_or("hipaa_safe_harbor");
    let preset = get_policy_preset(policy_id)
        .ok_or_else(|| invalid(format!("unknown privacy policy preset: {policy_id}")))?;
    let calendar = match req.calendar.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(raw) => CalendarCode::parse(raw).map_err(invalid)?,
        None => CalendarCode::Gregorian,
    };
    let date_offset_days = if let Some(days) = req.date_shift_days {
        Some(days)
    } else if let (Some(key), Some(secret)) = (&req.patient_key, &req.shift_secret) {
        let max_days = req.max_shift_days.unwrap_or(365);
        Some(
            stable_date_shift_offset(key.as_bytes(), max_days, secret.as_bytes())
                .map_err(invalid)?,
        )
    } else {
        None
    };

    // Map UTF-8 byte offsets from Spec 079 recognizers to Unicode scalar indices used by privacy_v4.
    let mut byte_to_char: BTreeMap<usize, usize> = req
        .text
        .char_indices()
        .enumerate()
        .map(|(c, (b, _))| (b, c))
        .collect();
    let total_chars = req.text.chars().count();
    byte_to_char.insert(req.text.len(), total_chars);

    let pattern_spans = privacy_recognizers::pattern_recognize(&req.text);
    let mut initial: Vec<DetectedSpanV4> = Vec::with_capacity(pattern_spans.len());
    for sp in pattern_spans {
        if let (Some(&start), Some(&end)) = (byte_to_char.get(&sp.start), byte_to_char.get(&sp.end))
            && start < end
        {
            initial.push(DetectedSpanV4 {
                label: span_kind_label(sp.kind).to_owned(),
                start,
                end,
                text: req.text[sp.start..sp.end].to_owned(),
                source: sp.recognizer_id,
            });
        }
    }

    let (mut active_spans, suppressed_clinical_terms) = if req.clinical_protect {
        let outcome = filter_clinical_protected_spans(&initial, &req.text);
        (outcome.retained, outcome.suppressed_count)
    } else {
        (initial, 0)
    };

    let run_sweep = req.safety_sweep || preset.safety_sweep_mandatory;
    let sweep_added_spans = if run_sweep {
        let extra = run_safety_sweep(&req.text, &active_spans);
        let added = extra.len();
        active_spans.extend(extra);
        added
    } else {
        0
    };

    // Coalesce overlapping char ranges, keeping first-sorted / wider spans deterministically.
    active_spans.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    let mut non_overlapping: Vec<DetectedSpanV4> = Vec::with_capacity(active_spans.len());
    let mut max_end = 0usize;
    for sp in active_spans {
        if sp.start >= max_end && sp.start < sp.end && sp.end <= total_chars {
            max_end = sp.end;
            non_overlapping.push(sp);
        }
    }

    let orig_chars: Vec<char> = req.text.chars().collect();
    let mut assembled = String::with_capacity(req.text.len());
    let mut replaced_char_ranges: Vec<(usize, usize)> = Vec::new();
    let mut shifted_dates: Vec<ShiftedDateDto> = Vec::new();
    let mut cursor = 0usize;
    let mut out_chars_len = 0usize;
    let mut total_redacted_spans = 0usize;

    for sp in &non_overlapping {
        for &c in &orig_chars[cursor..sp.start] {
            assembled.push(c);
            out_chars_len += 1;
        }
        let action = preset
            .actions
            .get(&sp.label)
            .map(String::as_str)
            .unwrap_or(&preset.default_action);

        let replacement = if sp.label == "DATE" {
            if let Some(offset) = date_offset_days {
                if let Some((shifted, dto)) = format_date_shift(&sp.text, calendar, offset) {
                    shifted_dates.push(dto);
                    shifted
                } else if action == "keep" || action == "allow" {
                    sp.text.clone()
                } else if action == "mask" {
                    "***".to_owned()
                } else {
                    "[REDACTED:DATE]".to_owned()
                }
            } else if action == "keep" || action == "allow" {
                sp.text.clone()
            } else if action == "mask" {
                "***".to_owned()
            } else {
                "[REDACTED:DATE]".to_owned()
            }
        } else {
            match action {
                "keep" | "allow" => sp.text.clone(),
                "mask" => "***".to_owned(),
                _ => format!("[REDACTED:{}]", sp.label),
            }
        };

        let rep_chars = replacement.chars().count();
        if replacement != sp.text {
            replaced_char_ranges.push((out_chars_len, out_chars_len + rep_chars));
            total_redacted_spans += 1;
        }
        assembled.push_str(&replacement);
        out_chars_len += rep_chars;
        cursor = sp.end;
    }
    for &c in &orig_chars[cursor..] {
        assembled.push(c);
    }

    let (final_text, base_dir, rtl_isolated) = if req.rtl_isolate {
        let rtl = render_redacted_rtl(&assembled, &replaced_char_ranges);
        (rtl.text, rtl.base_direction, rtl.isolated)
    } else {
        (
            assembled.clone(),
            base_direction(&assembled).to_owned(),
            false,
        )
    };

    let receipt = build_compliance_receipt(
        &preset.name,
        &preset.sha256,
        &req.text,
        &final_text,
        &base_dir,
        rtl_isolated,
        suppressed_clinical_terms,
        sweep_added_spans,
        total_redacted_spans,
    );
    let receipt_verified = verify_compliance_receipt(&receipt);

    Ok(PrivacyV4DeidentifyResponse {
        redacted_text: final_text,
        policy_name: preset.name.clone(),
        policy_sha256: preset.sha256.clone(),
        base_direction: base_dir,
        rtl_isolated,
        detected_spans: non_overlapping
            .into_iter()
            .map(|s| PrivacySpanDto {
                label: s.label,
                start: s.start,
                end: s.end,
                text: s.text,
                source: s.source,
            })
            .collect(),
        suppressed_clinical_terms,
        sweep_added_spans,
        shifted_dates,
        receipt: PrivacyComplianceReceiptDto {
            schema_version: receipt.schema_version,
            policy_name: receipt.policy_name,
            policy_sha256: receipt.policy_sha256,
            input_sha256: receipt.input_sha256,
            output_sha256: receipt.output_sha256,
            base_direction: receipt.base_direction,
            rtl_isolated: receipt.rtl_isolated,
            suppressed_clinical_terms: receipt.suppressed_clinical_terms,
            sweep_added_spans: receipt.sweep_added_spans,
            total_redacted_spans: receipt.total_redacted_spans,
            receipt_sha256: receipt.receipt_sha256,
        },
        receipt_verified,
    })
}

pub fn handle_risk(req: &PrivacyV4RiskRequest) -> Result<PrivacyV4RiskResponse, AuthorityError> {
    if req.records.len() > MAX_WAVE2_RECORDS {
        return Err(invalid(format!(
            "record table exceeds maximum {MAX_WAVE2_RECORDS} rows"
        )));
    }
    if req.quasi_identifiers.is_empty() && !req.records.is_empty() {
        return Err(invalid("provide at least one quasi_identifier"));
    }
    let report = evaluate_table_risk(
        &req.records,
        &req.quasi_identifiers,
        &req.sensitive_attributes,
    );
    Ok(PrivacyV4RiskResponse {
        record_count: report.record_count,
        k_min: report.k_min,
        class_count: report.class_count,
        singleton_count: report.singleton_count,
        l_diversity: report.l_diversity,
        t_closeness: report.t_closeness,
        prosecutor_risk: report.prosecutor_risk,
        journalist_risk: report.journalist_risk,
        marketer_risk: report.marketer_risk,
    })
}

fn candidate_clause_spans(
    text: &str,
    headers: &[medscale_clinical::clinical_nlp_v4::ClinicalSectionSpan],
) -> Vec<SpanInput> {
    const PREFIXES: &[&str] = &[
        "patient ",
        "denies ",
        "denied ",
        "no evidence of ",
        "negative for ",
        "without ",
        "ruled out ",
        "rule out ",
        "family history of ",
        "history of ",
        "hx of ",
        "possible ",
        "probable ",
        "suspected ",
        "r/o ",
        "no ",
    ];
    let header_starts: BTreeSet<usize> = headers.iter().map(|h| h.start).collect();
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < chars.len() {
        let mut line_end = idx;
        while line_end < chars.len() && chars[line_end] != '\n' {
            line_end += 1;
        }
        let mut s = idx;
        while s < line_end && chars[s].is_whitespace() {
            s += 1;
        }
        let mut e = line_end;
        while e > s && chars[e - 1].is_whitespace() {
            e -= 1;
        }
        if s < e && !header_starts.contains(&s) {
            let mut sub_start = s;
            for pos in s..=e {
                let at_delim = pos == e || matches!(chars[pos], ',' | ';');
                if at_delim {
                    let mut cs = sub_start;
                    let mut ce = pos;
                    while cs < ce && chars[cs].is_whitespace() {
                        cs += 1;
                    }
                    while ce > cs
                        && (chars[ce - 1].is_whitespace()
                            || matches!(chars[ce - 1], '.' | ',' | ';' | ':' | '!' | '?'))
                    {
                        ce -= 1;
                    }
                    loop {
                        let sub: String = chars[cs..ce].iter().collect();
                        let lower = sub.to_lowercase();
                        let mut stripped = false;
                        for pref in PREFIXES {
                            if lower.starts_with(pref) {
                                let plen = pref.chars().count();
                                cs = (cs + plen).min(ce);
                                while cs < ce && chars[cs].is_whitespace() {
                                    cs += 1;
                                }
                                stripped = true;
                                break;
                            }
                        }
                        if !stripped {
                            break;
                        }
                    }
                    if cs < ce {
                        out.push(SpanInput {
                            start: cs,
                            end: ce,
                            text: chars[cs..ce].iter().collect(),
                        });
                    }
                    sub_start = pos + 1;
                }
            }
        }
        idx = line_end + 1;
    }
    out
}

pub fn handle_clinical_analyze(
    req: &ClinicalV4AnalyzeRequest,
) -> Result<ClinicalV4AnalyzeResponse, AuthorityError> {
    if req.text.len() > MAX_WAVE2_TEXT_BYTES {
        return Err(invalid(format!(
            "clinical note exceeds maximum {MAX_WAVE2_TEXT_BYTES} bytes"
        )));
    }
    let sections = detect_sections(&req.text);
    let spans = candidate_clause_spans(&req.text, &sections);
    let neg_scopes = detect_negation_scopes(&req.text, &spans);
    let context_assertions = assert_context_axes(&req.text, &spans);

    let sig_inputs: Vec<String> = if req.sig_lines.is_empty() {
        let mut derived = Vec::new();
        for sp in &spans {
            let p = parse_sig(&sp.text);
            if p.dose.is_some() || p.route.is_some() || p.frequency_per_day.is_some() {
                derived.push(sp.text.clone());
            }
        }
        derived
    } else {
        req.sig_lines.clone()
    };
    let parsed_sigs = sig_inputs
        .iter()
        .map(|s| {
            let p = parse_sig(s);
            ParsedSigDto {
                raw: p.raw,
                dose: p.dose,
                unit: p.unit,
                form: p.form,
                route: p.route,
                frequency_per_day: p.frequency_per_day,
                frequency_period: p.frequency_period,
                frequency_period_unit: p.frequency_period_unit,
                as_needed: p.as_needed,
                condition: p.condition,
                duration_days: p.duration_days,
                missing: p.missing,
            }
        })
        .collect();

    let mentions: Vec<MedicationMentionInput> = req
        .med_mentions
        .iter()
        .map(|m| MedicationMentionInput {
            medication: m.medication.clone(),
            dose: m.dose.clone(),
            route: m.route.clone(),
            status: m.status.clone(),
            timestamp: m.timestamp.clone(),
        })
        .collect();
    let reconciled_meds = reconcile_medications(&mentions)
        .into_iter()
        .map(|r| ReconciledMedDto {
            ingredient: r.ingredient,
            current_status: r.current_status,
            current_dose: r.current_dose,
            current_route: r.current_route,
            history_len: r.history_len,
            has_conflict: r.has_conflict,
        })
        .collect();

    let dose_inputs: Vec<DoseCheckInput> = req
        .dose_checks
        .iter()
        .map(|d| DoseCheckInput {
            drug: d.drug.clone(),
            route: d.route.clone(),
            value: d.value,
            unit: d.unit.clone(),
        })
        .collect();
    let range_specs: Vec<DoseRangeSpec> = req
        .dose_ranges
        .iter()
        .map(|r| DoseRangeSpec {
            drug: r.drug.clone(),
            route: r.route.clone(),
            low: r.low,
            high: r.high,
            unit: r.unit.clone(),
        })
        .collect();
    let dose_advisories = check_dose_ranges(&dose_inputs, &range_specs)
        .into_iter()
        .map(|a| DoseAdvisoryDto {
            drug: a.drug,
            route: a.route,
            status: a.status,
            requires_clinician_review: a.requires_clinician_review,
            autonomous_decision: a.autonomous_decision,
            disclaimer: a.disclaimer,
        })
        .collect();

    let (relations, events) = extract_relations_and_events(&req.text);
    let sdoh = extract_sdoh(&req.text);

    let specialty_result = match req.specialty.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(raw) => match raw.trim().to_lowercase().as_str() {
            "radiology" => Some(json!(parse_radiology_report(&req.text))),
            "pathology" => Some(json!(extract_pathology_profile(&req.text))),
            "discharge" => Some(json!(extract_discharge_summary(&req.text))),
            other => return Err(invalid(format!("unsupported specialty profile: {other}"))),
        },
        None => None,
    };

    let nli_decision = req.nli_scores.map(|(e, c, n)| {
        let d = evaluate_nli_gate(e, c, n);
        NliDecisionDto {
            outcome: d.outcome,
            selected_probability: d.selected_probability,
            margin: d.margin,
            requires_human_review: d.requires_human_review,
        }
    });

    let claim_inputs: Vec<ClaimInput> = req
        .claims
        .iter()
        .map(|c| ClaimInput {
            claim_id: c.claim_id.clone(),
            claim_key: c.claim_key.clone(),
            value: c.value.clone(),
        })
        .collect();
    let claim_conflict_count = count_claim_conflicts(&claim_inputs);

    Ok(ClinicalV4AnalyzeResponse {
        disclaimer: CLINICIAN_REVIEW_DISCLAIMER.to_owned(),
        shac_notice: SHAC_NON_BUNDLING_NOTICE.to_owned(),
        sections: sections
            .into_iter()
            .map(|s| SectionDto {
                label: s.label,
                header: s.header,
                start: s.start,
                end: s.end,
                content_start: s.content_start,
                loinc_code: s.loinc_code,
            })
            .collect(),
        negation_scopes: neg_scopes
            .into_iter()
            .map(|n| NegationScopeDto {
                cue: n.cue,
                direction: n.direction,
                scope_start: n.scope_start,
                scope_end: n.scope_end,
                governed: n.governed,
            })
            .collect(),
        context_assertions: context_assertions
            .into_iter()
            .map(|c| ContextAssertionDto {
                text: c.text,
                start: c.start,
                end: c.end,
                negation: c.negation,
                temporality: c.temporality,
                certainty: c.certainty,
                experiencer: c.experiencer,
            })
            .collect(),
        parsed_sigs,
        reconciled_meds,
        dose_advisories,
        relations: relations
            .into_iter()
            .map(|r| RelationDto {
                relation_type: r.relation_type,
                source_text: r.source_text,
                target_text: r.target_text,
            })
            .collect(),
        events: events
            .into_iter()
            .map(|e| EventDto {
                event_type: e.event_type,
                subject: e.subject,
                temporality: e.temporality,
            })
            .collect(),
        sdoh: sdoh
            .into_iter()
            .map(|s| SdohDto {
                category: s.category,
                value: s.value,
                status: s.status,
                temporality: s.temporality,
            })
            .collect(),
        specialty_result,
        nli_decision,
        claim_conflict_count,
    })
}

pub fn handle_units_convert(
    req: &UnitsV4ConvertRequest,
) -> Result<UnitsV4ConvertResponse, AuthorityError> {
    if req.from_unit.trim().is_empty() || req.from_unit.len() > 128 {
        return Err(invalid("from_unit must be 1..=128 characters"));
    }
    let parsed = parse_ucum_v4(&req.from_unit);
    let conversion = match req.to_unit.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(target) => {
            if target.len() > 128 {
                return Err(invalid("to_unit must be <= 128 characters"));
            }
            let val = req.value.unwrap_or(1.0);
            if !val.is_finite() {
                return Err(invalid("value must be finite"));
            }
            if let Some(mm) = req.molar_mass_g_per_mol
                && (!mm.is_finite() || mm <= 0.0)
            {
                return Err(invalid("molar_mass_g_per_mol must be finite and positive"));
            }
            let res = if let Some(analyte) = req.analyte.as_deref().filter(|s| !s.trim().is_empty())
            {
                convert_analyte_concentration(
                    val,
                    &req.from_unit,
                    target,
                    analyte,
                    req.molar_mass_g_per_mol,
                )
            } else if let Some(mm) = req.molar_mass_g_per_mol {
                convert_analyte_concentration(val, &req.from_unit, target, "custom", Some(mm))
            } else {
                normalize_ucum_v4(val, &req.from_unit, target)
            };
            if res.status == UcumV4Status::Unknown {
                return Err(invalid(res.reason));
            }
            Some(serde_json::to_value(&res).map_err(|e| invalid(e.to_string()))?)
        }
        None => None,
    };
    Ok(UnitsV4ConvertResponse {
        parsed: serde_json::to_value(&parsed).map_err(|e| invalid(e.to_string()))?,
        conversion,
    })
}

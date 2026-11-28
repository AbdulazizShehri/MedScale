//! Authority execution handlers for OpenMed v4.0.0 Wave 3 (multimodal/DICOM,
//! healthcare interop, agent governance kernel, MCP SSRF/consent & DUA eval).

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;

use medscale_clinical::agent_eval_v4::{
    ActionNodeInput, ActionPhase, ApprovalToken, CalibrationPoint, EvalRelationItem, EvalSpanItem,
    McpConsentReceipt, PriorAuthEvidenceItem, PriorAuthRequirementSpec, TrialCriterionEval,
    build_trial_eligibility_review, check_restricted_terminology_bundle,
    check_split_overlap_leakage, compute_expected_calibration_error, default_mcp_tool_registry,
    evaluate_relation_f1, evaluate_span_f1, issue_approval_token, issue_consent_receipt,
    scan_prompt_injection, score_prior_auth_completeness, validate_action_graph,
    validate_action_transition, validate_dua_dataset_path, validate_upstream_endpoint,
    verify_and_consume_approval_token, verify_consent_receipt,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::openmed_wave2::PrivacyV4DeidentifyRequest;
use medscale_contracts::openmed_wave3::{
    AgentV4GovernRequest, AgentV4GovernResponse, DicomV4DeidentifyRequest,
    DicomV4DeidentifyResponse, EvalV4RunRequest, EvalV4RunResponse, InteropV4ProcessRequest,
    InteropV4ProcessResponse, McpV4PolicyCheckRequest, McpV4PolicyCheckResponse,
    MultimodalV4ProcessRequest, MultimodalV4ProcessResponse,
};
use medscale_fhir::interop_v4::{
    audit_smart_scopes, check_external_bridge_license, check_us_core_and_bundle_integrity,
    fhir_deidentify_resource, project_fhir_bundle_to_omop_v54, redact_cda_xml, redact_hl7v2,
    redact_x12_edi,
};
use medscale_pack::dicom_v4::{DicomDeidentifyOptions, PixelBlackoutRect, deidentify_dicom_part10};
use medscale_pack::multimodal_v4::{
    ArchiveMemberMeta, ArchiveSafetyPolicy, MAX_MULTIMODAL_INPUT_BYTES, evaluate_archive_members,
    extract_html_text, extract_markdown_text, extract_rtf_text, preflight_and_scrub_pdf,
    refuse_outlook_msg_gpl3, run_sanitized_local_tesseract, scrub_chatlog_jsonl,
    scrub_edf_ecg_header, scrub_eml, scrub_ical, scrub_jpeg_metadata, scrub_mp3_tags,
    scrub_ooxml_zip, scrub_png_metadata, scrub_rtf_info_and_text, scrub_subtitles_vtt_srt,
    scrub_tabular_csv, scrub_vcard, scrub_wav_metadata, sha256_prefixed,
};
use serde_json::json;

static APPROVAL_NONCES: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

const LOCAL_APPROVAL_KEY: &[u8] = b"medscale-wave3-local-approval-hmac-v1";
const LOCAL_MCP_CONSENT_KEY: &[u8] = b"medscale-wave3-local-mcp-consent-v1";

fn invalid(msg: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: msg.into(),
    }
}

fn core_text_scrubber<'a>(policy_name: Option<&'a str>) -> impl Fn(&str) -> String + 'a {
    move |input: &str| {
        if input.trim().is_empty() {
            return input.to_owned();
        }
        let req = PrivacyV4DeidentifyRequest {
            text: input.to_owned(),
            policy_name: policy_name.map(str::to_owned),
            calendar: None,
            date_shift_days: None,
            patient_key: None,
            shift_secret: None,
            max_shift_days: None,
            rtl_isolate: false,
            clinical_protect: true,
            safety_sweep: true,
        };
        super::openmed_wave2::handle_deidentify(&req)
            .map(|r| r.redacted_text)
            .unwrap_or_else(|_| input.to_owned())
    }
}

pub fn decode_base64(input: &str) -> Result<Vec<u8>, AuthorityError> {
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !clean.len().is_multiple_of(4) {
        return Err(invalid("invalid base64 payload length"));
    }
    if (clean.len() / 4) * 3 > MAX_MULTIMODAL_INPUT_BYTES {
        return Err(invalid("decoded payload exceeds 32 MiB limit"));
    }
    let decode_val = |b: u8| -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let total_chunks = clean.len() / 4;
    let mut out = Vec::with_capacity(total_chunks * 3);
    for (idx, chunk) in clean.chunks_exact(4).enumerate() {
        let is_last = idx + 1 == total_chunks;
        let a = decode_val(chunk[0]).ok_or_else(|| invalid("invalid base64 byte"))?;
        let b = decode_val(chunk[1]).ok_or_else(|| invalid("invalid base64 byte"))?;
        if chunk[2] == b'=' {
            if !is_last || chunk[3] != b'=' {
                return Err(invalid("invalid base64 padding"));
            }
            out.push((a << 2) | (b >> 4));
            break;
        }
        let c = decode_val(chunk[2]).ok_or_else(|| invalid("invalid base64 byte"))?;
        out.push((a << 2) | (b >> 4));
        out.push((b << 4) | (c >> 2));
        if chunk[3] == b'=' {
            if !is_last {
                return Err(invalid("invalid base64 padding"));
            }
            break;
        }
        let d = decode_val(chunk[3]).ok_or_else(|| invalid("invalid base64 byte"))?;
        out.push((c << 6) | d);
    }
    Ok(out)
}

#[must_use]
pub fn encode_base64(data: &[u8]) -> String {
    const TBL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        out.push(TBL[b0 >> 2] as char);
        out.push(TBL[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            out.push(TBL[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TBL[b2 & 0x3F] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn handle_multimodal_process(
    req: &MultimodalV4ProcessRequest,
) -> Result<MultimodalV4ProcessResponse, AuthorityError> {
    let fmt = req.format.trim().to_ascii_lowercase();
    let policy = req.policy_name.as_deref().or(Some("hipaa_safe_harbor"));
    let scrubber = core_text_scrubber(policy);
    let noop = |s: &str| s.to_owned();
    let scrub_fn: &dyn Fn(&str) -> String = if req.scrub { &scrubber } else { &noop };

    let raw_bytes = match req
        .base64_payload
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        Some(b64) => decode_base64(b64)?,
        None => req.text.as_deref().unwrap_or("").as_bytes().to_vec(),
    };
    if raw_bytes.len() > MAX_MULTIMODAL_INPUT_BYTES {
        return Err(invalid("input payload exceeds 32 MiB limit"));
    }

    match fmt.as_str() {
        "msg" => {
            refuse_outlook_msg_gpl3("input.msg", &raw_bytes).map_err(|e| invalid(e.message))?;
            unreachable!("msg format always refuses")
        }
        "html" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let extracted = extract_html_text(&txt);
            let scrubbed = scrub_fn(&extracted);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: extracted,
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: vec!["script_style_template".to_owned()],
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "markdown" | "asciidoc" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let extracted = extract_markdown_text(&txt);
            let scrubbed = scrub_fn(&extracted);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: extracted,
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: Vec::new(),
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "rtf" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let extracted = extract_rtf_text(&txt);
            let scrubbed = scrub_rtf_info_and_text(&txt, scrub_fn);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: extracted,
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: vec!["rtf.info".to_owned()],
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "eml" => {
            refuse_outlook_msg_gpl3("email.eml", &raw_bytes).map_err(|e| invalid(e.message))?;
            let txt = String::from_utf8_lossy(&raw_bytes);
            let res = scrub_eml(&txt, scrub_fn);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: res.body_text,
                output_sha256: sha256_prefixed(res.scrubbed_eml.as_bytes()),
                scrubbed_text: res.scrubbed_eml,
                output_base64: None,
                removed_segments: res.scrubbed_headers,
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "csv" | "tsv" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let res = scrub_tabular_csv(&txt, scrub_fn);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: txt.into_owned(),
                output_sha256: sha256_prefixed(res.scrubbed_csv.as_bytes()),
                scrubbed_text: res.scrubbed_csv,
                output_base64: None,
                removed_segments: res
                    .columns
                    .into_iter()
                    .filter(|c| c.action != "keep")
                    .map(|c| format!("{}:{}", c.name, c.action))
                    .collect(),
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "chatlog_jsonl" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let scrubbed = scrub_chatlog_jsonl(&txt, scrub_fn);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: txt.into_owned(),
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: vec!["speaker_pseudonym".to_owned()],
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "vcard" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let scrubbed = scrub_vcard(&txt);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: txt.into_owned(),
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: vec!["vcard_phi".to_owned()],
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "ical" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let scrubbed = scrub_ical(&txt, scrub_fn);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: txt.into_owned(),
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: vec!["ical_attendee".to_owned()],
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "vtt" | "srt" => {
            let txt = String::from_utf8_lossy(&raw_bytes);
            let scrubbed = scrub_subtitles_vtt_srt(&txt, scrub_fn);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: txt.into_owned(),
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: vec!["caption_speakers".to_owned()],
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "ooxml_zip" | "docx" | "xlsx" | "pptx" | "epub" => {
            let res = scrub_ooxml_zip(&raw_bytes, scrub_fn).map_err(|e| invalid(e.message))?;
            let clean_flag = res.post_scrub_findings.is_empty();
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: "processed".to_owned(),
                extracted_text: res.extracted_text.clone(),
                scrubbed_text: scrub_fn(&res.extracted_text),
                output_base64: Some(encode_base64(&res.clean_zip_bytes)),
                output_sha256: sha256_prefixed(&res.clean_zip_bytes),
                removed_segments: res
                    .residual_findings
                    .iter()
                    .map(|f| f.category.clone())
                    .collect(),
                residual_findings: res
                    .post_scrub_findings
                    .into_iter()
                    .map(|f| json!(f))
                    .collect(),
                residual_clean: clean_flag,
                archive_report: None,
            })
        }
        "pdf" => {
            let res =
                preflight_and_scrub_pdf(&raw_bytes, scrub_fn).map_err(|e| invalid(e.message))?;
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: res.status,
                extracted_text: res.extracted_text.clone(),
                scrubbed_text: scrub_fn(&res.extracted_text),
                output_base64: Some(encode_base64(&res.scrubbed_pdf_bytes)),
                output_sha256: sha256_prefixed(&res.scrubbed_pdf_bytes),
                removed_segments: res.reason_codes,
                residual_findings: Vec::new(),
                residual_clean: res.residual_clean,
                archive_report: None,
            })
        }
        "png" | "jpeg" | "wav" | "mp3" | "edf" => {
            let rep = match fmt.as_str() {
                "png" => scrub_png_metadata(&raw_bytes),
                "jpeg" => scrub_jpeg_metadata(&raw_bytes),
                "wav" => scrub_wav_metadata(&raw_bytes),
                "mp3" => Ok(scrub_mp3_tags(&raw_bytes)),
                _ => scrub_edf_ecg_header(&raw_bytes),
            }
            .map_err(|e| invalid(e.message))?;
            Ok(MultimodalV4ProcessResponse {
                format: rep.format,
                status: "processed".to_owned(),
                extracted_text: String::new(),
                scrubbed_text: String::new(),
                output_base64: Some(encode_base64(&rep.clean_bytes)),
                output_sha256: rep.output_sha256,
                removed_segments: rep.removed_segments,
                residual_findings: Vec::new(),
                residual_clean: true,
                archive_report: None,
            })
        }
        "archive_meta" => {
            let metas: Vec<ArchiveMemberMeta> = req
                .archive_members
                .iter()
                .map(|m| ArchiveMemberMeta {
                    path: m.path.clone(),
                    compressed_size: m.compressed_size,
                    uncompressed_size: m.uncompressed_size,
                    kind: if m.kind.is_empty() {
                        "file".to_owned()
                    } else {
                        m.kind.clone()
                    },
                    link_target: m.link_target.clone(),
                })
                .collect();
            let rep = evaluate_archive_members(&metas, &ArchiveSafetyPolicy::default());
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: format!("{:?}", rep.decision).to_ascii_lowercase(),
                extracted_text: String::new(),
                scrubbed_text: String::new(),
                output_base64: None,
                output_sha256: sha256_prefixed(
                    serde_json::to_string(&rep).unwrap_or_default().as_bytes(),
                ),
                removed_segments: rep.reasons.iter().map(|r| format!("{r:?}")).collect(),
                residual_findings: Vec::new(),
                residual_clean: rep.reasons.is_empty(),
                archive_report: Some(json!(rep)),
            })
        }
        "tesseract_ocr" => {
            let ocr = run_sanitized_local_tesseract(&raw_bytes, None);
            let scrubbed = scrub_fn(&ocr.extracted_text);
            Ok(MultimodalV4ProcessResponse {
                format: fmt,
                status: if ocr.available { "ok" } else { "unavailable" }.to_owned(),
                extracted_text: ocr.extracted_text,
                output_sha256: sha256_prefixed(scrubbed.as_bytes()),
                scrubbed_text: scrubbed,
                output_base64: None,
                removed_segments: vec![ocr.reason],
                residual_findings: Vec::new(),
                residual_clean: ocr.available,
                archive_report: None,
            })
        }
        other => Err(invalid(format!("unsupported multimodal format `{other}`"))),
    }
}

pub fn handle_dicom_deidentify(
    req: &DicomV4DeidentifyRequest,
) -> Result<DicomV4DeidentifyResponse, AuthorityError> {
    let raw = decode_base64(&req.base64_payload)?;
    let policy = req.policy_name.as_deref().or(Some("hipaa_safe_harbor"));
    let scrubber = core_text_scrubber(policy);
    let opts = DicomDeidentifyOptions {
        uid_hmac_secret: req
            .uid_hmac_secret
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "medscale-local-dicom-uid-v1".to_owned()),
        date_shift_days: req.date_shift_days,
        blackout_rects: req
            .blackout_rects
            .iter()
            .map(|r| PixelBlackoutRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            })
            .collect(),
    };
    let rep = deidentify_dicom_part10(&raw, &opts, &scrubber).map_err(|e| invalid(e.message))?;
    Ok(DicomV4DeidentifyResponse {
        transfer_syntax_uid: rep.transfer_syntax_uid,
        explicit_vr_dataset: rep.explicit_vr_dataset,
        tags_scrubbed: rep.tags_scrubbed,
        uids_remapped: rep.uids_remapped,
        sr_items_scrubbed: rep.sr_items_scrubbed,
        blackout_regions_applied: rep.blackout_regions_applied,
        residual_clean: rep.residual_clean,
        input_sha256: rep.input_sha256,
        output_sha256: rep.output_sha256,
        output_base64: encode_base64(&rep.scrubbed_bytes),
    })
}

pub fn handle_interop_process(
    req: &InteropV4ProcessRequest,
) -> Result<InteropV4ProcessResponse, AuthorityError> {
    let op = req.operation.trim().to_ascii_lowercase();
    let policy = req.policy_name.as_deref().or(Some("hipaa_safe_harbor"));
    let scrubber = core_text_scrubber(policy);
    match op.as_str() {
        "bridge_check" => {
            let bridge = req.bridge_name.as_deref().unwrap_or("");
            check_external_bridge_license(bridge).map_err(|e| invalid(e.message))?;
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: "allowed".to_owned(),
                result: json!({"bridge": bridge, "status": "allowed"}),
                warnings: Vec::new(),
                redaction_count: 0,
            })
        }
        "fhir_deidentify" => {
            let rep = fhir_deidentify_resource(&req.payload, req.date_shift_days, &scrubber)
                .map_err(|e| invalid(e.message))?;
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: "deidentified".to_owned(),
                result: json!({
                    "resource": rep.resource,
                    "operation_outcome": rep.operation_outcome,
                }),
                warnings: Vec::new(),
                redaction_count: rep.modified_paths,
            })
        }
        "fhir_integrity" => {
            let rep = check_us_core_and_bundle_integrity(&req.payload);
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: if rep.valid { "valid" } else { "invalid" }.to_owned(),
                warnings: rep.us_core_warnings.clone(),
                redaction_count: rep.dangling_paths.len(),
                result: json!(rep),
            })
        }
        "smart_audit" => {
            let rep = audit_smart_scopes(
                req.workflow_id.as_deref().unwrap_or("default"),
                &req.required_scopes,
                &req.declared_scopes,
            );
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: rep.status.clone(),
                warnings: rep.excessive_scopes.clone(),
                redaction_count: rep.missing_scopes.len() + rep.excessive_scopes.len(),
                result: json!(rep),
            })
        }
        "hl7v2_redact" => {
            let msg = req.raw_text.as_deref().unwrap_or("");
            let rep = redact_hl7v2(msg, req.date_shift_days, &scrubber)
                .map_err(|e| invalid(e.message))?;
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: "redacted".to_owned(),
                redaction_count: rep.scrubbed_fields,
                warnings: Vec::new(),
                result: json!(rep),
            })
        }
        "cda_redact" => {
            let xml = req.raw_text.as_deref().unwrap_or("");
            let rep = redact_cda_xml(xml, req.date_shift_days, &scrubber)
                .map_err(|e| invalid(e.message))?;
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: "redacted".to_owned(),
                redaction_count: rep.elements_scrubbed,
                warnings: Vec::new(),
                result: json!(rep),
            })
        }
        "x12_redact" => {
            let msg = req.raw_text.as_deref().unwrap_or("");
            let rep = redact_x12_edi(msg).map_err(|e| invalid(e.message))?;
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: "redacted".to_owned(),
                redaction_count: rep.redaction_count,
                warnings: Vec::new(),
                result: json!(rep),
            })
        }
        "omop_project" => {
            let rep = project_fhir_bundle_to_omop_v54(
                &req.payload,
                req.date_shift_days,
                req.reference_year.unwrap_or(2026),
                &scrubber,
            )
            .map_err(|e| invalid(e.message))?;
            Ok(InteropV4ProcessResponse {
                operation: op,
                status: "projected".to_owned(),
                redaction_count: rep.person.len() + rep.note.len(),
                warnings: Vec::new(),
                result: json!(rep),
            })
        }
        other => Err(invalid(format!("unsupported interop operation `{other}`"))),
    }
}

pub fn handle_agent_govern(
    req: &AgentV4GovernRequest,
) -> Result<AgentV4GovernResponse, AuthorityError> {
    let op = req.operation.trim().to_ascii_lowercase();
    match op.as_str() {
        "validate_graph" => {
            let nodes: Vec<ActionNodeInput> = req
                .nodes
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid ActionNodeInput: {e}")))?;
            let rep = validate_action_graph(&nodes).map_err(|e| invalid(e.message))?;
            Ok(AgentV4GovernResponse {
                operation: op,
                allowed: rep.valid,
                result: json!(rep),
            })
        }
        "check_phase" => {
            let cur = ActionPhase::parse(req.current_phase.as_deref().unwrap_or(""))
                .map_err(|e| invalid(e.message))?;
            let tgt = ActionPhase::parse(req.target_phase.as_deref().unwrap_or(""))
                .map_err(|e| invalid(e.message))?;
            match validate_action_transition(cur, tgt, req.reviewed) {
                Ok(()) => Ok(AgentV4GovernResponse {
                    operation: op,
                    allowed: true,
                    result: json!({"status": "allowed"}),
                }),
                Err(e) => Ok(AgentV4GovernResponse {
                    operation: op,
                    allowed: false,
                    result: json!({"status": "denied", "reason": e.message}),
                }),
            }
        }
        "issue_token" => {
            let digest = req.action_digest.as_deref().unwrap_or("");
            let role = req
                .reviewer_role
                .as_deref()
                .unwrap_or("role:medscale.clinical/clinician-reviewer@1.0.0");
            let nonce = req
                .nonce
                .as_deref()
                .unwrap_or("nonce_0123456789abcdef0123456789abcdef");
            let tok = issue_approval_token(
                LOCAL_APPROVAL_KEY,
                "medscale-local-1",
                digest,
                role,
                1_700_000_000,
                1_700_000_900,
                nonce,
            )
            .map_err(|e| invalid(e.message))?;
            Ok(AgentV4GovernResponse {
                operation: op,
                allowed: true,
                result: json!(tok),
            })
        }
        "verify_token" => {
            let val = req
                .token
                .clone()
                .ok_or_else(|| invalid("missing token object"))?;
            let tok: ApprovalToken =
                serde_json::from_value(val).map_err(|e| invalid(format!("invalid token: {e}")))?;
            let digest = req.action_digest.as_deref().unwrap_or(&tok.action_digest);
            let role = req.reviewer_role.as_deref().unwrap_or(&tok.reviewer_role);
            let mut guard = APPROVAL_NONCES
                .lock()
                .map_err(|_| AuthorityError::Internal {
                    message: "approval nonce mutex poisoned".to_owned(),
                })?;
            match verify_and_consume_approval_token(
                LOCAL_APPROVAL_KEY,
                &tok,
                digest,
                role,
                1_700_000_100,
                &mut guard,
            ) {
                Ok(()) => Ok(AgentV4GovernResponse {
                    operation: op,
                    allowed: true,
                    result: json!({"receipt_code": "approved"}),
                }),
                Err(e) => Ok(AgentV4GovernResponse {
                    operation: op,
                    allowed: false,
                    result: json!({"receipt_code": "denied", "reason": e.message}),
                }),
            }
        }
        "scan_injection" => {
            let txt = req.text.as_deref().unwrap_or("");
            let res = scan_prompt_injection(txt, req.strict);
            Ok(AgentV4GovernResponse {
                operation: op,
                allowed: !res.blocked,
                result: json!(res),
            })
        }
        "prior_auth_draft" => {
            let reqs: Vec<PriorAuthRequirementSpec> = req
                .requirements
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid prior_auth requirements: {e}")))?;
            let evs: Vec<PriorAuthEvidenceItem> = req
                .evidence
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid prior_auth evidence: {e}")))?;
            let draft = score_prior_auth_completeness(&reqs, &evs);
            Ok(AgentV4GovernResponse {
                operation: op,
                allowed: true,
                result: json!(draft),
            })
        }
        "trial_eligibility_draft" => {
            let criteria: Vec<TrialCriterionEval> = req
                .criteria
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid trial criteria: {e}")))?;
            let draft = build_trial_eligibility_review(
                req.trial_id.as_deref().unwrap_or("trial-draft"),
                &criteria,
            );
            Ok(AgentV4GovernResponse {
                operation: op,
                allowed: true,
                result: json!(draft),
            })
        }
        other => Err(invalid(format!(
            "unsupported agent governance op `{other}`"
        ))),
    }
}

pub fn handle_mcp_policy_check(
    req: &McpV4PolicyCheckRequest,
) -> Result<McpV4PolicyCheckResponse, AuthorityError> {
    let op = req.operation.trim().to_ascii_lowercase();
    match op.as_str() {
        "list_tools" => {
            let tools = default_mcp_tool_registry();
            Ok(McpV4PolicyCheckResponse {
                operation: op,
                allowed: true,
                result: json!({"tools": tools}),
            })
        }
        "validate_endpoint" => {
            let endpoint = req.endpoint.as_deref().unwrap_or("");
            let ips: Vec<IpAddr> = req
                .resolved_ips
                .iter()
                .map(|s| s.parse::<IpAddr>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid IP address in resolved_ips: {e}")))?;
            match validate_upstream_endpoint(
                endpoint,
                &ips,
                &req.allowed_origins,
                req.allow_loopback,
            ) {
                Ok(v) => Ok(McpV4PolicyCheckResponse {
                    operation: op,
                    allowed: true,
                    result: v,
                }),
                Err(e) => Ok(McpV4PolicyCheckResponse {
                    operation: op,
                    allowed: false,
                    result: json!({"allowed": false, "reason": e.message}),
                }),
            }
        }
        "issue_consent" => {
            let rcpt = issue_consent_receipt(
                LOCAL_MCP_CONSENT_KEY,
                "mcp-key-1",
                "rcpt-medscale-1",
                req.client.as_deref().unwrap_or("medscale-cli"),
                req.tool.as_deref().unwrap_or("medscale.privacy.deidentify"),
                "medscale://mcp",
                req.scope.as_deref().unwrap_or("mcp:state-changing"),
                &req.arguments,
                1_700_000_000,
                1_700_000_300,
            )
            .map_err(|e| invalid(e.message))?;
            Ok(McpV4PolicyCheckResponse {
                operation: op,
                allowed: true,
                result: json!(rcpt),
            })
        }
        "verify_consent" => {
            let val = req
                .receipt
                .clone()
                .ok_or_else(|| invalid("missing consent receipt"))?;
            let rcpt: McpConsentReceipt = serde_json::from_value(val)
                .map_err(|e| invalid(format!("invalid receipt: {e}")))?;
            match verify_consent_receipt(
                LOCAL_MCP_CONSENT_KEY,
                &rcpt,
                &req.arguments,
                1_700_000_050,
            ) {
                Ok(()) => Ok(McpV4PolicyCheckResponse {
                    operation: op,
                    allowed: true,
                    result: json!({"verified": true, "receipt_id": rcpt.receipt_id}),
                }),
                Err(e) => Ok(McpV4PolicyCheckResponse {
                    operation: op,
                    allowed: false,
                    result: json!({"verified": false, "reason": e.message}),
                }),
            }
        }
        other => Err(invalid(format!("unsupported MCP policy op `{other}`"))),
    }
}

pub fn handle_eval_run(req: &EvalV4RunRequest) -> Result<EvalV4RunResponse, AuthorityError> {
    let op = req.operation.trim().to_ascii_lowercase();
    match op.as_str() {
        "score_spans" => {
            let gold: Vec<EvalSpanItem> = req
                .gold_spans
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid gold_spans: {e}")))?;
            let pred: Vec<EvalSpanItem> = req
                .pred_spans
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid pred_spans: {e}")))?;
            let strict = evaluate_span_f1(&gold, &pred, false);
            let relaxed = evaluate_span_f1(&gold, &pred, true);
            Ok(EvalV4RunResponse {
                operation: op,
                passed: true,
                result: json!({"strict": strict, "relaxed": relaxed}),
            })
        }
        "score_relations" => {
            let gold: Vec<EvalRelationItem> = req
                .gold_relations
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid gold_relations: {e}")))?;
            let pred: Vec<EvalRelationItem> = req
                .pred_relations
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid pred_relations: {e}")))?;
            let metrics = evaluate_relation_f1(&gold, &pred);
            Ok(EvalV4RunResponse {
                operation: op,
                passed: true,
                result: json!(metrics),
            })
        }
        "calibration_ece" => {
            let pts: Vec<CalibrationPoint> = req
                .calibration_points
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(format!("invalid calibration_points: {e}")))?;
            let ece = compute_expected_calibration_error(&pts, req.n_bins.unwrap_or(5));
            Ok(EvalV4RunResponse {
                operation: op,
                passed: ece <= 0.25,
                result: json!({"ece": ece, "count": pts.len()}),
            })
        }
        "leakage_check" => {
            let overlap = check_split_overlap_leakage(&req.train_texts, &req.eval_texts);
            Ok(EvalV4RunResponse {
                operation: op,
                passed: overlap == 0,
                result: json!({"overlap_count": overlap}),
            })
        }
        "dua_path_guard" => {
            let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(|p| p.parent())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let ds = req.dataset_name.as_deref().unwrap_or("n2c2-2018");
            let cand = PathBuf::from(req.dataset_path.as_deref().unwrap_or(""));
            match validate_dua_dataset_path(ds, &cand, &repo_root, req.network_fetch) {
                Ok(()) => Ok(EvalV4RunResponse {
                    operation: op,
                    passed: true,
                    result: json!({"dataset": ds, "status": "outside_repo_ok"}),
                }),
                Err(e) => Ok(EvalV4RunResponse {
                    operation: op,
                    passed: false,
                    result: json!({"dataset": ds, "status": "blocked", "reason": e.message}),
                }),
            }
        }
        "terminology_guard" => {
            let term = req.terminology_name.as_deref().unwrap_or("");
            match check_restricted_terminology_bundle(term) {
                Ok(()) => Ok(EvalV4RunResponse {
                    operation: op,
                    passed: true,
                    result: json!({"terminology": term, "status": "allowed"}),
                }),
                Err(e) => Ok(EvalV4RunResponse {
                    operation: op,
                    passed: false,
                    result: json!({"terminology": term, "status": "blocked", "reason": e.message}),
                }),
            }
        }
        other => Err(invalid(format!("unsupported eval op `{other}`"))),
    }
}

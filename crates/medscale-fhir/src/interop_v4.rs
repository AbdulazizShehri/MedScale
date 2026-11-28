//! OpenMed v4.0.0 Wave 3 healthcare interoperability engine:
//! - FHIR R4 `$de-identify`, US Core & Bundle reference integrity
//! - SMART on FHIR v1/v2 scope grammar & least-privilege audit
//! - HL7v2 (`MSH`/`PID`/`NK1`/`PV1`/`OBR`/`OBX`/`NTE`) parser & scrubber
//! - C-CDA XML header & section narrative scrubber
//! - X12 837/835 EDI subscriber/patient scrubber
//! - OMOP CDM v5.4 Safe Harbor projection (<=89 age cap & date-shift)
//! - Quarantine refusal for GPL-2.0 (`sdcMicro`) & Elastic-2.0 (`MedCAT`)

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteropErrorKind {
    InvalidInput,
    BlockedExternalRights,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct InteropError {
    pub kind: InteropErrorKind,
    pub message: String,
}

impl InteropError {
    #[must_use]
    pub fn new(kind: InteropErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Explicitly refuses non-permissive external bridges quarantined per NOTICE policy.
pub fn check_external_bridge_license(bridge: &str) -> Result<(), InteropError> {
    let norm = bridge.trim().to_ascii_lowercase();
    if norm.contains("sdcmicro") {
        return Err(InteropError::new(
            InteropErrorKind::BlockedExternalRights,
            "sdcMicro bridge is excluded (GPL-2.0-or-later license); use built-in Apache-2.0 MedScale risk engine",
        ));
    }
    if norm.contains("medcat") {
        return Err(InteropError::new(
            InteropErrorKind::BlockedExternalRights,
            "MedCAT bridge is excluded (Elastic License 2.0 non-OSI); use built-in Apache-2.0 MedScale clinical NLP",
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for &b in &digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

fn civil_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_to_civil(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let yr = y + if m <= 2 { 1 } else { 0 };
    (yr as i32, m as u32, d as u32)
}

fn shift_iso_date(raw: &str, days: i64) -> String {
    if days == 0 || raw.len() < 10 {
        return raw.to_owned();
    }
    let head = &raw[..10];
    let mut parts = head.split('-');
    if let (Some(ys), Some(ms), Some(ds), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
        && let (Ok(y), Ok(m), Ok(d)) = (ys.parse::<i32>(), ms.parse::<u32>(), ds.parse::<u32>())
        && (1..=12).contains(&m)
        && (1..=31).contains(&d)
    {
        let (sy, sm, sd) = days_to_civil(civil_to_days(y, m, d) + days);
        return format!("{sy:04}-{sm:02}-{sd:02}{}", &raw[10..]);
    }
    raw.to_owned()
}

fn shift_compact_yyyymmdd(raw: &str, days: i64) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 8
        && trimmed.as_bytes()[..8].iter().all(u8::is_ascii_digit)
        && let (Ok(y), Ok(m), Ok(d)) = (
            trimmed[0..4].parse::<i32>(),
            trimmed[4..6].parse::<u32>(),
            trimmed[6..8].parse::<u32>(),
        )
        && (1..=12).contains(&m)
        && (1..=31).contains(&d)
    {
        let (sy, sm, sd) = days_to_civil(civil_to_days(y, m, d) + days);
        return format!("{sy:04}{sm:02}{sd:02}{}", &trimmed[8..]);
    }
    raw.to_owned()
}

/* ───────────────────────── FHIR R4 $de-identify ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FhirDeidentifyReport {
    pub resource: Value,
    pub modified_paths: usize,
    pub operation_outcome: Value,
}

fn is_fhir_skip_key(key: &str) -> bool {
    matches!(
        key,
        "resourceType"
            | "id"
            | "fullUrl"
            | "reference"
            | "type"
            | "system"
            | "code"
            | "version"
            | "url"
            | "uri"
            | "profile"
            | "status"
            | "use"
            | "gender"
            | "unit"
            | "comparator"
            | "contentType"
            | "language"
            | "valueCode"
            | "valueUri"
            | "valueUrl"
            | "valueCanonical"
            | "valueId"
            | "valueOid"
            | "valueUuid"
            | "valueBase64Binary"
    )
}

fn is_fhir_temporal_key(key: &str) -> bool {
    matches!(
        key,
        "birthDate"
            | "date"
            | "dateTime"
            | "effectiveDateTime"
            | "onsetDateTime"
            | "recordedDate"
            | "issued"
            | "start"
            | "end"
            | "valueDate"
            | "valueDateTime"
            | "valueInstant"
    )
}

fn deidentify_fhir_node(
    node: &Value,
    parent_key: &str,
    date_shift_days: i64,
    scrubber: &dyn Fn(&str) -> String,
    modified_paths: &mut usize,
) -> Value {
    match node {
        Value::Object(map) => {
            let mut out = Map::with_capacity(map.len());
            for (k, v) in map {
                if k == "coding" || k == "meta" {
                    out.insert(k.clone(), v.clone());
                    continue;
                }
                if is_fhir_skip_key(k) {
                    out.insert(k.clone(), v.clone());
                    continue;
                }
                if (k == "family" || k == "given")
                    && matches!(v, Value::String(_) | Value::Array(_))
                {
                    let masked = match v {
                        Value::String(_) => Value::String("[REDACTED:NAME]".to_owned()),
                        Value::Array(items) => Value::Array(
                            items
                                .iter()
                                .map(|_| Value::String("[REDACTED:NAME]".to_owned()))
                                .collect(),
                        ),
                        _ => v.clone(),
                    };
                    *modified_paths += 1;
                    out.insert(k.clone(), masked);
                    continue;
                }
                out.insert(
                    k.clone(),
                    deidentify_fhir_node(v, k, date_shift_days, scrubber, modified_paths),
                );
            }
            if out.contains_key("resourceType")
                && let Some(Value::String(rt)) = out.get("resourceType")
                && rt != "OperationOutcome"
            {
                let sec = json!({
                    "system": "http://terminology.hl7.org/CodeSystem/v3-ObservationValue",
                    "code": if date_shift_days != 0 { "PSEUDED" } else { "REDACTED" }
                });
                let meta_entry = out.entry("meta".to_owned()).or_insert_with(|| json!({}));
                if let Some(meta_obj) = meta_entry.as_object_mut() {
                    let tags = meta_obj
                        .entry("security".to_owned())
                        .or_insert_with(|| json!([]));
                    if let Some(arr) = tags.as_array_mut()
                        && !arr.iter().any(|x| x == &sec)
                    {
                        arr.push(sec);
                    }
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| {
                    deidentify_fhir_node(v, parent_key, date_shift_days, scrubber, modified_paths)
                })
                .collect(),
        ),
        Value::String(s) => {
            if is_fhir_temporal_key(parent_key) && date_shift_days != 0 {
                let shifted = shift_iso_date(s, date_shift_days);
                if shifted != *s {
                    *modified_paths += 1;
                }
                Value::String(shifted)
            } else if parent_key == "value" {
                // Identifier.value or ContactPoint.value
                let hashed = format!("ID_{}", &sha256_hex(s.as_bytes())[..12]);
                *modified_paths += 1;
                Value::String(hashed)
            } else {
                let clean = scrubber(s);
                if clean != *s {
                    *modified_paths += 1;
                }
                Value::String(clean)
            }
        }
        other => other.clone(),
    }
}

pub fn fhir_deidentify_resource(
    payload: &Value,
    date_shift_days: i64,
    scrubber: &dyn Fn(&str) -> String,
) -> Result<FhirDeidentifyReport, InteropError> {
    if !payload.is_object() {
        return Err(InteropError::new(
            InteropErrorKind::InvalidInput,
            "FHIR payload must be a JSON object",
        ));
    }
    let mut modified_paths = 0usize;
    let resource =
        deidentify_fhir_node(payload, "", date_shift_days, scrubber, &mut modified_paths);
    let outcome = json!({
        "resourceType": "OperationOutcome",
        "issue": [{
            "severity": "information",
            "code": "informational",
            "diagnostics": format!("medscale.$de-identify transformed {modified_paths} element(s)")
        }]
    });
    Ok(FhirDeidentifyReport {
        resource,
        modified_paths,
        operation_outcome: outcome,
    })
}

/* ───────────────────────── US Core & Bundle Reference Integrity ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FhirIntegrityReport {
    pub valid: bool,
    pub entry_count: usize,
    pub reference_count: usize,
    pub dangling_paths: Vec<String>,
    pub us_core_warnings: Vec<String>,
}

fn collect_references(val: &Value, path: &str, out: &mut Vec<(String, String)>) {
    match val {
        Value::Object(map) => {
            for (k, v) in map {
                let next_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                if k == "reference"
                    && let Value::String(r) = v
                {
                    out.push((next_path, r.clone()));
                } else {
                    collect_references(v, &next_path, out);
                }
            }
        }
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                collect_references(item, &format!("{path}[{idx}]"), out);
            }
        }
        _ => {}
    }
}

#[must_use]
pub fn check_us_core_and_bundle_integrity(bundle: &Value) -> FhirIntegrityReport {
    let entries = bundle
        .get("entry")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut known_targets = BTreeSet::new();
    let mut us_core_warnings = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        if let Some(full_url) = entry.get("fullUrl").and_then(Value::as_str) {
            known_targets.insert(full_url.to_owned());
        }
        if let Some(res) = entry.get("resource") {
            let rt = res
                .get("resourceType")
                .and_then(Value::as_str)
                .unwrap_or("");
            if let Some(id) = res.get("id").and_then(Value::as_str)
                && !rt.is_empty()
            {
                known_targets.insert(format!("{rt}/{id}"));
            }
            if let Some(contained) = res.get("contained").and_then(Value::as_array) {
                for c in contained {
                    if let Some(cid) = c.get("id").and_then(Value::as_str) {
                        known_targets.insert(format!("#{cid}"));
                    }
                }
            }
            // Minimal US Core structural requirements
            match rt {
                "Patient" => {
                    if res.get("identifier").is_none() && res.get("name").is_none() {
                        us_core_warnings.push(format!(
                            "Bundle.entry[{idx}].Patient missing identifier/name"
                        ));
                    }
                }
                "Observation" => {
                    for req in ["status", "code", "subject"] {
                        if res.get(req).is_none() {
                            us_core_warnings
                                .push(format!("Bundle.entry[{idx}].Observation missing {req}"));
                        }
                    }
                }
                "Condition" => {
                    for req in ["clinicalStatus", "code", "subject"] {
                        if res.get(req).is_none() {
                            us_core_warnings
                                .push(format!("Bundle.entry[{idx}].Condition missing {req}"));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let mut refs = Vec::new();
    collect_references(bundle, "Bundle", &mut refs);
    let mut dangling_paths = Vec::new();
    for (path, target) in &refs {
        if !known_targets.contains(target.as_str()) {
            dangling_paths.push(path.clone());
        }
    }

    FhirIntegrityReport {
        valid: dangling_paths.is_empty(),
        entry_count: entries.len(),
        reference_count: refs.len(),
        dangling_paths,
        us_core_warnings,
    }
}

/* ───────────────────────── SMART on FHIR Scope Grammar & Audit ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SmartScopeNormalized {
    pub context: String,
    pub resource_type: Option<String>,
    pub operations: String,
    pub is_clinical: bool,
    pub canonical: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartScopeAuditReport {
    pub workflow_id: String,
    pub status: String,
    pub missing_scopes: Vec<String>,
    pub excessive_scopes: Vec<String>,
    pub invalid_scopes: Vec<String>,
}

pub fn parse_smart_scope(raw: &str) -> Result<SmartScopeNormalized, String> {
    let s = raw.trim();
    if matches!(
        s,
        "openid"
            | "profile"
            | "fhirUser"
            | "launch"
            | "launch/patient"
            | "launch/encounter"
            | "online_access"
            | "offline_access"
    ) {
        return Ok(SmartScopeNormalized {
            context: "non_clinical".to_owned(),
            resource_type: None,
            operations: String::new(),
            is_clinical: false,
            canonical: s.to_owned(),
        });
    }
    let (base, query) = s.split_once('?').unwrap_or((s, ""));
    let (ctx, rest) = base
        .split_once('/')
        .ok_or_else(|| format!("missing context slash in `{raw}`"))?;
    if !matches!(ctx, "patient" | "user" | "system") {
        return Err(format!("invalid SMART context `{ctx}`"));
    }
    let (res, ops_raw) = rest
        .split_once('.')
        .ok_or_else(|| format!("missing resource/operation dot in `{raw}`"))?;
    if res != "*"
        && (res.is_empty()
            || res.len() > 64
            || !res.starts_with(|c: char| c.is_ascii_uppercase())
            || !res.chars().all(|c| c.is_ascii_alphanumeric()))
    {
        return Err(format!("invalid SMART resource `{res}`"));
    }
    let normalized_ops = match ops_raw {
        "read" => "rs".to_owned(),
        "write" => "cud".to_owned(),
        "*" => "cruds".to_owned(),
        other => {
            if other.is_empty()
                || !other
                    .chars()
                    .all(|c| matches!(c, 'c' | 'r' | 'u' | 'd' | 's'))
            {
                return Err(format!("invalid SMART operations `{ops_raw}`"));
            }
            let mut canonical = String::new();
            for ch in ['c', 'r', 'u', 'd', 's'] {
                if other.contains(ch) {
                    canonical.push(ch);
                }
            }
            canonical
        }
    };
    let canonical = if query.is_empty() {
        format!("{ctx}/{res}.{normalized_ops}")
    } else {
        format!("{ctx}/{res}.{normalized_ops}?{query}")
    };
    Ok(SmartScopeNormalized {
        context: ctx.to_owned(),
        resource_type: Some(res.to_owned()),
        operations: normalized_ops,
        is_clinical: true,
        canonical,
    })
}

fn scope_covers(granted: &SmartScopeNormalized, required: &SmartScopeNormalized) -> bool {
    if !granted.is_clinical || !required.is_clinical {
        return granted.canonical == required.canonical;
    }
    let ctx_rank = |c: &str| match c {
        "patient" => 0,
        "user" => 1,
        "system" => 2,
        _ => 9,
    };
    if ctx_rank(&granted.context) < ctx_rank(&required.context) {
        return false;
    }
    if granted.resource_type.as_deref() != Some("*")
        && granted.resource_type != required.resource_type
    {
        return false;
    }
    required
        .operations
        .chars()
        .all(|op| granted.operations.contains(op))
}

#[must_use]
pub fn audit_smart_scopes(
    workflow_id: &str,
    required_scopes: &[String],
    declared_scopes: &[String],
) -> SmartScopeAuditReport {
    let mut invalid = Vec::new();
    let mut req_parsed = Vec::new();
    for s in required_scopes {
        match parse_smart_scope(s) {
            Ok(p) => req_parsed.push(p),
            Err(e) => invalid.push(e),
        }
    }
    let mut decl_parsed = Vec::new();
    for s in declared_scopes {
        match parse_smart_scope(s) {
            Ok(p) => decl_parsed.push(p),
            Err(e) => invalid.push(e),
        }
    }
    let missing: Vec<String> = req_parsed
        .iter()
        .filter(|r| !decl_parsed.iter().any(|d| scope_covers(d, r)))
        .map(|r| r.canonical.clone())
        .collect();
    let excessive: Vec<String> = decl_parsed
        .iter()
        .filter(|d| !req_parsed.iter().any(|r| scope_covers(r, d)))
        .map(|d| d.canonical.clone())
        .collect();

    let status = if !invalid.is_empty() {
        "invalid"
    } else if !missing.is_empty() {
        "missing"
    } else if !excessive.is_empty() {
        "excessive"
    } else {
        "pass"
    };

    SmartScopeAuditReport {
        workflow_id: workflow_id.to_owned(),
        status: status.to_owned(),
        missing_scopes: missing,
        excessive_scopes: excessive,
        invalid_scopes: invalid,
    }
}

/* ───────────────────────── HL7v2 Parser & Segment Scrubber ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hl7v2ScrubReport {
    pub segment_names: Vec<String>,
    pub redacted_message: String,
    pub scrubbed_fields: usize,
}

fn hl7_hash_leaf(leaf: &str) -> String {
    if leaf.is_empty() {
        String::new()
    } else {
        format!("[ID_NUM_HASH_{}]", &sha256_hex(leaf.as_bytes())[..12])
    }
}

pub fn redact_hl7v2(
    message: &str,
    date_shift_days: i64,
    scrubber: &dyn Fn(&str) -> String,
) -> Result<Hl7v2ScrubReport, InteropError> {
    let normalized = message.replace("\r\n", "\r").replace('\n', "\r");
    let raw_segs: Vec<&str> = normalized
        .split('\r')
        .filter(|s| !s.trim().is_empty())
        .collect();
    let Some(first) = raw_segs.first() else {
        return Err(InteropError::new(
            InteropErrorKind::InvalidInput,
            "empty HL7v2 message",
        ));
    };
    if first.len() < 8 || !first.starts_with("MSH") {
        return Err(InteropError::new(
            InteropErrorKind::InvalidInput,
            "HL7v2 messages must start with MSH segment",
        ));
    }
    let sep = first.as_bytes()[3] as char;
    let mut segment_names = Vec::with_capacity(raw_segs.len());
    let mut out_segs = Vec::with_capacity(raw_segs.len());
    let mut scrubbed_fields = 0usize;

    for seg in raw_segs {
        let name = &seg[..3.min(seg.len())];
        segment_names.push(name.to_owned());
        if name == "MSH" {
            out_segs.push(seg.to_owned());
            continue;
        }
        let mut fields: Vec<String> = seg.split(sep).map(str::to_owned).collect();
        let clear_idx = |flds: &mut Vec<String>, idx: usize, count: &mut usize| {
            if let Some(slot) = flds.get_mut(idx)
                && !slot.is_empty()
            {
                slot.clear();
                *count += 1;
            }
        };
        match name {
            "PID" => {
                if let Some(id_fld) = fields.get_mut(3)
                    && !id_fld.is_empty()
                {
                    let hashed_parts: Vec<String> = id_fld
                        .split('^')
                        .map(|comp| {
                            if comp.is_empty() {
                                String::new()
                            } else {
                                hl7_hash_leaf(comp)
                            }
                        })
                        .collect();
                    *id_fld = hashed_parts.join("^");
                    scrubbed_fields += 1;
                }
                for idx in [5, 6, 9, 11, 13, 14, 18, 19] {
                    clear_idx(&mut fields, idx, &mut scrubbed_fields);
                }
                if let Some(dob) = fields.get_mut(7)
                    && !dob.is_empty()
                {
                    *dob = shift_compact_yyyymmdd(dob, date_shift_days);
                    scrubbed_fields += 1;
                }
            }
            "NK1" => {
                for idx in [2, 4, 5, 6] {
                    clear_idx(&mut fields, idx, &mut scrubbed_fields);
                }
            }
            "PV1" => {
                for idx in [7, 8, 9, 17, 19] {
                    clear_idx(&mut fields, idx, &mut scrubbed_fields);
                }
            }
            "OBR" => {
                clear_idx(&mut fields, 16, &mut scrubbed_fields);
            }
            "NTE" => {
                if let Some(txt) = fields.get_mut(3)
                    && !txt.is_empty()
                {
                    *txt = scrubber(txt);
                    scrubbed_fields += 1;
                }
            }
            "OBX" => {
                let vt = fields.get(2).map(String::as_str).unwrap_or("");
                if matches!(vt, "FT" | "TX")
                    && let Some(val) = fields.get_mut(5)
                    && !val.is_empty()
                {
                    *val = scrubber(val);
                    scrubbed_fields += 1;
                }
            }
            _ => {}
        }
        out_segs.push(fields.join(&sep.to_string()));
    }

    Ok(Hl7v2ScrubReport {
        segment_names,
        redacted_message: format!("{}\r", out_segs.join("\r")),
        scrubbed_fields,
    })
}

/* ───────────────────────── C-CDA XML Scrubber ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CdaScrubReport {
    pub is_cda: bool,
    pub redacted_xml: String,
    pub elements_scrubbed: usize,
}

#[must_use]
pub fn is_cda_document(xml: &str) -> bool {
    xml.contains("ClinicalDocument") && xml.contains("urn:hl7-org:v3")
}

fn replace_tag_block(xml: &str, tag: &str, replacement: &str, count: &mut usize) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(pos) = rest.find(&open) {
        out.push_str(&rest[..pos]);
        if let Some(end_rel) = rest[pos..].find(&close) {
            out.push_str(replacement);
            *count += 1;
            rest = &rest[pos + end_rel + close.len()..];
        } else {
            out.push_str(&rest[pos..]);
            return out;
        }
    }
    out.push_str(rest);
    out
}

pub fn redact_cda_xml(
    xml: &str,
    date_shift_days: i64,
    scrubber: &dyn Fn(&str) -> String,
) -> Result<CdaScrubReport, InteropError> {
    if !is_cda_document(xml) {
        return Err(InteropError::new(
            InteropErrorKind::InvalidInput,
            "document is not a valid HL7 v3 ClinicalDocument",
        ));
    }
    let mut elements_scrubbed = 0usize;
    let mut work = xml.to_owned();

    // Split before structuredBody so header-only <name>/<addr>/<telecom> rules only target header metadata.
    let body_idx = work.find("<structuredBody").unwrap_or(work.len());
    let mut hdr = work[..body_idx].to_owned();
    let body = work[body_idx..].to_owned();

    hdr = replace_tag_block(
        &hdr,
        "name",
        "<name nullFlavor=\"UNK\" />",
        &mut elements_scrubbed,
    );
    hdr = replace_tag_block(
        &hdr,
        "addr",
        "<addr nullFlavor=\"UNK\" />",
        &mut elements_scrubbed,
    );
    hdr = replace_tag_block(
        &hdr,
        "telecom",
        "<telecom nullFlavor=\"UNK\" />",
        &mut elements_scrubbed,
    );

    // Hash `extension="..."` on patientRole/id and shift `birthTime value="..."`
    let mut hashed_hdr = String::with_capacity(hdr.len());
    let mut rem = hdr.as_str();
    while let Some(pos) = rem.find("extension=\"") {
        let start = pos + "extension=\"".len();
        if let Some(end_rel) = rem[start..].find('"') {
            hashed_hdr.push_str(&rem[..start]);
            let val = &rem[start..start + end_rel];
            hashed_hdr.push('h');
            hashed_hdr.push_str(&sha256_hex(val.as_bytes())[..16]);
            elements_scrubbed += 1;
            rem = &rem[start + end_rel..];
        } else {
            break;
        }
    }
    hashed_hdr.push_str(rem);

    let mut date_hdr = String::with_capacity(hashed_hdr.len());
    let mut rem2 = hashed_hdr.as_str();
    while let Some(pos) = rem2.find("<birthTime ") {
        if let Some(end_rel) = rem2[pos..].find('>') {
            let tag_slice = &rem2[pos..pos + end_rel + 1];
            date_hdr.push_str(&rem2[..pos]);
            if let Some(vp) = tag_slice.find("value=\"") {
                let vs = vp + 7;
                if let Some(ve_rel) = tag_slice[vs..].find('"') {
                    let orig = &tag_slice[vs..vs + ve_rel];
                    let shifted = shift_compact_yyyymmdd(orig, date_shift_days);
                    date_hdr.push_str(&tag_slice[..vs]);
                    date_hdr.push_str(&shifted);
                    date_hdr.push_str(&tag_slice[vs + ve_rel..]);
                    elements_scrubbed += 1;
                } else {
                    date_hdr.push_str(tag_slice);
                }
            } else {
                date_hdr.push_str(tag_slice);
            }
            rem2 = &rem2[pos + end_rel + 1..];
        } else {
            break;
        }
    }
    date_hdr.push_str(rem2);

    // Scrub free text inside <text>...</text> nodes of structuredBody.
    let mut out_body = String::with_capacity(body.len());
    let mut b_rest = body.as_str();
    while let Some(ts) = b_rest.find("<text>") {
        let content_start = ts + 6;
        if let Some(te_rel) = b_rest[content_start..].find("</text>") {
            out_body.push_str(&b_rest[..content_start]);
            let raw_text = &b_rest[content_start..content_start + te_rel];
            let scrubbed = scrubber(raw_text);
            if scrubbed != raw_text {
                elements_scrubbed += 1;
            }
            out_body.push_str(&scrubbed);
            out_body.push_str("</text>");
            b_rest = &b_rest[content_start + te_rel + 7..];
        } else {
            break;
        }
    }
    out_body.push_str(b_rest);
    work = format!("{date_hdr}{out_body}");

    Ok(CdaScrubReport {
        is_cda: true,
        redacted_xml: work,
        elements_scrubbed,
    })
}

/* ───────────────────────── X12 837/835 EDI Scrubber ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct X12ScrubReport {
    pub transaction_type: String,
    pub redaction_count: usize,
    pub deidentified_text: String,
}

pub fn redact_x12_edi(message: &str) -> Result<X12ScrubReport, InteropError> {
    if message.len() < 106 || !message.starts_with("ISA") {
        return Err(InteropError::new(
            InteropErrorKind::InvalidInput,
            "X12 interchange must begin with 106-byte ISA header",
        ));
    }
    let elem_sep = message.as_bytes()[3] as char;
    let seg_term = message.as_bytes()[105] as char;
    let raw_segs: Vec<&str> = message
        .split(seg_term)
        .map(|s| s.trim_matches(['\r', '\n', ' ']))
        .filter(|s| !s.is_empty())
        .collect();
    let tags: Vec<&str> = raw_segs
        .iter()
        .map(|s| s.split(elem_sep).next().unwrap_or(""))
        .collect();
    if tags.first() != Some(&"ISA") || tags.last() != Some(&"IEA") {
        return Err(InteropError::new(
            InteropErrorKind::InvalidInput,
            "X12 interchange must be framed by ISA and IEA segments",
        ));
    }
    for req in ["GS", "ST", "SE", "GE"] {
        if !tags.contains(&req) {
            return Err(InteropError::new(
                InteropErrorKind::InvalidInput,
                format!("X12 envelope missing required segment {req}"),
            ));
        }
    }

    let mut tx_type = String::new();
    let mut redaction_count = 0usize;
    let mut out_segs = Vec::with_capacity(raw_segs.len());
    for seg in raw_segs {
        let mut fields: Vec<String> = seg.split(elem_sep).map(str::to_owned).collect();
        let tag = fields.first().map(String::as_str).unwrap_or("");
        if tag == "ST" && tx_type.is_empty() {
            tx_type = fields.get(1).cloned().unwrap_or_default();
            if !matches!(tx_type.as_str(), "837" | "835") {
                return Err(InteropError::new(
                    InteropErrorKind::InvalidInput,
                    format!("unsupported X12 transaction set `{tx_type}` (expected 837 or 835)"),
                ));
            }
        }
        let redact_indices = |flds: &mut Vec<String>, idxs: &[usize], cnt: &mut usize| {
            for &idx in idxs {
                if let Some(slot) = flds.get_mut(idx)
                    && !slot.is_empty()
                {
                    *slot = "REDACTED".to_owned();
                    *cnt += 1;
                }
            }
        };
        match tag {
            "NM1" => {
                let entity = fields.get(1).map(String::as_str).unwrap_or("");
                if matches!(entity, "IL" | "QC" | "85" | "87") {
                    redact_indices(&mut fields, &[3, 4, 5, 9], &mut redaction_count);
                }
            }
            "N3" => redact_indices(&mut fields, &[1, 2], &mut redaction_count),
            "N4" => redact_indices(&mut fields, &[1, 2, 3], &mut redaction_count),
            "DMG" => redact_indices(&mut fields, &[2], &mut redaction_count),
            "PER" => redact_indices(&mut fields, &[2, 4, 6], &mut redaction_count),
            "REF" => {
                let q = fields.get(1).map(String::as_str).unwrap_or("");
                if matches!(q, "SY" | "1W" | "EA") {
                    redact_indices(&mut fields, &[2], &mut redaction_count);
                }
            }
            _ => {}
        }
        out_segs.push(fields.join(&elem_sep.to_string()));
    }

    Ok(X12ScrubReport {
        transaction_type: tx_type,
        redaction_count,
        deidentified_text: format!("{}{seg_term}", out_segs.join(&seg_term.to_string())),
    })
}

/* ───────────────────────── OMOP CDM v5.4 Safe Harbor Projection ───────────────────────── */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmopProjectionReport {
    pub cdm_version: String,
    pub person: Vec<Value>,
    pub visit_occurrence: Vec<Value>,
    pub condition_occurrence: Vec<Value>,
    pub drug_exposure: Vec<Value>,
    pub measurement: Vec<Value>,
    pub note: Vec<Value>,
}

fn stable_surrogate_id(namespace: &str, raw: &str) -> u64 {
    let digest = Sha256::digest(format!("{namespace}:{raw}").as_bytes());
    let mut b = [0u8; 8];
    b.copy_from_slice(&digest[..8]);
    (u64::from_be_bytes(b) & 0x7FFF_FFFF_FFFF_FFFF).max(1)
}

pub fn project_fhir_bundle_to_omop_v54(
    bundle: &Value,
    date_shift_days: i64,
    reference_year: i32,
    scrubber: &dyn Fn(&str) -> String,
) -> Result<OmopProjectionReport, InteropError> {
    let entries = bundle
        .get("entry")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            InteropError::new(
                InteropErrorKind::InvalidInput,
                "FHIR Bundle must contain an entry array",
            )
        })?;

    let mut person = Vec::new();
    let mut visit_occurrence = Vec::new();
    let mut condition_occurrence = Vec::new();
    let mut drug_exposure = Vec::new();
    let mut measurement = Vec::new();
    let mut note = Vec::new();

    for entry in entries {
        let Some(res) = entry.get("resource") else {
            continue;
        };
        let rt = res
            .get("resourceType")
            .and_then(Value::as_str)
            .unwrap_or("");
        let id = res.get("id").and_then(Value::as_str).unwrap_or("anon");
        let subj_ref = res
            .get("subject")
            .or_else(|| res.get("patient"))
            .and_then(|s| s.get("reference"))
            .and_then(Value::as_str)
            .unwrap_or("Patient/anon");
        let person_id =
            stable_surrogate_id("omop.person", if rt == "Patient" { id } else { subj_ref });

        match rt {
            "Patient" => {
                let raw_year = res
                    .get("birthDate")
                    .and_then(Value::as_str)
                    .and_then(|s| s.get(..4))
                    .and_then(|y| y.parse::<i32>().ok());
                // HIPAA Safe Harbor age 90+ cap -> cap birth_year at reference_year - 90.
                let capped_year = raw_year.map(|y| {
                    if reference_year - y >= 90 {
                        reference_year - 90
                    } else {
                        y
                    }
                });
                let gender_concept_id = match res.get("gender").and_then(Value::as_str) {
                    Some("male") => 8507,
                    Some("female") => 8532,
                    _ => 0,
                };
                person.push(json!({
                    "person_id": person_id,
                    "gender_concept_id": gender_concept_id,
                    "year_of_birth": capped_year,
                    "month_of_birth": Value::Null,
                    "day_of_birth": Value::Null,
                    "person_source_value": format!("PSN_{}", &sha256_hex(id.as_bytes())[..12]),
                }));
            }
            "Encounter" => {
                let start = res
                    .get("period")
                    .and_then(|p| p.get("start"))
                    .and_then(Value::as_str)
                    .map(|s| shift_iso_date(s, date_shift_days));
                visit_occurrence.push(json!({
                    "visit_occurrence_id": stable_surrogate_id("omop.visit", id),
                    "person_id": person_id,
                    "visit_concept_id": 9202,
                    "visit_start_date": start,
                }));
            }
            "Condition" => {
                let code = res
                    .get("code")
                    .and_then(|c| c.get("coding"))
                    .and_then(Value::as_array)
                    .and_then(|a| a.first())
                    .and_then(|c| c.get("code"))
                    .and_then(Value::as_str)
                    .unwrap_or("0");
                let onset = res
                    .get("onsetDateTime")
                    .and_then(Value::as_str)
                    .map(|s| shift_iso_date(s, date_shift_days));
                condition_occurrence.push(json!({
                    "condition_occurrence_id": stable_surrogate_id("omop.cond", id),
                    "person_id": person_id,
                    "condition_source_value": code,
                    "condition_start_date": onset,
                }));
            }
            "MedicationRequest" | "MedicationStatement" => {
                let start = res
                    .get("authoredOn")
                    .and_then(Value::as_str)
                    .map(|s| shift_iso_date(s, date_shift_days));
                let med_src = res
                    .get("medicationCodeableConcept")
                    .and_then(|c| c.get("text"))
                    .and_then(Value::as_str)
                    .map(scrubber)
                    .unwrap_or_else(|| "medication".to_owned());
                drug_exposure.push(json!({
                    "drug_exposure_id": stable_surrogate_id("omop.drug", id),
                    "person_id": person_id,
                    "drug_source_value": med_src,
                    "drug_exposure_start_date": start,
                }));
            }
            "Observation" => {
                let code = res
                    .get("code")
                    .and_then(|c| c.get("coding"))
                    .and_then(Value::as_array)
                    .and_then(|a| a.first())
                    .and_then(|c| c.get("code"))
                    .and_then(Value::as_str)
                    .unwrap_or("0");
                let val_num = res
                    .get("valueQuantity")
                    .and_then(|q| q.get("value"))
                    .and_then(Value::as_f64);
                let unit = res
                    .get("valueQuantity")
                    .and_then(|q| q.get("unit"))
                    .and_then(Value::as_str);
                let eff = res
                    .get("effectiveDateTime")
                    .and_then(Value::as_str)
                    .map(|s| shift_iso_date(s, date_shift_days));
                measurement.push(json!({
                    "measurement_id": stable_surrogate_id("omop.meas", id),
                    "person_id": person_id,
                    "measurement_source_value": code,
                    "measurement_date": eff,
                    "value_as_number": val_num,
                    "unit_source_value": unit,
                }));
            }
            "DocumentReference" | "DiagnosticReport" => {
                let raw_note = res
                    .get("description")
                    .or_else(|| res.get("text").and_then(|t| t.get("div")))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                note.push(json!({
                    "note_id": stable_surrogate_id("omop.note", id),
                    "person_id": person_id,
                    "note_text": scrubber(raw_note),
                }));
            }
            _ => {}
        }
    }

    Ok(OmopProjectionReport {
        cdm_version: "5.4".to_owned(),
        person,
        visit_occurrence,
        condition_occurrence,
        drug_exposure,
        measurement,
        note,
    })
}

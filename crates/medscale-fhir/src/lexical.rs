//! Fail-closed lexical checks for FHIR JSON interchange.

use std::collections::HashSet;

use medscale_contracts::FHIR_R4_VERSION;
use serde_json::Value;
use thiserror::Error;

use crate::{MAX_INGEST_BYTES, MAX_JSON_DEPTH};

/// Lexical reject reasons.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LexicalError {
    #[error("payload exceeds size limit")]
    TooLarge,
    #[error("truncated or invalid JSON: {0}")]
    InvalidJson(String),
    #[error("duplicate object key: {0}")]
    DuplicateKey(String),
    #[error("JSON nesting too deep")]
    TooDeep,
    #[error("silent f64 clinical decimal coercion forbidden")]
    UnsafeDecimal,
    #[error("FHIR version not admitted: {0:?}")]
    Version(Option<String>),
    #[error("media type must be application/fhir+json")]
    MediaType,
    #[error("missing resourceType")]
    MissingResourceType,
}

/// Successful lexical gate report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalReport {
    pub resource_type: String,
    pub fhir_version: String,
}

/// Run lexical/structural gate on exact bytes (does not mutate bytes).
pub fn gate_fhir_json(
    media_type: &str,
    bytes: &[u8],
    version_hint: Option<&str>,
) -> Result<LexicalReport, LexicalError> {
    if media_type != "application/fhir+json" {
        return Err(LexicalError::MediaType);
    }
    if bytes.len() > MAX_INGEST_BYTES {
        return Err(LexicalError::TooLarge);
    }
    reject_duplicate_keys(bytes)?;
    reject_unsafe_decimals(bytes)?;

    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| LexicalError::InvalidJson(e.to_string()))?;
    check_depth(&value, 0)?;

    let resource_type = value
        .get("resourceType")
        .and_then(Value::as_str)
        .ok_or(LexicalError::MissingResourceType)?
        .to_owned();

    let declared = value
        .get("fhirVersion")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| version_hint.map(str::to_owned));

    let fhir_version = match declared.as_deref() {
        None => FHIR_R4_VERSION.to_owned(), // structural R4 JSON admitted when version omitted
        Some(v) if v == FHIR_R4_VERSION => v.to_owned(),
        Some(_) => return Err(LexicalError::Version(declared)),
    };

    Ok(LexicalReport {
        resource_type,
        fhir_version,
    })
}

/// Extract Patient.identifier pairs for identity evidence (no merge).
#[must_use]
pub fn extract_patient_identifiers(bytes: &[u8]) -> Vec<(String, String)> {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_identifiers(&value, &mut out);
    if value.get("resourceType").and_then(Value::as_str) == Some("Bundle")
        && let Some(entries) = value.get("entry").and_then(Value::as_array)
    {
        for entry in entries {
            if let Some(resource) = entry.get("resource") {
                collect_identifiers(resource, &mut out);
            }
        }
    }
    out
}

fn collect_identifiers(value: &Value, out: &mut Vec<(String, String)>) {
    if value.get("resourceType").and_then(Value::as_str) != Some("Patient") {
        return;
    }
    let Some(ids) = value.get("identifier").and_then(Value::as_array) else {
        return;
    };
    for id in ids {
        let system = id
            .get("system")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let value = id
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if !system.is_empty() && !value.is_empty() {
            out.push((system, value));
        }
    }
}

fn check_depth(value: &Value, depth: usize) -> Result<(), LexicalError> {
    if depth > MAX_JSON_DEPTH {
        return Err(LexicalError::TooDeep);
    }
    match value {
        Value::Array(items) => {
            for item in items {
                check_depth(item, depth + 1)?;
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                check_depth(v, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Scan raw JSON for duplicate keys within any object.
fn reject_duplicate_keys(bytes: &[u8]) -> Result<(), LexicalError> {
    let text = std::str::from_utf8(bytes).map_err(|e| LexicalError::InvalidJson(e.to_string()))?;
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    scan_value(&chars, &mut i, 0)?;
    skip_ws(&chars, &mut i);
    if i != chars.len() {
        return Err(LexicalError::InvalidJson("trailing content".to_owned()));
    }
    Ok(())
}

fn skip_ws(chars: &[char], i: &mut usize) {
    while *i < chars.len() && chars[*i].is_whitespace() {
        *i += 1;
    }
}

/// `depth` counts enclosing objects and arrays, as `check_depth` does. The
/// scan runs before serde parses anything, so it enforces the limit itself
/// instead of recursing until the stack overflows.
fn scan_value(chars: &[char], i: &mut usize, depth: usize) -> Result<(), LexicalError> {
    if depth > MAX_JSON_DEPTH {
        return Err(LexicalError::TooDeep);
    }
    skip_ws(chars, i);
    if *i >= chars.len() {
        return Err(LexicalError::InvalidJson("unexpected end".to_owned()));
    }
    match chars[*i] {
        '{' => scan_object(chars, i, depth),
        '[' => scan_array(chars, i, depth),
        '"' => {
            scan_string(chars, i)?;
            Ok(())
        }
        't' | 'f' | 'n' | '-' | '0'..='9' => {
            scan_literal_or_number(chars, i)?;
            Ok(())
        }
        other => Err(LexicalError::InvalidJson(format!("unexpected {other}"))),
    }
}

fn scan_object(chars: &[char], i: &mut usize, depth: usize) -> Result<(), LexicalError> {
    *i += 1; // '{'
    let mut keys = HashSet::new();
    skip_ws(chars, i);
    if *i < chars.len() && chars[*i] == '}' {
        *i += 1;
        return Ok(());
    }
    loop {
        skip_ws(chars, i);
        let key = scan_string(chars, i)?;
        if !keys.insert(key.clone()) {
            return Err(LexicalError::DuplicateKey(key));
        }
        skip_ws(chars, i);
        if *i >= chars.len() || chars[*i] != ':' {
            return Err(LexicalError::InvalidJson("expected ':'".to_owned()));
        }
        *i += 1;
        scan_value(chars, i, depth + 1)?;
        skip_ws(chars, i);
        if *i >= chars.len() {
            return Err(LexicalError::InvalidJson("unterminated object".to_owned()));
        }
        match chars[*i] {
            ',' => {
                *i += 1;
                continue;
            }
            '}' => {
                *i += 1;
                return Ok(());
            }
            _ => return Err(LexicalError::InvalidJson("expected ',' or '}'".to_owned())),
        }
    }
}

fn scan_array(chars: &[char], i: &mut usize, depth: usize) -> Result<(), LexicalError> {
    *i += 1; // '['
    skip_ws(chars, i);
    if *i < chars.len() && chars[*i] == ']' {
        *i += 1;
        return Ok(());
    }
    loop {
        scan_value(chars, i, depth + 1)?;
        skip_ws(chars, i);
        if *i >= chars.len() {
            return Err(LexicalError::InvalidJson("unterminated array".to_owned()));
        }
        match chars[*i] {
            ',' => {
                *i += 1;
                continue;
            }
            ']' => {
                *i += 1;
                return Ok(());
            }
            _ => return Err(LexicalError::InvalidJson("expected ',' or ']'".to_owned())),
        }
    }
}

fn scan_string(chars: &[char], i: &mut usize) -> Result<String, LexicalError> {
    if *i >= chars.len() || chars[*i] != '"' {
        return Err(LexicalError::InvalidJson("expected string".to_owned()));
    }
    *i += 1;
    let mut out = String::new();
    while *i < chars.len() {
        let c = chars[*i];
        *i += 1;
        match c {
            '"' => return Ok(out),
            '\\' => out.push(scan_escape(chars, i)?),
            _ => out.push(c),
        }
    }
    Err(LexicalError::InvalidJson("unterminated string".to_owned()))
}

/// Decodes one escape after its backslash, so keys that differ only in how
/// they are escaped (`"a"` and `"\u0061"`) compare equal, as serde sees them.
fn scan_escape(chars: &[char], i: &mut usize) -> Result<char, LexicalError> {
    let bad = || LexicalError::InvalidJson("bad escape".to_owned());
    let c = *chars.get(*i).ok_or_else(bad)?;
    *i += 1;
    Ok(match c {
        '"' | '\\' | '/' => c,
        'b' => '\u{8}',
        'f' => '\u{c}',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'u' => {
            let high = scan_hex4(chars, i).ok_or_else(bad)?;
            let code = if (0xD800..0xDC00).contains(&high) {
                if chars.get(*i) != Some(&'\\') || chars.get(*i + 1) != Some(&'u') {
                    return Err(bad());
                }
                *i += 2;
                let low = scan_hex4(chars, i).ok_or_else(bad)?;
                if !(0xDC00..0xE000).contains(&low) {
                    return Err(bad());
                }
                0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
            } else {
                high
            };
            char::from_u32(code).ok_or_else(bad)?
        }
        _ => return Err(bad()),
    })
}

fn scan_hex4(chars: &[char], i: &mut usize) -> Option<u32> {
    let digits = chars.get(*i..*i + 4)?;
    let mut value = 0;
    for d in digits {
        value = value * 16 + d.to_digit(16)?;
    }
    *i += 4;
    Some(value)
}

fn scan_literal_or_number(chars: &[char], i: &mut usize) -> Result<(), LexicalError> {
    let start = *i;
    if chars[*i] == '-' {
        *i += 1;
    }
    if *i < chars.len() && chars[*i] == 't' {
        expect_ident(chars, i, "true")?;
        return Ok(());
    }
    if *i < chars.len() && chars[*i] == 'f' {
        expect_ident(chars, i, "false")?;
        return Ok(());
    }
    if *i < chars.len() && chars[*i] == 'n' {
        expect_ident(chars, i, "null")?;
        return Ok(());
    }
    while *i < chars.len() && chars[*i].is_ascii_digit() {
        *i += 1;
    }
    if *i < chars.len() && chars[*i] == '.' {
        *i += 1;
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    if *i < chars.len() && (chars[*i] == 'e' || chars[*i] == 'E') {
        *i += 1;
        if *i < chars.len() && (chars[*i] == '+' || chars[*i] == '-') {
            *i += 1;
        }
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    if start == *i {
        return Err(LexicalError::InvalidJson("expected literal".to_owned()));
    }
    Ok(())
}

fn expect_ident(chars: &[char], i: &mut usize, word: &str) -> Result<(), LexicalError> {
    for ch in word.chars() {
        if *i >= chars.len() || chars[*i] != ch {
            return Err(LexicalError::InvalidJson(format!("expected {word}")));
        }
        *i += 1;
    }
    Ok(())
}

/// Reject JSON numbers that appear in contexts where FHIR decimal must not be f64.
/// Policy: any fractional/exponent number token is forbidden unless the value is a JSON string.
/// We approximate by forbidding `:` followed by a number containing `.` or `e` for known decimal-ish keys
/// and also forbidding any number with `.`/`e` adjacent to `"value":` clinical patterns.
fn reject_unsafe_decimals(bytes: &[u8]) -> Result<(), LexicalError> {
    let text = std::str::from_utf8(bytes).map_err(|e| LexicalError::InvalidJson(e.to_string()))?;
    // Fail closed on numeric tokens with fraction/exponent (clinical decimals must be strings).
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            // skip string
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if chars[i] == '-' || chars[i].is_ascii_digit() {
            let start = i;
            if chars[i] == '-' {
                i += 1;
            }
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut unsafe_num = false;
            if i < chars.len() && chars[i] == '.' {
                unsafe_num = true;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                unsafe_num = true;
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if unsafe_num && start < i {
                return Err(LexicalError::UnsafeDecimal);
            }
            continue;
        }
        i += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_key_rejected() {
        let bytes = br#"{"resourceType":"Patient","resourceType":"Observation"}"#;
        assert!(matches!(
            gate_fhir_json("application/fhir+json", bytes, None),
            Err(LexicalError::DuplicateKey(_))
        ));
    }

    #[test]
    fn escaped_duplicate_key_rejected() {
        let bytes = br#"{"resourceType":"Patient","\u0072esourceType":"Observation"}"#;
        assert!(matches!(
            gate_fhir_json("application/fhir+json", bytes, None),
            Err(LexicalError::DuplicateKey(_))
        ));
        let pair = br#"{"resourceType":"Patient","k\ud83d\ude00":1,"k\uD83D\uDE00":2}"#;
        assert!(matches!(
            gate_fhir_json("application/fhir+json", pair, None),
            Err(LexicalError::DuplicateKey(_))
        ));
        let lone = br#"{"resourceType":"Patient","k\ud83d":1}"#;
        assert!(matches!(
            gate_fhir_json("application/fhir+json", lone, None),
            Err(LexicalError::InvalidJson(_))
        ));
    }

    #[test]
    fn escapes_still_decode_in_values() {
        let bytes = br#"{"resourceType":"Patient","id":"a\"b\\c\/d\n\u00e9"}"#;
        assert!(gate_fhir_json("application/fhir+json", bytes, None).is_ok());
    }

    #[test]
    fn deep_nesting_is_refused_without_overflowing_the_stack() {
        let mut bytes = vec![b'['; MAX_INGEST_BYTES - 1];
        bytes.push(b']');
        assert_eq!(
            gate_fhir_json("application/fhir+json", &bytes, None),
            Err(LexicalError::TooDeep)
        );
        let mut objects = String::new();
        for _ in 0..200_000 {
            objects.push_str("{\"a\":");
        }
        assert_eq!(
            gate_fhir_json("application/fhir+json", objects.as_bytes(), None),
            Err(LexicalError::TooDeep)
        );
    }

    #[test]
    fn nesting_at_the_limit_is_admitted() {
        let mut text = String::from("{\"resourceType\":\"Patient\",\"x\":");
        for _ in 0..MAX_JSON_DEPTH - 1 {
            text.push('[');
        }
        for _ in 0..MAX_JSON_DEPTH - 1 {
            text.push(']');
        }
        text.push('}');
        assert!(gate_fhir_json("application/fhir+json", text.as_bytes(), None).is_ok());
    }

    #[test]
    fn fractional_number_rejected() {
        let bytes = br#"{"resourceType":"Observation","valueQuantity":{"value":1.5}}"#;
        assert_eq!(
            gate_fhir_json("application/fhir+json", bytes, None),
            Err(LexicalError::UnsafeDecimal)
        );
    }
}

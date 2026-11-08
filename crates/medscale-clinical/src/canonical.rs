//! Canonical JSON and fingerprints shared by every ported check.
//!
//! Matches the reference package's `_canonical_json` (`json.dumps` with
//! `sort_keys=True`, `separators=(",", ":")`, `ensure_ascii=True`) and
//! `_fingerprint` (`"sha256:" + hex(sha256(canonical_json))`).

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Serializes `value` with sorted keys, no whitespace and only ASCII output.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

/// `sha256:<hex>` of the canonical JSON of `value`.
#[must_use]
pub fn fingerprint(value: &Value) -> String {
    sha256_tag(canonical_json(value).as_bytes())
}

/// `sha256:<hex>` of `bytes`.
#[must_use]
pub fn sha256_tag(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            // Python sorts keys by code point; for UTF-8 strings byte order
            // gives the same result.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_value(&map[key.as_str()], out);
            }
            out.push('}');
        }
    }
}

/// Python's `json.dumps(..., ensure_ascii=True)` string encoding.
fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut units = [0_u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matches_python_json_dumps_sorted_compact_ascii() {
        let value = json!({"b": [1, true, null], "a": "x\"y\\z\n\u{e9}\u{1f600}\u{7f}"});
        assert_eq!(
            canonical_json(&value),
            r#"{"a":"x\"y\\z\n\u00e9\ud83d\ude00\u007f","b":[1,true,null]}"#
        );
    }

    #[test]
    fn fingerprint_is_sha256_of_canonical_json() {
        // python: hashlib.sha256(b"{}").hexdigest()
        assert_eq!(
            fingerprint(&json!({})),
            "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }
}

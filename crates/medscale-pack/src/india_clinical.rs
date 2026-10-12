//! Indian-English clinical code-mixing support for the PII merger (Spec 103),
//! ported from OpenMed v3.0.0 (`maziyarpanahi/openmed@ea920f36`, Apache-2.0):
//! - `openmed/clinical/normalization/india.py`: the abbreviation map and
//!   `normalize_indian_clinical_surface` / `normalize_indian_clinical_entities`;
//! - `openmed/core/pii_entity_merger.py`: `merge_india_code_mixed_spans`,
//!   `_should_bridge_india_spans`, `_india_entity_family`,
//!   `_bridge_india_spans`;
//! - `openmed/core/script_detect.py` `detect_script`, reduced to the scripts
//!   the bridge rule compares (Latin, Devanagari, Telugu; any other letter
//!   counts as another script).
//!
//! Opt-in only (`PiiMergeOptions::india_clinical`). Offsets are Unicode scalar
//! indices; original text is never rewritten, normalized surfaces are added.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use fancy_regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use super::{PiiEntity, normalize_label};

/// OpenMed `INDIAN_CLINICAL_NORMALIZATION_VERSION`.
pub const INDIAN_CLINICAL_NORMALIZATION_VERSION: &str = "india-clinical-abbreviations-v1";

/// OpenMed `INDIAN_CLINICAL_ABBREVIATIONS` (insertion order kept).
pub const INDIAN_CLINICAL_ABBREVIATIONS: &[(&str, &str)] = &[
    ("tab", "tablet"),
    ("cap", "capsule"),
    ("od", "once daily"),
    ("bd", "twice daily"),
    ("bid", "twice daily"),
    ("tds", "three times daily"),
    ("tid", "three times daily"),
    ("hs", "at bedtime"),
    ("sos", "as needed"),
    ("stat", "immediately"),
    ("ac", "before food"),
    ("pc", "after food"),
    ("1-0-1", "morning and evening"),
];

const PERSON_LABELS: &[&str] = &[
    "name",
    "person",
    "patient",
    "per",
    "patient_name",
    "full_name",
];
const LOCATION_LABELS: &[&str] = &["location", "loc", "place", "city"];
/// Maximum gap (in chars) between two bridged spans (OpenMed: 12).
const MAX_BRIDGE_GAP: usize = 12;

/// `_ABBREVIATION_RE`: longest surfaces first, an optional full stop after
/// alphabetic ones, not inside a word.
static ABBREVIATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    let mut surfaces: Vec<&str> = INDIAN_CLINICAL_ABBREVIATIONS
        .iter()
        .map(|(s, _)| *s)
        .collect();
    surfaces.sort_by_key(|s| std::cmp::Reverse(s.len()));
    let alternation = surfaces
        .iter()
        .map(|s| {
            let escaped = fancy_regex::escape(s);
            if s.chars().all(char::is_alphabetic) {
                format!("{escaped}\\.?")
            } else {
                escaped.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("|");
    RegexBuilder::new(&format!(r"(?<!\w)(?:{alternation})(?!\w)"))
        .case_insensitive(true)
        .build()
        .expect("abbreviation pattern compiles")
});

/// Gap allowed between bridged spans: whitespace, optionally around one of
/// `-`, `/`, `|`, or followed by an opening bracket.
static GAP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\s*|\s*[-/|]\s*|\s*[\[(]\s*)$").expect("gap pattern compiles")
});

/// PHI-free provenance of an abbreviation expansion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClinicalNormalization {
    pub locale: String,
    pub version: String,
    pub expansions: Vec<String>,
}

/// Marks an entity built by bridging a Latin/Indic script boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndiaClinicalMerge {
    pub script_boundary_bridge: bool,
    pub transliteration_pair: bool,
}

/// `normalize_indian_clinical_surface`: the expanded surface and the keys of
/// the abbreviations that changed it.
#[must_use]
pub fn normalize_indian_clinical_surface(surface: &str) -> (String, Vec<String>) {
    let mut expansions = Vec::new();
    let mut out = String::with_capacity(surface.len());
    let mut last = 0;
    for found in ABBREVIATION_RE.find_iter(surface) {
        let Ok(found) = found else { break };
        let raw = found.as_str();
        let lower = raw.to_lowercase();
        let key = lower.strip_suffix('.').unwrap_or(&lower);
        out.push_str(&surface[last..found.start()]);
        match INDIAN_CLINICAL_ABBREVIATIONS
            .iter()
            .find(|(s, _)| *s == key)
        {
            Some((s, canonical)) => {
                out.push_str(canonical);
                expansions.push((*s).to_owned());
            }
            None => out.push_str(raw),
        }
        last = found.end();
    }
    out.push_str(&surface[last..]);
    (out, expansions)
}

/// `normalize_indian_clinical_entities`: offsets and `text` stay untouched;
/// changed entities gain `normalized_text` and normalization provenance.
#[must_use]
pub fn normalize_indian_clinical_entities(entities: Vec<PiiEntity>) -> Vec<PiiEntity> {
    entities
        .into_iter()
        .map(|mut entity| {
            let (normalized, expansions) = normalize_indian_clinical_surface(&entity.text);
            if !expansions.is_empty() {
                entity.normalized_text = Some(normalized);
                entity.clinical_normalization = Some(ClinicalNormalization {
                    locale: "en_IN".to_owned(),
                    version: INDIAN_CLINICAL_NORMALIZATION_VERSION.to_owned(),
                    expansions,
                });
            }
            entity
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Script {
    Latin,
    Devanagari,
    Telugu,
    Other,
}

fn script_for_char(c: char) -> Option<Script> {
    let code = u32::from(c);
    match code {
        // Devanagari letters and marks (danda, digits and abbreviation sign excluded).
        0x0900..=0x0963 | 0x0971..=0x097F | 0xA8E0..=0xA8FF | 0x11B00..=0x11B5F => {
            Some(Script::Devanagari)
        }
        0x0964..=0x0970 => None,
        // Telugu letters and marks (digits and fraction signs excluded).
        0x0C00..=0x0C65 => Some(Script::Telugu),
        0x0C66..=0x0C7F => None,
        _ if !c.is_alphabetic() => None,
        0x0041..=0x005A
        | 0x0061..=0x007A
        | 0x00C0..=0x024F
        | 0x0250..=0x02AF
        | 0x1E00..=0x1EFF
        | 0x2C60..=0x2C7F
        | 0xA720..=0xA7FF
        | 0xAB30..=0xAB6F
        | 0xFF21..=0xFF3A
        | 0xFF41..=0xFF5A => Some(Script::Latin),
        _ => Some(Script::Other),
    }
}

/// `detect_script`: the most frequent script, ties to the first seen.
fn detect_script(text: &str) -> Option<Script> {
    let mut counts: Vec<(Script, usize, usize)> = Vec::new();
    for (index, c) in text.chars().enumerate() {
        let Some(script) = script_for_char(c) else {
            continue;
        };
        match counts.iter_mut().find(|(s, _, _)| *s == script) {
            Some(entry) => entry.1 += 1,
            None => counts.push((script, 1, index)),
        }
    }
    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)))
        .map(|(script, _, _)| script)
}

fn entity_family(label: &str) -> Option<&'static str> {
    let normalized = label.to_lowercase().replace("b-", "").replace("i-", "");
    if PERSON_LABELS.contains(&normalized.as_str()) {
        Some("person")
    } else if LOCATION_LABELS.contains(&normalized.as_str()) {
        Some("location")
    } else {
        None
    }
}

fn slice(chars: &[char], start: usize, end: usize) -> String {
    chars
        .get(start.min(chars.len())..end.min(chars.len()))
        .unwrap_or_default()
        .iter()
        .collect()
}

fn should_bridge(left: &PiiEntity, right: &PiiEntity, chars: &[char]) -> bool {
    let (Some(lf), Some(rf)) = (entity_family(&left.label), entity_family(&right.label)) else {
        return false;
    };
    if lf != rf || right.start < left.end || right.start - left.end > MAX_BRIDGE_GAP {
        return false;
    }
    if !GAP_RE
        .is_match(&slice(chars, left.end, right.start))
        .unwrap_or(false)
    {
        return false;
    }
    let scripts: BTreeSet<Option<Script>> = [
        detect_script(&slice(chars, left.start, left.end)),
        detect_script(&slice(chars, right.start, right.end)),
    ]
    .into_iter()
    .collect();
    scripts.contains(&Some(Script::Latin))
        && (scripts.contains(&Some(Script::Devanagari)) || scripts.contains(&Some(Script::Telugu)))
}

fn union(values: impl Iterator<Item = String>) -> Vec<String> {
    values
        .filter(|v| !v.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn bridge(left: &PiiEntity, right: &PiiEntity, chars: &[char]) -> PiiEntity {
    let start = left.start;
    let mut end = right.end;
    let gap = slice(chars, left.end, right.start);
    if (gap.contains('(') && chars.get(end) == Some(&')'))
        || (gap.contains('[') && chars.get(end) == Some(&']'))
    {
        end += 1;
    }
    let labels_of = |e: &PiiEntity| {
        e.source_labels
            .clone()
            .unwrap_or_else(|| vec![e.label.clone()])
    };
    let source_labels = union(labels_of(left).into_iter().chain(labels_of(right)));
    let sources_of = |e: &PiiEntity| {
        e.detector_sources
            .clone()
            .unwrap_or_else(|| vec!["ml".to_owned()])
    };
    let detector_sources = union(sources_of(left).into_iter().chain(sources_of(right)));
    // Python `max` keeps the first of equal scores.
    let label = if right.score > left.score {
        right.label.clone()
    } else {
        left.label.clone()
    };
    let mixed = source_labels
        .iter()
        .map(|l| normalize_label(l))
        .collect::<BTreeSet<_>>()
        .len()
        > 1;
    PiiEntity {
        label,
        score: left.score.max(right.score),
        start,
        end,
        text: slice(chars, start, end),
        merged_from: Some(
            left.merged_from.unwrap_or(1).max(1) + right.merged_from.unwrap_or(1).max(1),
        ),
        source_labels: Some(source_labels),
        mixed_label_union: Some(mixed),
        detector_sources: Some(detector_sources),
        normalized_text: None,
        clinical_normalization: None,
        india_clinical_merge: Some(IndiaClinicalMerge {
            script_boundary_bridge: true,
            transliteration_pair: true,
        }),
    }
}

/// `merge_india_code_mixed_spans`: unions adjacent same-family PERSON or
/// LOCATION spans only when one is Latin and the other Devanagari or Telugu,
/// then normalizes clinical abbreviations.
#[must_use]
pub fn merge_india_code_mixed_spans(mut entities: Vec<PiiEntity>, text: &str) -> Vec<PiiEntity> {
    if entities.is_empty() {
        return entities;
    }
    let chars: Vec<char> = text.chars().collect();
    entities.sort_by(super::entity_order);
    let mut merged = Vec::with_capacity(entities.len());
    let mut iter = entities.into_iter();
    let mut current = iter.next().expect("non-empty");
    for candidate in iter {
        if should_bridge(&current, &candidate, &chars) {
            current = bridge(&current, &candidate, &chars);
        } else {
            merged.push(std::mem::replace(&mut current, candidate));
        }
    }
    merged.push(current);
    normalize_indian_clinical_entities(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(label: &str, score: f64, start: usize, end: usize, text: &str) -> PiiEntity {
        PiiEntity::model(label, score, start, end, text)
    }

    #[test]
    fn abbreviations_expand_like_openmed() {
        let (n, e) = normalize_indian_clinical_surface("Tab. Paracetamol 500mg BD, 1-0-1 x 5 days");
        assert_eq!(
            n,
            "tablet Paracetamol 500mg twice daily, morning and evening x 5 days"
        );
        assert_eq!(e, ["tab", "bd", "1-0-1"]);
        // Inside words or unknown: untouched.
        let (n, e) = normalize_indian_clinical_surface("tablet capsules odds");
        assert_eq!(n, "tablet capsules odds");
        assert!(e.is_empty());
    }

    #[test]
    fn scripts_are_detected_by_majority() {
        assert_eq!(detect_script("राहुल"), Some(Script::Devanagari));
        assert_eq!(detect_script("రవి"), Some(Script::Telugu));
        assert_eq!(detect_script("Sharma"), Some(Script::Latin));
        assert_eq!(detect_script("12 ."), None);
        assert_eq!(detect_script("Ελένη"), Some(Script::Other));
    }

    #[test]
    fn latin_and_indic_name_fragments_bridge_across_a_script_change() {
        let text = "Patient राहुल Sharma admitted";
        let s = text.chars().position(|c| c == 'र').unwrap();
        let sh = text.chars().position(|c| c == 'S').unwrap();
        let merged = merge_india_code_mixed_spans(
            vec![
                ent("B-PERSON", 0.8, s, s + 5, text),
                ent("PERSON", 0.9, sh, sh + 6, text),
            ],
            text,
        );
        assert_eq!(merged.len(), 1, "{merged:?}");
        let m = &merged[0];
        assert_eq!(m.text, "राहुल Sharma");
        assert_eq!(m.label, "PERSON", "the higher-scoring label wins");
        assert_eq!(m.merged_from, Some(2));
        assert_eq!(
            m.india_clinical_merge,
            Some(IndiaClinicalMerge {
                script_boundary_bridge: true,
                transliteration_pair: true
            })
        );
    }

    #[test]
    fn parenthesized_transliterations_include_the_closing_bracket() {
        let text = "Lives in Hyderabad (హైదరాబాద్) now";
        let h = 9;
        let t = text.chars().position(|c| c == 'హ').unwrap();
        let tl = text.chars().skip(t).take_while(|c| *c != ')').count();
        let merged = merge_india_code_mixed_spans(
            vec![
                ent("LOC", 0.7, h, h + 9, text),
                ent("location", 0.6, t, t + tl, text),
            ],
            text,
        );
        assert_eq!(merged.len(), 1, "{merged:?}");
        assert_eq!(merged[0].text, "Hyderabad (హైదరాబాద్)");
        assert_eq!(merged[0].label, "LOC");
    }

    #[test]
    fn same_script_wide_gaps_and_other_families_are_not_bridged() {
        // Two Latin names: no script change.
        let text = "John Smith";
        let kept = merge_india_code_mixed_spans(
            vec![
                ent("PERSON", 0.9, 0, 4, text),
                ent("PERSON", 0.9, 5, 10, text),
            ],
            text,
        );
        assert_eq!(kept.len(), 2);
        // Different families.
        let text = "राहुल Delhi";
        let kept = merge_india_code_mixed_spans(
            vec![ent("PERSON", 0.9, 0, 5, text), ent("LOC", 0.9, 6, 11, text)],
            text,
        );
        assert_eq!(kept.len(), 2);
        // A gap that is not whitespace or a separator.
        let text = "राहुल and Sharma";
        let kept = merge_india_code_mixed_spans(
            vec![
                ent("PERSON", 0.9, 0, 5, text),
                ent("PERSON", 0.9, 10, 16, text),
            ],
            text,
        );
        assert_eq!(kept.len(), 2);
        // Medication entities gain normalized surfaces; offsets unchanged.
        let text = "Tab Dolo BD";
        let drug = merge_india_code_mixed_spans(vec![ent("DRUG", 0.9, 0, 11, text)], text);
        assert_eq!(drug[0].text, "Tab Dolo BD");
        assert_eq!(
            drug[0].normalized_text.as_deref(),
            Some("tablet Dolo twice daily")
        );
        assert_eq!(
            drug[0].clinical_normalization.as_ref().unwrap().expansions,
            ["tab", "bd"]
        );
    }
}

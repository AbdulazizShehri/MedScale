//! PII entity merging ported from OpenMed v3.0.0
//! (`openmed/core/pii_entity_merger.py`, `ea920f36`, Apache-2.0).
//!
//! Tokenizers split semantic units such as `01/15/1970` into fragments that a
//! model may label differently (`date` + `date_of_birth`). The merger finds
//! regex semantic units (dates, SSNs, phones, IBANs, ...) with context-aware
//! scores and checksum validators, takes the dominant model label over each
//! unit, blends confidences, and unions spans so no text the model already
//! flagged is dropped. The Indian code-mixing extension (`india_clinical`)
//! is not ported.
//!
//! Offsets are Unicode scalar (code point) indices, as in OpenMed and in the
//! document runtimes' `"offsets": "unicode_scalar"` payloads.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use fancy_regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

#[path = "pii_patterns.rs"]
mod patterns;

/// Characters OpenMed searches on each side of a match for context words.
const CONTEXT_WINDOW_CHARS: usize = 100;
/// Backtracking ceiling per match attempt; exceeding it counts as no match.
const BACKTRACK_LIMIT: usize = 100_000;

/// One `PIIPattern` row (see `pii_patterns.rs`).
pub(crate) struct RawPiiPattern {
    pub pattern: &'static str,
    pub entity_type: &'static str,
    pub priority: i32,
    pub ignore_case: bool,
    pub base_score: f64,
    pub context_boost: f64,
    pub context_words: &'static [&'static str],
    pub requires_context: bool,
    pub context_required: bool,
    pub reject_on_validation_failure: bool,
    pub validator: Option<fn(&str) -> bool>,
}

struct CompiledPattern {
    raw: &'static RawPiiPattern,
    regex: Regex,
}

/// Patterns sorted by priority, highest first (stable, like Python `sorted`).
static COMPILED: LazyLock<Vec<CompiledPattern>> = LazyLock::new(|| {
    let mut compiled: Vec<CompiledPattern> = patterns::PII_PATTERNS
        .iter()
        .map(|raw| CompiledPattern {
            raw,
            regex: RegexBuilder::new(raw.pattern)
                .case_insensitive(raw.ignore_case)
                .backtrack_limit(BACKTRACK_LIMIT)
                .build()
                .expect("generated PII patterns compile"),
        })
        .collect();
    compiled.sort_by_key(|c| std::cmp::Reverse(c.raw.priority));
    compiled
});

/// A model entity in, or a merged entity out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PiiEntity {
    pub label: String,
    pub score: f64,
    pub start: usize,
    pub end: usize,
    pub text: String,
    /// Number of model entities folded in (0: regex-only). `None` for a model
    /// entity passed through unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged_from: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_labels: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixed_label_union: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector_sources: Option<Vec<String>>,
}

impl PiiEntity {
    /// A raw model prediction over scalar offsets `start..end` of `text`.
    pub fn model(label: &str, score: f64, start: usize, end: usize, text: &str) -> Self {
        Self {
            label: label.to_owned(),
            score,
            start,
            end,
            text: text
                .chars()
                .skip(start)
                .take(end.saturating_sub(start))
                .collect(),
            merged_from: None,
            source_labels: None,
            mixed_label_union: None,
            detector_sources: None,
        }
    }
}

/// `merge_entities_with_semantic_units` options (OpenMed defaults).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PiiMergeOptions {
    pub use_semantic_patterns: bool,
    pub prefer_model_labels: bool,
    pub allow_semantic_only_matches: bool,
    pub allow_label_expansion: bool,
}

impl Default for PiiMergeOptions {
    fn default() -> Self {
        Self {
            use_semantic_patterns: true,
            prefer_model_labels: false,
            allow_semantic_only_matches: true,
            allow_label_expansion: true,
        }
    }
}

/// A regex semantic unit (`find_semantic_units`).
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticUnit {
    pub start: usize,
    pub end: usize,
    pub entity_type: &'static str,
    pub score: f64,
    pub validated: bool,
}

/// Byte offset of every char boundary, for scalar-index slicing.
struct CharMap {
    bytes: Vec<usize>,
}

impl CharMap {
    fn new(text: &str) -> Self {
        let mut bytes: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        bytes.push(text.len());
        Self { bytes }
    }

    fn len(&self) -> usize {
        self.bytes.len() - 1
    }

    fn slice<'t>(&self, text: &'t str, start: usize, end: usize) -> &'t str {
        &text[self.bytes[start]..self.bytes[end]]
    }

    fn scalar(&self, byte: usize) -> usize {
        self.bytes.binary_search(&byte).unwrap_or_else(|i| i)
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `find_context_words`: is any context word within 100 characters of the match?
fn find_context_words(
    text: &str,
    map: &CharMap,
    start: usize,
    end: usize,
    words: &[&str],
    require_boundaries: bool,
) -> bool {
    if words.is_empty() {
        return false;
    }
    let window_start = start.saturating_sub(CONTEXT_WINDOW_CHARS);
    let window_end = (end + CONTEXT_WINDOW_CHARS).min(map.len());
    let context = map.slice(text, window_start, window_end).to_lowercase();
    words.iter().any(|word| {
        let word = word.to_lowercase();
        if word.is_empty() {
            return false;
        }
        if !require_boundaries && context.contains(&word) {
            return true;
        }
        // Python `\b word \b`: a boundary sits between a word and a non-word
        // character, so it depends on the phrase's own first and last chars.
        let first_word = word.chars().next().is_some_and(is_word_char);
        let last_word = word.chars().next_back().is_some_and(is_word_char);
        context.match_indices(&word).any(|(i, m)| {
            let before = context[..i].chars().next_back().is_some_and(is_word_char);
            let after = context[i + m.len()..]
                .chars()
                .next()
                .is_some_and(is_word_char);
            before != first_word && after != last_word
        })
    })
}

/// `find_semantic_units` over every bundled pattern.
pub fn find_semantic_units(text: &str) -> Vec<SemanticUnit> {
    semantic_units(text, &CharMap::new(text))
}

fn semantic_units(text: &str, map: &CharMap) -> Vec<SemanticUnit> {
    let mut units: Vec<SemanticUnit> = Vec::new();
    for compiled in COMPILED.iter() {
        let raw = compiled.raw;
        for found in compiled.regex.find_iter(text) {
            let Ok(found) = found else {
                break; // backtrack limit: no further matches from this pattern
            };
            let (start, end) = (map.scalar(found.start()), map.scalar(found.end()));
            if start >= end || units.iter().any(|u| start < u.end && end > u.start) {
                continue;
            }
            let has_context = !raw.context_words.is_empty()
                && find_context_words(text, map, start, end, raw.context_words, false);
            if raw.requires_context
                && !find_context_words(text, map, start, end, raw.context_words, true)
            {
                continue;
            }
            let mut score = raw.base_score;
            if has_context {
                score = (score + raw.context_boost).min(1.0);
            }
            if raw.context_required && !has_context {
                continue;
            }
            let mut validated = true;
            if let Some(validator) = raw.validator
                && !validator(found.as_str())
            {
                if raw.reject_on_validation_failure {
                    continue;
                }
                score *= 0.3;
                validated = false;
            }
            units.push(SemanticUnit {
                start,
                end,
                entity_type: raw.entity_type,
                score,
                validated,
            });
        }
    }
    units.sort_by_key(|u| u.start);
    units
}

fn entity_order(a: &PiiEntity, b: &PiiEntity) -> Ordering {
    a.start
        .cmp(&b.start)
        .then(a.end.cmp(&b.end))
        .then_with(|| a.label.cmp(&b.label))
        .then_with(|| b.score.total_cmp(&a.score))
        .then_with(|| a.text.cmp(&b.text))
}

/// `calculate_dominant_label` (tie-break by confidence): the most frequent
/// label, ties broken by higher mean score then label; and the mean score.
fn dominant_label(entities: &[&PiiEntity]) -> (String, f64) {
    let mut per_label: BTreeMap<&str, (usize, f64)> = BTreeMap::new();
    for e in entities {
        let slot = per_label.entry(e.label.as_str()).or_default();
        slot.0 += 1;
        slot.1 += e.score;
    }
    let max_count = per_label.values().map(|(n, _)| *n).max().unwrap_or(0);
    let label = per_label
        .iter()
        .filter(|(_, (n, _))| *n == max_count)
        .min_by(|(la, (na, sa)), (lb, (nb, sb))| {
            (sb / *nb as f64)
                .total_cmp(&(sa / *na as f64))
                .then_with(|| la.cmp(lb))
        })
        .map(|(label, _)| (*label).to_owned())
        .unwrap_or_default();
    let mean = entities.iter().map(|e| e.score).sum::<f64>() / entities.len() as f64;
    (label, mean)
}

/// `normalize_label`.
pub fn normalize_label(label: &str) -> String {
    let lower = label.to_lowercase();
    if lower.contains("date") {
        return "date".into();
    }
    if lower.contains("phone") || lower.contains("fax") {
        return "phone".into();
    }
    if lower.contains("address") {
        return "address".into();
    }
    let mapped = match lower.as_str() {
        "ssn" | "social_security" | "social_security_number" => "ssn",
        "national_id" | "nir" | "insee" | "steuer_id" | "steuernummer" | "codice_fiscale"
        | "bsn" | "dni" | "nie" | "aadhaar" | "cpf" | "cnpj" | "teudat_zehut" | "teudatzehut"
        | "tz" => "national_id",
        "postcode" | "zipcode" | "zip" | "postal_code" => "postcode",
        "medical_record_number" | "mrn" | "medical_record" => "medical_record",
        "account_number" | "account" => "account",
        "credit_debit_card" | "credit_card" | "debit_card" | "payment_card" => "payment_card",
        _ => return lower,
    };
    mapped.into()
}

/// `is_more_specific`: is `label1` a more specific form of `label2`?
pub fn is_more_specific(label1: &str, label2: &str) -> bool {
    let l1 = label1.to_lowercase();
    let l2 = label2.to_lowercase();
    if l2 != l1 && l1.contains(&l2) {
        return true;
    }
    const HIERARCHY: &[(&str, &[&str])] = &[
        ("date", &["date_of_birth", "date_time"]),
        ("name", &["first_name", "last_name", "full_name"]),
        ("phone", &["phone_number", "fax_number", "mobile_number"]),
        (
            "address",
            &["street_address", "home_address", "billing_address"],
        ),
        (
            "id",
            &[
                "ssn",
                "medical_record_number",
                "account_number",
                "employee_id",
            ],
        ),
        (
            "national_id",
            &[
                "nir",
                "insee",
                "steuer_id",
                "steuernummer",
                "codice_fiscale",
                "cpf",
                "cnpj",
                "teudat_zehut",
            ],
        ),
    ];
    let general = normalize_label(label2);
    HIERARCHY
        .iter()
        .any(|(g, specific)| general == *g && specific.contains(&l1.as_str()))
}

fn label_union_is_mixed(labels: &[String]) -> bool {
    labels
        .iter()
        .map(|l| normalize_label(l))
        .collect::<BTreeSet<_>>()
        .len()
        > 1
}

/// `merge_entities_with_semantic_units` (without the `india_clinical` path).
pub fn merge_entities_with_semantic_units(
    entities: &[PiiEntity],
    text: &str,
    options: PiiMergeOptions,
) -> Vec<PiiEntity> {
    let map = CharMap::new(text);
    let mut entities = entities
        .iter()
        .filter(|e| e.start < e.end && e.end <= map.len())
        .cloned()
        .collect::<Vec<_>>();
    entities.sort_by(entity_order);
    if !options.use_semantic_patterns {
        return entities;
    }
    let units = semantic_units(text, &map);
    if units.is_empty() {
        return entities;
    }
    let mut merged = Vec::new();
    let mut used = vec![false; entities.len()];
    for unit in &units {
        let overlapping: Vec<usize> = (0..entities.len())
            .filter(|&i| entities[i].start < unit.end && entities[i].end > unit.start)
            .collect();
        if overlapping.is_empty() {
            if options.allow_semantic_only_matches {
                merged.push(PiiEntity {
                    label: unit.entity_type.to_owned(),
                    score: unit.score,
                    start: unit.start,
                    end: unit.end,
                    text: map.slice(text, unit.start, unit.end).to_owned(),
                    merged_from: Some(0),
                    source_labels: Some(vec![unit.entity_type.to_owned()]),
                    mixed_label_union: Some(false),
                    detector_sources: Some(vec!["locale_rule".to_owned()]),
                });
            }
            continue;
        }
        let group: Vec<&PiiEntity> = overlapping.iter().map(|&i| &entities[i]).collect();
        let (dominant, model_mean) = dominant_label(&group);
        let unit_norm = normalize_label(unit.entity_type);
        let label = if !options.allow_label_expansion {
            dominant
        } else if options.prefer_model_labels {
            if normalize_label(&dominant) == unit_norm
                || is_more_specific(&dominant, unit.entity_type)
            {
                dominant
            } else {
                unit.entity_type.to_owned()
            }
        } else if group.iter().any(|e| normalize_label(&e.label) == unit_norm)
            || is_more_specific(&dominant, unit.entity_type)
        {
            dominant
        } else {
            unit.entity_type.to_owned()
        };
        let source_labels: Vec<String> = std::iter::once(unit.entity_type.to_owned())
            .chain(group.iter().map(|e| e.label.clone()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let score = if unit.validated {
            0.6 * model_mean + 0.4 * unit.score
        } else {
            0.9 * model_mean + 0.1 * unit.score
        };
        let start = group.iter().map(|e| e.start).fold(unit.start, usize::min);
        let end = group.iter().map(|e| e.end).fold(unit.end, usize::max);
        merged.push(PiiEntity {
            label,
            score,
            start,
            end,
            text: map.slice(text, start, end).to_owned(),
            merged_from: Some(group.len()),
            mixed_label_union: Some(label_union_is_mixed(&source_labels)),
            source_labels: Some(source_labels),
            detector_sources: Some(vec!["ml".to_owned(), "locale_rule".to_owned()]),
        });
        for i in overlapping {
            used[i] = true;
        }
    }
    merged.extend(
        entities
            .into_iter()
            .zip(used)
            .filter(|(_, used)| !used)
            .map(|(e, _)| e),
    );
    coalesce_overlapping(merged, text, &map)
}

/// Merge the document runtimes' decoded entities.
pub(crate) fn merge_decoded(
    entities: &[crate::token_windows::DecodedEntity],
    text: &str,
) -> Vec<PiiEntity> {
    let entities: Vec<PiiEntity> = entities
        .iter()
        .map(|e| PiiEntity {
            label: e.label.clone(),
            score: f64::from(e.score),
            start: e.start,
            end: e.end,
            text: e.text.clone(),
            merged_from: None,
            source_labels: None,
            mixed_label_union: None,
            detector_sources: None,
        })
        .collect();
    merge_entities_with_semantic_units(&entities, text, PiiMergeOptions::default())
}

/// Add `pii_merged_entities` to a document payload when a merge was requested,
/// and `aggregation` when a non-default entity aggregation was used (default
/// payloads stay byte-identical).
pub(crate) fn with_merged(
    mut payload: serde_json::Value,
    merged: Option<Vec<PiiEntity>>,
    aggregation: Option<crate::token_windows::EntityAggregation>,
) -> serde_json::Value {
    if let (Some(aggregation), Some(object)) = (aggregation, payload.as_object_mut()) {
        object.insert(
            "aggregation".to_owned(),
            serde_json::to_value(aggregation).unwrap_or_default(),
        );
    }
    if let (Some(merged), Some(object)) = (merged, payload.as_object_mut()) {
        object.insert(
            "pii_merged_entities".to_owned(),
            serde_json::to_value(merged).unwrap_or_default(),
        );
    }
    payload
}

/// `_coalesce_overlapping_merged_entities`: union transitive overlaps.
fn coalesce_overlapping(mut entities: Vec<PiiEntity>, text: &str, map: &CharMap) -> Vec<PiiEntity> {
    if entities.is_empty() {
        return entities;
    }
    entities.sort_by(entity_order);
    let mut clusters: Vec<Vec<PiiEntity>> = Vec::new();
    let mut current_end = 0;
    for entity in entities {
        match clusters.last_mut() {
            Some(cluster) if entity.start < current_end => {
                current_end = current_end.max(entity.end);
                cluster.push(entity);
            }
            _ => {
                current_end = entity.end;
                clusters.push(vec![entity]);
            }
        }
    }
    clusters
        .into_iter()
        .map(|mut cluster| {
            if cluster.len() == 1 {
                return cluster.remove(0);
            }
            let start = cluster.iter().map(|e| e.start).min().unwrap_or(0);
            let end = cluster.iter().map(|e| e.end).max().unwrap_or(0);
            // Python `max` keeps the first of equal keys.
            let winner = cluster
                .iter()
                .reduce(|best, e| {
                    let key = |x: &PiiEntity| (x.end - x.start, x.score, x.label.clone());
                    let (kb, ke) = (key(best), key(e));
                    let better =
                        ke.0.cmp(&kb.0)
                            .then(ke.1.total_cmp(&kb.1))
                            .then_with(|| ke.2.cmp(&kb.2));
                    if better == Ordering::Greater { e } else { best }
                })
                .map(|e| e.label.clone())
                .unwrap_or_default();
            let source_labels: Vec<String> = cluster
                .iter()
                .flat_map(|e| {
                    e.source_labels
                        .clone()
                        .unwrap_or_else(|| vec![e.label.clone()])
                })
                .filter(|l| !l.is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let detector_sources: Vec<String> = cluster
                .iter()
                .flat_map(|e| {
                    e.detector_sources
                        .clone()
                        .unwrap_or_else(|| vec!["ml".to_owned()])
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            PiiEntity {
                label: winner,
                score: cluster.iter().map(|e| e.score).fold(f64::MIN, f64::max),
                start,
                end,
                text: map.slice(text, start, end).to_owned(),
                merged_from: Some(
                    cluster
                        .iter()
                        .map(|e| e.merged_from.unwrap_or(1).max(1))
                        .sum(),
                ),
                mixed_label_union: Some(label_union_is_mixed(&source_labels)),
                source_labels: Some(source_labels),
                detector_sources: Some(detector_sources),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn every_generated_pattern_compiles() {
        assert_eq!(COMPILED.len(), patterns::PII_PATTERNS.len());
        assert_eq!(patterns::BASE_PATTERN_COUNT, 24);
        assert_eq!(patterns::PII_PATTERNS.len(), 27);
    }

    #[test]
    fn fragmented_date_of_birth_is_merged_like_openmed() {
        // OpenMed docstring example: "01" (date) + "/15/1970" (date_of_birth).
        let text = "DOB: 01/15/1970";
        let merged = merge_entities_with_semantic_units(
            &[
                PiiEntity::model("date", 0.7, 5, 7, text),
                PiiEntity::model("date_of_birth", 0.9, 7, 15, text),
            ],
            text,
            PiiMergeOptions::default(),
        );
        assert_eq!(merged.len(), 1);
        let m = &merged[0];
        assert_eq!((m.label.as_str(), m.start, m.end), ("date_of_birth", 5, 15));
        assert_eq!(m.text, "01/15/1970");
        // Unit: base 0.6 + "dob" context 0.3 = 0.9; model mean 0.8.
        assert!(approx(m.score, 0.6 * 0.8 + 0.4 * 0.9), "{}", m.score);
        assert_eq!(m.merged_from, Some(2));
        assert_eq!(
            m.source_labels.as_deref(),
            Some(&["date".to_owned(), "date_of_birth".to_owned()][..])
        );
        assert_eq!(m.mixed_label_union, Some(false));
    }

    #[test]
    fn semantic_units_score_context_and_validators() {
        let text = "Patient SSN: 123-45-6789, alt 000-12-3456. Card 4111 1111 1111 1111.";
        let units = find_semantic_units(text);
        let ssn = units
            .iter()
            .find(|u| &text[u.start..u.end] == "123-45-6789")
            .unwrap();
        assert_eq!(ssn.entity_type, "ssn");
        assert!(ssn.validated);
        let bad = units
            .iter()
            .find(|u| &text[u.start..u.end] == "000-12-3456")
            .unwrap();
        assert!(!bad.validated, "000 area fails validate_ssn");
        assert!(bad.score < ssn.score);
        let card = units
            .iter()
            .find(|u| text[u.start..u.end].starts_with("4111"))
            .unwrap();
        assert!(card.validated);
    }

    #[test]
    fn regex_only_units_are_added_and_model_text_is_never_shrunk() {
        let text = "Contact jane.doe@example.org about MRN: 12345678 today.";
        // The model flags a wider span than the MRN unit.
        let model = PiiEntity::model("medical_record_number", 0.8, 31, 48, text);
        let merged = merge_entities_with_semantic_units(&[model], text, PiiMergeOptions::default());
        let email = merged.iter().find(|e| e.label == "email").unwrap();
        assert_eq!(email.text, "jane.doe@example.org");
        assert_eq!(email.merged_from, Some(0));
        let mrn = merged
            .iter()
            .find(|e| e.label == "medical_record_number")
            .unwrap();
        assert_eq!((mrn.start, mrn.end), (31, 48), "{mrn:?}");
    }

    #[test]
    fn options_switch_off_patterns_and_regex_only_matches() {
        let text = "Email jane@example.org";
        let off = PiiMergeOptions {
            use_semantic_patterns: false,
            ..PiiMergeOptions::default()
        };
        assert!(merge_entities_with_semantic_units(&[], text, off).is_empty());
        let no_regex_only = PiiMergeOptions {
            allow_semantic_only_matches: false,
            ..PiiMergeOptions::default()
        };
        assert!(merge_entities_with_semantic_units(&[], text, no_regex_only).is_empty());
        assert_eq!(
            merge_entities_with_semantic_units(&[], text, PiiMergeOptions::default()).len(),
            1
        );
    }

    #[test]
    fn labels_normalize_and_rank_like_openmed() {
        assert_eq!(normalize_label("date_of_birth"), "date");
        assert_eq!(normalize_label("phone_number"), "phone");
        assert_eq!(normalize_label("BSN"), "national_id");
        assert_eq!(normalize_label("email"), "email");
        assert!(is_more_specific("date_of_birth", "date"));
        assert!(!is_more_specific("date", "date_of_birth"));
        assert!(is_more_specific("first_name", "name"));
        assert!(is_more_specific("ssn", "id"));
    }

    #[test]
    fn context_words_respect_boundaries_when_required() {
        // "ssn" occurs inside "classnote" but not as a word.
        let text = "classnote 123";
        let map = CharMap::new(text);
        assert!(find_context_words(text, &map, 10, 13, &["ssn"], false));
        assert!(!find_context_words(text, &map, 10, 13, &["ssn"], true));
        let text = "my ssn 123";
        assert!(find_context_words(
            text,
            &CharMap::new(text),
            7,
            10,
            &["ssn"],
            true
        ));
    }

    #[test]
    fn offsets_are_unicode_scalars() {
        let text = "Née à Zürich, DOB: 01/15/1970";
        let start = text.chars().position(|c| c == '0').unwrap();
        let merged = merge_entities_with_semantic_units(
            &[PiiEntity::model(
                "date_of_birth",
                0.9,
                start,
                start + 10,
                text,
            )],
            text,
            PiiMergeOptions::default(),
        );
        let dob = merged.iter().find(|e| e.label == "date_of_birth").unwrap();
        assert_eq!((dob.start, dob.end), (start, start + 10));
        assert_eq!(dob.text, "01/15/1970");
    }

    #[test]
    fn units_absorb_fragments_and_overlapping_merges_coalesce() {
        // Two model fragments of one phone unit become one entity.
        let text = "Call 415-555-2671 now";
        let merged = merge_entities_with_semantic_units(
            &[
                PiiEntity::model("phone_number", 0.9, 5, 12, text),
                PiiEntity::model("phone_number", 0.6, 9, 17, text),
            ],
            text,
            PiiMergeOptions::default(),
        );
        assert_eq!(merged.len(), 1, "{merged:?}");
        assert_eq!((merged[0].start, merged[0].end), (5, 17));
        assert_eq!(merged[0].merged_from, Some(2));

        // One wide model span covering two units: both merges widen to it and
        // are coalesced into a single union.
        let text = "x 01/02/1990 and 123-45-6789 y";
        let merged = merge_entities_with_semantic_units(
            &[PiiEntity::model("id", 0.8, 2, 28, text)],
            text,
            PiiMergeOptions::default(),
        );
        assert_eq!(merged.len(), 1, "{merged:?}");
        assert_eq!((merged[0].start, merged[0].end), (2, 28));
        assert_eq!(merged[0].merged_from, Some(2));
        assert_eq!(merged[0].mixed_label_union, Some(true));
    }

    #[test]
    fn without_units_entities_pass_through_unmerged() {
        // OpenMed returns model entities as-is when no semantic unit matches.
        let text = "Call (415) 555-2671 now";
        let entities = [
            PiiEntity::model("phone_number", 0.9, 5, 14, text),
            PiiEntity::model("phone_number", 0.6, 11, 19, text),
        ];
        let merged =
            merge_entities_with_semantic_units(&entities, text, PiiMergeOptions::default());
        assert_eq!(merged, entities.to_vec());
    }
}

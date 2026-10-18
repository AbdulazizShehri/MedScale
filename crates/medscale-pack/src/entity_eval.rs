//! Spec 103: deterministic entity-level accuracy evaluation.
//!
//! Scores a model's decoded entities against a gold set with exact,
//! reproducible rules, so qualification can report how well a model tags
//! text, not only that it runs.
//!
//! - Offsets are Unicode scalar values, as every MedScale runtime emits.
//! - Model labels are mapped to the gold set's canonical labels through the
//!   set's label map (case-insensitive; `B-`/`I-` prefixes ignored). A
//!   prediction whose label has no mapping is not scored and is reported in
//!   `unmapped_labels`, because model label sets are larger than any gold set.
//! - **Strict** match: same canonical label and identical span. Matching is
//!   one-to-one; a gold span can be matched once.
//! - **Overlap** match: same canonical label and any overlapping span, also
//!   one-to-one (a lenient view for span-boundary differences).
//!
//! The result is evaluation evidence for a fixed gold set. It is not clinical
//! validation and says nothing about real patient text.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvalSetError {
    #[error("evaluation set is not valid JSON: {0}")]
    Json(String),
    #[error("document {doc}: span {start}..{end} is outside the text")]
    SpanOutOfRange {
        doc: String,
        start: usize,
        end: usize,
    },
    #[error("document {doc}: span {start}..{end} text {found:?} does not match {expected:?}")]
    SpanTextMismatch {
        doc: String,
        start: usize,
        end: usize,
        expected: String,
        found: String,
    },
    #[error("document {doc}: label {label:?} is not one of the set's canonical labels")]
    UnknownLabel { doc: String, label: String },
    #[error("duplicate document id {0}")]
    DuplicateDocument(String),
}

/// One annotated span; `text` must equal the span's slice of the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldSpan {
    pub start: usize,
    pub end: usize,
    pub label: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldDocument {
    pub id: String,
    pub language: String,
    pub text: String,
    pub spans: Vec<GoldSpan>,
}

/// A versioned gold set with its canonical labels and model label map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalSet {
    pub name: String,
    pub version: String,
    /// Provenance and terms, for example "synthetic, authored for MedScale".
    pub provenance: String,
    pub license: String,
    pub labels: Vec<String>,
    /// Model label (lowercase, no B-/I- prefix) to canonical label.
    pub label_map: BTreeMap<String, String>,
    pub documents: Vec<GoldDocument>,
}

fn slice_chars(text: &str, start: usize, end: usize) -> Option<String> {
    if start > end {
        return None;
    }
    let count = text.chars().count();
    (end <= count).then(|| text.chars().skip(start).take(end - start).collect())
}

impl EvalSet {
    /// Parses and validates a set: every span lies inside its text, carries
    /// the exact text it covers and a canonical label; document ids are
    /// unique; label-map targets are canonical labels.
    pub fn from_json(bytes: &[u8]) -> Result<Self, EvalSetError> {
        let set: EvalSet =
            serde_json::from_slice(bytes).map_err(|e| EvalSetError::Json(e.to_string()))?;
        let labels: BTreeSet<&str> = set.labels.iter().map(String::as_str).collect();
        for (from, to) in &set.label_map {
            if !labels.contains(to.as_str()) {
                return Err(EvalSetError::UnknownLabel {
                    doc: format!("label_map[{from}]"),
                    label: to.clone(),
                });
            }
        }
        let mut ids = BTreeSet::new();
        for doc in &set.documents {
            if !ids.insert(doc.id.as_str()) {
                return Err(EvalSetError::DuplicateDocument(doc.id.clone()));
            }
            for span in &doc.spans {
                let found = slice_chars(&doc.text, span.start, span.end).ok_or(
                    EvalSetError::SpanOutOfRange {
                        doc: doc.id.clone(),
                        start: span.start,
                        end: span.end,
                    },
                )?;
                if found != span.text {
                    return Err(EvalSetError::SpanTextMismatch {
                        doc: doc.id.clone(),
                        start: span.start,
                        end: span.end,
                        expected: span.text.clone(),
                        found,
                    });
                }
                if !labels.contains(span.label.as_str()) {
                    return Err(EvalSetError::UnknownLabel {
                        doc: doc.id.clone(),
                        label: span.label.clone(),
                    });
                }
            }
        }
        Ok(set)
    }

    /// Canonical label for a model label, if the set maps it.
    #[must_use]
    pub fn canonical(&self, model_label: &str) -> Option<&str> {
        let trimmed = model_label.trim();
        let bare = trimmed
            .strip_prefix("B-")
            .or_else(|| trimmed.strip_prefix("I-"))
            .unwrap_or(trimmed)
            .to_ascii_lowercase();
        self.label_map.get(&bare).map(String::as_str)
    }
}

/// A predicted span after label mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictedSpan {
    pub start: usize,
    pub end: usize,
    pub model_label: String,
}

/// Reads `{label, start, end}` entities from a runtime proposal payload
/// (`entities` array, Unicode-scalar offsets).
#[must_use]
pub fn predicted_spans(payload: &Value) -> Vec<PredictedSpan> {
    predicted_spans_at(payload, "entities")
}

/// [`predicted_spans`] from another payload array, for example
/// `pii_merged_entities` (the OpenMed PII entity merger's output).
#[must_use]
pub fn predicted_spans_at(payload: &Value, field: &str) -> Vec<PredictedSpan> {
    payload[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            Some(PredictedSpan {
                start: usize::try_from(e["start"].as_u64()?).ok()?,
                end: usize::try_from(e["end"].as_u64()?).ok()?,
                model_label: e["label"].as_str()?.to_owned(),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Counts {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

impl Counts {
    fn finish(mut self) -> Self {
        let tp = self.true_positives as f64;
        let predicted = (self.true_positives + self.false_positives) as f64;
        let gold = (self.true_positives + self.false_negatives) as f64;
        self.precision = if predicted > 0.0 { tp / predicted } else { 0.0 };
        self.recall = if gold > 0.0 { tp / gold } else { 0.0 };
        self.f1 = if self.precision + self.recall > 0.0 {
            2.0 * self.precision * self.recall / (self.precision + self.recall)
        } else {
            0.0
        };
        self
    }

    fn add(&mut self, other: &Counts) {
        self.true_positives += other.true_positives;
        self.false_positives += other.false_positives;
        self.false_negatives += other.false_negatives;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvalReport {
    pub set: String,
    pub set_version: String,
    pub documents: usize,
    pub gold_spans: usize,
    pub scored_predictions: usize,
    pub strict: Counts,
    pub overlap: Counts,
    pub strict_per_label: BTreeMap<String, Counts>,
    /// Model labels seen in predictions that the set does not map.
    pub unmapped_labels: BTreeSet<String>,
}

fn overlaps(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// One-to-one matching of predictions to gold spans of the same label;
/// returns per-label counts.
fn match_document(
    gold: &[GoldSpan],
    predicted: &[(usize, usize, String)],
    strict: bool,
) -> BTreeMap<String, Counts> {
    let mut per_label: BTreeMap<String, Counts> = BTreeMap::new();
    let mut used = vec![false; gold.len()];
    // Deterministic order: by start, end, label.
    let mut order: Vec<usize> = (0..predicted.len()).collect();
    order.sort_by(|&a, &b| predicted[a].cmp(&predicted[b]));
    for i in order {
        let (start, end, label) = &predicted[i];
        let hit = gold.iter().enumerate().position(|(g, span)| {
            !used[g]
                && span.label == *label
                && if strict {
                    span.start == *start && span.end == *end
                } else {
                    overlaps((span.start, span.end), (*start, *end))
                }
        });
        let counts = per_label.entry(label.clone()).or_default();
        match hit {
            Some(g) => {
                used[g] = true;
                counts.true_positives += 1;
            }
            None => counts.false_positives += 1,
        }
    }
    for (g, span) in gold.iter().enumerate() {
        if !used[g] {
            per_label
                .entry(span.label.clone())
                .or_default()
                .false_negatives += 1;
        }
    }
    per_label
}

/// Scores predictions (`document id → spans`) against the set. Documents
/// without predictions count all their gold spans as missed.
#[must_use]
pub fn evaluate(set: &EvalSet, predictions: &BTreeMap<String, Vec<PredictedSpan>>) -> EvalReport {
    let mut strict_per_label: BTreeMap<String, Counts> = set
        .labels
        .iter()
        .map(|l| (l.clone(), Counts::default()))
        .collect();
    let mut overlap = Counts::default();
    let mut unmapped = BTreeSet::new();
    let mut scored = 0;
    let mut gold_spans = 0;
    for doc in &set.documents {
        gold_spans += doc.spans.len();
        let mapped: Vec<(usize, usize, String)> = predictions
            .get(&doc.id)
            .into_iter()
            .flatten()
            .filter_map(|p| match set.canonical(&p.model_label) {
                Some(label) => Some((p.start, p.end, label.to_owned())),
                None => {
                    unmapped.insert(p.model_label.to_ascii_lowercase());
                    None
                }
            })
            .collect();
        scored += mapped.len();
        for (label, counts) in match_document(&doc.spans, &mapped, true) {
            strict_per_label.entry(label).or_default().add(&counts);
        }
        for counts in match_document(&doc.spans, &mapped, false).values() {
            overlap.add(counts);
        }
    }
    let mut strict = Counts::default();
    for counts in strict_per_label.values() {
        strict.add(counts);
    }
    EvalReport {
        set: set.name.clone(),
        set_version: set.version.clone(),
        documents: set.documents.len(),
        gold_spans,
        scored_predictions: scored,
        strict: strict.finish(),
        overlap: overlap.finish(),
        strict_per_label: strict_per_label
            .into_iter()
            .map(|(l, c)| (l, c.finish()))
            .collect(),
        unmapped_labels: unmapped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn set() -> EvalSet {
        let raw = json!({
            "name": "unit", "version": "1", "provenance": "synthetic", "license": "CC0-1.0",
            "labels": ["NAME", "DATE"],
            "label_map": {"first_name": "NAME", "last_name": "NAME", "date": "DATE"},
            "documents": [
                {"id": "d1", "language": "en", "text": "Anna Berg came on 3 May.",
                 "spans": [
                    {"start": 0, "end": 4, "label": "NAME", "text": "Anna"},
                    {"start": 5, "end": 9, "label": "NAME", "text": "Berg"},
                    {"start": 18, "end": 23, "label": "DATE", "text": "3 May"}]},
                {"id": "d2", "language": "en", "text": "Zoë saw Ömer.",
                 "spans": [
                    {"start": 0, "end": 3, "label": "NAME", "text": "Zoë"},
                    {"start": 8, "end": 12, "label": "NAME", "text": "Ömer"}]}
            ]
        });
        EvalSet::from_json(raw.to_string().as_bytes()).unwrap()
    }

    fn p(start: usize, end: usize, label: &str) -> PredictedSpan {
        PredictedSpan {
            start,
            end,
            model_label: label.into(),
        }
    }

    #[test]
    fn strict_and_overlap_scores_are_exact() {
        let s = set();
        let predictions = BTreeMap::from([
            (
                "d1".to_owned(),
                vec![
                    p(0, 4, "B-first_name"),
                    p(5, 9, "last_name"),
                    p(18, 20, "date"),     // boundary differs: overlap only
                    p(10, 14, "city"),     // unmapped
                    p(0, 4, "first_name"), // duplicate: false positive
                ],
            ),
            ("d2".to_owned(), vec![p(0, 3, "FIRST_NAME")]),
        ]);
        let r = evaluate(&s, &predictions);
        assert_eq!(r.gold_spans, 5);
        assert_eq!(r.scored_predictions, 5);
        assert_eq!(
            (
                r.strict.true_positives,
                r.strict.false_positives,
                r.strict.false_negatives
            ),
            (3, 2, 2)
        );
        assert!((r.strict.precision - 0.6).abs() < 1e-12);
        assert!((r.strict.recall - 0.6).abs() < 1e-12);
        assert_eq!(
            (
                r.overlap.true_positives,
                r.overlap.false_positives,
                r.overlap.false_negatives
            ),
            (4, 1, 1)
        );
        assert_eq!(r.strict_per_label["DATE"].false_negatives, 1);
        assert_eq!(r.unmapped_labels, BTreeSet::from(["city".to_owned()]));
        // Identical input, identical report.
        assert_eq!(evaluate(&s, &predictions), r);
    }

    #[test]
    fn missing_documents_count_as_misses_and_empty_scores_are_zero() {
        let r = evaluate(&set(), &BTreeMap::new());
        assert_eq!(r.strict.false_negatives, 5);
        assert_eq!(
            (r.strict.precision, r.strict.recall, r.strict.f1),
            (0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn invalid_sets_are_refused() {
        let base = |doc: Value| {
            json!({"name": "x", "version": "1", "provenance": "p", "license": "CC0-1.0",
                   "labels": ["NAME"], "label_map": {"first_name": "NAME"}, "documents": [doc]})
            .to_string()
        };
        let wrong_text = base(json!({"id": "a", "language": "en", "text": "Anna",
            "spans": [{"start": 0, "end": 4, "label": "NAME", "text": "Anne"}]}));
        assert!(matches!(
            EvalSet::from_json(wrong_text.as_bytes()),
            Err(EvalSetError::SpanTextMismatch { .. })
        ));
        let out_of_range = base(json!({"id": "a", "language": "en", "text": "Anna",
            "spans": [{"start": 2, "end": 9, "label": "NAME", "text": "na"}]}));
        assert!(matches!(
            EvalSet::from_json(out_of_range.as_bytes()),
            Err(EvalSetError::SpanOutOfRange { .. })
        ));
        let bad_label = base(json!({"id": "a", "language": "en", "text": "Anna",
            "spans": [{"start": 0, "end": 4, "label": "PERSON", "text": "Anna"}]}));
        assert!(matches!(
            EvalSet::from_json(bad_label.as_bytes()),
            Err(EvalSetError::UnknownLabel { .. })
        ));
        assert!(matches!(
            EvalSet::from_json(b"[]"),
            Err(EvalSetError::Json(_))
        ));
    }

    #[test]
    fn the_committed_synthetic_pii_gold_set_is_valid() {
        let set = EvalSet::from_json(include_bytes!(
            "../../../evidence/103-local-model-catalog/eval/pii-synthetic-en-v1.json"
        ))
        .unwrap();
        assert_eq!(set.documents.len(), 40);
        assert_eq!(
            set.documents.iter().map(|d| d.spans.len()).sum::<usize>(),
            130
        );
        assert_eq!(set.canonical("B-first_name"), Some("NAME"));
        assert_eq!(set.canonical("medical_record_number"), Some("ID"));
        assert_eq!(set.license, "CC0-1.0");
    }

    #[test]
    fn predicted_spans_are_read_from_runtime_payloads() {
        let payload = json!({"entities": [
            {"label": "first_name", "score": 0.9, "start": 0, "end": 4, "text": "Anna"},
            {"label": "date", "start": "bad"}
        ]});
        assert_eq!(predicted_spans(&payload), vec![p(0, 4, "first_name")]);
        assert!(predicted_spans(&json!({})).is_empty());
        let merged = json!({"pii_merged_entities": [
            {"label": "first_name", "score": 0.8, "start": 0, "end": 9, "text": "Anna Berg"}]});
        assert_eq!(
            predicted_spans_at(&merged, "pii_merged_entities"),
            vec![p(0, 9, "first_name")]
        );
    }
}

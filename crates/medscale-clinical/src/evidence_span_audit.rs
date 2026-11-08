//! Overlap audit for evidence spans.
//!
//! Port of `medscale_clinical.clinical.evidence_span_audit` (derived from
//! OpenMed `openmed/clinical/evidence_span_audit.py` @3e23232, Apache-2.0).
//! Changes from the reference: identifiers are typed strings rather than
//! duck-typed mappings, the number of spans is bounded (`MAX_AUDIT_SPANS`)
//! because the audit is quadratic, and `span_from_ref` adapts MedScale's
//! `EvidenceSpanRef`. Reports are byte-identical to the reference.
//!
//! The audit compares half-open offset ranges `[start, end)` and keeps only
//! hashed identifiers, offsets, classifications, counts and fingerprints. It
//! reads no source text and picks no winner.

use std::cmp::Ordering;

use medscale_contracts::knowledge::EvidenceSpanRef;
use serde_json::{Value, json};
use thiserror::Error;

use crate::canonical::{canonical_json, fingerprint, sha256_tag};

pub const EVIDENCE_SPAN_AUDIT_SCHEMA_VERSION: &str = "1.0";
pub const EVIDENCE_SPAN_AUDIT_ADVISORY: &str = "Evidence-span overlap findings are assistive review signals, not a clinical decision or automatic provenance resolution.";

/// Most spans one audit accepts. Pairs grow with the square of this.
pub const MAX_AUDIT_SPANS: usize = 2_048;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SpanAuditError {
    #[error("invalid evidence span at index {index}: {detail}")]
    InvalidSpan { index: usize, detail: &'static str },
    #[error("too many evidence spans: at most {MAX_AUDIT_SPANS}")]
    TooMany,
}

/// Classification of one overlapping pair, in report order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OverlapKind {
    Exact,
    Nested,
    Partial,
    CrossSource,
}

impl OverlapKind {
    pub const ALL: [Self; 4] = [Self::Exact, Self::Nested, Self::Partial, Self::CrossSource];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Nested => "nested",
            Self::Partial => "partial",
            Self::CrossSource => "cross_source",
        }
    }
}

/// Hashed identity and half-open range of one evidence record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSpan {
    source_id: String,
    evidence_id: String,
    start: u64,
    end: u64,
}

/// An identifier as the reference keeps it: an existing `sha256:<hex>` digest
/// unchanged, anything else replaced by its digest, so a caller id that holds
/// a patient value never reaches a report.
fn identifier(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let is_digest = trimmed.len() == 71
        && trimmed.starts_with("sha256:")
        && trimmed[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    Some(if is_digest {
        trimmed.to_owned()
    } else {
        sha256_tag(trimmed.as_bytes())
    })
}

impl EvidenceSpan {
    /// `source_id` names the coordinate space the offsets belong to;
    /// `evidence_id` names the record. Neither is read as text.
    pub fn new(
        source_id: &str,
        evidence_id: &str,
        start: u64,
        end: u64,
    ) -> Result<Self, &'static str> {
        let (Some(source_id), Some(evidence_id)) = (identifier(source_id), identifier(evidence_id))
        else {
            return Err("invalid record");
        };
        if end <= start {
            return Err("end must be greater than start");
        }
        Ok(Self {
            source_id,
            evidence_id,
            start,
            end,
        })
    }

    #[must_use]
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    #[must_use]
    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    #[must_use]
    pub fn range(&self) -> (u64, u64) {
        (self.start, self.end)
    }

    fn to_value(&self) -> Value {
        json!({
            "source_id": self.source_id,
            "evidence_id": self.evidence_id,
            "start": self.start,
            "end": self.end,
        })
    }

    fn sort_key(&self) -> (&str, u64, u64, &str) {
        (&self.source_id, self.start, self.end, &self.evidence_id)
    }
}

/// A span over MedScale's own evidence reference. The coordinate space is the
/// exact source revision plus the locator (cell, segment or excerpt), because
/// character offsets are relative to that location's text.
pub fn span_from_ref(evidence_id: &str, span: &EvidenceSpanRef) -> Result<EvidenceSpan, String> {
    span.validate()?;
    let space = serde_json::to_string(&(&span.source, &span.locator))
        .map_err(|_| "unserializable evidence span".to_owned())?;
    EvidenceSpan::new(
        &space,
        evidence_id,
        u64::from(span.char_start),
        u64::from(span.char_end),
    )
    .map_err(str::to_owned)
}

/// One overlapping pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSpanOverlap {
    pub first: EvidenceSpan,
    pub second: EvidenceSpan,
    pub kind: OverlapKind,
    pub overlap_start: u64,
    pub overlap_end: u64,
}

impl EvidenceSpanOverlap {
    fn payload(&self) -> Value {
        json!({
            "first": self.first.to_value(),
            "second": self.second.to_value(),
            "kind": self.kind.as_str(),
            "overlap": [self.overlap_start, self.overlap_end],
        })
    }

    #[must_use]
    pub fn fingerprint(&self) -> String {
        fingerprint(&self.payload())
    }

    fn to_value(&self) -> Value {
        let mut value = self.payload();
        value["overlap_length"] = json!(self.overlap_end - self.overlap_start);
        value["fingerprint"] = json!(self.fingerprint());
        value
    }

    fn cmp_report(&self, other: &Self) -> Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| self.first.sort_key().cmp(&other.first.sort_key()))
            .then_with(|| self.second.sort_key().cmp(&other.second.sort_key()))
    }
}

/// The complete audit. Holds no source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSpanAudit {
    pub spans: Vec<EvidenceSpan>,
    pub overlaps: Vec<EvidenceSpanOverlap>,
}

impl EvidenceSpanAudit {
    #[must_use]
    pub fn count(&self, kind: OverlapKind) -> usize {
        self.overlaps.iter().filter(|o| o.kind == kind).count()
    }

    #[must_use]
    pub fn source_count(&self) -> usize {
        let mut sources: Vec<&str> = self.spans.iter().map(EvidenceSpan::source_id).collect();
        sources.sort_unstable();
        sources.dedup();
        sources.len()
    }

    fn counts(&self) -> Value {
        let mut counts = serde_json::Map::new();
        for kind in OverlapKind::ALL {
            counts.insert(kind.as_str().to_owned(), json!(self.count(kind)));
        }
        Value::Object(counts)
    }

    #[must_use]
    pub fn fingerprint(&self) -> String {
        fingerprint(&json!({
            "schema_version": EVIDENCE_SPAN_AUDIT_SCHEMA_VERSION,
            "spans": self.spans.iter().map(EvidenceSpan::to_value).collect::<Vec<_>>(),
            "overlaps": self.overlaps.iter().map(EvidenceSpanOverlap::payload).collect::<Vec<_>>(),
            "counts": self.counts(),
        }))
    }

    /// The report, identical to the reference's `to_dict()`.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "schema_version": EVIDENCE_SPAN_AUDIT_SCHEMA_VERSION,
            "advisory": EVIDENCE_SPAN_AUDIT_ADVISORY,
            "span_count": self.spans.len(),
            "source_count": self.source_count(),
            "counts": self.counts(),
            "overlap_count": self.overlaps.len(),
            "spans": self.spans.iter().map(EvidenceSpan::to_value).collect::<Vec<_>>(),
            "overlaps": self.overlaps.iter().map(EvidenceSpanOverlap::to_value).collect::<Vec<_>>(),
            "fingerprint": self.fingerprint(),
        })
    }

    /// The report as canonical JSON, identical to the reference's `to_json()`.
    #[must_use]
    pub fn to_json(&self) -> String {
        canonical_json(&self.to_value())
    }
}

fn classify(first: &EvidenceSpan, second: &EvidenceSpan) -> Option<EvidenceSpanOverlap> {
    let overlap_start = first.start.max(second.start);
    let overlap_end = first.end.min(second.end);
    if overlap_start >= overlap_end {
        return None;
    }
    let kind = if first.source_id != second.source_id {
        OverlapKind::CrossSource
    } else if first.start == second.start && first.end == second.end {
        OverlapKind::Exact
    } else if (first.start <= second.start && second.end <= first.end)
        || (second.start <= first.start && first.end <= second.end)
    {
        OverlapKind::Nested
    } else {
        OverlapKind::Partial
    };
    Some(EvidenceSpanOverlap {
        first: first.clone(),
        second: second.clone(),
        kind,
        overlap_start,
        overlap_end,
    })
}

/// Audits every pair of spans. Input order doesn't affect the report.
pub fn audit_evidence_spans(spans: Vec<EvidenceSpan>) -> Result<EvidenceSpanAudit, SpanAuditError> {
    if spans.len() > MAX_AUDIT_SPANS {
        return Err(SpanAuditError::TooMany);
    }
    let mut spans = spans;
    spans.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    let mut overlaps = Vec::new();
    for (i, first) in spans.iter().enumerate() {
        for second in &spans[i + 1..] {
            if let Some(overlap) = classify(first, second) {
                overlaps.push(overlap);
            }
        }
    }
    overlaps.sort_by(EvidenceSpanOverlap::cmp_report);
    Ok(EvidenceSpanAudit { spans, overlaps })
}

/// Builds spans from `(source_id, evidence_id, start, end)` records, failing
/// on the first invalid one with its index, as the reference does.
pub fn audit_records(
    records: &[(&str, &str, u64, u64)],
) -> Result<EvidenceSpanAudit, SpanAuditError> {
    if records.len() > MAX_AUDIT_SPANS {
        return Err(SpanAuditError::TooMany);
    }
    let mut spans = Vec::with_capacity(records.len());
    for (index, (source, evidence, start, end)) in records.iter().enumerate() {
        spans.push(
            EvidenceSpan::new(source, evidence, *start, *end)
                .map_err(|detail| SpanAuditError::InvalidSpan { index, detail })?,
        );
    }
    audit_evidence_spans(spans)
}

//! Value-free evidence coverage matrix for claims.
//!
//! Port of `medscale_clinical.clinical.evidence_coverage` (derived from
//! OpenMed `openmed/clinical/evidence_coverage.py` @3e23232, Apache-2.0).
//! Changes from the reference: requirements are typed, so the reference's
//! field-name aliases are the caller's job, and source fingerprints must
//! already be `sha256:<hex>` digests (the reference also hashes raw sources).
//! Identifier handling, merging, validation, the report and its hashes are
//! identical to the reference.
//!
//! Each cell is one required evidence class for one claim. Two requirements
//! for the same cell merge: equal statuses stay, different ones become
//! `conflicting`, and fingerprints are unioned.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use thiserror::Error;

use crate::canonical::{canonical_json, fingerprint, sha256_tag};

pub const EVIDENCE_COVERAGE_SCHEMA_VERSION: &str =
    "medscale_clinical.clinical.evidence_coverage.v1";
/// Evidence classes the reference keeps in plain text; any other class is
/// replaced by its digest, like claim identifiers.
pub const PUBLIC_EVIDENCE_CLASSES: [&str; 5] = [
    "imaging",
    "lab_result",
    "local_record",
    "review_note",
    "second_source",
];

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoverageError {
    #[error("{0} must be an opaque identifier")]
    Identifier(&'static str),
    #[error("present evidence requires a fingerprint")]
    PresentWithoutFingerprint,
    #[error("missing evidence cannot have a fingerprint")]
    MissingWithFingerprint,
    #[error("source fingerprint must be a sha256 digest")]
    Fingerprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoverageStatus {
    Present,
    Missing,
    Conflicting,
    Unreviewed,
}

impl CoverageStatus {
    pub const ALL: [Self; 4] = [
        Self::Present,
        Self::Missing,
        Self::Conflicting,
        Self::Unreviewed,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Missing => "missing",
            Self::Conflicting => "conflicting",
            Self::Unreviewed => "unreviewed",
        }
    }

    /// The review state the reference pairs with each status.
    #[must_use]
    pub const fn review_state(self) -> &'static str {
        match self {
            Self::Present => "reviewed",
            other => other.as_str(),
        }
    }
}

/// One requirement as supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub claim_id: String,
    pub evidence_class: String,
    pub status: CoverageStatus,
    pub source_fingerprints: Vec<String>,
}

/// One validated, normalized cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageRecord {
    pub claim_id: String,
    pub evidence_class: String,
    pub status: CoverageStatus,
    pub source_fingerprints: Vec<String>,
}

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The reference's identifier rule: a plain opaque token, kept only when it
/// is already a digest (or, for classes, a public class code) and otherwise
/// replaced by its digest, so caller ids that hold patient values never
/// reach a report.
fn identifier(raw: &str, field: &'static str) -> Result<String, CoverageError> {
    let value = raw.trim();
    let mut chars = value.chars();
    let shape_ok = chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && value.chars().count() <= 128
        && chars.all(|c| c.is_ascii_alphanumeric() || "._:/+@-".contains(c));
    if !shape_ok {
        return Err(CoverageError::Identifier(field));
    }
    if field == "evidence_class" && PUBLIC_EVIDENCE_CLASSES.contains(&value) {
        return Ok(value.to_owned());
    }
    Ok(if is_digest(value) {
        value.to_owned()
    } else {
        sha256_tag(value.as_bytes())
    })
}

/// The identifier a claim id becomes in the report.
pub fn normalize_claim_id(raw: &str) -> Result<String, CoverageError> {
    identifier(raw, "claim_id")
}

/// The identifier an evidence class becomes in the report.
pub fn normalize_evidence_class(raw: &str) -> Result<String, CoverageError> {
    identifier(raw, "evidence_class")
}

impl CoverageRecord {
    fn new(
        claim_id: &str,
        evidence_class: &str,
        status: CoverageStatus,
        fingerprints: &[String],
    ) -> Result<Self, CoverageError> {
        let claim_id = identifier(claim_id, "claim_id")?;
        let evidence_class = identifier(evidence_class, "evidence_class")?;
        let mut normalized = BTreeSet::new();
        for raw in fingerprints {
            let value = raw.trim().to_ascii_lowercase();
            if !is_digest(&value) {
                return Err(CoverageError::Fingerprint);
            }
            normalized.insert(value);
        }
        if status == CoverageStatus::Present && normalized.is_empty() {
            return Err(CoverageError::PresentWithoutFingerprint);
        }
        if status == CoverageStatus::Missing && !normalized.is_empty() {
            return Err(CoverageError::MissingWithFingerprint);
        }
        Ok(Self {
            claim_id,
            evidence_class,
            status,
            source_fingerprints: normalized.into_iter().collect(),
        })
    }

    fn to_value(&self, include_claim_id: bool) -> Value {
        let mut value = json!({
            "evidence_class": self.evidence_class,
            "status": self.status.as_str(),
            "review_state": self.status.review_state(),
            "source_fingerprints": self.source_fingerprints,
        });
        if include_claim_id {
            value["claim_id"] = json!(self.claim_id);
        }
        value
    }

    fn merge(self, other: Self) -> Result<Self, CoverageError> {
        let status = if self.status == other.status {
            self.status
        } else {
            CoverageStatus::Conflicting
        };
        let mut fingerprints: BTreeSet<String> = self.source_fingerprints.into_iter().collect();
        fingerprints.extend(other.source_fingerprints);
        let fingerprints: Vec<String> = fingerprints.into_iter().collect();
        CoverageRecord::new(&self.claim_id, &self.evidence_class, status, &fingerprints)
    }
}

/// The matrix, sorted by claim then evidence class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageMatrix {
    pub records: Vec<CoverageRecord>,
}

impl CoverageMatrix {
    #[must_use]
    pub fn claim_records(&self, claim_id: &str) -> Vec<&CoverageRecord> {
        self.records
            .iter()
            .filter(|r| r.claim_id == claim_id)
            .collect()
    }

    fn claim_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        for record in &self.records {
            if ids.last() != Some(&record.claim_id.as_str()) {
                ids.push(&record.claim_id);
            }
        }
        ids
    }

    #[must_use]
    pub fn claim_hash(&self, claim_id: &str) -> String {
        let records = self.claim_records(claim_id);
        fingerprint(&json!({
            "claim_id": claim_id,
            "requirements": records.iter().map(|r| r.to_value(false)).collect::<Vec<_>>(),
        }))
    }

    #[must_use]
    pub fn matrix_hash(&self) -> String {
        fingerprint(&json!({
            "schema_version": EVIDENCE_COVERAGE_SCHEMA_VERSION,
            "records": self.records.iter().map(|r| r.to_value(true)).collect::<Vec<_>>(),
        }))
    }

    fn all_fingerprints(&self) -> BTreeSet<&str> {
        self.records
            .iter()
            .flat_map(|r| r.source_fingerprints.iter().map(String::as_str))
            .collect()
    }

    fn status_counts<'a>(records: impl Iterator<Item = &'a CoverageRecord>) -> Value {
        let mut counts: BTreeMap<&str, u64> = CoverageStatus::ALL
            .iter()
            .map(|s| (s.as_str(), 0))
            .collect();
        for record in records {
            *counts.entry(record.status.as_str()).or_default() += 1;
        }
        json!(counts)
    }

    /// The report, identical to the reference's `to_dict()`.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut classes: BTreeMap<&str, Vec<&CoverageRecord>> = BTreeMap::new();
        for record in &self.records {
            classes
                .entry(&record.evidence_class)
                .or_default()
                .push(record);
        }
        let class_counts: BTreeMap<&str, Value> = classes
            .into_iter()
            .map(|(class, records)| (class, Self::status_counts(records.into_iter())))
            .collect();
        let fingerprints: Vec<&str> = self.all_fingerprints().into_iter().collect();
        let claims: Vec<Value> = self
            .claim_ids()
            .into_iter()
            .map(|claim_id| {
                json!({
                    "claim_id": claim_id,
                    "requirements": self
                        .claim_records(claim_id)
                        .iter()
                        .map(|r| r.to_value(false))
                        .collect::<Vec<_>>(),
                    "claim_hash": self.claim_hash(claim_id),
                })
            })
            .collect();
        json!({
            "schema_version": EVIDENCE_COVERAGE_SCHEMA_VERSION,
            "claim_count": claims.len(),
            "required_evidence_count": self.records.len(),
            "status_counts": Self::status_counts(self.records.iter()),
            "evidence_class_counts": class_counts,
            "source_fingerprint_count": fingerprints.len(),
            "source_fingerprint_hash": fingerprint(&json!(fingerprints)),
            "claims": claims,
            "matrix_hash": self.matrix_hash(),
        })
    }

    /// Identical to the reference's compact `to_json()`.
    #[must_use]
    pub fn to_json(&self) -> String {
        canonical_json(&self.to_value())
    }
}

/// Builds the matrix: each requirement is validated, then requirements for
/// the same cell are merged.
pub fn build_coverage_matrix(
    requirements: Vec<Requirement>,
) -> Result<CoverageMatrix, CoverageError> {
    let mut cells: BTreeMap<(String, String), CoverageRecord> = BTreeMap::new();
    for requirement in requirements {
        let record = CoverageRecord::new(
            &requirement.claim_id,
            &requirement.evidence_class,
            requirement.status,
            &requirement.source_fingerprints,
        )?;
        let key = (record.claim_id.clone(), record.evidence_class.clone());
        let merged = match cells.remove(&key) {
            Some(previous) => previous.merge(record)?,
            None => record,
        };
        cells.insert(key, merged);
    }
    Ok(CoverageMatrix {
        records: cells.into_values().collect(),
    })
}

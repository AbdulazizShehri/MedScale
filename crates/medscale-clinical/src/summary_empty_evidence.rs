//! Refusal gate for generation with no approved evidence.
//!
//! Port of `medscale_clinical.clinical.summary_empty_evidence` (derived from
//! OpenMed `openmed/clinical/summary_empty_evidence.py` @3e23232, Apache-2.0).
//! Changes from the reference: candidates carry a typed approval state rather
//! than a duck-typed mapping, so the reference's field-name heuristics are
//! the caller's job; the counting, the refusal record and its JSON are
//! identical to the reference.
//!
//! The gate passes approved candidates to the caller, in order, and refuses
//! with a value-free record when none remain. It never generates anything.

use serde_json::{Value, json};

use crate::canonical::canonical_json;

pub const SUMMARY_EMPTY_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const SUMMARY_EMPTY_EVIDENCE_REFUSAL_CODE: &str = "empty_approved_evidence";
pub const SUMMARY_EMPTY_EVIDENCE_DISCLAIMER: &str = "Clinical summary generation was refused because no approved evidence survived filtering. This is an assistive safety gate, not a clinical decision; qualified clinical review is required.";

/// Whether one candidate may ground generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approval {
    Approved,
    /// Explicitly not approved (rejected, pending, stale).
    Unapproved,
    /// Malformed or unusable.
    Invalid,
}

/// The value-free refusal. Every candidate was excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyEvidenceRefusal {
    pub input_evidence_count: usize,
}

impl EmptyEvidenceRefusal {
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "schema_version": SUMMARY_EMPTY_EVIDENCE_SCHEMA_VERSION,
            "status": "refused",
            "refusal_code": SUMMARY_EMPTY_EVIDENCE_REFUSAL_CODE,
            "input_evidence_count": self.input_evidence_count,
            "approved_evidence_count": 0,
            "excluded_evidence_count": self.input_evidence_count,
            "requires_clinician_review": true,
            "autonomous_decision": false,
            "disclaimer": SUMMARY_EMPTY_EVIDENCE_DISCLAIMER,
        })
    }

    /// Identical to the reference's `to_json()`, trailing newline included.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = canonical_json(&self.to_value());
        out.push('\n');
        out
    }
}

/// Returns the approved candidates in their original order, or the refusal
/// when none is approved.
pub fn require_approved_evidence<T>(
    candidates: Vec<(T, Approval)>,
) -> Result<Vec<T>, EmptyEvidenceRefusal> {
    let input_evidence_count = candidates.len();
    let approved: Vec<T> = candidates
        .into_iter()
        .filter_map(|(item, approval)| (approval == Approval::Approved).then_some(item))
        .collect();
    if approved.is_empty() {
        Err(EmptyEvidenceRefusal {
            input_evidence_count,
        })
    } else {
        Ok(approved)
    }
}

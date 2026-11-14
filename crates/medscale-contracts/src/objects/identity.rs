//! Identity assertion and explicit merge decision.

use serde::{Deserialize, Serialize};

use super::{ObjectHeader, OpaqueId};

/// Explicit identity statement — not a silent merge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityAssertion {
    pub header: ObjectHeader,
    pub subject_id: OpaqueId,
    pub identifier_system: String,
    pub identifier_value: String,
    pub confidence: Option<f64>,
    pub evidence_refs: Vec<OpaqueId>,
}

/// Explicit decision required to unify subject identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityMergeDecision {
    pub header: ObjectHeader,
    pub surviving_subject_id: OpaqueId,
    pub merged_subject_ids: Vec<OpaqueId>,
    pub authorized_by: OpaqueId,
    pub rationale: String,
    pub evidence_refs: Vec<OpaqueId>,
}

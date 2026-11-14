//! Evaluation, projection, and audit/action records.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{DigestSha256, EffectState, MedicalTime, ObjectHeader, OpaqueId};

/// Evidence-only evaluation outcome (never clinical authority).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationRecord {
    pub header: ObjectHeader,
    pub target_refs: Vec<OpaqueId>,
    pub evaluator: String,
    pub result: Value,
    /// Constant true for authority purposes.
    pub evidence_only: bool,
}

/// Rebuildable non-authoritative projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub header: ObjectHeader,
    pub projection_kind: String,
    pub built_from: Vec<OpaqueId>,
    pub built_at: MedicalTime,
    pub body: Value,
    /// Constant false — projections are never truth.
    pub authoritative: bool,
}

/// Audit vs external-action intent kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionAuditKind {
    Audit,
    ExternalActionIntent,
}

/// Durable audit / external-action intent trail record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionAuditRecord {
    pub header: ObjectHeader,
    pub kind: ActionAuditKind,
    pub actor: OpaqueId,
    pub action: String,
    pub target_refs: Vec<OpaqueId>,
    pub effect_state: Option<EffectState>,
    pub payload_digest: Option<DigestSha256>,
    pub detail: Option<Value>,
}

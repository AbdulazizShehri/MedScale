//! Proposal and clinical assertion types (must remain distinct).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{MedicalTime, ObjectHeader, OpaqueId};

/// Producer of a proposal — never grants clinical authority by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProducerKind {
    Human,
    Rule,
    WorkerStub,
    /// A Spec 077 `AgentIdentity` (by id) produced this proposal. Added
    /// additively for MedAgent Workbench; carries no clinical/research
    /// authority beyond any other `Proposal`, and this variant alone never
    /// implies promotion to `ClinicalAssertion` (`security.md` T1 in
    /// `specs/077-medagent-workbench/`).
    Agent(OpaqueId),
    Other(String),
}

/// Non-authoritative candidate claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub header: ObjectHeader,
    pub subject_ref: Option<OpaqueId>,
    pub claim_kind: String,
    pub payload: Value,
    pub confidence: Option<f64>,
    pub evidence_refs: Vec<OpaqueId>,
    pub producer: ProducerKind,
}

/// Authorized clinical truth claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClinicalAssertion {
    pub header: ObjectHeader,
    pub subject_ref: OpaqueId,
    pub claim_kind: String,
    pub payload: Value,
    pub promoted_from_proposal_id: Option<OpaqueId>,
    pub authorized_by: OpaqueId,
    pub effective_time: Option<MedicalTime>,
    pub recorded_time: MedicalTime,
}

//! Versioned legal / flow decision records (EXTERNAL_GATES: LEGAL_COUNSEL_FLOW_MAPPING).
//!
//! These types store counsel-supplied conclusions. MedScale never invents PDPL/SFDA
//! lawful-basis answers; empty/pending records are the default.

use serde::{Deserialize, Serialize};

use crate::objects::{MedicalTime, OpaqueId};

/// Schema version for flow decision records.
pub const FLOW_DECISION_SCHEMA_VERSION: u32 = 1;

/// Lifecycle of a counsel-owned flow decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowDecisionStatus {
    /// Structure present; no counsel conclusion recorded.
    PendingCounsel,
    /// Counsel recorded a conclusion (payload opaque to MedScale semantics).
    Recorded,
    Superseded,
}

/// Jurisdiction / regime tag (opaque string; not a legal opinion).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JurisdictionTag {
    pub code: String,
}

/// Versioned decision record — conclusions only via `counsel_conclusion`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowDecisionRecord {
    pub schema_version: u32,
    pub decision_id: OpaqueId,
    pub flow_id: String,
    pub status: FlowDecisionStatus,
    pub jurisdictions: Vec<JurisdictionTag>,
    /// Opaque counsel text/id; MUST be None while PendingCounsel.
    pub counsel_conclusion: Option<String>,
    pub recorded_at: Option<MedicalTime>,
    pub recorded_by: Option<OpaqueId>,
}

impl FlowDecisionRecord {
    /// Empty pending shell — no invented conclusions.
    #[must_use]
    pub fn pending(flow_id: impl Into<String>) -> Self {
        let flow_id = flow_id.into();
        Self {
            schema_version: FLOW_DECISION_SCHEMA_VERSION,
            decision_id: OpaqueId::new(format!("flow-{flow_id}")),
            flow_id,
            status: FlowDecisionStatus::PendingCounsel,
            jurisdictions: Vec::new(),
            counsel_conclusion: None,
            recorded_at: None,
            recorded_by: None,
        }
    }

    /// Alias for [`Self::pending`].
    #[must_use]
    pub fn for_flow(flow_id: impl Into<String>) -> Self {
        Self::pending(flow_id)
    }

    /// Rejects invented conclusions: PendingCounsel cannot carry counsel_conclusion.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        match self.status {
            FlowDecisionStatus::PendingCounsel => self.counsel_conclusion.is_none(),
            FlowDecisionStatus::Recorded => self
                .counsel_conclusion
                .as_ref()
                .is_some_and(|s| !s.is_empty()),
            FlowDecisionStatus::Superseded => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_has_no_invented_conclusion() {
        let r = FlowDecisionRecord::for_flow("pdpl_controller_mapping");
        assert_eq!(r.status, FlowDecisionStatus::PendingCounsel);
        assert!(r.counsel_conclusion.is_none());
        assert!(r.is_structurally_valid());
    }

    #[test]
    fn pending_with_conclusion_is_invalid() {
        let mut r = FlowDecisionRecord::for_flow("x");
        r.counsel_conclusion = Some("invented".to_owned());
        assert!(!r.is_structurally_valid());
    }
}

//! Minimum lovable workflow contracts (Spec 021 / Trusted V1 Q07).
//!
//! Synthetic-only journey types, disclosure append, and doctor honesty.
//! Completing a journey never implies RELEASE_READY.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fhir::FhirLossAwareExport;
use crate::objects::{DigestSha256, OpaqueId};

/// Doctor axis for minimum lovable workflow readiness (Spec 021).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDoctorStatus {
    pub present: bool,
    /// Synthetic end-to-end journey is composed and tested (READY_BASE).
    pub workflow_ready_base: bool,
    /// Never true in Spec 021; product release remains ungated.
    pub release_ready: bool,
    pub synthetic_only: bool,
    pub disclosure_append_supported: bool,
}

impl WorkflowDoctorStatus {
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            workflow_ready_base: true,
            release_ready: false,
            synthetic_only: true,
            disclosure_append_supported: true,
        }
    }
}

/// Local append-only disclosure of an export / share event (synthetic).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisclosureRecord {
    pub disclosure_id: OpaqueId,
    pub purpose: String,
    pub scope: String,
    pub subject_ref: Option<OpaqueId>,
    pub artifact_refs: Vec<OpaqueId>,
    pub export_digest: Option<DigestSha256>,
    pub synthetic_only: bool,
    /// Always false for Spec 021 disclosures.
    pub release_ready_claimed: bool,
    pub note: Option<String>,
}

/// Preview of an imported FHIR resource before Accept/Reject.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportPreview {
    pub source_id: OpaqueId,
    pub proposal_id: OpaqueId,
    pub subject_ref: OpaqueId,
    pub claim_kind: String,
    pub resource_type: String,
    pub loss_aware_export: FhirLossAwareExport,
    pub promoted: bool,
}

/// Named steps in the minimum lovable journey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JourneyStep {
    Install,
    StartOffline,
    InspectPrivacyCapability,
    LoadSynthetic,
    ImportFhir,
    Preview,
    AcceptOrReject,
    TimelineBriefCoverage,
    SourceDrillDown,
    Close,
    Reopen,
    VerifySameRecord,
    Export,
    DisclosureRecord,
    Backup,
    VerifyBackup,
    Restore,
}

/// Result of one journey step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JourneyStepResult {
    pub step: JourneyStep,
    pub ok: bool,
    pub detail: Value,
}

/// Full synthetic journey report (non-authoritative).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JourneyReport {
    pub schema_version: u32,
    pub synthetic_only: bool,
    pub workflow_ready_base: bool,
    pub release_ready: bool,
    pub subject_ref: String,
    pub source_id: Option<String>,
    pub assertion_id: Option<String>,
    pub disclosure_id: Option<String>,
    pub steps: Vec<JourneyStepResult>,
}

impl JourneyReport {
    #[must_use]
    pub fn is_honest_ready_base(&self) -> bool {
        self.synthetic_only && self.workflow_ready_base && !self.release_ready
    }
}

/// Stable CLI / tool JSON error envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliJsonError {
    pub error: String,
    pub code: String,
    pub message: String,
    pub synthetic_only: bool,
}

impl CliJsonError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            error: "cli_error".to_owned(),
            code: code.clone(),
            message: message.into(),
            synthetic_only: true,
        }
    }
}

/// Action string stored on disclosure append audits.
pub const DISCLOSURE_APPEND_ACTION: &str = "disclosure.append";

/// Action string stored on proposal reject audits.
pub const PROPOSAL_REJECT_ACTION: &str = "proposal.reject";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_doctor_honest() {
        let s = WorkflowDoctorStatus::ready_base();
        assert!(s.workflow_ready_base);
        assert!(!s.release_ready);
        assert!(s.synthetic_only);
        assert!(s.disclosure_append_supported);
    }

    #[test]
    fn cli_json_error_shape() {
        let e = CliJsonError::new("vault_required", "open a vault first");
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["code"], "vault_required");
        assert_eq!(v["synthetic_only"], true);
    }
}

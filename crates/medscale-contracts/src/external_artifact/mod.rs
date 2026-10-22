//! MESC artifact admission + synthetic verifier contracts (Specs 012 / 036).
//!
//! ARTIFACT_FIRST only. Never Python import / shared DB / shared keys.
//! Spec 036 ships a synthetic-manifest verifier READY_BASE; product admit remains
//! fail-closed on `MESC_RELEASED_ARTIFACT` until a real upstream release qualifies.
//!
//! Current governance: MESC is a separate project/repository. These contracts are
//! retained only for historical interoperability and backward-compatible decoding.
//! They do not create a current MedScale integration lane, external gate, completion
//! axis, release axis, or execution authority.

use serde::{Deserialize, Serialize};

use crate::objects::DigestSha256;

/// Doctor axis for MESC artifact integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MescArtifactDoctorStatus {
    pub present: bool,
    pub artifact_admitted: bool,
    /// Spec 036: synthetic release-manifest verifier READY_BASE present.
    pub verifier_ready_base: bool,
    pub python_runtime_imported: bool,
    pub shared_db_or_keys: bool,
    pub disposition: String,
    pub gate: String,
    /// Historical compatibility field. Current MedScale doctor state keeps this
    /// false because MESC is a separate project and cannot block MedScale.
    #[serde(default)]
    pub required: bool,
    /// Compatibility status. Current MedScale doctor state uses OUT_OF_SCOPE.
    #[serde(default = "default_mesc_integration_status")]
    pub integration_status: String,
}

/// Default optional-integration status when no MESC artifact is configured.
fn default_mesc_integration_status() -> String {
    "NOT_CONFIGURED".to_owned()
}

impl MescArtifactDoctorStatus {
    /// Historical compatibility default retained for decoding/tests from Specs 012/036.
    #[must_use]
    pub fn not_configured() -> Self {
        Self {
            present: true,
            artifact_admitted: false,
            verifier_ready_base: true,
            python_runtime_imported: false,
            shared_db_or_keys: false,
            disposition: "OPTIONAL_INTEGRATION_NOT_CONFIGURED".to_owned(),
            gate: "OPTIONAL_MESC_ARTIFACT".to_owned(),
            required: false,
            integration_status: "NOT_CONFIGURED".to_owned(),
        }
    }

    /// Current MedScale doctor state: MESC is a separate project/repository.
    #[must_use]
    pub fn separate_project() -> Self {
        Self {
            present: true,
            artifact_admitted: false,
            verifier_ready_base: true,
            python_runtime_imported: false,
            shared_db_or_keys: false,
            disposition: "SEPARATE_PROJECT".to_owned(),
            gate: "NONE".to_owned(),
            required: false,
            integration_status: "OUT_OF_SCOPE".to_owned(),
        }
    }

    /// Admit-path view while the upstream artifact is unavailable: verifier
    /// READY_BASE may be present; nothing admitted. The gate blocks only the
    /// optional ARTIFACT_IMPORT lane, never core completion or release.
    #[must_use]
    pub fn gate_blocked() -> Self {
        Self {
            present: true,
            artifact_admitted: false,
            verifier_ready_base: true,
            python_runtime_imported: false,
            shared_db_or_keys: false,
            disposition: "ARTIFACT_IMPORT".to_owned(),
            gate: "MESC_RELEASED_ARTIFACT".to_owned(),
            required: false,
            integration_status: "NOT_AVAILABLE".to_owned(),
        }
    }

    #[must_use]
    pub fn is_honest_ready_base(&self) -> bool {
        let legacy_interop =
            self.gate == "MESC_RELEASED_ARTIFACT" || self.gate == "OPTIONAL_MESC_ARTIFACT";
        let separate_project = self.gate == "NONE"
            && self.integration_status == "OUT_OF_SCOPE"
            && self.disposition == "SEPARATE_PROJECT";
        self.present
            && !self.artifact_admitted
            && self.verifier_ready_base
            && !self.python_runtime_imported
            && !self.shared_db_or_keys
            && !self.required
            && (legacy_interop || separate_project)
    }

    /// Release-qualification predicate: MESC absence must never fail release.
    #[must_use]
    pub fn blocks_release(&self) -> bool {
        self.required && self.integration_status != "AVAILABLE"
    }
}

/// Request to admit a released MESC artifact through Pack (gated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MescArtifactAdmitRequest {
    pub artifact_uri: String,
    pub content_digest: DigestSha256,
    pub rights_uri: String,
    pub sbom_digest: DigestSha256,
    pub evaluation_digest: DigestSha256,
    /// Must be true — Pack path only.
    pub pack_path_required: bool,
}

/// Request to verify a local synthetic MESC release directory (Spec 036).
///
/// Never installs or enables a Pack. Digests establish byte identity only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MescArtifactVerifyRequest {
    /// Absolute or vault-relative path to a directory containing `manifest.json` + files.
    pub release_dir: String,
}

/// Explicit refuse of MESC Python / ambient service paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MescIntegrationMode {
    ArtifactImportOnly,
    PythonRuntimeForbidden,
    SharedDbForbidden,
    SharedKeysForbidden,
    AmbientServiceForbidden,
}

/// Admission / verification state machine (Spec 012 acceptance contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MescAdmissionState {
    Discovered,
    Quarantined,
    Verified,
    Qualified,
    InstalledDisabled,
    Enabled,
    Rejected,
}

/// Stable reject / verify reason codes (Spec 036).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MescVerifyReason {
    Ok,
    MissingManifest,
    MalformedManifest,
    UnsupportedSchema,
    DuplicateArtifactPath,
    MissingField,
    MissingArtifactFile,
    SizeMismatch,
    DigestMismatch,
    MissingRights,
    MissingSbom,
    MissingEvaluation,
    MissingTrainingReceipt,
    MissingProducer,
    MissingReleaseIdentity,
    AntiRollback,
    ReplayRejected,
    GateBlocked,
}

impl MescVerifyReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::MissingManifest => "missing_manifest",
            Self::MalformedManifest => "malformed_manifest",
            Self::UnsupportedSchema => "unsupported_schema",
            Self::DuplicateArtifactPath => "duplicate_artifact_path",
            Self::MissingField => "missing_field",
            Self::MissingArtifactFile => "missing_artifact_file",
            Self::SizeMismatch => "size_mismatch",
            Self::DigestMismatch => "digest_mismatch",
            Self::MissingRights => "missing_rights",
            Self::MissingSbom => "missing_sbom",
            Self::MissingEvaluation => "missing_evaluation",
            Self::MissingTrainingReceipt => "missing_training_receipt",
            Self::MissingProducer => "missing_producer",
            Self::MissingReleaseIdentity => "missing_release_identity",
            Self::AntiRollback => "anti_rollback",
            Self::ReplayRejected => "replay_rejected",
            Self::GateBlocked => "gate_blocked",
        }
    }
}

/// One file in a synthetic MESC release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MescArtifactFile {
    pub kind: String,
    pub path: String,
    pub byte_length: u64,
    pub sha256_hex: String,
}

/// Versioned synthetic MESC release manifest (Spec 036 READY_BASE).
///
/// This is MedScale's independent admission contract input — not a claim that a
/// real TheHalfMoon/MESC release exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MescReleaseManifestV0 {
    pub schema_version: u32,
    pub producer_id: String,
    pub release_id: String,
    pub release_tag: String,
    pub source_commit: String,
    pub source_tree: String,
    pub model_id: String,
    pub tokenizer_id: String,
    pub base_model_id: String,
    pub corpus_id: String,
    pub training_receipt_digest_hex: String,
    pub evaluation_receipt_digest_hex: String,
    pub sbom_path: String,
    pub sbom_digest_hex: String,
    pub rights_license: String,
    pub rights_notice_path: String,
    pub provenance_note: String,
    pub limitations: Vec<String>,
    pub runtime_requirements: String,
    /// Monotonic anti-rollback epoch for this producer.
    pub epoch: u64,
    pub artifacts: Vec<MescArtifactFile>,
}

impl MescReleaseManifestV0 {
    pub const SCHEMA_VERSION: u32 = 1;
}

/// Result of verifying a local synthetic MESC release directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MescVerifyReport {
    pub state: MescAdmissionState,
    pub reason: MescVerifyReason,
    pub detail: String,
    pub producer_id: Option<String>,
    pub release_id: Option<String>,
    pub epoch: Option<u64>,
    /// Always false while EXTERNAL_GATES MESC_RELEASED_ARTIFACT is open.
    pub product_admit_authorized: bool,
}

impl MescVerifyReport {
    #[must_use]
    pub fn rejected(reason: MescVerifyReason, detail: impl Into<String>) -> Self {
        Self {
            state: MescAdmissionState::Rejected,
            reason,
            detail: detail.into(),
            producer_id: None,
            release_id: None,
            epoch: None,
            product_admit_authorized: false,
        }
    }

    #[must_use]
    pub fn verified_synthetic(producer_id: String, release_id: String, epoch: u64) -> Self {
        Self {
            state: MescAdmissionState::Verified,
            reason: MescVerifyReason::Ok,
            detail: "synthetic manifest verified; product admit remains gate-blocked".to_owned(),
            producer_id: Some(producer_id),
            release_id: Some(release_id),
            epoch: Some(epoch),
            product_admit_authorized: false,
        }
    }
}

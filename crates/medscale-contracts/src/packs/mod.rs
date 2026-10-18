//! Pack v0 contracts (Spec 008 + Spec 026 signer/anti-rollback).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::objects::{DigestSha256, OpaqueId};

/// Promotion lifecycle for admitted packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackPromotionState {
    Candidate,
    Current,
    LastGreen,
    Canary,
}

/// Artifact class declared in a pack (fail-closed on forbidden kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackArtifactKind {
    FixtureBytes,
    TokenizerMeta,
    ModelMetadata,
    OnnxModel,
    /// Data-only safetensors weights (Spec 104). No executable content.
    SafetensorsModel,
    /// Forbidden by default.
    Pickle,
    /// Forbidden by default (code-bearing).
    CodeBin,
    /// Forbidden unless signed custom-op admission (not in Spec 008).
    OnnxCustomOp,
}

impl PackArtifactKind {
    /// Returns true when Spec 008 must deny admission.
    #[must_use]
    pub const fn is_forbidden_by_default(self) -> bool {
        matches!(self, Self::Pickle | Self::CodeBin | Self::OnnxCustomOp)
    }
}

/// Signed provenance for one model artifact source (Spec 069).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSourceProvenance {
    pub source_kind: String,
    pub repository: String,
    pub revision: String,
    pub original_file: String,
    pub source_sha256: String,
    pub transform: String,
    pub fixed_sequence_length: u32,
    pub task: String,
    pub export_format: String,
    pub runtime_family: String,
    pub license_id: String,
    pub upstream_rights_uri: String,
}

/// One artifact entry in PackManifestV0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackArtifactEntry {
    pub relative_path: String,
    pub kind: PackArtifactKind,
    pub digest: DigestSha256,
}

/// Content-addressed Pack v0 manifest (Spec 026: signer + epoch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifestV0 {
    pub pack_id: OpaqueId,
    pub version: String,
    /// Monotonic anti-rollback epoch (Spec 026).
    pub pack_epoch: u64,
    pub content_digest: DigestSha256,
    pub artifacts: Vec<PackArtifactEntry>,
    pub rights_uri: String,
    pub sbom_ref: String,
    pub runtime_requirements: String,
    pub benchmark_links: Vec<String>,
    pub promotion_state: PackPromotionState,
    /// Must match synthetic trust root id (e.g. `synthetic-pack-trust-v1`).
    pub trust_root_id: String,
    /// Hex-encoded ed25519 signature over the canonical signing payload.
    pub signature_hex: String,
}

/// Doctor axis for packs / runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PacksRuntimeDoctorStatus {
    pub present: bool,
    pub offline_only: bool,
    pub admitted_count: u32,
    pub current_pack_id: Option<String>,
    pub confinement_claim: String,
    pub online_download_authorized: bool,
}

/// Bounded local pack evaluation request (Spec 069).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackEvaluationRequest {
    pub pack_id: OpaqueId,
    pub local_path: String,
    pub input: String,
    pub max_tokens: u32,
    /// Spec 069 remains synthetic-only; real PHI requires a later explicit gate.
    pub synthetic_only: bool,
    /// Spec 104: entity types for a GLiNER zero-shot Pack (1 to 64). Required
    /// for zero-shot Packs and refused for token-classification Packs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero_shot_labels: Option<Vec<String>>,
    /// Spec 103: how token predictions become entities, for token-
    /// classification Packs. Absent: the OpenMed-compatible decoding on the
    /// request's single window, exactly as before. Present: the whole input
    /// runs in overlapping windows with this decoding. Refused for zero-shot
    /// Packs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoding: Option<EntityDecoding>,
}

/// Entity decoding for token-classification Packs (Spec 103). Measured on
/// MedScale's synthetic PII gold set (24 OpenMed PII models): mean strict F1
/// 0.487 for `openmed_compatible` and 0.893 for `word_first_pii_merge`
/// (`evidence/103-local-model-catalog/eval/`). Evaluation evidence only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityDecoding {
    /// OpenMed's ONNX decoder (token-level), the parity reference.
    #[default]
    OpenmedCompatible,
    /// Each word takes its first sub-token's label.
    WordFirst,
    /// Each word takes its sub-tokens' averaged probabilities.
    WordAverage,
    /// Each word takes its most confident sub-token's label.
    WordMax,
    /// OpenMed-compatible decoding plus OpenMed's PII entity merger.
    PiiMerge,
    /// `word_first` plus OpenMed's PII entity merger.
    WordFirstPiiMerge,
}

/// One prepared model held in a Core residency pool (Spec 103).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentModelView {
    /// `tract_onnx`, `candle_token_classifier` or `candle_gliner`.
    pub runtime: String,
    pub content_digest: DigestSha256,
    /// Admitted Packs with this content digest.
    pub pack_ids: Vec<OpaqueId>,
    /// Budgeted size (the model artifact bytes).
    pub resident_bytes: u64,
    pub pinned: bool,
}

/// Prepared models resident in Core (`PacksResidency`, `PacksUnloadLocal`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackResidencyView {
    /// Byte budget of each runtime's pool.
    pub budget_bytes_per_pool: u64,
    pub models: Vec<ResidentModelView>,
    /// Content digests unloaded by this request (empty for a listing).
    pub unloaded: Vec<DigestSha256>,
}

/// Bounded local text generation with an admitted generation Pack (Spec 105).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackGenerationRequest {
    pub pack_id: OpaqueId,
    pub local_path: String,
    pub prompt: String,
    /// 1 to 256 generated tokens.
    pub max_new_tokens: u32,
    /// Synthetic input only; real PHI requires a later explicit gate.
    pub synthetic_only: bool,
}

/// Evidence-only output from a local admitted model runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackEvaluationResult {
    pub pack_id: OpaqueId,
    pub content_digest: DigestSha256,
    pub runtime_id: String,
    pub prepared_cache_hit: bool,
    pub evidence_only: bool,
    pub proposal_payload: Value,
    pub provenance: ModelSourceProvenance,
    pub audit_id: OpaqueId,
}

/// Pack signer / trust READY_BASE posture (Spec 026 / Q09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackSignerDoctorStatus {
    pub present: bool,
    pub ready_base: bool,
    pub synthetic_trust_root: bool,
    pub anti_rollback: bool,
    pub release_ready: bool,
}

impl PackSignerDoctorStatus {
    #[must_use]
    pub fn ready_base() -> Self {
        Self {
            present: true,
            ready_base: true,
            synthetic_trust_root: true,
            anti_rollback: true,
            release_ready: false,
        }
    }

    #[must_use]
    pub fn is_honest_ready_base(&self) -> bool {
        self.present
            && self.ready_base
            && self.synthetic_trust_root
            && self.anti_rollback
            && !self.release_ready
    }
}

/// Admission decision reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackAdmitReason {
    Ok,
    MissingRights,
    DigestMismatch,
    ForbiddenArtifactKind,
    InvalidManifest,
    OnlinePathRefused,
    MissingSignature,
    InvalidSignature,
    UnknownTrustRoot,
    AntiRollback,
}

/// Result of local pack install.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackAdmitResult {
    pub admitted: bool,
    pub reason: PackAdmitReason,
    pub pack_id: Option<OpaqueId>,
    pub audit_id: OpaqueId,
}

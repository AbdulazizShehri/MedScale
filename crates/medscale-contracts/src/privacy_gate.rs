//! Privacy Gate contracts (Spec 079).
//!
//! One Core-owned boundary for data classification, local sensitive-span
//! recognition, de-identification transforms, reversible pseudonym maps and
//! egress decisions. Nothing here claims that PHI/PII is absent: a residual
//! scan reports only what the admitted recognizers did or did not find
//! (`security.md` T14). No type in this module carries a detected plaintext
//! value; receipts, decisions and audit rows hold ids, digests and counts.

use serde::{Deserialize, Serialize};

use crate::network::EgressDataClass;
use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};
use crate::project_graph::{ProjectRevision, check_revision, initial_revision};

/// Durable schema version for every Privacy Gate object (Spec 079 v1).
pub const PRIVACY_GATE_SCHEMA_VERSION: u32 = 1;

pub const PROFILE_NAME_MAX_CHARS: usize = 128;
pub const REID_REASON_MAX_CHARS: usize = 512;
pub const TRANSFORM_INPUT_MAX_BYTES: usize = 1_048_576;
pub const MAX_SPANS_PER_TRANSFORM: usize = 20_000;
pub const PSEUDONYM_HEX_CHARS: usize = 12;
pub const PSEUDONYM_PREFIX: &str = "PSN-";

fn bounded_text(value: &str, max_chars: usize, what: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{what} must not be empty"));
    }
    if value.chars().count() > max_chars {
        return Err(format!("{what} exceeds bound"));
    }
    if value.contains('\0') {
        return Err(format!("{what} must not contain NUL"));
    }
    Ok(())
}

/// Declares a closed string vocabulary for a fieldless enum: `as_str`,
/// `parse` and `ALL` in declaration order.
macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

/// Data class of an artifact. Declared most restrictive first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClass {
    LocalPhi,
    TeamProtected,
    ExternalDeidentified,
    Public,
}

closed_vocabulary!(DataClass, "data class", {
    LocalPhi => "local_phi",
    TeamProtected => "team_protected",
    ExternalDeidentified => "external_deidentified",
    Public => "public",
});

impl DataClass {
    /// Higher is more restrictive. `LocalPhi` is the maximum.
    #[must_use]
    pub const fn restrictiveness(self) -> u8 {
        match self {
            Self::LocalPhi => 3,
            Self::TeamProtected => 2,
            Self::ExternalDeidentified => 1,
            Self::Public => 0,
        }
    }

    /// The Spec 013 broker data class for this class, if the broker may carry
    /// it at all. `LocalPhi` and `TeamProtected` have none and are refused
    /// before any broker call.
    #[must_use]
    pub const fn broker_data_class(self) -> Option<EgressDataClass> {
        match self {
            Self::Public => Some(EgressDataClass::NonPhiMetadata),
            Self::ExternalDeidentified => Some(EgressDataClass::RedactedEvidence),
            Self::TeamProtected | Self::LocalPhi => None,
        }
    }
}

/// Why an artifact has its effective class. `DefaultUnclassified` is never
/// stored: Core reports it for an artifact that has no classification row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationBasis {
    Declared,
    DefaultUnclassified,
    DeidReceipt,
}

closed_vocabulary!(ClassificationBasis, "classification basis", {
    Declared => "declared",
    DefaultUnclassified => "default_unclassified",
    DeidReceipt => "deid_receipt",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactClassification {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub artifact_id: OpaqueId,
    pub data_class: DataClass,
    pub basis: ClassificationBasis,
    pub deid_receipt_id: Option<OpaqueId>,
}

impl ArtifactClassification {
    /// A caller-declared classification (revision 1).
    #[must_use]
    pub fn declared(
        header: ObjectHeader,
        project_id: OpaqueId,
        artifact_id: OpaqueId,
        data_class: DataClass,
    ) -> Self {
        Self {
            header,
            revision: initial_revision(),
            project_id,
            artifact_id,
            data_class,
            basis: ClassificationBasis::Declared,
            deid_receipt_id: None,
        }
    }

    /// The classification the transform path writes for its output artifact.
    #[must_use]
    pub fn from_receipt(
        header: ObjectHeader,
        project_id: OpaqueId,
        artifact_id: OpaqueId,
        data_class: DataClass,
        deid_receipt_id: OpaqueId,
    ) -> Self {
        Self {
            header,
            revision: initial_revision(),
            project_id,
            artifact_id,
            data_class,
            basis: ClassificationBasis::DeidReceipt,
            deid_receipt_id: Some(deid_receipt_id),
        }
    }

    /// Shape invariants for a stored row.
    pub fn validate(&self) -> Result<(), String> {
        match (self.basis, &self.deid_receipt_id) {
            (ClassificationBasis::DefaultUnclassified, _) => {
                Err("default_unclassified is reported, never stored".to_owned())
            }
            (ClassificationBasis::DeidReceipt, None) => {
                Err("deid_receipt basis requires deid_receipt_id".to_owned())
            }
            (ClassificationBasis::Declared, Some(_)) => {
                Err("declared basis must not carry deid_receipt_id".to_owned())
            }
            (ClassificationBasis::DeidReceipt, Some(_)) if self.data_class == DataClass::Public => {
                Err("a de-identification receipt cannot make an artifact public".to_owned())
            }
            _ => Ok(()),
        }
    }

    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

/// What Core reports for an artifact in a Project: the stored row when one
/// exists, else `LocalPhi` with basis `DefaultUnclassified` (fail closed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveClassification {
    pub project_id: OpaqueId,
    pub artifact_id: OpaqueId,
    pub data_class: DataClass,
    pub basis: ClassificationBasis,
    pub classification: Option<ArtifactClassification>,
}

impl EffectiveClassification {
    #[must_use]
    pub fn of(
        project_id: OpaqueId,
        artifact_id: OpaqueId,
        row: Option<ArtifactClassification>,
    ) -> Self {
        match row {
            Some(row) => Self {
                project_id,
                artifact_id,
                data_class: row.data_class,
                basis: row.basis,
                classification: Some(row),
            },
            None => Self {
                project_id,
                artifact_id,
                data_class: DataClass::LocalPhi,
                basis: ClassificationBasis::DefaultUnclassified,
                classification: None,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveSpanKind {
    PersonName,
    Date,
    Identifier,
    Phone,
    Email,
    PostalAddress,
    PostalCode,
    Url,
    IpAddress,
    ModelEntity,
}

closed_vocabulary!(SensitiveSpanKind, "sensitive span kind", {
    PersonName => "person_name",
    Date => "date",
    Identifier => "identifier",
    Phone => "phone",
    Email => "email",
    PostalAddress => "postal_address",
    PostalCode => "postal_code",
    Url => "url",
    IpAddress => "ip_address",
    ModelEntity => "model_entity",
});

impl SensitiveSpanKind {
    /// Upper-case label used in redaction and token placeholders.
    #[must_use]
    pub const fn placeholder_label(self) -> &'static str {
        match self {
            Self::PersonName => "PERSON_NAME",
            Self::Date => "DATE",
            Self::Identifier => "IDENTIFIER",
            Self::Phone => "PHONE",
            Self::Email => "EMAIL",
            Self::PostalAddress => "POSTAL_ADDRESS",
            Self::PostalCode => "POSTAL_CODE",
            Self::Url => "URL",
            Self::IpAddress => "IP_ADDRESS",
            Self::ModelEntity => "ENTITY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecognizerFamily {
    Deterministic,
    StructuredFhir,
    LocalModel,
}

closed_vocabulary!(RecognizerFamily, "recognizer family", {
    Deterministic => "deterministic",
    StructuredFhir => "structured_fhir",
    LocalModel => "local_model",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognizerIdentity {
    pub recognizer_id: String,
    pub version: String,
    pub family: RecognizerFamily,
    pub model_pack_id: Option<OpaqueId>,
}

impl RecognizerIdentity {
    pub fn validate(&self) -> Result<(), String> {
        bounded_text(&self.recognizer_id, 128, "recognizer_id")?;
        bounded_text(&self.version, 64, "recognizer version")?;
        match (self.family, &self.model_pack_id) {
            (RecognizerFamily::LocalModel, None) => {
                Err("local_model recognizer requires model_pack_id".to_owned())
            }
            (RecognizerFamily::Deterministic | RecognizerFamily::StructuredFhir, Some(_)) => {
                Err("only local_model recognizers carry model_pack_id".to_owned())
            }
            _ => Ok(()),
        }
    }
}

/// One detected span as byte offsets into the scanned UTF-8 text. Used in
/// memory only; never serialized into a receipt or decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitiveSpan {
    pub kind: SensitiveSpanKind,
    pub start: usize,
    pub end: usize,
    pub recognizer_id: String,
}

impl SensitiveSpan {
    pub fn validate_in(&self, text: &str) -> Result<(), String> {
        if self.start >= self.end || self.end > text.len() {
            return Err("span offsets out of range".to_owned());
        }
        if !text.is_char_boundary(self.start) || !text.is_char_boundary(self.end) {
            return Err("span offsets must be on char boundaries".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecognizerStatus {
    Completed,
    Unavailable,
    Failed,
}

closed_vocabulary!(RecognizerStatus, "recognizer status", {
    Completed => "completed",
    Unavailable => "unavailable",
    Failed => "failed",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognizerResult {
    pub recognizer: RecognizerIdentity,
    pub status: RecognizerStatus,
    pub span_count: u32,
}

// ---------------------------------------------------------------------------
// Policy profiles
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformOp {
    Redact,
    Tokenize,
    Generalize,
    Pseudonymize,
    Drop,
}

closed_vocabulary!(TransformOp, "transform op", {
    Redact => "redact",
    Tokenize => "tokenize",
    Generalize => "generalize",
    Pseudonymize => "pseudonymize",
    Drop => "drop",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyProfileStatus {
    Active,
    Revoked,
}

closed_vocabulary!(PrivacyProfileStatus, "privacy profile status", {
    Active => "active",
    Revoked => "revoked",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRule {
    pub kind: SensitiveSpanKind,
    pub op: TransformOp,
}

/// An explicit, versioned, revocable rule set. Every `SensitiveSpanKind` maps
/// to exactly one `TransformOp`; an incomplete profile is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyPolicyProfile {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub name: String,
    pub target_class: DataClass,
    pub rules: Vec<ProfileRule>,
    pub use_model_recognizer: bool,
    pub status: PrivacyProfileStatus,
}

impl PrivacyPolicyProfile {
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        name: String,
        target_class: DataClass,
        mut rules: Vec<ProfileRule>,
        use_model_recognizer: bool,
    ) -> Result<Self, String> {
        rules.sort_by_key(|rule| rule.kind);
        let profile = Self {
            header,
            revision: initial_revision(),
            project_id,
            name,
            target_class,
            rules,
            use_model_recognizer,
            status: PrivacyProfileStatus::Active,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), String> {
        bounded_text(&self.name, PROFILE_NAME_MAX_CHARS, "profile name")?;
        if !matches!(
            self.target_class,
            DataClass::ExternalDeidentified | DataClass::TeamProtected
        ) {
            return Err(
                "profile target_class must be external_deidentified or team_protected".to_owned(),
            );
        }
        for kind in SensitiveSpanKind::ALL {
            let count = self.rules.iter().filter(|rule| rule.kind == *kind).count();
            if count != 1 {
                return Err(format!(
                    "profile must map {} exactly once (found {count})",
                    kind.as_str()
                ));
            }
        }
        if self.rules.len() != SensitiveSpanKind::ALL.len() {
            return Err("profile has extra rules".to_owned());
        }
        Ok(())
    }

    /// The op for `kind`. Total on a validated profile.
    #[must_use]
    pub fn op_for(&self, kind: SensitiveSpanKind) -> Option<TransformOp> {
        self.rules
            .iter()
            .find(|rule| rule.kind == kind)
            .map(|rule| rule.op)
    }

    #[must_use]
    pub fn uses_pseudonyms(&self) -> bool {
        self.rules
            .iter()
            .any(|rule| rule.op == TransformOp::Pseudonymize)
    }

    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

// ---------------------------------------------------------------------------
// Transform receipts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidualScanStatus {
    NoResidualDetectedByAdmittedRecognizers,
    ResidualDetected,
    Unavailable,
}

closed_vocabulary!(ResidualScanStatus, "residual scan status", {
    NoResidualDetectedByAdmittedRecognizers => "no_residual_detected_by_admitted_recognizers",
    ResidualDetected => "residual_detected",
    Unavailable => "unavailable",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidualScanResult {
    pub status: ResidualScanStatus,
    pub residual_span_count: u32,
    pub recognizers: Vec<RecognizerResult>,
}

impl ResidualScanResult {
    /// Derives the status from the re-scan: any recognizer not `Completed`
    /// makes the scan `Unavailable`; otherwise any span makes it
    /// `ResidualDetected`.
    #[must_use]
    pub fn from_rescan(recognizers: Vec<RecognizerResult>) -> Self {
        let residual_span_count = recognizers.iter().map(|r| r.span_count).sum();
        let status = if recognizers.is_empty()
            || recognizers
                .iter()
                .any(|r| r.status != RecognizerStatus::Completed)
        {
            ResidualScanStatus::Unavailable
        } else if residual_span_count > 0 {
            ResidualScanStatus::ResidualDetected
        } else {
            ResidualScanStatus::NoResidualDetectedByAdmittedRecognizers
        };
        Self {
            status,
            residual_span_count,
            recognizers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformOpCount {
    pub kind: SensitiveSpanKind,
    pub op: TransformOp,
    pub count: u32,
}

/// Fixed limitation vocabulary attached to every receipt. Nothing here says
/// or implies that sensitive data is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptLimitation {
    /// Automated recognition can miss sensitive values.
    AutomatedRecognitionIsIncomplete,
    /// Recognizers were evaluated on synthetic fixtures only.
    SyntheticEvaluationOnly,
    /// The local model recognizer is a fixture Pack, not a qualified PHI model.
    ModelRecognizerNotQualified,
    /// Scripts other than Latin were not evaluated.
    NonLatinScriptsNotEvaluated,
    /// Narrative text inside structured records is scanned only by pattern rules.
    StructuredNarrativePatternOnly,
}

closed_vocabulary!(ReceiptLimitation, "receipt limitation", {
    AutomatedRecognitionIsIncomplete => "automated_recognition_is_incomplete",
    SyntheticEvaluationOnly => "synthetic_evaluation_only",
    ModelRecognizerNotQualified => "model_recognizer_not_qualified",
    NonLatinScriptsNotEvaluated => "non_latin_scripts_not_evaluated",
    StructuredNarrativePatternOnly => "structured_narrative_pattern_only",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeidReceiptStatus {
    Valid,
    Revoked,
}

closed_vocabulary!(DeidReceiptStatus, "deid receipt status", {
    Valid => "valid",
    Revoked => "revoked",
});

/// The persisted record of one privacy transform: one source, one new
/// output artifact, one receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeidReceipt {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub source_artifact_id: OpaqueId,
    pub source_digest: DigestSha256,
    pub output_artifact_id: OpaqueId,
    pub output_digest: DigestSha256,
    pub output_class: DataClass,
    pub profile_id: OpaqueId,
    pub profile_revision: ProjectRevision,
    pub recognizers: Vec<RecognizerResult>,
    pub op_counts: Vec<TransformOpCount>,
    pub pseudonym_map_id: Option<OpaqueId>,
    pub residual: ResidualScanResult,
    pub limitations: Vec<ReceiptLimitation>,
    pub status: DeidReceiptStatus,
}

impl DeidReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_artifact_id == self.output_artifact_id {
            return Err("output artifact must differ from source artifact".to_owned());
        }
        if !matches!(
            self.output_class,
            DataClass::ExternalDeidentified | DataClass::TeamProtected
        ) {
            return Err(
                "receipt output_class must be external_deidentified or team_protected".to_owned(),
            );
        }
        if self.recognizers.is_empty() {
            return Err("receipt must name at least one recognizer".to_owned());
        }
        for result in self.recognizers.iter().chain(&self.residual.recognizers) {
            result.recognizer.validate()?;
        }
        if !self
            .limitations
            .contains(&ReceiptLimitation::AutomatedRecognitionIsIncomplete)
        {
            return Err("receipt must state that automated recognition is incomplete".to_owned());
        }
        let pseudonymized = self
            .op_counts
            .iter()
            .any(|c| c.op == TransformOp::Pseudonymize && c.count > 0);
        if pseudonymized && self.pseudonym_map_id.is_none() {
            return Err("pseudonymized output requires pseudonym_map_id".to_owned());
        }
        Ok(())
    }

    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

// ---------------------------------------------------------------------------
// Pseudonym maps
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PseudonymMapStatus {
    Active,
    Revoked,
}

closed_vocabulary!(PseudonymMapStatus, "pseudonym map status", {
    Active => "active",
    Revoked => "revoked",
});

/// A reversible pseudonym map. `key_account` names the `KeyStore` entry
/// that holds the map key; the key itself is never part of this record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PseudonymMapRef {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub key_account: String,
    pub entry_count: u32,
    pub status: PseudonymMapStatus,
}

impl PseudonymMapRef {
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

/// True when `value` has the exact pseudonym shape `PSN-` + 12 lowercase hex.
#[must_use]
pub fn is_pseudonym(value: &str) -> bool {
    value.len() == PSEUDONYM_PREFIX.len() + PSEUDONYM_HEX_CHARS
        && value.starts_with(PSEUDONYM_PREFIX)
        && value[PSEUDONYM_PREFIX.len()..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReidentificationOutcome {
    Returned,
    DeniedRevoked,
    DeniedUnknownPseudonym,
    DeniedKeyUnavailable,
}

closed_vocabulary!(ReidentificationOutcome, "reidentification outcome", {
    Returned => "returned",
    DeniedRevoked => "denied_revoked",
    DeniedUnknownPseudonym => "denied_unknown_pseudonym",
    DeniedKeyUnavailable => "denied_key_unavailable",
});

/// Append-only audit of one re-identification request. Holds the pseudonym
/// (already non-identifying) and the reason, never the resolved value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReidentificationAudit {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub map_id: OpaqueId,
    pub pseudonym: String,
    pub requested_by: OpaqueId,
    pub reason: String,
    pub outcome: ReidentificationOutcome,
}

impl ReidentificationAudit {
    pub fn validate(&self) -> Result<(), String> {
        bounded_text(
            &self.reason,
            REID_REASON_MAX_CHARS,
            "re-identification reason",
        )?;
        if !is_pseudonym(&self.pseudonym) {
            return Err("audit pseudonym has an invalid shape".to_owned());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Egress
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressBoundary {
    ModelExternalDelegate,
    Browse,
    DataSourceExport,
    DataSourceWrite,
    Hub,
    Compute,
    RWorkspace,
    Connector,
    Extension,
    AnalyticsAdapter,
    NetworkBroker,
}

closed_vocabulary!(EgressBoundary, "egress boundary", {
    ModelExternalDelegate => "model_external_delegate",
    Browse => "browse",
    DataSourceExport => "data_source_export",
    DataSourceWrite => "data_source_write",
    Hub => "hub",
    Compute => "compute",
    RWorkspace => "r_workspace",
    Connector => "connector",
    Extension => "extension",
    AnalyticsAdapter => "analytics_adapter",
    NetworkBroker => "network_broker",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressOutcome {
    Allow,
    Deny,
}

closed_vocabulary!(EgressOutcome, "egress outcome", {
    Allow => "allow",
    Deny => "deny",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressReason {
    AllowedPublic,
    AllowedDeidentified,
    AllowedTeamHub,
    DeniedLocalPhi,
    DeniedUnclassified,
    DeniedTeamProtectedOffHub,
    DeniedNoReceipt,
    DeniedReceiptRevoked,
    DeniedDigestMismatch,
    DeniedResidualDetected,
    DeniedResidualUnavailable,
    DeniedProfileRevoked,
}

closed_vocabulary!(EgressReason, "egress reason", {
    AllowedPublic => "allowed_public",
    AllowedDeidentified => "allowed_deidentified",
    AllowedTeamHub => "allowed_team_hub",
    DeniedLocalPhi => "denied_local_phi",
    DeniedUnclassified => "denied_unclassified",
    DeniedTeamProtectedOffHub => "denied_team_protected_off_hub",
    DeniedNoReceipt => "denied_no_receipt",
    DeniedReceiptRevoked => "denied_receipt_revoked",
    DeniedDigestMismatch => "denied_digest_mismatch",
    DeniedResidualDetected => "denied_residual_detected",
    DeniedResidualUnavailable => "denied_residual_unavailable",
    DeniedProfileRevoked => "denied_profile_revoked",
});

impl EgressReason {
    #[must_use]
    pub const fn outcome(self) -> EgressOutcome {
        match self {
            Self::AllowedPublic | Self::AllowedDeidentified | Self::AllowedTeamHub => {
                EgressOutcome::Allow
            }
            _ => EgressOutcome::Deny,
        }
    }
}

/// Facts the egress policy needs about a de-identification receipt, gathered
/// by Core (including a fresh digest of the stored output bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptEvidence {
    pub receipt_status: DeidReceiptStatus,
    pub profile_status: PrivacyProfileStatus,
    pub residual: ResidualScanStatus,
    pub output_digest_matches: bool,
}

/// The single egress policy. Pure: Core gathers the facts, this decides.
/// Order of checks is fixed so the reported reason is deterministic.
#[must_use]
pub fn decide_egress(
    boundary: EgressBoundary,
    data_class: DataClass,
    basis: ClassificationBasis,
    receipt: Option<&ReceiptEvidence>,
) -> EgressReason {
    if basis == ClassificationBasis::DefaultUnclassified {
        return EgressReason::DeniedUnclassified;
    }
    match data_class {
        DataClass::LocalPhi => EgressReason::DeniedLocalPhi,
        DataClass::Public => EgressReason::AllowedPublic,
        DataClass::TeamProtected if boundary != EgressBoundary::Hub => {
            EgressReason::DeniedTeamProtectedOffHub
        }
        DataClass::TeamProtected | DataClass::ExternalDeidentified => {
            if basis != ClassificationBasis::DeidReceipt {
                // A declared (not transformed) non-public class may go only
                // where its class allows without a receipt: TeamProtected to
                // the Hub. ExternalDeidentified must be backed by a receipt.
                return if data_class == DataClass::TeamProtected {
                    EgressReason::AllowedTeamHub
                } else {
                    EgressReason::DeniedNoReceipt
                };
            }
            let Some(evidence) = receipt else {
                return EgressReason::DeniedNoReceipt;
            };
            if evidence.receipt_status == DeidReceiptStatus::Revoked {
                return EgressReason::DeniedReceiptRevoked;
            }
            if evidence.profile_status == PrivacyProfileStatus::Revoked {
                return EgressReason::DeniedProfileRevoked;
            }
            if !evidence.output_digest_matches {
                return EgressReason::DeniedDigestMismatch;
            }
            match evidence.residual {
                ResidualScanStatus::Unavailable => EgressReason::DeniedResidualUnavailable,
                ResidualScanStatus::ResidualDetected => EgressReason::DeniedResidualDetected,
                ResidualScanStatus::NoResidualDetectedByAdmittedRecognizers => {
                    if data_class == DataClass::TeamProtected {
                        EgressReason::AllowedTeamHub
                    } else {
                        EgressReason::AllowedDeidentified
                    }
                }
            }
        }
    }
}

/// A persisted egress decision (append-only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EgressDecision {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub artifact_id: OpaqueId,
    pub boundary: EgressBoundary,
    pub data_class: DataClass,
    pub basis: ClassificationBasis,
    pub outcome: EgressOutcome,
    pub reason: EgressReason,
    pub deid_receipt_id: Option<OpaqueId>,
}

impl EgressDecision {
    pub fn validate(&self) -> Result<(), String> {
        if self.reason.outcome() != self.outcome {
            return Err("egress outcome does not match its reason".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{AuthorityScopeId, RealmId};

    fn h(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: PRIVACY_GATE_SCHEMA_VERSION,
            realm_id: RealmId::new("realm"),
            authority_scope_id: AuthorityScopeId::new("scope"),
        }
    }

    fn full_rules(op: TransformOp) -> Vec<ProfileRule> {
        SensitiveSpanKind::ALL
            .iter()
            .map(|kind| ProfileRule { kind: *kind, op })
            .collect()
    }

    fn profile(rules: Vec<ProfileRule>) -> Result<PrivacyPolicyProfile, String> {
        PrivacyPolicyProfile::new(
            h("prof"),
            OpaqueId::new("proj"),
            "Research export".to_owned(),
            DataClass::ExternalDeidentified,
            rules,
            false,
        )
    }

    fn completed(id: &str, spans: u32) -> RecognizerResult {
        RecognizerResult {
            recognizer: RecognizerIdentity {
                recognizer_id: id.to_owned(),
                version: "1".to_owned(),
                family: RecognizerFamily::Deterministic,
                model_pack_id: None,
            },
            status: RecognizerStatus::Completed,
            span_count: spans,
        }
    }

    fn receipt() -> DeidReceipt {
        DeidReceipt {
            header: h("rcpt"),
            revision: initial_revision(),
            project_id: OpaqueId::new("proj"),
            source_artifact_id: OpaqueId::new("src"),
            source_digest: DigestSha256::of(b"source"),
            output_artifact_id: OpaqueId::new("out"),
            output_digest: DigestSha256::of(b"output"),
            output_class: DataClass::ExternalDeidentified,
            profile_id: OpaqueId::new("prof"),
            profile_revision: initial_revision(),
            recognizers: vec![completed("pattern", 3)],
            op_counts: vec![TransformOpCount {
                kind: SensitiveSpanKind::Email,
                op: TransformOp::Redact,
                count: 3,
            }],
            pseudonym_map_id: None,
            residual: ResidualScanResult::from_rescan(vec![completed("pattern", 0)]),
            limitations: vec![ReceiptLimitation::AutomatedRecognitionIsIncomplete],
            status: DeidReceiptStatus::Valid,
        }
    }

    macro_rules! assert_round_trips {
        ($($ty:ty),+) => {
            $(for value in <$ty>::ALL {
                assert_eq!(<$ty>::parse(value.as_str()).unwrap(), *value);
                let json = serde_json::to_string(value).unwrap();
                assert_eq!(json, format!("\"{}\"", value.as_str()));
            }
            assert!(<$ty>::parse("nope").is_err());)+
        };
    }

    #[test]
    fn every_vocabulary_round_trips_and_is_closed() {
        assert_round_trips!(
            DataClass,
            ClassificationBasis,
            SensitiveSpanKind,
            RecognizerFamily,
            RecognizerStatus,
            TransformOp,
            PrivacyProfileStatus,
            ResidualScanStatus,
            ReceiptLimitation,
            DeidReceiptStatus,
            PseudonymMapStatus,
            ReidentificationOutcome,
            EgressBoundary,
            EgressOutcome,
            EgressReason
        );
    }

    #[test]
    fn data_class_restrictiveness_is_strictly_ordered() {
        let levels: Vec<u8> = DataClass::ALL.iter().map(|c| c.restrictiveness()).collect();
        assert_eq!(levels, vec![3, 2, 1, 0]);
        assert_eq!(DataClass::LocalPhi.broker_data_class(), None);
        assert_eq!(DataClass::TeamProtected.broker_data_class(), None);
        assert_eq!(
            DataClass::ExternalDeidentified.broker_data_class(),
            Some(EgressDataClass::RedactedEvidence)
        );
    }

    #[test]
    fn classification_basis_invariants_hold() {
        let mut row = ArtifactClassification::declared(
            h("c1"),
            OpaqueId::new("proj"),
            OpaqueId::new("art"),
            DataClass::Public,
        );
        assert!(row.validate().is_ok());
        row.deid_receipt_id = Some(OpaqueId::new("r"));
        assert!(row.validate().is_err());
        row.basis = ClassificationBasis::DefaultUnclassified;
        assert!(row.validate().is_err());
        let receipt_row = ArtifactClassification::from_receipt(
            h("c2"),
            OpaqueId::new("proj"),
            OpaqueId::new("art"),
            DataClass::ExternalDeidentified,
            OpaqueId::new("r"),
        );
        assert!(receipt_row.validate().is_ok());
        let mut public_from_receipt = receipt_row.clone();
        public_from_receipt.data_class = DataClass::Public;
        assert!(public_from_receipt.validate().is_err());
        let mut missing = receipt_row;
        missing.deid_receipt_id = None;
        assert!(missing.validate().is_err());
    }

    #[test]
    fn profile_must_map_every_kind_exactly_once() {
        assert!(profile(full_rules(TransformOp::Redact)).is_ok());
        let mut incomplete = full_rules(TransformOp::Redact);
        incomplete.pop();
        assert!(profile(incomplete).is_err());
        let mut duplicate = full_rules(TransformOp::Redact);
        duplicate.push(ProfileRule {
            kind: SensitiveSpanKind::Email,
            op: TransformOp::Drop,
        });
        assert!(profile(duplicate).is_err());
        let mut public = profile(full_rules(TransformOp::Redact)).unwrap();
        public.target_class = DataClass::Public;
        assert!(public.validate().is_err());
        let mut long = profile(full_rules(TransformOp::Redact)).unwrap();
        long.name = "x".repeat(PROFILE_NAME_MAX_CHARS + 1);
        assert!(long.validate().is_err());
    }

    #[test]
    fn profile_op_lookup_and_pseudonym_flag() {
        let mut rules = full_rules(TransformOp::Redact);
        rules[0].op = TransformOp::Pseudonymize;
        let p = profile(rules).unwrap();
        assert!(p.uses_pseudonyms());
        for kind in SensitiveSpanKind::ALL {
            assert!(p.op_for(*kind).is_some());
        }
    }

    #[test]
    fn residual_scan_never_reports_clean_when_a_recognizer_did_not_complete() {
        assert_eq!(
            ResidualScanResult::from_rescan(vec![completed("a", 0)]).status,
            ResidualScanStatus::NoResidualDetectedByAdmittedRecognizers
        );
        assert_eq!(
            ResidualScanResult::from_rescan(vec![completed("a", 2)]).status,
            ResidualScanStatus::ResidualDetected
        );
        let mut unavailable = completed("m", 0);
        unavailable.status = RecognizerStatus::Unavailable;
        assert_eq!(
            ResidualScanResult::from_rescan(vec![completed("a", 0), unavailable]).status,
            ResidualScanStatus::Unavailable
        );
        assert_eq!(
            ResidualScanResult::from_rescan(vec![]).status,
            ResidualScanStatus::Unavailable
        );
    }

    #[test]
    fn receipt_invariants_hold() {
        assert!(receipt().validate().is_ok());
        let mut same = receipt();
        same.output_artifact_id = same.source_artifact_id.clone();
        assert!(same.validate().is_err());
        let mut no_limitation = receipt();
        no_limitation.limitations.clear();
        assert!(no_limitation.validate().is_err());
        let mut pseudo = receipt();
        pseudo.op_counts[0].op = TransformOp::Pseudonymize;
        assert!(pseudo.validate().is_err());
        pseudo.pseudonym_map_id = Some(OpaqueId::new("map"));
        assert!(pseudo.validate().is_ok());
        let mut public = receipt();
        public.output_class = DataClass::Public;
        assert!(public.validate().is_err());
    }

    #[test]
    fn limitation_vocabulary_makes_no_absence_claim() {
        for limitation in ReceiptLimitation::ALL {
            let text = limitation.as_str();
            for banned in [
                "free",
                "absent",
                "removed",
                "clean",
                "anonymi",
                "compliant",
                "safe",
            ] {
                assert!(!text.contains(banned), "{text} contains {banned}");
            }
        }
        for status in ResidualScanStatus::ALL {
            assert!(!status.as_str().contains("clean"));
            assert!(!status.as_str().contains("free"));
        }
    }

    #[test]
    fn pseudonym_shape_is_exact() {
        assert!(is_pseudonym("PSN-0123456789ab"));
        assert!(!is_pseudonym("PSN-0123456789AB"));
        assert!(!is_pseudonym("PSN-0123456789a"));
        assert!(!is_pseudonym("XYZ-0123456789ab"));
    }

    #[test]
    fn egress_policy_matrix_fails_closed() {
        let good = ReceiptEvidence {
            receipt_status: DeidReceiptStatus::Valid,
            profile_status: PrivacyProfileStatus::Active,
            residual: ResidualScanStatus::NoResidualDetectedByAdmittedRecognizers,
            output_digest_matches: true,
        };
        for boundary in EgressBoundary::ALL {
            let b = *boundary;
            assert_eq!(
                decide_egress(
                    b,
                    DataClass::LocalPhi,
                    ClassificationBasis::DefaultUnclassified,
                    None
                ),
                EgressReason::DeniedUnclassified
            );
            assert_eq!(
                decide_egress(b, DataClass::LocalPhi, ClassificationBasis::Declared, None),
                EgressReason::DeniedLocalPhi
            );
            assert_eq!(
                decide_egress(b, DataClass::Public, ClassificationBasis::Declared, None),
                EgressReason::AllowedPublic
            );
            assert_eq!(
                decide_egress(
                    b,
                    DataClass::ExternalDeidentified,
                    ClassificationBasis::Declared,
                    None
                ),
                EgressReason::DeniedNoReceipt
            );
            assert_eq!(
                decide_egress(
                    b,
                    DataClass::ExternalDeidentified,
                    ClassificationBasis::DeidReceipt,
                    Some(&good)
                ),
                EgressReason::AllowedDeidentified
            );
            let team = decide_egress(
                b,
                DataClass::TeamProtected,
                ClassificationBasis::Declared,
                None,
            );
            if b == EgressBoundary::Hub {
                assert_eq!(team, EgressReason::AllowedTeamHub);
            } else {
                assert_eq!(team, EgressReason::DeniedTeamProtectedOffHub);
            }
        }
        let cases = [
            (
                ReceiptEvidence {
                    receipt_status: DeidReceiptStatus::Revoked,
                    ..good.clone()
                },
                EgressReason::DeniedReceiptRevoked,
            ),
            (
                ReceiptEvidence {
                    profile_status: PrivacyProfileStatus::Revoked,
                    ..good.clone()
                },
                EgressReason::DeniedProfileRevoked,
            ),
            (
                ReceiptEvidence {
                    output_digest_matches: false,
                    ..good.clone()
                },
                EgressReason::DeniedDigestMismatch,
            ),
            (
                ReceiptEvidence {
                    residual: ResidualScanStatus::ResidualDetected,
                    ..good.clone()
                },
                EgressReason::DeniedResidualDetected,
            ),
            (
                ReceiptEvidence {
                    residual: ResidualScanStatus::Unavailable,
                    ..good.clone()
                },
                EgressReason::DeniedResidualUnavailable,
            ),
        ];
        for (evidence, expected) in cases {
            let got = decide_egress(
                EgressBoundary::Browse,
                DataClass::ExternalDeidentified,
                ClassificationBasis::DeidReceipt,
                Some(&evidence),
            );
            assert_eq!(got, expected);
            assert_eq!(got.outcome(), EgressOutcome::Deny);
        }
        assert_eq!(
            decide_egress(
                EgressBoundary::Browse,
                DataClass::ExternalDeidentified,
                ClassificationBasis::DeidReceipt,
                None
            ),
            EgressReason::DeniedNoReceipt
        );
    }

    #[test]
    fn egress_decision_outcome_must_match_reason() {
        let mut decision = EgressDecision {
            header: h("d"),
            project_id: OpaqueId::new("proj"),
            artifact_id: OpaqueId::new("art"),
            boundary: EgressBoundary::Browse,
            data_class: DataClass::LocalPhi,
            basis: ClassificationBasis::Declared,
            outcome: EgressOutcome::Deny,
            reason: EgressReason::DeniedLocalPhi,
            deid_receipt_id: None,
        };
        assert!(decision.validate().is_ok());
        decision.outcome = EgressOutcome::Allow;
        assert!(decision.validate().is_err());
    }

    #[test]
    fn span_validation_rejects_bad_offsets() {
        let text = "caf\u{e9} x";
        let span = |start, end| SensitiveSpan {
            kind: SensitiveSpanKind::PersonName,
            start,
            end,
            recognizer_id: "r".to_owned(),
        };
        assert!(span(0, 3).validate_in(text).is_ok());
        assert!(span(0, 4).validate_in(text).is_err());
        assert!(span(3, 3).validate_in(text).is_err());
        assert!(span(0, 99).validate_in(text).is_err());
    }

    #[test]
    fn reidentification_audit_is_bounded() {
        let mut audit = ReidentificationAudit {
            header: h("a"),
            project_id: OpaqueId::new("proj"),
            map_id: OpaqueId::new("map"),
            pseudonym: "PSN-0123456789ab".to_owned(),
            requested_by: OpaqueId::new("holder"),
            reason: "study follow-up".to_owned(),
            outcome: ReidentificationOutcome::Returned,
        };
        assert!(audit.validate().is_ok());
        audit.reason = "x".repeat(REID_REASON_MAX_CHARS + 1);
        assert!(audit.validate().is_err());
        audit.reason = "ok".to_owned();
        audit.pseudonym = "Jane".to_owned();
        assert!(audit.validate().is_err());
    }
}

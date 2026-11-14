//! Document / OCR / ASR contracts (Spec 010) — stubs + MIME quarantine.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, OpaqueId};

/// Declared MIME class for intake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentMimeClass {
    TextPlain,
    ApplicationPdf,
    ImagePng,
    ImageJpeg,
    AudioWav,
    AudioMpeg,
    /// Always denied in Spec 010 default matrix.
    ApplicationOctetStream,
    /// Always denied (active content).
    TextHtml,
    /// Always denied until sandboxed office worker.
    OfficeOpenXml,
}

impl DocumentMimeClass {
    #[must_use]
    pub const fn is_denied_by_default(self) -> bool {
        matches!(
            self,
            Self::ApplicationOctetStream | Self::TextHtml | Self::OfficeOpenXml
        )
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TextPlain => "text/plain",
            Self::ApplicationPdf => "application/pdf",
            Self::ImagePng => "image/png",
            Self::ImageJpeg => "image/jpeg",
            Self::AudioWav => "audio/wav",
            Self::AudioMpeg => "audio/mpeg",
            Self::ApplicationOctetStream => "application/octet-stream",
            Self::TextHtml => "text/html",
            Self::OfficeOpenXml => "application/vnd.openxmlformats-officedocument",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineDecision {
    Admit,
    Quarantine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentDenyReason {
    Ok,
    MimeDenied,
    EmptyPayload,
    WorkerAmbientViolation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentIntakeRequest {
    pub mime: DocumentMimeClass,
    pub bytes: Vec<u8>,
    pub fixture_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentIntakeResult {
    pub decision: QuarantineDecision,
    pub reason: DocumentDenyReason,
    pub source_id: Option<OpaqueId>,
    pub content_digest: Option<DigestSha256>,
    pub audit_id: OpaqueId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrStubRequest {
    pub source_id: OpaqueId,
    pub fixture_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsrStubRequest {
    pub source_id: OpaqueId,
    pub fixture_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaStubResult {
    pub derived_id: OpaqueId,
    pub proposal_id: OpaqueId,
    pub evidence_only: bool,
    pub text: String,
}

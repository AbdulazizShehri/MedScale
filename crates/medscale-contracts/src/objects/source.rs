//! Source and derived source artifact types.

use serde::{Deserialize, Serialize};

use super::{DigestSha256, MedicalTime, ObjectHeader, OpaqueId};

/// Loss classification for a derived transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LossClass {
    Lossless,
    Lossy,
    Unknown,
}

/// Extensible representation kind for derived artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationKind {
    NormalizedText,
    TokenStream,
    Other(String),
}

/// Immutable raw evidence record. Source identity is `header.id`, not `content_digest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub header: ObjectHeader,
    pub bytes: Vec<u8>,
    pub content_digest: DigestSha256,
    pub media_type: String,
    pub acquired_at: Option<MedicalTime>,
    pub provenance_note: Option<String>,
}

impl SourceRecord {
    /// Returns true when `content_digest` matches `bytes`.
    #[must_use]
    pub fn digest_valid(&self) -> bool {
        self.content_digest == DigestSha256::of(&self.bytes)
    }
}

/// Versioned transform of a source. Cannot mutate the parent source bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedSourceArtifact {
    pub header: ObjectHeader,
    pub source_id: OpaqueId,
    pub transform_id: String,
    pub transform_version: String,
    pub loss_class: LossClass,
    pub representation: RepresentationKind,
    pub bytes: Vec<u8>,
    pub content_digest: DigestSha256,
    pub parent_span_map_ref: Option<OpaqueId>,
}

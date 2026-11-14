//! Evidence / lexical retrieval contracts (Specs 011 + 025).
//!
//! Relevance is never clinical authority. Corpus manifests are synthetic-owned
//! fixtures with source identity distinct from content digests.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, OpaqueId};

/// Rights class for an admitted lexical evidence corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCorpusRights {
    /// MedScale-owned synthetic fixture; not licensed external literature.
    SyntheticOwned,
}

/// Document lifecycle within an evidence corpus (metadata only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDocStatus {
    Active,
    Retracted,
}

/// One document entry in a versioned evidence corpus manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCorpusDocument {
    pub doc_id: String,
    pub text: String,
    pub content_digest: DigestSha256,
    pub status: EvidenceDocStatus,
    /// Freshness marker (evidence metadata; not clinical authority).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness_marker: Option<String>,
    /// Conflict marker (evidence metadata; not clinical authority).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_marker: Option<String>,
}

/// Pack-like local corpus manifest (source identity ≠ content digest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCorpusManifest {
    /// Stable corpus source identity (not a content hash).
    pub corpus_id: String,
    /// Semantic corpus version; bump changes source identity binding.
    pub version: String,
    /// Content digest over sorted document digests (evidence metadata only).
    pub content_digest: DigestSha256,
    pub rights: EvidenceCorpusRights,
    pub rights_uri: String,
    pub documents: Vec<EvidenceCorpusDocument>,
}

impl EvidenceCorpusManifest {
    /// Source identity string: `{corpus_id}@{version}` (not content hash).
    #[must_use]
    pub fn source_identity(&self) -> String {
        format!("{}@{}", self.corpus_id, self.version)
    }
}

/// Doctor axis for evidence corpus readiness (Spec 025 / Q10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCorpusDoctorStatus {
    pub present: bool,
    pub ready_base: bool,
    pub versioned_corpus: bool,
    pub synthetic_owned_only: bool,
    /// Always false in Spec 025 — relevance/corpus is not clinical quality.
    pub clinical_quality_claimed: bool,
    /// Always false — Spec 025 does not establish RELEASE_READY.
    pub release_ready: bool,
    /// Spec 045: procedural scale corpus generator present (not clinical quality).
    pub scale_corpus_generator_present: bool,
    pub current_corpus_id: Option<String>,
    pub current_version: Option<String>,
}

impl EvidenceCorpusDoctorStatus {
    /// Spec 025/045 READY_BASE honesty for the synthetic versioned corpus.
    #[must_use]
    pub fn ready_base(corpus_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            present: true,
            ready_base: true,
            versioned_corpus: true,
            synthetic_owned_only: true,
            clinical_quality_claimed: false,
            release_ready: false,
            scale_corpus_generator_present: true,
            current_corpus_id: Some(corpus_id.into()),
            current_version: Some(version.into()),
        }
    }
}

/// One ranked retrieval hit (relevance only — never authority).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalHit {
    pub doc_id: OpaqueId,
    pub score: f64,
    pub snippet: String,
    pub relevance_only: bool,
    /// True when the hit came from a retracted document (only if filter allowed).
    #[serde(default)]
    pub retracted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness_marker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_marker: Option<String>,
    /// Corpus version that produced this hit (evidence metadata).
    #[serde(default)]
    pub corpus_version: String,
}

/// Request for deterministic lexical retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalRetrieveRequest {
    pub query: String,
    /// Corpus source id (`synthetic-lexical`) or exact `id@version`.
    pub corpus_id: String,
    pub max_hits: u32,
    /// When false (default), retracted documents are excluded from ranking.
    #[serde(default)]
    pub include_retracted: bool,
}

/// Response envelope before persistence as EvaluationRecord.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalRetrieveResult {
    pub corpus_id: String,
    pub corpus_version: String,
    pub corpus_source_identity: String,
    pub query: String,
    pub hits: Vec<RetrievalHit>,
    pub evaluation_id: OpaqueId,
    pub evidence_only: bool,
    /// Explicit constitutional reminder.
    pub relevance_is_not_authority: bool,
}

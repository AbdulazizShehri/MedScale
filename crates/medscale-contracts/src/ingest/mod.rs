//! H0-A ingest / durability contract types.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, OpaqueId};

/// Outcome of a synthetic FHIR ingest attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestOutcome {
    Accepted,
    Duplicate,
    Rejected,
    Quarantined,
}

/// Reference to a content-addressed blob (digest is evidence, not source id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRef {
    pub digest: DigestSha256,
    pub byte_length: u64,
}

/// Durable ingest receipt returned to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestReceipt {
    pub receipt_id: OpaqueId,
    pub outcome: IngestOutcome,
    pub source_id: Option<OpaqueId>,
    pub content_digest: Option<DigestSha256>,
    pub byte_length: Option<u64>,
    pub blob_ref: Option<BlobRef>,
    pub evaluation_refs: Vec<OpaqueId>,
    pub identity_refs: Vec<OpaqueId>,
    pub error: Option<String>,
}

/// Blob lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlobState {
    Live,
    Tombstoned,
    Quarantined,
}

/// Filesystem claim scope note for durability evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemClaimScope {
    pub os: String,
    pub path_class: String,
    pub sync_root_refused: bool,
    pub note: String,
}

/// Backup manifest (synthetic unencrypted H0-A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub schema_version: u32,
    pub vault_id: String,
    pub created_at: String,
    pub metadata_snapshot_digest: DigestSha256,
    pub blob_entries: Vec<BlobRef>,
    pub claim_scope_note: String,
}

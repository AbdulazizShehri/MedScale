//! Opaque identifiers and object headers.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Vault-scoped opaque object identifier (not a content hash).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpaqueId(String);

impl OpaqueId {
    /// Creates an opaque id from an owned string.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Vault identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VaultId(String);

impl VaultId {
    /// Creates a vault id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// SHA-256 content digest (evidence metadata only; never source identity).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DigestSha256([u8; 32]);

impl DigestSha256 {
    /// Digests raw bytes with SHA-256.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut out = [0_u8; 32];
        out.copy_from_slice(&digest);
        Self(out)
    }

    /// Constructs a digest from raw SHA-256 bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex encoding of the digest.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Discriminator for durable object classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectClass {
    SourceRecord,
    DerivedSourceArtifact,
    Proposal,
    ClinicalAssertion,
    EvaluationRecord,
    ActionAuditRecord,
    Projection,
    IdentityAssertion,
    IdentityMergeDecision,
    AmendmentRecord,
}

/// Common header fields for authority-bearing objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectHeader {
    pub id: OpaqueId,
    pub schema_version: u32,
    pub realm_id: super::RealmId,
    pub authority_scope_id: super::AuthorityScopeId,
}

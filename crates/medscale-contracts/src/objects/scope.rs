//! Realm and authority scope identifiers.

use serde::{Deserialize, Serialize};

use super::OpaqueId;

/// Explicit realm identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RealmId(OpaqueId);

impl RealmId {
    /// Creates a realm id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(OpaqueId::new(value))
    }

    /// Returns the underlying opaque id.
    #[must_use]
    pub fn as_opaque(&self) -> &OpaqueId {
        &self.0
    }
}

/// Opaque authorization scope (not an org-chart leak).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthorityScopeId(OpaqueId);

impl AuthorityScopeId {
    /// Creates an authority scope id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(OpaqueId::new(value))
    }

    /// Returns the underlying opaque id.
    #[must_use]
    pub fn as_opaque(&self) -> &OpaqueId {
        &self.0
    }
}

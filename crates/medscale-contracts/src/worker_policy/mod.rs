//! Worker supervision policy stubs (deny-by-default).

use serde::{Deserialize, Serialize};

/// Explicit capability grant to a worker (expiring, enumerated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerCapabilityGrant {
    pub name: String,
    pub expires_at_unix_ms: Option<u64>,
}

/// Supervision policy: ambient privileges default deny.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerSupervisionPolicy {
    pub allow_ambient_canonical_db: bool,
    pub allow_master_keys: bool,
    pub allow_unrestricted_filesystem: bool,
    pub allow_network: bool,
    pub allow_secrets: bool,
    pub allow_authority: bool,
    /// Profile id for evidence (Spec 008); OS PLATFORM_QUALIFIED is separate.
    pub confinement_profile: String,
    pub grants: Vec<WorkerCapabilityGrant>,
}

impl WorkerSupervisionPolicy {
    /// Deny-by-default policy with empty grants.
    #[must_use]
    pub fn deny_by_default() -> Self {
        Self {
            allow_ambient_canonical_db: false,
            allow_master_keys: false,
            allow_unrestricted_filesystem: false,
            allow_network: false,
            allow_secrets: false,
            allow_authority: false,
            confinement_profile: "policy_ambient_deny_v0".to_owned(),
            grants: Vec::new(),
        }
    }

    /// Bind confinement profile id to OS sandbox scaffold evidence (not PLATFORM_QUALIFIED).
    #[must_use]
    pub fn with_os_sandbox_scaffold_profile(mut self) -> Self {
        self.confinement_profile = "os_sandbox_scaffold_not_platform_qualified".to_owned();
        self
    }

    /// Returns true when no ambient privilege is enabled.
    #[must_use]
    pub fn ambient_denied(&self) -> bool {
        !self.allow_ambient_canonical_db
            && !self.allow_master_keys
            && !self.allow_unrestricted_filesystem
            && !self.allow_network
            && !self.allow_secrets
            && !self.allow_authority
    }
}

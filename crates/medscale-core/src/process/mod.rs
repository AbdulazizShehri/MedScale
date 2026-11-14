//! Single-writer Core Host lease simulator + client sessions.

mod session;

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use medscale_contracts::objects::{OpaqueId, VaultId};
use thiserror::Error;

pub use session::SessionRegistry;

/// Spec 024 session enforcement for mutating authority calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionEnforcement {
    /// Fail-closed: mutating capabilities require a live `session_id` (default / IPC).
    #[default]
    Strict,
    /// Engineering/test escape: Spec 018 lease-only mutation without `session_id`.
    LegacyLeaseOnlyEngineering,
}

/// Errors from lease acquire/release.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LeaseError {
    #[error("lease already held")]
    AlreadyHeld { holder_id: OpaqueId },
    #[error("caller is not the holder")]
    NotHolder,
    #[error("no lease held")]
    NotHeld,
}

/// In-process per-vault exclusive lease registry.
#[derive(Debug, Default)]
pub struct LeaseRegistry {
    inner: Mutex<HashMap<String, OpaqueId>>,
}

impl LeaseRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, OpaqueId>> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Acquires an exclusive lease for `vault_id`.
    pub fn acquire(
        &self,
        vault_id: &VaultId,
        client_id: OpaqueId,
        holder_id_hint: Option<OpaqueId>,
    ) -> Result<OpaqueId, LeaseError> {
        let holder = holder_id_hint.unwrap_or(client_id);
        let mut map = self.lock();
        let key = vault_id.as_str().to_owned();
        if let Some(existing) = map.get(&key) {
            return Err(LeaseError::AlreadyHeld {
                holder_id: existing.clone(),
            });
        }
        map.insert(key, holder.clone());
        Ok(holder)
    }

    /// Releases a lease if `holder_id` matches.
    pub fn release(&self, vault_id: &VaultId, holder_id: &OpaqueId) -> Result<(), LeaseError> {
        let mut map = self.lock();
        let key = vault_id.as_str();
        match map.get(key) {
            None => Err(LeaseError::NotHeld),
            Some(current) if current != holder_id => Err(LeaseError::NotHolder),
            Some(_) => {
                map.remove(key);
                Ok(())
            }
        }
    }

    /// Returns the current holder if any.
    pub fn holder(&self, vault_id: &VaultId) -> Option<OpaqueId> {
        self.lock().get(vault_id.as_str()).cloned()
    }
}

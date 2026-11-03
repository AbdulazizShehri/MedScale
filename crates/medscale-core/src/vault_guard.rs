//! Narrow writer-exclusion guard for desktop recovery (Spec 108 step 4).
//!
//! Surfaces never depend on storage layout directly; this exposes only
//! "hold Core's writer lock on this vault folder" and three outcomes.

use std::path::Path;

use medscale_storage::{WriterLock, WriterLockError};

/// Holds Core's exclusive writer lock on one vault folder until dropped.
#[derive(Debug)]
pub struct VaultWriterGuard {
    _lock: WriterLock,
}

/// Why the writer lock could not be held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultWriterGuardError {
    /// Another connection or process holds the writer lock.
    Held,
    /// The path is refused by Core's claim policy (for example a sync root).
    Refused,
    /// Any other I/O or SQLite failure.
    Io,
}

impl VaultWriterGuard {
    /// Try to hold the writer lock for `vault_root` without waiting.
    ///
    /// # Errors
    /// [`VaultWriterGuardError`] when the lock is held, refused or unreadable.
    pub fn try_hold(vault_root: &Path) -> Result<Self, VaultWriterGuardError> {
        match WriterLock::try_acquire(vault_root) {
            Ok(lock) => Ok(Self { _lock: lock }),
            Err(WriterLockError::WriterHeld) => Err(VaultWriterGuardError::Held),
            Err(WriterLockError::Claim(_)) => Err(VaultWriterGuardError::Refused),
            Err(_) => Err(VaultWriterGuardError::Io),
        }
    }
}

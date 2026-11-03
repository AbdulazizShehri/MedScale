//! Narrow writer-exclusion guard for desktop recovery (Spec 108 step 4).
//!
//! Surfaces never depend on storage layout directly; this exposes only
//! "hold Core's writer lock on this vault folder" and three outcomes.

use std::path::{Path, PathBuf};

use medscale_storage::{SettleJournalError, WriterLock, WriterLockError};

/// Holds Core's exclusive writer lock on one vault folder until dropped.
///
/// The recorded root is the path at lock time; if the folder is renamed
/// while held, settling fails safely (no create flag) instead of following.
#[derive(Debug)]
pub struct VaultWriterGuard {
    _lock: WriterLock,
    root: PathBuf,
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
            Ok(lock) => Ok(Self {
                _lock: lock,
                root: vault_root.to_path_buf(),
            }),
            Err(WriterLockError::WriterHeld) => Err(VaultWriterGuardError::Held),
            Err(WriterLockError::Claim(_)) => Err(VaultWriterGuardError::Refused),
            Err(_) => Err(VaultWriterGuardError::Io),
        }
    }

    /// Let SQLite roll back an interrupted (uncommitted) write on the vault
    /// this guard holds, exactly as Core's next open would. Never creates,
    /// migrates or touches any other file.
    ///
    /// # Errors
    /// [`VaultWriterGuardError::Held`] when another connection is writing,
    /// [`VaultWriterGuardError::Io`] for any other failure.
    pub fn settle_interrupted_write(&self) -> Result<(), VaultWriterGuardError> {
        medscale_storage::settle_meta_journal(&self.root).map_err(|e| match e {
            SettleJournalError::Busy => VaultWriterGuardError::Held,
            SettleJournalError::Io => VaultWriterGuardError::Io,
        })
    }
}

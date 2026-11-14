//! OS-process exclusive writer ownership for an open vault (Q02).
//!
//! Uses a dedicated SQLite connection held with `BEGIN EXCLUSIVE`. Lock-file presence
//! alone is not ownership — the live exclusive transaction is.

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;
use thiserror::Error;

use crate::claim::{ClaimError, assert_claim_path};

/// Writer lock errors.
#[derive(Debug, Error)]
pub enum WriterLockError {
    #[error(transparent)]
    Claim(#[from] ClaimError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("writer held by another process")]
    WriterHeld,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Held exclusive writer lock. Drop releases the lock with the connection.
#[derive(Debug)]
pub struct WriterLock {
    _conn: Connection,
}

impl WriterLock {
    /// Acquire exclusive writer ownership for `vault_root`.
    pub fn try_acquire(vault_root: &Path) -> Result<Self, WriterLockError> {
        let root = assert_claim_path(vault_root)?;
        std::fs::create_dir_all(&root)?;
        let path = root.join("writer.lock.sqlite3");
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_millis(0))?;
        match conn.execute_batch("PRAGMA locking_mode=EXCLUSIVE; BEGIN EXCLUSIVE;") {
            Ok(()) => Ok(Self { _conn: conn }),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::DatabaseBusy
                    || err.code == rusqlite::ErrorCode::DatabaseLocked =>
            {
                Err(WriterLockError::WriterHeld)
            }
            Err(e) => Err(WriterLockError::Sqlite(e)),
        }
    }
}

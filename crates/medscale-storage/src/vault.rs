//! Open synthetic vault handle (blobs + sqlite metadata + writer lock).

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::blob::{BlobError, FsBlobStore};
use crate::claim::{ClaimError, assert_claim_path};
use crate::sqlite_meta::{MetaError, SqliteMetaStore};
use crate::writer_lock::{WriterLock, WriterLockError};

/// Vault open errors.
#[derive(Debug, Error)]
pub enum VaultError {
    #[error(transparent)]
    Claim(#[from] ClaimError),
    #[error(transparent)]
    Blob(#[from] BlobError),
    #[error(transparent)]
    Meta(#[from] MetaError),
    #[error(transparent)]
    Writer(#[from] WriterLockError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Synthetic H0-A vault: claim-scoped root with FS blobs + SQLite metadata.
#[derive(Debug)]
pub struct SyntheticVault {
    pub vault_id: String,
    pub root: PathBuf,
    pub blobs: FsBlobStore,
    pub meta: SqliteMetaStore,
    _writer: WriterLock,
}

impl SyntheticVault {
    pub fn open(vault_id: &str, vault_root: &Path) -> Result<Self, VaultError> {
        let root = assert_claim_path(vault_root)?;
        std::fs::create_dir_all(&root)?;
        let writer = WriterLock::try_acquire(&root)?;
        let blobs = FsBlobStore::open(&root)?;
        let meta = SqliteMetaStore::open(&root)?;
        Ok(Self {
            vault_id: vault_id.to_owned(),
            root,
            blobs,
            meta,
            _writer: writer,
        })
    }
}

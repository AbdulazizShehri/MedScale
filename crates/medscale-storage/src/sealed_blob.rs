//! AES-GCM sealed blob store (vault-scoped).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::objects::DigestSha256;
use medscale_keys::{VaultDek, seal, unseal};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::claim::{ClaimError, assert_claim_path};

#[derive(Debug, Error)]
pub enum SealedBlobError {
    #[error(transparent)]
    Claim(#[from] ClaimError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("crypto")]
    Crypto,
    #[error("digest mismatch")]
    DigestMismatch,
    #[error("not found")]
    NotFound,
}

#[derive(Debug)]
pub struct SealedBlobStore {
    root: PathBuf,
    vault_id: String,
}

impl SealedBlobStore {
    pub fn open(vault_root: &Path, vault_id: &str) -> Result<Self, SealedBlobError> {
        let root = assert_claim_path(vault_root)?.join("sealed_blobs");
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            vault_id: vault_id.to_owned(),
        })
    }

    fn path_for(&self, digest: &DigestSha256) -> PathBuf {
        self.root.join(format!("{}.sealed", digest.to_hex()))
    }

    fn aad(&self, digest: &DigestSha256) -> Vec<u8> {
        format!("blob:{}:{}", self.vault_id, digest.to_hex()).into_bytes()
    }

    /// Put plaintext bytes; returns content digest of plaintext.
    pub fn put(&self, dek: &VaultDek, bytes: &[u8]) -> Result<DigestSha256, SealedBlobError> {
        let digest = DigestSha256::of(bytes);
        // Content-addressed and renamed into place only when complete. Core
        // re-puts every blob on each snapshot, so this also avoids an fsync
        // per blob per request.
        let dest = self.path_for(&digest);
        if dest.exists() {
            return Ok(digest);
        }
        let sealed =
            seal(dek.as_bytes(), bytes, &self.aad(&digest)).map_err(|_| SealedBlobError::Crypto)?;
        let encoded = serde_json::to_vec(&sealed).map_err(|_| SealedBlobError::Crypto)?;
        let tmp = self.root.join(format!("{}.tmp", digest.to_hex()));
        // Synced before its reference can be sealed, so a power loss cannot
        // leave a durable row that points at a lost blob.
        let mut file = fs::File::create(&tmp)?;
        std::io::Write::write_all(&mut file, &encoded)?;
        file.sync_all()?;
        drop(file);
        crate::encrypted_vault::rename_with_retry(&tmp, &dest)?;
        // A committed row can reference this blob before any seal happens.
        #[cfg(unix)]
        fs::File::open(&self.root)?.sync_all()?;
        Ok(digest)
    }

    pub fn get(&self, dek: &VaultDek, digest: &DigestSha256) -> Result<Vec<u8>, SealedBlobError> {
        let path = self.path_for(digest);
        // Only a real absence is NotFound: an antivirus lock or a read error
        // is transient and must not read as a lost blob.
        let encoded = fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SealedBlobError::NotFound
            } else {
                SealedBlobError::Io(e)
            }
        })?;
        let sealed: medscale_keys::WrappedBlob =
            serde_json::from_slice(&encoded).map_err(|_| SealedBlobError::Crypto)?;
        let plain = unseal(dek.as_bytes(), &sealed, &self.aad(digest))
            .map_err(|_| SealedBlobError::Crypto)?;
        if DigestSha256::of(&plain) != *digest {
            return Err(SealedBlobError::DigestMismatch);
        }
        Ok(plain)
    }

    /// True if any sealed file contains the plaintext needle when wrongly decrypted is impossible;
    /// scans raw sealed files for plaintext marker (should be absent).
    pub fn plaintext_marker_absent(&self, marker: &[u8]) -> bool {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return true;
        };
        for entry in entries.flatten() {
            if let Ok(bytes) = fs::read(entry.path())
                && bytes.windows(marker.len()).any(|w| w == marker)
            {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Content digest helper used when copying from plaintext blob stores.
#[must_use]
#[allow(dead_code)]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

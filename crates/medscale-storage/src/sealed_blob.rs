//! AES-GCM sealed blob store (vault-scoped).

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::objects::DigestSha256;
use medscale_keys::{VaultDek, seal, unseal};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroize;

use crate::claim::{ClaimError, assert_claim_path};

const KEYED_BLOB_PREFIX: &str = "k1_";

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

/// RFC 2104 HMAC-SHA256 keyed with the 32-byte vault DEK so blob filenames
/// cannot be used to test whether a known plaintext exists in the vault.
fn hmac_sha256_key32(key: &[u8; 32], msg: &[u8]) -> [u8; 32] {
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..32 {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let mut inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    let out_digest = outer.finalize();

    ipad.zeroize();
    opad.zeroize();
    inner_hash.zeroize();

    let mut out = [0u8; 32];
    out.copy_from_slice(&out_digest);
    out
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

    fn keyed_stem(&self, dek: &VaultDek, digest: &DigestSha256) -> String {
        let msg = format!(
            "medscale.sealed_blob.name.v1:{}:{}",
            self.vault_id,
            digest.to_hex()
        );
        let mac = hmac_sha256_key32(dek.as_bytes(), msg.as_bytes());
        let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
        format!("{KEYED_BLOB_PREFIX}{hex}")
    }

    /// Keyed on-disk path for a sealed blob (used for all new writes).
    #[must_use]
    pub fn path_for_keyed(&self, dek: &VaultDek, digest: &DigestSha256) -> PathBuf {
        self.root
            .join(format!("{}.sealed", self.keyed_stem(dek, digest)))
    }

    /// Legacy unkeyed on-disk path (read-only fallback for existing vaults).
    fn legacy_path_for(&self, digest: &DigestSha256) -> PathBuf {
        self.root.join(format!("{}.sealed", digest.to_hex()))
    }

    fn aad(&self, digest: &DigestSha256) -> Vec<u8> {
        format!("blob:{}:{}", self.vault_id, digest.to_hex()).into_bytes()
    }

    /// Put plaintext bytes; returns content digest of plaintext.
    pub fn put(&self, dek: &VaultDek, bytes: &[u8]) -> Result<DigestSha256, SealedBlobError> {
        let digest = DigestSha256::of(bytes);
        // Keyed content-addressed filename, renamed into place only when
        // complete. Core re-puts every blob on each snapshot, so this avoids
        // an fsync per blob per request and prevents predicting filenames
        // without the vault DEK.
        let stem = self.keyed_stem(dek, &digest);
        let dest = self.root.join(format!("{stem}.sealed"));
        if dest.exists() {
            return Ok(digest);
        }
        let sealed =
            seal(dek.as_bytes(), bytes, &self.aad(&digest)).map_err(|_| SealedBlobError::Crypto)?;
        let encoded = serde_json::to_vec(&sealed).map_err(|_| SealedBlobError::Crypto)?;
        let tmp = self.root.join(format!("{stem}.tmp"));
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
        let keyed_path = self.path_for_keyed(dek, digest);
        // Only a real absence is NotFound: an antivirus lock or a read error
        // is transient and must not read as a lost blob. Fall back to the
        // legacy unkeyed filename only when the keyed filename is absent.
        let encoded = match fs::read(&keyed_path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::read(self.legacy_path_for(digest)).map_err(|legacy_err| {
                    if legacy_err.kind() == std::io::ErrorKind::NotFound {
                        SealedBlobError::NotFound
                    } else {
                        SealedBlobError::Io(legacy_err)
                    }
                })?
            }
            Err(e) => return Err(SealedBlobError::Io(e)),
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "medscale-sealed-blob-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn blob_filenames_are_keyed_and_differ_across_vault_keys() {
        let root_a = scratch("keyed-a");
        let root_b = scratch("keyed-b");
        let store_a = SealedBlobStore::open(&root_a, "v-same").unwrap();
        let store_b = SealedBlobStore::open(&root_b, "v-same").unwrap();
        let dek_a = VaultDek::generate();
        let dek_b = VaultDek::generate();
        let payload = b"synthetic clinical record bytes";

        let digest_a = store_a.put(&dek_a, payload).unwrap();
        let digest_b = store_b.put(&dek_b, payload).unwrap();
        assert_eq!(digest_a, digest_b);

        // Plaintext SHA-256 must never appear as the on-disk blob filename.
        assert!(!store_a.legacy_path_for(&digest_a).exists());
        let path_a = store_a.path_for_keyed(&dek_a, &digest_a);
        let path_b = store_b.path_for_keyed(&dek_b, &digest_b);
        assert!(path_a.is_file());
        assert!(path_b.is_file());
        assert_ne!(path_a.file_name(), path_b.file_name());
        assert!(
            !path_a
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains(&digest_a.to_hex())
        );

        assert_eq!(store_a.get(&dek_a, &digest_a).unwrap(), payload);
        assert_eq!(store_b.get(&dek_b, &digest_b).unwrap(), payload);

        let _ = fs::remove_dir_all(&root_a);
        let _ = fs::remove_dir_all(&root_b);
    }

    #[test]
    fn legacy_unkeyed_blob_still_reads_via_fallback() {
        let root = scratch("legacy-fallback");
        let store = SealedBlobStore::open(&root, "v-legacy").unwrap();
        let dek = VaultDek::generate();
        let payload = b"legacy sealed payload";
        let digest = DigestSha256::of(payload);

        let sealed = seal(dek.as_bytes(), payload, &store.aad(&digest)).unwrap();
        let encoded = serde_json::to_vec(&sealed).unwrap();
        fs::write(store.legacy_path_for(&digest), encoded).unwrap();

        assert!(!store.path_for_keyed(&dek, &digest).exists());
        assert_eq!(store.get(&dek, &digest).unwrap(), payload);

        let _ = fs::remove_dir_all(&root);
    }
}

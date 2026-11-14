//! Filesystem blob store: temp → fsync → rename; verify on read.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use medscale_contracts::ingest::{BlobRef, BlobState};
use medscale_contracts::objects::DigestSha256;
use thiserror::Error;

use crate::claim::{ClaimError, assert_claim_path};

/// Blob store errors.
#[derive(Debug, Error)]
pub enum BlobError {
    #[error(transparent)]
    Claim(#[from] ClaimError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("digest mismatch")]
    DigestMismatch,
    #[error("missing blob")]
    Missing,
}

/// Content-addressed FS blob store under `blobs/`.
#[derive(Debug)]
pub struct FsBlobStore {
    root: PathBuf,
}

impl FsBlobStore {
    pub fn open(vault_root: &Path) -> Result<Self, BlobError> {
        let root = assert_claim_path(vault_root)?.join("blobs");
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("quarantine"))?;
        Ok(Self { root })
    }

    fn path_for(&self, digest: &DigestSha256) -> PathBuf {
        self.root.join(hex_digest(digest))
    }

    pub fn put_blob(&self, bytes: &[u8]) -> Result<BlobRef, BlobError> {
        let digest = DigestSha256::of(bytes);
        let final_path = self.path_for(&digest);
        if final_path.exists() {
            return Ok(BlobRef {
                digest,
                byte_length: bytes.len() as u64,
            });
        }
        let tmp = self.root.join(format!(".{}.tmp", hex_digest(&digest)));
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &final_path)?;
        // Best-effort directory sync on Unix would go here; Windows rename is durable enough for H0-A claim class.
        Ok(BlobRef {
            digest,
            byte_length: bytes.len() as u64,
        })
    }

    pub fn get_blob(&self, digest: &DigestSha256) -> Result<Vec<u8>, BlobError> {
        let path = self.path_for(digest);
        if !path.exists() {
            return Err(BlobError::Missing);
        }
        let mut file = File::open(&path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        if DigestSha256::of(&bytes) != *digest {
            self.quarantine(digest, "digest mismatch")?;
            return Err(BlobError::DigestMismatch);
        }
        Ok(bytes)
    }

    pub fn verify(&self, digest: &DigestSha256, size: u64) -> Result<(), BlobError> {
        let bytes = self.get_blob(digest)?;
        if bytes.len() as u64 != size {
            self.quarantine(digest, "size mismatch")?;
            return Err(BlobError::DigestMismatch);
        }
        Ok(())
    }

    pub fn quarantine(&self, digest: &DigestSha256, _reason: &str) -> Result<(), BlobError> {
        let src = self.path_for(digest);
        if src.exists() {
            let dest = self.root.join("quarantine").join(hex_digest(digest));
            let _ = fs::rename(src, dest);
        }
        Ok(())
    }

    pub fn state(&self, digest: &DigestSha256) -> BlobState {
        if self
            .root
            .join("quarantine")
            .join(hex_digest(digest))
            .exists()
        {
            return BlobState::Quarantined;
        }
        if self.path_for(digest).exists() {
            BlobState::Live
        } else if self
            .root
            .join("tombstone")
            .join(hex_digest(digest))
            .exists()
        {
            BlobState::Tombstoned
        } else {
            BlobState::Quarantined
        }
    }

    pub fn list_live_digests(&self) -> Result<Vec<DigestSha256>, BlobError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                if let Some(digest) = parse_hex_digest(&name) {
                    out.push(digest);
                }
            }
        }
        Ok(out)
    }

    pub fn tombstone(&self, digest: &DigestSha256) -> Result<(), BlobError> {
        let src = self.path_for(digest);
        if src.exists() {
            let tomb = self.root.join("tombstone");
            fs::create_dir_all(&tomb)?;
            fs::rename(src, tomb.join(hex_digest(digest)))?;
        }
        Ok(())
    }

    pub fn sweep_tombstones(&self) -> Result<u64, BlobError> {
        let tomb = self.root.join("tombstone");
        if !tomb.exists() {
            return Ok(0);
        }
        let mut n = 0_u64;
        for entry in fs::read_dir(&tomb)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::remove_file(entry.path())?;
                n += 1;
            }
        }
        Ok(n)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn hex_digest(digest: &DigestSha256) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn parse_hex_digest(name: &str) -> Option<DigestSha256> {
    if name.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (i, chunk) in name.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).ok()?;
        bytes[i] = u8::from_str_radix(s, 16).ok()?;
    }
    // Reconstruct via of() only works for content; store raw:
    Some(DigestSha256::from_bytes(bytes))
}

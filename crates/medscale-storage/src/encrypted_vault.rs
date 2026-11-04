//! EncryptedVault: SQLCipher page-encrypted work metadata + AES-GCM sealed at-rest (Specs 005/023).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use medscale_contracts::objects::DigestSha256;
use medscale_keys::{
    KeyProvider, KeyStore, RecoveryCodeSet, VaultDek, VaultKeyHeader, seal, unseal,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::claim::{ClaimError, assert_claim_path};
use crate::sealed_blob::{SealedBlobError, SealedBlobStore};
use crate::sqlite_meta::{MetaError, SourceMeta, SqliteMetaStore};

const HEADER_FILE: &str = "vault_header.json";
const META_SEALED: &str = "meta.sealed";
const META_SEALED_TMP: &str = "meta.sealed.tmp";
const META_WORK: &str = "meta.work.sqlite3";
const META_WORK_TMP: &str = "meta.work.sqlite3.tmp";
const LEASE_FILE: &str = ".writer.lock";
const SESSION_LOCK_FILE: &str = ".session.lock";

#[derive(Debug, Error)]
pub enum EncryptedVaultError {
    #[error(transparent)]
    Claim(#[from] ClaimError),
    #[error(transparent)]
    Meta(#[from] MetaError),
    #[error(transparent)]
    Sealed(#[from] SealedBlobError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing key material")]
    MissingKeyMaterial,
    #[error("lease held by {0}")]
    LeaseHeld(String),
    #[error("crypto")]
    Crypto,
    /// The key was right but `meta.sealed` is missing, torn or fails
    /// authentication. Nothing on disk was changed.
    #[error("sealed vault metadata is unreadable")]
    SealedMetaUnreadable,
    #[error("invalid: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnDiskHeader {
    keys: VaultKeyHeader,
}

/// Open encrypted vault handle (DEK held only while unlocked).
#[derive(Debug)]
pub struct EncryptedVault {
    pub vault_id: String,
    pub root: PathBuf,
    pub header: VaultKeyHeader,
    dek: VaultDek,
    pub meta: SqliteMetaStore,
    pub blobs: SealedBlobStore,
    #[allow(dead_code)]
    lease_holder: String,
    /// OS lock held for the whole session, so another process can neither
    /// adopt nor wipe this session's live work database (Spec 109).
    session_lock: fs::File,
    discarded_interrupted_session: bool,
}

impl EncryptedVault {
    /// Create a new encrypted vault; returns recovery codes once.
    pub fn create(
        vault_id: &str,
        vault_root: &Path,
        passphrase: &str,
        holder_id: &str,
        keystore: Option<&dyn KeyStore>,
    ) -> Result<(Self, RecoveryCodeSet), EncryptedVaultError> {
        let root = assert_claim_path(vault_root)?;
        fs::create_dir_all(&root)?;
        if root.join(HEADER_FILE).exists() {
            return Err(EncryptedVaultError::Invalid(
                "vault already exists".to_owned(),
            ));
        }
        Self::acquire_lease(&root, holder_id)?;

        let (dek, header, codes) = KeyProvider::create_vault_keys(vault_id, passphrase)
            .map_err(|_| EncryptedVaultError::Crypto)?;
        if let Some(store) = keystore {
            KeyProvider::store_wrapped_dek(store, vault_id, &dek)
                .map_err(|_| EncryptedVaultError::MissingKeyMaterial)?;
        }

        let on_disk = OnDiskHeader {
            keys: header.clone(),
        };
        fs::write(
            root.join(HEADER_FILE),
            serde_json::to_vec_pretty(&on_disk).map_err(|_| EncryptedVaultError::Crypto)?,
        )?;

        let work = root.join(META_WORK);
        {
            let _meta = SqliteMetaStore::open_at_sqlcipher(&work, dek.as_bytes())?;
        }
        Self::seal_meta_file(&root, vault_id, &dek)?;
        let meta = SqliteMetaStore::open_at_sqlcipher(&work, dek.as_bytes())?;
        let blobs = SealedBlobStore::open(&root, vault_id)?;
        let session_lock = Self::take_session_lock(&root, holder_id)?;

        Ok((
            Self {
                vault_id: vault_id.to_owned(),
                root,
                header,
                dek,
                meta,
                blobs,
                lease_holder: holder_id.to_owned(),
                session_lock,
                discarded_interrupted_session: false,
            },
            codes,
        ))
    }

    pub fn open_with_passphrase(
        vault_root: &Path,
        passphrase: &str,
        holder_id: &str,
    ) -> Result<Self, EncryptedVaultError> {
        let root = assert_claim_path(vault_root)?;
        let header = Self::load_header(&root)?;
        let dek = KeyProvider::unlock_passphrase(&header, passphrase)
            .map_err(|_| EncryptedVaultError::MissingKeyMaterial)?;
        Self::finish_open(root, header, dek, holder_id)
    }

    pub fn open_with_recovery(
        vault_root: &Path,
        code: &str,
        holder_id: &str,
    ) -> Result<Self, EncryptedVaultError> {
        let root = assert_claim_path(vault_root)?;
        let header = Self::load_header(&root)?;
        let dek = KeyProvider::unlock_recovery(&header, code)
            .map_err(|_| EncryptedVaultError::MissingKeyMaterial)?;
        Self::finish_open(root, header, dek, holder_id)
    }

    pub fn open_with_keystore(
        vault_root: &Path,
        store: &dyn KeyStore,
        holder_id: &str,
    ) -> Result<Self, EncryptedVaultError> {
        let root = assert_claim_path(vault_root)?;
        let header = Self::load_header(&root)?;
        let dek = KeyProvider::unlock_keystore(store, &header)
            .map_err(|_| EncryptedVaultError::MissingKeyMaterial)?;
        Self::finish_open(root, header, dek, holder_id)
    }

    fn finish_open(
        root: PathBuf,
        header: VaultKeyHeader,
        dek: VaultDek,
        holder_id: &str,
    ) -> Result<Self, EncryptedVaultError> {
        // Spec 109: nothing is touched until the key proved right and the
        // replacement is authenticated. A wrong passphrase or a torn or foreign
        // `meta.sealed` fails here with every file intact.
        let plain = Self::decrypt_sealed_meta(&root, &header.vault_id, &dek)?;
        Self::acquire_lease(&root, holder_id)?;
        let session_lock = Self::take_session_lock(&root, holder_id)?;
        let _ = fs::remove_file(root.join(META_SEALED_TMP));
        let _ = fs::remove_file(root.join(META_WORK_TMP));
        // Crash leftovers are not treated as sealed (Spec 017 FR-003); the
        // caller is told so it can say that recent changes may be lost.
        let discarded_interrupted_session = Self::leftover_work_present(&root);
        if discarded_interrupted_session {
            Self::wipe_work_sidecars(&root)?;
        }
        Self::remove_remaining_sidecars(&root)?;
        let work = root.join(META_WORK);
        write_durably(&root.join(META_WORK_TMP), &work, &plain)?;
        let meta = SqliteMetaStore::open_at_sqlcipher_or_rekey_legacy(&work, dek.as_bytes())?;
        let blobs = SealedBlobStore::open(&root, &header.vault_id)?;
        Ok(Self {
            vault_id: header.vault_id.clone(),
            root,
            header,
            dek,
            meta,
            blobs,
            lease_holder: holder_id.to_owned(),
            session_lock,
            discarded_interrupted_session,
        })
    }

    /// True when this open found a work database from a session that never
    /// sealed, and discarded it. Its unsealed changes are gone.
    #[must_use]
    pub fn discarded_interrupted_session(&self) -> bool {
        self.discarded_interrupted_session
    }

    fn take_session_lock(root: &Path, holder_id: &str) -> Result<fs::File, EncryptedVaultError> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(SESSION_LOCK_FILE))?;
        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(fs::TryLockError::WouldBlock) => {
                Err(EncryptedVaultError::LeaseHeld(holder_id.to_owned()))
            }
            Err(fs::TryLockError::Error(e)) => Err(EncryptedVaultError::Io(e)),
        }
    }

    /// A stale journal beside a fresh copy could be rolled back into it.
    fn remove_remaining_sidecars(root: &Path) -> Result<(), EncryptedVaultError> {
        let work = root.join(META_WORK);
        for suffix in ["-wal", "-shm", "-journal"] {
            let side = sqlite_sidecar(&work, suffix);
            if side.exists() {
                fs::remove_file(&side)?;
            }
        }
        Ok(())
    }

    fn load_header(root: &Path) -> Result<VaultKeyHeader, EncryptedVaultError> {
        let bytes = fs::read(root.join(HEADER_FILE))?;
        let on_disk: OnDiskHeader =
            serde_json::from_slice(&bytes).map_err(|_| EncryptedVaultError::Crypto)?;
        Ok(on_disk.keys)
    }

    fn meta_aad(vault_id: &str) -> Vec<u8> {
        format!("meta:{vault_id}").into_bytes()
    }

    fn seal_meta_file(
        root: &Path,
        vault_id: &str,
        dek: &VaultDek,
    ) -> Result<(), EncryptedVaultError> {
        let work = root.join(META_WORK);
        if !work.exists() {
            return Err(EncryptedVaultError::Invalid(
                "missing meta work file".to_owned(),
            ));
        }
        let plain = fs::read(&work)?;
        let sealed = seal(dek.as_bytes(), &plain, &Self::meta_aad(vault_id))
            .map_err(|_| EncryptedVaultError::Crypto)?;
        // Atomic: a crash or full disk leaves the previous seal intact. The
        // caller wipes the work file only after this returns.
        write_durably(
            &root.join(META_SEALED_TMP),
            &root.join(META_SEALED),
            &serde_json::to_vec(&sealed).map_err(|_| EncryptedVaultError::Crypto)?,
        )?;
        Ok(())
    }

    /// Authenticates `meta.sealed` into memory without touching any file.
    fn decrypt_sealed_meta(
        root: &Path,
        vault_id: &str,
        dek: &VaultDek,
    ) -> Result<Vec<u8>, EncryptedVaultError> {
        let unreadable = || EncryptedVaultError::SealedMetaUnreadable;
        let encoded = fs::read(root.join(META_SEALED)).map_err(|_| unreadable())?;
        let sealed: medscale_keys::WrappedBlob =
            serde_json::from_slice(&encoded).map_err(|_| unreadable())?;
        unseal(dek.as_bytes(), &sealed, &Self::meta_aad(vault_id)).map_err(|_| unreadable())
    }

    fn acquire_lease(root: &Path, holder_id: &str) -> Result<(), EncryptedVaultError> {
        let lease = root.join(LEASE_FILE);
        if lease.exists() {
            let existing = fs::read_to_string(&lease).unwrap_or_default();
            // An empty lease is a torn write, not a holder; a live holder is
            // refused by the session lock regardless of this file.
            if !existing.trim().is_empty() && existing != holder_id {
                return Err(EncryptedVaultError::LeaseHeld(existing));
            }
        }
        fs::write(&lease, holder_id)?;
        Ok(())
    }

    /// Persist sealed meta and release lease; zeros DEK via drop. On failure
    /// the vault is dropped and its work file stays on disk; use [`Self::try_close`]
    /// to keep it open.
    pub fn close(self) -> Result<(), EncryptedVaultError> {
        self.try_close().map_err(|failed| failed.1)
    }

    /// Seals and closes, or hands the still-open vault back with the error so
    /// a failed lock loses nothing and can be retried (Spec 109). The store
    /// uses delete-journal mode and no transaction is open between requests,
    /// so the work file is complete while its connection stays open.
    pub fn try_close(self) -> Result<(), Box<(Self, EncryptedVaultError)>> {
        if let Err(error) = Self::seal_meta_file(&self.root, &self.vault_id, &self.dek) {
            return Err(Box::new((self, error)));
        }
        let EncryptedVault {
            root,
            meta,
            session_lock,
            ..
        } = self;
        drop(meta);
        let _ = Self::wipe_work_sidecars(&root);
        let _ = fs::remove_file(root.join(LEASE_FILE));
        drop(session_lock);
        let _ = fs::remove_file(root.join(SESSION_LOCK_FILE));
        Ok(())
    }

    /// Remove work DB and SQLite sidecar files if present.
    pub fn wipe_work_sidecars(root: &Path) -> Result<(), EncryptedVaultError> {
        let work = root.join(META_WORK);
        // Sidecars first: a crash here must not leave a journal with no database.
        for path in [
            sqlite_sidecar(&work, "-wal"),
            sqlite_sidecar(&work, "-shm"),
            sqlite_sidecar(&work, "-journal"),
            work.clone(),
        ] {
            if path.exists() {
                // Best-effort overwrite then remove (not a secure OS wipe claim).
                if let Ok(meta) = fs::metadata(&path) {
                    let len = meta.len() as usize;
                    let _ = fs::write(&path, vec![0_u8; len.min(1024 * 1024)]);
                }
                let _ = fs::remove_file(&path);
            }
        }
        Ok(())
    }

    /// True when work or journal sidecars remain on disk.
    #[must_use]
    pub fn leftover_work_present(root: &Path) -> bool {
        let work = root.join(META_WORK);
        work.exists()
            || sqlite_sidecar(&work, "-wal").exists()
            || sqlite_sidecar(&work, "-shm").exists()
            || sqlite_sidecar(&work, "-journal").exists()
    }

    pub fn put_blob(&self, bytes: &[u8]) -> Result<DigestSha256, EncryptedVaultError> {
        Ok(self.blobs.put(&self.dek, bytes)?)
    }

    pub fn get_blob(&self, digest: &DigestSha256) -> Result<Vec<u8>, EncryptedVaultError> {
        Ok(self.blobs.get(&self.dek, digest)?)
    }

    pub fn insert_source(&self, meta: &SourceMeta) -> Result<(), EncryptedVaultError> {
        self.meta.insert_source(meta)?;
        Ok(())
    }

    /// Embedded SQLCipher version for evidence (`PRAGMA cipher_version`).
    pub fn sqlcipher_cipher_version(&self) -> Result<String, EncryptedVaultError> {
        Ok(self.meta.cipher_version()?)
    }

    /// True when Spec 023 SQLCipher EncryptedVault backend is compiled in.
    #[must_use]
    pub fn sqlcipher_enabled() -> bool {
        SqliteMetaStore::sqlcipher_backend_active()
    }

    /// Destroy key wraps (retention); leaves ciphertext unreadable.
    pub fn destroy_key_material(vault_root: &Path) -> Result<(), EncryptedVaultError> {
        let root = assert_claim_path(vault_root)?;
        let header_path = root.join(HEADER_FILE);
        if header_path.exists() {
            fs::remove_file(header_path)?;
        }
        Ok(())
    }

    /// Backup sealed vault tree (header + sealed meta + sealed blobs).
    pub fn backup(vault_root: &Path, dest: &Path) -> Result<(), EncryptedVaultError> {
        let root = assert_claim_path(vault_root)?;
        let dest = assert_claim_path(dest)?;
        fs::create_dir_all(&dest)?;
        for name in [HEADER_FILE, META_SEALED] {
            let src = root.join(name);
            if src.exists() {
                fs::copy(&src, dest.join(name))?;
            }
        }
        let blobs_src = root.join("sealed_blobs");
        if blobs_src.exists() {
            copy_dir(&blobs_src, &dest.join("sealed_blobs"))?;
        }
        Ok(())
    }

    pub fn restore(source: &Path, dest: &Path) -> Result<(), EncryptedVaultError> {
        let source = assert_claim_path(source)?;
        let dest = assert_claim_path(dest)?;
        fs::create_dir_all(&dest)?;
        for name in [HEADER_FILE, META_SEALED] {
            let src = source.join(name);
            if src.exists() {
                fs::copy(&src, dest.join(name))?;
            }
        }
        let blobs_src = source.join("sealed_blobs");
        if blobs_src.exists() {
            copy_dir(&blobs_src, &dest.join("sealed_blobs"))?;
        }
        Ok(())
    }

    /// Migrate Spec 003 SyntheticVault plaintext into a new EncryptedVault.
    pub fn migrate_from_synthetic(
        synthetic_root: &Path,
        encrypted_root: &Path,
        vault_id: &str,
        passphrase: &str,
        holder_id: &str,
    ) -> Result<(Self, RecoveryCodeSet), EncryptedVaultError> {
        let synth = crate::SyntheticVault::open(vault_id, synthetic_root)
            .map_err(|e| EncryptedVaultError::Invalid(e.to_string()))?;
        let (enc, codes) = Self::create(vault_id, encrypted_root, passphrase, holder_id, None)?;
        for src in synth.meta.list_sources()? {
            let bytes = synth
                .blobs
                .get_blob(&src.digest)
                .map_err(|e| EncryptedVaultError::Invalid(e.to_string()))?;
            let digest = enc.put_blob(&bytes)?;
            if digest != src.digest {
                return Err(EncryptedVaultError::Crypto);
            }
            enc.insert_source(&src)?;
        }
        Ok((enc, codes))
    }

    /// Open-work + sealed surfaces must not contain the marker (Spec 023 page encryption).
    pub fn plaintext_marker_absent_on_disk(&self, marker: &[u8]) -> bool {
        let work = self.root.join(META_WORK);
        if let Ok(bytes) = fs::read(&work)
            && bytes.windows(marker.len()).any(|w| w == marker)
        {
            return false;
        }
        for suffix in ["-wal", "-shm", "-journal"] {
            let side = sqlite_sidecar(&work, suffix);
            if let Ok(bytes) = fs::read(&side)
                && bytes.windows(marker.len()).any(|w| w == marker)
            {
                return false;
            }
        }
        if let Ok(bytes) = fs::read(self.root.join(META_SEALED))
            && bytes.windows(marker.len()).any(|w| w == marker)
        {
            return false;
        }
        self.blobs.plaintext_marker_absent(marker)
    }
}

/// Writes `bytes` to `tmp`, syncs it and renames it over `dest`. A sharing
/// violation from antivirus or indexing on Windows is retried briefly.
fn write_durably(tmp: &Path, dest: &Path, bytes: &[u8]) -> Result<(), EncryptedVaultError> {
    let mut file = fs::File::create(tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        match fs::rename(tmp, dest) {
            Ok(()) => break,
            Err(e)
                if cfg!(windows)
                    && matches!(e.raw_os_error(), Some(5 | 32 | 33))
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(EncryptedVaultError::Io(e)),
        }
    }
    #[cfg(unix)]
    if let Some(parent) = dest.parent() {
        fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

fn sqlite_sidecar(work: &Path, suffix: &str) -> PathBuf {
    let mut os = work.as_os_str().to_owned();
    os.push(suffix);
    PathBuf::from(os)
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), EncryptedVaultError> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dest.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

/// Resolve default local app-data vault path (not yet claim-checked).
#[must_use]
pub fn default_vault_root(vault_id: &str) -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MedScale")
            .join("vaults")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
            })
            .unwrap_or_else(|| PathBuf::from("."))
            .join("medscale")
            .join("vaults")
    };
    base.join(vault_id)
}

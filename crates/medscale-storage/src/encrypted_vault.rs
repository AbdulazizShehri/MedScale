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
const HEADER_TMP: &str = "vault_header.json.tmp";
const META_SEALED: &str = "meta.sealed";
const META_SEALED_TMP: &str = "meta.sealed.tmp";
const META_WORK: &str = "meta.work.sqlite3";
const META_WORK_TMP: &str = "meta.work.sqlite3.tmp";
const META_WORK_TMP_JOURNAL: &str = "meta.work.sqlite3.tmp-journal";
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
    _session_lock: fs::File,
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
        // First, so two concurrent creates cannot both pass the check below and
        // overwrite each other's header and seal.
        let session_lock = Self::take_session_lock(&root, holder_id)?;
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
        // Durable before anything is sealed under the keys it wraps.
        write_durably(
            &root.join(HEADER_TMP),
            &root.join(HEADER_FILE),
            &serde_json::to_vec_pretty(&on_disk).map_err(|_| EncryptedVaultError::Crypto)?,
        )?;

        let work = root.join(META_WORK);
        {
            let _meta = SqliteMetaStore::open_at_sqlcipher(&work, dek.as_bytes())?;
        }
        Self::seal_meta_file(&root, vault_id, &dek)?;
        sync_dir(&root)?;
        // A standard restore, so a crash in the first session has a marker.
        let (plain, seal_digest) = Self::decrypt_sealed_meta(&root, vault_id, &dek)?;
        Self::restore_work_file(&root, &dek, &plain, &seal_digest)?;
        let meta = SqliteMetaStore::open_at_sqlcipher(&work, dek.as_bytes())?;
        let blobs = SealedBlobStore::open(&root, vault_id)?;

        Ok((
            Self {
                vault_id: vault_id.to_owned(),
                root,
                header,
                dek,
                meta,
                blobs,
                lease_holder: holder_id.to_owned(),
                _session_lock: session_lock,
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
        // The lock comes first. Reading the seal without it could restore a seal
        // that a closing session has just replaced, and this session's close
        // would then overwrite the newer one. A refused opener rewrites nothing.
        let session_lock = Self::take_session_lock(&root, holder_id)?;
        // Spec 109: nothing else is touched until the replacement is
        // authenticated. A torn or foreign `meta.sealed` fails here.
        let (plain, seal_digest) = Self::decrypt_sealed_meta(&root, &header.vault_id, &dek)?;
        Self::acquire_lease(&root, holder_id)?;
        let _ = fs::remove_file(root.join(META_SEALED_TMP));
        // Crash leftovers are not treated as sealed (Spec 017 FR-003); the
        // caller is told so it can say that recent changes may be lost.
        let discarded_interrupted_session = Self::leftover_work_present(&root);
        Self::restore_work_file(&root, &dek, &plain, &seal_digest)?;
        let work = root.join(META_WORK);
        let meta = SqliteMetaStore::open_at_sqlcipher(&work, dek.as_bytes())?;
        let blobs = SealedBlobStore::open(&root, &header.vault_id)?;
        Ok(Self {
            vault_id: header.vault_id.clone(),
            root,
            header,
            dek,
            meta,
            blobs,
            lease_holder: holder_id.to_owned(),
            _session_lock: session_lock,
            discarded_interrupted_session,
        })
    }

    /// True when this open found a work database from a session that never
    /// sealed, and discarded it. Its unsealed changes are gone.
    #[must_use]
    pub fn discarded_interrupted_session(&self) -> bool {
        self.discarded_interrupted_session
    }

    /// Replaces the work file with a copy of the authenticated seal, carrying
    /// the marker that binds it to that seal. Legacy rekey and migration happen
    /// on the temp file, which nothing ever adopts. The replacement is durable
    /// before the leftover goes, and a rename replaces it, so a crash or full
    /// disk cannot hide a discard of a work file from the next open.
    fn restore_work_file(
        root: &Path,
        dek: &VaultDek,
        plain: &[u8],
        seal_digest: &str,
    ) -> Result<(), EncryptedVaultError> {
        let work = root.join(META_WORK);
        let work_tmp = root.join(META_WORK_TMP);
        for stale in [META_WORK_TMP, META_WORK_TMP_JOURNAL] {
            let path = root.join(stale);
            if path.try_exists()? {
                remove_with_retry(&path)?;
            }
        }
        write_synced(&work_tmp, plain)?;
        crash_point("restore_after_temp");
        {
            let meta =
                SqliteMetaStore::open_at_sqlcipher_or_rekey_legacy(&work_tmp, dek.as_bytes())?;
            meta.set_base_seal_digest(seal_digest)?;
        }
        fs::OpenOptions::new()
            .write(true)
            .open(&work_tmp)?
            .sync_all()?;
        crash_point("restore_after_marker");
        Self::remove_sidecars(root)?;
        crash_point("restore_after_sidecars");
        // Not scrubbed first: a failed rename must leave the leftover whole.
        rename_with_retry(&work_tmp, &work)?;
        sync_dir(root)?;
        crash_point("restore_after_rename");
        Ok(())
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
    fn remove_sidecars(root: &Path) -> Result<(), EncryptedVaultError> {
        let work = root.join(META_WORK);
        for suffix in ["-wal", "-shm", "-journal"] {
            let side = sqlite_sidecar(&work, suffix);
            if side.exists() {
                scrub(&side);
                remove_with_retry(&side)?;
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
        // A sealed row must not outlive its blob after power loss. `create`
        // seals before this folder exists.
        #[cfg(unix)]
        {
            let blobs = root.join("sealed_blobs");
            if blobs.is_dir() {
                fs::File::open(&blobs)?.sync_all()?;
            }
        }
        // Atomic: a crash or full disk leaves the previous seal intact. The
        // caller syncs the directory and wipes the work file only after this.
        let seal_tmp = root.join(META_SEALED_TMP);
        write_synced(
            &seal_tmp,
            &serde_json::to_vec(&sealed).map_err(|_| EncryptedVaultError::Crypto)?,
        )?;
        crash_point("seal_after_temp");
        rename_with_retry(&seal_tmp, &root.join(META_SEALED))?;
        crash_point("seal_after_rename");
        Ok(())
    }

    /// Authenticates `meta.sealed` into memory without touching any file, and
    /// returns the plaintext with the SHA-256 of the sealed bytes.
    fn decrypt_sealed_meta(
        root: &Path,
        vault_id: &str,
        dek: &VaultDek,
    ) -> Result<(Vec<u8>, String), EncryptedVaultError> {
        let unreadable = || EncryptedVaultError::SealedMetaUnreadable;
        let encoded = fs::read(root.join(META_SEALED)).map_err(|_| unreadable())?;
        let sealed: medscale_keys::WrappedBlob =
            serde_json::from_slice(&encoded).map_err(|_| unreadable())?;
        let plain =
            unseal(dek.as_bytes(), &sealed, &Self::meta_aad(vault_id)).map_err(|_| unreadable())?;
        Ok((plain, DigestSha256::of(&encoded).to_hex()))
    }

    /// The caller already holds the session lock, which is what refuses a live
    /// holder, so anything found here is stale: a crash or a torn write.
    fn acquire_lease(root: &Path, holder_id: &str) -> Result<(), EncryptedVaultError> {
        fs::write(root.join(LEASE_FILE), holder_id)?;
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
        let EncryptedVault { root, meta, .. } = self;
        drop(meta);
        // The rename already happened, so the session is over either way. If
        // the directory did not sync, keep the work file: should power loss
        // revert the rename, its marker still matches the older seal.
        if sync_dir(&root).is_err() {
            return Ok(());
        }
        crash_point("close_before_wipe");
        let _ = Self::wipe_work_sidecars(&root);
        crash_point("close_after_wipe");
        let _ = fs::remove_file(root.join(LEASE_FILE));
        // `.session.lock` stays: unlinking it would let one opener lock the
        // removed file while another locks a new one. It is always empty.
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
                scrub(&path);
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
        let _session_lock = Self::take_session_lock(&dest, "restore")?;
        // Blobs first: a seal is never in place before the blobs it names.
        let blobs_src = source.join("sealed_blobs");
        if blobs_src.exists() {
            copy_dir(&blobs_src, &dest.join("sealed_blobs"))?;
        }
        for (name, tmp) in [(HEADER_FILE, HEADER_TMP), (META_SEALED, META_SEALED_TMP)] {
            let src = source.join(name);
            if src.exists() {
                write_durably(&dest.join(tmp), &dest.join(name), &fs::read(&src)?)?;
            }
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
        for path in [
            work.clone(),
            self.root.join(META_WORK_TMP),
            self.root.join(META_WORK_TMP_JOURNAL),
            self.root.join(META_SEALED_TMP),
        ] {
            if let Ok(bytes) = fs::read(&path)
                && bytes.windows(marker.len()).any(|w| w == marker)
            {
                return false;
            }
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

/// Best-effort overwrite before removal (not a secure OS wipe claim).
fn scrub(path: &Path) {
    if let Ok(meta) = fs::metadata(path) {
        let len = meta.len() as usize;
        let _ = fs::write(path, vec![0_u8; len.min(1024 * 1024)]);
    }
}

fn write_durably(tmp: &Path, dest: &Path, bytes: &[u8]) -> Result<(), EncryptedVaultError> {
    write_synced(tmp, bytes)?;
    rename_with_retry(tmp, dest)?;
    if let Some(parent) = dest.parent() {
        sync_dir(parent)?;
    }
    Ok(())
}

fn write_synced(tmp: &Path, bytes: &[u8]) -> Result<(), EncryptedVaultError> {
    let mut file = fs::File::create(tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Syncs a directory on Unix. Windows has no equivalent; NTFS journaling is
/// relied on.
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    fs::File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// Test-only crash hook: exit with a sentinel code at a named boundary.
#[cfg(test)]
fn crash_point(boundary: &str) {
    if std::env::var("MEDSCALE_TEST_VAULT_CRASH").as_deref() == Ok(boundary) {
        std::process::exit(86);
    }
}

#[cfg(not(test))]
fn crash_point(_boundary: &str) {}

/// Renames `tmp` over `dest`.
pub(crate) fn rename_with_retry(tmp: &Path, dest: &Path) -> std::io::Result<()> {
    with_sharing_retry(|| fs::rename(tmp, dest))
}

fn remove_with_retry(path: &Path) -> std::io::Result<()> {
    with_sharing_retry(|| fs::remove_file(path))
}

/// A sharing violation from antivirus or indexing on Windows is retried briefly.
fn with_sharing_retry(mut op: impl FnMut() -> std::io::Result<()>) -> std::io::Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        match op() {
            Ok(()) => return Ok(()),
            Err(e)
                if cfg!(windows)
                    && matches!(e.raw_os_error(), Some(5 | 32 | 33))
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(e),
        }
    }
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
            fs::copy(entry.path(), &to)?;
            // Blobs must be durable before a restore puts the seal naming them.
            fs::OpenOptions::new().write(true).open(&to)?.sync_all()?;
        }
    }
    sync_dir(dest)?;
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

#[cfg(test)]
mod crash_tests {
    //! Spec 109 Part B: a real process exit at each boundary, real SQLCipher.
    //! After every crash the seal still authenticates and the next open works.
    use super::*;
    use crate::SourceMeta;
    use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId};

    const PASS: &str = "passphrase-ok";
    const HOLDER: &str = "holder-a";

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "medscale-109-crash-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    fn source(id: &str) -> SourceMeta {
        SourceMeta {
            source_id: OpaqueId::new(id),
            realm_id: RealmId::new("r"),
            authority_scope_id: AuthorityScopeId::new("s"),
            digest: DigestSha256::of(id.as_bytes()),
            byte_length: 1,
            media_type: "text/plain".to_owned(),
            visible: true,
            resource_type: "Marker".to_owned(),
        }
    }

    /// Runs one action in a child process that stops at `boundary`.
    fn run_child(root: &Path, action: &str, boundary: &str) {
        let out = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "encrypted_vault::crash_tests::crash_child",
                "--exact",
                "--ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("MEDSCALE_TEST_VAULT_DIR", root)
            .env("MEDSCALE_TEST_VAULT_ACTION", action)
            .env("MEDSCALE_TEST_VAULT_CRASH", boundary)
            .output()
            .expect("child");
        assert_eq!(
            out.status.code(),
            Some(86),
            "{action}/{boundary}: did not stop at the boundary\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    #[ignore = "re-executed by the crash tests only"]
    fn crash_child() {
        let Some(root) = std::env::var_os("MEDSCALE_TEST_VAULT_DIR") else {
            return;
        };
        let root = PathBuf::from(root);
        match std::env::var("MEDSCALE_TEST_VAULT_ACTION").as_deref() {
            Ok("open") => {
                let _ = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER);
            }
            Ok("write_and_close") => {
                let vault =
                    EncryptedVault::open_with_passphrase(&root, PASS, HOLDER).expect("open");
                vault
                    .insert_source(&source("in-this-session"))
                    .expect("write");
                let _ = vault.close();
            }
            other => panic!("unknown action {other:?}"),
        }
    }

    fn sealed_with_one_source(tag: &str) -> PathBuf {
        let root = scratch(tag);
        let (vault, _) =
            EncryptedVault::create("v-109", &root, PASS, HOLDER, None).expect("create");
        vault.insert_source(&source("sealed")).expect("write");
        vault.close().expect("close");
        root
    }

    fn sources(vault: &EncryptedVault) -> Vec<String> {
        let mut names: Vec<String> = vault
            .meta
            .list_sources()
            .expect("list")
            .iter()
            .map(|s| s.source_id.as_str().to_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_crash_during_restore_keeps_the_seal_and_the_next_open_works() {
        for boundary in [
            "restore_after_temp",
            "restore_after_marker",
            "restore_after_sidecars",
            "restore_after_rename",
        ] {
            let root = sealed_with_one_source("restore");
            let seal = fs::read(root.join(META_SEALED)).expect("seal");
            // A crashed session, so the sidecar and rename steps have work to do.
            let session = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER).expect("open");
            session.insert_source(&source("unsealed")).expect("write");
            drop(session);
            fs::write(root.join("meta.work.sqlite3-journal"), b"stale").expect("journal");
            run_child(&root, "open", boundary);
            assert_eq!(
                fs::read(root.join(META_SEALED)).expect("seal"),
                seal,
                "{boundary}"
            );
            let vault = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER)
                .unwrap_or_else(|e| panic!("{boundary}: {e}"));
            assert_eq!(sources(&vault), ["sealed"], "{boundary}");
            assert!(
                vault.discarded_interrupted_session(),
                "{boundary}: the crash did not hide the discard"
            );
            vault.close().expect("close");
            fs::remove_dir_all(&root).expect("cleanup");
        }
    }

    #[test]
    fn a_crash_during_close_leaves_either_the_old_seal_or_the_new_one() {
        // Before the rename the old seal is current; from the rename on, the
        // new one is. Part A discards the leftover, so this is what opens next.
        for (boundary, expected) in [
            ("seal_after_temp", &["sealed"][..]),
            ("seal_after_rename", &["in-this-session", "sealed"][..]),
            ("close_before_wipe", &["in-this-session", "sealed"][..]),
            ("close_after_wipe", &["in-this-session", "sealed"][..]),
        ] {
            let root = sealed_with_one_source("close");
            run_child(&root, "write_and_close", boundary);
            let vault = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER)
                .unwrap_or_else(|e| panic!("{boundary}: {e}"));
            assert_eq!(sources(&vault), expected, "{boundary}");
            vault.close().expect("close");
            fs::remove_dir_all(&root).expect("cleanup");
        }
    }

    #[test]
    fn create_and_every_restore_bind_the_work_file_to_the_seal() {
        let root = scratch("marker");
        let (vault, _) =
            EncryptedVault::create("v-109", &root, PASS, HOLDER, None).expect("create");
        let seal = DigestSha256::of(&fs::read(root.join(META_SEALED)).expect("seal")).to_hex();
        assert_eq!(vault.meta.base_seal_digest().expect("read"), Some(seal));
        vault.insert_source(&source("x")).expect("write");
        vault.close().expect("close");

        let reopened = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER).expect("open");
        let seal = DigestSha256::of(&fs::read(root.join(META_SEALED)).expect("seal")).to_hex();
        assert_eq!(reopened.meta.base_seal_digest().expect("read"), Some(seal));
        // The marker is not a sequence: backups enumerate sequences as numbers.
        assert!(reopened.meta.list_id_sequences().is_ok());
        reopened.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn the_no_migrate_open_creates_nothing_and_changes_no_schema() {
        let root = scratch("no-migrate");
        let key = [7_u8; 32];
        let missing = root.join("missing.sqlite3");
        assert!(SqliteMetaStore::open_existing_sqlcipher_no_migrate(&missing, &key).is_err());
        assert!(!missing.exists(), "no file was created");

        // A migrating open would recreate this table; this one must not.
        let db = root.join("existing.sqlite3");
        {
            let store = SqliteMetaStore::open_at_sqlcipher(&db, &key).expect("create");
            store
                .conn()
                .execute_batch("DROP TABLE migration_journal")
                .expect("drop");
        }
        let store = SqliteMetaStore::open_existing_sqlcipher_no_migrate(&db, &key)
            .expect("the right key opens it");
        assert!(
            store.migration_journal().is_err(),
            "no migration ran, so the journal table is still gone"
        );
        drop(store);
        assert!(SqliteMetaStore::open_existing_sqlcipher_no_migrate(&db, &[8_u8; 32]).is_err());
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_legacy_plaintext_seal_is_rekeyed_on_the_temp_file_and_opens() {
        let root = sealed_with_one_source("legacy");
        let plain_path = root.join("plain.sqlite3");
        {
            let plain = SqliteMetaStore::open_at(&plain_path).expect("plaintext db");
            plain.insert_source(&source("legacy-row")).expect("write");
        }
        let plain_bytes = fs::read(&plain_path).expect("read");
        assert!(
            plain_bytes.starts_with(b"SQLite format 3"),
            "really plaintext"
        );
        fs::remove_file(&plain_path).expect("remove");
        let header = EncryptedVault::load_header(&root).expect("header");
        let dek = KeyProvider::unlock_passphrase(&header, PASS).expect("key");
        let sealed = seal(
            dek.as_bytes(),
            &plain_bytes,
            &EncryptedVault::meta_aad(&header.vault_id),
        )
        .expect("seal");
        fs::write(
            root.join(META_SEALED),
            serde_json::to_vec(&sealed).expect("encode"),
        )
        .expect("write seal");

        let vault = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER).expect("open");
        assert_eq!(sources(&vault), ["legacy-row"]);
        assert!(
            !fs::read(root.join(META_WORK))
                .expect("work")
                .starts_with(b"SQLite format 3"),
            "the work file is encrypted"
        );
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn keyed_connections_stay_at_full_synchronous() {
        let root = sealed_with_one_source("synchronous");
        let vault = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER).expect("open");
        let level: i64 = vault
            .meta
            .conn()
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .expect("pragma");
        assert_eq!(level, 2, "2 is FULL");
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_database_from_a_newer_build_is_refused() {
        let root = sealed_with_one_source("newer");
        let vault = EncryptedVault::open_with_passphrase(&root, PASS, HOLDER).expect("open");
        vault
            .meta
            .conn()
            .execute(
                "INSERT INTO migration_journal(version, state) VALUES (?1, 'finished')",
                rusqlite::params![crate::CURRENT_META_SCHEMA_VERSION + 1],
            )
            .expect("future row");
        vault.close().expect("close");
        assert!(matches!(
            EncryptedVault::open_with_passphrase(&root, PASS, HOLDER),
            Err(EncryptedVaultError::Meta(MetaError::UnsupportedSchema(_)))
        ));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_corrupt_next_seq_is_an_error_not_zero() {
        let root = scratch("next-seq");
        let (vault, _) =
            EncryptedVault::create("v-109", &root, PASS, HOLDER, None).expect("create");
        vault
            .meta
            .conn()
            .execute(
                "INSERT OR REPLACE INTO store_state(key, value) VALUES ('next_seq', 'nope')",
                [],
            )
            .expect("corrupt");
        assert!(vault.meta.get_next_seq().is_err());
        vault.meta.set_next_seq(1).expect("repair");
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }
}

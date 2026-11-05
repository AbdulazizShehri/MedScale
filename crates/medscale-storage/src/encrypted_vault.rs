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
use crate::sqlite_meta::{MetaError, SourceMeta, SqliteMetaStore, WorkDatabaseCheck};

const HEADER_FILE: &str = "vault_header.json";
const HEADER_TMP: &str = "vault_header.json.tmp";
const META_SEALED: &str = "meta.sealed";
const META_SEALED_TMP: &str = "meta.sealed.tmp";
const META_WORK: &str = "meta.work.sqlite3";
const META_WORK_TMP: &str = "meta.work.sqlite3.tmp";
const META_WORK_TMP_JOURNAL: &str = "meta.work.sqlite3.tmp-journal";
const META_WORK_TMP_REKEY: &str = "meta.work.sqlite3.tmp.rekey";
const LEASE_FILE: &str = ".writer.lock";
const SESSION_LOCK_FILE: &str = ".session.lock";
const PRE_RECOVERY_DIR: &str = "pre-recovery";
const DISCARDED_DIR: &str = "discarded-session";
const VERDICT_FILE: &str = "verdict";
const NOTICE_FILE: &str = ".notice-pending";
const COMPLETE_FILE: &str = ".complete";
const VERDICT_STALE: &str = "stale";
const VERDICT_UNVERIFIABLE: &str = "unverifiable";
/// Kept when evicting. Only stale generations and copies are ever evicted.
const RECOVERY_RETENTION: usize = 3;
const MIN_WORK_FILE_BYTES: u64 = 4096;

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
    /// An unsealed session could not be checked. Nothing was changed.
    #[error("the interrupted session could not be checked: {0}")]
    WorkFileCheckUnavailable(String),
    /// A confirmed move-aside could not move the file. Nothing was changed.
    #[error("the interrupted session could not be moved aside: {0}")]
    WorkFileBlocked(String),
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
    work_source: WorkSource,
    notice_pending: bool,
    notice_ack_requested: bool,
}

/// Where this session's work file came from (Spec 109 Part B). The `next_seq`
/// raise and every load action key on this, never on the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkSource {
    /// A fresh copy of the seal.
    Restored,
    /// A verified leftover from a session that never sealed.
    Adopted,
}

/// How to open: normally, or having confirmed the leftover should be moved aside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftoverPolicy {
    Classify,
    MoveAside,
}

/// The verdict on a leftover work file.
enum LeftoverVerdict {
    Verified {
        schema: u32,
    },
    /// Definitive: damaged, incomplete or unbound.
    Unverifiable,
    /// Valid, but it descends from an older seal.
    Stale,
    /// The check itself could not finish. Nothing may change.
    Retryable(String),
    NewerSchema,
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
                work_source: WorkSource::Restored,
                notice_pending: false,
                notice_ack_requested: false,
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
        Self::finish_open(root, header, dek, holder_id, LeftoverPolicy::Classify)
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
        Self::finish_open(root, header, dek, holder_id, LeftoverPolicy::Classify)
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
        Self::finish_open(root, header, dek, holder_id, LeftoverPolicy::Classify)
    }

    /// Opens after the user confirmed moving an unsealed session aside, because
    /// its check could not finish. Requires the key, like any other open.
    pub fn open_moving_leftover_aside(
        vault_root: &Path,
        passphrase: Option<&str>,
        recovery_code: Option<&str>,
        holder_id: &str,
    ) -> Result<Self, EncryptedVaultError> {
        let root = assert_claim_path(vault_root)?;
        let header = Self::load_header(&root)?;
        let dek = match (passphrase, recovery_code) {
            (_, Some(code)) => KeyProvider::unlock_recovery(&header, code),
            (Some(passphrase), None) => KeyProvider::unlock_passphrase(&header, passphrase),
            (None, None) => return Err(EncryptedVaultError::MissingKeyMaterial),
        }
        .map_err(|_| EncryptedVaultError::MissingKeyMaterial)?;
        Self::finish_open(root, header, dek, holder_id, LeftoverPolicy::MoveAside)
    }

    fn finish_open(
        root: PathBuf,
        header: VaultKeyHeader,
        dek: VaultDek,
        holder_id: &str,
        policy: LeftoverPolicy,
    ) -> Result<Self, EncryptedVaultError> {
        // The lock comes first. Reading the seal without it could restore a seal
        // that a closing session has just replaced, and this session's close
        // would then overwrite the newer one. A refused opener rewrites nothing.
        let session_lock = Self::take_session_lock(&root, holder_id)?;
        // Spec 109: nothing else is touched until the seal is authenticated, and
        // a leftover is never adopted without a seal to compare it with.
        let (plain, seal_digest) = Self::decrypt_sealed_meta(&root, &header.vault_id, &dek)?;
        Self::acquire_lease(&root, holder_id)?;
        let _ = fs::remove_file(root.join(META_SEALED_TMP));
        let work = root.join(META_WORK);
        let leftover = Self::leftover_work_exists(&root)?;
        let work_source = match (leftover, policy) {
            (false, _) => {
                Self::restore_work_file(&root, &dek, &plain, &seal_digest)?;
                WorkSource::Restored
            }
            (true, LeftoverPolicy::MoveAside) => {
                Self::quarantine(&root, VERDICT_UNVERIFIABLE)
                    .map_err(|e| EncryptedVaultError::WorkFileBlocked(e.to_string()))?;
                Self::restore_work_file(&root, &dek, &plain, &seal_digest)?;
                WorkSource::Restored
            }
            (true, LeftoverPolicy::Classify) => {
                match Self::verify_leftover(&root, &dek, &seal_digest) {
                    LeftoverVerdict::Retryable(why) => {
                        return Err(EncryptedVaultError::WorkFileCheckUnavailable(why));
                    }
                    LeftoverVerdict::NewerSchema => {
                        return Err(EncryptedVaultError::Meta(MetaError::UnsupportedSchema(
                            "the interrupted session is from a newer build".to_owned(),
                        )));
                    }
                    verdict @ (LeftoverVerdict::Unverifiable | LeftoverVerdict::Stale) => {
                        let label = if matches!(verdict, LeftoverVerdict::Stale) {
                            VERDICT_STALE
                        } else {
                            VERDICT_UNVERIFIABLE
                        };
                        Self::quarantine(&root, label).map_err(|e| {
                            EncryptedVaultError::WorkFileCheckUnavailable(e.to_string())
                        })?;
                        Self::restore_work_file(&root, &dek, &plain, &seal_digest)?;
                        WorkSource::Restored
                    }
                    LeftoverVerdict::Verified { schema } => {
                        Self::keep_pre_recovery_copy(&root, &seal_digest)?;
                        if schema < crate::CURRENT_META_SCHEMA_VERSION {
                            Self::migrate_work_on_a_copy(&root, &dek).map_err(|e| {
                                EncryptedVaultError::WorkFileCheckUnavailable(e.to_string())
                            })?;
                        }
                        crash_point("adopt_before_open");
                        WorkSource::Adopted
                    }
                }
            }
        };
        let meta = match work_source {
            WorkSource::Restored => SqliteMetaStore::open_at_sqlcipher(&work, dek.as_bytes())?,
            // Offered the confirmed move-aside like any other failed check, so
            // a good seal can't stay out of reach.
            WorkSource::Adopted => {
                SqliteMetaStore::open_existing_sqlcipher_no_migrate(&work, dek.as_bytes())
                    .map_err(|e| EncryptedVaultError::WorkFileCheckUnavailable(e.to_string()))?
            }
        };
        let blobs = SealedBlobStore::open(&root, &header.vault_id)?;
        let notice_pending = Self::any_notice_pending(&root)?;
        Ok(Self {
            vault_id: header.vault_id.clone(),
            root,
            header,
            dek,
            meta,
            blobs,
            lease_holder: holder_id.to_owned(),
            _session_lock: session_lock,
            work_source,
            notice_pending,
            notice_ack_requested: false,
        })
    }

    #[must_use]
    pub fn work_source(&self) -> WorkSource {
        self.work_source
    }

    /// True when a discarded session has not been reported to the user yet.
    #[must_use]
    pub fn notice_pending(&self) -> bool {
        self.notice_pending
    }

    /// Asks for the notice to be cleared after the next successful persist, once
    /// it is safe to say it was delivered.
    pub fn request_notice_acknowledgement(&mut self) {
        self.notice_ack_requested = true;
    }

    /// Clears the notice if that was requested. A failed persist never reaches
    /// this, so the discard is reported again on the next open.
    pub fn acknowledge_notice_if_requested(&mut self) {
        if !std::mem::take(&mut self.notice_ack_requested) {
            return;
        }
        self.notice_pending = false;
        let Ok(entries) = fs::read_dir(self.root.join(DISCARDED_DIR)) else {
            return;
        };
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path().join(NOTICE_FILE));
        }
    }

    /// A recovered session failed to load for a definitive reason. Move it aside
    /// and restore from the seal, keeping the session lock throughout.
    pub fn discard_adopted_and_restore(mut self) -> Result<Self, Box<(Self, EncryptedVaultError)>> {
        let plain = match Self::decrypt_sealed_meta(&self.root, &self.vault_id, &self.dek) {
            Ok(decrypted) => decrypted,
            Err(e) => return Err(Box::new((self, e))),
        };
        // Windows: SQLite opens without delete sharing, so close before moving.
        let closed = match Self::placeholder_store() {
            Ok(store) => store,
            Err(e) => return Err(Box::new((self, EncryptedVaultError::Meta(e)))),
        };
        drop(std::mem::replace(&mut self.meta, closed));
        let result = Self::quarantine(&self.root, VERDICT_UNVERIFIABLE)
            .map_err(|e| EncryptedVaultError::WorkFileBlocked(e.to_string()))
            .and_then(|()| Self::restore_work_file(&self.root, &self.dek, &plain.0, &plain.1))
            .and_then(|()| {
                Ok(SqliteMetaStore::open_at_sqlcipher(
                    &self.root.join(META_WORK),
                    self.dek.as_bytes(),
                )?)
            });
        match result {
            Ok(meta) => {
                self.meta = meta;
                self.work_source = WorkSource::Restored;
                self.notice_pending = true;
                Ok(self)
            }
            Err(e) => Err(Box::new((self, e))),
        }
    }

    /// A restored copy failed to load. It holds no session, so it is removed and
    /// the seal is left as it was.
    pub fn remove_restored_work_file(self) {
        let EncryptedVault {
            root,
            meta,
            work_source,
            ..
        } = self;
        drop(meta);
        if work_source == WorkSource::Restored {
            let _ = Self::wipe_work_sidecars(&root);
        }
    }

    /// An in-memory stand-in, so the real connection can close while the vault
    /// keeps its lock.
    fn placeholder_store() -> Result<SqliteMetaStore, MetaError> {
        SqliteMetaStore::open_in_memory_placeholder()
    }

    /// `try_exists`, so a metadata error can't read as "no session here".
    fn leftover_work_exists(root: &Path) -> Result<bool, EncryptedVaultError> {
        let work = root.join(META_WORK);
        for path in [
            work.clone(),
            sqlite_sidecar(&work, "-wal"),
            sqlite_sidecar(&work, "-shm"),
            sqlite_sidecar(&work, "-journal"),
        ] {
            if path
                .try_exists()
                .map_err(|e| EncryptedVaultError::WorkFileCheckUnavailable(e.to_string()))?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Spec 109 I3: an unknown result is retryable, and only a result seen twice
    /// is definitive, because a failed salt read can make a good file report
    /// "not a database". The connection is closed before this returns.
    fn verify_leftover(root: &Path, dek: &VaultDek, seal_digest: &str) -> LeftoverVerdict {
        let work = root.join(META_WORK);
        match fs::metadata(&work) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return LeftoverVerdict::Unverifiable;
            }
            Err(e) => return LeftoverVerdict::Retryable(e.to_string()),
            Ok(meta) if meta.len() < MIN_WORK_FILE_BYTES => return LeftoverVerdict::Unverifiable,
            Ok(_) => {}
        }
        let first = Self::check_work_file(&work, dek);
        let settled = if first.is_bad() {
            // Only two matching results are definitive. A good second look wins.
            let second = Self::confirm_definitive(&work, dek);
            if second.is_healthy() || second.same_kind_of_bad(&first) {
                second
            } else {
                WorkCheckResult::Retryable("the check gave different results".to_owned())
            }
        } else {
            first
        };
        match settled {
            WorkCheckResult::Retryable(why) => LeftoverVerdict::Retryable(why),
            WorkCheckResult::Definitive
            | WorkCheckResult::Done(WorkDatabaseCheck::Damaged | WorkDatabaseCheck::Incomplete) => {
                LeftoverVerdict::Unverifiable
            }
            WorkCheckResult::Done(WorkDatabaseCheck::Healthy { marker, schema }) => {
                if schema > crate::CURRENT_META_SCHEMA_VERSION {
                    LeftoverVerdict::NewerSchema
                } else if marker.is_none() {
                    LeftoverVerdict::Unverifiable
                } else if marker.as_deref() == Some(seal_digest) {
                    LeftoverVerdict::Verified { schema }
                } else {
                    LeftoverVerdict::Stale
                }
            }
        }
    }

    /// Confirms the file reads end to end, waits, and checks once more.
    fn confirm_definitive(work: &Path, dek: &VaultDek) -> WorkCheckResult {
        if let Err(e) = fs::read(work) {
            return WorkCheckResult::Retryable(e.to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
        Self::check_work_file(work, dek)
    }

    fn check_work_file(work: &Path, dek: &VaultDek) -> WorkCheckResult {
        #[cfg(test)]
        if test_hooks::take_transient_failure() {
            return WorkCheckResult::Definitive;
        }
        let store = match SqliteMetaStore::open_existing_sqlcipher_no_migrate(work, dek.as_bytes())
        {
            Ok(store) => store,
            Err(e) => return WorkCheckResult::from_error(&e),
        };
        match store.verify_work_database() {
            Ok(check) => WorkCheckResult::Done(check),
            Err(e) => WorkCheckResult::from_error(&e),
        }
    }

    /// Moves the leftover to a new generation folder, and never overwrites one
    /// (I6). A failure moves everything back and leaves no generation.
    fn quarantine(root: &Path, verdict: &str) -> Result<(), EncryptedVaultError> {
        let base = root.join(DISCARDED_DIR);
        fs::create_dir_all(&base)?;
        let next = Self::generations(&base)?
            .iter()
            .map(|(n, _)| n + 1)
            .max()
            .unwrap_or(1);
        let generation = base.join(next.to_string());
        fs::create_dir(&generation)?;
        fs::write(generation.join(VERDICT_FILE), verdict)?;
        fs::write(generation.join(NOTICE_FILE), b"")?;
        let work = root.join(META_WORK);
        let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
        for path in [
            work.clone(),
            sqlite_sidecar(&work, "-journal"),
            sqlite_sidecar(&work, "-wal"),
            sqlite_sidecar(&work, "-shm"),
        ] {
            let name = path
                .file_name()
                .map(std::ffi::OsStr::to_os_string)
                .unwrap_or_default();
            let to = generation.join(name);
            let attempt = path.try_exists().and_then(|exists| {
                if exists {
                    rename_with_retry(&path, &to).map(|()| true)
                } else {
                    Ok(false)
                }
            });
            match attempt {
                Ok(true) => {
                    moved.push((path, to));
                    crash_point("quarantine_after_move");
                }
                Ok(false) => {}
                Err(e) => {
                    let mut all_back = true;
                    for (from, to) in moved.iter().rev() {
                        all_back &= rename_with_retry(to, from).is_ok();
                    }
                    // A generation that still holds a file is the only copy of
                    // it, so it stays, however this attempt ended (I6).
                    if all_back {
                        let _ = fs::remove_dir_all(&generation);
                    }
                    return Err(EncryptedVaultError::Io(e));
                }
            }
        }
        // The session is out of the way. A failure below can't undo that and
        // must not read as "nothing changed": an incomplete generation is never
        // evicted, so nothing is at risk.
        let _ = fs::write(generation.join(COMPLETE_FILE), b"");
        let _ = sync_dir(root);
        let _ = sync_dir(&base);
        Self::evict_stale_generations(&base);
        Ok(())
    }

    fn generations(base: &Path) -> Result<Vec<(u64, PathBuf)>, EncryptedVaultError> {
        let mut found = Vec::new();
        for entry in fs::read_dir(base)? {
            let entry = entry?;
            if let Some(number) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u64>().ok())
            {
                found.push((number, entry.path()));
            }
        }
        found.sort();
        Ok(found)
    }

    /// Only complete `stale` generations are ever evicted. An `unverifiable` one
    /// may hold real work, so it stays until the user deletes it.
    fn evict_stale_generations(base: &Path) {
        let Ok(generations) = Self::generations(base) else {
            return;
        };
        let stale: Vec<PathBuf> = generations
            .into_iter()
            .map(|(_, path)| path)
            .filter(|path| {
                path.join(COMPLETE_FILE).exists()
                    && fs::read_to_string(path.join(VERDICT_FILE))
                        .is_ok_and(|verdict| verdict == VERDICT_STALE)
            })
            .collect();
        let excess = stale.len().saturating_sub(RECOVERY_RETENTION);
        for path in stale.into_iter().take(excess) {
            let _ = fs::remove_dir_all(path);
        }
    }

    fn any_notice_pending(root: &Path) -> Result<bool, EncryptedVaultError> {
        let base = root.join(DISCARDED_DIR);
        if !base.try_exists()? {
            return Ok(false);
        }
        for (_, generation) in Self::generations(&base)? {
            if generation.join(NOTICE_FILE).try_exists()? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// I2: a durable, verified copy of the seal this session descends from. Named
    /// by digest, and rewritten if an existing file doesn't match its name.
    fn keep_pre_recovery_copy(root: &Path, seal_digest: &str) -> Result<(), EncryptedVaultError> {
        let unavailable =
            |e: std::io::Error| EncryptedVaultError::WorkFileCheckUnavailable(e.to_string());
        let dir = root.join(PRE_RECOVERY_DIR);
        fs::create_dir_all(&dir).map_err(unavailable)?;
        let copy = dir.join(format!("{seal_digest}.sealed"));
        if let Ok(existing) = fs::read(&copy)
            && DigestSha256::of(&existing).to_hex() == seal_digest
        {
            return Ok(());
        }
        let bytes = fs::read(root.join(META_SEALED)).map_err(unavailable)?;
        if DigestSha256::of(&bytes).to_hex() != seal_digest {
            return Err(EncryptedVaultError::WorkFileCheckUnavailable(
                "the sealed file changed during recovery".to_owned(),
            ));
        }
        write_durably(&dir.join(format!("{seal_digest}.tmp")), &copy, &bytes)
            .map_err(|e| EncryptedVaultError::WorkFileCheckUnavailable(e.to_string()))?;
        crash_point("adopt_after_copy");
        Self::evict_old_copies(&dir);
        Ok(())
    }

    fn evict_old_copies(dir: &Path) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut copies: Vec<(std::time::SystemTime, PathBuf)> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "sealed"))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        copies.sort();
        let excess = copies.len().saturating_sub(RECOVERY_RETENTION);
        for (_, path) in copies.into_iter().take(excess) {
            let _ = fs::remove_file(path);
        }
    }

    /// `migrate()` is not atomic, so migration runs on a temp copy. A crash leaves
    /// only that removable file, and the marker survives in the copy.
    fn migrate_work_on_a_copy(root: &Path, dek: &VaultDek) -> Result<(), EncryptedVaultError> {
        let work = root.join(META_WORK);
        let tmp = root.join(META_WORK_TMP);
        for stale in [META_WORK_TMP, META_WORK_TMP_JOURNAL] {
            let path = root.join(stale);
            if path.try_exists()? {
                remove_with_retry(&path)?;
            }
        }
        fs::copy(&work, &tmp)?;
        {
            let _migrated = SqliteMetaStore::open_at_sqlcipher(&tmp, dek.as_bytes())?;
        }
        fs::OpenOptions::new().write(true).open(&tmp)?.sync_all()?;
        crash_point("adopt_after_migrated_copy");
        rename_with_retry(&tmp, &work)?;
        sync_dir(root)?;
        Ok(())
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
        for stale in [META_WORK_TMP, META_WORK_TMP_JOURNAL, META_WORK_TMP_REKEY] {
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
        // An antivirus lock is transient, not a damaged seal (I3). Only absence
        // and a failed check read as unreadable.
        let encoded = fs::read(root.join(META_SEALED)).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                unreadable()
            } else {
                EncryptedVaultError::Io(e)
            }
        })?;
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
            self.root.join(META_WORK_TMP_REKEY),
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
        for dir in [PRE_RECOVERY_DIR, DISCARDED_DIR] {
            if contains_marker(&self.root.join(dir), marker) {
                return false;
            }
        }
        self.blobs.plaintext_marker_absent(marker)
    }
}

/// True if any file under `dir` contains the marker.
fn contains_marker(dir: &Path, marker: &[u8]) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            contains_marker(&path, marker)
        } else {
            fs::read(&path).is_ok_and(|bytes| bytes.windows(marker.len()).any(|w| w == marker))
        }
    })
}

#[cfg(test)]
mod test_hooks {
    use std::cell::Cell;

    thread_local! {
        static TRANSIENT_FAILURES: Cell<u32> = const { Cell::new(0) };
    }

    /// Makes the next keyed check report "not a database" once, as a failed salt
    /// read can on a good file.
    pub(super) fn inject_transient_failure() {
        TRANSIENT_FAILURES.with(|n| n.set(n.get() + 1));
    }

    pub(super) fn take_transient_failure() -> bool {
        TRANSIENT_FAILURES.with(|n| {
            let left = n.get();
            n.set(left.saturating_sub(1));
            left > 0
        })
    }
}

/// One keyed check of a leftover.
enum WorkCheckResult {
    Done(WorkDatabaseCheck),
    /// SQLite said the file itself is bad. Needs confirming.
    Definitive,
    Retryable(String),
}

impl WorkCheckResult {
    fn is_healthy(&self) -> bool {
        matches!(self, Self::Done(WorkDatabaseCheck::Healthy { .. }))
    }

    fn is_bad(&self) -> bool {
        matches!(
            self,
            Self::Definitive
                | Self::Done(WorkDatabaseCheck::Damaged | WorkDatabaseCheck::Incomplete)
        )
    }

    fn same_kind_of_bad(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Definitive, Self::Definitive)
                | (
                    Self::Done(WorkDatabaseCheck::Damaged),
                    Self::Done(WorkDatabaseCheck::Damaged)
                )
                | (
                    Self::Done(WorkDatabaseCheck::Incomplete),
                    Self::Done(WorkDatabaseCheck::Incomplete)
                )
        )
    }

    /// The allowlist. Everything else, including an unknown error, is retryable.
    fn from_error(error: &MetaError) -> Self {
        use rusqlite::ErrorCode;
        match error {
            MetaError::Sqlite(rusqlite::Error::SqliteFailure(failure, _))
                if matches!(
                    failure.code,
                    ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt
                ) =>
            {
                Self::Definitive
            }
            other => Self::Retryable(other.to_string()),
        }
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

    fn adopted_or_restored(vault: &EncryptedVault) -> (Vec<String>, WorkSource, bool) {
        (sources(vault), vault.work_source(), vault.notice_pending())
    }

    fn generation_verdicts(root: &Path) -> Vec<String> {
        let base = root.join(DISCARDED_DIR);
        if !base.exists() {
            return Vec::new();
        }
        EncryptedVault::generations(&base)
            .expect("generations")
            .iter()
            .map(|(_, path)| fs::read_to_string(path.join(VERDICT_FILE)).expect("verdict"))
            .collect()
    }

    fn open_vault(root: &Path) -> Result<EncryptedVault, EncryptedVaultError> {
        EncryptedVault::open_with_passphrase(root, PASS, HOLDER)
    }

    /// A session that wrote and then crashed: a real marker-bound leftover.
    fn crashed_session(tag: &str) -> PathBuf {
        let root = sealed_with_one_source(tag);
        let session = open_vault(&root).expect("open");
        session.insert_source(&source("committed")).expect("write");
        drop(session);
        root
    }

    #[test]
    fn a_crash_while_unlocked_recovers_committed_work_and_keeps_the_seal() {
        let root = crashed_session("recover");
        let seal = fs::read(root.join(META_SEALED)).expect("seal");
        let digest = DigestSha256::of(&seal).to_hex();
        let vault = open_vault(&root).expect("recover");
        assert_eq!(sources(&vault), ["committed", "sealed"]);
        assert_eq!(vault.work_source(), WorkSource::Adopted);
        let copy = root.join(PRE_RECOVERY_DIR).join(format!("{digest}.sealed"));
        assert_eq!(
            fs::read(&copy).expect("copy"),
            seal,
            "I2: the seal was kept"
        );
        assert_eq!(fs::read(root.join(META_SEALED)).expect("seal"), seal);
        vault.close().expect("close");

        let reopened = open_vault(&root).expect("open");
        assert_eq!(reopened.work_source(), WorkSource::Restored);
        assert_eq!(sources(&reopened), ["committed", "sealed"]);
        assert!(copy.exists(), "a later load purges nothing");
        reopened.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
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
            // An unverifiable leftover, so the restore path is the one taken.
            fs::write(root.join(META_WORK), b"not a database").expect("leftover");
            run_child(&root, "open", boundary);
            assert_eq!(
                fs::read(root.join(META_SEALED)).expect("seal"),
                seal,
                "{boundary}"
            );
            let vault = open_vault(&root).unwrap_or_else(|e| panic!("{boundary}: {e}"));
            let (names, _, notice) = adopted_or_restored(&vault);
            assert_eq!(names, ["sealed"], "{boundary}");
            assert!(notice, "{boundary}: the crash did not hide the discard");
            vault.close().expect("close");
            fs::remove_dir_all(&root).expect("cleanup");
        }
    }

    #[test]
    fn a_crash_during_close_never_loses_the_session() {
        // Before the rename the work file is recovered; from the rename on, the
        // new seal already holds it. Either way nothing is lost.
        for (boundary, source_after) in [
            ("seal_after_temp", WorkSource::Adopted),
            ("seal_after_rename", WorkSource::Restored),
            ("close_before_wipe", WorkSource::Restored),
            ("close_after_wipe", WorkSource::Restored),
        ] {
            let root = sealed_with_one_source("close");
            run_child(&root, "write_and_close", boundary);
            let vault = open_vault(&root).unwrap_or_else(|e| panic!("{boundary}: {e}"));
            assert_eq!(sources(&vault), ["in-this-session", "sealed"], "{boundary}");
            assert_eq!(vault.work_source(), source_after, "{boundary}");
            vault.close().expect("close");
            fs::remove_dir_all(&root).expect("cleanup");
        }
    }

    #[test]
    fn a_crash_during_adoption_leaves_a_session_the_next_open_recovers() {
        for boundary in ["adopt_after_copy", "adopt_before_open"] {
            let root = sealed_with_one_source("adopt");
            let session = open_vault(&root).expect("open");
            session.insert_source(&source("committed")).expect("write");
            drop(session);
            run_child(&root, "open", boundary);
            let vault = open_vault(&root).unwrap_or_else(|e| panic!("{boundary}: {e}"));
            assert_eq!(sources(&vault), ["committed", "sealed"], "{boundary}");
            assert_eq!(vault.work_source(), WorkSource::Adopted, "{boundary}");
            vault.close().expect("close");
            fs::remove_dir_all(&root).expect("cleanup");
        }
    }

    #[test]
    fn a_crash_mid_quarantine_keeps_the_files_and_the_notice() {
        let root = sealed_with_one_source("quarantine-crash");
        fs::write(root.join(META_WORK), b"not a database").expect("work");
        run_child(&root, "open", "quarantine_after_move");
        let vault = open_vault(&root).expect("open");
        assert!(vault.notice_pending());
        assert_eq!(sources(&vault), ["sealed"]);
        vault.close().expect("close");
        let moved = root.join(DISCARDED_DIR).join("1").join(META_WORK);
        assert_eq!(fs::read(moved).expect("kept"), b"not a database");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn what_cannot_be_verified_is_moved_aside_never_adopted() {
        let random: Vec<u8> = (0..8192_u32)
            .map(|i| (i.wrapping_mul(31) % 251) as u8)
            .collect();
        let plaintext = {
            let dir = scratch("plain");
            let path = dir.join("plain.sqlite3");
            {
                let _ = SqliteMetaStore::open_at(&path).expect("plain");
            }
            let bytes = fs::read(&path).expect("read");
            fs::remove_dir_all(&dir).expect("cleanup");
            bytes
        };
        for (name, bytes) in [
            ("empty", Vec::new()),
            ("short", vec![1_u8; 100]),
            ("zero-filled", vec![0_u8; 8192]),
            ("random", random),
            ("plaintext", plaintext),
        ] {
            let root = sealed_with_one_source("garbage");
            let seal = fs::read(root.join(META_SEALED)).expect("seal");
            fs::write(root.join(META_WORK), &bytes).expect("plant");
            let vault = open_vault(&root).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(sources(&vault), ["sealed"], "{name}");
            assert_eq!(vault.work_source(), WorkSource::Restored, "{name}");
            assert!(vault.notice_pending(), "{name}");
            assert_eq!(generation_verdicts(&root), [VERDICT_UNVERIFIABLE], "{name}");
            assert_eq!(
                fs::read(root.join(META_SEALED)).expect("seal"),
                seal,
                "{name}"
            );
            vault.close().expect("close");
            let kept = root.join(DISCARDED_DIR).join("1").join(META_WORK);
            assert_eq!(fs::read(kept).expect("kept"), bytes, "{name}: I6");
            fs::remove_dir_all(&root).expect("cleanup");
        }
    }

    #[test]
    fn a_missing_or_foreign_marker_is_never_adopted() {
        // Missing: unverifiable.
        let root = crashed_session("no-marker");
        {
            let key = {
                let header = EncryptedVault::load_header(&root).expect("header");
                KeyProvider::unlock_passphrase(&header, PASS).expect("key")
            };
            let store = SqliteMetaStore::open_existing_sqlcipher_no_migrate(
                &root.join(META_WORK),
                key.as_bytes(),
            )
            .expect("open");
            store
                .conn()
                .execute("DELETE FROM store_state WHERE key = 'base_seal_digest'", [])
                .expect("delete");
        }
        let vault = open_vault(&root).expect("open");
        assert_eq!(sources(&vault), ["sealed"]);
        assert_eq!(generation_verdicts(&root), [VERDICT_UNVERIFIABLE]);
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");

        // Naming an older seal: a work file from an earlier session. Not adopted.
        let root = sealed_with_one_source("old-work");
        let session = open_vault(&root).expect("open");
        session.insert_source(&source("older")).expect("write");
        let old_work = fs::read(root.join(META_WORK)).expect("work");
        session.close().expect("close");
        let session = open_vault(&root).expect("open");
        session.insert_source(&source("newer")).expect("write");
        session.close().expect("close");
        fs::write(root.join(META_WORK), old_work).expect("plant");
        let vault = open_vault(&root).expect("open");
        assert_eq!(
            sources(&vault),
            ["newer", "older", "sealed"],
            "the seal won"
        );
        assert!(vault.notice_pending(), "I7: every mismatch shows a notice");
        assert_eq!(generation_verdicts(&root), [VERDICT_STALE]);
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_deleted_seal_never_lets_a_work_file_be_adopted() {
        let root = crashed_session("no-seal");
        fs::remove_file(root.join(META_SEALED)).expect("delete");
        let work = fs::read(root.join(META_WORK)).expect("work");
        assert!(matches!(
            open_vault(&root),
            Err(EncryptedVaultError::SealedMetaUnreadable)
        ));
        assert_eq!(fs::read(root.join(META_WORK)).expect("work"), work);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_held_lock_is_retryable_and_changes_nothing() {
        let root = crashed_session("busy");
        let header = EncryptedVault::load_header(&root).expect("header");
        let key = KeyProvider::unlock_passphrase(&header, PASS).expect("key");
        let holder = SqliteMetaStore::open_existing_sqlcipher_no_migrate(
            &root.join(META_WORK),
            key.as_bytes(),
        )
        .expect("holder");
        holder
            .conn()
            .execute_batch("PRAGMA busy_timeout = 0; BEGIN EXCLUSIVE")
            .expect("lock");
        let work = fs::read(root.join(META_WORK));
        let seal = fs::read(root.join(META_SEALED)).expect("seal");
        let outcome = open_vault(&root);
        assert!(
            matches!(
                outcome,
                Err(EncryptedVaultError::WorkFileCheckUnavailable(_))
            ),
            "{outcome:?}"
        );
        assert_eq!(fs::read(root.join(META_SEALED)).expect("seal"), seal);
        assert!(generation_verdicts(&root).is_empty(), "nothing moved");
        if let Ok(before) = work {
            assert_eq!(fs::read(root.join(META_WORK)).expect("work"), before);
        }
        drop(holder);
        let vault = open_vault(&root).expect("recovers once the lock is gone");
        assert_eq!(sources(&vault), ["committed", "sealed"]);
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_confirmed_move_aside_opens_the_last_saved_state() {
        let root = crashed_session("move-aside");
        let vault = EncryptedVault::open_moving_leftover_aside(&root, Some(PASS), None, HOLDER)
            .expect("open");
        assert_eq!(sources(&vault), ["sealed"]);
        assert!(vault.notice_pending());
        assert_eq!(generation_verdicts(&root), [VERDICT_UNVERIFIABLE]);
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_interrupted_migration_or_a_newer_schema_is_never_adopted() {
        let key_of = |root: &Path| {
            let header = EncryptedVault::load_header(root).expect("header");
            KeyProvider::unlock_passphrase(&header, PASS).expect("key")
        };
        let root = crashed_session("interrupted");
        {
            let key = key_of(&root);
            let store = SqliteMetaStore::open_existing_sqlcipher_no_migrate(
                &root.join(META_WORK),
                key.as_bytes(),
            )
            .expect("open");
            store
                .conn()
                .execute(
                    "INSERT INTO migration_journal(version, state) VALUES (?1, 'started')",
                    rusqlite::params![crate::CURRENT_META_SCHEMA_VERSION + 5],
                )
                .expect("started");
        }
        let vault = open_vault(&root).expect("open");
        assert_eq!(sources(&vault), ["sealed"]);
        assert_eq!(generation_verdicts(&root), [VERDICT_UNVERIFIABLE]);
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");

        let root = crashed_session("newer-leftover");
        {
            let key = key_of(&root);
            let store = SqliteMetaStore::open_existing_sqlcipher_no_migrate(
                &root.join(META_WORK),
                key.as_bytes(),
            )
            .expect("open");
            store
                .conn()
                .execute(
                    "INSERT INTO migration_journal(version, state) VALUES (?1, 'finished')",
                    rusqlite::params![crate::CURRENT_META_SCHEMA_VERSION + 1],
                )
                .expect("finished");
        }
        assert!(matches!(
            open_vault(&root),
            Err(EncryptedVaultError::Meta(MetaError::UnsupportedSchema(_)))
        ));
        assert!(generation_verdicts(&root).is_empty(), "nothing moved");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_hot_journal_keeps_committed_rows_and_drops_uncommitted_ones() {
        let root = sealed_with_one_source("hot");
        let vault = open_vault(&root).expect("open");
        vault.insert_source(&source("committed")).expect("write");
        let committed_bytes = fs::read(root.join(META_WORK)).expect("work");
        let conn = vault.meta.conn();
        conn.execute_batch("PRAGMA cache_size = 1; BEGIN IMMEDIATE")
            .expect("begin");
        for i in 0..300 {
            vault
                .insert_source(&source(&format!("uncommitted-{i}")))
                .expect("uncommitted");
        }
        let work = fs::read(root.join(META_WORK)).expect("work");
        assert_ne!(work, committed_bytes, "uncommitted pages reached the file");
        let journal = fs::read(root.join("meta.work.sqlite3-journal")).expect("a real hot journal");
        conn.execute_batch("ROLLBACK").expect("rollback");
        drop(vault);
        fs::write(root.join(META_WORK), work).expect("work");
        fs::write(root.join("meta.work.sqlite3-journal"), journal).expect("journal");

        let recovered = open_vault(&root).expect("recover");
        assert_eq!(sources(&recovered), ["committed", "sealed"]);
        assert_eq!(recovered.work_source(), WorkSource::Adopted);
        recovered.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn one_transient_failure_ends_in_adoption_not_discard() {
        let root = crashed_session("transient");
        test_hooks::inject_transient_failure();
        let vault = open_vault(&root).expect("recover");
        assert_eq!(sources(&vault), ["committed", "sealed"]);
        assert_eq!(vault.work_source(), WorkSource::Adopted);
        assert!(generation_verdicts(&root).is_empty(), "nothing was moved");
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_work_file_under_another_key_is_never_adopted() {
        let root = sealed_with_one_source("wrong-key");
        {
            let other = root.join(META_WORK);
            let store = SqliteMetaStore::open_at_sqlcipher(&other, &[9_u8; 32]).expect("create");
            store.insert_source(&source("foreign")).expect("write");
        }
        let vault = open_vault(&root).expect("open");
        assert_eq!(sources(&vault), ["sealed"]);
        assert_eq!(generation_verdicts(&root), [VERDICT_UNVERIFIABLE]);
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    /// SQLCipher's error for a bad page is not documented here, so this asserts
    /// the guarantee, not the code: damaged work is never adopted, the seal is
    /// untouched, and nothing is lost whichever way it classifies.
    #[test]
    fn a_corrupt_page_is_never_adopted_and_the_seal_survives() {
        let root = crashed_session("corrupt");
        let seal = fs::read(root.join(META_SEALED)).expect("seal");
        let mut work = fs::read(root.join(META_WORK)).expect("work");
        assert!(work.len() > 8192, "more than one page");
        let page = 4096;
        for byte in &mut work[page + 100..page + 200] {
            *byte ^= 0xFF;
        }
        fs::write(root.join(META_WORK), &work).expect("corrupt");
        match open_vault(&root) {
            Ok(vault) => {
                assert_eq!(sources(&vault), ["sealed"], "opened from the seal");
                assert_eq!(generation_verdicts(&root), [VERDICT_UNVERIFIABLE]);
                vault.close().expect("close");
            }
            Err(e) => {
                println!("corrupt page classified as: {e}");
                assert!(
                    matches!(e, EncryptedVaultError::WorkFileCheckUnavailable(_)),
                    "{e}"
                );
                assert_eq!(fs::read(root.join(META_WORK)).expect("work"), work);
                assert_eq!(fs::read(root.join(META_SEALED)).expect("seal"), seal);
                let vault =
                    EncryptedVault::open_moving_leftover_aside(&root, Some(PASS), None, HOLDER)
                        .expect("the confirmed way out");
                assert_eq!(sources(&vault), ["sealed"]);
                vault.close().expect("close");
            }
        }
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn permission_denied_is_retryable_and_changes_nothing() {
        use std::os::unix::fs::PermissionsExt;
        let root = crashed_session("denied");
        let work = root.join(META_WORK);
        let before = fs::read(&work).expect("work");
        fs::set_permissions(&work, fs::Permissions::from_mode(0o000)).expect("chmod");
        let outcome = open_vault(&root);
        fs::set_permissions(&work, fs::Permissions::from_mode(0o600)).expect("chmod");
        assert!(
            matches!(
                outcome,
                Err(EncryptedVaultError::WorkFileCheckUnavailable(_))
            ),
            "{outcome:?}"
        );
        assert_eq!(fs::read(&work).expect("work"), before);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    /// An older schema: remove the newest migration's journal row. Its DDL is
    /// `CREATE ... IF NOT EXISTS`, so running it again is safe.
    fn age_work_file_schema(root: &Path) {
        let header = EncryptedVault::load_header(root).expect("header");
        let key = KeyProvider::unlock_passphrase(&header, PASS).expect("key");
        let store = SqliteMetaStore::open_existing_sqlcipher_no_migrate(
            &root.join(META_WORK),
            key.as_bytes(),
        )
        .expect("open");
        store
            .conn()
            .execute(
                "DELETE FROM migration_journal WHERE version = ?1",
                rusqlite::params![crate::CURRENT_META_SCHEMA_VERSION],
            )
            .expect("age");
    }

    #[test]
    fn an_older_schema_migrates_on_a_copy_and_a_crash_there_loses_nothing() {
        let root = crashed_session("migrate-crash");
        age_work_file_schema(&root);
        run_child(&root, "open", "adopt_after_migrated_copy");
        // The real work file was not touched: still the older schema.
        let vault = open_vault(&root).expect("recover");
        assert_eq!(sources(&vault), ["committed", "sealed"]);
        assert_eq!(vault.work_source(), WorkSource::Adopted);
        assert_eq!(
            vault
                .meta
                .migration_journal()
                .expect("journal")
                .finished_version,
            crate::CURRENT_META_SCHEMA_VERSION
        );
        vault.close().expect("close");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn retention_evicts_only_old_stale_generations() {
        let root = sealed_with_one_source("retention");
        let plant = |verdict: &str| {
            fs::write(root.join(META_WORK), b"x").expect("work");
            EncryptedVault::quarantine(&root, verdict).expect("quarantine");
        };
        plant(VERDICT_UNVERIFIABLE);
        for _ in 0..5 {
            plant(VERDICT_STALE);
        }
        let verdicts = generation_verdicts(&root);
        assert_eq!(
            verdicts.iter().filter(|v| *v == VERDICT_STALE).count(),
            RECOVERY_RETENTION
        );
        assert_eq!(
            verdicts
                .iter()
                .filter(|v| *v == VERDICT_UNVERIFIABLE)
                .count(),
            1,
            "an unverifiable generation stays until the user deletes it"
        );
        fs::remove_dir_all(&root).expect("cleanup");
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

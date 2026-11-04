//! Spec 109: a torn seal, a failed lock and a crash never destroy the last good
//! copy. Synthetic data only, real SQLCipher.

use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId};
use medscale_storage::{EncryptedVault, EncryptedVaultError, SourceMeta};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const PASS: &str = "passphrase-ok";
const SEALED: &str = "meta.sealed";
const WORK: &str = "meta.work.sqlite3";

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-109-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn sealed_vault(name: &str) -> PathBuf {
    let root = temp_root(name);
    let (vault, _) = EncryptedVault::create("v-109", &root, PASS, "holder-a", None).unwrap();
    assert!(!vault.discarded_interrupted_session());
    vault.close().unwrap();
    root
}

fn open(root: &Path) -> Result<EncryptedVault, EncryptedVaultError> {
    EncryptedVault::open_with_passphrase(root, PASS, "holder-a")
}

/// Every file name and its bytes, directly under `root`.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().unwrap().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

fn torn(root: &Path) {
    let bytes = fs::read(root.join(SEALED)).unwrap();
    fs::write(root.join(SEALED), &bytes[..bytes.len() / 2]).unwrap();
}

fn source_count(vault: &EncryptedVault) -> usize {
    vault.meta.list_sources().unwrap().len()
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

#[test]
fn a_torn_seal_refuses_and_destroys_nothing() {
    let root = sealed_vault("torn-alone");
    torn(&root);
    let before = snapshot(&root);
    assert!(matches!(
        open(&root),
        Err(EncryptedVaultError::SealedMetaUnreadable)
    ));
    assert_eq!(snapshot(&root), before, "nothing on disk changed");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_torn_seal_keeps_the_work_database_it_could_not_replace() {
    let root = temp_root("torn-work");
    let (vault, _) = EncryptedVault::create("v-109", &root, PASS, "holder-a", None).unwrap();
    vault.insert_source(&source("kept")).unwrap();
    drop(vault); // A crash: the only copy of `kept` is the work database.
    torn(&root);
    let work_before = fs::read(root.join(WORK)).unwrap();
    assert!(matches!(
        open(&root),
        Err(EncryptedVaultError::SealedMetaUnreadable)
    ));
    assert_eq!(fs::read(root.join(WORK)).unwrap(), work_before);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_wrong_passphrase_changes_nothing_even_with_a_leftover() {
    // After a clean close no lease exists, so a lease taken before the key is
    // checked would be caught here as a new file.
    let root = sealed_vault("wrong-pass");
    fs::write(root.join(WORK), b"leftover").unwrap();
    let before = snapshot(&root);
    assert!(matches!(
        EncryptedVault::open_with_passphrase(&root, "not-the-pass", "holder-b"),
        Err(EncryptedVaultError::MissingKeyMaterial)
    ));
    assert_eq!(snapshot(&root), before);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_crash_discards_the_session_and_says_so() {
    let root = sealed_vault("crash");
    let vault = open(&root).unwrap();
    assert!(
        !vault.discarded_interrupted_session(),
        "a clean close reports nothing"
    );
    vault.insert_source(&source("unsealed")).unwrap();
    drop(vault);

    let reopened = open(&root).unwrap();
    assert!(reopened.discarded_interrupted_session());
    assert_eq!(source_count(&reopened), 0, "opened from the last seal");
    reopened.close().unwrap();

    let clean = open(&root).unwrap();
    assert!(!clean.discarded_interrupted_session());
    clean.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn stale_temp_files_and_a_stale_journal_are_ignored() {
    let root = sealed_vault("stale");
    fs::write(root.join("meta.sealed.tmp"), b"half a seal").unwrap();
    fs::write(root.join("meta.work.sqlite3.tmp"), b"half a copy").unwrap();
    fs::write(root.join("meta.work.sqlite3-journal"), b"stale journal").unwrap();
    let vault = open(&root).unwrap();
    assert!(!root.join("meta.sealed.tmp").exists());
    assert!(!root.join("meta.work.sqlite3.tmp").exists());
    assert!(!root.join("meta.work.sqlite3-journal").exists());
    assert_eq!(source_count(&vault), 0);
    vault.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_failed_seal_hands_back_an_open_vault_and_a_retry_succeeds() {
    let root = sealed_vault("retry");
    let vault = open(&root).unwrap();
    vault.insert_source(&source("saved")).unwrap();
    let seal_before = fs::read(root.join(SEALED)).unwrap();

    // A directory at the temp path makes the seal fail like a full disk would.
    fs::create_dir(root.join("meta.sealed.tmp")).unwrap();
    let failed = vault.try_close().expect_err("seal must fail");
    let (vault, _error) = *failed;
    assert_eq!(fs::read(root.join(SEALED)).unwrap(), seal_before);
    assert!(root.join(WORK).exists(), "the work file was not wiped");
    assert_eq!(source_count(&vault), 1, "still open and readable");
    vault.insert_source(&source("after-failure")).unwrap();

    fs::remove_dir(root.join("meta.sealed.tmp")).unwrap();
    assert!(vault.try_close().is_ok());
    assert!(!EncryptedVault::leftover_work_present(&root));

    let reopened = open(&root).unwrap();
    assert!(!reopened.discarded_interrupted_session());
    assert_eq!(source_count(&reopened), 2, "both writes were sealed");
    reopened.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_second_open_of_a_live_session_is_refused_and_wipes_nothing() {
    let root = sealed_vault("live");
    let live = open(&root).unwrap();
    live.insert_source(&source("live-row")).unwrap();
    // The same holder name is what every Desktop and CLI session uses.
    assert!(matches!(
        open(&root),
        Err(EncryptedVaultError::LeaseHeld(_))
    ));
    assert_eq!(source_count(&live), 1, "the live session is untouched");
    live.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_lease_left_by_a_crash_or_torn_write_does_not_strand_the_vault() {
    let root = sealed_vault("lease");
    for stale in [&b""[..], b"holder-that-crashed", b"holder-tha"] {
        fs::write(root.join(".writer.lock"), stale).unwrap();
        let vault = EncryptedVault::open_with_passphrase(&root, PASS, "holder-z").unwrap();
        vault.close().unwrap();
    }
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_session_lock_file_survives_close_and_is_empty() {
    let root = sealed_vault("lock-file");
    assert_eq!(fs::read(root.join(".session.lock")).unwrap(), b"");
    let vault = open(&root).unwrap();
    vault.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_clean_close_leaves_no_session_files() {
    let root = sealed_vault("clean");
    let names: Vec<String> = snapshot(&root).into_keys().collect();
    for gone in [
        ".writer.lock",
        "meta.sealed.tmp",
        WORK,
        "meta.work.sqlite3.tmp",
    ] {
        assert!(!names.iter().any(|n| n == gone), "{gone} remained");
    }
    assert!(root.join(SEALED).exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn only_a_real_absence_reads_as_a_missing_blob() {
    use medscale_storage::SealedBlobError;
    let root = temp_root("blob-errors");
    let (vault, _) = EncryptedVault::create("v-109", &root, PASS, "holder-a", None).unwrap();
    let digest = vault.put_blob(b"synthetic bytes").unwrap();
    let path = root
        .join("sealed_blobs")
        .join(format!("{}.sealed", digest.to_hex()));

    // A directory at the path fails the read without the file being absent,
    // the shape of an antivirus lock: transient, not lost.
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(matches!(
        vault.get_blob(&digest),
        Err(EncryptedVaultError::Sealed(SealedBlobError::Io(_)))
    ));
    fs::remove_dir(&path).unwrap();
    assert!(matches!(
        vault.get_blob(&digest),
        Err(EncryptedVaultError::Sealed(SealedBlobError::NotFound))
    ));

    // A blob that exists is not rewritten, so re-putting costs no fsync.
    let digest = vault.put_blob(b"synthetic bytes").unwrap();
    let path = root
        .join("sealed_blobs")
        .join(format!("{}.sealed", digest.to_hex()));
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    vault.put_blob(b"synthetic bytes").unwrap();
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    vault.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

//! Spec 017 vault privacy lifecycle tests.

use medscale_storage::EncryptedVault;
use std::fs;
use std::path::PathBuf;

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-017-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn encrypted_close_removes_work_and_sidecars_017() {
    let root = temp_root("wipe");
    let (vault, _) =
        EncryptedVault::create("v-priv", &root, "passphrase-ok", "holder-a", None).unwrap();
    let work = root.join("meta.work.sqlite3");
    assert!(work.exists(), "work file present while open");
    // Simulate sidecars that SQLite may leave after a crash.
    fs::write(format!("{}-wal", work.display()), b"wal-leftover").unwrap();
    fs::write(format!("{}-shm", work.display()), b"shm-leftover").unwrap();
    vault.close().unwrap();
    assert!(
        !EncryptedVault::leftover_work_present(&root),
        "plaintext work/sidecars must be absent after close"
    );
    assert!(root.join("meta.sealed").exists());
}

#[test]
fn leftover_work_is_not_adopted_and_is_moved_aside_017() {
    let root = temp_root("leftover");
    let (vault, codes) =
        EncryptedVault::create("v-left", &root, "passphrase-ok", "holder-a", None).unwrap();
    vault.close().unwrap();

    // Plant crash leftover after sealed close.
    let work = root.join("meta.work.sqlite3");
    fs::write(&work, b"STALE_PLAINTEXT_SHOULD_NOT_SURVIVE_OPEN").unwrap();
    assert!(EncryptedVault::leftover_work_present(&root));

    let code = codes.codes.first().cloned().expect("recovery code");
    let reopened = EncryptedVault::open_with_recovery(&root, &code, "holder-b").unwrap();
    // Spec 109: a leftover that can't be verified is moved aside, not deleted,
    // and the session runs on a fresh copy of the seal. Close must wipe that.
    assert!(root.join("meta.work.sqlite3").exists());
    assert_ne!(
        fs::read(root.join("meta.work.sqlite3")).unwrap(),
        b"STALE_PLAINTEXT_SHOULD_NOT_SURVIVE_OPEN"
    );
    assert!(reopened.notice_pending());
    assert_eq!(
        fs::read(
            root.join("discarded-session")
                .join("1")
                .join("meta.work.sqlite3")
        )
        .unwrap(),
        b"STALE_PLAINTEXT_SHOULD_NOT_SURVIVE_OPEN"
    );
    reopened.close().unwrap();
    assert!(!EncryptedVault::leftover_work_present(&root));
    let _ = work;
}

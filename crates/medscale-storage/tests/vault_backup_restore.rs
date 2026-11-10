//! Verified backup of a locked vault and restore that keeps the replaced
//! vault (Desktop "Lock and back up" and CLI `vault backup` / `vault restore`).

use medscale_storage::{EncryptedVault, EncryptedVaultError};
use std::fs;
use std::path::PathBuf;

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-bkr-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn backup_of_a_locked_vault_restores_over_another_vault_and_keeps_it() {
    let base = temp_root("roundtrip");
    let root = base.join("vault");
    let (vault, _) = EncryptedVault::create("v-a", &root, "pw-a", "h1", None).unwrap();
    let digest = vault.put_blob(b"first-vault-payload").unwrap();
    vault.close().unwrap();

    let backups = base.join("backups");
    let report = EncryptedVault::backup_locked(&root, &backups, "20261110-1").unwrap();
    assert_eq!(report.path, backups.join("MedScale-backup-20261110-1"));
    assert!(report.files >= 3, "header, sealed meta and the blob");
    assert!(report.bytes > 0);
    // A second backup with the same label gets its own folder.
    let again = EncryptedVault::backup_locked(&root, &backups, "20261110-1").unwrap();
    assert_eq!(again.path, backups.join("MedScale-backup-20261110-1-2"));

    // Replace the vault with a different one, then restore the backup.
    fs::remove_dir_all(&root).unwrap();
    let (other, _) = EncryptedVault::create("v-b", &root, "pw-b", "h2", None).unwrap();
    other.close().unwrap();

    let restored = EncryptedVault::restore_replacing(&report.path, &root, "20261110-2").unwrap();
    let kept = restored
        .previous_kept_at
        .expect("the replaced vault is kept");
    assert_eq!(kept, base.join("vault.before-restore-20261110-2"));

    let opened = EncryptedVault::open_with_passphrase(&root, "pw-a", "h3").unwrap();
    assert_eq!(opened.get_blob(&digest).unwrap(), b"first-vault-payload");
    opened.close().unwrap();
    let previous = EncryptedVault::open_with_passphrase(&kept, "pw-b", "h4").unwrap();
    previous.close().unwrap();
}

#[test]
fn restore_into_an_empty_place_keeps_nothing() {
    let base = temp_root("fresh");
    let root = base.join("vault");
    let (vault, _) = EncryptedVault::create("v-f", &root, "pw", "h1", None).unwrap();
    vault.close().unwrap();
    let report = EncryptedVault::backup_locked(&root, &base.join("b"), "x").unwrap();

    let target = base.join("new-machine-vault");
    let restored = EncryptedVault::restore_replacing(&report.path, &target, "y").unwrap();
    assert_eq!(restored.previous_kept_at, None);
    EncryptedVault::open_with_passphrase(&target, "pw", "h2")
        .unwrap()
        .close()
        .unwrap();
}

#[test]
fn an_open_vault_is_neither_backed_up_nor_replaced() {
    let base = temp_root("open");
    let root = base.join("vault");
    let (vault, _) = EncryptedVault::create("v-o", &root, "pw", "h1", None).unwrap();
    let held = EncryptedVault::backup_locked(&root, &base.join("b"), "x");
    assert!(
        matches!(held, Err(EncryptedVaultError::LeaseHeld(_))),
        "{held:?}"
    );
    vault.close().unwrap();

    let report = EncryptedVault::backup_locked(&root, &base.join("b"), "x").unwrap();
    let vault = EncryptedVault::open_with_passphrase(&root, "pw", "h2").unwrap();
    let replace = EncryptedVault::restore_replacing(&report.path, &root, "y");
    assert!(
        matches!(replace, Err(EncryptedVaultError::LeaseHeld(_))),
        "{replace:?}"
    );
    vault.close().unwrap();
    assert!(!base.join("vault.before-restore-y").exists());
}

#[test]
fn bad_places_and_labels_are_refused() {
    let base = temp_root("refuse");
    let root = base.join("vault");
    let (vault, _) = EncryptedVault::create("v-r", &root, "pw", "h1", None).unwrap();
    vault.close().unwrap();

    for label in ["", "../x", "a b", "a/b"] {
        assert!(EncryptedVault::backup_locked(&root, &base.join("b"), label).is_err());
    }
    assert!(EncryptedVault::backup_locked(&root, &root.join("inside"), "x").is_err());
    assert!(EncryptedVault::backup_locked(&base.join("nothing"), &base.join("b"), "x").is_err());

    let not_a_backup = base.join("empty");
    fs::create_dir_all(&not_a_backup).unwrap();
    assert!(EncryptedVault::restore_replacing(&not_a_backup, &root, "x").is_err());
    let report = EncryptedVault::backup_locked(&root, &base.join("b"), "x").unwrap();
    assert!(EncryptedVault::restore_replacing(&report.path, &report.path, "x").is_err());
}

#[test]
fn a_tampered_copy_is_detected() {
    let base = temp_root("tamper");
    let root = base.join("vault");
    let (vault, _) = EncryptedVault::create("v-t", &root, "pw", "h1", None).unwrap();
    vault.close().unwrap();
    let report = EncryptedVault::backup_locked(&root, &base.join("b"), "x").unwrap();
    // A backup that is changed after it was made is still restored byte for
    // byte, but it then fails to open: the seal is authenticated.
    let meta = report.path.join("meta.sealed");
    let mut bytes = fs::read(&meta).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    fs::write(&meta, bytes).unwrap();
    let target = base.join("restored");
    EncryptedVault::restore_replacing(&report.path, &target, "y").unwrap();
    assert!(EncryptedVault::open_with_passphrase(&target, "pw", "h2").is_err());
}

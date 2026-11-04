//! Spec 109: through Core, a failed seal leaves the vault open and retryable,
//! and an open that discards an unsealed session reports it. Synthetic only.

use medscale_core::CliSession;
use std::fs;
use std::path::PathBuf;

const PASS: &str = "passphrase-ok";

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-109c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_failed_close_stays_open_and_a_retry_seals() {
    let root = tmp_dir("retry");
    let root_text = root.display().to_string();
    let mut session = CliSession::connect("v-109").unwrap();
    session.create_encrypted_vault(&root_text, PASS).unwrap();

    fs::create_dir(root.join("meta.sealed.tmp")).unwrap();
    assert!(session.close_encrypted_vault().is_err());
    assert!(session.is_open(), "the vault is still open");
    assert!(root.join("meta.work.sqlite3").exists());

    fs::remove_dir(root.join("meta.sealed.tmp")).unwrap();
    session.close_encrypted_vault().unwrap();
    assert!(!session.is_open());
    assert!(!root.join("meta.work.sqlite3").exists());
    drop(session);

    let mut reopened = CliSession::connect("v-109").unwrap();
    reopened.open_encrypted_vault(&root_text, PASS).unwrap();
    assert!(!reopened.discarded_interrupted_session());
    reopened.close_encrypted_vault().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_unsealed_session_is_reported_on_the_next_open() {
    let root = tmp_dir("crash");
    let root_text = root.display().to_string();
    let mut session = CliSession::connect("v-109").unwrap();
    session.create_encrypted_vault(&root_text, PASS).unwrap();
    session.close_encrypted_vault().unwrap();
    session.open_encrypted_vault(&root_text, PASS).unwrap();
    drop(session); // A crash: nothing sealed this session.

    let mut reopened = CliSession::connect("v-109").unwrap();
    reopened.open_encrypted_vault(&root_text, PASS).unwrap();
    assert!(reopened.discarded_interrupted_session());
    reopened.close_encrypted_vault().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

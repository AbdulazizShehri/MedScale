//! Spec 023: EncryptedVault open-metadata SQLCipher privacy.

use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId};
use medscale_storage::{EncryptedVault, SourceMeta, SqliteMetaStore};
use std::fs;
use std::path::PathBuf;

fn temp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-023-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn wrong_passphrase_fail_closed_023() {
    let root = temp_root("wrong-pw");
    let (vault, _) =
        EncryptedVault::create("v-023", &root, "passphrase-ok", "holder-a", None).unwrap();
    vault.close().unwrap();
    assert!(
        EncryptedVault::open_with_passphrase(&root, "wrong-passphrase", "holder-b").is_err(),
        "wrong passphrase must fail closed"
    );
}

#[test]
fn sqlcipher_wrong_key_fail_closed_023() {
    let root = temp_root("wrong-key");
    let (vault, _) =
        EncryptedVault::create("v-key", &root, "passphrase-ok", "holder-a", None).unwrap();
    let work = root.join("meta.work.sqlite3");
    assert!(work.exists());
    let wrong = [0x11_u8; 32];
    assert!(
        SqliteMetaStore::open_at_sqlcipher(&work, &wrong).is_err(),
        "wrong SQLCipher key must fail closed"
    );
    vault.close().unwrap();
}

#[test]
fn roundtrip_authority_meta_through_sqlcipher_023() {
    let root = temp_root("roundtrip");
    let marker = b"SYNTHETIC_CLINICAL_MARKER_SQLCIPHER_023_SHOULD_NOT_LEAK";
    let (vault, _) =
        EncryptedVault::create("v-rt", &root, "passphrase-ok", "holder-a", None).unwrap();
    let digest = vault.put_blob(marker).unwrap();
    vault
        .insert_source(&SourceMeta {
            source_id: OpaqueId::new("src-023"),
            realm_id: RealmId::new("r"),
            authority_scope_id: AuthorityScopeId::new("s"),
            digest: digest.clone(),
            byte_length: marker.len() as u64,
            media_type: "text/plain".to_owned(),
            visible: true,
            resource_type: "Marker".to_owned(),
        })
        .unwrap();
    vault.close().unwrap();

    let reopened =
        EncryptedVault::open_with_passphrase(&root, "passphrase-ok", "holder-b").unwrap();
    let sources = reopened.meta.list_sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].source_id.as_str(), "src-023");
    assert_eq!(reopened.get_blob(&digest).unwrap(), marker);
    reopened.close().unwrap();
}

#[test]
fn open_work_disk_scan_hides_plaintext_marker_023() {
    let root = temp_root("scan");
    let marker = b"PLANTED_PLAINTEXT_META_MARKER_FOR_DISK_SCAN_023";
    let (vault, _) =
        EncryptedVault::create("v-scan", &root, "passphrase-ok", "holder-a", None).unwrap();
    let digest = vault.put_blob(b"blob-bytes").unwrap();
    vault
        .insert_source(&SourceMeta {
            source_id: OpaqueId::new("src-scan"),
            realm_id: RealmId::new("r"),
            authority_scope_id: AuthorityScopeId::new("s"),
            digest,
            byte_length: 10,
            media_type: "text/plain".to_owned(),
            visible: true,
            // Plant marker in durable metadata columns that would appear in plaintext SQLite.
            resource_type: String::from_utf8_lossy(marker).into_owned(),
        })
        .unwrap();

    let work = root.join("meta.work.sqlite3");
    assert!(work.exists(), "work file present while unlocked");
    let work_bytes = fs::read(&work).unwrap();
    assert!(
        !work_bytes.windows(marker.len()).any(|w| w == marker),
        "open work DB must not contain planted plaintext marker (page encryption)"
    );
    assert!(
        vault.plaintext_marker_absent_on_disk(marker),
        "work/sealed/blob surfaces must hide marker"
    );

    let version = vault.sqlcipher_cipher_version().unwrap();
    assert!(
        !version.is_empty(),
        "cipher_version must be non-empty for evidence"
    );
    // Recorded in evidence/admission after measured run; assert SQLCipher is active.
    assert!(EncryptedVault::sqlcipher_enabled());
    eprintln!("Spec 023 SQLCipher cipher_version={version}");

    vault.close().unwrap();
}

#[test]
fn sqlcipher_backend_active_023() {
    assert!(SqliteMetaStore::sqlcipher_backend_active());
    assert!(EncryptedVault::sqlcipher_enabled());
}

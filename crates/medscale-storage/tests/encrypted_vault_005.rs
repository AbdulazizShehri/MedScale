//! Spec 005 encrypted vault tests.

use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId};
use medscale_keys::MemoryKeyStore;
use medscale_storage::{
    EncryptedVault, SourceMeta, SyntheticVault, assert_claim_path, default_vault_root,
};
use std::fs;

fn temp_root(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-005-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn encrypted_vault_roundtrip() {
    let root = temp_root("roundtrip");
    let marker = b"SYNTHETIC_MARKER_PLAINTEXT_SHOULD_NOT_APPEAR";
    let (vault, _codes) =
        EncryptedVault::create("v-enc", &root, "passphrase-ok", "holder-a", None).unwrap();
    let digest = vault.put_blob(marker).unwrap();
    vault
        .insert_source(&SourceMeta {
            source_id: OpaqueId::new("src-1"),
            realm_id: RealmId::new("r"),
            authority_scope_id: AuthorityScopeId::new("s"),
            digest: digest.clone(),
            byte_length: marker.len() as u64,
            media_type: "text/plain".to_owned(),
            visible: true,
            resource_type: "Marker".to_owned(),
        })
        .unwrap();
    // While open, sealed files must not contain plaintext marker.
    assert!(vault.plaintext_marker_absent_on_disk(marker));
    vault.close().unwrap();

    assert!(
        EncryptedVault::open_with_passphrase(&root, "wrong", "holder-b").is_err(),
        "wrong key must fail closed"
    );

    let reopened =
        EncryptedVault::open_with_passphrase(&root, "passphrase-ok", "holder-b").unwrap();
    let got = reopened.get_blob(&digest).unwrap();
    assert_eq!(got, marker);
    assert!(reopened.plaintext_marker_absent_on_disk(marker));
    reopened.close().unwrap();

    // At rest: sealed meta + blobs must not contain marker
    let sealed = fs::read(root.join("meta.sealed")).unwrap();
    assert!(!sealed.windows(marker.len()).any(|w| w == marker));
}

#[test]
fn vault_sync_root_and_lease() {
    assert!(assert_claim_path(std::path::Path::new("C:/Users/x/OneDrive/vault")).is_err());
    assert!(assert_claim_path(std::path::Path::new("/home/u/Dropbox/vault")).is_err());
    assert!(assert_claim_path(std::path::Path::new("/Users/u/iCloud/vault")).is_err());
    assert!(assert_claim_path(std::path::Path::new("D:/Google Drive/vault")).is_err());

    let root = temp_root("lease");
    let (vault, _) = EncryptedVault::create("v-lease", &root, "pw", "holder-1", None).unwrap();
    let err = EncryptedVault::open_with_passphrase(&root, "pw", "holder-2");
    assert!(matches!(
        err,
        Err(medscale_storage::EncryptedVaultError::LeaseHeld(_))
    ));
    vault.close().unwrap();

    let app = default_vault_root("demo");
    assert!(
        app.to_string_lossy()
            .to_ascii_lowercase()
            .contains("medscale")
            || app.to_string_lossy().contains("MedScale")
    );
}

#[test]
fn vault_recovery_key_loss() {
    let root = temp_root("recovery");
    let store = MemoryKeyStore::new();
    let (vault, codes) =
        EncryptedVault::create("v-rec", &root, "pw-main", "h1", Some(&store)).unwrap();
    let digest = vault.put_blob(b"secret-bytes-xyz").unwrap();
    vault.close().unwrap();

    store.wipe();
    assert!(EncryptedVault::open_with_keystore(&root, &store, "h2").is_err());

    let via_pw = EncryptedVault::open_with_passphrase(&root, "pw-main", "h2").unwrap();
    assert_eq!(via_pw.get_blob(&digest).unwrap(), b"secret-bytes-xyz");
    via_pw.close().unwrap();

    let via_code = EncryptedVault::open_with_recovery(&root, &codes.codes[3], "h3").unwrap();
    assert_eq!(via_code.get_blob(&digest).unwrap(), b"secret-bytes-xyz");
    via_code.close().unwrap();

    EncryptedVault::destroy_key_material(&root).unwrap();
    assert!(EncryptedVault::open_with_passphrase(&root, "pw-main", "h4").is_err());
    let sealed = fs::read(root.join("meta.sealed")).unwrap();
    assert!(
        !sealed
            .windows(b"secret-bytes-xyz".len())
            .any(|w| w == b"secret-bytes-xyz")
    );
}

#[test]
fn encrypted_backup_restore_retention() {
    let root = temp_root("bak-src");
    let dest = temp_root("bak-dst");
    let restore_to = temp_root("bak-restore");
    let (vault, _) = EncryptedVault::create("v-bak", &root, "pw", "h1", None).unwrap();
    let digest = vault.put_blob(b"backup-payload").unwrap();
    vault.close().unwrap();

    EncryptedVault::backup(&root, &dest).unwrap();
    EncryptedVault::restore(&dest, &restore_to).unwrap();

    assert!(EncryptedVault::open_with_passphrase(&restore_to, "wrong", "h2").is_err());
    let opened = EncryptedVault::open_with_passphrase(&restore_to, "pw", "h2").unwrap();
    assert_eq!(opened.get_blob(&digest).unwrap(), b"backup-payload");
    opened.close().unwrap();

    EncryptedVault::destroy_key_material(&restore_to).unwrap();
    assert!(EncryptedVault::open_with_passphrase(&restore_to, "pw", "h3").is_err());
}

#[test]
fn migrate_synthetic_to_encrypted() {
    let synth_root = temp_root("synth");
    let enc_root = temp_root("enc");
    let synth = SyntheticVault::open("mig", &synth_root).unwrap();
    let bytes = b"migrate-me";
    let bref = synth.blobs.put_blob(bytes).unwrap();
    synth
        .meta
        .insert_source(&SourceMeta {
            source_id: OpaqueId::new("s-mig"),
            realm_id: RealmId::new("r"),
            authority_scope_id: AuthorityScopeId::new("s"),
            digest: bref.digest.clone(),
            byte_length: bytes.len() as u64,
            media_type: "application/octet-stream".to_owned(),
            visible: true,
            resource_type: "Binary".to_owned(),
        })
        .unwrap();
    drop(synth);

    let (enc, _) =
        EncryptedVault::migrate_from_synthetic(&synth_root, &enc_root, "mig", "pw", "h1").unwrap();
    assert_eq!(enc.get_blob(&bref.digest).unwrap(), bytes);
    enc.close().unwrap();
}

#[test]
fn digest_helper_compiles() {
    let _ = DigestSha256::of(b"x").to_hex();
}

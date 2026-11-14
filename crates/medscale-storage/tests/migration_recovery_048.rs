//! Spec 048 — migration interrupt detection + recovery via backup/restore.

use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId};
use medscale_storage::{
    EncryptedVault, MetaError, MigrationJournal, SourceMeta, SyntheticVault, VaultError,
    backup_vault, restore_vault,
};
use std::fs;

fn temp_root(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("medscale-048-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn migration_journal_interrupted_predicate() {
    let clean = MigrationJournal {
        finished_version: 2,
        started_version: None,
    };
    assert!(!clean.interrupted());
    let interrupted = MigrationJournal {
        finished_version: 2,
        started_version: Some(3),
    };
    assert!(interrupted.interrupted());
}

#[test]
fn migration_interrupt_detected_fail_closed_on_reopen() {
    let root = temp_root("mig-int");
    {
        let vault = SyntheticVault::open("v-int", &root).unwrap();
        let journal = vault.meta.migration_journal().unwrap();
        assert!(!journal.interrupted());
        assert!(journal.finished_version >= 2);
        vault
            .meta
            .begin_migration(journal.finished_version + 1)
            .unwrap();
        assert!(vault.meta.migration_journal().unwrap().interrupted());
        // Drop without finish_migration — simulates crash mid-migration.
    }

    let err = SyntheticVault::open("v-int", &root);
    assert!(
        matches!(
            err,
            Err(VaultError::Meta(MetaError::MigrationIncomplete(_)))
        ),
        "interrupted migration must fail closed, got {err:?}"
    );
}

#[test]
fn interrupted_synthetic_vault_recovered_via_backup_restore() {
    let root = temp_root("src");
    let bak = temp_root("bak");
    let restored = temp_root("restored");
    let payload = b"048-recovery-payload";

    let digest = {
        let vault = SyntheticVault::open("v-rec", &root).unwrap();
        let bref = vault.blobs.put_blob(payload).unwrap();
        vault
            .meta
            .insert_source(&SourceMeta {
                source_id: OpaqueId::new("s-048"),
                realm_id: RealmId::new("r"),
                authority_scope_id: AuthorityScopeId::new("s"),
                digest: bref.digest.clone(),
                byte_length: payload.len() as u64,
                media_type: "application/octet-stream".to_owned(),
                visible: true,
                resource_type: "Binary".to_owned(),
            })
            .unwrap();
        backup_vault(&vault, &bak).unwrap();
        let journal = vault.meta.migration_journal().unwrap();
        vault
            .meta
            .begin_migration(journal.finished_version + 1)
            .unwrap();
        bref.digest
    };

    assert!(matches!(
        SyntheticVault::open("v-rec", &root),
        Err(VaultError::Meta(MetaError::MigrationIncomplete(_)))
    ));

    restore_vault(&bak, &restored).unwrap();
    let vault = SyntheticVault::open("v-rec", &restored).unwrap();
    assert_eq!(vault.blobs.get_blob(&digest).unwrap(), payload);
    assert!(!vault.meta.migration_journal().unwrap().interrupted());
}

#[test]
fn encrypted_backup_restore_release_bar_closure() {
    let root = temp_root("enc-src");
    let dest = temp_root("enc-bak");
    let restore_to = temp_root("enc-restore");
    let (vault, _) = EncryptedVault::create("v-enc-048", &root, "pw", "h1", None).unwrap();
    let digest = vault.put_blob(b"enc-048-payload").unwrap();
    vault.close().unwrap();

    EncryptedVault::backup(&root, &dest).unwrap();
    EncryptedVault::restore(&dest, &restore_to).unwrap();

    assert!(EncryptedVault::open_with_passphrase(&restore_to, "wrong", "h2").is_err());
    let opened = EncryptedVault::open_with_passphrase(&restore_to, "pw", "h2").unwrap();
    assert_eq!(opened.get_blob(&digest).unwrap(), b"enc-048-payload");
    assert_eq!(digest, DigestSha256::of(b"enc-048-payload"));
    opened.close().unwrap();
}

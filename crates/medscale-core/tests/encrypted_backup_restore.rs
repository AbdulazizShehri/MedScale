//! Backup and restore of a locked encrypted vault through
//! `CliSession` -> `CoreFacade::dispatch`.

use std::path::PathBuf;

use medscale_contracts::envelopes::AuthorityError;
use medscale_core::CliSession;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-bkr-core-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_locked_vault_backs_up_and_restores_through_core() {
    let base = temp("roundtrip");
    let root = base.join("vault").display().to_string();
    let parent = base.join("backups").display().to_string();

    let mut s = CliSession::connect("vault-bkr").unwrap();
    s.create_encrypted_vault(&root, "pw-backup").unwrap();
    // Refused while the vault is open: its newest changes are not sealed.
    let open = s.backup_encrypted_vault(&root, &parent, "a");
    assert!(
        matches!(open, Err(AuthorityError::InvalidArgument { .. })),
        "{open:?}"
    );
    s.close_encrypted_vault().unwrap();

    let (folder, files, bytes) = s.backup_encrypted_vault(&root, &parent, "a").unwrap();
    assert!(folder.ends_with("MedScale-backup-a"), "{folder}");
    assert!(files >= 2 && bytes > 0);

    let kept = s.restore_encrypted_vault(&folder, &root, "b").unwrap();
    let kept = kept.expect("the replaced vault is kept");
    assert!(kept.ends_with("vault.before-restore-b"), "{kept}");
    assert!(
        std::path::Path::new(&kept)
            .join("vault_header.json")
            .is_file()
    );

    s.open_encrypted_vault(&root, "pw-backup").unwrap();
    let restore_while_open = s.restore_encrypted_vault(&folder, &root, "c");
    assert!(
        matches!(
            restore_while_open,
            Err(AuthorityError::InvalidArgument { .. })
        ),
        "{restore_while_open:?}"
    );
    s.close_encrypted_vault().unwrap();
}

#[test]
fn a_folder_that_is_not_a_backup_is_refused() {
    let base = temp("refuse");
    let empty = base.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let mut s = CliSession::connect("vault-bkr-refuse").unwrap();
    let err = s
        .restore_encrypted_vault(
            &empty.display().to_string(),
            &base.join("vault").display().to_string(),
            "x",
        )
        .expect_err("not a backup");
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "{err:?}"
    );
}

//! Opens the vault a command works on.
//!
//! A folder holding an encrypted vault (its `vault_header.json`) opens
//! through Core with the passphrase in `MEDSCALE_PASSPHRASE`, or else a
//! recovery code in `MEDSCALE_RECOVERY_CODE`. Any other folder opens as a
//! synthetic vault, as before. An encrypted vault is sealed and closed when
//! the session is dropped, so a command never leaves it unsealed.

use std::ops::{Deref, DerefMut};
use std::path::Path;

use medscale_contracts::envelopes::AuthorityError;
use medscale_core::CliSession;

pub const PASSPHRASE_ENV: &str = "MEDSCALE_PASSPHRASE";
pub const RECOVERY_CODE_ENV: &str = "MEDSCALE_RECOVERY_CODE";

pub struct VaultSession {
    session: CliSession,
    encrypted: bool,
}

impl Deref for VaultSession {
    type Target = CliSession;
    fn deref(&self) -> &CliSession {
        &self.session
    }
}

impl DerefMut for VaultSession {
    fn deref_mut(&mut self) -> &mut CliSession {
        &mut self.session
    }
}

impl Drop for VaultSession {
    fn drop(&mut self) {
        if !self.encrypted {
            return;
        }
        if let Err(err) = self.session.close_encrypted_vault() {
            eprintln!(
                "warning: the encrypted vault could not be sealed ({err:?}); the next open recovers what was committed"
            );
        }
    }
}

/// True when `vault_root` holds an encrypted vault.
#[must_use]
pub fn is_encrypted(vault_root: &Path) -> bool {
    vault_root.join("vault_header.json").is_file()
}

/// Connects and opens the vault at `vault_root`: encrypted when the folder
/// holds one, synthetic otherwise.
pub fn open(vault_id: &str, vault_root: &Path) -> Result<VaultSession, AuthorityError> {
    open_configured(vault_id, vault_root, |_| {})
}

/// [`open`], with `configure` applied to the session before the vault opens.
pub fn open_configured(
    vault_id: &str,
    vault_root: &Path,
    configure: impl FnOnce(&mut CliSession),
) -> Result<VaultSession, AuthorityError> {
    let from_env = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    open_with_secrets(
        vault_id,
        vault_root,
        from_env(PASSPHRASE_ENV),
        from_env(RECOVERY_CODE_ENV),
        configure,
    )
}

/// The passphrase wins over a recovery code when both are given. Neither is
/// needed for a synthetic vault.
fn open_with_secrets(
    vault_id: &str,
    vault_root: &Path,
    passphrase: Option<String>,
    recovery_code: Option<String>,
    configure: impl FnOnce(&mut CliSession),
) -> Result<VaultSession, AuthorityError> {
    let mut session = CliSession::connect(vault_id)?;
    configure(&mut session);
    let root = vault_root.display().to_string();
    if !is_encrypted(vault_root) {
        session.open_synthetic_vault(&root)?;
        return Ok(VaultSession {
            session,
            encrypted: false,
        });
    }
    match (passphrase, recovery_code) {
        (Some(passphrase), _) => session.open_encrypted_vault(&root, &passphrase)?,
        (None, Some(code)) => session.open_encrypted_vault_with_recovery_code(&root, &code)?,
        (None, None) => {
            return Err(AuthorityError::InvalidArgument {
                message: format!(
                    "this vault is encrypted: set {PASSPHRASE_ENV}, or {RECOVERY_CODE_ENV} with a recovery code"
                ),
            });
        }
    }
    Ok(VaultSession {
        session,
        encrypted: true,
    })
}

/// [`open`] when a vault folder is given; otherwise only connects, for a
/// persistent Core Host that already has the vault open.
pub fn open_or_connect(
    vault_id: &str,
    vault_root: Option<&Path>,
) -> Result<VaultSession, AuthorityError> {
    match vault_root {
        Some(root) => open(vault_id, root),
        None => Ok(VaultSession {
            session: CliSession::connect(vault_id)?,
            encrypted: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("medscale-cli-vs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn create_encrypted(vault_id: &str, root: &Path) -> Vec<String> {
        let mut s = CliSession::connect(vault_id).unwrap();
        let codes = s
            .create_encrypted_vault(&root.display().to_string(), "correct horse battery")
            .unwrap();
        s.close_encrypted_vault().unwrap();
        codes
    }

    #[test]
    fn data_commands_work_in_an_encrypted_vault_and_seal_it() {
        let root = temp("enc");
        let codes = create_encrypted("vs-enc", &root);
        {
            let mut v = open_with_secrets(
                "vs-enc",
                &root,
                Some("correct horse battery".to_owned()),
                None,
                |_| {},
            )
            .unwrap();
            v.project_create("study".to_owned(), None).unwrap();
        }
        assert!(
            !root.join("meta.work.sqlite3").exists(),
            "the vault is sealed when the session ends"
        );
        let mut by_code =
            open_with_secrets("vs-enc", &root, None, Some(codes[0].clone()), |_| {}).unwrap();
        let (projects, _) = by_code.project_list(None, None, None).unwrap();
        assert_eq!(projects.len(), 1, "the project is in the sealed vault");
        drop(by_code);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_encrypted_vault_without_a_secret_is_refused() {
        let root = temp("nosecret");
        create_encrypted("vs-nosecret", &root);
        let err = open_with_secrets("vs-nosecret", &root, None, None, |_| {})
            .err()
            .expect("refused");
        assert!(
            matches!(err, AuthorityError::InvalidArgument { ref message } if message.contains(PASSPHRASE_ENV)),
            "{err:?}"
        );
        let wrong = open_with_secrets(
            "vs-nosecret",
            &root,
            Some("not the passphrase".to_owned()),
            None,
            |_| {},
        );
        assert!(wrong.is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn any_other_folder_still_opens_as_a_synthetic_vault() {
        let root = temp("synthetic");
        std::fs::create_dir_all(&root).unwrap();
        let v = open_with_secrets("vs-synthetic", &root, None, None, |_| {}).unwrap();
        assert!(!v.encrypted);
        drop(v);
        let _ = std::fs::remove_dir_all(&root);
    }
}

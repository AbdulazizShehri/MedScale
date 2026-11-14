//! Spec 087 Community Extensions commands (CLI vertical slice through Core).
//!
//! Lifecycle and invocation commands dispatch typed Core requests via
//! `CliSession`. `keygen` and `pack` are the Extension SDK: they create a
//! publisher key and build a signed pack from a manifest file, locally,
//! without touching a vault. Packs are read from files by this host and
//! sent to Core as bytes; no request carries a path. No extension code
//! exists or runs.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::extensions::{ExtensionCapability, ExtensionManifest, PACK_BYTES_MAX};
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_core::authority::{generate_publisher_key, sign_extension_pack};

use super::{fail_json, print_json_or_debug};

fn ext_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => ("denied", debug),
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::LeaseRequired | AuthorityError::VaultRequired => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn open_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| ext_fail(&err, json))
}

fn read_pack(path: &std::path::Path, json: bool) -> anyhow::Result<String> {
    let meta = std::fs::metadata(path).map_err(|e| fail_json("invalid", e.to_string(), json))?;
    if meta.len() > PACK_BYTES_MAX {
        return Err(fail_json(
            "invalid",
            "pack file is too large".to_owned(),
            json,
        ));
    }
    std::fs::read_to_string(path).map_err(|e| fail_json("invalid", e.to_string(), json))
}

fn parse_digest(hex: &str) -> Result<DigestSha256, String> {
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected 64 hex digits".to_owned());
    }
    let mut out = [0_u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(DigestSha256::from_bytes(out))
}

#[derive(Debug, Subcommand)]
pub enum ExtensionCmd {
    /// SDK: create a publisher key pair (secret written to a new file).
    Keygen {
        #[arg(long)]
        secret_out: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// SDK: build a signed pack from a manifest JSON file.
    Pack {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        secret: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Trust a publisher key in this vault.
    Trust {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        publisher_id: String,
        #[arg(long)]
        key_hex: String,
        #[arg(long)]
        json: bool,
    },
    /// Install or upgrade a pack in a Project.
    Install {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        pack: PathBuf,
        #[arg(long)]
        upgrade: bool,
        #[arg(long)]
        json: bool,
    },
    /// rollback | enable | disable | uninstall
    Set {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        extension_id: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        json: bool,
    },
    /// Grant a declared capability up to a data-class ceiling, or revoke it.
    Grant {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        extension_id: String,
        #[arg(long)]
        capability: String,
        /// local_phi | team_protected | external_deidentified | public
        #[arg(long)]
        ceiling: Option<String>,
        #[arg(long)]
        revoke: bool,
        #[arg(long)]
        json: bool,
    },
    /// Run one declarative command.
    Invoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        extension_id: String,
        #[arg(long)]
        command: String,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Revoke a publisher or one release; affected installs are quarantined.
    Revoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        publisher_id: Option<String>,
        #[arg(long)]
        release_sha256: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Installs, grants and receipts of a Project.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_extension(cmd: ExtensionCmd) -> anyhow::Result<()> {
    match cmd {
        ExtensionCmd::Keygen { secret_out, json } => {
            let (secret, public) = generate_publisher_key();
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&secret_out)
                .and_then(|mut f| std::io::Write::write_all(&mut f, secret.as_bytes()))
                .map_err(|e| fail_json("invalid", e.to_string(), json))?;
            if json {
                return print_json_or_debug(&serde_json::json!({ "public_key_hex": public }), true);
            }
            println!("public_key_hex: {public}");
            Ok(())
        }
        ExtensionCmd::Pack {
            manifest,
            secret,
            out,
            json,
        } => {
            let raw = std::fs::read_to_string(&manifest)
                .map_err(|e| fail_json("invalid", e.to_string(), json))?;
            let m: ExtensionManifest = serde_json::from_str(&raw)
                .map_err(|e| fail_json("invalid", e.to_string(), json))?;
            let secret = std::fs::read_to_string(&secret)
                .map_err(|e| fail_json("invalid", e.to_string(), json))?;
            let pack = sign_extension_pack(&m, secret.trim())
                .map_err(|e| fail_json("invalid", e, json))?;
            let bytes = serde_json::to_vec(&pack)
                .map_err(|e| fail_json("internal", e.to_string(), json))?;
            std::fs::write(&out, &bytes).map_err(|e| fail_json("invalid", e.to_string(), json))?;
            let digest = DigestSha256::of(pack.manifest_json.as_bytes());
            if json {
                return print_json_or_debug(
                    &serde_json::json!({ "release_sha256": digest.to_hex() }),
                    true,
                );
            }
            println!("release_sha256: {}", digest.to_hex());
            Ok(())
        }
        ExtensionCmd::Trust {
            vault_id,
            vault_root,
            publisher_id,
            key_hex,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let p = s
                .ext_trust_publisher(publisher_id, key_hex)
                .map_err(|err| ext_fail(&err, json))?;
            print_json_or_debug(&p, json)
        }
        ExtensionCmd::Install {
            vault_id,
            vault_root,
            project_id,
            pack,
            upgrade,
            json,
        } => {
            let pack_json = read_pack(&pack, json)?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let project = OpaqueId::new(project_id);
            let r = if upgrade {
                s.ext_upgrade(project, pack_json)
            } else {
                s.ext_install(project, pack_json)
            }
            .map_err(|err| ext_fail(&err, json))?;
            print_json_or_debug(&r, json)
        }
        ExtensionCmd::Set {
            vault_id,
            vault_root,
            project_id,
            extension_id,
            action,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let project = OpaqueId::new(project_id);
            let r = match action.as_str() {
                "rollback" => s.ext_rollback(project, extension_id),
                "enable" => s.ext_set_enabled(project, extension_id, true),
                "disable" => s.ext_set_enabled(project, extension_id, false),
                "uninstall" => s.ext_uninstall(project, extension_id),
                other => {
                    return Err(fail_json(
                        "invalid",
                        format!("unknown action {other}"),
                        json,
                    ));
                }
            }
            .map_err(|err| ext_fail(&err, json))?;
            print_json_or_debug(&r, json)
        }
        ExtensionCmd::Grant {
            vault_id,
            vault_root,
            project_id,
            extension_id,
            capability,
            ceiling,
            revoke,
            json,
        } => {
            let capability = ExtensionCapability::parse(&capability)
                .map_err(|e| fail_json("invalid", e, json))?;
            let ceiling = match (revoke, ceiling.as_deref()) {
                (true, None) => None,
                (false, Some(c)) => {
                    Some(DataClass::parse(c).map_err(|e| fail_json("invalid", e, json))?)
                }
                _ => {
                    return Err(fail_json(
                        "invalid",
                        "give --ceiling to grant, or --revoke alone".to_owned(),
                        json,
                    ));
                }
            };
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let r = s
                .ext_grant(OpaqueId::new(project_id), extension_id, capability, ceiling)
                .map_err(|err| ext_fail(&err, json))?;
            print_json_or_debug(&r, json)
        }
        ExtensionCmd::Invoke {
            vault_id,
            vault_root,
            project_id,
            extension_id,
            command,
            target,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let r = s
                .ext_invoke(
                    OpaqueId::new(project_id),
                    extension_id,
                    command,
                    target.map(OpaqueId::new),
                )
                .map_err(|err| ext_fail(&err, json))?;
            print_json_or_debug(&r, json)
        }
        ExtensionCmd::Revoke {
            vault_id,
            vault_root,
            publisher_id,
            release_sha256,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let receipts = match (publisher_id, release_sha256) {
                (Some(p), None) => s.ext_revoke_publisher(p),
                (None, Some(d)) => {
                    let digest = parse_digest(&d).map_err(|e| fail_json("invalid", e, json))?;
                    s.ext_revoke_release(digest)
                }
                _ => {
                    return Err(fail_json(
                        "invalid",
                        "give exactly one of --publisher-id and --release-sha256".to_owned(),
                        json,
                    ));
                }
            }
            .map_err(|err| ext_fail(&err, json))?;
            print_json_or_debug(&receipts, json)
        }
        ExtensionCmd::List {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let v = s
                .ext_list(OpaqueId::new(project_id))
                .map_err(|err| ext_fail(&err, json))?;
            print_json_or_debug(&v, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_parse_strictly() {
        let d = DigestSha256::of(b"x");
        assert_eq!(parse_digest(&d.to_hex()).unwrap(), d);
        assert!(parse_digest("12").is_err());
        assert!(parse_digest(&"z".repeat(64)).is_err());
    }
}

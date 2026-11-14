//! Spec 089 Research Pack commands (CLI slice through Core).
//!
//! `act` takes one typed Pack act as JSON (the closed `PackActRequest`
//! vocabulary; unknown acts or fields are refused) and dispatches it
//! through Core, where schemas and workflows are enforced. `catalog`,
//! `show` and `list` are reads.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::research_packs::PackActRequest;

use super::{fail_json, print_json_or_debug};

fn pack_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
    crate::vault_session::open(vault_id, vault_root).map_err(|err| pack_fail(&err, json))
}

/// Parses one act strictly.
fn parse_act(raw: &str) -> Result<PackActRequest, String> {
    serde_json::from_str(raw).map_err(|e| e.to_string())
}

#[derive(Debug, Subcommand)]
pub enum PackCmd {
    /// Run one Pack act given as JSON, e.g.
    /// `{"act":"install","project_id":"project-1","pack_id":"medscale.clinical-research"}`.
    Act {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        request: String,
        #[arg(long)]
        json: bool,
    },
    /// Shipped Research Packs.
    Catalog {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        artifact_id: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        pack_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_pack(cmd: PackCmd) -> anyhow::Result<()> {
    match cmd {
        PackCmd::Act {
            vault_id,
            vault_root,
            request,
            json,
        } => {
            let act = parse_act(&request).map_err(|e| fail_json("invalid", e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let result = s.pack_act(act).map_err(|err| pack_fail(&err, json))?;
            print_json_or_debug(&result, json)
        }
        PackCmd::Catalog {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let packs = s.pack_catalog().map_err(|err| pack_fail(&err, json))?;
            print_json_or_debug(&packs, json)
        }
        PackCmd::Show {
            vault_id,
            vault_root,
            artifact_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let artifact = s
                .pack_artifact_get(OpaqueId::new(artifact_id))
                .map_err(|err| pack_fail(&err, json))?;
            print_json_or_debug(&artifact, json)
        }
        PackCmd::List {
            vault_id,
            vault_root,
            project_id,
            pack_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let list = s
                .pack_artifact_list(OpaqueId::new(project_id), pack_id)
                .map_err(|err| pack_fail(&err, json))?;
            print_json_or_debug(&list, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acts_parse_strictly() {
        assert!(
            parse_act(
                r#"{"act":"install","project_id":"p","pack_id":"medscale.clinical-research"}"#
            )
            .is_ok()
        );
        assert!(parse_act(r#"{"act":"run_script","project_id":"p","pack_id":"x"}"#).is_err());
        assert!(
            parse_act(r#"{"act":"install","project_id":"p","pack_id":"x","grant":"all"}"#).is_err()
        );
    }
}

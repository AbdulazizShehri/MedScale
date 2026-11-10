//! Spec 091 federation commands (CLI slice through Core).
//!
//! `act` takes one typed federation act as JSON (the closed
//! `FederationActRequest` vocabulary). Bundles are returned to the caller
//! and moved between institutions out of band; there is no network path.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::federation::FederationActRequest;

use super::{fail_json, print_json_or_debug};

fn fed_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
    crate::vault_session::open(vault_id, vault_root).map_err(|err| fed_fail(&err, json))
}

/// Parses one act strictly.
fn parse_act(raw: &str) -> Result<FederationActRequest, String> {
    serde_json::from_str(raw).map_err(|e| e.to_string())
}

#[derive(Debug, Subcommand)]
pub enum FederationCmd {
    /// Run one federation act given as JSON, e.g.
    /// `{"act":"create_identity","institution_id":"hospital-a"}`.
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
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_federation(cmd: FederationCmd) -> anyhow::Result<()> {
    match cmd {
        FederationCmd::Act {
            vault_id,
            vault_root,
            request,
            json,
        } => {
            let act = parse_act(&request).map_err(|e| fail_json("invalid", e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let result = s.federation_act(act).map_err(|err| fed_fail(&err, json))?;
            print_json_or_debug(&result, json)
        }
        FederationCmd::Show {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s.federation_get().map_err(|err| fed_fail(&err, json))?;
            print_json_or_debug(&view, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acts_parse_strictly() {
        assert!(parse_act(r#"{"act":"create_identity","institution_id":"hospital-a"}"#).is_ok());
        assert!(parse_act(r#"{"act":"sync_all","institution_id":"x"}"#).is_err());
        assert!(parse_act(r#"{"act":"revoke_peer","institution_id":"x","cascade":true}"#).is_err());
    }
}

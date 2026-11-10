//! Spec 090 institutional adapter commands (CLI slice through Core).
//!
//! `act` takes one typed adapter act as JSON (the closed
//! `AdapterActRequest` vocabulary). The product build has no institutional
//! transport: sends report `unreachable` and nothing leaves the machine.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::institutional::AdapterActRequest;
use medscale_contracts::objects::OpaqueId;

use super::{fail_json, print_json_or_debug};

fn adapter_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
    crate::vault_session::open(vault_id, vault_root).map_err(|err| adapter_fail(&err, json))
}

/// Parses one act strictly.
fn parse_act(raw: &str) -> Result<AdapterActRequest, String> {
    serde_json::from_str(raw).map_err(|e| e.to_string())
}

#[derive(Debug, Subcommand)]
pub enum AdapterCmd {
    /// Run one adapter act given as JSON, e.g.
    /// `{"act":"send","intent_id":"write-intent-1"}`.
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
        adapter_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_adapter(cmd: AdapterCmd) -> anyhow::Result<()> {
    match cmd {
        AdapterCmd::Act {
            vault_id,
            vault_root,
            request,
            json,
        } => {
            let act = parse_act(&request).map_err(|e| fail_json("invalid", e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let result = s.adapter_act(act).map_err(|err| adapter_fail(&err, json))?;
            print_json_or_debug(&result, json)
        }
        AdapterCmd::Show {
            vault_id,
            vault_root,
            adapter_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .adapter_get(OpaqueId::new(adapter_id))
                .map_err(|err| adapter_fail(&err, json))?;
            print_json_or_debug(&view, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acts_parse_strictly() {
        assert!(parse_act(r#"{"act":"send","intent_id":"write-intent-1"}"#).is_ok());
        assert!(parse_act(r#"{"act":"send","intent_id":"i","force":true}"#).is_err());
        assert!(parse_act(r#"{"act":"delete_remote","intent_id":"i"}"#).is_err());
    }
}

//! Spec 088 AudioFlow Advanced huddle commands (CLI slice through Core).
//!
//! `act` takes one typed huddle act as JSON (the closed
//! `HuddleActRequest` vocabulary; unknown acts or fields are refused) and
//! dispatches it through Core, where every consent check happens. `show`
//! and `list` are reads.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::huddles::HuddleActRequest;
use medscale_contracts::objects::OpaqueId;

use super::{fail_json, print_json_or_debug};

fn huddle_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
    crate::vault_session::open(vault_id, vault_root).map_err(|err| huddle_fail(&err, json))
}

/// Parses one act strictly.
fn parse_act(raw: &str) -> Result<HuddleActRequest, String> {
    serde_json::from_str(raw).map_err(|e| e.to_string())
}

#[derive(Debug, Subcommand)]
pub enum HuddleCmd {
    /// Run one huddle act given as JSON, e.g.
    /// `{"act":"consent","huddle_id":"huddle-1","participant_id":"huddle-participant-1","consent_act":"record","value":true}`.
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
        huddle_id: String,
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
        json: bool,
    },
}

pub fn run_huddle(cmd: HuddleCmd) -> anyhow::Result<()> {
    match cmd {
        HuddleCmd::Act {
            vault_id,
            vault_root,
            request,
            json,
        } => {
            let act = parse_act(&request).map_err(|e| fail_json("invalid", e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let result = s.huddle_act(act).map_err(|err| huddle_fail(&err, json))?;
            print_json_or_debug(&result, json)
        }
        HuddleCmd::Show {
            vault_id,
            vault_root,
            huddle_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .huddle_get(OpaqueId::new(huddle_id))
                .map_err(|err| huddle_fail(&err, json))?;
            print_json_or_debug(&view, json)
        }
        HuddleCmd::List {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let list = s
                .huddle_list(OpaqueId::new(project_id))
                .map_err(|err| huddle_fail(&err, json))?;
            print_json_or_debug(&list, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acts_parse_strictly() {
        assert!(parse_act(r#"{"act":"end","huddle_id":"huddle-1"}"#).is_ok());
        assert!(parse_act(r#"{"act":"clone_voice","huddle_id":"huddle-1"}"#).is_err());
        assert!(parse_act(r#"{"act":"end","huddle_id":"huddle-1","force":true}"#).is_err());
        assert!(
            parse_act(
                r#"{"act":"consent","huddle_id":"h","participant_id":"p","consent_act":"record_all","value":true}"#
            )
            .is_err()
        );
    }
}

//! Spec 080 Governed Browse commands (CLI vertical slice through Core).
//!
//! Every command dispatches one typed Core request via `CliSession` and
//! renders the typed result as human lines or stable JSON. The CLI holds no
//! network code: requests leave only through Core's Browse policy and the
//! network crate's public-only transport. `--offline-fixture` swaps that
//! transport for a synthetic page (demos and tests), with the same policy.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::browse::{
    BrowseAllowlistEntry, BrowseIntentKind, BrowseRequest, BrowseSession, BrowseSessionView,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;

use super::{fail_json, print_json_or_debug};

fn browse_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
    offline_fixture: bool,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open_configured(vault_id, vault_root, |session| {
        if offline_fixture {
            session.use_offline_browse_fixture();
        }
    })
    .map_err(|err| browse_fail(&err, json))
}

fn print_entry(e: &BrowseAllowlistEntry, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(e, true);
    }
    println!(
        "{}\t{}{}\t{}\trev {}",
        e.header.id.as_str(),
        e.host,
        e.path_prefix,
        if e.enabled { "enabled" } else { "disabled" },
        e.revision
    );
    Ok(())
}

fn print_view(v: &BrowseSessionView, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(v, true);
    }
    let s = &v.session;
    println!("session_id: {}", s.header.id.as_str());
    println!("route: {}", s.route.as_str());
    println!("state: {}", s.state.as_str());
    println!("decision: {}", s.decision.outcome.as_str());
    if let Some(reason) = s.decision.reason {
        println!("reason: {}", reason.as_str());
    }
    for step in &s.steps {
        println!(
            "step {}: {} status={} {}{}",
            step.seq,
            step.url.escape_debug(),
            step.http_status
                .map_or_else(|| "-".to_owned(), |c| c.to_string()),
            step.decision.outcome.as_str(),
            step.redirect_to
                .as_deref()
                .map(|r| format!(" -> {}", r.escape_debug()))
                .unwrap_or_default()
        );
    }
    if let Some(t) = &s.takeover {
        println!(
            "human_takeover: {} {}",
            t.reason.as_str(),
            t.url.escape_debug()
        );
    }
    for e in &v.evidence {
        println!(
            "evidence: {} {} bytes={} instruction_like_flagged={}",
            e.header.id.as_str(),
            e.content_type,
            e.byte_length,
            e.instruction_like_content_flagged
        );
        println!("excerpt: {}", e.excerpt.escape_debug());
    }
    for d in &v.downloads {
        println!(
            "download: {} {} bytes={} {}",
            d.header.id.as_str(),
            d.content_type,
            d.byte_length,
            d.status.as_str()
        );
    }
    println!("receipt_id: {}", v.receipt.header.id.as_str());
    Ok(())
}

fn print_sessions(sessions: &[BrowseSession], json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(&sessions, true);
    }
    for s in sessions {
        println!(
            "{}\t{}\t{}\t{}",
            s.header.id.as_str(),
            s.route.as_str(),
            s.state.as_str(),
            s.decision.reason.map_or("-", |r| r.as_str())
        );
    }
    Ok(())
}

/// Spec 080 Governed Browse commands.
#[derive(Debug, Subcommand)]
pub enum BrowseCmd {
    /// Allow one host + path prefix for this Project (https only).
    AllowAdd {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        host: String,
        #[arg(long, default_value = "/")]
        path_prefix: String,
        #[arg(long)]
        json: bool,
    },
    /// List the Project's Browse allowlist.
    AllowList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Disable one allowlist entry.
    AllowDisable {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        entry_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Show which Browse routes this build can run.
    Routes {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Fetch one https URL under the full Browse policy.
    Fetch {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        url: String,
        /// Use the synthetic offline page instead of the network.
        #[arg(long)]
        offline_fixture: bool,
        #[arg(long)]
        json: bool,
    },
    /// Request a search (reports the route as unavailable in this build).
    Search {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        query: String,
        /// Project artifact whose content the query derives from.
        #[arg(long)]
        context_artifact_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one session with its evidence, downloads and receipt.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List the Project's Browse sessions.
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
    /// Cancel a session awaiting human takeover.
    Cancel {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_browse(cmd: BrowseCmd) -> anyhow::Result<()> {
    match cmd {
        BrowseCmd::AllowAdd {
            vault_id,
            vault_root,
            project_id,
            host,
            path_prefix,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let e = s
                .browse_allowlist_add(OpaqueId::new(project_id), host, path_prefix)
                .map_err(|err| browse_fail(&err, json))?;
            print_entry(&e, json)
        }
        BrowseCmd::AllowList {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let entries = s
                .browse_allowlist_list(OpaqueId::new(project_id))
                .map_err(|err| browse_fail(&err, json))?;
            if json {
                return print_json_or_debug(&entries, true);
            }
            for e in &entries {
                print_entry(e, false)?;
            }
            Ok(())
        }
        BrowseCmd::AllowDisable {
            vault_id,
            vault_root,
            entry_id,
            expected_revision,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let e = s
                .browse_allowlist_disable(OpaqueId::new(entry_id), expected_revision)
                .map_err(|err| browse_fail(&err, json))?;
            print_entry(&e, json)
        }
        BrowseCmd::Routes {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let routes = s.browse_routes().map_err(|err| browse_fail(&err, json))?;
            if json {
                return print_json_or_debug(&routes, true);
            }
            for r in &routes {
                println!(
                    "{}\t{}\t{}",
                    r.route.as_str(),
                    if r.available {
                        "available"
                    } else {
                        "unavailable"
                    },
                    r.reason.as_deref().unwrap_or("-")
                );
            }
            Ok(())
        }
        BrowseCmd::Fetch {
            vault_id,
            vault_root,
            project_id,
            url,
            offline_fixture,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, offline_fixture, json)?;
            let v = s
                .browse_run(BrowseRequest {
                    project_id: OpaqueId::new(project_id),
                    intent: BrowseIntentKind::FetchUrl,
                    url: Some(url),
                    query: None,
                    context_artifact_id: None,
                })
                .map_err(|err| browse_fail(&err, json))?;
            print_view(&v, json)
        }
        BrowseCmd::Search {
            vault_id,
            vault_root,
            project_id,
            query,
            context_artifact_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let v = s
                .browse_run(BrowseRequest {
                    project_id: OpaqueId::new(project_id),
                    intent: BrowseIntentKind::Search,
                    url: None,
                    query: Some(query),
                    context_artifact_id: context_artifact_id.map(OpaqueId::new),
                })
                .map_err(|err| browse_fail(&err, json))?;
            print_view(&v, json)
        }
        BrowseCmd::Show {
            vault_id,
            vault_root,
            session_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let v = s
                .browse_session_get(OpaqueId::new(session_id))
                .map_err(|err| browse_fail(&err, json))?;
            print_view(&v, json)
        }
        BrowseCmd::List {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let sessions = s
                .browse_session_list(OpaqueId::new(project_id))
                .map_err(|err| browse_fail(&err, json))?;
            print_sessions(&sessions, json)
        }
        BrowseCmd::Cancel {
            vault_id,
            vault_root,
            session_id,
            expected_revision,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, false, json)?;
            let v = s
                .browse_session_cancel(OpaqueId::new(session_id), expected_revision)
                .map_err(|err| browse_fail(&err, json))?;
            print_view(&v, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::browse::{BrowseDenyReason, BrowseSessionState};
    use medscale_core::CliSession;

    const VAULT: &str = "vault-080-cli";

    #[test]
    fn browse_commands_run_through_core_across_fresh_sessions() {
        let root = std::env::temp_dir().join(format!("medscale-080-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let project = session
            .project_create("browse".to_owned(), None)
            .unwrap()
            .header
            .id;
        drop(session);
        let pid = project.as_str().to_owned();

        // Denied before any request: empty allowlist, then a bad host.
        run_browse(BrowseCmd::Fetch {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            project_id: pid.clone(),
            url: CliSession::OFFLINE_BROWSE_FIXTURE_URL.to_owned(),
            offline_fixture: true,
            json: true,
        })
        .expect("denied fetch still returns a session");
        assert!(
            run_browse(BrowseCmd::AllowAdd {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                host: "192.168.0.1".to_owned(),
                path_prefix: "/".to_owned(),
                json: true,
            })
            .is_err()
        );
        for json in [true, false] {
            run_browse(BrowseCmd::AllowAdd {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                host: "fixture.medscale.test".to_owned(),
                path_prefix: if json { "/" } else { "/guideline" }.to_owned(),
                json,
            })
            .expect("allow add");
        }
        for json in [true, false] {
            run_browse(BrowseCmd::Fetch {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                url: CliSession::OFFLINE_BROWSE_FIXTURE_URL.to_owned(),
                offline_fixture: true,
                json,
            })
            .expect("fixture fetch");
            run_browse(BrowseCmd::Search {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                query: "statin myopathy".to_owned(),
                context_artifact_id: None,
                json,
            })
            .expect("search reports the route");
            for cmd in [
                BrowseCmd::Routes {
                    vault_id: VAULT.to_owned(),
                    vault_root: root.clone(),
                    json,
                },
                BrowseCmd::AllowList {
                    vault_id: VAULT.to_owned(),
                    vault_root: root.clone(),
                    project_id: pid.clone(),
                    json,
                },
                BrowseCmd::List {
                    vault_id: VAULT.to_owned(),
                    vault_root: root.clone(),
                    project_id: pid.clone(),
                    json,
                },
            ] {
                run_browse(cmd).expect("read command");
            }
        }

        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let sessions = session.browse_session_list(project.clone()).unwrap();
        assert_eq!(sessions.len(), 5);
        assert_eq!(
            sessions[0].decision.reason,
            Some(BrowseDenyReason::EmptyAllowlist)
        );
        assert_eq!(sessions[1].state, BrowseSessionState::Completed);
        assert_eq!(
            sessions[2].decision.reason,
            Some(BrowseDenyReason::RouteUnavailable)
        );
        let view = session
            .browse_session_get(sessions[1].header.id.clone())
            .unwrap();
        assert_eq!(view.evidence.len(), 1);
        assert!(view.evidence[0].instruction_like_content_flagged);
        drop(session);
        run_browse(BrowseCmd::Show {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            session_id: sessions[1].header.id.as_str().to_owned(),
            json: false,
        })
        .expect("show");
        assert!(
            run_browse(BrowseCmd::Cancel {
                vault_id: VAULT.to_owned(),
                vault_root: root,
                session_id: sessions[1].header.id.as_str().to_owned(),
                expected_revision: 1,
                json: true,
            })
            .is_err(),
            "a completed session is final"
        );
    }
}

//! Spec 077 MedAgent Workbench commands (CLI vertical slice through Core).
//!
//! T077-03/T077-04/T077-05 scope: `AgentIdentity` + `AgentCapabilityManifest`
//! register/show/list/revoke, `ContextManifest` create/show, and `AgentRun`
//! create/show/list/start/cancel/turns. Every command opens the session
//! scope, dispatches one typed Core request via `CliSession`, and renders
//! the typed result as human lines or stable JSON. The CLI never writes
//! medagent storage directly.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::medagent::{
    AgentCapabilityManifest, AgentIdentity, AgentProposal, AgentRun, AgentTurn, ContextManifest,
    RunReceipt, ToolInvocation, ToolKind, ToolReceipt,
};
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactVersionBinding, ReferenceResolution,
};

use super::{fail_json, print_json_or_debug};

fn medagent_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
        AuthorityError::UnsupportedSchema { message } => ("unsupported_schema", message.clone()),
        AuthorityError::LeaseRequired
        | AuthorityError::VaultRequired
        | AuthorityError::MissingKeyMaterial => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

fn open_medagent_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| medagent_fail(&err, json))
}

fn parse_tool_kinds(value: &str) -> Result<Vec<ToolKind>, String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToolKind::parse)
        .collect()
}

/// Parses one `object_id:kind` artifact spec into an `IdentityOnly`-bound
/// descriptor (CLI scope simplification -- mirrors `collaboration.rs`'s own
/// `simple_anchor` precedent; a full `ArtifactVersionBinding` is a Core
/// capability, not a CLI-operator-surface requirement).
fn parse_artifact_spec(spec: &str) -> Result<ArtifactDescriptor, String> {
    let (object_id, kind) = spec
        .split_once(':')
        .ok_or_else(|| format!("expected object_id:kind, got {spec}"))?;
    let kind = crate::project::parse_kind(kind)?;
    Ok(ArtifactDescriptor {
        object_id: OpaqueId::new(object_id),
        kind,
        binding: ArtifactVersionBinding::IdentityOnly,
    })
}

fn print_run_human(run: &AgentRun) {
    println!("run_id: {}", run.header.id.as_str());
    println!("project_id: {}", run.project_id.as_str());
    println!("agent_identity_id: {}", run.agent_identity_id.as_str());
    println!("context_manifest_id: {}", run.context_manifest_id.as_str());
    println!("status: {}", run.status.as_str());
    println!("revision: {}", run.revision);
    println!("prompt: {}", run.prompt);
}

fn print_run_receipt_human(receipt: &RunReceipt) {
    println!("run_id: {}", receipt.run_id.as_str());
    println!("final_state: {}", receipt.final_state.as_str());
    println!("pack_id: {}", receipt.pack_id.as_str());
    println!("pack_version: {}", receipt.pack_version);
    if let Some(reason) = &receipt.failure_reason {
        println!("failure_reason: {reason}");
    }
}

fn print_turn_human(turn: &AgentTurn) {
    println!(
        "seq={} kind={} payload={}",
        turn.seq,
        turn.kind.as_str(),
        turn.payload
    );
}

fn print_tool_invocation_human(invocation: &ToolInvocation, receipt: Option<&ToolReceipt>) {
    println!("invocation_id: {}", invocation.header.id.as_str());
    println!("kind: {}", invocation.kind.as_str());
    println!("status: {}", invocation.status.as_str());
    if let Some(reason) = &invocation.refusal_reason {
        println!("refusal_reason: {reason}");
    }
    if let Some(receipt) = receipt {
        println!("result: {}", receipt.result);
    }
}

fn print_run_executed_human(turn: &AgentTurn, proposal: &AgentProposal) {
    print_turn_human(turn);
    println!("agent_proposal_id: {}", proposal.header.id.as_str());
    println!("proposal_id: {}", proposal.proposal_id.as_str());
}

fn print_context_human(manifest: &ContextManifest, resolutions: &[ReferenceResolution]) {
    println!("context_id: {}", manifest.header.id.as_str());
    println!("project_id: {}", manifest.project_id.as_str());
    println!("revision: {}", manifest.revision);
    for (artifact, resolution) in manifest.selected_artifacts.iter().zip(resolutions) {
        println!(
            "artifact: {} kind={:?} resolution={resolution:?}",
            artifact.object_id.as_str(),
            artifact.kind
        );
    }
}

fn print_identity_human(identity: &AgentIdentity, capabilities: &AgentCapabilityManifest) {
    println!("agent_id: {}", identity.header.id.as_str());
    println!("project_id: {}", identity.project_id.as_str());
    println!("pack_id: {}", identity.pack_id.as_str());
    println!("pack_version: {}", identity.pack_version);
    println!("display_name: {}", identity.display_name);
    println!("status: {}", identity.status.as_str());
    println!("revision: {}", identity.revision);
    let kinds: Vec<&str> = capabilities
        .granted_tool_kinds
        .iter()
        .map(|k| k.as_str())
        .collect();
    println!("granted_tool_kinds: {}", kinds.join(","));
}

#[derive(Debug, serde::Serialize)]
struct IdentityJson {
    identity: AgentIdentity,
    capabilities: AgentCapabilityManifest,
}

#[derive(Debug, serde::Serialize)]
struct ContextJson {
    manifest: ContextManifest,
    resolutions: Vec<ReferenceResolution>,
}

#[derive(Debug, serde::Serialize)]
struct RunStartedJson {
    run: AgentRun,
    turn: AgentTurn,
}

#[derive(Debug, serde::Serialize)]
struct RunTerminalJson {
    run: AgentRun,
    receipt: RunReceipt,
}

#[derive(Debug, serde::Serialize)]
struct ToolInvocationJson {
    invocation: ToolInvocation,
    receipt: Option<ToolReceipt>,
}

#[derive(Debug, serde::Serialize)]
struct RunExecutedJson {
    turn: AgentTurn,
    proposal: AgentProposal,
}

fn parse_tool_kind(value: &str) -> Result<ToolKind, String> {
    ToolKind::parse(value)
}

/// Spec 077 MedAgent Workbench commands (agent identity + context manifest
/// + agent run).
#[derive(Debug, Subcommand)]
pub enum MedAgentCmd {
    /// Register a new agent identity bound to an admitted local model Pack.
    IdentityRegister {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        pack_id: String,
        #[arg(long)]
        display_name: String,
        /// Comma-separated tool kinds, e.g. `read_context_artifact,search_context_artifacts`.
        #[arg(long)]
        granted_tool_kinds: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one agent identity.
    IdentityShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        agent_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List agent identities in one Project.
    IdentityList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Revoke an agent identity.
    IdentityRevoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        agent_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Create a ContextManifest from an explicit artifact list.
    ContextCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        /// Repeatable `object_id:kind` artifact spec (IdentityOnly binding).
        #[arg(long = "artifact", required = true)]
        artifacts: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one ContextManifest, with each artifact's live resolution.
    ContextShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        context_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Create a new Pending AgentRun.
    RunCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        agent_id: String,
        #[arg(long)]
        context_id: String,
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one AgentRun.
    RunShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List AgentRuns in one Project, optionally filtered by agent identity.
    RunList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        agent_id: Option<String>,
        /// One of: pending, running, cancelled, completed, failed.
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Start a Pending run (Pending -> Running).
    RunStart {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Cancel a Pending or Running run.
    RunCancel {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Mark a Running run Completed.
    RunComplete {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Mark a Running run Failed with a reason.
    RunFail {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        failure_reason: String,
        #[arg(long)]
        json: bool,
    },
    /// List one run's turns in seq order.
    RunTurns {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Invoke one typed tool against a Running run.
    ToolInvoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        kind: String,
        /// Raw JSON arguments, e.g. `{"object_id":"src-1"}`.
        #[arg(long)]
        arguments: String,
        #[arg(long)]
        json: bool,
    },
    /// Run a Running run's prompt through its bound admitted local model
    /// Pack (zero network) and persist the result as an AgentProposal.
    RunExecute {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        run_id: String,
        /// Local directory of the admitted model Pack (never persisted by
        /// Core; supplied fresh on every call).
        #[arg(long)]
        local_path: String,
        #[arg(long, default_value_t = 512)]
        max_tokens: u32,
        /// Must be explicitly acknowledged: real PHI through this runtime
        /// requires a later, explicit gate this spec does not grant.
        #[arg(long)]
        synthetic_only: bool,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_medagent(action: MedAgentCmd) -> anyhow::Result<()> {
    match action {
        MedAgentCmd::IdentityRegister {
            vault_id,
            vault_root,
            project_id,
            pack_id,
            display_name,
            granted_tool_kinds,
            json,
        } => {
            let granted_tool_kinds =
                parse_tool_kinds(&granted_tool_kinds).map_err(|m| invalid(m, json))?;
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (identity, capabilities) = session
                .medagent_identity_register(
                    OpaqueId::new(project_id),
                    OpaqueId::new(pack_id),
                    display_name,
                    granted_tool_kinds,
                )
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &IdentityJson {
                        identity,
                        capabilities,
                    },
                    true,
                )?;
            } else {
                print_identity_human(&identity, &capabilities);
            }
            Ok(())
        }
        MedAgentCmd::IdentityShow {
            vault_id,
            vault_root,
            agent_id,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (identity, capabilities) = session
                .medagent_identity_get(OpaqueId::new(agent_id))
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &IdentityJson {
                        identity,
                        capabilities,
                    },
                    true,
                )?;
            } else {
                print_identity_human(&identity, &capabilities);
            }
            Ok(())
        }
        MedAgentCmd::IdentityList {
            vault_id,
            vault_root,
            project_id,
            limit,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let identities = session
                .medagent_identity_list(OpaqueId::new(project_id), limit)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&identities, true)?;
            } else {
                for identity in &identities {
                    println!(
                        "{}\t{}\t{}\t{}",
                        identity.header.id.as_str(),
                        identity.display_name,
                        identity.status.as_str(),
                        identity.revision
                    );
                }
            }
            Ok(())
        }
        MedAgentCmd::IdentityRevoke {
            vault_id,
            vault_root,
            agent_id,
            expected_revision,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (identity, capabilities) = session
                .medagent_identity_revoke(OpaqueId::new(agent_id), expected_revision)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &IdentityJson {
                        identity,
                        capabilities,
                    },
                    true,
                )?;
            } else {
                print_identity_human(&identity, &capabilities);
            }
            Ok(())
        }
        MedAgentCmd::ContextCreate {
            vault_id,
            vault_root,
            project_id,
            artifacts,
            json,
        } => {
            let selected_artifacts = artifacts
                .iter()
                .map(|spec| parse_artifact_spec(spec))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|m| invalid(m, json))?;
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (manifest, resolutions) = session
                .medagent_context_create(OpaqueId::new(project_id), selected_artifacts)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &ContextJson {
                        manifest,
                        resolutions,
                    },
                    true,
                )?;
            } else {
                print_context_human(&manifest, &resolutions);
            }
            Ok(())
        }
        MedAgentCmd::ContextShow {
            vault_id,
            vault_root,
            context_id,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (manifest, resolutions) = session
                .medagent_context_get(OpaqueId::new(context_id))
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &ContextJson {
                        manifest,
                        resolutions,
                    },
                    true,
                )?;
            } else {
                print_context_human(&manifest, &resolutions);
            }
            Ok(())
        }
        MedAgentCmd::RunCreate {
            vault_id,
            vault_root,
            project_id,
            agent_id,
            context_id,
            prompt,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let run = session
                .medagent_run_create(
                    OpaqueId::new(project_id),
                    OpaqueId::new(agent_id),
                    OpaqueId::new(context_id),
                    prompt,
                )
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&run, true)?;
            } else {
                print_run_human(&run);
            }
            Ok(())
        }
        MedAgentCmd::RunShow {
            vault_id,
            vault_root,
            run_id,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let run = session
                .medagent_run_get(OpaqueId::new(run_id))
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&run, true)?;
            } else {
                print_run_human(&run);
            }
            Ok(())
        }
        MedAgentCmd::RunList {
            vault_id,
            vault_root,
            project_id,
            agent_id,
            status,
            limit,
            json,
        } => {
            let status = status
                .map(|s| medscale_contracts::medagent::AgentRunState::parse(&s))
                .transpose()
                .map_err(|m| invalid(m, json))?;
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let runs = session
                .medagent_run_list(
                    OpaqueId::new(project_id),
                    agent_id.map(OpaqueId::new),
                    status,
                    limit,
                )
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&runs, true)?;
            } else {
                for run in &runs {
                    println!(
                        "{}\t{}\t{}",
                        run.header.id.as_str(),
                        run.status.as_str(),
                        run.revision
                    );
                }
            }
            Ok(())
        }
        MedAgentCmd::RunStart {
            vault_id,
            vault_root,
            run_id,
            expected_revision,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (run, turn) = session
                .medagent_run_start(OpaqueId::new(run_id), expected_revision)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&RunStartedJson { run, turn }, true)?;
            } else {
                print_run_human(&run);
                print_turn_human(&turn);
            }
            Ok(())
        }
        MedAgentCmd::RunCancel {
            vault_id,
            vault_root,
            run_id,
            expected_revision,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (run, receipt) = session
                .medagent_run_cancel(OpaqueId::new(run_id), expected_revision)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&RunTerminalJson { run, receipt }, true)?;
            } else {
                print_run_human(&run);
                print_run_receipt_human(&receipt);
            }
            Ok(())
        }
        MedAgentCmd::RunComplete {
            vault_id,
            vault_root,
            run_id,
            expected_revision,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (run, receipt) = session
                .medagent_run_complete(OpaqueId::new(run_id), expected_revision)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&RunTerminalJson { run, receipt }, true)?;
            } else {
                print_run_human(&run);
                print_run_receipt_human(&receipt);
            }
            Ok(())
        }
        MedAgentCmd::RunFail {
            vault_id,
            vault_root,
            run_id,
            expected_revision,
            failure_reason,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (run, receipt) = session
                .medagent_run_fail(OpaqueId::new(run_id), expected_revision, failure_reason)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&RunTerminalJson { run, receipt }, true)?;
            } else {
                print_run_human(&run);
                print_run_receipt_human(&receipt);
            }
            Ok(())
        }
        MedAgentCmd::RunTurns {
            vault_id,
            vault_root,
            run_id,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let turns = session
                .medagent_run_turns(OpaqueId::new(run_id))
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&turns, true)?;
            } else {
                for turn in &turns {
                    print_turn_human(turn);
                }
            }
            Ok(())
        }
        MedAgentCmd::ToolInvoke {
            vault_id,
            vault_root,
            run_id,
            kind,
            arguments,
            json,
        } => {
            let kind = parse_tool_kind(&kind).map_err(|m| invalid(m, json))?;
            let arguments: serde_json::Value =
                serde_json::from_str(&arguments).map_err(|e| invalid(e.to_string(), json))?;
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (invocation, receipt) = session
                .medagent_tool_invoke(OpaqueId::new(run_id), kind, arguments)
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &ToolInvocationJson {
                        invocation,
                        receipt,
                    },
                    true,
                )?;
            } else {
                print_tool_invocation_human(&invocation, receipt.as_ref());
            }
            Ok(())
        }
        MedAgentCmd::RunExecute {
            vault_id,
            vault_root,
            run_id,
            local_path,
            max_tokens,
            synthetic_only,
            json,
        } => {
            let mut session = open_medagent_session(&vault_id, &vault_root, json)?;
            let (turn, proposal) = session
                .medagent_run_execute(
                    OpaqueId::new(run_id),
                    local_path,
                    max_tokens,
                    synthetic_only,
                )
                .map_err(|err| medagent_fail(&err, json))?;
            if json {
                print_json_or_debug(&RunExecutedJson { turn, proposal }, true)?;
            } else {
                print_run_executed_human(&turn, &proposal);
            }
            Ok(())
        }
    }
}

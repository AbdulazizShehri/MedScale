//! Spec 078 Model Fleet + Compare commands (CLI vertical slice through Core).
//!
//! T078-03 scope: `AgentLane` create/show/list/retire. T078-04 scope:
//! `FleetRun` create/show/list/dispatch/execute-lane/cancel. T078-05/06:
//! comparison compute/list. Every command opens
//! the session scope, dispatches one typed Core request via `CliSession`,
//! and renders the typed result as human lines or stable JSON. The CLI never
//! reads or writes model_fleet storage directly.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::medagent::{AgentProposal, AgentRun, ToolKind};
use medscale_contracts::model_fleet::{
    AgentLane, AgentLaneStatus, ComparisonReport, FleetRun, FleetRunState, LaneRunRef,
};
use medscale_contracts::objects::OpaqueId;

use super::{fail_json, print_json_or_debug};

fn fleet_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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

fn open_fleet_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| fleet_fail(&err, json))
}

fn parse_tool_kinds(value: &str) -> Result<Vec<ToolKind>, String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToolKind::parse)
        .collect()
}

fn parse_ids(value: &str) -> Vec<OpaqueId> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(OpaqueId::new)
        .collect()
}

fn print_lane_human(lane: &AgentLane) {
    println!("lane_id: {}", lane.header.id.as_str());
    println!("project_id: {}", lane.project_id.as_str());
    println!("agent_identity_id: {}", lane.agent_identity_id.as_str());
    println!("context_manifest_id: {}", lane.context_manifest_id.as_str());
    println!("role_label: {}", lane.role_label.escape_debug());
    println!("status: {}", lane.status.as_str());
    println!("revision: {}", lane.revision);
    match &lane.policy.granted_tool_kinds {
        Some(kinds) => {
            let kinds: Vec<&str> = kinds.iter().map(|k| k.as_str()).collect();
            println!("policy_tool_kinds: {}", kinds.join(","));
        }
        None => println!("policy_tool_kinds: (inherit identity grant)"),
    }
    match &lane.policy.context_artifact_ids {
        Some(ids) => {
            let ids: Vec<&str> = ids.iter().map(OpaqueId::as_str).collect();
            println!("policy_context_artifacts: {}", ids.join(","));
        }
        None => println!("policy_context_artifacts: (inherit context manifest)"),
    }
}

fn print_fleet_human(run: &FleetRun, refs: &[LaneRunRef]) {
    println!("fleet_run_id: {}", run.header.id.as_str());
    println!("project_id: {}", run.project_id.as_str());
    println!("status: {}", run.status.as_str());
    println!("revision: {}", run.revision);
    println!("task_prompt_chars: {}", run.task_prompt.chars().count());
    for lane_ref in refs {
        println!(
            "lane: {}\trun: {}",
            lane_ref.agent_lane_id.as_str(),
            lane_ref.agent_run_id.as_str()
        );
    }
}

#[derive(Debug, serde::Serialize)]
struct FleetJson {
    run: FleetRun,
    lane_run_refs: Vec<LaneRunRef>,
}

#[derive(Debug, serde::Serialize)]
struct LaneExecutedJson {
    run: FleetRun,
    lane_run: AgentRun,
    proposal: Option<AgentProposal>,
}

fn print_fleet(run: FleetRun, lane_run_refs: Vec<LaneRunRef>, json: bool) -> anyhow::Result<()> {
    if json {
        print_json_or_debug(&FleetJson { run, lane_run_refs }, true)?;
    } else {
        print_fleet_human(&run, &lane_run_refs);
    }
    Ok(())
}

fn print_report_human(report: &ComparisonReport) {
    println!("report_id: {}", report.header.id.as_str());
    println!("fleet_run_id: {}", report.fleet_run_id.as_str());
    let ids = |ids: &[OpaqueId]| {
        ids.iter()
            .map(OpaqueId::as_str)
            .collect::<Vec<_>>()
            .join(",")
    };
    println!(
        "participating_lanes: {}",
        ids(&report.participating_lane_ids)
    );
    println!("excluded_lanes: {}", ids(&report.excluded_lane_ids));
    for observation in &report.observations {
        println!(
            "{}\t[{}]\t{}",
            observation.kind.as_str(),
            ids(&observation.participating_lane_ids),
            observation.detail.escape_debug()
        );
    }
}

/// Spec 078 Model Fleet + Compare commands.
#[derive(Debug, Subcommand)]
pub enum ModelFleetCmd {
    /// Create an agent lane over an existing agent identity + context manifest.
    LaneCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        agent_identity_id: String,
        #[arg(long)]
        context_manifest_id: String,
        #[arg(long)]
        role_label: String,
        /// Comma-separated subset of the identity's granted tool kinds;
        /// omit to inherit the full grant.
        #[arg(long)]
        tool_kinds: Option<String>,
        /// Comma-separated subset of the context manifest's artifact ids;
        /// omit to inherit the full manifest.
        #[arg(long)]
        context_artifacts: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one agent lane.
    LaneShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        lane_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List agent lanes in one Project.
    LaneList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        /// `active` or `retired`.
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Retire an agent lane (status tombstone; history is kept).
    LaneRetire {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        lane_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Create a pending fleet run with one task prompt for every lane.
    FleetCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        task_prompt: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one fleet run with its lane bindings.
    FleetShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        fleet_run_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List fleet runs in one Project.
    FleetList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        /// `pending`, `running`, `completed`, `partially_failed`, `failed` or `cancelled`.
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Dispatch a pending fleet run: one real agent run per lane.
    FleetDispatch {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        fleet_run_id: String,
        #[arg(long)]
        expected_revision: u64,
        /// Comma-separated agent lane ids (2..=8, distinct).
        #[arg(long)]
        lanes: String,
        /// Directory of an admitted Pack a lane identity binds (repeatable).
        /// Core keeps admitted Packs per process, so each is admitted first.
        #[arg(long = "pack-dir")]
        pack_dirs: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Execute one dispatched lane through its admitted local model Pack.
    FleetExecuteLane {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        fleet_run_id: String,
        #[arg(long)]
        lane_id: String,
        /// Directory of the exact admitted Pack the lane's identity binds.
        #[arg(long)]
        pack_dir: PathBuf,
        #[arg(long, default_value_t = 64)]
        max_tokens: usize,
        #[arg(long)]
        json: bool,
    },
    /// Compute a new comparison report over a completed or partially
    /// failed fleet run (factual observations only; never a ranking).
    CompareRun {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        fleet_run_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List every comparison report computed over one fleet run.
    CompareList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        fleet_run_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Cancel a pending or running fleet run.
    FleetCancel {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        fleet_run_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
}

fn print_lane(lane: &AgentLane, json: bool) -> anyhow::Result<()> {
    if json {
        print_json_or_debug(lane, true)?;
    } else {
        print_lane_human(lane);
    }
    Ok(())
}

pub fn run_model_fleet(action: ModelFleetCmd) -> anyhow::Result<()> {
    match action {
        ModelFleetCmd::LaneCreate {
            vault_id,
            vault_root,
            project_id,
            agent_identity_id,
            context_manifest_id,
            role_label,
            tool_kinds,
            context_artifacts,
            json,
        } => {
            let granted_tool_kinds = tool_kinds
                .as_deref()
                .map(parse_tool_kinds)
                .transpose()
                .map_err(|m| invalid(m, json))?;
            let context_artifact_ids = context_artifacts.as_deref().map(parse_ids);
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let lane = session
                .model_fleet_lane_create(
                    OpaqueId::new(project_id),
                    OpaqueId::new(agent_identity_id),
                    OpaqueId::new(context_manifest_id),
                    role_label,
                    granted_tool_kinds,
                    context_artifact_ids,
                )
                .map_err(|err| fleet_fail(&err, json))?;
            print_lane(&lane, json)
        }
        ModelFleetCmd::LaneShow {
            vault_id,
            vault_root,
            lane_id,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let lane = session
                .model_fleet_lane_get(OpaqueId::new(lane_id))
                .map_err(|err| fleet_fail(&err, json))?;
            print_lane(&lane, json)
        }
        ModelFleetCmd::LaneList {
            vault_id,
            vault_root,
            project_id,
            status,
            limit,
            json,
        } => {
            let status = status
                .as_deref()
                .map(AgentLaneStatus::parse)
                .transpose()
                .map_err(|m| invalid(m, json))?;
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let lanes = session
                .model_fleet_lane_list(OpaqueId::new(project_id), status, limit)
                .map_err(|err| fleet_fail(&err, json))?;
            if json {
                print_json_or_debug(&lanes, true)?;
            } else {
                for lane in &lanes {
                    println!(
                        "{}\t{}\t{}\t{}",
                        lane.header.id.as_str(),
                        lane.role_label.escape_debug(),
                        lane.status.as_str(),
                        lane.revision
                    );
                }
            }
            Ok(())
        }
        ModelFleetCmd::LaneRetire {
            vault_id,
            vault_root,
            lane_id,
            expected_revision,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let lane = session
                .model_fleet_lane_retire(OpaqueId::new(lane_id), expected_revision)
                .map_err(|err| fleet_fail(&err, json))?;
            print_lane(&lane, json)
        }
        ModelFleetCmd::FleetCreate {
            vault_id,
            vault_root,
            project_id,
            task_prompt,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let run = session
                .model_fleet_run_create(OpaqueId::new(project_id), task_prompt)
                .map_err(|err| fleet_fail(&err, json))?;
            print_fleet(run, Vec::new(), json)
        }
        ModelFleetCmd::FleetShow {
            vault_id,
            vault_root,
            fleet_run_id,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let (run, refs) = session
                .model_fleet_run_get(OpaqueId::new(fleet_run_id))
                .map_err(|err| fleet_fail(&err, json))?;
            print_fleet(run, refs, json)
        }
        ModelFleetCmd::FleetList {
            vault_id,
            vault_root,
            project_id,
            status,
            limit,
            json,
        } => {
            let status = status
                .as_deref()
                .map(FleetRunState::parse)
                .transpose()
                .map_err(|m| invalid(m, json))?;
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let runs = session
                .model_fleet_run_list(OpaqueId::new(project_id), status, limit)
                .map_err(|err| fleet_fail(&err, json))?;
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
        ModelFleetCmd::FleetDispatch {
            vault_id,
            vault_root,
            fleet_run_id,
            expected_revision,
            lanes,
            pack_dirs,
            json,
        } => {
            let lane_ids = parse_ids(&lanes);
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            for pack_dir in &pack_dirs {
                session
                    .packs_install_local(&pack_dir.display().to_string())
                    .map_err(|err| fleet_fail(&err, json))?;
            }
            let (run, refs) = session
                .model_fleet_run_dispatch(OpaqueId::new(fleet_run_id), expected_revision, lane_ids)
                .map_err(|err| fleet_fail(&err, json))?;
            print_fleet(run, refs, json)
        }
        ModelFleetCmd::FleetExecuteLane {
            vault_id,
            vault_root,
            fleet_run_id,
            lane_id,
            pack_dir,
            max_tokens,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            // The Core facade keeps admitted Packs per process: admit the
            // exact directory first, as `medagent run-execute` does.
            session
                .packs_install_local(&pack_dir.display().to_string())
                .map_err(|err| fleet_fail(&err, json))?;
            let (run, lane_run, proposal) = session
                .model_fleet_run_execute_lane(
                    OpaqueId::new(fleet_run_id),
                    OpaqueId::new(lane_id),
                    pack_dir.display().to_string(),
                    max_tokens,
                    true,
                )
                .map_err(|err| fleet_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &LaneExecutedJson {
                        run,
                        lane_run,
                        proposal,
                    },
                    true,
                )?;
            } else {
                println!("fleet_run_id: {}", run.header.id.as_str());
                println!("fleet_status: {}", run.status.as_str());
                println!("lane_run_id: {}", lane_run.header.id.as_str());
                println!("lane_run_status: {}", lane_run.status.as_str());
                match proposal {
                    Some(p) => println!("proposal_id: {}", p.proposal_id.as_str()),
                    None => println!("proposal_id: (none)"),
                }
            }
            Ok(())
        }
        ModelFleetCmd::CompareRun {
            vault_id,
            vault_root,
            fleet_run_id,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let report = session
                .model_fleet_compare(OpaqueId::new(fleet_run_id))
                .map_err(|err| fleet_fail(&err, json))?;
            if json {
                print_json_or_debug(&report, true)?;
            } else {
                print_report_human(&report);
            }
            Ok(())
        }
        ModelFleetCmd::CompareList {
            vault_id,
            vault_root,
            fleet_run_id,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let reports = session
                .model_fleet_compare_list(OpaqueId::new(fleet_run_id))
                .map_err(|err| fleet_fail(&err, json))?;
            if json {
                print_json_or_debug(&reports, true)?;
            } else {
                for report in &reports {
                    print_report_human(report);
                }
            }
            Ok(())
        }
        ModelFleetCmd::FleetCancel {
            vault_id,
            vault_root,
            fleet_run_id,
            expected_revision,
            json,
        } => {
            let mut session = open_fleet_session(&vault_id, &vault_root, json)?;
            let (run, refs) = session
                .model_fleet_run_cancel(OpaqueId::new(fleet_run_id), expected_revision)
                .map_err(|err| fleet_fail(&err, json))?;
            print_fleet(run, refs, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::project_graph::{
        ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding,
    };
    use medscale_core::CliSession;

    const VAULT: &str = "cli-fleet";

    /// Seeds a Project, an identity and a context manifest through Core in
    /// one session, then drops it: every lane command below runs in its own
    /// fresh CLI session over the persisted vault, exactly as the binary does.
    fn seed(root: &std::path::Path) -> (OpaqueId, OpaqueId, OpaqueId) {
        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let project = session
            .project_create("cli fleet".to_owned(), None)
            .unwrap();
        let pack_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0");
        let admitted = session
            .packs_install_local(&pack_dir.display().to_string())
            .unwrap();
        let (identity, _) = session
            .medagent_identity_register(
                project.header.id.clone(),
                admitted.pack_id.unwrap(),
                "cli lane agent".to_owned(),
                vec![ToolKind::ReadContextArtifact],
            )
            .unwrap();
        let (context, _) = session
            .medagent_context_create(
                project.header.id.clone(),
                vec![ArtifactDescriptor {
                    object_id: OpaqueId::new("artifact-1"),
                    kind: ArtifactKind::SourceRecord,
                    binding: ArtifactVersionBinding::IdentityOnly,
                }],
            )
            .unwrap();
        (project.header.id, identity.header.id, context.header.id)
    }

    #[test]
    fn lane_commands_run_through_core_across_fresh_sessions() {
        let root = std::env::temp_dir().join(format!("medscale-078-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let (project_id, agent_id, context_id) = seed(&root);

        let create = |tool_kinds: Option<&str>, json: bool| {
            run_model_fleet(ModelFleetCmd::LaneCreate {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.as_str().to_owned(),
                agent_identity_id: agent_id.as_str().to_owned(),
                context_manifest_id: context_id.as_str().to_owned(),
                role_label: "reviewer".to_owned(),
                tool_kinds: tool_kinds.map(str::to_owned),
                context_artifacts: Some("artifact-1".to_owned()),
                json,
            })
        };
        create(Some("read_context_artifact"), true).expect("json lane create");
        create(None, false).expect("human lane create");
        // T2 refusal surfaces as a CLI error, not a created lane.
        assert!(create(Some("search_context_artifacts"), true).is_err());
        assert!(create(Some("not_a_tool_kind"), true).is_err());

        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let lanes = session
            .model_fleet_lane_list(project_id.clone(), None, None)
            .unwrap();
        assert_eq!(lanes.len(), 2);
        let lane_id = lanes[0].header.id.as_str().to_owned();
        drop(session);

        for json in [true, false] {
            run_model_fleet(ModelFleetCmd::LaneShow {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                lane_id: lane_id.clone(),
                json,
            })
            .expect("lane show");
            run_model_fleet(ModelFleetCmd::LaneList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.as_str().to_owned(),
                status: Some("active".to_owned()),
                limit: None,
                json,
            })
            .expect("lane list");
        }
        assert!(
            run_model_fleet(ModelFleetCmd::LaneList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.as_str().to_owned(),
                status: Some("winner".to_owned()),
                limit: None,
                json: true,
            })
            .is_err(),
            "an unknown status is refused, never coerced"
        );
        run_model_fleet(ModelFleetCmd::LaneRetire {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            lane_id: lane_id.clone(),
            expected_revision: 1,
            json: true,
        })
        .expect("lane retire");
        assert!(
            run_model_fleet(ModelFleetCmd::LaneRetire {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                lane_id: lane_id.clone(),
                expected_revision: 1,
                json: true,
            })
            .is_err(),
            "stale retire is a conflict"
        );

        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let retired = session
            .model_fleet_lane_get(OpaqueId::new(lane_id.clone()))
            .unwrap();
        assert_eq!(retired.status, AgentLaneStatus::Retired);
        assert_eq!(retired.revision, 2);
        let second_lane = lanes[1].header.id.as_str().to_owned();
        let third_lane = session
            .model_fleet_lane_create(
                project_id.clone(),
                agent_id.clone(),
                context_id.clone(),
                "second reviewer".to_owned(),
                None,
                None,
            )
            .unwrap()
            .header
            .id;
        drop(session);

        // Fleet commands: create -> dispatch -> cancel, human + JSON.
        run_model_fleet(ModelFleetCmd::FleetCreate {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            project_id: project_id.as_str().to_owned(),
            task_prompt: "compare".to_owned(),
            json: true,
        })
        .expect("fleet create");
        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let fleets = session
            .model_fleet_run_list(project_id.clone(), None, None)
            .unwrap();
        assert_eq!(fleets.len(), 1);
        let fleet_id = fleets[0].header.id.as_str().to_owned();
        drop(session);

        // Refused without writes: the retired lane, and lanes whose Pack is
        // not admitted in this process. Then two active lanes dispatch.
        let pack_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0");
        for (lanes, pack_dirs) in [
            (format!("{lane_id},{second_lane}"), vec![pack_dir.clone()]),
            (format!("{second_lane},{}", third_lane.as_str()), Vec::new()),
        ] {
            assert!(
                run_model_fleet(ModelFleetCmd::FleetDispatch {
                    vault_id: VAULT.to_owned(),
                    vault_root: root.clone(),
                    fleet_run_id: fleet_id.clone(),
                    expected_revision: 1,
                    lanes,
                    pack_dirs,
                    json: true,
                })
                .is_err()
            );
        }
        run_model_fleet(ModelFleetCmd::FleetDispatch {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            fleet_run_id: fleet_id.clone(),
            expected_revision: 1,
            lanes: format!("{second_lane},{}", third_lane.as_str()),
            pack_dirs: vec![pack_dir],
            json: false,
        })
        .expect("fleet dispatch");
        for json in [true, false] {
            run_model_fleet(ModelFleetCmd::FleetShow {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                fleet_run_id: fleet_id.clone(),
                json,
            })
            .expect("fleet show");
            run_model_fleet(ModelFleetCmd::FleetList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project_id.as_str().to_owned(),
                status: Some("running".to_owned()),
                limit: None,
                json,
            })
            .expect("fleet list");
        }
        run_model_fleet(ModelFleetCmd::FleetCancel {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            fleet_run_id: fleet_id.clone(),
            expected_revision: 2,
            json: true,
        })
        .expect("fleet cancel");

        let mut session = CliSession::connect(VAULT).unwrap();
        session
            .open_synthetic_vault(&root.display().to_string())
            .unwrap();
        let (fleet, refs) = session
            .model_fleet_run_get(OpaqueId::new(fleet_id))
            .unwrap();
        assert_eq!(fleet.status, FleetRunState::Cancelled);
        assert_eq!(refs.len(), 2);
        drop(session);

        assert!(
            run_model_fleet(ModelFleetCmd::CompareRun {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                fleet_run_id: fleet.header.id.as_str().to_owned(),
                json: true,
            })
            .is_err(),
            "a cancelled fleet has no comparable output"
        );
        for json in [true, false] {
            run_model_fleet(ModelFleetCmd::CompareList {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                fleet_run_id: fleet.header.id.as_str().to_owned(),
                json,
            })
            .expect("compare list");
        }
    }
}

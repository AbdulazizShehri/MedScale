//! Model Fleet + Compare view-models (Spec 078, T078-07).
//!
//! Desktop reads and mutates fleet state only through the Core-owned
//! `CliSession` (facade authority; never storage or the model runtime
//! directly). Every function maps one typed Core result to plain view-model
//! rows plus an explicit status string; no product data is synthesized.
//! Reuses the same `desktop-projects` session Specs 074-077 already open:
//! lanes and fleet runs are scoped to Projects.
//!
//! Scope matches `tasks.md` T078-07: fleet list/detail, per-lane status, and
//! the comparison-report view, plus the fleet actions a reviewer needs to
//! produce them (create, dispatch over operator-supplied lane ids, execute
//! one lane against an operator-supplied Pack directory, cancel, compare).
//! Agent-lane creation stays CLI-only in this slice, exactly like Spec 077's
//! identity/context creation (`medagent_workspace.rs`).

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::model_fleet::ComparisonReport;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

/// One agent-lane row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaneRowVm {
    pub id: String,
    pub role_label: String,
    pub status: String,
    pub revision: u64,
}

/// One fleet-run row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FleetRowVm {
    pub id: String,
    pub status: String,
    pub revision: u64,
}

/// One lane binding inside a fleet run, with the lane run's live state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaneRunRowVm {
    pub lane_id: String,
    pub run_id: String,
    pub run_status: String,
}

/// One fleet run with its lane bindings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FleetDetailVm {
    pub fleet: FleetRowVm,
    pub lanes: Vec<LaneRunRowVm>,
}

/// One comparison observation row (factual text; never a score).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservationRowVm {
    pub kind: String,
    pub lanes: String,
    pub detail: String,
}

/// One comparison report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportVm {
    pub id: String,
    pub participating: String,
    pub excluded: String,
    pub observations: Vec<ObservationRowVm>,
}

/// Maps a typed Core error to an explicit status (no payload leak).
#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::Conflict { .. } => "Conflict: stale revision or fleet is not in that state",
        AuthorityError::InvalidArgument { .. } => "Invalid: rejected before any write",
        AuthorityError::Corrupt { .. } => "Corrupt: integrity check failed",
        AuthorityError::UnsupportedSchema { .. } => "Unsupported: unknown schema value",
        AuthorityError::ExternalGateRequired { .. } => {
            "Blocked: requires an explicit gate not granted here"
        }
        _ => "Unavailable: vault or Core not ready",
    }
}

fn ids(ids: &[OpaqueId]) -> String {
    ids.iter()
        .map(OpaqueId::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

fn fleet_row(run: &medscale_contracts::model_fleet::FleetRun) -> FleetRowVm {
    FleetRowVm {
        id: run.header.id.as_str().to_owned(),
        status: run.status.as_str().to_owned(),
        revision: run.revision,
    }
}

fn report_vm(report: ComparisonReport) -> ReportVm {
    ReportVm {
        id: report.header.id.as_str().to_owned(),
        participating: ids(&report.participating_lane_ids),
        excluded: ids(&report.excluded_lane_ids),
        observations: report
            .observations
            .into_iter()
            .map(|o| ObservationRowVm {
                kind: o.kind.as_str().to_owned(),
                lanes: ids(&o.participating_lane_ids),
                detail: o.detail,
            })
            .collect(),
    }
}

/// Lists agent lanes in one Project.
pub fn refresh_lanes(
    session: &mut CliSession,
    project_id: &str,
) -> Result<Vec<LaneRowVm>, AuthorityError> {
    let lanes = session.model_fleet_lane_list(OpaqueId::new(project_id), None, Some(100))?;
    Ok(lanes
        .into_iter()
        .map(|lane| LaneRowVm {
            id: lane.header.id.as_str().to_owned(),
            role_label: lane.role_label,
            status: lane.status.as_str().to_owned(),
            revision: lane.revision,
        })
        .collect())
}

/// Lists fleet runs in one Project.
pub fn refresh_fleets(
    session: &mut CliSession,
    project_id: &str,
) -> Result<Vec<FleetRowVm>, AuthorityError> {
    let runs = session.model_fleet_run_list(OpaqueId::new(project_id), None, Some(100))?;
    Ok(runs.iter().map(fleet_row).collect())
}

/// Reads one fleet run with each bound lane's live run state.
pub fn open_fleet(
    session: &mut CliSession,
    fleet_run_id: &str,
) -> Result<FleetDetailVm, AuthorityError> {
    let (run, refs) = session.model_fleet_run_get(OpaqueId::new(fleet_run_id))?;
    let mut lanes = Vec::with_capacity(refs.len());
    for lane_ref in refs {
        let lane_run = session.medagent_run_get(lane_ref.agent_run_id.clone())?;
        lanes.push(LaneRunRowVm {
            lane_id: lane_ref.agent_lane_id.as_str().to_owned(),
            run_id: lane_ref.agent_run_id.as_str().to_owned(),
            run_status: lane_run.status.as_str().to_owned(),
        });
    }
    Ok(FleetDetailVm {
        fleet: fleet_row(&run),
        lanes,
    })
}

/// Creates a `Pending` fleet run.
pub fn create_fleet(
    session: &mut CliSession,
    project_id: &str,
    task_prompt: String,
) -> Result<FleetRowVm, AuthorityError> {
    let run = session.model_fleet_run_create(OpaqueId::new(project_id), task_prompt)?;
    Ok(fleet_row(&run))
}

/// Dispatches a `Pending` fleet run over comma-separated lane ids, first
/// admitting the operator-supplied Pack directory in this Core session.
pub fn dispatch_fleet(
    session: &mut CliSession,
    fleet_run_id: &str,
    expected_revision: u64,
    lane_ids_csv: &str,
    pack_dir: &str,
) -> Result<FleetRowVm, AuthorityError> {
    let lane_ids: Vec<OpaqueId> = lane_ids_csv
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(OpaqueId::new)
        .collect();
    session.packs_install_local(pack_dir)?;
    let (run, _refs) = session.model_fleet_run_dispatch(
        OpaqueId::new(fleet_run_id),
        expected_revision,
        lane_ids,
    )?;
    Ok(fleet_row(&run))
}

/// Executes one dispatched lane through its admitted local Pack.
pub fn execute_lane(
    session: &mut CliSession,
    fleet_run_id: &str,
    lane_id: &str,
    pack_dir: &str,
) -> Result<FleetRowVm, AuthorityError> {
    session.packs_install_local(pack_dir)?;
    let (run, _lane_run, _proposal) = session.model_fleet_run_execute_lane(
        OpaqueId::new(fleet_run_id),
        OpaqueId::new(lane_id),
        pack_dir.to_owned(),
        64,
        true,
    )?;
    Ok(fleet_row(&run))
}

/// Cancels a `Pending` or `Running` fleet run.
pub fn cancel_fleet(
    session: &mut CliSession,
    fleet_run_id: &str,
    expected_revision: u64,
) -> Result<FleetRowVm, AuthorityError> {
    let (run, _refs) =
        session.model_fleet_run_cancel(OpaqueId::new(fleet_run_id), expected_revision)?;
    Ok(fleet_row(&run))
}

/// Computes a new comparison report over a fleet run.
pub fn compare_fleet(
    session: &mut CliSession,
    fleet_run_id: &str,
) -> Result<ReportVm, AuthorityError> {
    Ok(report_vm(
        session.model_fleet_compare(OpaqueId::new(fleet_run_id))?,
    ))
}

/// The most recent comparison report over a fleet run, if any.
pub fn latest_report(
    session: &mut CliSession,
    fleet_run_id: &str,
) -> Result<Option<ReportVm>, AuthorityError> {
    Ok(session
        .model_fleet_compare_list(OpaqueId::new(fleet_run_id))?
        .pop()
        .map(report_vm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::medagent::ToolKind;
    use medscale_contracts::project_graph::{
        ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding,
    };

    fn test_session(name: &str) -> CliSession {
        let dir = std::env::temp_dir().join(format!("medscale-078d-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut session = CliSession::connect("desktop-fleet-test").expect("operator session");
        session
            .open_synthetic_vault(&dir.display().to_string())
            .expect("open vault");
        session
    }

    fn onnx_pack_dir() -> String {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0")
            .display()
            .to_string()
    }

    /// Every view-model function is exercised against a real `CliSession`
    /// (Core) with real local ONNX execution, never fake data, so the
    /// Desktop Model Fleet panel's data path is proven even though the Slint
    /// UI is not rendered in CI.
    #[test]
    fn model_fleet_workspace_flows_through_real_core_session() {
        let mut session = test_session("flows");
        let project_id = session
            .project_create("fleet study".to_owned(), None)
            .expect("project")
            .header
            .id;
        let pack = session
            .packs_install_local(&onnx_pack_dir())
            .expect("install onnx pack");
        let pack_id = pack.pack_id.expect("admitted pack id");
        let mut lane_ids = Vec::new();
        // Each lane's context is a real source: a run cites only evidence
        // that resolves, and refuses to generate without any.
        for n in 0..2 {
            let artifact = session
                .create_source_record(
                    "text/plain".to_owned(),
                    format!("synthetic note for lane {n}").into_bytes(),
                )
                .expect("source record");
            let (identity, _) = session
                .medagent_identity_register(
                    project_id.clone(),
                    pack_id.clone(),
                    format!("lane agent {n}"),
                    vec![ToolKind::ReadContextArtifact],
                )
                .expect("identity");
            let (context, _) = session
                .medagent_context_create(
                    project_id.clone(),
                    vec![ArtifactDescriptor {
                        object_id: artifact,
                        kind: ArtifactKind::SourceRecord,
                        binding: ArtifactVersionBinding::IdentityOnly,
                    }],
                )
                .expect("context");
            let lane = session
                .model_fleet_lane_create(
                    project_id.clone(),
                    identity.header.id,
                    context.header.id,
                    format!("reviewer {n}"),
                    None,
                    None,
                )
                .expect("lane");
            lane_ids.push(lane.header.id.as_str().to_owned());
        }
        let project = project_id.as_str();

        let lanes = refresh_lanes(&mut session, project).expect("lanes");
        assert_eq!(lanes.len(), 2);
        assert!(lanes.iter().all(|l| l.status == "active"));

        let fleet = create_fleet(&mut session, project, "summarize".to_owned()).expect("create");
        assert_eq!((fleet.status.as_str(), fleet.revision), ("pending", 1));
        assert_eq!(
            refresh_fleets(&mut session, project).unwrap(),
            vec![fleet.clone()]
        );
        // A comparison over a pending fleet is refused, never faked.
        assert!(matches!(
            compare_fleet(&mut session, &fleet.id),
            Err(AuthorityError::InvalidArgument { .. })
        ));

        let running = dispatch_fleet(
            &mut session,
            &fleet.id,
            fleet.revision,
            &lane_ids.join(", "),
            &onnx_pack_dir(),
        )
        .expect("dispatch");
        assert_eq!(running.status, "running");
        let detail = open_fleet(&mut session, &fleet.id).expect("open");
        assert_eq!(detail.lanes.len(), 2);
        assert!(detail.lanes.iter().all(|l| l.run_status == "running"));

        for lane in &lane_ids {
            execute_lane(&mut session, &fleet.id, lane, &onnx_pack_dir()).expect("execute");
        }
        let detail = open_fleet(&mut session, &fleet.id).expect("open");
        assert_eq!(detail.fleet.status, "completed");
        assert!(detail.lanes.iter().all(|l| l.run_status == "completed"));

        assert!(latest_report(&mut session, &fleet.id).unwrap().is_none());
        let report = compare_fleet(&mut session, &fleet.id).expect("compare");
        assert!(report.excluded.is_empty());
        assert!(report.observations.iter().any(|o| o.kind == "agreement"));
        assert_eq!(
            latest_report(&mut session, &fleet.id).unwrap(),
            Some(report)
        );

        // Cancel on a terminal fleet is an explicit conflict.
        let err = cancel_fleet(&mut session, &fleet.id, detail.fleet.revision).unwrap_err();
        assert_eq!(
            status_message(&err),
            "Conflict: stale revision or fleet is not in that state"
        );
        let pending = create_fleet(&mut session, project, "again".to_owned()).unwrap();
        let cancelled = cancel_fleet(&mut session, &pending.id, pending.revision).unwrap();
        assert_eq!(cancelled.status, "cancelled");
    }

    #[test]
    fn status_message_is_explicit_for_every_error_class() {
        assert_eq!(
            status_message(&AuthorityError::NotFound),
            "Missing: not found in this vault"
        );
        assert!(
            !status_message(&AuthorityError::Internal {
                message: "x".to_owned()
            })
            .is_empty()
        );
    }
}

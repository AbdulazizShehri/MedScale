//! Native MedScale Desktop shell — Slint, no WebView/Tauri (Spec 060).

use medscale_core::{CliSession, CoreFacade, build_doctor_report, privacy_proof_artifact_present};
use std::cell::RefCell;
use std::env;
use std::hint::black_box;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, VecModel};

mod analytics_workspace;
mod audio_workspace;
mod browse_workspace;
mod collaboration_workspace;
mod data_workbench;
mod knowledge_workspace;
mod medagent_workspace;
mod model_fleet_workspace;
mod patient_workspace;
mod population_insights;
mod privacy_workspace;
mod product_intelligence;
mod project_workspace;
mod research_os_workspace;
mod utility_surfaces;
mod visual_evidence;
mod workflow_studio;

slint::include_modules!();

const PERF_IDLE_MAX_MS: u64 = 10_000;

fn apply_product_intelligence(ui: &AppWindow, vm: &product_intelligence::ProductIntelligenceVm) {
    ui.set_model_runtime_summary(vm.model_runtime_summary.clone().into());
    ui.set_model_runtime_boundary(vm.model_runtime_boundary.clone().into());
    ui.set_model_source_summary(vm.model_source_summary.clone().into());
    ui.set_model_inventory_summary(vm.model_inventory_summary.clone().into());
    ui.set_reference_baseline(vm.reference_baseline.clone().into());
    ui.set_competitive_summary(vm.competitive_summary.clone().into());
    ui.set_model_rows(ModelRc::new(VecModel::from_iter(vm.models.iter().map(
        |row| ModelStatusItem {
            scope: row.scope.clone().into(),
            task: row.task.clone().into(),
            model: row.model.clone().into(),
            source: row.source.clone().into(),
            runtime: row.runtime.clone().into(),
            device: row.device.clone().into(),
            trust: row.trust.clone().into(),
            benchmark: row.benchmark.clone().into(),
            digest: row.digest.clone().into(),
            state: row.state.clone().into(),
        },
    ))));
    ui.set_competitive_evidence_rows(ModelRc::new(VecModel::from_iter(vm.evidence.iter().map(
        |row| CompetitiveEvidenceItem {
            capability: row.capability.clone().into(),
            medscale: row.medscale.clone().into(),
            reference: row.reference.clone().into(),
            verdict: row.verdict.clone().into(),
            evidence: row.evidence.clone().into(),
            limitations: row.limitations.clone().into(),
        },
    ))));
}

fn refresh_model_center(
    ui: &AppWindow,
    session: &Rc<RefCell<CliSession>>,
) -> Result<(), medscale_contracts::envelopes::AuthorityError> {
    let vm = product_intelligence::ProductIntelligenceVm::from_session(&mut session.borrow_mut())?;
    apply_product_intelligence(ui, &vm);
    Ok(())
}

fn admit_model_pack(
    session: &Rc<RefCell<CliSession>>,
    path: &str,
) -> Result<medscale_contracts::packs::PackAdmitResult, medscale_contracts::envelopes::AuthorityError>
{
    let mut session = session.borrow_mut();
    session.packs_install_local(path)
}

/// Refreshes the Projects list from Core; honest empty/denied/unavailable states.
fn refresh_project_list(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    match project_workspace::refresh_projects(&mut session.borrow_mut()) {
        Ok(rows) => {
            if rows.is_empty() {
                ui.set_project_status("No projects yet · create one above".into());
            } else {
                ui.set_project_status(
                    format!(
                        "{} project{} · Core-backed",
                        rows.len(),
                        if rows.len() == 1 { "" } else { "s" }
                    )
                    .into(),
                );
            }
            ui.set_project_rows(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                ProjectRowItem {
                    id: row.id.clone().into(),
                    name: row.name.clone().into(),
                    status: row.status.clone().into(),
                    revision: row.revision.to_string().into(),
                    experiments: row.experiments.to_string().into(),
                    refs: row.refs.to_string().into(),
                    edges: row.edges.to_string().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_project_rows(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_project_status(project_workspace::status_message(&err).into());
        }
    }
}

/// Refreshes the Data Source list for the active Project from Core.
fn refresh_data_sources(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_data_status("Open a project to inspect its data sources".into());
        ui.set_data_sources(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    match data_workbench::refresh_sources(&mut session.borrow_mut(), &project_id) {
        Ok(rows) => {
            if rows.is_empty() {
                ui.set_data_status("No data sources yet in this project".into());
            } else {
                ui.set_data_status(
                    format!(
                        "{} source{} · Core-backed",
                        rows.len(),
                        if rows.len() == 1 { "" } else { "s" }
                    )
                    .into(),
                );
            }
            ui.set_data_sources(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                DataSourceRowItem {
                    id: row.id.clone().into(),
                    name: row.name.clone().into(),
                    kind: row.kind.clone().into(),
                    health: row.health.clone().into(),
                    revision: row.revision.to_string().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_data_sources(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_data_status(data_workbench::status_message(&err).into());
        }
    }
}

/// Opens one data source: snapshots plus the latest workbench page.
fn open_data_source(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, source_id: &str) {
    match data_workbench::source_snapshots(&mut session.borrow_mut(), source_id) {
        Ok(snapshots) => {
            ui.set_data_active_source(source_id.into());
            ui.set_data_snapshots(ModelRc::new(VecModel::from_iter(snapshots.iter().map(
                |row| SnapshotRowItem {
                    id: row.id.clone().into(),
                    rows: row.rows.to_string().into(),
                    digest: row.digest.clone().into(),
                },
            ))));
            // list_snapshots pages oldest snapshot_id first; the most recent
            // fetched snapshot is the last element, not the first.
            if let Some(latest) = snapshots.last() {
                let id = latest.id.clone();
                open_data_snapshot(ui, session, &id);
            } else {
                ui.set_data_status(
                    "Source has no snapshots yet · import to materialize one".into(),
                );
            }
        }
        Err(err) => ui.set_data_status(data_workbench::status_message(&err).into()),
    }
}

/// Opens one snapshot workbench page (typed schema plus rendered rows).
fn open_data_snapshot(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, snapshot_id: &str) {
    match data_workbench::workbench_page(&mut session.borrow_mut(), snapshot_id) {
        Ok(page) => {
            ui.set_data_active_snapshot(page.snapshot_id.clone().into());
            ui.set_data_schema_line(page.schema_line.clone().into());
            ui.set_data_detail(page.detail.clone().into());
            ui.set_data_rows(ModelRc::new(VecModel::from_iter(page.rows.iter().map(
                |text| WorkbenchRowItem {
                    text: text.clone().into(),
                },
            ))));
            ui.set_data_next_cursor(page.next_cursor.unwrap_or_default().into());
            ui.set_data_status(format!("Open: {}", page.snapshot_id).into());
        }
        Err(err) => ui.set_data_status(data_workbench::status_message(&err).into()),
    }
}

/// Opens one Project detail (overview, experiments, refs, relationships).
fn open_project_detail(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, project_id: &str) {
    match project_workspace::project_detail(&mut session.borrow_mut(), project_id) {
        Ok(detail) => {
            // A switched project must not leave the previous project's data
            // workbench selection active: otherwise a subsequent data action
            // reads the stale source/snapshot id and mutates the wrong
            // project's data while the UI shows this one.
            ui.set_data_active_source("".into());
            ui.set_data_active_snapshot("".into());
            ui.set_data_sources(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_data_snapshots(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_data_rows(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_data_schema_line("".into());
            ui.set_data_detail("".into());
            ui.set_data_next_cursor("".into());
            ui.set_data_status("Data not loaded".into());
            ui.set_project_active_id(detail.project_id.clone().into());
            ui.set_project_active_revision(detail.revision.to_string().into());
            ui.set_project_detail_name(detail.name.clone().into());
            ui.set_project_detail_meta(
                format!(
                    "{} · {} · status {} · revision {} · {} experiments · {} refs · {} edges",
                    detail.project_id,
                    detail.name,
                    detail.status,
                    detail.revision,
                    detail.experiment_count,
                    detail.active_ref_count,
                    detail.active_edge_count
                )
                .into(),
            );
            ui.set_project_experiments(ModelRc::new(VecModel::from_iter(
                detail.experiments.iter().map(|row| ExperimentRowItem {
                    id: row.id.clone().into(),
                    name: row.name.clone().into(),
                    status: row.status.clone().into(),
                    revision: row.revision.to_string().into(),
                }),
            )));
            ui.set_project_refs(ModelRc::new(VecModel::from_iter(detail.refs.iter().map(
                |row| RefRowItem {
                    ref_id: row.ref_id.clone().into(),
                    kind: row.kind.clone().into(),
                    object_id: row.object_id.clone().into(),
                    resolution: row.resolution.clone().into(),
                },
            ))));
            ui.set_project_edges(ModelRc::new(VecModel::from_iter(detail.edges.iter().map(
                |row| EdgeRowItem {
                    edge_id: row.edge_id.clone().into(),
                    subject: row.subject.clone().into(),
                    predicate: row.predicate.clone().into(),
                    object: row.object.clone().into(),
                    revision: row.revision.to_string().into(),
                },
            ))));
            ui.set_project_status(format!("Open: {}", detail.name).into());
        }
        Err(err) => {
            ui.set_project_active_id("".into());
            ui.set_project_status(project_workspace::status_message(&err).into());
        }
    }
}

/// Refreshes the Rooms list for the active Project from Core.
fn refresh_collab_rooms(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_collab_status("Open a project to inspect its rooms".into());
        ui.set_collab_rooms(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    match collaboration_workspace::refresh_rooms(&mut session.borrow_mut(), &project_id) {
        Ok(rows) => {
            ui.set_collab_status(
                format!(
                    "{} room{} · Core-backed",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                )
                .into(),
            );
            ui.set_collab_rooms(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                RoomRowItem {
                    id: row.id.clone().into(),
                    name: row.name.clone().into(),
                    revision: row.revision.to_string().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_collab_rooms(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_collab_status(collaboration_workspace::status_message(&err).into());
        }
    }
}

/// Opens one Room: loads its threads and tasks.
fn open_collab_room(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, room_id: &str) {
    ui.set_collab_active_room_id(room_id.into());
    ui.set_collab_active_thread_id("".into());
    ui.set_collab_messages(ModelRc::new(VecModel::from(Vec::new())));
    refresh_collab_threads(ui, session, room_id);
    refresh_collab_tasks(ui, session, room_id);
}

fn refresh_collab_threads(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, room_id: &str) {
    match collaboration_workspace::refresh_threads(&mut session.borrow_mut(), room_id) {
        Ok(rows) => {
            ui.set_collab_threads(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                ThreadRowItem {
                    id: row.id.clone().into(),
                    status: row.status.clone().into(),
                    resolution: row.resolution.clone().into(),
                    artifact_id: row.artifact_id.clone().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_collab_threads(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_collab_status(collaboration_workspace::status_message(&err).into());
        }
    }
}

fn refresh_collab_tasks(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, room_id: &str) {
    match collaboration_workspace::refresh_tasks(&mut session.borrow_mut(), room_id) {
        Ok(rows) => {
            ui.set_collab_tasks(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                TaskRowItem {
                    id: row.id.clone().into(),
                    title: row.title.clone().into(),
                    status: row.status.clone().into(),
                    revision: row.revision.to_string().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_collab_tasks(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_collab_status(collaboration_workspace::status_message(&err).into());
        }
    }
}

fn select_collab_thread(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, thread_id: &str) {
    ui.set_collab_active_thread_id(thread_id.into());
    match collaboration_workspace::refresh_messages(&mut session.borrow_mut(), thread_id) {
        Ok(rows) => {
            ui.set_collab_messages(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                MessageRowItem {
                    author: row.author.clone().into(),
                    body: row.body.clone().into(),
                    seq: row.seq.to_string().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_collab_messages(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_collab_status(collaboration_workspace::status_message(&err).into());
        }
    }
}

fn medagent_run_row(row: &medagent_workspace::RunRowVm) -> AgentRunRowItem {
    AgentRunRowItem {
        id: row.id.clone().into(),
        status: row.status.clone().into(),
        revision: row.revision.to_string().into(),
        prompt: row.prompt.clone().into(),
    }
}

fn refresh_medagent_runs(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_medagent_status("Open a project to inspect its runs".into());
        ui.set_medagent_runs(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    match medagent_workspace::refresh_runs(&mut session.borrow_mut(), &project_id) {
        Ok(rows) => {
            ui.set_medagent_status(
                format!(
                    "{} run{} · Core-backed",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                )
                .into(),
            );
            ui.set_medagent_runs(ModelRc::new(VecModel::from_iter(
                rows.iter().map(medagent_run_row),
            )));
        }
        Err(err) => {
            ui.set_medagent_runs(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_medagent_status(medagent_workspace::status_message(&err).into());
        }
    }
}

fn refresh_medagent_turns(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, run_id: &str) {
    match medagent_workspace::refresh_turns(&mut session.borrow_mut(), run_id) {
        Ok(rows) => {
            ui.set_medagent_turns(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                AgentTurnRowItem {
                    seq: row.seq.to_string().into(),
                    kind: row.kind.clone().into(),
                    payload: row.payload.clone().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_medagent_turns(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_medagent_status(medagent_workspace::status_message(&err).into());
        }
    }
}

fn select_medagent_run(
    ui: &AppWindow,
    session: &Rc<RefCell<CliSession>>,
    run_id: &str,
    revision: &str,
) {
    ui.set_medagent_active_run_id(run_id.into());
    ui.set_medagent_active_run_revision(revision.into());
    refresh_medagent_turns(ui, session, run_id);
}

fn refresh_model_fleet(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_fleet_status("Open a project to inspect its fleet runs".into());
        ui.set_fleet_lanes(ModelRc::new(VecModel::from(Vec::new())));
        ui.set_fleet_runs(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    let lanes = model_fleet_workspace::refresh_lanes(&mut session.borrow_mut(), &project_id);
    let fleets = model_fleet_workspace::refresh_fleets(&mut session.borrow_mut(), &project_id);
    match (lanes, fleets) {
        (Ok(lanes), Ok(fleets)) => {
            ui.set_fleet_status(
                format!(
                    "{} lane{} · {} fleet run{} · Core-backed",
                    lanes.len(),
                    if lanes.len() == 1 { "" } else { "s" },
                    fleets.len(),
                    if fleets.len() == 1 { "" } else { "s" }
                )
                .into(),
            );
            ui.set_fleet_lanes(ModelRc::new(VecModel::from_iter(lanes.iter().map(|row| {
                FleetLaneRowItem {
                    id: row.id.clone().into(),
                    role: row.role_label.clone().into(),
                    status: row.status.clone().into(),
                    revision: row.revision.to_string().into(),
                }
            }))));
            ui.set_fleet_runs(ModelRc::new(VecModel::from_iter(fleets.iter().map(
                |row| FleetRunRowItem {
                    id: row.id.clone().into(),
                    status: row.status.clone().into(),
                    revision: row.revision.to_string().into(),
                },
            ))));
        }
        (Err(err), _) | (_, Err(err)) => {
            ui.set_fleet_lanes(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_fleet_runs(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_fleet_status(model_fleet_workspace::status_message(&err).into());
        }
    }
}

fn show_fleet_report(ui: &AppWindow, report: Option<&model_fleet_workspace::ReportVm>) {
    match report {
        Some(report) => {
            ui.set_fleet_report_summary(
                format!(
                    "Comparison {} · participating: {} · excluded: {}",
                    report.id,
                    report.participating,
                    if report.excluded.is_empty() {
                        "none"
                    } else {
                        report.excluded.as_str()
                    }
                )
                .into(),
            );
            ui.set_fleet_observations(ModelRc::new(VecModel::from_iter(
                report.observations.iter().map(|o| FleetObservationRowItem {
                    kind: o.kind.clone().into(),
                    lanes: o.lanes.clone().into(),
                    detail: o.detail.clone().into(),
                }),
            )));
        }
        None => {
            ui.set_fleet_report_summary("".into());
            ui.set_fleet_observations(ModelRc::new(VecModel::from(Vec::new())));
        }
    }
}

fn open_model_fleet(ui: &AppWindow, session: &Rc<RefCell<CliSession>>, fleet_id: &str) {
    let detail = model_fleet_workspace::open_fleet(&mut session.borrow_mut(), fleet_id);
    match detail {
        Ok(detail) => {
            ui.set_fleet_active_id(detail.fleet.id.clone().into());
            ui.set_fleet_active_status(detail.fleet.status.clone().into());
            ui.set_fleet_active_revision(detail.fleet.revision.to_string().into());
            ui.set_fleet_lane_runs(ModelRc::new(VecModel::from_iter(detail.lanes.iter().map(
                |row| FleetLaneRunRowItem {
                    lane_id: row.lane_id.clone().into(),
                    run_id: row.run_id.clone().into(),
                    run_status: row.run_status.clone().into(),
                },
            ))));
            let report = model_fleet_workspace::latest_report(&mut session.borrow_mut(), fleet_id);
            match report {
                Ok(report) => show_fleet_report(ui, report.as_ref()),
                Err(err) => {
                    show_fleet_report(ui, None);
                    ui.set_fleet_status(model_fleet_workspace::status_message(&err).into());
                }
            }
        }
        Err(err) => {
            ui.set_fleet_active_id("".into());
            ui.set_fleet_lane_runs(ModelRc::new(VecModel::from(Vec::new())));
            show_fleet_report(ui, None);
            ui.set_fleet_status(model_fleet_workspace::status_message(&err).into());
        }
    }
}

fn refresh_research_os(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_research_os_status("Open a project to see Research OS planes".into());
        ui.set_research_os_rows(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    let rows = research_os_workspace::rows(&mut session.borrow_mut(), &project_id);
    let unavailable = rows.iter().filter(|r| r.state == "unavailable").count();
    ui.set_research_os_status(
        format!(
            "{} planes · {} row{} · {} unavailable · read-only, Core-backed",
            research_os_workspace::PLANES.len(),
            rows.len(),
            if rows.len() == 1 { "" } else { "s" },
            unavailable
        )
        .into(),
    );
    ui.set_research_os_rows(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
        ResearchOsRowItem {
            plane: row.plane.clone().into(),
            id: row.id.clone().into(),
            state: row.state.clone().into(),
            detail: row.detail.clone().into(),
            action: row.action.clone().into(),
        }
    }))));
}

fn refresh_analytics(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_analytics_status("Open a project to use Analytics".into());
        ui.set_analytics_receipts(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    match analytics_workspace::receipts(&mut session.borrow_mut(), &project_id) {
        Ok(rows) => {
            ui.set_analytics_status(
                format!(
                    "{} query receipt{} · read-only engine · Core-backed",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                )
                .into(),
            );
            ui.set_analytics_receipts(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                AnalyticsReceiptRowItem {
                    id: row.id.clone().into(),
                    outcome: row.outcome.clone().into(),
                    detail: row.detail.clone().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_analytics_receipts(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_analytics_status(analytics_workspace::status_message(&err).into());
        }
    }
}

fn refresh_knowledge(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_knowledge_status("Open a project to use Knowledge".into());
        ui.set_knowledge_canvases(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    let mut s = session.borrow_mut();
    match knowledge_workspace::index_status(&mut s, &project_id) {
        Ok(line) => ui.set_knowledge_status(line.into()),
        Err(err) => {
            ui.set_knowledge_status(knowledge_workspace::status_message(&err).into());
            ui.set_knowledge_canvases(ModelRc::new(VecModel::from(Vec::new())));
            return;
        }
    }
    match knowledge_workspace::canvases(&mut s, &project_id) {
        Ok(rows) => {
            ui.set_knowledge_canvases(ModelRc::new(VecModel::from_iter(rows.iter().map(|row| {
                KnowledgeCanvasRowItem {
                    id: row.id.clone().into(),
                    title: row.title.clone().into(),
                    detail: row.detail.clone().into(),
                }
            }))));
        }
        Err(err) => {
            ui.set_knowledge_canvases(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_knowledge_status(knowledge_workspace::status_message(&err).into());
        }
    }
}

fn refresh_audio(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_audio_status("Open a project to use AudioFlow".into());
        ui.set_audio_sources(ModelRc::new(VecModel::from(Vec::new())));
        ui.set_audio_captures(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    match audio_workspace::refresh(&mut session.borrow_mut(), &project_id) {
        Ok(overview) => {
            ui.set_audio_status(
                format!(
                    "{} source{} · {} capture{} · {} · Core-backed, local only",
                    overview.sources.len(),
                    if overview.sources.len() == 1 { "" } else { "s" },
                    overview.captures.len(),
                    if overview.captures.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    overview.routes
                )
                .into(),
            );
            ui.set_audio_native_capture(
                format!(
                    "Live microphone capture unavailable: {}",
                    overview.native_capture
                )
                .into(),
            );
            ui.set_audio_sources(ModelRc::new(VecModel::from_iter(
                overview.sources.iter().map(|row| AudioSourceRowItem {
                    id: row.id.clone().into(),
                    label: row.label.clone().into(),
                    kind: row.kind.clone().into(),
                    detail: row.detail.clone().into(),
                    health: row.health.clone().into(),
                }),
            )));
            ui.set_audio_captures(ModelRc::new(VecModel::from_iter(
                overview.captures.iter().map(|row| AudioCaptureRowItem {
                    id: row.id.clone().into(),
                    label: row.label.clone().into(),
                    state: row.state.clone().into(),
                    detail: row.detail.clone().into(),
                }),
            )));
        }
        Err(err) => {
            ui.set_audio_sources(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_audio_captures(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_audio_status(audio_workspace::status_message(&err).into());
        }
    }
}

fn refresh_browse(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_browse_status("Open a project to use Governed Browse".into());
        ui.set_browse_allowlist(ModelRc::new(VecModel::from(Vec::new())));
        ui.set_browse_sessions(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    match browse_workspace::refresh(&mut session.borrow_mut(), &project_id) {
        Ok(overview) => {
            ui.set_browse_status(
                format!(
                    "{} allowed target{} · {} session{} · {} · Core-backed",
                    overview.allowlist.len(),
                    if overview.allowlist.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    overview.sessions.len(),
                    if overview.sessions.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    overview.routes
                )
                .into(),
            );
            ui.set_browse_allowlist(ModelRc::new(VecModel::from_iter(
                overview.allowlist.iter().map(|row| BrowseAllowRowItem {
                    id: row.id.clone().into(),
                    target: row.target.clone().into(),
                    enabled: row.enabled,
                    revision: row.revision.to_string().into(),
                }),
            )));
            ui.set_browse_sessions(ModelRc::new(VecModel::from_iter(
                overview.sessions.iter().map(|row| BrowseSessionRowItem {
                    id: row.id.clone().into(),
                    url: row.url.clone().into(),
                    state: row.state.clone().into(),
                    reason: row.reason.clone().into(),
                }),
            )));
        }
        Err(err) => {
            ui.set_browse_allowlist(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_browse_sessions(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_browse_status(browse_workspace::status_message(&err).into());
        }
    }
}

fn refresh_privacy(ui: &AppWindow, session: &Rc<RefCell<CliSession>>) {
    let project_id = ui.get_project_active_id().to_string();
    if project_id.is_empty() {
        ui.set_privacy_status("Open a project to inspect its privacy state".into());
        ui.set_privacy_classifications(ModelRc::new(VecModel::from(Vec::new())));
        ui.set_privacy_receipts(ModelRc::new(VecModel::from(Vec::new())));
        ui.set_privacy_decisions(ModelRc::new(VecModel::from(Vec::new())));
        return;
    }
    match privacy_workspace::refresh(&mut session.borrow_mut(), &project_id) {
        Ok(overview) => {
            ui.set_privacy_status(
                format!(
                    "{} classified · {} receipt{} · {} egress decision{} · unclassified artifacts are local_phi · Core-backed",
                    overview.classifications.len(),
                    overview.receipts.len(),
                    if overview.receipts.len() == 1 { "" } else { "s" },
                    overview.decisions.len(),
                    if overview.decisions.len() == 1 { "" } else { "s" }
                )
                .into(),
            );
            ui.set_privacy_classifications(ModelRc::new(VecModel::from_iter(
                overview
                    .classifications
                    .iter()
                    .map(|row| PrivacyClassificationRowItem {
                        artifact_id: row.artifact_id.clone().into(),
                        data_class: row.data_class.clone().into(),
                        basis: row.basis.clone().into(),
                        revision: row.revision.to_string().into(),
                    }),
            )));
            ui.set_privacy_receipts(ModelRc::new(VecModel::from_iter(
                overview.receipts.iter().map(|row| PrivacyReceiptRowItem {
                    id: row.id.clone().into(),
                    source: row.source.clone().into(),
                    output: row.output.clone().into(),
                    status: row.status.clone().into(),
                    residual: row.residual.clone().into(),
                    revision: row.revision.to_string().into(),
                }),
            )));
            ui.set_privacy_decisions(ModelRc::new(VecModel::from_iter(
                overview.decisions.iter().map(|row| PrivacyDecisionRowItem {
                    artifact_id: row.artifact_id.clone().into(),
                    boundary: row.boundary.clone().into(),
                    outcome: row.outcome.clone().into(),
                    reason: row.reason.clone().into(),
                }),
            )));
        }
        Err(err) => {
            ui.set_privacy_classifications(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_privacy_receipts(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_privacy_decisions(ModelRc::new(VecModel::from(Vec::new())));
            ui.set_privacy_status(privacy_workspace::status_message(&err).into());
        }
    }
}

fn validated_model_pack_path(raw: &str) -> Result<&str, &'static str> {
    let path = raw.trim();
    if path.is_empty() {
        return Err("Enter an absolute local Pack directory");
    }
    if !Path::new(path).is_absolute() {
        return Err("Local Pack directory must be an absolute path");
    }
    Ok(path)
}

fn perf_idle_ms(args: &[String]) -> Result<Option<u64>, &'static str> {
    let Some(index) = args.iter().position(|arg| arg == "--perf-idle-ms") else {
        return Ok(None);
    };
    let Some(raw) = args.get(index + 1) else {
        return Err("--perf-idle-ms requires a millisecond value");
    };
    let value = raw
        .parse::<u64>()
        .map_err(|_| "invalid --perf-idle-ms value")?;
    if !(50..=PERF_IDLE_MAX_MS).contains(&value) {
        return Err("--perf-idle-ms must be between 50 and 10000");
    }
    Ok(Some(value))
}

fn evidence_theme_override() -> Option<i32> {
    match env::var("MEDSCALE_EVIDENCE_THEME")
        .ok()?
        .to_ascii_lowercase()
        .as_str()
    {
        "light" => Some(1),
        "dark" => Some(2),
        _ => None,
    }
}

fn evidence_route_override() -> Option<&'static str> {
    let requested = env::var("MEDSCALE_EVIDENCE_ROUTE").ok()?;
    visual_evidence::ROUTES
        .into_iter()
        .find(|route| *route == requested)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let render = match visual_evidence::RenderOptions::parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let smoke = args.iter().any(|arg| arg == "--smoke");
    let idle_ms = match perf_idle_ms(&args) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    if let Some(ms) = idle_ms {
        black_box(&report);
        println!("medscale-desktop perf-idle-ready");
        let _ = io::stdout().flush();
        thread::sleep(Duration::from_millis(ms));
        black_box(&report);
        return ExitCode::SUCCESS;
    }

    if smoke {
        println!("medscale-desktop smoke ok");
        println!("version={}", CoreFacade::version());
        println!("tauri_admitted={}", report.tauri_admitted);
        println!("desktop_shell={}", report.desktop_shell);
        return ExitCode::SUCCESS;
    }

    let ui = match AppWindow::new() {
        Ok(ui) => ui,
        Err(err) => {
            eprintln!("failed to initialize MedScale Desktop UI: {err}");
            return ExitCode::from(1);
        }
    };
    ui.set_product_version(report.version.clone().into());
    if let Some(theme) = evidence_theme_override() {
        ui.global::<Theme>().set_evidence_theme(theme);
    }
    if let Some(route) = evidence_route_override() {
        ui.set_active_route(route.into());
    }
    if let Some(options) = &render {
        ui.global::<Theme>().set_evidence_theme(options.theme);
        ui.global::<Theme>().set_compact(options.compact);
        ui.set_active_route(options.route.clone().into());
        ui.window().set_size(slint::LogicalSize::new(
            options.width as f32,
            options.height as f32,
        ));
    }

    let patient = patient_workspace::PatientWorkspaceVm::synthetic_demo();
    ui.set_patient_name(patient.display_name.into());
    ui.set_patient_subject_ref(patient.subject_ref.into());
    ui.set_patient_birth_date(patient.birth_date.into());
    ui.set_patient_condition_summary(patient.condition_summary.into());
    ui.set_patient_coverage_summary(patient.coverage_summary.into());
    ui.set_patient_coverage_state(patient.coverage_state.into());
    ui.set_patient_medications_state(patient.medications_state.into());
    ui.set_patient_documents_state(patient.documents_state.into());
    ui.set_patient_care_plan_state(patient.care_plan_state.into());
    ui.set_patient_timeline(ModelRc::new(VecModel::from_iter(
        patient.timeline.into_iter().map(|row| PatientTimelineItem {
            date: row.date.into(),
            title: row.title.into(),
            detail: row.detail.into(),
            source: row.source.into(),
            status: row.status.into(),
        }),
    )));
    ui.set_patient_labs(ModelRc::new(VecModel::from_iter(
        patient.labs.into_iter().map(|row| PatientLabItem {
            label: row.label.into(),
            value: row.value.into(),
            unit: row.unit.into(),
            status: row.status.into(),
            source: row.source.into(),
        }),
    )));
    ui.set_patient_sources(ModelRc::new(VecModel::from_iter(
        patient.sources.into_iter().map(|row| PatientSourceItem {
            source_id: row.source_id.into(),
            note: row.note.into(),
        }),
    )));

    let insights = population_insights::PopulationInsightsVm::synthetic_demo();
    ui.set_insights_cohort_size(insights.cohort_size.to_string().into());
    ui.set_insights_review_attention(insights.review_attention_count.to_string().into());
    ui.set_insights_evidence_gaps(insights.evidence_gap_count.to_string().into());
    ui.set_insights_present_count(insights.present_count.to_string().into());
    ui.set_insights_risk_state(insights.risk_state.clone().into());
    ui.set_insights_trend_state(insights.trend_state.clone().into());
    ui.set_insights_gap_state(insights.gap_state.clone().into());
    ui.set_insights_recommendation_state(insights.recommendation_state.clone().into());
    ui.set_insights_assistant_answer(insights.assistant_default.clone().into());
    ui.set_insights_conditions(ModelRc::new(VecModel::from_iter(
        insights
            .condition_distribution
            .iter()
            .map(|row| PopulationDistributionItem {
                label: row.label.clone().into(),
                value: row.value.clone().into(),
                detail: row.detail.clone().into(),
                status: row.status.clone().into(),
            }),
    )));
    ui.set_insights_coverage(ModelRc::new(VecModel::from_iter(
        insights
            .coverage_distribution
            .iter()
            .map(|row| PopulationDistributionItem {
                label: row.label.clone().into(),
                value: row.value.clone().into(),
                detail: row.detail.clone().into(),
                status: row.status.clone().into(),
            }),
    )));
    ui.set_insights_cohorts(ModelRc::new(VecModel::from_iter(
        insights.cohorts.iter().map(|row| PopulationCohortItem {
            name: row.display_name.clone().into(),
            subject_ref: row.subject_ref.clone().into(),
            condition: row.condition.clone().into(),
            evidence_state: row.evidence_state.clone().into(),
            review_attention: row.review_attention.clone().into(),
        }),
    )));
    ui.set_insights_evidence(ModelRc::new(VecModel::from_iter(
        insights.evidence.iter().map(|row| AssistantEvidenceItem {
            source: row.source.clone().into(),
            title: row.title.clone().into(),
            snippet: row.snippet.clone().into(),
            metadata: row.metadata.clone().into(),
        }),
    )));

    let workflow = workflow_studio::WorkflowStudioVm::synthetic_demo();
    ui.set_workflow_name(workflow.workflow_name.clone().into());
    ui.set_workflow_state(workflow.workflow_state.clone().into());
    ui.set_workflow_outbox_summary(workflow.outbox_summary.clone().into());
    ui.set_workflow_action_boundary(workflow.action_boundary.clone().into());
    ui.set_workflow_task_boundary(workflow.task_boundary.clone().into());
    ui.set_workflow_message_boundary(workflow.message_boundary.clone().into());
    ui.set_workflow_review_detail(
        "Select a derived review task. No action is committed from this surface.".into(),
    );
    ui.set_workflow_steps(ModelRc::new(VecModel::from_iter(
        workflow.steps.iter().map(|row| WorkflowStepItem {
            order: row.order.clone().into(),
            title: row.title.clone().into(),
            detail: row.detail.clone().into(),
            state: row.state.clone().into(),
        }),
    )));
    ui.set_workflow_tasks(ModelRc::new(VecModel::from_iter(
        workflow.tasks.iter().map(|row| WorkflowTaskItem {
            action_id: row.action_id.clone().into(),
            title: row.title.clone().into(),
            state: row.state.clone().into(),
            payload_digest: row.payload_digest.clone().into(),
            next_step: row.next_step.clone().into(),
            reconcile_required: row.reconcile_required,
        }),
    )));
    ui.set_workflow_messages(ModelRc::new(VecModel::from_iter(
        workflow.messages.iter().map(|row| WorkflowMessageItem {
            title: row.title.clone().into(),
            body: row.body.clone().into(),
            status: row.status.clone().into(),
            related_action_id: row.related_action_id.clone().into(),
        }),
    )));

    let utilities = utility_surfaces::UtilitySurfacesVm::synthetic_demo(&report);
    ui.set_utility_audit_boundary(utilities.audit_boundary.clone().into());
    ui.set_utility_export_boundary(utilities.export_boundary.clone().into());
    ui.set_utility_settings_boundary(utilities.settings_boundary.clone().into());
    ui.set_utility_integrations_boundary(utilities.integrations_boundary.clone().into());
    ui.set_utility_missing_release_evidence(utilities.missing_release_evidence.clone().into());
    ui.set_utility_audit_rows(ModelRc::new(VecModel::from_iter(
        utilities.audit_rows.iter().map(|row| AuditTrailItem {
            record_id: row.record_id.clone().into(),
            action: row.action.clone().into(),
            subject: row.subject.clone().into(),
            detail: row.detail.clone().into(),
            status: row.status.clone().into(),
        }),
    )));
    ui.set_utility_export_rows(ModelRc::new(VecModel::from_iter(
        utilities.export_rows.iter().map(|row| UtilityStatusItem {
            label: row.label.clone().into(),
            value: row.value.clone().into(),
            detail: row.detail.clone().into(),
            status: row.status.clone().into(),
        }),
    )));
    ui.set_utility_settings_rows(ModelRc::new(VecModel::from_iter(
        utilities.settings_rows.iter().map(|row| UtilityStatusItem {
            label: row.label.clone().into(),
            value: row.value.clone().into(),
            detail: row.detail.clone().into(),
            status: row.status.clone().into(),
        }),
    )));
    ui.set_utility_integration_rows(ModelRc::new(VecModel::from_iter(
        utilities
            .integration_rows
            .iter()
            .map(|row| UtilityStatusItem {
                label: row.label.clone().into(),
                value: row.value.clone().into(),
                detail: row.detail.clone().into(),
                status: row.status.clone().into(),
            }),
    )));

    let model_session = match CliSession::connect_pack_operator("desktop-model-center") {
        Ok(session) => {
            ui.set_model_operator_status("Core session ready · local admission only".into());
            Some(Rc::new(RefCell::new(session)))
        }
        Err(err) => {
            ui.set_model_operator_status(format!("Core session unavailable: {err:?}").into());
            None
        }
    };
    let product_intelligence = if let Some(session) = &model_session {
        match product_intelligence::ProductIntelligenceVm::from_session(&mut session.borrow_mut()) {
            Ok(vm) => vm,
            Err(err) => {
                ui.set_model_operator_status(
                    format!("Core inventory unavailable at startup: {err:?}").into(),
                );
                product_intelligence::ProductIntelligenceVm::current_truth()
            }
        }
    } else {
        product_intelligence::ProductIntelligenceVm::current_truth()
    };
    apply_product_intelligence(&ui, &product_intelligence);

    // Spec 074 Projects workspace: operator session plus a local synthetic
    // vault at the platform default root. Failures stay visible as status.
    let project_session = match CliSession::connect("desktop-projects") {
        Ok(mut session) => {
            let root = render.as_ref().map_or_else(
                || CliSession::default_vault_root("desktop-projects"),
                visual_evidence::RenderOptions::synthetic_vault_root,
            );
            match session.open_synthetic_vault(&root.display().to_string()) {
                Ok(()) => {
                    let session = Rc::new(RefCell::new(session));
                    refresh_project_list(&ui, &session);
                    Some(session)
                }
                Err(err) => {
                    ui.set_project_status(format!("Projects unavailable: {err:?}").into());
                    None
                }
            }
        }
        Err(err) => {
            ui.set_project_status(format!("Projects unavailable: {err:?}").into());
            None
        }
    };
    if render.is_some() && project_session.is_none() {
        eprintln!("native evidence capture requires an isolated synthetic Core vault");
        return ExitCode::from(1);
    }

    // Spec 061 keeps patient presentation read-only and routes consequential work to review surfaces.
    // Consequential operations are routed to their owning review surfaces; no action is committed here.
    let weak = ui.as_weak();
    let insights_for_actions = insights.clone();
    let workflow_for_actions = workflow.clone();
    let model_session_for_actions = model_session.clone();
    let project_session_for_actions = project_session.clone();
    ui.on_ui_action(move |action| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let action = action.as_str();
        // Spec 074 project workspace actions (Core-backed, revision-guarded).
        if action == "projects-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_project_list(&ui, session);
                let active = ui.get_project_active_id().to_string();
                if !active.is_empty() {
                    open_project_detail(&ui, session, &active);
                }
            } else {
                ui.set_project_status("Projects unavailable: no Core session".into());
            }
            return;
        }
        if action == "projects-create" {
            let Some(session) = &project_session_for_actions else {
                ui.set_project_status("Projects unavailable: no Core session".into());
                return;
            };
            let raw_name = ui.get_project_name_input().to_string();
            let raw_desc = ui.get_project_desc_input().to_string();
            let name = match project_workspace::validate_name(&raw_name) {
                Ok(name) => name,
                Err(message) => {
                    ui.set_project_status(message.into());
                    return;
                }
            };
            let description = if raw_desc.trim().is_empty() {
                None
            } else {
                Some(raw_desc.trim().to_owned())
            };
            match project_workspace::create_project(&mut session.borrow_mut(), name, description) {
                Ok(row) => {
                    ui.set_project_name_input("".into());
                    ui.set_project_desc_input("".into());
                    refresh_project_list(&ui, session);
                    open_project_detail(&ui, session, &row.id);
                }
                Err(err) => ui.set_project_status(project_workspace::status_message(&err).into()),
            }
            return;
        }
        if let Some(project_id) = action.strip_prefix("projects-open:") {
            if let Some(session) = &project_session_for_actions {
                open_project_detail(&ui, session, project_id);
            } else {
                ui.set_project_status("Projects unavailable: no Core session".into());
            }
            return;
        }
        if action == "projects-archive" {
            let Some(session) = &project_session_for_actions else {
                ui.set_project_status("Projects unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            let revision = ui.get_project_active_revision().to_string().parse::<u64>();
            let Ok(revision) = revision else {
                ui.set_project_status("Invalid: open project has no revision".into());
                return;
            };
            match project_workspace::archive_project(&mut session.borrow_mut(), &project_id, revision) {
                Ok(_) => {
                    refresh_project_list(&ui, session);
                    open_project_detail(&ui, session, &project_id);
                }
                Err(err) => ui.set_project_status(project_workspace::status_message(&err).into()),
            }
            return;
        }
        if action == "projects-exp-create" {
            let Some(session) = &project_session_for_actions else {
                ui.set_project_status("Projects unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            if project_id.is_empty() {
                ui.set_project_status("Invalid: open a project first".into());
                return;
            }
            let raw_name = ui.get_project_experiment_input().to_string();
            let name = match project_workspace::validate_name(&raw_name) {
                Ok(name) => name,
                Err(message) => {
                    ui.set_project_status(message.into());
                    return;
                }
            };
            match project_workspace::create_experiment(&mut session.borrow_mut(), &project_id, name) {
                Ok(_) => {
                    ui.set_project_experiment_input("".into());
                    open_project_detail(&ui, session, &project_id);
                }
                Err(err) => ui.set_project_status(project_workspace::status_message(&err).into()),
            }
            return;
        }
        // Spec 075 data workbench actions (Core-backed snapshots and views).
        if action == "data-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_data_sources(&ui, session);
                let active = ui.get_data_active_source().to_string();
                if !active.is_empty() {
                    open_data_source(&ui, session, &active);
                }
            } else {
                ui.set_data_status("Data unavailable: no Core session".into());
            }
            return;
        }
        if let Some(source_id) = action.strip_prefix("data-open:") {
            if let Some(session) = &project_session_for_actions {
                open_data_source(&ui, session, source_id);
            } else {
                ui.set_data_status("Data unavailable: no Core session".into());
            }
            return;
        }
        if let Some(snapshot_id) = action.strip_prefix("data-snapshot:") {
            if let Some(session) = &project_session_for_actions {
                open_data_snapshot(&ui, session, snapshot_id);
            } else {
                ui.set_data_status("Data unavailable: no Core session".into());
            }
            return;
        }
        if action == "data-import" {
            let Some(session) = &project_session_for_actions else {
                ui.set_data_status("Data unavailable: no Core session".into());
                return;
            };
            let source_id = ui.get_data_active_source().to_string();
            if source_id.is_empty() {
                ui.set_data_status("Invalid: open a data source first".into());
                return;
            }
            match data_workbench::import_snapshot(&mut session.borrow_mut(), &source_id) {
                Ok(row) => {
                    refresh_data_sources(&ui, session);
                    open_data_source(&ui, session, &source_id);
                    ui.set_data_status(format!("Imported snapshot {}", row.id).into());
                }
                Err(err) => ui.set_data_status(data_workbench::status_message(&err).into()),
            }
            return;
        }
        if action == "data-refresh-source" || action == "data-refresh-source-force" {
            let Some(session) = &project_session_for_actions else {
                ui.set_data_status("Data unavailable: no Core session".into());
                return;
            };
            let source_id = ui.get_data_active_source().to_string();
            if source_id.is_empty() {
                ui.set_data_status("Invalid: open a data source first".into());
                return;
            }
            let allow = action == "data-refresh-source-force";
            match data_workbench::refresh_source(&mut session.borrow_mut(), &source_id, allow) {
                Ok(summary) => {
                    refresh_data_sources(&ui, session);
                    open_data_source(&ui, session, &source_id);
                    ui.set_data_status(summary.into());
                }
                Err(err) => ui.set_data_status(data_workbench::status_message(&err).into()),
            }
            return;
        }
        if action == "data-view-grid" {
            let Some(session) = &project_session_for_actions else {
                ui.set_data_status("Data unavailable: no Core session".into());
                return;
            };
            let snapshot_id = ui.get_data_active_snapshot().to_string();
            if snapshot_id.is_empty() {
                ui.set_data_status("Invalid: open a snapshot first".into());
                return;
            }
            match data_workbench::create_grid_view(&mut session.borrow_mut(), &snapshot_id) {
                Ok(view_id) => {
                    ui.set_data_status(format!("Saved grid view {view_id}").into());
                }
                Err(err) => ui.set_data_status(data_workbench::status_message(&err).into()),
            }
            return;
        }
        if action == "data-transform-apply" {
            let Some(session) = &project_session_for_actions else {
                ui.set_data_status("Data unavailable: no Core session".into());
                return;
            };
            let snapshot_id = ui.get_data_active_snapshot().to_string();
            if snapshot_id.is_empty() {
                ui.set_data_status("Invalid: open a snapshot first".into());
                return;
            }
            let columns_raw = ui.get_data_columns_input().to_string();
            let columns = if columns_raw.trim().is_empty() {
                None
            } else {
                Some(
                    columns_raw
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                )
            };
            let filter = match data_workbench::parse_filter_input(
                &ui.get_data_filter_col(),
                &ui.get_data_filter_op(),
                &ui.get_data_filter_value(),
            ) {
                Ok(filter) => filter,
                Err(message) => {
                    ui.set_data_status(message.into());
                    return;
                }
            };
            let sort = data_workbench::parse_sort_input(&ui.get_data_sort_input());
            match data_workbench::apply_workbench_transform(
                &mut session.borrow_mut(),
                &snapshot_id,
                columns,
                filter,
                sort,
            ) {
                Ok(summary) => {
                    open_data_source(&ui, session, &ui.get_data_active_source());
                    ui.set_data_status(summary.into());
                }
                Err(err) => ui.set_data_status(data_workbench::status_message(&err).into()),
            }
            return;
        }
        // Spec 076 collaboration substrate actions (Core-backed, membership-gated).
        if action == "collab-rooms-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_collab_rooms(&ui, session);
            } else {
                ui.set_collab_status("Rooms unavailable: no Core session".into());
            }
            return;
        }
        if action == "collab-room-create" {
            let Some(session) = &project_session_for_actions else {
                ui.set_collab_status("Rooms unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            if project_id.is_empty() {
                ui.set_collab_status("Invalid: open a project first".into());
                return;
            }
            let name = ui.get_collab_room_name_input().to_string();
            if name.trim().is_empty() {
                ui.set_collab_status("Invalid: room name is empty".into());
                return;
            }
            match collaboration_workspace::create_room(&mut session.borrow_mut(), &project_id, name)
            {
                Ok(_) => {
                    ui.set_collab_room_name_input("".into());
                    refresh_collab_rooms(&ui, session);
                }
                Err(err) => {
                    ui.set_collab_status(collaboration_workspace::status_message(&err).into());
                }
            }
            return;
        }
        if let Some(room_id) = action.strip_prefix("collab-room-open:") {
            if let Some(session) = &project_session_for_actions {
                open_collab_room(&ui, session, room_id);
            } else {
                ui.set_collab_status("Rooms unavailable: no Core session".into());
            }
            return;
        }
        if action == "collab-thread-open" {
            let Some(session) = &project_session_for_actions else {
                ui.set_collab_status("Threads unavailable: no Core session".into());
                return;
            };
            let room_id = ui.get_collab_active_room_id().to_string();
            let artifact_id = ui.get_collab_thread_artifact_input().to_string();
            if room_id.is_empty() || artifact_id.trim().is_empty() {
                ui.set_collab_status("Invalid: open a room and enter an artifact id".into());
                return;
            }
            match collaboration_workspace::open_thread(
                &mut session.borrow_mut(),
                &room_id,
                artifact_id.trim(),
            ) {
                Ok(_) => {
                    ui.set_collab_thread_artifact_input("".into());
                    refresh_collab_threads(&ui, session, &room_id);
                }
                Err(err) => {
                    ui.set_collab_status(collaboration_workspace::status_message(&err).into());
                }
            }
            return;
        }
        if let Some(thread_id) = action.strip_prefix("collab-thread-select:") {
            if let Some(session) = &project_session_for_actions {
                select_collab_thread(&ui, session, thread_id);
            } else {
                ui.set_collab_status("Threads unavailable: no Core session".into());
            }
            return;
        }
        if action == "collab-message-post" {
            let Some(session) = &project_session_for_actions else {
                ui.set_collab_status("Messages unavailable: no Core session".into());
                return;
            };
            let thread_id = ui.get_collab_active_thread_id().to_string();
            let body = ui.get_collab_message_input().to_string();
            if thread_id.is_empty() || body.trim().is_empty() {
                ui.set_collab_status("Invalid: open a thread and enter a message".into());
                return;
            }
            match collaboration_workspace::post_message(&mut session.borrow_mut(), &thread_id, body)
            {
                Ok(()) => {
                    ui.set_collab_message_input("".into());
                    select_collab_thread(&ui, session, &thread_id);
                }
                Err(err) => {
                    ui.set_collab_status(collaboration_workspace::status_message(&err).into());
                }
            }
            return;
        }
        if action == "collab-task-create" {
            let Some(session) = &project_session_for_actions else {
                ui.set_collab_status("Tasks unavailable: no Core session".into());
                return;
            };
            let room_id = ui.get_collab_active_room_id().to_string();
            let title = ui.get_collab_task_title_input().to_string();
            if room_id.is_empty() || title.trim().is_empty() {
                ui.set_collab_status("Invalid: open a room and enter a task title".into());
                return;
            }
            match collaboration_workspace::create_task(&mut session.borrow_mut(), &room_id, title) {
                Ok(_) => {
                    ui.set_collab_task_title_input("".into());
                    refresh_collab_tasks(&ui, session, &room_id);
                }
                Err(err) => {
                    ui.set_collab_status(collaboration_workspace::status_message(&err).into());
                }
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("collab-task-complete:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_collab_status("Tasks unavailable: no Core session".into());
                return;
            };
            let Some((task_id, revision_str)) = rest.rsplit_once(':') else {
                ui.set_collab_status("Invalid: malformed task action".into());
                return;
            };
            let Ok(expected_revision) = revision_str.parse::<u64>() else {
                ui.set_collab_status("Invalid: task has no revision".into());
                return;
            };
            let room_id = ui.get_collab_active_room_id().to_string();
            match collaboration_workspace::complete_task(
                &mut session.borrow_mut(),
                task_id,
                expected_revision,
            ) {
                Ok(_) => refresh_collab_tasks(&ui, session, &room_id),
                Err(err) => {
                    ui.set_collab_status(collaboration_workspace::status_message(&err).into());
                }
            }
            return;
        }
        // Spec 077 MedAgent Workbench actions (Core-backed; identity
        // registration and context-manifest creation remain CLI-only).
        if action == "medagent-runs-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_medagent_runs(&ui, session);
            } else {
                ui.set_medagent_status("Runs unavailable: no Core session".into());
            }
            return;
        }
        if action == "medagent-run-create-start" {
            let Some(session) = &project_session_for_actions else {
                ui.set_medagent_status("Runs unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            if project_id.is_empty() {
                ui.set_medagent_status("Invalid: open a project first".into());
                return;
            }
            let agent_id = ui.get_medagent_agent_id_input().to_string();
            let context_id = ui.get_medagent_context_id_input().to_string();
            let prompt = ui.get_medagent_prompt_input().to_string();
            if agent_id.trim().is_empty() || context_id.trim().is_empty() || prompt.trim().is_empty()
            {
                ui.set_medagent_status(
                    "Invalid: agent id, context id, and prompt are all required".into(),
                );
                return;
            }
            let created = medagent_workspace::create_run(
                &mut session.borrow_mut(),
                &project_id,
                agent_id.trim(),
                context_id.trim(),
                prompt,
            );
            match created {
                Ok(run) => {
                    match medagent_workspace::start_run(
                        &mut session.borrow_mut(),
                        &run.id,
                        run.revision,
                    ) {
                        Ok(_) => {
                            ui.set_medagent_prompt_input("".into());
                            refresh_medagent_runs(&ui, session);
                        }
                        Err(err) => {
                            ui.set_medagent_status(medagent_workspace::status_message(&err).into());
                            refresh_medagent_runs(&ui, session);
                        }
                    }
                }
                Err(err) => {
                    ui.set_medagent_status(medagent_workspace::status_message(&err).into());
                }
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("medagent-run-select:") {
            if let Some(session) = &project_session_for_actions {
                if let Some((run_id, revision)) = rest.rsplit_once(':') {
                    select_medagent_run(&ui, session, run_id, revision);
                } else {
                    ui.set_medagent_status("Invalid: malformed run selection".into());
                }
            } else {
                ui.set_medagent_status("Runs unavailable: no Core session".into());
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("medagent-run-cancel:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_medagent_status("Runs unavailable: no Core session".into());
                return;
            };
            let Some((run_id, revision_str)) = rest.rsplit_once(':') else {
                ui.set_medagent_status("Invalid: malformed cancel action".into());
                return;
            };
            let Ok(expected_revision) = revision_str.parse::<u64>() else {
                ui.set_medagent_status("Invalid: run has no revision".into());
                return;
            };
            match medagent_workspace::cancel_run(&mut session.borrow_mut(), run_id, expected_revision)
            {
                Ok(_) => refresh_medagent_runs(&ui, session),
                Err(err) => {
                    ui.set_medagent_status(medagent_workspace::status_message(&err).into());
                }
            }
            return;
        }
        // Spec 078 Model Fleet actions (Core-backed; lane creation stays
        // CLI-only, like Spec 077's identity/context creation).
        if action == "fleet-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_model_fleet(&ui, session);
            } else {
                ui.set_fleet_status("Fleet runs unavailable: no Core session".into());
            }
            return;
        }
        if action == "fleet-create" {
            let Some(session) = &project_session_for_actions else {
                ui.set_fleet_status("Fleet runs unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            let prompt = ui.get_fleet_prompt_input().to_string();
            if project_id.is_empty() || prompt.trim().is_empty() {
                ui.set_fleet_status("Invalid: open a project and enter a task prompt".into());
                return;
            }
            let created =
                model_fleet_workspace::create_fleet(&mut session.borrow_mut(), &project_id, prompt);
            match created {
                Ok(_) => {
                    ui.set_fleet_prompt_input("".into());
                    refresh_model_fleet(&ui, session);
                }
                Err(err) => ui.set_fleet_status(model_fleet_workspace::status_message(&err).into()),
            }
            return;
        }
        if let Some(fleet_id) = action.strip_prefix("fleet-open:") {
            if let Some(session) = &project_session_for_actions {
                open_model_fleet(&ui, session, fleet_id);
            } else {
                ui.set_fleet_status("Fleet runs unavailable: no Core session".into());
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("fleet-dispatch:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_fleet_status("Fleet runs unavailable: no Core session".into());
                return;
            };
            let Some((fleet_id, revision)) = rest.rsplit_once(':') else {
                ui.set_fleet_status("Invalid: malformed dispatch action".into());
                return;
            };
            let Ok(expected_revision) = revision.parse::<u64>() else {
                ui.set_fleet_status("Invalid: fleet run has no revision".into());
                return;
            };
            let raw_pack_dir = ui.get_fleet_pack_dir_input().to_string();
            let pack_dir = match validated_model_pack_path(&raw_pack_dir) {
                Ok(path) => path,
                Err(message) => {
                    ui.set_fleet_status(format!("Invalid: {message}").into());
                    return;
                }
            };
            let lane_ids = ui.get_fleet_lane_ids_input().to_string();
            let dispatched = model_fleet_workspace::dispatch_fleet(
                &mut session.borrow_mut(),
                fleet_id,
                expected_revision,
                &lane_ids,
                pack_dir,
            );
            match dispatched {
                Ok(_) => {
                    refresh_model_fleet(&ui, session);
                    open_model_fleet(&ui, session, fleet_id);
                }
                Err(err) => ui.set_fleet_status(model_fleet_workspace::status_message(&err).into()),
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("fleet-execute:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_fleet_status("Fleet runs unavailable: no Core session".into());
                return;
            };
            let Some((fleet_id, lane_id)) = rest.split_once(':') else {
                ui.set_fleet_status("Invalid: malformed execute action".into());
                return;
            };
            let raw_pack_dir = ui.get_fleet_pack_dir_input().to_string();
            let pack_dir = match validated_model_pack_path(&raw_pack_dir) {
                Ok(path) => path,
                Err(message) => {
                    ui.set_fleet_status(format!("Invalid: {message}").into());
                    return;
                }
            };
            let executed = model_fleet_workspace::execute_lane(
                &mut session.borrow_mut(),
                fleet_id,
                lane_id,
                pack_dir,
            );
            match executed {
                Ok(_) => {
                    refresh_model_fleet(&ui, session);
                    open_model_fleet(&ui, session, fleet_id);
                }
                Err(err) => ui.set_fleet_status(model_fleet_workspace::status_message(&err).into()),
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("fleet-cancel:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_fleet_status("Fleet runs unavailable: no Core session".into());
                return;
            };
            let Some((fleet_id, revision)) = rest.rsplit_once(':') else {
                ui.set_fleet_status("Invalid: malformed cancel action".into());
                return;
            };
            let Ok(expected_revision) = revision.parse::<u64>() else {
                ui.set_fleet_status("Invalid: fleet run has no revision".into());
                return;
            };
            let cancelled = model_fleet_workspace::cancel_fleet(
                &mut session.borrow_mut(),
                fleet_id,
                expected_revision,
            );
            match cancelled {
                Ok(_) => refresh_model_fleet(&ui, session),
                Err(err) => ui.set_fleet_status(model_fleet_workspace::status_message(&err).into()),
            }
            return;
        }
        if let Some(fleet_id) = action.strip_prefix("fleet-compare:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_fleet_status("Fleet runs unavailable: no Core session".into());
                return;
            };
            let compared = model_fleet_workspace::compare_fleet(&mut session.borrow_mut(), fleet_id);
            match compared {
                Ok(report) => show_fleet_report(&ui, Some(&report)),
                Err(err) => ui.set_fleet_status(model_fleet_workspace::status_message(&err).into()),
            }
            return;
        }
        // Spec 079 Privacy Gate actions (Core-backed; profiles, transforms
        // and re-identification stay CLI-only in this slice).
        // Spec 083 Knowledge actions (Core-backed; lexical index is a
        // projection, every search leaves a receipt).
        if action == "knowledge-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_knowledge(&ui, session);
            } else {
                ui.set_knowledge_status("Knowledge unavailable: no Core session".into());
            }
            return;
        }
        if action == "knowledge-build" || action == "knowledge-search" {
            let Some(session) = &project_session_for_actions else {
                ui.set_knowledge_status("Knowledge unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            if project_id.is_empty() {
                ui.set_knowledge_status("Invalid: open a project first".into());
                return;
            }
            if action == "knowledge-build" {
                let built = knowledge_workspace::build_index(&mut session.borrow_mut(), &project_id);
                if let Err(err) = built {
                    ui.set_knowledge_status(knowledge_workspace::status_message(&err).into());
                    return;
                }
            } else {
                let query = ui.get_knowledge_query_input().to_string();
                let found = knowledge_workspace::search(&mut session.borrow_mut(), &project_id, &query);
                match found {
                    Ok(result) => {
                        ui.set_knowledge_last_result(result.summary.into());
                        ui.set_knowledge_hits(result.hits.into());
                    }
                    Err(err) => {
                        ui.set_knowledge_status(knowledge_workspace::status_message(&err).into());
                        return;
                    }
                }
            }
            refresh_knowledge(&ui, session);
            return;
        }
        // Spec 082 Analytics actions (Core-backed; read-only engine).
        if action == "analytics-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_analytics(&ui, session);
            } else {
                ui.set_analytics_status("Analytics unavailable: no Core session".into());
            }
            return;
        }
        // Spec 094 Research OS row actions (reversible; Core-backed).
        if let Some(rest) = action.strip_prefix("research-os-act|") {
            let Some(session) = &project_session_for_actions else {
                ui.set_research_os_status("Research OS unavailable: no Core session".into());
                return;
            };
            let mut parts = rest.splitn(3, '|');
            let (Some(plane), Some(verb), Some(id)) = (parts.next(), parts.next(), parts.next())
            else {
                ui.set_research_os_status("Research OS action not understood; nothing changed".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            let outcome = research_os_workspace::act(
                &mut session.borrow_mut(),
                &project_id,
                plane,
                id,
                verb,
            );
            refresh_research_os(&ui, session);
            ui.set_research_os_status(
                match outcome {
                    Ok(state) => format!("{plane} {id}: {verb} done, now {state} (receipt recorded by Core)"),
                    Err(reason) => format!("{plane} {id}: {verb} not done: {reason}"),
                }
                .into(),
            );
            return;
        }
        // Spec 093 Research OS operations (Core-backed).
        if action == "research-os-refresh" {
            let Some(session) = &project_session_for_actions else {
                ui.set_research_os_status("Research OS unavailable: no Core session".into());
                return;
            };
            refresh_research_os(&ui, session);
            return;
        }
        if action == "analytics-run" {
            let Some(session) = &project_session_for_actions else {
                ui.set_analytics_status("Analytics unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            let sql = ui.get_analytics_sql_input().to_string();
            let bindings = ui.get_analytics_bindings_input().to_string();
            if project_id.is_empty() || sql.trim().is_empty() {
                ui.set_analytics_status("Invalid: open a project and enter a query".into());
                return;
            }
            let ran =
                analytics_workspace::run_query(&mut session.borrow_mut(), &project_id, &sql, &bindings);
            match ran {
                Ok(result) => {
                    ui.set_analytics_last_result(result.summary.into());
                    ui.set_analytics_preview(result.preview.into());
                    refresh_analytics(&ui, session);
                }
                Err(err) => ui.set_analytics_status(analytics_workspace::status_message(&err).into()),
            }
            return;
        }
        // Spec 081 AudioFlow actions (Core-backed; local only).
        if action == "audio-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_audio(&ui, session);
            } else {
                ui.set_audio_status("AudioFlow unavailable: no Core session".into());
            }
            return;
        }
        if let Some(source_id) = action.strip_prefix("audio-segment:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_audio_status("AudioFlow unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            if project_id.is_empty() {
                ui.set_audio_status("Invalid: open a project first".into());
                return;
            }
            let segmented =
                audio_workspace::segment(&mut session.borrow_mut(), &project_id, source_id);
            match segmented {
                Ok((summary, lines)) => {
                    ui.set_audio_last_result(summary.into());
                    ui.set_audio_transcript(lines.join("\n").into());
                    refresh_audio(&ui, session);
                }
                Err(err) => ui.set_audio_status(audio_workspace::status_message(&err).into()),
            }
            return;
        }
        // Spec 080 Governed Browse actions (Core-backed; the transport and
        // every policy check stay in Core).
        if action == "browse-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_browse(&ui, session);
            } else {
                ui.set_browse_status("Browse unavailable: no Core session".into());
            }
            return;
        }
        if action == "browse-allow-add" {
            let Some(session) = &project_session_for_actions else {
                ui.set_browse_status("Browse unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            if project_id.is_empty() {
                ui.set_browse_status("Invalid: open a project first".into());
                return;
            }
            let host = ui.get_browse_host_input().to_string();
            let prefix = ui.get_browse_prefix_input().to_string();
            let added = browse_workspace::add_allowed_host(
                &mut session.borrow_mut(),
                &project_id,
                &host,
                &prefix,
            );
            match added {
                Ok(_) => {
                    ui.set_browse_host_input("".into());
                    refresh_browse(&ui, session);
                }
                Err(err) => ui.set_browse_status(browse_workspace::status_message(&err).into()),
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("browse-allow-disable:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_browse_status("Browse unavailable: no Core session".into());
                return;
            };
            let Some((entry_id, revision)) = rest.rsplit_once(':') else {
                ui.set_browse_status("Invalid: malformed disable action".into());
                return;
            };
            let Ok(expected_revision) = revision.parse::<u64>() else {
                ui.set_browse_status("Invalid: entry has no revision".into());
                return;
            };
            let disabled = browse_workspace::disable_allowed_host(
                &mut session.borrow_mut(),
                entry_id,
                expected_revision,
            );
            match disabled {
                Ok(()) => refresh_browse(&ui, session),
                Err(err) => ui.set_browse_status(browse_workspace::status_message(&err).into()),
            }
            return;
        }
        if action == "browse-fetch" {
            let Some(session) = &project_session_for_actions else {
                ui.set_browse_status("Browse unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            let url = ui.get_browse_url_input().to_string();
            if project_id.is_empty() || url.trim().is_empty() {
                ui.set_browse_status("Invalid: open a project and enter an https URL".into());
                return;
            }
            let fetched = browse_workspace::fetch(&mut session.borrow_mut(), &project_id, &url);
            match fetched {
                Ok(result) => {
                    ui.set_browse_last_result(
                        format!(
                            "{} · {}{}{}",
                            result.session.state,
                            result.session.url,
                            if result.session.reason.is_empty() {
                                String::new()
                            } else {
                                format!(" · {}", result.session.reason)
                            },
                            if result.flagged {
                                " · instruction-like text flagged (ignored)"
                            } else {
                                ""
                            }
                        )
                        .into(),
                    );
                    ui.set_browse_last_excerpt(result.excerpt.into());
                    refresh_browse(&ui, session);
                }
                Err(err) => ui.set_browse_status(browse_workspace::status_message(&err).into()),
            }
            return;
        }
        if action == "privacy-refresh" {
            if let Some(session) = &project_session_for_actions {
                refresh_privacy(&ui, session);
            } else {
                ui.set_privacy_status("Privacy state unavailable: no Core session".into());
            }
            return;
        }
        if action == "privacy-egress-check" {
            let Some(session) = &project_session_for_actions else {
                ui.set_privacy_status("Privacy state unavailable: no Core session".into());
                return;
            };
            let project_id = ui.get_project_active_id().to_string();
            if project_id.is_empty() {
                ui.set_privacy_status("Invalid: open a project first".into());
                return;
            }
            let artifact = ui.get_privacy_artifact_input().to_string();
            let boundary = ui.get_privacy_boundary_input().to_string();
            let checked = privacy_workspace::check_egress(
                &mut session.borrow_mut(),
                &project_id,
                &artifact,
                &boundary,
            );
            match checked {
                Ok(row) => {
                    refresh_privacy(&ui, session);
                    ui.set_privacy_last_decision(
                        format!("{} → {}: {} ({})", row.artifact_id, row.boundary, row.outcome, row.reason).into(),
                    );
                }
                Err(err) => ui.set_privacy_status(privacy_workspace::status_message(&err).into()),
            }
            return;
        }
        if let Some(rest) = action.strip_prefix("privacy-receipt-revoke:") {
            let Some(session) = &project_session_for_actions else {
                ui.set_privacy_status("Privacy state unavailable: no Core session".into());
                return;
            };
            let Some((receipt_id, revision)) = rest.rsplit_once(':') else {
                ui.set_privacy_status("Invalid: malformed revoke action".into());
                return;
            };
            let Ok(expected_revision) = revision.parse::<u64>() else {
                ui.set_privacy_status("Invalid: receipt has no revision".into());
                return;
            };
            let revoked = privacy_workspace::revoke_receipt(
                &mut session.borrow_mut(),
                receipt_id,
                expected_revision,
            );
            match revoked {
                Ok(_) => refresh_privacy(&ui, session),
                Err(err) => ui.set_privacy_status(privacy_workspace::status_message(&err).into()),
            }
            return;
        }
        if action == "model-refresh" {            if let Some(session) = &model_session_for_actions {
                match refresh_model_center(&ui, session) {
                    Ok(()) => ui.set_model_operator_status("Core inventory refreshed".into()),
                    Err(err) => ui.set_model_operator_status(
                        format!("Core inventory refresh failed: {err:?}").into(),
                    ),
                }
            } else {
                ui.set_model_operator_status(
                    "Core session unavailable; inventory not refreshed".into(),
                );
            }
            return;
        }
        if action == "model-admit" {
            let Some(session) = &model_session_for_actions else {
                ui.set_model_operator_status(
                    "Core session unavailable; Pack admission denied".into(),
                );
                return;
            };
            let raw_path = ui.get_model_pack_path().to_string();
            let path = match validated_model_pack_path(&raw_path) {
                Ok(path) => path,
                Err(message) => {
                    ui.set_model_operator_status(message.into());
                    return;
                }
            };
            let admission = admit_model_pack(session, path);
            match admission {
                Ok(result) if result.admitted => {
                    let pack_id = result
                        .pack_id
                        .as_ref()
                        .map_or("unknown", medscale_contracts::objects::OpaqueId::as_str);
                    ui.set_model_pack_path("".into());
                    match refresh_model_center(&ui, session) {
                        Ok(()) => ui.set_model_operator_status(
                            format!("Pack admitted by Core: {pack_id}").into(),
                        ),
                        Err(err) => ui.set_model_operator_status(
                            format!("Pack admitted by Core: {pack_id}; inventory refresh failed: {err:?}").into(),
                        ),
                    }
                }
                Ok(result) => ui.set_model_operator_status(
                    format!("Pack admission refused: {:?}", result.reason).into(),
                ),
                Err(err) => {
                    ui.set_model_operator_status(format!("Pack admission failed: {err:?}").into())
                }
            }
            return;
        }
        if let Some(query) = action.strip_prefix("insights-assistant:") {
            ui.set_insights_assistant_answer(insights_for_actions.answer_query(query).into());
            ui.set_active_route("Insights".into());
            return;
        }
        if let Some(action_id) = action.strip_prefix("workflow-review:") {
            ui.set_workflow_review_detail(workflow_for_actions.review_detail(action_id).into());
            ui.set_active_route("Tasks".into());
            return;
        }
        let route = if action == "care-plan" {
            Some("Workflows")
        } else if action == "summarize" || action == "import" || action.starts_with("search:") {
            Some("Patients")
        } else if action == "privacy-status" {
            Some("Settings")
        } else {
            None
        };
        if let Some(route) = route {
            ui.set_active_route(route.into());
        }
    });

    let capture_failed = Rc::new(std::cell::Cell::new(false));
    let capture_timer = slint::Timer::default();
    if let Some(options) = render {
        let weak = ui.as_weak();
        let failed = capture_failed.clone();
        // One bounded capture after a normal shown native frame; no background watcher.
        capture_timer.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(1500),
            move || {
                let result = weak
                    .upgrade()
                    .ok_or_else(|| "native window unavailable".to_owned())
                    .and_then(|ui| {
                        ui.window()
                            .take_snapshot()
                            .map_err(|error| error.to_string())
                    })
                    .and_then(|pixels| {
                        visual_evidence::write_ppm(&options.output, &pixels)
                            .map_err(|error| error.to_string())
                    });
                match result {
                    Ok(()) => println!("native evidence captured: {}", options.output.display()),
                    Err(message) => {
                        failed.set(true);
                        eprintln!("native evidence capture failed: {message}");
                    }
                }
                let _ = slint::quit_event_loop();
            },
        );
    }
    match ui.run() {
        Ok(()) if capture_failed.get() => ExitCode::from(1),
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("MedScale Desktop UI exited with an error: {err}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{admit_model_pack, perf_idle_ms, validated_model_pack_path};
    use medscale_core::CliSession;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn perf_idle_probe_is_bounded_and_explicit() {
        assert_eq!(
            perf_idle_ms(&["--perf-idle-ms".to_owned(), "250".to_owned()]),
            Ok(Some(250))
        );
        assert!(perf_idle_ms(&["--perf-idle-ms".to_owned(), "49".to_owned()]).is_err());
        assert!(perf_idle_ms(&["--perf-idle-ms".to_owned()]).is_err());
    }

    #[test]
    fn model_pack_path_requires_an_absolute_directory_path() {
        assert!(validated_model_pack_path("").is_err());
        assert!(validated_model_pack_path("relative/pack").is_err());

        let absolute = std::env::temp_dir().join("signed-pack");
        let raw = format!(" {} ", absolute.display());
        assert_eq!(
            validated_model_pack_path(&raw),
            Ok(absolute.to_str().expect("temp path must be valid UTF-8"))
        );
    }

    #[test]
    fn model_pack_admission_releases_session_borrow_before_refresh() {
        let session = Rc::new(RefCell::new(
            CliSession::connect_pack_operator("desktop-model-center-borrow-test")
                .expect("connect least-privilege Model Center session"),
        ));
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0");
        let result = admit_model_pack(&session, &fixture.display().to_string())
            .expect("admit signed fixture through scoped borrow");
        assert!(result.admitted);
        assert!(
            session.try_borrow_mut().is_ok(),
            "admission must release RefCell borrow before Model Center refresh"
        );
    }

    #[test]
    fn no_tauri_or_runtime_crate_in_manifest() {
        let manifest =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
        assert!(!manifest.contains("tauri"));
        assert!(
            !manifest.contains("medscale-pack"),
            "Desktop must reach Pack state through Core authority, not the runtime crate"
        );
    }
}

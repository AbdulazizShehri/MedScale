//! Intelligence and operations commands: models, MedAgent, model fleet,
//! collaboration, workflows and audio.

use medscale_contracts::audio::PcmFormat;
use medscale_contracts::medagent::ToolKind;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding};
use medscale_core::CliSession;
use medscale_desktop_vm::{
    audio_workspace, collaboration_workspace, medagent_workspace, model_fleet_workspace,
    product_intelligence::ProductIntelligenceVm, workflow_studio::WorkflowStudioVm,
};
use serde_json::{Value, json};

use crate::cmd_core::lock;
use crate::host::{self, CmdError, CmdResult, Host, ONNX_FIXTURE_PACK, State};

// ───────────────────────────── models ─────────────────────────────

#[tauri::command(async)]
pub fn models_overview(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let vm = ProductIntelligenceVm::from_session(state.packs()?)?;
    Ok(json!({
        "model_runtime_summary": vm.model_runtime_summary,
        "model_runtime_boundary": vm.model_runtime_boundary,
        "model_source_summary": vm.model_source_summary,
        "model_inventory_summary": vm.model_inventory_summary,
        "openmed_baseline": vm.openmed_baseline,
        "competitive_summary": vm.competitive_summary,
        "models": vm.models.iter().map(|m| crate::obj!(m => scope, task, model, source, runtime, device, trust, benchmark, digest, state)).collect::<Vec<_>>(),
        "evidence": vm.evidence.iter().map(|e| crate::obj!(e => capability, medscale, openmed, verdict, evidence, limitations)).collect::<Vec<_>>(),
    }))
}

/// Admits the bundled synthetic ONNX fixture pack through the least-privilege
/// Model Center session. Core verifies the manifest, digests and signature.
#[tauri::command(async)]
pub fn models_admit_fixture_pack(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let dir = state.materialize_pack(ONNX_FIXTURE_PACK)?;
    let result = state
        .packs()?
        .packs_install_local(&dir.display().to_string())?;
    Ok(json!(result))
}

// ─────────────────────── shared agent fixtures ───────────────────────

/// Installs the ONNX fixture pack into the workspace session and returns
/// (pack id, pack dir). Idempotent at the Core level.
fn workspace_pack(state: &mut State) -> CmdResult<(OpaqueId, String)> {
    let dir = state.materialize_pack(ONNX_FIXTURE_PACK)?;
    let dir = dir.display().to_string();
    let result = state.session()?.packs_install_local(&dir)?;
    let pack_id = result
        .pack_id
        .ok_or_else(|| CmdError::new("denied", "Denied: fixture pack was not admitted"))?;
    Ok((pack_id, dir))
}

/// Context artifacts bind to real sources this desktop ingested.
fn source_artifacts(state: &mut State, count: usize) -> CmdResult<Vec<ArtifactDescriptor>> {
    let ws = state.workspace()?;
    let mut ids: Vec<String> = ws
        .seed
        .sources
        .iter()
        .map(|s| s.source_id.clone())
        .collect();
    ids.dedup();
    if ids.is_empty() {
        return Err(CmdError::unavailable(
            "Unavailable: no ingested sources to bind; load synthetic fixtures first",
        ));
    }
    Ok(ids
        .into_iter()
        .take(count)
        .map(|id| ArtifactDescriptor {
            object_id: OpaqueId::new(id),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        })
        .collect())
}

// ───────────────────────────── medagent ─────────────────────────────

#[tauri::command(async)]
pub fn medagent_runs(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows = medagent_workspace::refresh_runs(state.session()?, host::bounded_id(&project_id)?)?;
    let identities = state
        .session()?
        .medagent_identity_list(OpaqueId::new(host::bounded_id(&project_id)?), None)?;
    Ok(json!({
        "runs": rows.iter().map(|r| crate::obj!(r => id, status, revision, prompt)).collect::<Vec<_>>(),
        "agents": identities.iter().map(|i| json!({ "id": i.header.id.as_str(), "label": i.display_name, "status": i.status })).collect::<Vec<_>>(),
    }))
}

#[tauri::command(async)]
pub fn medagent_turns(host: tauri::State<'_, Host>, run_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows = medagent_workspace::refresh_turns(state.session()?, host::bounded_id(&run_id)?)?;
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => seq, kind, payload))
            .collect::<Vec<_>>()
    ))
}

/// Folder of an admitted Pack: the bundled fixture, or one this desktop
/// admitted into the open vault.
fn pack_dir(state: &mut State, pack_id: &str) -> CmdResult<String> {
    let (fixture_id, fixture_dir) = workspace_pack(state)?;
    if fixture_id.as_str() == pack_id {
        return Ok(fixture_dir);
    }
    let root = state.workspace()?.vault_root.clone();
    host::load_pack_dirs(&root).remove(pack_id).ok_or_else(|| {
        CmdError::unavailable(
            "Unavailable: this Pack's folder is not known to this desktop; admit it again",
        )
    })
}

/// Models an agent can use: Packs admitted in the open vault. The bundled
/// fixture is always admitted; other Packs come from `medagent_admit_pack`.
#[tauri::command(async)]
pub fn medagent_models(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let (fixture_id, _) = workspace_pack(&mut state)?;
    let root = state.workspace()?.vault_root.clone();
    let dirs = host::load_pack_dirs(&root);
    let packs = state.session()?.packs_list()?;
    Ok(json!(
        packs
            .iter()
            .map(|p| {
                let id = p.pack_id.as_str();
                json!({
                    "id": id,
                    "version": p.version,
                    "runtime": p.runtime_requirements,
                    "fixture": id == fixture_id.as_str(),
                    "runnable": id == fixture_id.as_str() || dirs.contains_key(id),
                })
            })
            .collect::<Vec<_>>()
    ))
}

/// Admits a local Pack folder into the open vault. Core verifies the
/// manifest, digests, signature and trust root; nothing is downloaded.
#[tauri::command(async)]
pub fn medagent_admit_pack(host: tauri::State<'_, Host>, path: String) -> CmdResult<Value> {
    let path = host::bounded_text(&path, 1024)?;
    let dir = std::path::Path::new(&path);
    if !dir.is_absolute() || !dir.is_dir() {
        return Err(CmdError::invalid(
            "Invalid: choose an existing absolute Pack folder",
        ));
    }
    let mut state = lock(&host)?;
    let result = state.session()?.packs_install_local(&path)?;
    let Some(pack_id) = result.pack_id.clone().filter(|_| result.admitted) else {
        return Err(CmdError::new(
            "denied",
            format!("Denied: Pack not admitted ({:?})", result.reason),
        ));
    };
    let root = state.workspace()?.vault_root.clone();
    let mut dirs = host::load_pack_dirs(&root);
    dirs.insert(pack_id.as_str().to_owned(), path);
    host::save_pack_dirs(&root, &dirs)?;
    Ok(json!({ "id": pack_id.as_str(), "admitted": true }))
}

/// Ingested sources an agent's context can bind.
#[tauri::command(async)]
pub fn medagent_sources(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let ws = state.workspace()?;
    let mut seen = std::collections::BTreeSet::new();
    Ok(json!(
        ws.seed
            .sources
            .iter()
            .filter(|s| seen.insert(s.source_id.clone()))
            .map(|s| json!({
                "id": s.source_id,
                "fixture": s.fixture,
                "resource_type": s.resource_type,
                "subject": s.subject_ref,
            }))
            .collect::<Vec<_>>()
    ))
}

/// Registers a named agent on an admitted Pack with the granted tools.
#[tauri::command(async)]
pub fn medagent_create_agent(
    host: tauri::State<'_, Host>,
    project_id: String,
    name: String,
    pack_id: String,
    tools: Vec<String>,
) -> CmdResult<Value> {
    let project_id = host::bounded_id(&project_id)?.to_owned();
    let pack_id = host::bounded_id(&pack_id)?.to_owned();
    let name = host::bounded_text(&name, 80)?;
    let tools = tools
        .iter()
        .map(|t| {
            serde_json::from_value::<ToolKind>(Value::String(t.clone()))
                .map_err(|_| CmdError::invalid("Invalid: unknown agent tool"))
        })
        .collect::<CmdResult<Vec<_>>>()?;
    if tools.is_empty() {
        return Err(CmdError::invalid("Invalid: grant at least one tool"));
    }
    let mut state = lock(&host)?;
    let (identity, capabilities) = state.session()?.medagent_identity_register(
        OpaqueId::new(&project_id),
        OpaqueId::new(&pack_id),
        name,
        tools,
    )?;
    Ok(agent_json(&identity, &capabilities))
}

fn agent_json(
    identity: &medscale_contracts::medagent::AgentIdentity,
    capabilities: &medscale_contracts::medagent::AgentCapabilityManifest,
) -> Value {
    json!({
        "id": identity.header.id.as_str(),
        "label": identity.display_name,
        "status": identity.status,
        "revision": identity.revision,
        "pack_id": identity.pack_id.as_str(),
        "pack_version": identity.pack_version,
        "tools": capabilities.granted_tool_kinds,
    })
}

/// Revokes an agent; its existing runs and turns stay on record.
#[tauri::command(async)]
pub fn medagent_revoke_agent(
    host: tauri::State<'_, Host>,
    agent_id: String,
    revision: u64,
) -> CmdResult<Value> {
    let agent_id = host::bounded_id(&agent_id)?.to_owned();
    let mut state = lock(&host)?;
    let session = state.session()?;
    session.medagent_identity_revoke(OpaqueId::new(&agent_id), revision)?;
    let (identity, capabilities) = session.medagent_identity_get(OpaqueId::new(&agent_id))?;
    Ok(agent_json(&identity, &capabilities))
}

/// One run with its agent, model and the evidence its context binds.
#[tauri::command(async)]
pub fn medagent_run_detail(host: tauri::State<'_, Host>, run_id: String) -> CmdResult<Value> {
    let run_id = host::bounded_id(&run_id)?.to_owned();
    let mut state = lock(&host)?;
    let sources: std::collections::BTreeMap<String, host::SeededSource> = state
        .workspace()?
        .seed
        .sources
        .iter()
        .map(|s| (s.source_id.clone(), s.clone()))
        .collect();
    let session = state.session()?;
    let run = session.medagent_run_get(OpaqueId::new(&run_id))?;
    let (identity, capabilities) = session.medagent_identity_get(run.agent_identity_id.clone())?;
    let (context, resolutions) = session.medagent_context_get(run.context_manifest_id.clone())?;
    let evidence = context
        .selected_artifacts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let id = a.object_id.as_str();
            let source = sources.get(id);
            json!({
                "id": id,
                "resolution": resolutions.get(i),
                "fixture": source.map(|s| s.fixture.clone()),
                "resource_type": source.map(|s| s.resource_type.clone()),
                "subject": source.map(|s| s.subject_ref.clone()),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "run": {
            "id": run.header.id.as_str(),
            "status": run.status.as_str(),
            "revision": run.revision,
            "prompt": run.prompt,
        },
        "agent": agent_json(&identity, &capabilities),
        "context_id": context.header.id.as_str(),
        "evidence": evidence,
    }))
}

/// Creates a pending run. With `agent_id` and `source_ids`, the run uses that
/// agent and a context bound to exactly those ingested sources; without them,
/// the first active agent (or a new synthetic review agent on the fixture
/// Pack) and the first two sources.
#[tauri::command(async)]
pub fn medagent_create_run(
    host: tauri::State<'_, Host>,
    project_id: String,
    prompt: String,
    agent_id: Option<String>,
    source_ids: Option<Vec<String>>,
) -> CmdResult<Value> {
    let project_id = host::bounded_id(&project_id)?.to_owned();
    let prompt = host::bounded_text(&prompt, 2000)?;
    let agent_id = agent_id
        .map(|a| host::bounded_id(&a).map(str::to_owned))
        .transpose()?;
    let mut state = lock(&host)?;
    let artifacts = match source_ids {
        Some(ids) => selected_sources(&mut state, &ids)?,
        None => source_artifacts(&mut state, 2)?,
    };
    let (pack_id, _) = workspace_pack(&mut state)?;
    let session = state.session()?;
    let existing = session.medagent_identity_list(OpaqueId::new(&project_id), None)?;
    let active = existing
        .iter()
        .find(|i| i.status == medscale_contracts::medagent::AgentIdentityStatus::Active);
    let agent_id = match (agent_id, active) {
        (Some(id), _) => OpaqueId::new(id),
        (None, Some(identity)) => identity.header.id.clone(),
        (None, None) => {
            session
                .medagent_identity_register(
                    OpaqueId::new(&project_id),
                    pack_id,
                    "Synthetic review agent".to_owned(),
                    vec![ToolKind::ReadContextArtifact],
                )?
                .0
                .header
                .id
        }
    };
    let (context, _) = session.medagent_context_create(OpaqueId::new(&project_id), artifacts)?;
    let run = medagent_workspace::create_run(
        session,
        &project_id,
        agent_id.as_str(),
        context.header.id.as_str(),
        prompt,
    )?;
    Ok(crate::obj!(run => id, status, revision, prompt))
}

/// Context artifacts for exactly the chosen ingested sources (1 to 16).
fn selected_sources(state: &mut State, ids: &[String]) -> CmdResult<Vec<ArtifactDescriptor>> {
    if ids.is_empty() || ids.len() > 16 {
        return Err(CmdError::invalid("Invalid: choose 1 to 16 sources"));
    }
    let ws = state.workspace()?;
    let mut out: Vec<ArtifactDescriptor> = Vec::new();
    for id in ids {
        let id = host::bounded_id(id)?;
        if !ws.seed.sources.iter().any(|s| s.source_id == id) {
            return Err(CmdError::invalid(
                "Invalid: source is not ingested in this workspace",
            ));
        }
        if out.iter().any(|a| a.object_id.as_str() == id) {
            continue;
        }
        out.push(ArtifactDescriptor {
            object_id: OpaqueId::new(id),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        });
    }
    Ok(out)
}

#[tauri::command(async)]
pub fn medagent_start(
    host: tauri::State<'_, Host>,
    run_id: String,
    revision: u64,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = medagent_workspace::start_run(state.session()?, host::bounded_id(&run_id)?, revision)?;
    Ok(crate::obj!(r => id, status, revision, prompt))
}

#[tauri::command(async)]
pub fn medagent_cancel(
    host: tauri::State<'_, Host>,
    run_id: String,
    revision: u64,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = medagent_workspace::cancel_run(state.session()?, host::bounded_id(&run_id)?, revision)?;
    Ok(crate::obj!(r => id, status, revision, prompt))
}

/// Completes a running run (`Running -> Completed`) and returns its receipt.
#[tauri::command(async)]
pub fn medagent_complete(
    host: tauri::State<'_, Host>,
    run_id: String,
    revision: u64,
) -> CmdResult<Value> {
    let run_id = host::bounded_id(&run_id)?.to_owned();
    let mut state = lock(&host)?;
    let (run, receipt) = state
        .session()?
        .medagent_run_complete(OpaqueId::new(&run_id), revision)?;
    Ok(json!({
        "id": run.header.id.as_str(),
        "status": run.status.as_str(),
        "revision": run.revision,
        "prompt": run.prompt,
        "receipt": receipt,
    }))
}

/// Executes a started run on the local ONNX fixture runtime. The result is a
/// proposal; nothing is promoted or written to a record.
#[tauri::command(async)]
pub fn medagent_execute(host: tauri::State<'_, Host>, run_id: String) -> CmdResult<Value> {
    let run_id = host::bounded_id(&run_id)?.to_owned();
    let mut state = lock(&host)?;
    // The run's own agent Pack; Core rejects a folder that is not exactly it.
    let session = state.session()?;
    let run = session.medagent_run_get(OpaqueId::new(&run_id))?;
    let (identity, _) = session.medagent_identity_get(run.agent_identity_id)?;
    let dir = pack_dir(&mut state, identity.pack_id.as_str())?;
    let (turn, proposal) =
        state
            .session()?
            .medagent_run_execute(OpaqueId::new(&run_id), dir, 4, true)?;
    Ok(json!({ "turn": turn, "proposal": proposal }))
}

// ───────────────────────────── model fleet ─────────────────────────────

#[tauri::command(async)]
pub fn fleet_overview(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let project_id = host::bounded_id(&project_id)?.to_owned();
    let mut state = lock(&host)?;
    let session = state.session()?;
    let lanes = model_fleet_workspace::refresh_lanes(session, &project_id)?;
    let fleets = model_fleet_workspace::refresh_fleets(session, &project_id)?;
    Ok(json!({
        "lanes": lanes.iter().map(|r| crate::obj!(r => id, role_label, status, revision)).collect::<Vec<_>>(),
        "fleets": fleets.iter().map(|r| crate::obj!(r => id, status, revision)).collect::<Vec<_>>(),
    }))
}

/// Creates two independent reviewer lanes on the fixture pack, each bound to a
/// different ingested source.
#[tauri::command(async)]
pub fn fleet_setup_lanes(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let project_id = host::bounded_id(&project_id)?.to_owned();
    let mut state = lock(&host)?;
    let (pack_id, _) = workspace_pack(&mut state)?;
    let artifacts = source_artifacts(&mut state, 2)?;
    let session = state.session()?;
    let mut created = Vec::new();
    for (n, artifact) in artifacts.into_iter().enumerate() {
        let (identity, _) = session.medagent_identity_register(
            OpaqueId::new(&project_id),
            pack_id.clone(),
            format!("Fleet lane agent {}", n + 1),
            vec![ToolKind::ReadContextArtifact],
        )?;
        let (context, _) =
            session.medagent_context_create(OpaqueId::new(&project_id), vec![artifact])?;
        let lane = session.model_fleet_lane_create(
            OpaqueId::new(&project_id),
            identity.header.id,
            context.header.id,
            format!("reviewer {}", n + 1),
            None,
            None,
        )?;
        created.push(lane.header.id.as_str().to_owned());
    }
    Ok(json!({ "lanes": created }))
}

#[tauri::command(async)]
pub fn fleet_create(
    host: tauri::State<'_, Host>,
    project_id: String,
    prompt: String,
) -> CmdResult<Value> {
    let prompt = host::bounded_text(&prompt, 2000)?;
    let mut state = lock(&host)?;
    let r = model_fleet_workspace::create_fleet(
        state.session()?,
        host::bounded_id(&project_id)?,
        prompt,
    )?;
    Ok(crate::obj!(r => id, status, revision))
}

#[tauri::command(async)]
pub fn fleet_dispatch(
    host: tauri::State<'_, Host>,
    fleet_id: String,
    revision: u64,
    lane_ids: Vec<String>,
) -> CmdResult<Value> {
    let fleet_id = host::bounded_id(&fleet_id)?.to_owned();
    let mut lanes = Vec::new();
    for id in &lane_ids {
        lanes.push(host::bounded_id(id)?.to_owned());
    }
    if lanes.is_empty() || lanes.len() > 16 {
        return Err(CmdError::invalid("Invalid: choose 1–16 lanes"));
    }
    let mut state = lock(&host)?;
    let (_, dir) = workspace_pack(&mut state)?;
    let r = model_fleet_workspace::dispatch_fleet(
        state.session()?,
        &fleet_id,
        revision,
        &lanes.join(","),
        &dir,
    )?;
    Ok(crate::obj!(r => id, status, revision))
}

#[tauri::command(async)]
pub fn fleet_execute_lane(
    host: tauri::State<'_, Host>,
    fleet_id: String,
    lane_id: String,
) -> CmdResult<Value> {
    let fleet_id = host::bounded_id(&fleet_id)?.to_owned();
    let lane_id = host::bounded_id(&lane_id)?.to_owned();
    let mut state = lock(&host)?;
    let (_, dir) = workspace_pack(&mut state)?;
    let r = model_fleet_workspace::execute_lane(state.session()?, &fleet_id, &lane_id, &dir)?;
    Ok(crate::obj!(r => id, status, revision))
}

#[tauri::command(async)]
pub fn fleet_cancel(
    host: tauri::State<'_, Host>,
    fleet_id: String,
    revision: u64,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = model_fleet_workspace::cancel_fleet(
        state.session()?,
        host::bounded_id(&fleet_id)?,
        revision,
    )?;
    Ok(crate::obj!(r => id, status, revision))
}

fn report_json(r: &model_fleet_workspace::ReportVm) -> Value {
    json!({
        "id": r.id, "participating": r.participating, "excluded": r.excluded,
        "observations": r.observations.iter().map(|o| crate::obj!(o => kind, lanes, detail)).collect::<Vec<_>>(),
    })
}

#[tauri::command(async)]
pub fn fleet_detail(host: tauri::State<'_, Host>, fleet_id: String) -> CmdResult<Value> {
    let fleet_id = host::bounded_id(&fleet_id)?.to_owned();
    let mut state = lock(&host)?;
    let session = state.session()?;
    let d = model_fleet_workspace::open_fleet(session, &fleet_id)?;
    let report = model_fleet_workspace::latest_report(session, &fleet_id)?;
    Ok(json!({
        "fleet": crate::obj!(d.fleet => id, status, revision),
        "lanes": d.lanes.iter().map(|l| crate::obj!(l => lane_id, run_id, run_status)).collect::<Vec<_>>(),
        "report": report.as_ref().map(report_json),
    }))
}

#[tauri::command(async)]
pub fn fleet_compare(host: tauri::State<'_, Host>, fleet_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = model_fleet_workspace::compare_fleet(state.session()?, host::bounded_id(&fleet_id)?)?;
    Ok(report_json(&r))
}

// ───────────────────────────── collaboration ─────────────────────────────

#[tauri::command(async)]
pub fn collab_rooms(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows =
        collaboration_workspace::refresh_rooms(state.session()?, host::bounded_id(&project_id)?)?;
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => id, name, revision))
            .collect::<Vec<_>>()
    ))
}

#[tauri::command(async)]
pub fn collab_create_room(
    host: tauri::State<'_, Host>,
    project_id: String,
    name: String,
) -> CmdResult<Value> {
    let name = host::bounded_text(&name, 128)?;
    let mut state = lock(&host)?;
    let r = collaboration_workspace::create_room(
        state.session()?,
        host::bounded_id(&project_id)?,
        name,
    )?;
    Ok(crate::obj!(r => id, name, revision))
}

#[tauri::command(async)]
pub fn collab_room_detail(host: tauri::State<'_, Host>, room_id: String) -> CmdResult<Value> {
    let room_id = host::bounded_id(&room_id)?.to_owned();
    let mut state = lock(&host)?;
    let session = state.session()?;
    let threads = collaboration_workspace::refresh_threads(session, &room_id)?;
    let tasks = collaboration_workspace::refresh_tasks(session, &room_id)?;
    Ok(json!({
        "threads": threads.iter().map(|r| crate::obj!(r => id, status, resolution, artifact_id)).collect::<Vec<_>>(),
        "tasks": tasks.iter().map(|r| crate::obj!(r => id, title, status, revision)).collect::<Vec<_>>(),
    }))
}

/// Opens a discussion thread anchored to an ingested source (a real artifact).
#[tauri::command(async)]
pub fn collab_open_thread(
    host: tauri::State<'_, Host>,
    room_id: String,
    artifact_id: String,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = collaboration_workspace::open_thread(
        state.session()?,
        host::bounded_id(&room_id)?,
        host::bounded_id(&artifact_id)?,
    )?;
    Ok(crate::obj!(r => id, status, resolution, artifact_id))
}

#[tauri::command(async)]
pub fn collab_messages(host: tauri::State<'_, Host>, thread_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows =
        collaboration_workspace::refresh_messages(state.session()?, host::bounded_id(&thread_id)?)?;
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => author, body, seq))
            .collect::<Vec<_>>()
    ))
}

#[tauri::command(async)]
pub fn collab_post_message(
    host: tauri::State<'_, Host>,
    thread_id: String,
    body: String,
) -> CmdResult<()> {
    let body = host::bounded_text(&body, 4000)?;
    let mut state = lock(&host)?;
    Ok(collaboration_workspace::post_message(
        state.session()?,
        host::bounded_id(&thread_id)?,
        body,
    )?)
}

#[tauri::command(async)]
pub fn collab_create_task(
    host: tauri::State<'_, Host>,
    room_id: String,
    title: String,
) -> CmdResult<Value> {
    let title = host::bounded_text(&title, 256)?;
    let mut state = lock(&host)?;
    let r =
        collaboration_workspace::create_task(state.session()?, host::bounded_id(&room_id)?, title)?;
    Ok(crate::obj!(r => id, title, status, revision))
}

#[tauri::command(async)]
pub fn collab_complete_task(
    host: tauri::State<'_, Host>,
    task_id: String,
    revision: u64,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = collaboration_workspace::complete_task(
        state.session()?,
        host::bounded_id(&task_id)?,
        revision,
    )?;
    Ok(crate::obj!(r => id, title, status, revision))
}

// ───────────────────────────── workflows ─────────────────────────────

#[tauri::command(async)]
pub fn workflow_overview(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let entries = state.session()?.outbox()?;
    let vm = WorkflowStudioVm::from_outbox(&entries);
    Ok(json!({
        "synthetic_only": vm.synthetic_only,
        "workflow_name": vm.workflow_name,
        "workflow_state": vm.workflow_state,
        "outbox_summary": vm.outbox_summary,
        "action_boundary": vm.action_boundary,
        "task_boundary": vm.task_boundary,
        "message_boundary": vm.message_boundary,
        "steps": vm.steps.iter().map(|s| crate::obj!(s => order, title, detail, state)).collect::<Vec<_>>(),
        "tasks": vm.tasks.iter().map(|t| crate::obj!(t => action_id, title, state, payload_digest, next_step, reconcile_required)).collect::<Vec<_>>(),
        "messages": vm.messages.iter().map(|m| crate::obj!(m => title, body, status, related_action_id)).collect::<Vec<_>>(),
        "outbox_entries": entries.len(),
    }))
}

// ───────────────────────────── audio ─────────────────────────────

#[tauri::command(async)]
pub fn audio_overview(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let o = audio_workspace::refresh(state.session()?, host::bounded_id(&project_id)?)?;
    Ok(json!({
        "sources": o.sources.iter().map(|r| crate::obj!(r => id, label, kind, detail, health)).collect::<Vec<_>>(),
        "captures": o.captures.iter().map(|r| crate::obj!(r => id, label, state, detail)).collect::<Vec<_>>(),
        "routes": o.routes,
        "native_capture": o.native_capture,
    }))
}

/// Imports a generated synthetic tone/silence WAV (no microphone, no person).
#[tauri::command(async)]
pub fn audio_import_synthetic(
    host: tauri::State<'_, Host>,
    project_id: String,
) -> CmdResult<Value> {
    let project_id = host::bounded_id(&project_id)?.to_owned();
    let wav = CliSession::synthetic_wav(
        PcmFormat::mono_16k(),
        &[(300, false), (800, true), (600, false), (400, true)],
    );
    let mut state = lock(&host)?;
    let source = state.session()?.audio_import(
        OpaqueId::new(&project_id),
        "Synthetic tone sample".to_owned(),
        wav,
    )?;
    Ok(json!({ "source_id": source.header.id.as_str() }))
}

#[tauri::command(async)]
pub fn audio_segment(
    host: tauri::State<'_, Host>,
    project_id: String,
    source_id: String,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let (summary, lines) = audio_workspace::segment(
        state.session()?,
        host::bounded_id(&project_id)?,
        host::bounded_id(&source_id)?,
    )?;
    Ok(json!({ "summary": summary, "lines": lines }))
}

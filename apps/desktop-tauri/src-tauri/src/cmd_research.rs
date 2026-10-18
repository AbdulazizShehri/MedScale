//! Research commands: projects, data, analytics, knowledge, browse, Research OS
//! and privacy. Project-scoped commands take a bounded project id; Core
//! validates that it exists and is in scope.

use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
use medscale_contracts::objects::OpaqueId;
use medscale_desktop_vm::{
    analytics_workspace, browse_workspace, data_workbench, knowledge_workspace, privacy_workspace,
    project_workspace, research_os_workspace,
};
use serde_json::{Value, json};

use crate::cmd_core::lock;
use crate::host::{self, CmdError, CmdResult, Host};

// ───────────────────────────── projects ─────────────────────────────

#[tauri::command(async)]
pub fn projects_list(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows = project_workspace::refresh_projects(state.session()?)?;
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => id, name, status, revision, experiments, refs, edges))
            .collect::<Vec<_>>()
    ))
}

#[tauri::command(async)]
pub fn project_detail(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let d = project_workspace::project_detail(state.session()?, host::bounded_id(&project_id)?)?;
    Ok(json!({
        "project_id": d.project_id, "name": d.name, "status": d.status, "revision": d.revision,
        "experiment_count": d.experiment_count, "active_ref_count": d.active_ref_count, "active_edge_count": d.active_edge_count,
        "experiments": d.experiments.iter().map(|r| crate::obj!(r => id, name, status, revision)).collect::<Vec<_>>(),
        "refs": d.refs.iter().map(|r| crate::obj!(r => ref_id, kind, object_id, resolution)).collect::<Vec<_>>(),
        "edges": d.edges.iter().map(|r| crate::obj!(r => edge_id, subject, predicate, object, revision)).collect::<Vec<_>>(),
    }))
}

#[tauri::command(async)]
pub fn project_create(
    host: tauri::State<'_, Host>,
    name: String,
    description: Option<String>,
) -> CmdResult<Value> {
    let name = project_workspace::validate_name(&name).map_err(CmdError::invalid)?;
    let description = match description.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => Some(host::bounded_text(text, 512)?),
        _ => None,
    };
    let mut state = lock(&host)?;
    let r = project_workspace::create_project(state.session()?, name, description)?;
    Ok(crate::obj!(r => id, name, status, revision, experiments, refs, edges))
}

#[tauri::command(async)]
pub fn project_archive(
    host: tauri::State<'_, Host>,
    project_id: String,
    revision: u64,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = project_workspace::archive_project(
        state.session()?,
        host::bounded_id(&project_id)?,
        revision,
    )?;
    Ok(crate::obj!(r => id, name, status, revision, experiments, refs, edges))
}

#[tauri::command(async)]
pub fn experiment_create(
    host: tauri::State<'_, Host>,
    project_id: String,
    name: String,
) -> CmdResult<Value> {
    let name = project_workspace::validate_name(&name).map_err(CmdError::invalid)?;
    let mut state = lock(&host)?;
    let r = project_workspace::create_experiment(
        state.session()?,
        host::bounded_id(&project_id)?,
        name,
    )?;
    Ok(crate::obj!(r => id, name, status, revision))
}

// ───────────────────────────── data ─────────────────────────────

#[tauri::command(async)]
pub fn data_sources(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows = data_workbench::refresh_sources(state.session()?, host::bounded_id(&project_id)?)?;
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => id, name, kind, health, revision))
            .collect::<Vec<_>>()
    ))
}

/// Creates a small, clearly synthetic, non-clinical CSV inside the vault and
/// registers it as a Core data source. No user-supplied path is ever read.
#[tauri::command(async)]
pub fn data_add_sample_source(
    host: tauri::State<'_, Host>,
    project_id: String,
) -> CmdResult<Value> {
    let project_id = host::bounded_id(&project_id)?.to_owned();
    let mut state = lock(&host)?;
    let ws = state.workspace()?;
    let file = "synthetic-fixture-table.csv";
    std::fs::write(
        ws.vault_root.join(file),
        b"label,value,group\nalpha,5,a\nbeta,9,a\ngamma,12,b\ndelta,7,b\n",
    )
    .map_err(|_| CmdError::unavailable("Unavailable: vault folder not writable"))?;
    let source = ws.session.data_source_create(
        OpaqueId::new(&project_id),
        "Synthetic fixture table".to_owned(),
        SourceLocator::LocalPath {
            path: file.to_owned(),
            format: LocalFileFormat::Csv,
        },
        None,
    )?;
    Ok(json!({ "source_id": source.header.id.as_str() }))
}

#[tauri::command(async)]
pub fn data_snapshots(host: tauri::State<'_, Host>, source_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows = data_workbench::source_snapshots(state.session()?, host::bounded_id(&source_id)?)?;
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => id, rows, digest))
            .collect::<Vec<_>>()
    ))
}

#[tauri::command(async)]
pub fn data_import_snapshot(host: tauri::State<'_, Host>, source_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let r = data_workbench::import_snapshot(state.session()?, host::bounded_id(&source_id)?)?;
    Ok(crate::obj!(r => id, rows, digest))
}

#[tauri::command(async)]
pub fn data_page(host: tauri::State<'_, Host>, snapshot_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let p = data_workbench::workbench_page(state.session()?, host::bounded_id(&snapshot_id)?)?;
    Ok(crate::obj!(p => snapshot_id, schema_line, rows, next_cursor, detail))
}

#[tauri::command(async)]
pub fn data_transform(
    host: tauri::State<'_, Host>,
    snapshot_id: String,
    columns: Option<Vec<String>>,
    filter_column: Option<String>,
    filter_op: Option<String>,
    filter_value: Option<String>,
    sort: Option<String>,
) -> CmdResult<String> {
    let filter = match (filter_column, filter_op, filter_value) {
        (Some(c), Some(o), Some(v)) if !c.trim().is_empty() => {
            data_workbench::parse_filter_input(&c, &o, &v).map_err(CmdError::invalid)?
        }
        _ => None,
    };
    let sort = sort.as_deref().and_then(data_workbench::parse_sort_input);
    let columns = columns.filter(|c| !c.is_empty());
    let mut state = lock(&host)?;
    Ok(data_workbench::apply_workbench_transform(
        state.session()?,
        host::bounded_id(&snapshot_id)?,
        columns,
        filter,
        sort,
    )?)
}

// ───────────────────────────── analytics ─────────────────────────────

#[tauri::command(async)]
pub fn analytics_receipts(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows = analytics_workspace::receipts(state.session()?, host::bounded_id(&project_id)?)?;
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => id, outcome, detail))
            .collect::<Vec<_>>()
    ))
}

#[tauri::command(async)]
pub fn analytics_query(
    host: tauri::State<'_, Host>,
    project_id: String,
    sql: String,
    bindings: String,
) -> CmdResult<Value> {
    let sql = host::bounded_text(&sql, 4000)?;
    let bindings = bindings.trim().to_owned();
    if bindings.len() > 512 {
        return Err(CmdError::invalid("Invalid: bindings are too long"));
    }
    let mut state = lock(&host)?;
    let r = analytics_workspace::run_query(
        state.session()?,
        host::bounded_id(&project_id)?,
        &sql,
        &bindings,
    )?;
    Ok(crate::obj!(r => summary, preview))
}

// ───────────────────────────── knowledge ─────────────────────────────

#[tauri::command(async)]
pub fn knowledge_overview(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let project_id = host::bounded_id(&project_id)?.to_owned();
    let mut state = lock(&host)?;
    let session = state.session()?;
    let status = knowledge_workspace::index_status(session, &project_id)?;
    let canvases = knowledge_workspace::canvases(session, &project_id)?;
    Ok(json!({
        "index_status": status,
        "canvases": canvases.iter().map(|r| crate::obj!(r => id, title, detail)).collect::<Vec<_>>(),
    }))
}

#[tauri::command(async)]
pub fn knowledge_build(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<String> {
    let mut state = lock(&host)?;
    Ok(knowledge_workspace::build_index(
        state.session()?,
        host::bounded_id(&project_id)?,
    )?)
}

#[tauri::command(async)]
pub fn knowledge_search(
    host: tauri::State<'_, Host>,
    project_id: String,
    query: String,
) -> CmdResult<Value> {
    let query = host::bounded_text(&query, 256)?;
    let mut state = lock(&host)?;
    let r = knowledge_workspace::search(state.session()?, host::bounded_id(&project_id)?, &query)?;
    Ok(crate::obj!(r => summary, hits))
}

// ───────────────────────────── browse ─────────────────────────────

fn browse_json(o: &browse_workspace::BrowseOverviewVm) -> Value {
    json!({
        "allowlist": o.allowlist.iter().map(|r| crate::obj!(r => id, target, enabled, revision)).collect::<Vec<_>>(),
        "sessions": o.sessions.iter().map(|r| crate::obj!(r => id, url, state, reason)).collect::<Vec<_>>(),
        "routes": o.routes,
        "fixture_url": medscale_core::CliSession::OFFLINE_BROWSE_FIXTURE_URL,
    })
}

#[tauri::command(async)]
pub fn browse_overview(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let o = browse_workspace::refresh(state.session()?, host::bounded_id(&project_id)?)?;
    Ok(browse_json(&o))
}

#[tauri::command(async)]
pub fn browse_allow(
    host: tauri::State<'_, Host>,
    project_id: String,
    host_name: String,
    path_prefix: String,
) -> CmdResult<Value> {
    let host_name = host::bounded_text(&host_name, 253)?;
    if path_prefix.len() > 256 {
        return Err(CmdError::invalid("Invalid: path prefix is too long"));
    }
    let mut state = lock(&host)?;
    let r = browse_workspace::add_allowed_host(
        state.session()?,
        host::bounded_id(&project_id)?,
        &host_name,
        path_prefix.trim(),
    )?;
    Ok(crate::obj!(r => id, target, enabled, revision))
}

#[tauri::command(async)]
pub fn browse_disable(
    host: tauri::State<'_, Host>,
    entry_id: String,
    revision: u64,
) -> CmdResult<()> {
    let mut state = lock(&host)?;
    Ok(browse_workspace::disable_allowed_host(
        state.session()?,
        host::bounded_id(&entry_id)?,
        revision,
    )?)
}

#[tauri::command(async)]
pub fn browse_fetch(
    host: tauri::State<'_, Host>,
    project_id: String,
    url: String,
) -> CmdResult<Value> {
    let url = host::bounded_text(&url, 2048)?;
    let mut state = lock(&host)?;
    let r = browse_workspace::fetch(state.session()?, host::bounded_id(&project_id)?, &url)?;
    Ok(json!({
        "session": crate::obj!(r.session => id, url, state, reason),
        "excerpt": r.excerpt, "flagged": r.flagged, "downloads": r.downloads,
    }))
}

// ───────────────────────────── research os ─────────────────────────────

#[tauri::command(async)]
pub fn research_os_rows(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let rows = research_os_workspace::rows(state.session()?, host::bounded_id(&project_id)?);
    Ok(json!(
        rows.iter()
            .map(|r| crate::obj!(r => plane, id, state, detail, action))
            .collect::<Vec<_>>()
    ))
}

#[tauri::command(async)]
pub fn research_os_act(
    host: tauri::State<'_, Host>,
    project_id: String,
    plane: String,
    id: String,
    action: String,
) -> CmdResult<String> {
    let plane = host::bounded_text(&plane, 32)?;
    let action = host::bounded_text(&action, 32)?;
    let mut state = lock(&host)?;
    research_os_workspace::act(
        state.session()?,
        host::bounded_id(&project_id)?,
        &plane,
        host::bounded_id(&id)?,
        &action,
    )
    .map_err(|message| CmdError::new("invalid", message))
}

// ───────────────────────────── privacy ─────────────────────────────

#[tauri::command(async)]
pub fn privacy_overview(host: tauri::State<'_, Host>, project_id: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let o = privacy_workspace::refresh(state.session()?, host::bounded_id(&project_id)?)?;
    Ok(json!({
        "classifications": o.classifications.iter().map(|r| crate::obj!(r => artifact_id, data_class, basis, revision)).collect::<Vec<_>>(),
        "receipts": o.receipts.iter().map(|r| crate::obj!(r => id, source, output, status, residual, revision)).collect::<Vec<_>>(),
        "decisions": o.decisions.iter().map(|r| crate::obj!(r => artifact_id, boundary, outcome, reason)).collect::<Vec<_>>(),
    }))
}

#[tauri::command(async)]
pub fn privacy_check_egress(
    host: tauri::State<'_, Host>,
    project_id: String,
    artifact_id: String,
    boundary: String,
) -> CmdResult<Value> {
    let boundary = host::bounded_text(&boundary, 64)?;
    let mut state = lock(&host)?;
    let r = privacy_workspace::check_egress(
        state.session()?,
        host::bounded_id(&project_id)?,
        host::bounded_id(&artifact_id)?,
        &boundary,
    )?;
    Ok(crate::obj!(r => artifact_id, boundary, outcome, reason))
}

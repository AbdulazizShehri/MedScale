//! Workspace, clinical and governance commands. Every command is named and
//! typed; none accepts a capability, Core method, path or arbitrary request.

use medscale_contracts::evidence::LexicalRetrieveRequest;
use medscale_core::{CliSession, build_doctor_report, default_synthetic_corpus};
use medscale_desktop_vm::{
    patient_workspace::PatientWorkspaceVm, population_insights::PopulationInsightsVm,
    utility_surfaces::UtilitySurfacesVm,
};
use serde_json::{Value, json};

use crate::host::{self, CmdError, CmdResult, Host, State, VaultKind};

pub(crate) fn lock(host: &Host) -> CmdResult<std::sync::MutexGuard<'_, State>> {
    host.state
        .lock()
        .map_err(|_| CmdError::new("internal", "Internal: host state lock poisoned"))
}

fn vault_label(state: &State) -> Value {
    match &state.workspace {
        None => json!(null),
        Some(ws) => json!({
            "kind": ws.kind,
            "label": match ws.kind {
                VaultKind::Synthetic => "Synthetic workspace",
                VaultKind::Encrypted => "Encrypted workspace",
            },
            "vault_id": match ws.kind {
                VaultKind::Synthetic => "desktop-synthetic",
                VaultKind::Encrypted => "desktop-encrypted",
            },
            "subjects": ws.seed.subjects.len(),
            "sources": ws.seed.sources.len(),
        }),
    }
}

fn doctor(state: &State) -> medscale_contracts::doctor::DoctorReport {
    let root = state
        .workspace
        .as_ref()
        .map(|ws| ws.vault_root.display().to_string());
    build_doctor_report(
        root.as_deref(),
        state.workspace.is_some(),
        medscale_core::privacy_proof_artifact_present(),
    )
}

// ───────────────────────────── workspace ─────────────────────────────

/// Shell diagnostic kept for the existing invoke-boundary tests. Version 2
/// reports the real connection state instead of a fixed "unavailable".
#[tauri::command(async)]
pub fn get_shell_status(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let state = lock(&host)?;
    let open = state.workspace.is_some();
    let synthetic = state
        .workspace
        .as_ref()
        .is_none_or(|ws| ws.kind == VaultKind::Synthetic);
    Ok(json!({
        "schemaVersion": 2,
        "coreConnection": if open { "connected" } else { "idle" },
        "detail": if open {
            "Core is running in-process against the open local vault."
        } else {
            "No workspace is open. Unlock a vault or open the synthetic workspace."
        },
        "syntheticOnly": synthetic,
    }))
}

#[tauri::command(async)]
pub fn workspace_status(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let state = lock(&host)?;
    let report = doctor(&state);
    Ok(json!({
        "open": state.workspace.is_some(),
        "workspace": vault_label(&state),
        "encrypted_vault_exists": host::encrypted_root(&state)
            .map(|root| root.with_file_name("encrypted-workspace.desktop-seed.json").exists())
            .unwrap_or(false),
        "version": medscale_contracts::MEDSCALE_VERSION,
        "local_only": report.local_only,
        "real_phi_authorized": report.real_phi_authorized,
        "network_default_deny": report.network_broker.default_deny,
    }))
}

#[tauri::command(async)]
pub fn workspace_open_synthetic(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    state.workspace = None;
    host::open_synthetic(&mut state)?;
    Ok(vault_label(&state))
}

#[tauri::command(async)]
pub fn workspace_create_encrypted(
    host: tauri::State<'_, Host>,
    passphrase: String,
) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    state.workspace = None;
    let codes = host::create_encrypted(&mut state, &passphrase)?;
    Ok(json!({ "workspace": vault_label(&state), "recovery_codes": codes }))
}

#[tauri::command(async)]
pub fn workspace_unlock(host: tauri::State<'_, Host>, passphrase: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    state.workspace = None;
    host::unlock_encrypted(&mut state, &passphrase)?;
    Ok(vault_label(&state))
}

#[tauri::command(async)]
pub fn workspace_lock(host: tauri::State<'_, Host>) -> CmdResult<()> {
    let mut state = lock(&host)?;
    // Seal first: if Core cannot seal the encrypted vault the workspace stays
    // open so nothing written since unlock is lost, and the error is shown.
    if let Some(ws) = state.workspace.as_mut() {
        ws.close()?;
    }
    // Dropping the session releases the lease and the in-memory keys.
    state.workspace = None;
    Ok(())
}

// ───────────────────────────── clinical ─────────────────────────────

fn owned_subject<'a>(state: &'a mut State, subject: &str) -> CmdResult<&'a str> {
    let ws = state.workspace()?;
    ws.seed
        .subjects
        .iter()
        .find(|s| s.as_str() == subject)
        .map(String::as_str)
        .ok_or_else(|| CmdError::new("denied", "Denied: subject is outside this workspace"))
}

fn patient_vm(session: &mut CliSession, subject: &str) -> CmdResult<(PatientWorkspaceVm, Value)> {
    let brief = session.brief(subject)?;
    let timeline = session.timeline(subject)?;
    let coverage = session.coverage(subject)?;
    let vm = PatientWorkspaceVm::from_trusted(&brief, &timeline, &coverage)
        .map_err(|_| CmdError::new("corrupt", "Corrupt: projection failed authority checks"))?;
    let raw = json!({ "brief": brief, "timeline": timeline, "coverage": coverage });
    Ok((vm, raw))
}

#[tauri::command(async)]
pub fn patients_list(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let ws = state.workspace()?;
    let subjects = ws.seed.subjects.clone();
    let mut rows = Vec::new();
    for subject in &subjects {
        let (vm, raw) = patient_vm(&mut ws.session, subject)?;
        let sources = ws
            .seed
            .sources
            .iter()
            .filter(|s| &s.subject_ref == subject)
            .count();
        rows.push(json!({
            "subject_ref": vm.subject_ref,
            "display_name": vm.display_name,
            "birth_date": vm.birth_date,
            "condition_summary": vm.condition_summary,
            "coverage_summary": vm.coverage_summary,
            "coverage_state": vm.coverage_state,
            "coverage_slots": raw["coverage"]["slots"].as_array().map(|slots| {
                slots.iter().map(|slot| json!({
                    "concept_key": slot["concept_key"],
                    "status": slot["status"],
                })).collect::<Vec<_>>()
            }),
            "latest_event": vm.timeline.first().map(|t| t.date.clone()),
            "event_count": vm.timeline.len(),
            "source_count": sources,
        }));
    }
    Ok(json!({ "subjects": rows, "scope": "Subjects promoted into this vault by this desktop" }))
}

#[tauri::command(async)]
pub fn patient_detail(host: tauri::State<'_, Host>, subject_ref: String) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let subject = owned_subject(&mut state, host::bounded_id(&subject_ref)?)?.to_owned();
    let ws = state.workspace()?;
    let (vm, raw) = patient_vm(&mut ws.session, &subject)?;
    let sources: Vec<Value> = ws
        .seed
        .sources
        .iter()
        .filter(|s| s.subject_ref == subject)
        .map(|s| json!({ "source_id": s.source_id, "resource_type": s.resource_type, "fixture": s.fixture, "claim_kind": s.claim_kind }))
        .collect();
    Ok(json!({
        "subject_ref": vm.subject_ref,
        "display_name": vm.display_name,
        "birth_date": vm.birth_date,
        "condition_summary": vm.condition_summary,
        "coverage_summary": vm.coverage_summary,
        "coverage_state": vm.coverage_state,
        "timeline": vm.timeline.iter().map(|t| crate::obj!(t => date, title, detail, source, status)).collect::<Vec<_>>(),
        "labs": vm.labs.iter().map(|l| crate::obj!(l => label, value, unit, status, source)).collect::<Vec<_>>(),
        "medications_state": vm.medications_state,
        "documents_state": vm.documents_state,
        "care_plan_state": vm.care_plan_state,
        "sources": sources,
        "contracts": raw,
    }))
}

#[tauri::command(async)]
pub fn documents_list(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let ws = state.workspace()?;
    let rows: Vec<Value> = ws
        .seed
        .sources
        .iter()
        .map(|s| {
            json!({
                "source_id": s.source_id,
                "resource_type": s.resource_type,
                "fixture": s.fixture,
                "claim_kind": s.claim_kind,
                "subject_ref": s.subject_ref,
            })
        })
        .collect();
    Ok(json!({
        "sources": rows,
        "document_understanding": "No OCR, transcription or clinical-note extraction projection is admitted. Holding a source does not mean its text was extracted.",
    }))
}

fn corpus_json() -> Value {
    let corpus = default_synthetic_corpus();
    json!({
        "corpus_id": corpus.corpus_id,
        "version": corpus.version,
        "source_identity": corpus.source_identity(),
        "content_digest": corpus.content_digest.to_hex(),
        "rights": corpus.rights,
        "rights_uri": corpus.rights_uri,
        "documents": corpus.documents.iter().map(|d| json!({
            "doc_id": d.doc_id,
            "text": d.text,
            "content_digest": d.content_digest.to_hex(),
            "status": d.status,
            "freshness_marker": d.freshness_marker,
            "conflict_marker": d.conflict_marker,
        })).collect::<Vec<_>>(),
    })
}

#[tauri::command(async)]
pub fn evidence_corpus() -> CmdResult<Value> {
    Ok(corpus_json())
}

#[tauri::command(async)]
pub fn evidence_search(
    host: tauri::State<'_, Host>,
    query: String,
    include_retracted: bool,
) -> CmdResult<Value> {
    let query = host::bounded_text(&query, 256)?;
    let mut state = lock(&host)?;
    let result = state.session()?.retrieve_lexical(LexicalRetrieveRequest {
        query,
        corpus_id: default_synthetic_corpus().corpus_id.clone(),
        max_hits: 20,
        include_retracted,
    })?;
    Ok(json!(result))
}

fn insights_vm(state: &mut State) -> CmdResult<PopulationInsightsVm> {
    let ws = state.workspace()?;
    let subjects = ws.seed.subjects.clone();
    let mut pairs = Vec::new();
    for subject in &subjects {
        pairs.push((ws.session.brief(subject)?, ws.session.coverage(subject)?));
    }
    let retrieval = ws.session.retrieve_lexical(LexicalRetrieveRequest {
        query: "evidence review coverage".to_owned(),
        corpus_id: default_synthetic_corpus().corpus_id.clone(),
        max_hits: 5,
        include_retracted: true,
    })?;
    PopulationInsightsVm::from_trusted(&pairs, &retrieval).map_err(|_| {
        CmdError::new(
            "corrupt",
            "Corrupt: cohort projection failed authority checks",
        )
    })
}

#[tauri::command(async)]
pub fn insights_overview(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let vm = insights_vm(&mut state)?;
    Ok(json!({
        "synthetic_only": vm.synthetic_only,
        "cohort_size": vm.cohort_size,
        "review_attention_count": vm.review_attention_count,
        "evidence_gap_count": vm.evidence_gap_count,
        "present_count": vm.present_count,
        "condition_distribution": vm.condition_distribution.iter().map(|r| crate::obj!(r => label, value, detail, status)).collect::<Vec<_>>(),
        "coverage_distribution": vm.coverage_distribution.iter().map(|r| crate::obj!(r => label, value, detail, status)).collect::<Vec<_>>(),
        "cohorts": vm.cohorts.iter().map(|r| crate::obj!(r => subject_ref, display_name, condition, evidence_state, review_attention)).collect::<Vec<_>>(),
        "evidence": vm.evidence.iter().map(|r| crate::obj!(r => source, title, snippet, metadata)).collect::<Vec<_>>(),
        "risk_state": vm.risk_state,
        "trend_state": vm.trend_state,
        "gap_state": vm.gap_state,
        "recommendation_state": vm.recommendation_state,
        "assistant_default": vm.assistant_default,
    }))
}

#[tauri::command(async)]
pub fn insights_ask(host: tauri::State<'_, Host>, query: String) -> CmdResult<String> {
    let query = host::bounded_text(&query, 256)?;
    let mut state = lock(&host)?;
    Ok(insights_vm(&mut state)?.answer_query(&query))
}

// ───────────────────────────── governance ─────────────────────────────

fn utility(state: &mut State) -> CmdResult<UtilitySurfacesVm> {
    let report = doctor(state);
    let disclosures = match state.workspace.as_mut() {
        Some(ws) => ws.session.disclosures()?,
        None => Vec::new(),
    };
    Ok(UtilitySurfacesVm::from_doctor(&report, &disclosures))
}

#[tauri::command(async)]
pub fn governance_overview(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let mut state = lock(&host)?;
    let vm = utility(&mut state)?;
    let fhir = match state.workspace.as_mut() {
        Some(ws) => Some(ws.session.fhir_support_matrix()?),
        None => None,
    };
    Ok(json!({
        "synthetic_only": vm.synthetic_only,
        "audit_boundary": vm.audit_boundary,
        "export_boundary": vm.export_boundary,
        "settings_boundary": vm.settings_boundary,
        "integrations_boundary": vm.integrations_boundary,
        "audit_rows": vm.audit_rows.iter().map(|r| crate::obj!(r => record_id, action, subject, detail, status)).collect::<Vec<_>>(),
        "export_rows": vm.export_rows.iter().map(|r| crate::obj!(r => label, value, detail, status)).collect::<Vec<_>>(),
        "settings_rows": vm.settings_rows.iter().map(|r| crate::obj!(r => label, value, detail, status)).collect::<Vec<_>>(),
        "integration_rows": vm.integration_rows.iter().map(|r| crate::obj!(r => label, value, detail, status)).collect::<Vec<_>>(),
        "missing_release_evidence": vm.missing_release_evidence,
        "fhir_support": fhir,
    }))
}

#[tauri::command(async)]
pub fn about_info(host: tauri::State<'_, Host>) -> CmdResult<Value> {
    let state = lock(&host)?;
    let report = doctor(&state);
    Ok(json!({
        "product": "MedScale",
        "tagline": "Local Clinical Intelligence",
        "version": medscale_contracts::MEDSCALE_VERSION,
        "shell": "Tauri 2",
        "core": "Rust Core Host, in-process",
        "local_only": report.local_only,
        "network_default_deny": report.network_broker.default_deny,
        "license": "Apache-2.0",
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        // Qualification state comes from the Core doctor, never from the product name.
        "release_ready": report.release_qualification.release_ready,
        "target": std::env::consts::OS.to_owned() + "-" + std::env::consts::ARCH,
    }))
}

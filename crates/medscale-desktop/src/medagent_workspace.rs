//! MedAgent Workbench view-models (Spec 077, T077-09).
//!
//! Desktop reads and mutates MedAgent state only through the Core-owned
//! `CliSession` (facade authority; never storage or the model runtime
//! directly). Every function below maps one typed Core result to plain
//! view-model rows plus an explicit status string. No fake product data is
//! ever synthesized. Reuses the same `desktop-projects` session Spec
//! 074/075/076 already open: runs are scoped to Projects, so no second
//! vault/session is needed.
//!
//! Scope matches `tasks.md` T077-09 exactly: run list/detail, context
//! selection (an operator-supplied context id, mirroring
//! `collaboration_workspace.rs`'s own `collab-thread-artifact-input`
//! precedent -- a CLI-parity text field, not a full picker widget),
//! prompt submission, and cancel/interrupt. Agent identity registration
//! and context-manifest creation remain CLI-only in this slice, exactly
//! like Spec 076's own Desktop panel leaves Notes/Approvals CLI-only.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

/// One AgentRun list row.
#[derive(Debug, Clone, Default)]
pub struct RunRowVm {
    pub id: String,
    pub status: String,
    pub revision: u64,
    pub prompt: String,
}

/// One AgentTurn row within a run.
#[derive(Debug, Clone, Default)]
pub struct TurnRowVm {
    pub seq: u64,
    pub kind: String,
    pub payload: String,
}

/// Maps a typed Core error to an explicit MedAgent status (no payload leak).
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
        AuthorityError::Conflict { .. } => "Conflict: stale revision or run is not in that state",
        AuthorityError::InvalidArgument { .. } => "Invalid: rejected before any write",
        AuthorityError::Corrupt { .. } => "Corrupt: integrity check failed",
        AuthorityError::UnsupportedSchema { .. } => "Unsupported: unknown schema value",
        AuthorityError::Cancelled { .. } => "Cancelled: no write was performed",
        AuthorityError::ExternalGateRequired { .. } => {
            "Blocked: requires an explicit gate not granted here"
        }
        _ => "Unavailable: vault or Core not ready",
    }
}

fn run_row(run: medscale_contracts::medagent::AgentRun) -> RunRowVm {
    RunRowVm {
        id: run.header.id.as_str().to_owned(),
        status: run.status.as_str().to_owned(),
        revision: run.revision,
        prompt: run.prompt,
    }
}

/// Lists runs for one Project.
pub fn refresh_runs(
    session: &mut CliSession,
    project_id: &str,
) -> Result<Vec<RunRowVm>, AuthorityError> {
    let runs = session.medagent_run_list(OpaqueId::new(project_id), None, None, Some(100))?;
    Ok(runs.into_iter().map(run_row).collect())
}

/// Creates a new `Pending` run against an operator-supplied agent identity
/// and context manifest id (CLI-parity text fields; both are registered/
/// created via the CLI vertical slice, matching Spec 076's own
/// artifact-id-by-hand precedent for opening a thread).
pub fn create_run(
    session: &mut CliSession,
    project_id: &str,
    agent_id: &str,
    context_id: &str,
    prompt: String,
) -> Result<RunRowVm, AuthorityError> {
    let run = session.medagent_run_create(
        OpaqueId::new(project_id),
        OpaqueId::new(agent_id),
        OpaqueId::new(context_id),
        prompt,
    )?;
    Ok(run_row(run))
}

/// Starts a `Pending` run (`Pending -> Running`).
pub fn start_run(
    session: &mut CliSession,
    run_id: &str,
    expected_revision: u64,
) -> Result<RunRowVm, AuthorityError> {
    let (run, _turn) = session.medagent_run_start(OpaqueId::new(run_id), expected_revision)?;
    Ok(run_row(run))
}

/// Cancels a `Pending` or `Running` run (the Desktop "interrupt" control).
pub fn cancel_run(
    session: &mut CliSession,
    run_id: &str,
    expected_revision: u64,
) -> Result<RunRowVm, AuthorityError> {
    let (run, _receipt) = session.medagent_run_cancel(OpaqueId::new(run_id), expected_revision)?;
    Ok(run_row(run))
}

/// Lists one run's turns in `seq` order.
pub fn refresh_turns(
    session: &mut CliSession,
    run_id: &str,
) -> Result<Vec<TurnRowVm>, AuthorityError> {
    let turns = session.medagent_run_turns(OpaqueId::new(run_id))?;
    Ok(turns
        .into_iter()
        .map(|turn| TurnRowVm {
            seq: turn.seq,
            kind: turn.kind.as_str().to_owned(),
            payload: turn.payload.to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::medagent::ToolKind;
    use medscale_contracts::project_graph::{
        ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding,
    };

    fn test_session(name: &str) -> CliSession {
        let dir = std::env::temp_dir().join(format!("medscale-077d-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut session = CliSession::connect("desktop-medagent-test").expect("operator session");
        session
            .open_synthetic_vault(&dir.display().to_string())
            .expect("open vault");
        session
    }

    fn fixture_pack_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0")
    }

    /// Mirrors `collaboration_workspace.rs`'s own
    /// `collab_workspace_flows_through_real_core_session`: every
    /// view-model function in this module is exercised against a real
    /// `CliSession` (Core), never synthetic/fake data, so the Desktop
    /// MedAgent panel's data path is proven even though this workstation
    /// cannot render the Slint UI locally or in CI.
    #[test]
    fn medagent_workspace_flows_through_real_core_session() {
        let mut session = test_session("flows");
        let project = session
            .project_create("study".to_owned(), None)
            .expect("project");
        let project_id = project.header.id.as_str().to_owned();

        let pack = session
            .packs_install_local(&fixture_pack_dir().display().to_string())
            .expect("install pack");
        assert!(pack.admitted);
        let pack_id = pack.pack_id.expect("admitted pack id").as_str().to_owned();

        let (identity, _capabilities) = session
            .medagent_identity_register(
                OpaqueId::new(&project_id),
                OpaqueId::new(&pack_id),
                "Desktop Test Agent".to_owned(),
                vec![ToolKind::ReadContextArtifact],
            )
            .expect("register identity");
        let agent_id = identity.header.id.as_str().to_owned();

        let (context, _resolutions) = session
            .medagent_context_create(
                OpaqueId::new(&project_id),
                vec![ArtifactDescriptor {
                    object_id: OpaqueId::new("source-unknown"),
                    kind: ArtifactKind::SourceRecord,
                    binding: ArtifactVersionBinding::IdentityOnly,
                }],
            )
            .expect("create context");
        let context_id = context.header.id.as_str().to_owned();

        let run = create_run(
            &mut session,
            &project_id,
            &agent_id,
            &context_id,
            "summarize the bound context".to_owned(),
        )
        .expect("create run");
        assert_eq!(run.status, "pending");

        let runs = refresh_runs(&mut session, &project_id).expect("refresh runs");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, run.id);

        let started = start_run(&mut session, &run.id, run.revision).expect("start run");
        assert_eq!(started.status, "running");

        let turns = refresh_turns(&mut session, &run.id).expect("refresh turns");
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].kind, "prompt_submitted");

        let cancelled = cancel_run(&mut session, &run.id, started.revision).expect("cancel run");
        assert_eq!(cancelled.status, "cancelled");

        // Statuses stay explicit for every error class (never a payload leak).
        assert_eq!(
            status_message(&AuthorityError::NotFound),
            "Missing: not found in this vault"
        );
    }

    #[test]
    fn status_message_never_empty() {
        assert!(!status_message(&AuthorityError::NotFound).is_empty());
        assert!(
            !status_message(&AuthorityError::Conflict {
                message: "x".to_owned()
            })
            .is_empty()
        );
    }
}

//! Projects workspace view-models (Spec 074).
//!
//! Desktop reads and mutates Projects only through the Core-owned `CliSession`
//! API shared with the CLI. No storage, network, or authority shortcuts live
//! here. Typed Core errors become explicit status strings; raw payloads never
//! cross into presentation.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::project_graph::{
    ExperimentStatus, GraphEndpoint, ProjectGraphPredicate, ProjectStatus, ReferenceResolution,
};
use medscale_core::CliSession;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRowVm {
    pub id: String,
    pub name: String,
    pub status: String,
    pub revision: u64,
    pub experiments: u64,
    pub refs: u64,
    pub edges: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentRowVm {
    pub id: String,
    pub name: String,
    pub status: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRowVm {
    pub ref_id: String,
    pub kind: String,
    pub object_id: String,
    pub resolution: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRowVm {
    pub edge_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDetailVm {
    pub project_id: String,
    pub name: String,
    pub status: String,
    pub revision: u64,
    pub experiment_count: u64,
    pub active_ref_count: u64,
    pub active_edge_count: u64,
    pub experiments: Vec<ExperimentRowVm>,
    pub refs: Vec<RefRowVm>,
    pub edges: Vec<EdgeRowVm>,
}

/// Maps a typed Core error to an explicit workspace status (no payload leak).
#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope
        | AuthorityError::BrokerDenied { .. }
        | AuthorityError::PackDenied { .. } => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::Conflict { .. } => "Conflict: stale revision, nothing written",
        AuthorityError::StaleReference { .. } => "Stale: target changed since pinned",
        AuthorityError::InvalidArgument { .. }
        | AuthorityError::LexicalReject { .. }
        | AuthorityError::VersionReject { .. }
        | AuthorityError::IllegalTransition => "Invalid: rejected before any write",
        AuthorityError::Corrupt { .. } | AuthorityError::DigestMismatch => {
            "Corrupt: integrity check failed"
        }
        AuthorityError::UnsupportedSchema { .. } => "Unsupported: unknown schema value",
        AuthorityError::Cancelled { .. } => "Cancelled: no write was performed",
        AuthorityError::Unavailable { .. }
        | AuthorityError::LeaseRequired
        | AuthorityError::VaultRequired
        | AuthorityError::LeaseHeld { .. }
        | AuthorityError::AlreadyHeld { .. }
        | AuthorityError::NotHolder
        | AuthorityError::NotHeld
        | AuthorityError::MissingKeyMaterial
        | AuthorityError::ExternalGateRequired { .. }
        | AuthorityError::UnknownRequiresReconcile => "Unavailable: vault or lease not ready",
        AuthorityError::PathOutsideClaim | AuthorityError::Internal { .. } => {
            "Internal: unexpected Core failure"
        }
    }
}

/// Validates a project/experiment name typed into the workspace.
pub fn validate_name(raw: &str) -> Result<String, &'static str> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("Enter a non-empty name");
    }
    if name.chars().count() > 128 {
        return Err("Name must be at most 128 characters");
    }
    Ok(name.to_owned())
}

/// Short human label for one graph endpoint (identity only, no payload).
#[must_use]
pub fn endpoint_label(endpoint: &GraphEndpoint) -> String {
    match endpoint {
        GraphEndpoint::Experiment(id) => format!("experiment:{}", id.as_str()),
        GraphEndpoint::Artifact(descriptor) => format!(
            "artifact:{} ({})",
            descriptor.object_id.as_str(),
            descriptor.kind.as_str()
        ),
    }
}

fn project_status_label(status: ProjectStatus) -> &'static str {
    status.as_str()
}

fn experiment_status_label(status: ExperimentStatus) -> &'static str {
    status.as_str()
}

fn resolution_label(resolution: ReferenceResolution) -> &'static str {
    match resolution {
        ReferenceResolution::Current => "current",
        ReferenceResolution::Stale => "stale",
        ReferenceResolution::Missing => "missing",
        ReferenceResolution::UnsupportedKind => "unsupported kind",
        ReferenceResolution::Denied => "denied",
        ReferenceResolution::Corrupt => "corrupt",
    }
}

fn predicate_label(predicate: ProjectGraphPredicate) -> &'static str {
    predicate.as_str()
}

/// Lists Projects with summaries through Core.
pub fn refresh_projects(session: &mut CliSession) -> Result<Vec<ProjectRowVm>, AuthorityError> {
    let (projects, _) = session.project_list(None, Some(100), None)?;
    Ok(projects
        .into_iter()
        .map(|summary| ProjectRowVm {
            id: summary.project_id.as_str().to_owned(),
            name: summary.name.clone(),
            status: project_status_label(summary.status).to_owned(),
            revision: summary.revision,
            experiments: summary.experiment_count,
            refs: summary.active_ref_count,
            edges: summary.active_edge_count,
        })
        .collect())
}

/// Resolves the bounded detail view for one Project through Core.
pub fn project_detail(
    session: &mut CliSession,
    project_id: &str,
) -> Result<ProjectDetailVm, AuthorityError> {
    let context = session.project_context(OpaqueId::new(project_id), None, Some(100), Some(100))?;
    let (experiments, _) = session.experiment_list(OpaqueId::new(project_id), Some(100), None)?;
    Ok(ProjectDetailVm {
        project_id: context.project.project_id.as_str().to_owned(),
        name: context.project.name.clone(),
        status: project_status_label(context.project.status).to_owned(),
        revision: context.project.revision,
        experiment_count: context.project.experiment_count,
        active_ref_count: context.project.active_ref_count,
        active_edge_count: context.project.active_edge_count,
        experiments: experiments
            .into_iter()
            .map(|summary| ExperimentRowVm {
                id: summary.experiment_id.as_str().to_owned(),
                name: summary.name.clone(),
                status: experiment_status_label(summary.status).to_owned(),
                revision: summary.revision,
            })
            .collect(),
        refs: context
            .authorized_artifact_refs
            .into_iter()
            .map(|resolved| RefRowVm {
                ref_id: resolved.reference.header.id.as_str().to_owned(),
                kind: resolved.reference.artifact.kind.as_str().to_owned(),
                object_id: resolved.reference.artifact.object_id.as_str().to_owned(),
                resolution: resolution_label(resolved.resolution).to_owned(),
            })
            .collect(),
        edges: context
            .graph_slice
            .into_iter()
            .map(|edge| EdgeRowVm {
                edge_id: edge.header.id.as_str().to_owned(),
                subject: endpoint_label(&edge.subject),
                predicate: predicate_label(edge.predicate).to_owned(),
                object: endpoint_label(&edge.object),
                revision: edge.revision,
            })
            .collect(),
    })
}

/// Creates a Project through Core.
pub fn create_project(
    session: &mut CliSession,
    name: String,
    description: Option<String>,
) -> Result<ProjectRowVm, AuthorityError> {
    let project = session.project_create(name, description)?;
    let summary = session.project_summary(project.header.id.clone())?;
    Ok(ProjectRowVm {
        id: summary.project_id.as_str().to_owned(),
        name: summary.name.clone(),
        status: project_status_label(summary.status).to_owned(),
        revision: summary.revision,
        experiments: summary.experiment_count,
        refs: summary.active_ref_count,
        edges: summary.active_edge_count,
    })
}

/// Archives a Project through Core (references never cascade).
pub fn archive_project(
    session: &mut CliSession,
    project_id: &str,
    expected_revision: u64,
) -> Result<ProjectRowVm, AuthorityError> {
    let project = session.project_archive(OpaqueId::new(project_id), expected_revision)?;
    let summary = session.project_summary(project.header.id.clone())?;
    Ok(ProjectRowVm {
        id: summary.project_id.as_str().to_owned(),
        name: summary.name.clone(),
        status: project_status_label(summary.status).to_owned(),
        revision: summary.revision,
        experiments: summary.experiment_count,
        refs: summary.active_ref_count,
        edges: summary.active_edge_count,
    })
}

/// Creates an Experiment in a Project through Core.
pub fn create_experiment(
    session: &mut CliSession,
    project_id: &str,
    name: String,
) -> Result<ExperimentRowVm, AuthorityError> {
    let experiment = session.experiment_create(OpaqueId::new(project_id), name, None)?;
    Ok(ExperimentRowVm {
        id: experiment.header.id.as_str().to_owned(),
        name: experiment.name.clone(),
        status: experiment_status_label(experiment.status).to_owned(),
        revision: experiment.revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session(name: &str) -> (CliSession, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("medscale-074d-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut session = CliSession::connect("desktop-projects-test").expect("operator session");
        session
            .open_synthetic_vault(&dir.display().to_string())
            .expect("synthetic vault");
        (session, dir)
    }

    #[test]
    fn status_messages_stay_typed() {
        assert_eq!(
            status_message(&AuthorityError::NotFound),
            "Missing: not found in this vault"
        );
        assert_eq!(
            status_message(&AuthorityError::Conflict {
                message: "x".to_owned()
            }),
            "Conflict: stale revision, nothing written"
        );
        assert_eq!(
            status_message(&AuthorityError::WrongScope),
            "Denied by Core authority"
        );
        assert_eq!(
            status_message(&AuthorityError::InvalidArgument {
                message: "x".to_owned()
            }),
            "Invalid: rejected before any write"
        );
        assert_eq!(
            status_message(&AuthorityError::Corrupt {
                message: "x".to_owned()
            }),
            "Corrupt: integrity check failed"
        );
        assert_eq!(
            status_message(&AuthorityError::Unavailable {
                message: "x".to_owned()
            }),
            "Unavailable: vault or lease not ready"
        );
    }

    #[test]
    fn name_validation_rejects_empty_and_overlong() {
        assert!(validate_name("  ").is_err());
        assert!(validate_name(&"n".repeat(129)).is_err());
        assert_eq!(validate_name("  Cohort  ").unwrap(), "Cohort");
    }

    #[test]
    fn endpoint_labels_carry_identity_only() {
        let experiment = endpoint_label(&GraphEndpoint::Experiment(OpaqueId::new("exp-1")));
        assert_eq!(experiment, "experiment:exp-1");
        let artifact = endpoint_label(&GraphEndpoint::Artifact(
            medscale_contracts::project_graph::ArtifactDescriptor {
                object_id: OpaqueId::new("src-1"),
                kind: medscale_contracts::project_graph::ArtifactKind::SourceRecord,
                binding: medscale_contracts::project_graph::ArtifactVersionBinding::IdentityOnly,
            },
        ));
        assert_eq!(artifact, "artifact:src-1 (source_record)");
    }

    #[test]
    fn resolution_labels_cover_every_state() {
        for (resolution, label) in [
            (ReferenceResolution::Current, "current"),
            (ReferenceResolution::Stale, "stale"),
            (ReferenceResolution::Missing, "missing"),
            (ReferenceResolution::UnsupportedKind, "unsupported kind"),
            (ReferenceResolution::Denied, "denied"),
            (ReferenceResolution::Corrupt, "corrupt"),
        ] {
            assert_eq!(resolution_label(resolution), label);
        }
    }

    #[test]
    fn workspace_flows_through_real_core_session() {
        let (mut session, _dir) = test_session("flows");
        // Empty start is an honest empty state, not an error.
        assert!(refresh_projects(&mut session).expect("refresh").is_empty());
        let created = create_project(&mut session, "Cohort".to_owned(), None).expect("create");
        assert_eq!(created.name, "Cohort");
        assert_eq!(created.status, "active");
        let rows = refresh_projects(&mut session).expect("refresh");
        assert_eq!(rows.len(), 1);
        let experiment =
            create_experiment(&mut session, &created.id, "Baseline".to_owned()).expect("exp");
        assert_eq!(experiment.status, "draft");
        let detail = project_detail(&mut session, &created.id).expect("detail");
        assert_eq!(detail.name, "Cohort");
        assert_eq!(detail.experiment_count, 1);
        assert_eq!(detail.experiments.len(), 1);
        assert!(detail.refs.is_empty());
        assert!(detail.edges.is_empty());
        let archived =
            archive_project(&mut session, &created.id, created.revision).expect("archive");
        assert_eq!(archived.status, "archived");
        assert_eq!(archived.revision, created.revision + 1);
    }
}

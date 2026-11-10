//! Spec 074 project workspace commands (CLI vertical slice through Core).
//!
//! Every command opens the session scope, dispatches one typed Core request,
//! and renders the typed result as human lines or stable JSON. The CLI never
//! writes project storage directly. Authority errors keep typed codes in both
//! output modes.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, GraphDirection, GraphEndpoint,
    ProjectGraphPredicate, ProjectStatus,
};

use super::{fail_json, print_json_or_debug};

/// Maps an authority error to a stable typed CLI failure.
fn project_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => ("denied", debug),
        AuthorityError::BrokerDenied { .. } | AuthorityError::PackDenied { .. } => {
            ("denied", debug)
        }
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::StaleReference { message } => ("stale_reference", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::LexicalReject { reason } => ("invalid", reason.clone()),
        AuthorityError::IllegalTransition => ("invalid", debug),
        AuthorityError::VersionReject { got } => (
            "invalid",
            format!(
                "unsupported version: {}",
                got.as_deref().unwrap_or("unknown")
            ),
        ),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::DigestMismatch => ("corrupt", debug),
        AuthorityError::UnsupportedSchema { message } => ("unsupported_schema", message.clone()),
        AuthorityError::Unavailable { message } => ("unavailable", message.clone()),
        AuthorityError::Cancelled { message } => ("cancelled", message.clone()),
        AuthorityError::LeaseRequired
        | AuthorityError::VaultRequired
        | AuthorityError::LeaseHeld { .. }
        | AuthorityError::AlreadyHeld { .. }
        | AuthorityError::NotHolder
        | AuthorityError::NotHeld
        | AuthorityError::MissingKeyMaterial
        | AuthorityError::ExternalGateRequired { .. }
        | AuthorityError::UnknownRequiresReconcile => ("unavailable", debug),
        AuthorityError::PathOutsideClaim | AuthorityError::Internal { .. } => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn open_project_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| project_fail(&err, json))
}

fn print_project_human(project: &medscale_contracts::project_graph::Project) {
    println!("project_id: {}", project.header.id.as_str());
    println!("name: {}", project.name);
    println!("status: {}", project.status.as_str());
    println!("revision: {}", project.revision);
    if let Some(description) = &project.description {
        println!("description: {description}");
    }
}

fn print_experiment_human(experiment: &medscale_contracts::project_graph::Experiment) {
    println!("experiment_id: {}", experiment.header.id.as_str());
    println!("project_id: {}", experiment.project_id.as_str());
    println!("name: {}", experiment.name);
    println!("status: {}", experiment.status.as_str());
    println!("revision: {}", experiment.revision);
}

/// Parses a closed project status vocabulary (fail closed).
pub fn parse_status(value: &str) -> Result<ProjectStatus, String> {
    match value {
        "active" => Ok(ProjectStatus::Active),
        "archived" => Ok(ProjectStatus::Archived),
        _ => Err(format!("unknown project status: {value}")),
    }
}

/// Parses a closed artifact-kind vocabulary (fail closed).
pub fn parse_kind(value: &str) -> Result<ArtifactKind, String> {
    match value {
        "source_record" => Ok(ArtifactKind::SourceRecord),
        "derived_source_artifact" => Ok(ArtifactKind::DerivedSourceArtifact),
        "proposal" => Ok(ArtifactKind::Proposal),
        "clinical_assertion" => Ok(ArtifactKind::ClinicalAssertion),
        "evaluation_record" => Ok(ArtifactKind::EvaluationRecord),
        "identity_assertion" => Ok(ArtifactKind::IdentityAssertion),
        "amendment_record" => Ok(ArtifactKind::AmendmentRecord),
        "evidence_document" => Ok(ArtifactKind::EvidenceDocument),
        "pack_manifest" => Ok(ArtifactKind::PackManifest),
        _ => Err(format!("unknown artifact kind: {value}")),
    }
}

/// Parses a closed predicate vocabulary (fail closed).
pub fn parse_predicate(value: &str) -> Result<ProjectGraphPredicate, String> {
    ProjectGraphPredicate::parse(value).ok_or_else(|| format!("unknown graph predicate: {value}"))
}

/// Parses a closed direction vocabulary (fail closed).
pub fn parse_direction(value: &str) -> Result<GraphDirection, String> {
    match value {
        "outgoing" => Ok(GraphDirection::Outgoing),
        "incoming" => Ok(GraphDirection::Incoming),
        "both" => Ok(GraphDirection::Both),
        _ => Err(format!("unknown graph direction: {value}")),
    }
}

/// Parses a hex SHA-256 digest (fail closed).
pub fn parse_digest_hex(value: &str) -> Result<medscale_contracts::objects::DigestSha256, String> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("digest must be 64 lowercase hex characters".to_owned());
    }
    let mut bytes = [0_u8; 32];
    for (i, chunk) in value.as_bytes().chunks(2).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|_| "digest is not UTF-8".to_owned())?;
        bytes[i] = u8::from_str_radix(text, 16).map_err(|_| "digest is not hex".to_owned())?;
    }
    Ok(medscale_contracts::objects::DigestSha256::from_bytes(bytes))
}

/// Parses one graph endpoint spec.
///
/// Accepted forms: `experiment:<id>` or
/// `artifact:<id>:<kind>[:digest=<hex>|revision=<token>]`.
pub fn parse_endpoint(spec: &str) -> Result<GraphEndpoint, String> {
    let mut parts = spec.splitn(4, ':');
    match parts.next() {
        Some("experiment") => {
            let id = parts.next().filter(|s| !s.is_empty()).ok_or_else(|| {
                format!("malformed experiment endpoint (want experiment:<id>): {spec}")
            })?;
            if parts.next().is_some() {
                return Err(format!("malformed experiment endpoint: {spec}"));
            }
            Ok(GraphEndpoint::Experiment(OpaqueId::new(id)))
        }
        Some("artifact") => {
            let id = parts.next().filter(|s| !s.is_empty()).ok_or_else(|| {
                format!("malformed artifact endpoint (want artifact:<id>:<kind>): {spec}")
            })?;
            let kind = parts
                .next()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("malformed artifact endpoint (missing kind): {spec}"))?;
            let binding = match parts.next() {
                None => ArtifactVersionBinding::IdentityOnly,
                Some(rest) => {
                    if parts.next().is_some() {
                        return Err(format!("malformed artifact endpoint: {spec}"));
                    }
                    if let Some(hex) = rest.strip_prefix("digest=") {
                        ArtifactVersionBinding::Digest(parse_digest_hex(hex)?)
                    } else if let Some(token) = rest.strip_prefix("revision=") {
                        if token.trim().is_empty() {
                            return Err(format!("empty revision token: {spec}"));
                        }
                        ArtifactVersionBinding::Revision(token.to_owned())
                    } else {
                        return Err(format!(
                            "malformed binding (want digest=|revision=): {spec}"
                        ));
                    }
                }
            };
            Ok(GraphEndpoint::Artifact(ArtifactDescriptor {
                object_id: OpaqueId::new(id),
                kind: parse_kind(kind)?,
                binding,
            }))
        }
        _ => Err(format!(
            "malformed endpoint (want experiment:<id> or artifact:<id>:<kind>): {spec}"
        )),
    }
}

/// Project workspace commands.
#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    /// Create a Project.
    Create {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List Projects in the session scope.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one Project.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Update Project metadata (revision-guarded).
    Update {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        clear_description: bool,
        #[arg(long)]
        json: bool,
    },
    /// Archive a Project (references never cascade).
    Archive {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Restore an archived Project.
    Restore {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Resolve the bounded Project context.
    Context {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        experiment_id: Option<String>,
        #[arg(long)]
        refs_limit: Option<u32>,
        #[arg(long)]
        graph_limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Show the Project summary.
    Summary {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Attach an artifact reference (target must resolve now).
    Attach {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        experiment_id: Option<String>,
        #[arg(long)]
        object_id: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        digest: Option<String>,
        #[arg(long)]
        revision: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Detach an artifact reference (tombstone; canonical untouched).
    Detach {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        ref_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// List resolved artifact references.
    ListRefs {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        experiment_id: Option<String>,
        #[arg(long, default_value_t = true)]
        active_only: bool,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Typed relationship operations.
    Graph {
        #[command(subcommand)]
        action: ProjectGraphCmd,
    },
}

/// Project relationship subcommands.
#[derive(Debug, Subcommand)]
pub enum ProjectGraphCmd {
    /// Bounded single-hop neighbor query.
    Neighbors {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        start: String,
        #[arg(long)]
        predicate: Vec<String>,
        #[arg(long)]
        direction: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create a typed edge.
    EdgeCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        predicate: String,
        #[arg(long)]
        object: String,
        #[arg(long)]
        json: bool,
    },
    /// Remove a typed edge (endpoints untouched).
    EdgeRemove {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        edge_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
}

/// Experiment workspace commands.
#[derive(Debug, Subcommand)]
pub enum ExperimentCmd {
    /// Create an Experiment in a Project.
    Create {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List Experiments of a Project.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one Experiment.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        experiment_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Update Experiment metadata (revision-guarded).
    Update {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        experiment_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        clear_description: bool,
        #[arg(long)]
        json: bool,
    },
    /// Archive an Experiment (referenced artifacts untouched).
    Archive {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        experiment_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

fn description_update(
    description: Option<String>,
    clear: bool,
    json: bool,
) -> anyhow::Result<Option<Option<String>>> {
    if clear && description.is_some() {
        return Err(invalid(
            "use either --description or --clear-description".to_owned(),
            json,
        ));
    }
    if clear {
        Ok(Some(None))
    } else {
        Ok(description.map(Some))
    }
}

fn build_binding(
    digest: Option<String>,
    revision: Option<String>,
    json: bool,
) -> anyhow::Result<ArtifactVersionBinding> {
    match (digest, revision) {
        (Some(_), Some(_)) => Err(invalid(
            "use either --digest or --revision".to_owned(),
            json,
        )),
        (Some(hex), None) => parse_digest_hex(&hex)
            .map(ArtifactVersionBinding::Digest)
            .map_err(|message| invalid(message, json)),
        (None, Some(token)) => {
            if token.trim().is_empty() {
                return Err(invalid("empty revision token".to_owned(), json));
            }
            Ok(ArtifactVersionBinding::Revision(token))
        }
        (None, None) => Ok(ArtifactVersionBinding::IdentityOnly),
    }
}

/// Runs one `medscale project ...` invocation through Core.
pub fn run_project(action: ProjectCmd) -> anyhow::Result<()> {
    match action {
        ProjectCmd::Create {
            vault_id,
            vault_root,
            name,
            description,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let project = session
                .project_create(name, description)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&project, true)?;
            } else {
                print_project_human(&project);
            }
            Ok(())
        }
        ProjectCmd::List {
            vault_id,
            vault_root,
            status,
            limit,
            cursor,
            json,
        } => {
            let status = status
                .map(|s| parse_status(&s))
                .transpose()
                .map_err(|message| invalid(message, json))?;
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let (projects, next_cursor) = session
                .project_list(status, limit, cursor)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"projects": projects, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if projects.is_empty() {
                println!("projects: empty");
            } else {
                for summary in &projects {
                    println!(
                        "{} | {} | {} | rev={} | exp={} refs={} edges={}",
                        summary.project_id.as_str(),
                        summary.name,
                        summary.status.as_str(),
                        summary.revision,
                        summary.experiment_count,
                        summary.active_ref_count,
                        summary.active_edge_count
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        ProjectCmd::Show {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let project = session
                .project_get(OpaqueId::new(project_id))
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&project, true)?;
            } else {
                print_project_human(&project);
            }
            Ok(())
        }
        ProjectCmd::Update {
            vault_id,
            vault_root,
            project_id,
            expected_revision,
            name,
            description,
            clear_description,
            json,
        } => {
            let description = description_update(description, clear_description, json)?;
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let project = session
                .project_update(
                    OpaqueId::new(project_id),
                    expected_revision,
                    name,
                    description,
                )
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&project, true)?;
            } else {
                print_project_human(&project);
            }
            Ok(())
        }
        ProjectCmd::Archive {
            vault_id,
            vault_root,
            project_id,
            expected_revision,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let project = session
                .project_archive(OpaqueId::new(project_id), expected_revision)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&project, true)?;
            } else {
                print_project_human(&project);
            }
            Ok(())
        }
        ProjectCmd::Restore {
            vault_id,
            vault_root,
            project_id,
            expected_revision,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let project = session
                .project_restore(OpaqueId::new(project_id), expected_revision)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&project, true)?;
            } else {
                print_project_human(&project);
            }
            Ok(())
        }
        ProjectCmd::Context {
            vault_id,
            vault_root,
            project_id,
            experiment_id,
            refs_limit,
            graph_limit,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let context = session
                .project_context(
                    OpaqueId::new(project_id),
                    experiment_id.map(OpaqueId::new),
                    refs_limit,
                    graph_limit,
                )
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&context, true)?;
            } else {
                println!("project: {}", context.project.name);
                println!("status: {}", context.project.status.as_str());
                println!("revision: {}", context.project.revision);
                if let Some(experiment) = &context.experiment {
                    println!(
                        "experiment: {} ({})",
                        experiment.name,
                        experiment.status.as_str()
                    );
                }
                println!(
                    "refs: {} shown of {}",
                    context.authorized_artifact_refs.len(),
                    context.refs_total
                );
                for resolved in &context.authorized_artifact_refs {
                    println!(
                        "  {} | {} | {:?}",
                        resolved.reference.header.id.as_str(),
                        resolved.reference.artifact.object_id.as_str(),
                        resolved.resolution
                    );
                }
                println!(
                    "graph: {} shown of {}",
                    context.graph_slice.len(),
                    context.graph_total
                );
            }
            Ok(())
        }
        ProjectCmd::Summary {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let summary = session
                .project_summary(OpaqueId::new(project_id))
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&summary, true)?;
            } else {
                println!("project_id: {}", summary.project_id.as_str());
                println!("name: {}", summary.name);
                println!("status: {}", summary.status.as_str());
                println!("revision: {}", summary.revision);
                println!("experiments: {}", summary.experiment_count);
                println!("active_refs: {}", summary.active_ref_count);
                println!("active_edges: {}", summary.active_edge_count);
            }
            Ok(())
        }
        ProjectCmd::Attach {
            vault_id,
            vault_root,
            project_id,
            experiment_id,
            object_id,
            kind,
            digest,
            revision,
            json,
        } => {
            let kind = parse_kind(&kind).map_err(|message| invalid(message, json))?;
            let binding = build_binding(digest, revision, json)?;
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let reference = session
                .project_attach(
                    OpaqueId::new(project_id),
                    experiment_id.map(OpaqueId::new),
                    ArtifactDescriptor {
                        object_id: OpaqueId::new(object_id),
                        kind,
                        binding,
                    },
                )
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&reference, true)?;
            } else {
                println!("ref_id: {}", reference.header.id.as_str());
                println!("project_id: {}", reference.project_id.as_str());
                println!("revision: {}", reference.revision);
            }
            Ok(())
        }
        ProjectCmd::Detach {
            vault_id,
            vault_root,
            ref_id,
            expected_revision,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let reference = session
                .project_detach(OpaqueId::new(ref_id), expected_revision)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&reference, true)?;
            } else {
                println!("ref_id: {}", reference.header.id.as_str());
                println!("status: detached");
                println!("revision: {}", reference.revision);
            }
            Ok(())
        }
        ProjectCmd::ListRefs {
            vault_id,
            vault_root,
            project_id,
            experiment_id,
            active_only,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let (refs, next_cursor) = session
                .project_list_refs(
                    OpaqueId::new(project_id),
                    experiment_id.map(OpaqueId::new),
                    active_only,
                    limit,
                    cursor,
                )
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"refs": refs, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if refs.is_empty() {
                println!("refs: empty");
            } else {
                for resolved in &refs {
                    println!(
                        "{} | {} | {} | {:?}",
                        resolved.reference.header.id.as_str(),
                        resolved.reference.artifact.kind.as_str(),
                        resolved.reference.artifact.object_id.as_str(),
                        resolved.resolution
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        ProjectCmd::Graph { action } => match action {
            ProjectGraphCmd::Neighbors {
                vault_id,
                vault_root,
                project_id,
                start,
                predicate,
                direction,
                limit,
                cursor,
                json,
            } => {
                let start = parse_endpoint(&start).map_err(|message| invalid(message, json))?;
                let mut predicates = Vec::new();
                for name in &predicate {
                    predicates
                        .push(parse_predicate(name).map_err(|message| invalid(message, json))?);
                }
                let direction = direction
                    .map(|d| parse_direction(&d))
                    .transpose()
                    .map_err(|message| invalid(message, json))?;
                let mut session = open_project_session(&vault_id, &vault_root, json)?;
                let page = session
                    .graph_neighbors(
                        OpaqueId::new(project_id),
                        start,
                        Some(predicates).filter(|p| !p.is_empty()),
                        direction,
                        limit,
                        cursor,
                    )
                    .map_err(|err| project_fail(&err, json))?;
                if json {
                    print_json_or_debug(&page, true)?;
                } else if page.edges.is_empty() {
                    println!("neighbors: empty");
                } else {
                    for edge in &page.edges {
                        println!(
                            "{} | {} | rev={}",
                            edge.header.id.as_str(),
                            edge.predicate.as_str(),
                            edge.revision
                        );
                    }
                    if let Some(cursor) = page.next_cursor {
                        println!("next_cursor: {cursor}");
                    }
                }
                Ok(())
            }
            ProjectGraphCmd::EdgeCreate {
                vault_id,
                vault_root,
                project_id,
                subject,
                predicate,
                object,
                json,
            } => {
                let subject = parse_endpoint(&subject).map_err(|message| invalid(message, json))?;
                let predicate =
                    parse_predicate(&predicate).map_err(|message| invalid(message, json))?;
                let object = parse_endpoint(&object).map_err(|message| invalid(message, json))?;
                let mut session = open_project_session(&vault_id, &vault_root, json)?;
                let edge = session
                    .graph_edge_create(OpaqueId::new(project_id), subject, predicate, object)
                    .map_err(|err| project_fail(&err, json))?;
                if json {
                    print_json_or_debug(&edge, true)?;
                } else {
                    println!("edge_id: {}", edge.header.id.as_str());
                    println!("predicate: {}", edge.predicate.as_str());
                    println!("revision: {}", edge.revision);
                }
                Ok(())
            }
            ProjectGraphCmd::EdgeRemove {
                vault_id,
                vault_root,
                edge_id,
                expected_revision,
                json,
            } => {
                let mut session = open_project_session(&vault_id, &vault_root, json)?;
                let edge = session
                    .graph_edge_remove(OpaqueId::new(edge_id), expected_revision)
                    .map_err(|err| project_fail(&err, json))?;
                if json {
                    print_json_or_debug(&edge, true)?;
                } else {
                    println!("edge_id: {}", edge.header.id.as_str());
                    println!("status: removed");
                    println!("revision: {}", edge.revision);
                }
                Ok(())
            }
        },
    }
}

/// Runs one `medscale experiment ...` invocation through Core.
pub fn run_experiment(action: ExperimentCmd) -> anyhow::Result<()> {
    match action {
        ExperimentCmd::Create {
            vault_id,
            vault_root,
            project_id,
            name,
            description,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let experiment = session
                .experiment_create(OpaqueId::new(project_id), name, description)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&experiment, true)?;
            } else {
                print_experiment_human(&experiment);
            }
            Ok(())
        }
        ExperimentCmd::List {
            vault_id,
            vault_root,
            project_id,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let (experiments, next_cursor) = session
                .experiment_list(OpaqueId::new(project_id), limit, cursor)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"experiments": experiments, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if experiments.is_empty() {
                println!("experiments: empty");
            } else {
                for summary in &experiments {
                    println!(
                        "{} | {} | {} | rev={}",
                        summary.experiment_id.as_str(),
                        summary.name,
                        summary.status.as_str(),
                        summary.revision
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        ExperimentCmd::Show {
            vault_id,
            vault_root,
            experiment_id,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let experiment = session
                .experiment_get(OpaqueId::new(experiment_id))
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&experiment, true)?;
            } else {
                print_experiment_human(&experiment);
            }
            Ok(())
        }
        ExperimentCmd::Update {
            vault_id,
            vault_root,
            experiment_id,
            expected_revision,
            name,
            description,
            clear_description,
            json,
        } => {
            let description = description_update(description, clear_description, json)?;
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let experiment = session
                .experiment_update(
                    OpaqueId::new(experiment_id),
                    expected_revision,
                    name,
                    description,
                )
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&experiment, true)?;
            } else {
                print_experiment_human(&experiment);
            }
            Ok(())
        }
        ExperimentCmd::Archive {
            vault_id,
            vault_root,
            experiment_id,
            expected_revision,
            json,
        } => {
            let mut session = open_project_session(&vault_id, &vault_root, json)?;
            let experiment = session
                .experiment_archive(OpaqueId::new(experiment_id), expected_revision)
                .map_err(|err| project_fail(&err, json))?;
            if json {
                print_json_or_debug(&experiment, true)?;
            } else {
                print_experiment_human(&experiment);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_vocabularies_reject_unknown() {
        assert!(parse_status("deleted").is_err());
        assert!(parse_kind("proves").is_err());
        assert!(parse_kind("DerivedFrom").is_err());
        assert!(parse_predicate("proves").is_err());
        assert!(parse_predicate("Contains").is_err());
        assert!(parse_direction("sideways").is_err());
        assert!(parse_digest_hex("xyz").is_err());
        assert!(parse_digest_hex(&"ab".repeat(31)).is_err());
        assert!(parse_digest_hex(&"ab".repeat(32)).is_ok());
    }

    #[test]
    fn endpoint_specs_parse_strictly() {
        assert!(matches!(
            parse_endpoint("experiment:exp-1"),
            Ok(GraphEndpoint::Experiment(_))
        ));
        assert!(parse_endpoint("experiment:").is_err());
        assert!(parse_endpoint("experiment:exp-1:extra").is_err());
        let endpoint = parse_endpoint("artifact:src-1:source_record").unwrap();
        assert!(matches!(endpoint, GraphEndpoint::Artifact(_)));
        assert!(parse_endpoint("artifact:src-1:proves").is_err());
        assert!(parse_endpoint("artifact:src-1").is_err());
        assert!(parse_endpoint("blob:src-1:source_record").is_err());
        assert!(parse_endpoint("artifact:src-1:source_record:digest=zz").is_err());
        let with_digest = parse_endpoint(&format!(
            "artifact:src-1:source_record:digest={}",
            "ab".repeat(32)
        ))
        .unwrap();
        assert!(matches!(with_digest, GraphEndpoint::Artifact(_)));
        let with_revision = parse_endpoint("artifact:pack-1:pack_manifest:revision=v3").unwrap();
        assert!(matches!(with_revision, GraphEndpoint::Artifact(_)));
    }
}

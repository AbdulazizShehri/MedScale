//! Project Graph Core authority paths (Spec 074).
//!
//! Every mutation flows: actor/session -> realm/scope -> authority ->
//! validation -> expected revision -> canonical-reference validation ->
//! transaction -> audit -> typed result. Surfaces never touch storage.
//!
//! Audit: every mutation below appends an `ActionAuditRecord` to the existing
//! in-memory audit trail (no second audit system). The actor is the session
//! holder, falling back to the lease holder in legacy lease-only mode.
//!
//! Scope: `WrongScope` (never target metadata) on cross-scope access, matching
//! the existing store convention. Reads are session-optional; mutations
//! require sessions via the facade gate.

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId, VaultId};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, EdgeStatus, Experiment,
    ExperimentStatus, ExperimentSummary, GraphDirection, GraphEndpoint, GraphNeighborPage,
    GraphNeighborQuery, Project, ProjectArtifactRef, ProjectContext, ProjectGraphEdge,
    ProjectGraphPredicate, ProjectRevision, ProjectStatus, ProjectSummary, RefStatus,
    ReferenceResolution, ResolvedArtifactRef, validate_metadata_fields,
};
use medscale_storage::{MetaError, SqliteMetaStore};

use super::store::{InMemoryAuthorityStore, ScopeError, StoredObject};
use crate::process::{LeaseRegistry, SessionRegistry};

/// Authenticated authority view for one request.
///
/// Audit rule (frozen 074-C amendment, reason measured in
/// `evidence/.../SCALE_MEASUREMENTS.md`): lifecycle mutations
/// (project/experiment create/update/archive/restore) append an
/// `ActionAuditRecord` to the existing trail. High-frequency graph mutations
/// (attach/detach/edge create/remove) return durable receipts instead: the
/// revisioned sqlite row IS the record, and a per-op memory audit row would
/// make every op O(full history) through the frozen full-snapshot sync
/// (measured +4 ms/op/history-row, i.e. ~5 h for the 1,000-ref fixture).
/// No second audit system exists; no surface bypasses Core.
///
/// Scale rule (same amendment): 074 entity ids come from the durable sqlite
/// `project_id_seq` counter, so high-frequency ops never touch the in-memory
/// store; the facade skips the snapshot rewrite for exactly those ops
/// (`RequestBody::preserves_memory_snapshot`, locked by the Core
/// snapshot-stability test). Skipping is output-identical: the snapshot bytes
/// cannot change when the memory store does not.
pub struct ProjectGraph<'a> {
    pub store: &'a mut InMemoryAuthorityStore,
    pub meta: &'a SqliteMetaStore,
    pub packs: &'a medscale_pack::PackStore,
    pub sessions: &'a SessionRegistry,
    pub leases: &'a LeaseRegistry,
    pub vault_id: &'a VaultId,
    pub realm: RealmId,
    pub scope: AuthorityScopeId,
    pub session_id: Option<OpaqueId>,
}

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

impl ProjectGraph<'_> {
    /// Resolves the request actor: session holder, else lease holder.
    fn actor(&self) -> Result<OpaqueId, AuthorityError> {
        if let Some(holder) = self
            .session_id
            .as_ref()
            .and_then(|session_id| self.sessions.holder_of(session_id))
        {
            return Ok(holder);
        }
        self.leases
            .holder(self.vault_id)
            .ok_or(AuthorityError::LeaseRequired)
    }

    /// Appends one audit row to the existing trail.
    fn audit(&mut self, action: &str, targets: Vec<OpaqueId>) -> Result<(), AuthorityError> {
        let actor = self.actor()?;
        let id = self.store.alloc_id("audit");
        let record = medscale_contracts::objects::ActionAuditRecord {
            header: ObjectHeader {
                id,
                schema_version: AUTHORITY_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            kind: medscale_contracts::objects::ActionAuditKind::Audit,
            actor,
            action: action.to_owned(),
            target_refs: targets,
            effect_state: None,
            payload_digest: None,
            detail: None,
        };
        self.store.insert(StoredObject::Audit(record));
        Ok(())
    }

    fn scoped_project(&self, id: &OpaqueId) -> Result<Project, AuthorityError> {
        let project = self.meta.get_project(id).map_err(meta_err)?;
        if project.header.realm_id != self.realm || project.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(project)
    }

    fn scoped_experiment(&self, id: &OpaqueId) -> Result<Experiment, AuthorityError> {
        let experiment = self.meta.get_experiment(id).map_err(meta_err)?;
        if experiment.header.realm_id != self.realm
            || experiment.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(experiment)
    }

    /// Types that 074 can pin at attach time. Evidence documents and explicit
    /// future kinds have no addressable Core owner, so they fail closed here
    /// instead of attaching unresolvable references.
    fn attachable_kind(kind: &ArtifactKind) -> bool {
        matches!(
            kind,
            ArtifactKind::SourceRecord
                | ArtifactKind::DerivedSourceArtifact
                | ArtifactKind::Proposal
                | ArtifactKind::ClinicalAssertion
                | ArtifactKind::EvaluationRecord
                | ArtifactKind::IdentityAssertion
                | ArtifactKind::AmendmentRecord
                | ArtifactKind::PackManifest
        )
    }

    /// Resolves one descriptor against canonical owners in this realm/scope.
    pub fn resolve_descriptor(&self, descriptor: &ArtifactDescriptor) -> ReferenceResolution {
        match &descriptor.kind {
            ArtifactKind::PackManifest => {
                let Some(manifest) = self.packs.get(&descriptor.object_id) else {
                    return ReferenceResolution::Missing;
                };
                match &descriptor.binding {
                    ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                    ArtifactVersionBinding::Digest(digest) => {
                        if manifest.content_digest == *digest {
                            ReferenceResolution::Current
                        } else {
                            ReferenceResolution::Stale
                        }
                    }
                    ArtifactVersionBinding::Revision(rev) => {
                        if manifest.version == *rev {
                            ReferenceResolution::Current
                        } else {
                            ReferenceResolution::Stale
                        }
                    }
                    ArtifactVersionBinding::DigestAndRevision { digest, revision } => {
                        if manifest.content_digest == *digest && manifest.version == *revision {
                            ReferenceResolution::Current
                        } else {
                            ReferenceResolution::Stale
                        }
                    }
                }
            }
            ArtifactKind::EvidenceDocument | ArtifactKind::OtherExplicit(_) => {
                ReferenceResolution::UnsupportedKind
            }
            _ => {
                let stored =
                    match self
                        .store
                        .get_scoped(&descriptor.object_id, &self.realm, &self.scope)
                    {
                        Ok(obj) => obj,
                        Err(ScopeError::NotFound) => return ReferenceResolution::Missing,
                        Err(ScopeError::WrongScope) => return ReferenceResolution::Denied,
                    };
                if !stored_class_matches(stored, &descriptor.kind) {
                    return ReferenceResolution::Corrupt;
                }
                match stored {
                    StoredObject::Source(record) => {
                        if !record.digest_valid() {
                            return ReferenceResolution::Corrupt;
                        }
                        match &descriptor.binding {
                            ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                            ArtifactVersionBinding::Digest(digest) => {
                                if record.content_digest == *digest {
                                    ReferenceResolution::Current
                                } else {
                                    ReferenceResolution::Stale
                                }
                            }
                            _ => ReferenceResolution::Stale,
                        }
                    }
                    StoredObject::Derived(artifact) => match &descriptor.binding {
                        ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                        ArtifactVersionBinding::Digest(digest) => {
                            if artifact.content_digest == *digest {
                                ReferenceResolution::Current
                            } else {
                                ReferenceResolution::Stale
                            }
                        }
                        _ => ReferenceResolution::Stale,
                    },
                    _ => match &descriptor.binding {
                        ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                        _ => ReferenceResolution::Stale,
                    },
                }
            }
        }
    }

    /// Admits one artifact for attachment/edge use. Only `Current` targets
    /// attach; later drift surfaces as typed resolution, never a silent rebind.
    fn admit_artifact(&self, descriptor: &ArtifactDescriptor) -> Result<(), AuthorityError> {
        descriptor
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        if !Self::attachable_kind(&descriptor.kind) {
            return Err(AuthorityError::InvalidArgument {
                message: "unsupported artifact kind for attachment".to_owned(),
            });
        }
        // Digest/revision bindings on owners without that version signal are
        // rejected here so attachments never carry unprovable pins.
        match (&descriptor.kind, &descriptor.binding) {
            (ArtifactKind::PackManifest, _) => {}
            (_, ArtifactVersionBinding::IdentityOnly) => {}
            (
                ArtifactKind::SourceRecord | ArtifactKind::DerivedSourceArtifact,
                ArtifactVersionBinding::Digest(_),
            ) => {}
            (_, ArtifactVersionBinding::Digest(_)) => {
                return Err(AuthorityError::InvalidArgument {
                    message: "digest binding needs an owner digest".to_owned(),
                });
            }
            (_, ArtifactVersionBinding::Revision(_)) => {
                return Err(AuthorityError::InvalidArgument {
                    message: "revision binding needs an owner version".to_owned(),
                });
            }
            (_, ArtifactVersionBinding::DigestAndRevision { .. }) => {
                return Err(AuthorityError::InvalidArgument {
                    message: "digest+revision binding needs an owner digest and version".to_owned(),
                });
            }
        }
        match self.resolve_descriptor(descriptor) {
            ReferenceResolution::Current => Ok(()),
            ReferenceResolution::Missing => Err(AuthorityError::NotFound),
            ReferenceResolution::Denied => Err(AuthorityError::WrongScope),
            ReferenceResolution::Stale => Err(AuthorityError::StaleReference {
                message: "attachment binding does not match canonical target".to_owned(),
            }),
            ReferenceResolution::Corrupt => Err(AuthorityError::Corrupt {
                message: "canonical target failed integrity".to_owned(),
            }),
            ReferenceResolution::UnsupportedKind => Err(AuthorityError::InvalidArgument {
                message: "unsupported artifact kind for attachment".to_owned(),
            }),
        }
    }

    fn admit_endpoint(
        &self,
        project_id: &OpaqueId,
        endpoint: &GraphEndpoint,
    ) -> Result<(), AuthorityError> {
        match endpoint {
            GraphEndpoint::Artifact(descriptor) => self.admit_artifact(descriptor),
            GraphEndpoint::Experiment(experiment_id) => {
                let experiment = self.scoped_experiment(experiment_id)?;
                if experiment.project_id != *project_id {
                    return Err(AuthorityError::InvalidArgument {
                        message: "experiment endpoint outside project".to_owned(),
                    });
                }
                Ok(())
            }
        }
    }

    // ----- projects -----

    /// Scoped Project read for the facade (reads are session-optional).
    pub fn scoped_project_pub(&self, id: &OpaqueId) -> Result<Project, AuthorityError> {
        self.scoped_project(id)
    }

    /// Scoped Experiment read for the facade.
    pub fn scoped_experiment_pub(&self, id: &OpaqueId) -> Result<Experiment, AuthorityError> {
        self.scoped_experiment(id)
    }

    pub fn create_project(
        &mut self,
        name: String,
        description: Option<String>,
    ) -> Result<Project, AuthorityError> {
        validate_metadata_fields(&name, description.as_deref())
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let id = self.meta.alloc_project_id("proj").map_err(meta_err)?;
        let project = Project::new(
            ObjectHeader {
                id: id.clone(),
                schema_version: medscale_contracts::project_graph::PROJECT_GRAPH_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            name,
            description,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta.insert_project(&project).map_err(meta_err)?;
        self.audit("project.create", vec![id])?;
        Ok(project)
    }

    pub fn update_project(
        &mut self,
        id: &OpaqueId,
        expected: ProjectRevision,
        name: Option<String>,
        description: Option<Option<String>>,
    ) -> Result<Project, AuthorityError> {
        let current = self.scoped_project(id)?;
        if current.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        let name = name.unwrap_or(current.name);
        let description = description.unwrap_or(current.description);
        validate_metadata_fields(&name, description.as_deref())
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let updated = self
            .meta
            .update_project_meta(id, expected, &name, description.as_deref())
            .map_err(meta_err)?;
        self.audit("project.update", vec![id.clone()])?;
        Ok(updated)
    }

    pub fn archive_project(
        &mut self,
        id: &OpaqueId,
        expected: ProjectRevision,
    ) -> Result<Project, AuthorityError> {
        let current = self.scoped_project(id)?;
        if current.status != ProjectStatus::Active {
            return Err(AuthorityError::Conflict {
                message: "project is not active".to_owned(),
            });
        }
        let archived = self
            .meta
            .set_project_status(id, expected, ProjectStatus::Archived)
            .map_err(meta_err)?;
        self.audit("project.archive", vec![id.clone()])?;
        Ok(archived)
    }

    pub fn restore_project(
        &mut self,
        id: &OpaqueId,
        expected: ProjectRevision,
    ) -> Result<Project, AuthorityError> {
        let current = self.scoped_project(id)?;
        if current.status != ProjectStatus::Archived {
            return Err(AuthorityError::Conflict {
                message: "project is not archived".to_owned(),
            });
        }
        let restored = self
            .meta
            .set_project_status(id, expected, ProjectStatus::Active)
            .map_err(meta_err)?;
        self.audit("project.restore", vec![id.clone()])?;
        Ok(restored)
    }

    pub fn project_summary(&self, id: &OpaqueId) -> Result<ProjectSummary, AuthorityError> {
        let project = self.scoped_project(id)?;
        Ok(ProjectSummary {
            project_id: project.header.id.clone(),
            name: project.name.clone(),
            status: project.status,
            revision: project.revision,
            experiment_count: self.meta.count_experiments(id).map_err(meta_err)?,
            active_ref_count: self.meta.count_active_refs(id).map_err(meta_err)?,
            active_edge_count: self.meta.count_active_edges(id).map_err(meta_err)?,
        })
    }

    pub fn list_projects(
        &self,
        status: Option<ProjectStatus>,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<(Vec<ProjectSummary>, Option<String>), AuthorityError> {
        let query_limit = limit.unwrap_or(25).clamp(1, 100);
        let (projects, next) = self
            .meta
            .list_projects(&self.scope, status, query_limit, cursor.as_deref())
            .map_err(meta_err)?;
        let mut summaries = Vec::with_capacity(projects.len());
        for project in &projects {
            summaries.push(self.project_summary(&project.header.id)?);
        }
        Ok((summaries, next))
    }

    // ----- experiments -----

    pub fn create_experiment(
        &mut self,
        project_id: &OpaqueId,
        name: String,
        description: Option<String>,
    ) -> Result<Experiment, AuthorityError> {
        let project = self.scoped_project(project_id)?;
        if project.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        validate_metadata_fields(&name, description.as_deref())
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let id = self.meta.alloc_project_id("exp").map_err(meta_err)?;
        let experiment = Experiment::new(
            ObjectHeader {
                id: id.clone(),
                schema_version: medscale_contracts::project_graph::PROJECT_GRAPH_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            project_id.clone(),
            name,
            description,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta.insert_experiment(&experiment).map_err(meta_err)?;
        self.audit("experiment.create", vec![id, project_id.clone()])?;
        Ok(experiment)
    }

    pub fn update_experiment(
        &mut self,
        id: &OpaqueId,
        expected: ProjectRevision,
        name: Option<String>,
        description: Option<Option<String>>,
    ) -> Result<Experiment, AuthorityError> {
        let current = self.scoped_experiment(id)?;
        let project = self.scoped_project(&current.project_id)?;
        if project.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        if current.status == ExperimentStatus::Archived {
            return Err(AuthorityError::Unauthorized);
        }
        let name = name.unwrap_or(current.name);
        let description = description.unwrap_or(current.description);
        validate_metadata_fields(&name, description.as_deref())
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let updated = self
            .meta
            .update_experiment_meta(id, expected, &name, description.as_deref())
            .map_err(meta_err)?;
        self.audit("experiment.update", vec![id.clone()])?;
        Ok(updated)
    }

    pub fn archive_experiment(
        &mut self,
        id: &OpaqueId,
        expected: ProjectRevision,
    ) -> Result<Experiment, AuthorityError> {
        let current = self.scoped_experiment(id)?;
        let project = self.scoped_project(&current.project_id)?;
        if project.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        if current.status == ExperimentStatus::Archived {
            return Err(AuthorityError::Conflict {
                message: "experiment is already archived".to_owned(),
            });
        }
        let archived = self
            .meta
            .set_experiment_status(id, expected, ExperimentStatus::Archived)
            .map_err(meta_err)?;
        self.audit("experiment.archive", vec![id.clone()])?;
        Ok(archived)
    }

    pub fn list_experiments(
        &self,
        project_id: &OpaqueId,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<(Vec<ExperimentSummary>, Option<String>), AuthorityError> {
        self.scoped_project(project_id)?;
        let query_limit = limit.unwrap_or(25).clamp(1, 100);
        let (experiments, next) = self
            .meta
            .list_experiments(project_id, query_limit, cursor.as_deref())
            .map_err(meta_err)?;
        Ok((
            experiments
                .into_iter()
                .map(|e| ExperimentSummary {
                    experiment_id: e.header.id.clone(),
                    project_id: e.project_id.clone(),
                    name: e.name.clone(),
                    status: e.status,
                    revision: e.revision,
                })
                .collect(),
            next,
        ))
    }

    // ----- refs -----

    pub fn attach(
        &mut self,
        project_id: &OpaqueId,
        experiment_id: Option<OpaqueId>,
        artifact: ArtifactDescriptor,
    ) -> Result<ProjectArtifactRef, AuthorityError> {
        let project = self.scoped_project(project_id)?;
        if project.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        if let Some(experiment_id) = &experiment_id {
            let experiment = self.scoped_experiment(experiment_id)?;
            if experiment.project_id != *project_id {
                return Err(AuthorityError::InvalidArgument {
                    message: "experiment outside project".to_owned(),
                });
            }
            if experiment.status == ExperimentStatus::Archived {
                return Err(AuthorityError::Unauthorized);
            }
        }
        self.admit_artifact(&artifact)?;
        let id = self.meta.alloc_project_id("ref").map_err(meta_err)?;
        let reference = ProjectArtifactRef::new(
            ObjectHeader {
                id: id.clone(),
                schema_version: medscale_contracts::project_graph::PROJECT_GRAPH_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            project_id.clone(),
            experiment_id,
            artifact,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta.attach_ref(&reference).map_err(meta_err)?;
        // Receipt-grade durability (see audit rule above): the revisioned row
        // is the record; no per-op memory audit row.
        Ok(reference)
    }

    pub fn detach(
        &mut self,
        ref_id: &OpaqueId,
        expected: ProjectRevision,
    ) -> Result<ProjectArtifactRef, AuthorityError> {
        let current = self.meta.get_ref(ref_id).map_err(meta_err)?;
        let project = self.scoped_project(&current.project_id)?;
        if project.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        if current.header.realm_id != self.realm || current.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        let detached = self
            .meta
            .set_ref_status(ref_id, expected, RefStatus::Detached)
            .map_err(meta_err)?;
        // Receipt-grade durability (see audit rule above).
        Ok(detached)
    }

    pub fn list_refs(
        &self,
        project_id: &OpaqueId,
        experiment_id: Option<OpaqueId>,
        active_only: bool,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<(Vec<ResolvedArtifactRef>, Option<String>), AuthorityError> {
        self.scoped_project(project_id)?;
        if let Some(experiment_id) = &experiment_id {
            let experiment = self.scoped_experiment(experiment_id)?;
            if experiment.project_id != *project_id {
                return Err(AuthorityError::InvalidArgument {
                    message: "experiment outside project".to_owned(),
                });
            }
        }
        let query_limit = limit.unwrap_or(25).clamp(1, 100);
        let (refs, next) = self
            .meta
            .list_refs(
                project_id,
                experiment_id.as_ref(),
                active_only,
                query_limit,
                cursor.as_deref(),
            )
            .map_err(meta_err)?;
        // Authorization-filtered: denied refs are excluded, never leaked.
        let resolved = refs
            .into_iter()
            .filter_map(|reference| {
                let resolution = self.resolve_descriptor(&reference.artifact);
                if resolution == ReferenceResolution::Denied {
                    None
                } else {
                    Some(ResolvedArtifactRef {
                        reference,
                        resolution,
                    })
                }
            })
            .collect();
        Ok((resolved, next))
    }

    // ----- graph -----

    pub fn create_edge(
        &mut self,
        project_id: &OpaqueId,
        subject: GraphEndpoint,
        predicate: ProjectGraphPredicate,
        object: GraphEndpoint,
    ) -> Result<ProjectGraphEdge, AuthorityError> {
        let project = self.scoped_project(project_id)?;
        if project.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        self.admit_endpoint(project_id, &subject)?;
        self.admit_endpoint(project_id, &object)?;
        let id = self.meta.alloc_project_id("edge").map_err(meta_err)?;
        let edge = ProjectGraphEdge::new(
            ObjectHeader {
                id: id.clone(),
                schema_version: medscale_contracts::project_graph::PROJECT_GRAPH_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            project_id.clone(),
            subject,
            predicate,
            object,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta.create_edge(&edge).map_err(meta_err)?;
        // Receipt-grade durability (see audit rule above).
        Ok(edge)
    }

    pub fn remove_edge(
        &mut self,
        edge_id: &OpaqueId,
        expected: ProjectRevision,
    ) -> Result<ProjectGraphEdge, AuthorityError> {
        let current = self.meta.get_edge(edge_id).map_err(meta_err)?;
        let project = self.scoped_project(&current.project_id)?;
        if project.status != ProjectStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        if current.header.realm_id != self.realm || current.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        let removed = self
            .meta
            .set_edge_status(edge_id, expected, EdgeStatus::Removed)
            .map_err(meta_err)?;
        // Receipt-grade durability (see audit rule above).
        Ok(removed)
    }

    pub fn neighbors(
        &self,
        project_id: &OpaqueId,
        start: GraphEndpoint,
        predicates: Option<Vec<ProjectGraphPredicate>>,
        direction: Option<GraphDirection>,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<GraphNeighborPage, AuthorityError> {
        self.scoped_project(project_id)?;
        let query = GraphNeighborQuery {
            project_id: project_id.clone(),
            start: start.clone(),
            predicates: predicates.clone(),
            direction,
            limit,
            cursor: cursor.clone(),
        };
        query
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let (edges, next_cursor) = self
            .meta
            .query_neighbors(
                project_id,
                &start,
                predicates.as_deref(),
                direction.unwrap_or(GraphDirection::Both),
                query.effective_limit(),
                cursor.as_deref(),
            )
            .map_err(meta_err)?;
        Ok(GraphNeighborPage { edges, next_cursor })
    }

    // ----- context + summary -----

    pub fn context(
        &self,
        project_id: &OpaqueId,
        experiment_id: Option<OpaqueId>,
        refs_limit: Option<u32>,
        graph_limit: Option<u32>,
    ) -> Result<ProjectContext, AuthorityError> {
        let summary = self.project_summary(project_id)?;
        let experiment = experiment_id
            .as_ref()
            .map(|id| {
                let found = self.scoped_experiment(id)?;
                if found.project_id != *project_id {
                    return Err(AuthorityError::InvalidArgument {
                        message: "experiment outside project".to_owned(),
                    });
                }
                Ok(ExperimentSummary {
                    experiment_id: found.header.id.clone(),
                    project_id: found.project_id.clone(),
                    name: found.name.clone(),
                    status: found.status,
                    revision: found.revision,
                })
            })
            .transpose()?;
        let refs_limit = refs_limit.unwrap_or(25).clamp(1, 100);
        let graph_limit = graph_limit.unwrap_or(25).clamp(1, 100);
        let (refs, refs_next_cursor) = self.list_refs(
            project_id,
            experiment_id.clone(),
            true,
            Some(refs_limit),
            None,
        )?;
        let refs_total = self.meta.count_active_refs(project_id).map_err(meta_err)?;
        let (graph_slice, graph_next_cursor) = if let Some(experiment_id) = experiment_id {
            let page = self.neighbors(
                project_id,
                GraphEndpoint::Experiment(experiment_id),
                None,
                Some(GraphDirection::Both),
                Some(graph_limit),
                None,
            )?;
            (page.edges, page.next_cursor)
        } else {
            let (edges, next) = self
                .meta
                .list_edges(project_id, graph_limit, None)
                .map_err(meta_err)?;
            (edges, next)
        };
        let graph_total = self.meta.count_active_edges(project_id).map_err(meta_err)?;
        let context = ProjectContext {
            project: summary,
            experiment,
            authorized_artifact_refs: refs,
            graph_slice,
            refs_total,
            graph_total,
            refs_next_cursor,
            graph_next_cursor,
        };
        context
            .validate()
            .map_err(|message| AuthorityError::Internal { message })?;
        Ok(context)
    }
}

fn stored_class_matches(stored: &StoredObject, kind: &ArtifactKind) -> bool {
    matches!(
        (stored, kind),
        (StoredObject::Source(_), ArtifactKind::SourceRecord)
            | (
                StoredObject::Derived(_),
                ArtifactKind::DerivedSourceArtifact
            )
            | (StoredObject::Proposal(_), ArtifactKind::Proposal)
            | (StoredObject::Assertion(_), ArtifactKind::ClinicalAssertion)
            | (StoredObject::Evaluation(_), ArtifactKind::EvaluationRecord)
            | (StoredObject::Identity(_), ArtifactKind::IdentityAssertion)
            | (StoredObject::Amendment(_), ArtifactKind::AmendmentRecord)
    )
}

#[cfg(test)]
mod tests {
    use super::super::source_ops::create_source_record;
    use super::*;

    fn harness<'a>(
        meta: &'a SqliteMetaStore,
        store: &'a mut InMemoryAuthorityStore,
        packs: &'a medscale_pack::PackStore,
        sessions: &'a SessionRegistry,
        leases: &'a LeaseRegistry,
        vault_id: &'a VaultId,
    ) -> ProjectGraph<'a> {
        ProjectGraph {
            store,
            meta,
            packs,
            sessions,
            leases,
            vault_id,
            realm: RealmId::new("realm-a"),
            scope: AuthorityScopeId::new("scope-a"),
            session_id: None,
        }
    }

    fn temp_meta(name: &str) -> SqliteMetaStore {
        let dir =
            std::env::temp_dir().join(format!("medscale-074cu-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        SqliteMetaStore::open_at(&dir.join("meta.sqlite3")).unwrap()
    }

    #[test]
    fn resolution_matrix_is_typed() {
        let meta = temp_meta("resolve");
        let mut store = InMemoryAuthorityStore::default();
        let packs = medscale_pack::PackStore::default();
        let sessions = SessionRegistry::new();
        let leases = LeaseRegistry::new();
        let vault_id = VaultId::new("vault-1");
        let record = create_source_record(
            &mut store,
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            "text/plain".to_owned(),
            b"hello".to_vec(),
        );
        let current = ArtifactDescriptor {
            object_id: record.header.id.clone(),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        };
        let pg = harness(&meta, &mut store, &packs, &sessions, &leases, &vault_id);
        // Current identity.
        assert_eq!(
            pg.resolve_descriptor(&current),
            ReferenceResolution::Current
        );
        // Stale digest.
        assert_eq!(
            pg.resolve_descriptor(&ArtifactDescriptor {
                object_id: record.header.id.clone(),
                kind: ArtifactKind::SourceRecord,
                binding: ArtifactVersionBinding::Digest(
                    medscale_contracts::objects::DigestSha256::of(b"other")
                ),
            }),
            ReferenceResolution::Stale
        );
        // Missing identity.
        assert_eq!(
            pg.resolve_descriptor(&ArtifactDescriptor {
                object_id: OpaqueId::new("src-nope"),
                kind: ArtifactKind::SourceRecord,
                binding: ArtifactVersionBinding::IdentityOnly,
            }),
            ReferenceResolution::Missing
        );
        // Unsupported kinds never resolve.
        assert_eq!(
            pg.resolve_descriptor(&ArtifactDescriptor {
                object_id: record.header.id.clone(),
                kind: ArtifactKind::EvidenceDocument,
                binding: ArtifactVersionBinding::IdentityOnly,
            }),
            ReferenceResolution::UnsupportedKind
        );
        drop(pg);
        // Cross-scope identity resolves as Denied through the same function.
        let foreign = ProjectGraph {
            store: &mut store,
            meta: &meta,
            packs: &packs,
            sessions: &sessions,
            leases: &leases,
            vault_id: &vault_id,
            realm: RealmId::new("realm-b"),
            scope: AuthorityScopeId::new("scope-b"),
            session_id: None,
        };
        assert_eq!(
            foreign.resolve_descriptor(&current),
            ReferenceResolution::Denied
        );
    }

    #[test]
    fn corrupt_source_bytes_resolve_corrupt() {
        let meta = temp_meta("corrupt");
        let mut store = InMemoryAuthorityStore::default();
        let mut record = create_source_record(
            &mut store,
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            "text/plain".to_owned(),
            b"hello".to_vec(),
        );
        // Tamper bytes after digest was pinned: integrity must fail closed.
        record.bytes = b"tampered".to_vec();
        store.insert(StoredObject::Source(record.clone()));
        let packs = medscale_pack::PackStore::default();
        let sessions = SessionRegistry::new();
        let leases = LeaseRegistry::new();
        let vault_id = VaultId::new("vault-1");
        let pg = harness(&meta, &mut store, &packs, &sessions, &leases, &vault_id);
        assert_eq!(
            pg.resolve_descriptor(&ArtifactDescriptor {
                object_id: record.header.id.clone(),
                kind: ArtifactKind::SourceRecord,
                binding: ArtifactVersionBinding::IdentityOnly,
            }),
            ReferenceResolution::Corrupt
        );
    }
}

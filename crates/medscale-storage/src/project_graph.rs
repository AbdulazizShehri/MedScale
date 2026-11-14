//! Project Graph durable rows (Spec 074, storage schema v3).
//!
//! Dedicated tables inside the existing encrypted metadata DB: no second
//! database, no canonical payload copies. Every 074 contract field is a scalar
//! column except the two nested enums (`ArtifactVersionBinding`, endpoint
//! descriptors), which travel as validated JSON. Rows therefore ARE the
//! canonical encoding; there is no parallel `body_json` to diverge.
//!
//! Concurrency: compare-and-swap on `revision` inside `unchecked_transaction`.
//! `Ok` returns the re-read row; stale preconditions return
//! [`MetaError::Conflict`] with zero writes. Duplicate active attachments and
//! edges are rejected by pre-check plus partial unique indexes as backstop.
//!
//! Pagination: deterministic `*_id TEXT` order with `after` cursors carrying
//! the last seen id. Cursor length is bounded by the contracts vocabulary.

use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, EdgeStatus, Experiment,
    ExperimentStatus, GraphDirection, GraphEndpoint, PROJECT_GRAPH_SCHEMA_VERSION, Project,
    ProjectArtifactRef, ProjectGraphEdge, ProjectGraphPredicate, ProjectRevision, RefStatus,
};
use rusqlite::{OptionalExtension, params};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v3 DDL, executed inside `begin/finish_migration(3)`.
pub(crate) const V3_DDL: &str = r"
CREATE TABLE IF NOT EXISTS projects (
  project_id TEXT PRIMARY KEY,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_projects_scope_status
  ON projects(authority_scope_id, status);
CREATE TABLE IF NOT EXISTS experiments (
  experiment_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_experiments_project
  ON experiments(project_id);
CREATE INDEX IF NOT EXISTS idx_experiments_project_status
  ON experiments(project_id, status);
CREATE TABLE IF NOT EXISTS project_artifact_refs (
  ref_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  experiment_id TEXT,
  object_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  binding_json TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_refs_project
  ON project_artifact_refs(project_id);
CREATE INDEX IF NOT EXISTS idx_refs_project_experiment
  ON project_artifact_refs(project_id, experiment_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_refs_active_tuple
  ON project_artifact_refs(project_id, COALESCE(experiment_id, ''), object_id, kind, binding_json)
  WHERE status = 'active';
CREATE TABLE IF NOT EXISTS project_graph_edges (
  edge_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  subject_kind TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  subject_json TEXT NOT NULL,
  predicate TEXT NOT NULL,
  object_kind TEXT NOT NULL,
  object_id TEXT NOT NULL,
  object_json TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_edges_project_subject
  ON project_graph_edges(project_id, subject_id);
CREATE INDEX IF NOT EXISTS idx_edges_project_object
  ON project_graph_edges(project_id, object_id);
CREATE INDEX IF NOT EXISTS idx_edges_project_predicate
  ON project_graph_edges(project_id, predicate);
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_active_tuple
  ON project_graph_edges(project_id, subject_kind, subject_id, predicate, object_kind, object_id)
  WHERE status = 'active';
";

/// Bounded cursor guard (mirrors the contracts vocabulary).
const CURSOR_MAX_BYTES: usize = 256;

// ---------- value encoding ----------

fn project_status_str(status: medscale_contracts::project_graph::ProjectStatus) -> &'static str {
    match status {
        medscale_contracts::project_graph::ProjectStatus::Active => "active",
        medscale_contracts::project_graph::ProjectStatus::Archived => "archived",
    }
}

fn parse_project_status(
    value: &str,
) -> Result<medscale_contracts::project_graph::ProjectStatus, MetaError> {
    match value {
        "active" => Ok(medscale_contracts::project_graph::ProjectStatus::Active),
        "archived" => Ok(medscale_contracts::project_graph::ProjectStatus::Archived),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown project status {other}"
        ))),
    }
}

fn experiment_status_str(status: ExperimentStatus) -> &'static str {
    match status {
        ExperimentStatus::Draft => "draft",
        ExperimentStatus::Active => "active",
        ExperimentStatus::Completed => "completed",
        ExperimentStatus::Archived => "archived",
    }
}

fn parse_experiment_status(value: &str) -> Result<ExperimentStatus, MetaError> {
    match value {
        "draft" => Ok(ExperimentStatus::Draft),
        "active" => Ok(ExperimentStatus::Active),
        "completed" => Ok(ExperimentStatus::Completed),
        "archived" => Ok(ExperimentStatus::Archived),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown experiment status {other}"
        ))),
    }
}

fn ref_status_str(status: RefStatus) -> &'static str {
    match status {
        RefStatus::Active => "active",
        RefStatus::Detached => "detached",
    }
}

fn parse_ref_status(value: &str) -> Result<RefStatus, MetaError> {
    match value {
        "active" => Ok(RefStatus::Active),
        "detached" => Ok(RefStatus::Detached),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown ref status {other}"
        ))),
    }
}

fn edge_status_str(status: EdgeStatus) -> &'static str {
    match status {
        EdgeStatus::Active => "active",
        EdgeStatus::Removed => "removed",
    }
}

fn parse_edge_status(value: &str) -> Result<EdgeStatus, MetaError> {
    match value {
        "active" => Ok(EdgeStatus::Active),
        "removed" => Ok(EdgeStatus::Removed),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown edge status {other}"
        ))),
    }
}

fn kind_to_str(kind: &ArtifactKind) -> String {
    match kind {
        ArtifactKind::OtherExplicit(id) => format!("other_explicit:{id}"),
        _ => kind.as_str().to_owned(),
    }
}

fn kind_from_str(value: &str) -> Result<ArtifactKind, MetaError> {
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
        other => other
            .strip_prefix("other_explicit:")
            .map(ArtifactKind::other_explicit)
            .transpose()
            .map_err(MetaError::CorruptObjectBody)?
            .ok_or_else(|| MetaError::UnsupportedSchema(format!("unknown artifact kind {other}"))),
    }
}

fn binding_to_json(binding: &ArtifactVersionBinding) -> Result<String, MetaError> {
    serde_json::to_string(binding)
        .map_err(|e| MetaError::CorruptObjectBody(format!("binding encode: {e}")))
}

fn binding_from_json(value: &str) -> Result<ArtifactVersionBinding, MetaError> {
    let binding: ArtifactVersionBinding = serde_json::from_str(value)
        .map_err(|e| MetaError::CorruptObjectBody(format!("binding decode: {e}")))?;
    binding.validate().map_err(MetaError::CorruptObjectBody)?;
    Ok(binding)
}

fn endpoint_parts(endpoint: &GraphEndpoint) -> Result<(String, String, String), MetaError> {
    let json = serde_json::to_string(endpoint)
        .map_err(|e| MetaError::CorruptObjectBody(format!("endpoint encode: {e}")))?;
    match endpoint {
        GraphEndpoint::Artifact(d) => {
            Ok(("artifact".to_owned(), d.object_id.as_str().to_owned(), json))
        }
        GraphEndpoint::Experiment(id) => {
            Ok(("experiment".to_owned(), id.as_str().to_owned(), json))
        }
    }
}

fn endpoint_from_parts(kind: &str, id: &str, json: &str) -> Result<GraphEndpoint, MetaError> {
    let endpoint: GraphEndpoint = serde_json::from_str(json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("endpoint decode: {e}")))?;
    let actual = match &endpoint {
        GraphEndpoint::Artifact(d) => ("artifact", d.object_id.as_str()),
        GraphEndpoint::Experiment(eid) => ("experiment", eid.as_str()),
    };
    if actual.0 != kind || actual.1 != id {
        return Err(MetaError::CorruptObjectBody(
            "endpoint index columns diverge from endpoint payload".to_owned(),
        ));
    }
    endpoint.validate().map_err(MetaError::CorruptObjectBody)?;
    Ok(endpoint)
}

fn check_cursor(after: Option<&str>) -> Result<(), MetaError> {
    if after.is_some_and(|c| c.len() > CURSOR_MAX_BYTES) {
        return Err(MetaError::CorruptObjectBody(
            "cursor exceeds maximum length".to_owned(),
        ));
    }
    Ok(())
}

/// Reads a stored revision. Rejects negative/tampered values instead of
/// wrapping them into huge counters.
fn rev_from_db(revision: i64) -> Result<ProjectRevision, MetaError> {
    ProjectRevision::try_from(revision)
        .map_err(|_| MetaError::CorruptObjectBody("revision out of range".to_owned()))
        .and_then(|rev| {
            if rev < 1 {
                Err(MetaError::CorruptObjectBody(
                    "revision must be positive".to_owned(),
                ))
            } else {
                Ok(rev)
            }
        })
}

fn is_conflict(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

impl SqliteMetaStore {
    /// Allocates one `prefix-N` id from a durable sqlite-backed sequence.
    ///
    /// Spec 074 entity ids come from this counter (not the in-memory authority
    /// sequence) so high-frequency graph ops never touch the memory store and
    /// the frozen full-snapshot sync stays constant-size. Transactional:
    /// crash-safe, no reuse after reopen.
    pub fn alloc_project_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = 'project_id_seq'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let next: u64 = current.as_deref().unwrap_or("0").parse().unwrap_or(0) + 1;
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES ('project_id_seq', ?1)",
            params![next.to_string()],
        )?;
        tx.commit()?;
        Ok(OpaqueId::new(format!("{prefix}-{next}")))
    }
}

// ---------- header helper ----------

fn header_for(
    id: &OpaqueId,
    realm: &str,
    scope: &str,
) -> medscale_contracts::objects::ObjectHeader {
    medscale_contracts::objects::ObjectHeader {
        id: id.clone(),
        schema_version: PROJECT_GRAPH_SCHEMA_VERSION,
        realm_id: RealmId::new(realm),
        authority_scope_id: AuthorityScopeId::new(scope),
    }
}

// ---------- projects ----------

impl SqliteMetaStore {
    /// Inserts a new Project row. Duplicate ids fail as `Conflict`.
    pub fn insert_project(&self, project: &Project) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO projects(project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project.header.id.as_str(),
                project.header.realm_id.as_opaque().as_str(),
                project.header.authority_scope_id.as_opaque().as_str(),
                project.name,
                project.description,
                project_status_str(project.status),
                project.revision as i64,
                project.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate project {}",
                project.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one Project row with integrity validation.
    pub fn get_project(&self, id: &OpaqueId) -> Result<Project, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version
             FROM projects WHERE project_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_project_row(row)
    }

    /// Lists Projects in one scope with deterministic id order and `after` cursor.
    pub fn list_projects(
        &self,
        scope: &AuthorityScopeId,
        status: Option<medscale_contracts::project_graph::ProjectStatus>,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<Project>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let status_str = status.map(project_status_str);
        let mut stmt = self.conn().prepare(
            "SELECT project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version
             FROM projects
             WHERE authority_scope_id = ?1 AND (?2 IS NULL OR status = ?2) AND project_id > ?3
             ORDER BY project_id LIMIT ?4",
        )?;
        let mapped = stmt.query_map(
            params![scope.as_opaque().as_str(), status_str, after, limit],
            map_project_row_result,
        )?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        let next = if out.len() == limit as usize {
            // Drop the lookahead row; the cursor is the last RETURNED id.
            out.pop();
            out.last().map(|p| p.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Compare-and-swap Project metadata. Stale `expected` fails as `Conflict`.
    pub fn update_project_meta(
        &self,
        id: &OpaqueId,
        expected: ProjectRevision,
        name: &str,
        description: Option<&str>,
    ) -> Result<Project, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision, status FROM projects WHERE project_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((current_rev, _)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale project revision: expected {expected}"
            )));
        }
        tx.execute(
            "UPDATE projects SET name = ?1, description = ?2, revision = revision + 1 WHERE project_id = ?3",
            params![name, description, id.as_str()],
        )?;
        tx.commit()?;
        self.get_project(id)
    }

    /// Compare-and-swap Project status (archive/restore). Never touches targets.
    pub fn set_project_status(
        &self,
        id: &OpaqueId,
        expected: ProjectRevision,
        status: medscale_contracts::project_graph::ProjectStatus,
    ) -> Result<Project, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM projects WHERE project_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale project revision: expected {expected}"
            )));
        }
        tx.execute(
            "UPDATE projects SET status = ?1, revision = revision + 1 WHERE project_id = ?2",
            params![project_status_str(status), id.as_str()],
        )?;
        tx.commit()?;
        self.get_project(id)
    }

    /// Restores one Project row exactly (backup restore path only).
    pub fn restore_project_row(&self, project: &Project) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO projects(project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project.header.id.as_str(),
                project.header.realm_id.as_opaque().as_str(),
                project.header.authority_scope_id.as_opaque().as_str(),
                project.name,
                project.description,
                project_status_str(project.status),
                project.revision as i64,
                project.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Full Project scan for snapshot export (bounded callers only).
    pub fn list_all_projects(&self) -> Result<Vec<Project>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version
             FROM projects ORDER BY project_id",
        )?;
        let mapped = stmt.query_map([], map_project_row_result)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        Ok(out)
    }
}

fn to_meta(err: rusqlite::Error) -> MetaError {
    MetaError::Sqlite(err)
}

fn map_project_row_result(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Project, MetaError>> {
    Ok(map_project_row_inner(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, i64>(6)?,
        row.get::<_, i64>(7)?,
    ))
}

fn map_project_row(row: &rusqlite::Row<'_>) -> Result<Project, MetaError> {
    map_project_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
        row.get(6).map_err(MetaError::Sqlite)?,
        row.get(7).map_err(MetaError::Sqlite)?,
    )
}

#[allow(clippy::too_many_arguments)]
fn map_project_row_inner(
    id: String,
    realm: String,
    scope: String,
    name: String,
    description: Option<String>,
    status: String,
    revision: i64,
    schema_version: i64,
) -> Result<Project, MetaError> {
    let status = parse_project_status(&status)?;
    let project = Project {
        header: header_for(&OpaqueId::new(id), &realm, &scope),
        revision: rev_from_db(revision)?,
        name,
        description,
        status,
    };
    if u64::from(PROJECT_GRAPH_SCHEMA_VERSION) != schema_version as u64 {
        return Err(MetaError::UnsupportedSchema(format!(
            "project schema {schema_version}"
        )));
    }
    Ok(project)
}

// ---------- experiments ----------

impl SqliteMetaStore {
    /// Inserts a new Experiment row. Duplicate ids fail as `Conflict`.
    pub fn insert_experiment(&self, experiment: &Experiment) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO experiments(experiment_id, project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                experiment.header.id.as_str(),
                experiment.project_id.as_str(),
                experiment.header.realm_id.as_opaque().as_str(),
                experiment.header.authority_scope_id.as_opaque().as_str(),
                experiment.name,
                experiment.description,
                experiment_status_str(experiment.status),
                experiment.revision as i64,
                experiment.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate experiment {}",
                experiment.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one Experiment row with integrity validation.
    pub fn get_experiment(&self, id: &OpaqueId) -> Result<Experiment, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT experiment_id, project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version
             FROM experiments WHERE experiment_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_experiment_row(row)
    }

    /// Lists Experiments of one Project with deterministic id order.
    pub fn list_experiments(
        &self,
        project_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<Experiment>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT experiment_id, project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version
             FROM experiments WHERE project_id = ?1 AND experiment_id > ?2
             ORDER BY experiment_id LIMIT ?3",
        )?;
        let mapped = stmt.query_map(params![project_id.as_str(), after, limit], |row| {
            map_experiment_row_result(row)
        })?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        let next = if out.len() == limit as usize {
            // Drop the lookahead row; the cursor is the last RETURNED id.
            out.pop();
            out.last().map(|e| e.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Compare-and-swap Experiment metadata.
    pub fn update_experiment_meta(
        &self,
        id: &OpaqueId,
        expected: ProjectRevision,
        name: &str,
        description: Option<&str>,
    ) -> Result<Experiment, MetaError> {
        cas_experiment_field(
            self,
            id,
            expected,
            "UPDATE experiments SET name = ?1, description = ?2, revision = revision + 1 WHERE experiment_id = ?3",
            name,
            description,
        )
    }

    /// Compare-and-swap Experiment status with lifecycle validation in Core.
    /// Storage enforces the revision gate; Core owns transition semantics.
    pub fn set_experiment_status(
        &self,
        id: &OpaqueId,
        expected: ProjectRevision,
        status: ExperimentStatus,
    ) -> Result<Experiment, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM experiments WHERE experiment_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale experiment revision: expected {expected}"
            )));
        }
        tx.execute(
            "UPDATE experiments SET status = ?1, revision = revision + 1 WHERE experiment_id = ?2",
            params![experiment_status_str(status), id.as_str()],
        )?;
        tx.commit()?;
        self.get_experiment(id)
    }

    /// Restores one Experiment row exactly (backup restore path only).
    pub fn restore_experiment_row(&self, experiment: &Experiment) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO experiments(experiment_id, project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                experiment.header.id.as_str(),
                experiment.project_id.as_str(),
                experiment.header.realm_id.as_opaque().as_str(),
                experiment.header.authority_scope_id.as_opaque().as_str(),
                experiment.name,
                experiment.description,
                experiment_status_str(experiment.status),
                experiment.revision as i64,
                experiment.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Full Experiment scan for snapshot export (bounded callers only).
    pub fn list_all_experiments(&self) -> Result<Vec<Experiment>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT experiment_id, project_id, realm_id, authority_scope_id, name, description, status, revision, schema_version
             FROM experiments ORDER BY experiment_id",
        )?;
        let mapped = stmt.query_map([], map_experiment_row_result)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        Ok(out)
    }

    /// Counts Experiments of one Project (summary support).
    pub fn count_experiments(&self, project_id: &OpaqueId) -> Result<u64, MetaError> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM experiments WHERE project_id = ?1",
            params![project_id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }
}

fn cas_experiment_field(
    store: &SqliteMetaStore,
    id: &OpaqueId,
    expected: ProjectRevision,
    sql: &str,
    name: &str,
    description: Option<&str>,
) -> Result<Experiment, MetaError> {
    let tx = store.conn().unchecked_transaction()?;
    let current: Option<i64> = tx
        .query_row(
            "SELECT revision FROM experiments WHERE experiment_id = ?1",
            params![id.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    let Some(current_rev) = current else {
        return Err(MetaError::NotFound);
    };
    if current_rev != expected as i64 {
        return Err(MetaError::Conflict(format!(
            "stale experiment revision: expected {expected}"
        )));
    }
    tx.execute(sql, params![name, description, id.as_str()])?;
    tx.commit()?;
    store.get_experiment(id)
}

fn map_experiment_row_result(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Result<Experiment, MetaError>> {
    Ok(map_experiment_row_inner(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, Option<String>>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, i64>(7)?,
        row.get::<_, i64>(8)?,
    ))
}

fn map_experiment_row(row: &rusqlite::Row<'_>) -> Result<Experiment, MetaError> {
    map_experiment_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
        row.get(6).map_err(MetaError::Sqlite)?,
        row.get(7).map_err(MetaError::Sqlite)?,
        row.get(8).map_err(MetaError::Sqlite)?,
    )
}

#[allow(clippy::too_many_arguments)]
fn map_experiment_row_inner(
    id: String,
    project_id: String,
    realm: String,
    scope: String,
    name: String,
    description: Option<String>,
    status: String,
    revision: i64,
    schema_version: i64,
) -> Result<Experiment, MetaError> {
    if u64::from(PROJECT_GRAPH_SCHEMA_VERSION) != schema_version as u64 {
        return Err(MetaError::UnsupportedSchema(format!(
            "experiment schema {schema_version}"
        )));
    }
    Ok(Experiment {
        header: header_for(&OpaqueId::new(id), &realm, &scope),
        project_id: OpaqueId::new(project_id),
        revision: rev_from_db(revision)?,
        name,
        description,
        status: parse_experiment_status(&status)?,
    })
}

// ---------- artifact refs ----------

impl SqliteMetaStore {
    /// Attaches one artifact ref. A repeat of the same active tuple fails as
    /// `Conflict` (idempotent-or-conflict is decided in Core; storage never
    /// writes a silent duplicate).
    pub fn attach_ref(&self, reference: &ProjectArtifactRef) -> Result<(), MetaError> {
        let binding = binding_to_json(&reference.artifact.binding)?;
        let result = self.conn().execute(
            "INSERT INTO project_artifact_refs(ref_id, project_id, experiment_id, object_id, kind, binding_json, status, revision, realm_id, authority_scope_id, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                reference.header.id.as_str(),
                reference.project_id.as_str(),
                reference.experiment_id.as_ref().map(OpaqueId::as_str),
                reference.artifact.object_id.as_str(),
                kind_to_str(&reference.artifact.kind),
                binding,
                ref_status_str(reference.status),
                reference.revision as i64,
                reference.header.realm_id.as_opaque().as_str(),
                reference.header.authority_scope_id.as_opaque().as_str(),
                reference.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(
                "duplicate project artifact attachment".to_owned(),
            )),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one ref row with integrity validation.
    pub fn get_ref(&self, id: &OpaqueId) -> Result<ProjectArtifactRef, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT ref_id, project_id, experiment_id, object_id, kind, binding_json, status, revision, realm_id, authority_scope_id, schema_version
             FROM project_artifact_refs WHERE ref_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_ref_row(row)
    }

    /// Lists refs of one Project (optionally one Experiment) in id order.
    pub fn list_refs(
        &self,
        project_id: &OpaqueId,
        experiment_id: Option<&OpaqueId>,
        active_only: bool,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<ProjectArtifactRef>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let experiment = experiment_id.map(OpaqueId::as_str);
        let mut stmt = self.conn().prepare(
            "SELECT ref_id, project_id, experiment_id, object_id, kind, binding_json, status, revision, realm_id, authority_scope_id, schema_version
             FROM project_artifact_refs
             WHERE project_id = ?1 AND (?2 IS NULL OR COALESCE(experiment_id, '') = COALESCE(?2, ''))
               AND (?3 = 0 OR status = 'active') AND ref_id > ?4
             ORDER BY ref_id LIMIT ?5",
        )?;
        let mapped = stmt.query_map(
            params![
                project_id.as_str(),
                experiment,
                i64::from(active_only),
                after,
                limit
            ],
            map_ref_row_result,
        )?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        let next = if out.len() == limit as usize {
            // Drop the lookahead row; the cursor is the last RETURNED id.
            out.pop();
            out.last().map(|r| r.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Compare-and-swap ref status (detach). The canonical target is untouched.
    pub fn set_ref_status(
        &self,
        id: &OpaqueId,
        expected: ProjectRevision,
        status: RefStatus,
    ) -> Result<ProjectArtifactRef, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision, status FROM project_artifact_refs WHERE ref_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((current_rev, current_status)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale ref revision: expected {expected}"
            )));
        }
        if current_status == ref_status_str(status) {
            return Err(MetaError::Conflict(
                "ref already in requested status".to_owned(),
            ));
        }
        tx.execute(
            "UPDATE project_artifact_refs SET status = ?1, revision = revision + 1 WHERE ref_id = ?2",
            params![ref_status_str(status), id.as_str()],
        )?;
        tx.commit()?;
        self.get_ref(id)
    }

    /// Restores one ref row exactly (backup restore path only).
    pub fn restore_ref_row(&self, reference: &ProjectArtifactRef) -> Result<(), MetaError> {
        let binding = binding_to_json(&reference.artifact.binding)?;
        self.conn().execute(
            "INSERT OR REPLACE INTO project_artifact_refs(ref_id, project_id, experiment_id, object_id, kind, binding_json, status, revision, realm_id, authority_scope_id, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                reference.header.id.as_str(),
                reference.project_id.as_str(),
                reference.experiment_id.as_ref().map(OpaqueId::as_str),
                reference.artifact.object_id.as_str(),
                kind_to_str(&reference.artifact.kind),
                binding,
                ref_status_str(reference.status),
                reference.revision as i64,
                reference.header.realm_id.as_opaque().as_str(),
                reference.header.authority_scope_id.as_opaque().as_str(),
                reference.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Full ref scan for snapshot export (bounded callers only).
    pub fn list_all_refs(&self) -> Result<Vec<ProjectArtifactRef>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT ref_id, project_id, experiment_id, object_id, kind, binding_json, status, revision, realm_id, authority_scope_id, schema_version
             FROM project_artifact_refs ORDER BY ref_id",
        )?;
        let mapped = stmt.query_map([], map_ref_row_result)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        Ok(out)
    }

    /// Counts active refs of one Project (summary support).
    pub fn count_active_refs(&self, project_id: &OpaqueId) -> Result<u64, MetaError> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM project_artifact_refs WHERE project_id = ?1 AND status = 'active'",
            params![project_id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }
}

fn map_ref_row_result(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Result<ProjectArtifactRef, MetaError>> {
    Ok(map_ref_row_inner(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, Option<String>>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, i64>(7)?,
        row.get::<_, String>(8)?,
        row.get::<_, String>(9)?,
        row.get::<_, i64>(10)?,
    ))
}

fn map_ref_row(row: &rusqlite::Row<'_>) -> Result<ProjectArtifactRef, MetaError> {
    map_ref_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
        row.get(6).map_err(MetaError::Sqlite)?,
        row.get(7).map_err(MetaError::Sqlite)?,
        row.get(8).map_err(MetaError::Sqlite)?,
        row.get(9).map_err(MetaError::Sqlite)?,
        row.get(10).map_err(MetaError::Sqlite)?,
    )
}

#[allow(clippy::too_many_arguments)]
fn map_ref_row_inner(
    id: String,
    project_id: String,
    experiment_id: Option<String>,
    object_id: String,
    kind: String,
    binding_json: String,
    status: String,
    revision: i64,
    realm: String,
    scope: String,
    schema_version: i64,
) -> Result<ProjectArtifactRef, MetaError> {
    if u64::from(PROJECT_GRAPH_SCHEMA_VERSION) != schema_version as u64 {
        return Err(MetaError::UnsupportedSchema(format!(
            "ref schema {schema_version}"
        )));
    }
    let reference = ProjectArtifactRef {
        header: header_for(&OpaqueId::new(id), &realm, &scope),
        project_id: OpaqueId::new(project_id),
        experiment_id: experiment_id.map(OpaqueId::new),
        artifact: ArtifactDescriptor {
            object_id: OpaqueId::new(object_id),
            kind: kind_from_str(&kind)?,
            binding: binding_from_json(&binding_json)?,
        },
        revision: rev_from_db(revision)?,
        status: parse_ref_status(&status)?,
    };
    reference
        .artifact
        .validate()
        .map_err(MetaError::CorruptObjectBody)?;
    Ok(reference)
}

// ---------- graph edges ----------

impl SqliteMetaStore {
    /// Creates one edge. A repeat of the same active tuple fails as `Conflict`.
    pub fn create_edge(&self, edge: &ProjectGraphEdge) -> Result<(), MetaError> {
        let (subject_kind, subject_id, subject_json) = endpoint_parts(&edge.subject)?;
        let (object_kind, object_id, object_json) = endpoint_parts(&edge.object)?;
        let result = self.conn().execute(
            "INSERT INTO project_graph_edges(edge_id, project_id, subject_kind, subject_id, subject_json, predicate, object_kind, object_id, object_json, status, revision, realm_id, authority_scope_id, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                edge.header.id.as_str(),
                edge.project_id.as_str(),
                subject_kind,
                subject_id,
                subject_json,
                edge.predicate.as_str(),
                object_kind,
                object_id,
                object_json,
                edge_status_str(edge.status),
                edge.revision as i64,
                edge.header.realm_id.as_opaque().as_str(),
                edge.header.authority_scope_id.as_opaque().as_str(),
                edge.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(
                "duplicate project graph edge".to_owned(),
            )),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one edge row with integrity validation.
    pub fn get_edge(&self, id: &OpaqueId) -> Result<ProjectGraphEdge, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT edge_id, project_id, subject_kind, subject_id, subject_json, predicate, object_kind, object_id, object_json, status, revision, realm_id, authority_scope_id, schema_version
             FROM project_graph_edges WHERE edge_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_edge_row(row)
    }

    /// Compare-and-swap edge status (remove). Endpoints are never deleted.
    pub fn set_edge_status(
        &self,
        id: &OpaqueId,
        expected: ProjectRevision,
        status: EdgeStatus,
    ) -> Result<ProjectGraphEdge, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision, status FROM project_graph_edges WHERE edge_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((current_rev, current_status)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale edge revision: expected {expected}"
            )));
        }
        if current_status == edge_status_str(status) {
            return Err(MetaError::Conflict(
                "edge already in requested status".to_owned(),
            ));
        }
        tx.execute(
            "UPDATE project_graph_edges SET status = ?1, revision = revision + 1 WHERE edge_id = ?2",
            params![edge_status_str(status), id.as_str()],
        )?;
        tx.commit()?;
        self.get_edge(id)
    }

    /// Bounded single-hop neighbor query in deterministic edge-id order.
    ///
    /// Storage clamps `limit` defensively; Core enforces the contracts bounds
    /// first. Returns the page plus the `after` cursor for the next page.
    pub fn query_neighbors(
        &self,
        project_id: &OpaqueId,
        start: &GraphEndpoint,
        predicates: Option<&[ProjectGraphPredicate]>,
        direction: GraphDirection,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<ProjectGraphEdge>, Option<String>), MetaError> {
        check_cursor(after)?;
        let (_, start_id, _) = endpoint_parts(start)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let direction_sql = match direction {
            GraphDirection::Outgoing => "subject_id = ?2",
            GraphDirection::Incoming => "object_id = ?2",
            GraphDirection::Both => "(subject_id = ?2 OR object_id = ?2)",
        };
        let sql = format!(
            "SELECT edge_id, project_id, subject_kind, subject_id, subject_json, predicate, object_kind, object_id, object_json, status, revision, realm_id, authority_scope_id, schema_version
             FROM project_graph_edges
             WHERE project_id = ?1 AND {direction_sql} AND status = 'active'
               AND (?3 = '' OR predicate IN (SELECT value FROM json_each('[' || ?3 || ']')))
               AND edge_id > ?4
             ORDER BY edge_id LIMIT ?5"
        );
        // Predicate filter via a JSON list probe keeps the statement static.
        let quoted = predicates
            .unwrap_or(&[])
            .iter()
            .map(|p| format!("\"{}\"", p.as_str()))
            .collect::<Vec<_>>()
            .join(",");
        let mut stmt = self.conn().prepare(&sql)?;
        let mapped = stmt.query_map(
            params![project_id.as_str(), start_id, quoted, after, limit],
            map_edge_row_result,
        )?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        let next = if out.len() == limit as usize {
            // Drop the lookahead row; the cursor is the last RETURNED id.
            out.pop();
            out.last().map(|e| e.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Restores one edge row exactly (backup restore path only).
    pub fn restore_edge_row(&self, edge: &ProjectGraphEdge) -> Result<(), MetaError> {
        let (subject_kind, subject_id, subject_json) = endpoint_parts(&edge.subject)?;
        let (object_kind, object_id, object_json) = endpoint_parts(&edge.object)?;
        self.conn().execute(
            "INSERT OR REPLACE INTO project_graph_edges(edge_id, project_id, subject_kind, subject_id, subject_json, predicate, object_kind, object_id, object_json, status, revision, realm_id, authority_scope_id, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                edge.header.id.as_str(),
                edge.project_id.as_str(),
                subject_kind,
                subject_id,
                subject_json,
                edge.predicate.as_str(),
                object_kind,
                object_id,
                object_json,
                edge_status_str(edge.status),
                edge.revision as i64,
                edge.header.realm_id.as_opaque().as_str(),
                edge.header.authority_scope_id.as_opaque().as_str(),
                edge.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Full edge scan for snapshot export (bounded callers only).
    pub fn list_all_edges(&self) -> Result<Vec<ProjectGraphEdge>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT edge_id, project_id, subject_kind, subject_id, subject_json, predicate, object_kind, object_id, object_json, status, revision, realm_id, authority_scope_id, schema_version
             FROM project_graph_edges ORDER BY edge_id",
        )?;
        let mapped = stmt.query_map([], map_edge_row_result)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        Ok(out)
    }

    /// Counts active edges of one Project (summary support).
    pub fn count_active_edges(&self, project_id: &OpaqueId) -> Result<u64, MetaError> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM project_graph_edges WHERE project_id = ?1 AND status = 'active'",
            params![project_id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /// Lists active edges of one Project in edge-id order (context slices).
    pub fn list_edges(
        &self,
        project_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<ProjectGraphEdge>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT edge_id, project_id, subject_kind, subject_id, subject_json, predicate, object_kind, object_id, object_json, status, revision, realm_id, authority_scope_id, schema_version
             FROM project_graph_edges
             WHERE project_id = ?1 AND status = 'active' AND edge_id > ?2
             ORDER BY edge_id LIMIT ?3",
        )?;
        let mapped = stmt.query_map(
            params![project_id.as_str(), after, limit],
            map_edge_row_result,
        )?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        let next = if out.len() == limit as usize {
            // Drop the lookahead row; the cursor is the last RETURNED id.
            out.pop();
            out.last().map(|e| e.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }
}

fn map_edge_row_result(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Result<ProjectGraphEdge, MetaError>> {
    Ok(map_edge_row_inner(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, String>(7)?,
        row.get::<_, String>(8)?,
        row.get::<_, String>(9)?,
        row.get::<_, i64>(10)?,
        row.get::<_, String>(11)?,
        row.get::<_, String>(12)?,
        row.get::<_, i64>(13)?,
    ))
}

fn map_edge_row(row: &rusqlite::Row<'_>) -> Result<ProjectGraphEdge, MetaError> {
    map_edge_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
        row.get(6).map_err(MetaError::Sqlite)?,
        row.get(7).map_err(MetaError::Sqlite)?,
        row.get(8).map_err(MetaError::Sqlite)?,
        row.get(9).map_err(MetaError::Sqlite)?,
        row.get(10).map_err(MetaError::Sqlite)?,
        row.get(11).map_err(MetaError::Sqlite)?,
        row.get(12).map_err(MetaError::Sqlite)?,
        row.get(13).map_err(MetaError::Sqlite)?,
    )
}

#[allow(clippy::too_many_arguments)]
fn map_edge_row_inner(
    id: String,
    project_id: String,
    subject_kind: String,
    subject_id: String,
    subject_json: String,
    predicate: String,
    object_kind: String,
    object_id: String,
    object_json: String,
    status: String,
    revision: i64,
    realm: String,
    scope: String,
    schema_version: i64,
) -> Result<ProjectGraphEdge, MetaError> {
    if u64::from(PROJECT_GRAPH_SCHEMA_VERSION) != schema_version as u64 {
        return Err(MetaError::UnsupportedSchema(format!(
            "edge schema {schema_version}"
        )));
    }
    let predicate = ProjectGraphPredicate::parse(&predicate)
        .ok_or_else(|| MetaError::UnsupportedSchema(format!("stored predicate {predicate}")))?;
    Ok(ProjectGraphEdge {
        header: header_for(&OpaqueId::new(id), &realm, &scope),
        project_id: OpaqueId::new(project_id),
        subject: endpoint_from_parts(&subject_kind, &subject_id, &subject_json)?,
        predicate,
        object: endpoint_from_parts(&object_kind, &object_id, &object_json)?,
        revision: rev_from_db(revision)?,
        status: parse_edge_status(&status)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::project_graph::{
        ExperimentStatus as ES, ProjectGraphPredicate as Pred, ProjectStatus as PS,
        ReferenceResolution,
    };

    #[test]
    fn kind_encoding_round_trips() {
        for kind in [
            ArtifactKind::SourceRecord,
            ArtifactKind::ClinicalAssertion,
            ArtifactKind::PackManifest,
            ArtifactKind::EvidenceDocument,
        ] {
            assert_eq!(kind_from_str(&kind_to_str(&kind)).unwrap(), kind);
        }
        let other = ArtifactKind::other_explicit("custom.v1").unwrap();
        assert_eq!(kind_from_str(&kind_to_str(&other)).unwrap(), other);
        assert!(kind_from_str("proves").is_err());
    }

    #[test]
    fn predicate_parse_is_closed_on_read_path() {
        assert_eq!(
            ProjectGraphPredicate::parse("contains"),
            Some(Pred::Contains)
        );
        assert!(ProjectGraphPredicate::parse("derived_from").is_none());
    }

    #[test]
    fn endpoint_parts_reject_divergence() {
        let endpoint = GraphEndpoint::Experiment(OpaqueId::new("exp-1"));
        let (kind, id, json) = endpoint_parts(&endpoint).unwrap();
        assert_eq!((kind.as_str(), id.as_str()), ("experiment", "exp-1"));
        assert!(endpoint_from_parts("artifact", "exp-1", &json).is_err());
        assert!(endpoint_from_parts("experiment", "exp-1", &json).is_ok());
    }

    #[test]
    fn status_vocabularies_fail_closed() {
        assert!(parse_project_status("deleted").is_err());
        assert!(parse_experiment_status("deleted").is_err());
        assert!(parse_ref_status("deleted").is_err());
        assert!(parse_edge_status("deleted").is_err());
        assert_eq!(parse_project_status("archived").unwrap(), PS::Archived);
        assert_eq!(parse_experiment_status("draft").unwrap(), ES::Draft);
    }

    #[test]
    fn reference_resolution_variants_are_addressable() {
        // Guards the resolution vocabulary used by 074-C against typos.
        let _ = ReferenceResolution::Current;
    }
}

//! MedAgent Workbench durable rows (Spec 077, storage schema v6).
//!
//! Dedicated tables inside the existing encrypted metadata DB: no second
//! database. `ContextManifest.selected_artifacts` and
//! `AgentCapabilityManifest.granted_tool_kinds` travel as validated JSON on
//! their owning row (mirroring `collab_approval_requests.assignees_json`),
//! not a separate child table -- there is exactly one row per manifest,
//! never a partially-visible artifact/capability list.
//!
//! Concurrency: compare-and-swap on `revision` inside `unchecked_transaction`
//! for `medagent_identities`/`medagent_runs`. `AgentRunState` transition
//! legality is a Core concern (frozen table lives in contracts); storage
//! only enforces the revision CAS.
//!
//! Every mutation that commits a run's terminal state also commits its
//! `RunReceipt` in the same transaction (`migration.md` section 5): there
//! is no code path that commits one without the other. This is the exact
//! invariant Spec 076's own exact-range review found missing from its
//! restore path (`evidence/076-collaboration-substrate/EXACT_RANGE_REVIEW.md`)
//! -- 077's restore path re-verifies it explicitly (see `restore_v6` in
//! `backup.rs`).

use medscale_contracts::medagent::{
    AgentCapabilityManifest, AgentIdentity, AgentIdentityStatus, AgentProposal, AgentRun,
    AgentRunState, AgentTurn, AgentTurnKind, ContextManifest, RunReceipt, ToolInvocation,
    ToolInvocationStatus, ToolKind, ToolReceipt,
};
use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId};
use medscale_contracts::project_graph::ArtifactDescriptor;
use rusqlite::{OptionalExtension, params};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v6 DDL, executed inside `begin/finish_migration(6)`.
pub(crate) const V6_DDL: &str = r"
CREATE TABLE IF NOT EXISTS medagent_identities (
  agent_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  pack_id TEXT NOT NULL,
  pack_version TEXT NOT NULL,
  display_name TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_medagent_identities_project
  ON medagent_identities(project_id, status);
CREATE TABLE IF NOT EXISTS medagent_capability_manifests (
  agent_id TEXT PRIMARY KEY,
  granted_tool_kinds_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS medagent_context_manifests (
  context_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  selected_artifacts_json TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_medagent_context_manifests_project
  ON medagent_context_manifests(project_id);
CREATE TABLE IF NOT EXISTS medagent_runs (
  run_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  context_id TEXT NOT NULL,
  prompt TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_medagent_runs_project
  ON medagent_runs(project_id, status);
CREATE INDEX IF NOT EXISTS idx_medagent_runs_agent
  ON medagent_runs(agent_id, status);
CREATE TABLE IF NOT EXISTS medagent_turns (
  turn_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  seq INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_medagent_turns_run_seq
  ON medagent_turns(run_id, seq);
CREATE TABLE IF NOT EXISTS medagent_tool_invocations (
  invocation_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  turn_seq INTEGER NOT NULL,
  kind TEXT NOT NULL,
  arguments_json TEXT NOT NULL,
  status TEXT NOT NULL,
  refusal_reason TEXT,
  seq INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_medagent_tool_invocations_run_seq
  ON medagent_tool_invocations(run_id, seq);
CREATE TABLE IF NOT EXISTS medagent_tool_receipts (
  receipt_id TEXT PRIMARY KEY,
  invocation_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  result_json TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_medagent_tool_receipts_invocation
  ON medagent_tool_receipts(invocation_id);
CREATE TABLE IF NOT EXISTS medagent_run_receipts (
  run_id TEXT PRIMARY KEY,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  pack_id TEXT NOT NULL,
  pack_version TEXT NOT NULL,
  context_id TEXT NOT NULL,
  context_revision INTEGER NOT NULL,
  tool_invocation_ids_json TEXT NOT NULL,
  final_state TEXT NOT NULL,
  failure_reason TEXT,
  schema_version INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS medagent_proposals (
  agent_proposal_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  proposal_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_medagent_proposals_run
  ON medagent_proposals(run_id);
";

// ---------------------------------------------------------------------------
// small helpers (deliberately duplicated from collaboration.rs rather than
// exported from it, so this module never depends on Spec 076's closed file)
// ---------------------------------------------------------------------------

fn is_conflict(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(f, _)
        if f.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

fn revision_to_i64(revision: u64) -> i64 {
    i64::try_from(revision).unwrap_or(i64::MAX)
}

fn revision_from_i64(value: i64, what: &str) -> Result<u64, MetaError> {
    u64::try_from(value).map_err(|_| MetaError::CorruptObjectBody(format!("bad {what} revision")))
}

fn header_of(
    id: &str,
    realm: &str,
    scope: &str,
    schema_version: i64,
    default_schema_version: u32,
) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: u32::try_from(schema_version).unwrap_or(default_schema_version),
        realm_id: RealmId::new(realm),
        authority_scope_id: AuthorityScopeId::new(scope),
    }
}

fn agent_identity_status_str(status: AgentIdentityStatus) -> &'static str {
    status.as_str()
}

fn parse_agent_identity_status(value: &str) -> Result<AgentIdentityStatus, MetaError> {
    AgentIdentityStatus::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn agent_run_state_str(state: AgentRunState) -> &'static str {
    state.as_str()
}

fn parse_agent_run_state(value: &str) -> Result<AgentRunState, MetaError> {
    AgentRunState::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn agent_turn_kind_str(kind: AgentTurnKind) -> &'static str {
    kind.as_str()
}

fn parse_agent_turn_kind(value: &str) -> Result<AgentTurnKind, MetaError> {
    AgentTurnKind::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn tool_kind_str(kind: ToolKind) -> &'static str {
    kind.as_str()
}

fn parse_tool_kind(value: &str) -> Result<ToolKind, MetaError> {
    ToolKind::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn tool_invocation_status_str(status: ToolInvocationStatus) -> &'static str {
    status.as_str()
}

fn parse_tool_invocation_status(value: &str) -> Result<ToolInvocationStatus, MetaError> {
    ToolInvocationStatus::parse(value).map_err(MetaError::UnsupportedSchema)
}

// ---------------------------------------------------------------------------
// id allocation (reuses the durable sqlite-backed sequence pattern)
// ---------------------------------------------------------------------------

impl SqliteMetaStore {
    /// Allocates one `prefix-N` MedAgent id from a durable sqlite-backed
    /// sequence, independent of every other spec's id sequence space.
    pub fn alloc_medagent_id(&self, seq_key: &str, prefix: &str) -> Result<OpaqueId, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = ?1",
                params![seq_key],
                |row| row.get(0),
            )
            .optional()?;
        let next: u64 = match current.as_deref() {
            None => 1,
            Some(raw) => {
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("id sequence {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, next.to_string()],
        )?;
        tx.commit()?;
        Ok(OpaqueId::new(format!("{prefix}-{next}")))
    }

    /// Allocates the next `seq` for one append-only family scoped by
    /// `scope_key` (e.g. a run id for turns/tool invocations).
    fn next_medagent_seq(&self, seq_key: &str) -> Result<u64, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = ?1",
                params![seq_key],
                |row| row.get(0),
            )
            .optional()?;
        let next: u64 = match current.as_deref() {
            None => 1,
            Some(raw) => {
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("seq counter {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, next.to_string()],
        )?;
        tx.commit()?;
        Ok(next)
    }
}

// ---------------------------------------------------------------------------
// AgentIdentity + AgentCapabilityManifest
// ---------------------------------------------------------------------------

fn map_agent_identity_row(row: &rusqlite::Row<'_>) -> Result<AgentIdentity, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let project_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let pack_id: String = row.get(4).map_err(MetaError::Sqlite)?;
    let pack_version: String = row.get(5).map_err(MetaError::Sqlite)?;
    let display_name: String = row.get(6).map_err(MetaError::Sqlite)?;
    let status: String = row.get(7).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(8).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(9).map_err(MetaError::Sqlite)?;
    Ok(AgentIdentity {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "agent identity")?,
        project_id: OpaqueId::new(project_id),
        pack_id: OpaqueId::new(pack_id),
        pack_version,
        display_name,
        status: parse_agent_identity_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new agent identity and its capability manifest in one
    /// transaction: neither is ever visible without the other.
    pub fn insert_agent_identity_with_capabilities(
        &self,
        identity: &AgentIdentity,
        manifest: &AgentCapabilityManifest,
    ) -> Result<(), MetaError> {
        let granted_json = serde_json::to_string(&manifest.granted_tool_kinds)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO medagent_identities(agent_id, project_id, realm_id, authority_scope_id, pack_id, pack_version, display_name, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                identity.header.id.as_str(),
                identity.project_id.as_str(),
                identity.header.realm_id.as_opaque().as_str(),
                identity.header.authority_scope_id.as_opaque().as_str(),
                identity.pack_id.as_str(),
                identity.pack_version,
                identity.display_name,
                agent_identity_status_str(identity.status),
                revision_to_i64(identity.revision),
                identity.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!(
                    "duplicate agent identity {}",
                    identity.header.id.as_str()
                ))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        tx.execute(
            "INSERT INTO medagent_capability_manifests(agent_id, granted_tool_kinds_json)
             VALUES (?1, ?2)",
            params![identity.header.id.as_str(), granted_json],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Reads one agent identity row.
    pub fn get_agent_identity(&self, id: &OpaqueId) -> Result<AgentIdentity, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT agent_id, project_id, realm_id, authority_scope_id, pack_id, pack_version, display_name, status, revision, schema_version
             FROM medagent_identities WHERE agent_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_agent_identity_row(row)
    }

    /// Reads one agent identity's capability manifest.
    pub fn get_capability_manifest(
        &self,
        agent_id: &OpaqueId,
    ) -> Result<AgentCapabilityManifest, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT granted_tool_kinds_json FROM medagent_capability_manifests WHERE agent_id = ?1",
        )?;
        let mut rows = stmt.query(params![agent_id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        let json: String = row.get(0).map_err(MetaError::Sqlite)?;
        let kinds_str: Vec<String> =
            serde_json::from_str(&json).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let granted_tool_kinds = kinds_str
            .iter()
            .map(|s| parse_tool_kind(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AgentCapabilityManifest {
            agent_identity_id: agent_id.clone(),
            granted_tool_kinds,
        })
    }

    /// Lists agent identities in one Project, newest-id-last.
    pub fn list_agent_identities(
        &self,
        project_id: &OpaqueId,
        limit: u32,
    ) -> Result<Vec<AgentIdentity>, MetaError> {
        let limit = i64::from(limit.clamp(1, 200));
        let mut stmt = self.conn().prepare(
            "SELECT agent_id, project_id, realm_id, authority_scope_id, pack_id, pack_version, display_name, status, revision, schema_version
             FROM medagent_identities WHERE project_id = ?1 ORDER BY agent_id LIMIT ?2",
        )?;
        let mut rows = stmt.query(params![project_id.as_str(), limit])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_agent_identity_row(row)?);
        }
        Ok(out)
    }

    /// Revokes an agent identity (status only; CAS on `revision`).
    pub fn revoke_agent_identity(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<AgentIdentity, MetaError> {
        let current = self.get_agent_identity(id)?;
        if current.revision != expected_revision {
            return Err(MetaError::Conflict(format!(
                "stale revision: expected {expected_revision}, current is {}",
                current.revision
            )));
        }
        let next_revision = current.revision + 1;
        let updated = self.conn().execute(
            "UPDATE medagent_identities SET status = ?1, revision = ?2 WHERE agent_id = ?3 AND revision = ?4",
            params![
                agent_identity_status_str(AgentIdentityStatus::Revoked),
                revision_to_i64(next_revision),
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent revoke".to_owned()));
        }
        self.get_agent_identity(id)
    }
}

// ---------------------------------------------------------------------------
// ContextManifest
// ---------------------------------------------------------------------------

fn map_context_manifest_row(row: &rusqlite::Row<'_>) -> Result<ContextManifest, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let project_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let artifacts_json: String = row.get(4).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(5).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(6).map_err(MetaError::Sqlite)?;
    let selected_artifacts: Vec<ArtifactDescriptor> = serde_json::from_str(&artifacts_json)
        .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
    Ok(ContextManifest {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "context manifest")?,
        project_id: OpaqueId::new(project_id),
        selected_artifacts,
    })
}

impl SqliteMetaStore {
    /// Inserts a new context manifest.
    pub fn insert_context_manifest(&self, manifest: &ContextManifest) -> Result<(), MetaError> {
        let artifacts_json = serde_json::to_string(&manifest.selected_artifacts)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let result = self.conn().execute(
            "INSERT INTO medagent_context_manifests(context_id, project_id, realm_id, authority_scope_id, selected_artifacts_json, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                manifest.header.id.as_str(),
                manifest.project_id.as_str(),
                manifest.header.realm_id.as_opaque().as_str(),
                manifest.header.authority_scope_id.as_opaque().as_str(),
                artifacts_json,
                revision_to_i64(manifest.revision),
                manifest.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate context manifest {}",
                manifest.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one context manifest.
    pub fn get_context_manifest(&self, id: &OpaqueId) -> Result<ContextManifest, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT context_id, project_id, realm_id, authority_scope_id, selected_artifacts_json, revision, schema_version
             FROM medagent_context_manifests WHERE context_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_context_manifest_row(row)
    }
}

// ---------------------------------------------------------------------------
// AgentRun + AgentTurn
// ---------------------------------------------------------------------------

fn map_agent_run_row(row: &rusqlite::Row<'_>) -> Result<AgentRun, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let project_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let agent_id: String = row.get(4).map_err(MetaError::Sqlite)?;
    let context_id: String = row.get(5).map_err(MetaError::Sqlite)?;
    let prompt: String = row.get(6).map_err(MetaError::Sqlite)?;
    let status: String = row.get(7).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(8).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(9).map_err(MetaError::Sqlite)?;
    Ok(AgentRun {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "agent run")?,
        project_id: OpaqueId::new(project_id),
        agent_identity_id: OpaqueId::new(agent_id),
        context_manifest_id: OpaqueId::new(context_id),
        prompt,
        status: parse_agent_run_state(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new `Pending` agent run.
    pub fn insert_agent_run(&self, run: &AgentRun) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO medagent_runs(run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                run.header.id.as_str(),
                run.project_id.as_str(),
                run.header.realm_id.as_opaque().as_str(),
                run.header.authority_scope_id.as_opaque().as_str(),
                run.agent_identity_id.as_str(),
                run.context_manifest_id.as_str(),
                run.prompt,
                agent_run_state_str(run.status),
                revision_to_i64(run.revision),
                run.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate agent run {}",
                run.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one agent run.
    pub fn get_agent_run(&self, id: &OpaqueId) -> Result<AgentRun, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version
             FROM medagent_runs WHERE run_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_agent_run_row(row)
    }

    /// Lists agent runs in one Project, newest-id-last, optionally
    /// filtered by agent identity and/or `AgentRunState` (T077-08:
    /// bounded, filterable run history). The status filter uses
    /// `idx_medagent_runs_project(project_id, status)` directly rather
    /// than filtering post-fetch, so `limit` bounds the actual matching
    /// set, not a pre-filter window that could silently under-return.
    pub fn list_agent_runs(
        &self,
        project_id: &OpaqueId,
        agent_id: Option<&OpaqueId>,
        status: Option<AgentRunState>,
        limit: u32,
    ) -> Result<Vec<AgentRun>, MetaError> {
        let limit = i64::from(limit.clamp(1, 200));
        // AgentRunState::as_str() -> &'static str, so this Option is Copy
        // and may be matched on again below without moving it.
        let status_str: Option<&'static str> = status.map(agent_run_state_str);
        let mut out = Vec::new();
        let mut stmt = match (agent_id, status_str) {
            (Some(_), Some(_)) => self.conn().prepare(
                "SELECT run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version
                 FROM medagent_runs WHERE project_id = ?1 AND agent_id = ?2 AND status = ?3 ORDER BY run_id LIMIT ?4",
            )?,
            (Some(_), None) => self.conn().prepare(
                "SELECT run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version
                 FROM medagent_runs WHERE project_id = ?1 AND agent_id = ?2 ORDER BY run_id LIMIT ?3",
            )?,
            (None, Some(_)) => self.conn().prepare(
                "SELECT run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version
                 FROM medagent_runs WHERE project_id = ?1 AND status = ?2 ORDER BY run_id LIMIT ?3",
            )?,
            (None, None) => self.conn().prepare(
                "SELECT run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version
                 FROM medagent_runs WHERE project_id = ?1 ORDER BY run_id LIMIT ?2",
            )?,
        };
        let mut rows = match (agent_id, status_str) {
            (Some(agent_id), Some(status_str)) => stmt.query(params![
                project_id.as_str(),
                agent_id.as_str(),
                status_str,
                limit
            ])?,
            (Some(agent_id), None) => {
                stmt.query(params![project_id.as_str(), agent_id.as_str(), limit])?
            }
            (None, Some(status_str)) => {
                stmt.query(params![project_id.as_str(), status_str, limit])?
            }
            (None, None) => stmt.query(params![project_id.as_str(), limit])?,
        };
        while let Some(row) = rows.next()? {
            out.push(map_agent_run_row(row)?);
        }
        Ok(out)
    }

    /// Transitions a run to a non-terminal state (`Pending -> Running`
    /// only; terminal transitions go through
    /// `commit_terminal_transition_with_receipt` so the `RunReceipt`
    /// always commits atomically). CAS on `revision`.
    pub fn set_agent_run_running(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<AgentRun, MetaError> {
        let current = self.get_agent_run(id)?;
        if current.revision != expected_revision {
            return Err(MetaError::Conflict(format!(
                "stale revision: expected {expected_revision}, current is {}",
                current.revision
            )));
        }
        let next_revision = current.revision + 1;
        let updated = self.conn().execute(
            "UPDATE medagent_runs SET status = ?1, revision = ?2 WHERE run_id = ?3 AND revision = ?4",
            params![
                agent_run_state_str(AgentRunState::Running),
                revision_to_i64(next_revision),
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent run transition".to_owned()));
        }
        self.get_agent_run(id)
    }

    /// Commits a run's terminal transition (`Cancelled`/`Completed`/
    /// `Failed`) and its `RunReceipt` in one transaction: the run is never
    /// visible in a terminal state without its receipt, and the receipt
    /// never exists without the run actually having reached that state
    /// (`migration.md` section 5, `security.md` T11).
    pub fn commit_terminal_transition_with_receipt(
        &self,
        run_id: &OpaqueId,
        expected_revision: u64,
        final_state: AgentRunState,
        receipt: &RunReceipt,
    ) -> Result<(AgentRun, RunReceipt), MetaError> {
        let current = self.get_agent_run(run_id)?;
        if current.revision != expected_revision {
            return Err(MetaError::Conflict(format!(
                "stale revision: expected {expected_revision}, current is {}",
                current.revision
            )));
        }
        let next_revision = current.revision + 1;
        let tool_ids_json = serde_json::to_string(
            &receipt
                .tool_invocation_ids
                .iter()
                .map(OpaqueId::as_str)
                .collect::<Vec<_>>(),
        )
        .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let tx = self.conn().unchecked_transaction()?;
        let updated = tx.execute(
            "UPDATE medagent_runs SET status = ?1, revision = ?2 WHERE run_id = ?3 AND revision = ?4",
            params![
                agent_run_state_str(final_state),
                revision_to_i64(next_revision),
                run_id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent run transition".to_owned()));
        }
        tx.execute(
            "INSERT INTO medagent_run_receipts(run_id, realm_id, authority_scope_id, pack_id, pack_version, context_id, context_revision, tool_invocation_ids_json, final_state, failure_reason, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                run_id.as_str(),
                receipt.header.realm_id.as_opaque().as_str(),
                receipt.header.authority_scope_id.as_opaque().as_str(),
                receipt.pack_id.as_str(),
                receipt.pack_version,
                receipt.context_manifest_id.as_str(),
                revision_to_i64(receipt.context_manifest_revision),
                tool_ids_json,
                agent_run_state_str(receipt.final_state),
                receipt.failure_reason,
                receipt.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!("duplicate run receipt for {}", run_id.as_str()))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        tx.commit()?;
        Ok((self.get_agent_run(run_id)?, self.get_run_receipt(run_id)?))
    }

    /// Appends one `AgentTurn` (storage-assigned `seq`).
    pub fn insert_agent_turn(
        &self,
        header: ObjectHeader,
        run_id: &OpaqueId,
        kind: AgentTurnKind,
        payload: &serde_json::Value,
    ) -> Result<AgentTurn, MetaError> {
        let payload_json = serde_json::to_string(payload)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let seq = self.next_medagent_seq(&format!("medagent-turn-seq-{}", run_id.as_str()))?;
        self.conn().execute(
            "INSERT INTO medagent_turns(turn_id, run_id, realm_id, authority_scope_id, kind, payload_json, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                header.id.as_str(),
                run_id.as_str(),
                header.realm_id.as_opaque().as_str(),
                header.authority_scope_id.as_opaque().as_str(),
                agent_turn_kind_str(kind),
                payload_json,
                seq as i64,
                header.schema_version as i64,
            ],
        )?;
        Ok(AgentTurn {
            header,
            run_id: run_id.clone(),
            seq,
            kind,
            payload: payload.clone(),
        })
    }

    /// Lists turns for one run, in `seq` order.
    pub fn list_agent_turns(&self, run_id: &OpaqueId) -> Result<Vec<AgentTurn>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT turn_id, run_id, realm_id, authority_scope_id, kind, payload_json, seq, schema_version
             FROM medagent_turns WHERE run_id = ?1 ORDER BY seq",
        )?;
        let mut rows = stmt.query(params![run_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let rid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let kind: String = row.get(4).map_err(MetaError::Sqlite)?;
            let payload_json: String = row.get(5).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(6).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
            let payload: serde_json::Value = serde_json::from_str(&payload_json)
                .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
            out.push(AgentTurn {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                run_id: OpaqueId::new(rid),
                seq: u64::try_from(seq).unwrap_or(0),
                kind: parse_agent_turn_kind(&kind)?,
                payload,
            });
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// ToolInvocation + ToolReceipt
// ---------------------------------------------------------------------------

fn map_tool_invocation_row(row: &rusqlite::Row<'_>) -> Result<ToolInvocation, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let run_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let turn_seq: i64 = row.get(4).map_err(MetaError::Sqlite)?;
    let kind: String = row.get(5).map_err(MetaError::Sqlite)?;
    let arguments_json: String = row.get(6).map_err(MetaError::Sqlite)?;
    let status: String = row.get(7).map_err(MetaError::Sqlite)?;
    let refusal_reason: Option<String> = row.get(8).map_err(MetaError::Sqlite)?;
    let seq: i64 = row.get(9).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(10).map_err(MetaError::Sqlite)?;
    let arguments: serde_json::Value = serde_json::from_str(&arguments_json)
        .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
    Ok(ToolInvocation {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        run_id: OpaqueId::new(run_id),
        turn_seq: u64::try_from(turn_seq).unwrap_or(0),
        seq: u64::try_from(seq).unwrap_or(0),
        kind: parse_tool_kind(&kind)?,
        arguments,
        status: parse_tool_invocation_status(&status)?,
        refusal_reason,
    })
}

impl SqliteMetaStore {
    /// Inserts a `Refused` tool invocation (no receipt, ever).
    pub fn insert_refused_tool_invocation(
        &self,
        header: ObjectHeader,
        run_id: &OpaqueId,
        turn_seq: u64,
        kind: ToolKind,
        arguments: &serde_json::Value,
        refusal_reason: &str,
    ) -> Result<ToolInvocation, MetaError> {
        let arguments_json = serde_json::to_string(arguments)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let seq =
            self.next_medagent_seq(&format!("medagent-invocation-seq-{}", run_id.as_str()))?;
        self.conn().execute(
            "INSERT INTO medagent_tool_invocations(invocation_id, run_id, realm_id, authority_scope_id, turn_seq, kind, arguments_json, status, refusal_reason, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                header.id.as_str(),
                run_id.as_str(),
                header.realm_id.as_opaque().as_str(),
                header.authority_scope_id.as_opaque().as_str(),
                turn_seq as i64,
                tool_kind_str(kind),
                arguments_json,
                tool_invocation_status_str(ToolInvocationStatus::Refused),
                refusal_reason,
                seq as i64,
                header.schema_version as i64,
            ],
        )?;
        Ok(ToolInvocation {
            header,
            run_id: run_id.clone(),
            turn_seq,
            seq,
            kind,
            arguments: arguments.clone(),
            status: ToolInvocationStatus::Refused,
            refusal_reason: Some(refusal_reason.to_owned()),
        })
    }

    /// Inserts an `Executed` tool invocation and its `ToolReceipt` in one
    /// transaction: an executed invocation never exists without its
    /// receipt.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_executed_tool_invocation_with_receipt(
        &self,
        invocation_header: ObjectHeader,
        receipt_header: ObjectHeader,
        run_id: &OpaqueId,
        turn_seq: u64,
        kind: ToolKind,
        arguments: &serde_json::Value,
        result: &serde_json::Value,
    ) -> Result<(ToolInvocation, ToolReceipt), MetaError> {
        let arguments_json = serde_json::to_string(arguments)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let result_json = serde_json::to_string(result)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let seq =
            self.next_medagent_seq(&format!("medagent-invocation-seq-{}", run_id.as_str()))?;
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO medagent_tool_invocations(invocation_id, run_id, realm_id, authority_scope_id, turn_seq, kind, arguments_json, status, refusal_reason, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)",
            params![
                invocation_header.id.as_str(),
                run_id.as_str(),
                invocation_header.realm_id.as_opaque().as_str(),
                invocation_header.authority_scope_id.as_opaque().as_str(),
                turn_seq as i64,
                tool_kind_str(kind),
                arguments_json,
                tool_invocation_status_str(ToolInvocationStatus::Executed),
                seq as i64,
                invocation_header.schema_version as i64,
            ],
        )?;
        tx.execute(
            "INSERT INTO medagent_tool_receipts(receipt_id, invocation_id, realm_id, authority_scope_id, result_json, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                receipt_header.id.as_str(),
                invocation_header.id.as_str(),
                receipt_header.realm_id.as_opaque().as_str(),
                receipt_header.authority_scope_id.as_opaque().as_str(),
                result_json,
                receipt_header.schema_version as i64,
            ],
        )?;
        tx.commit()?;
        let invocation_id = invocation_header.id.clone();
        Ok((
            ToolInvocation {
                header: invocation_header,
                run_id: run_id.clone(),
                turn_seq,
                seq,
                kind,
                arguments: arguments.clone(),
                status: ToolInvocationStatus::Executed,
                refusal_reason: None,
            },
            ToolReceipt {
                header: receipt_header,
                invocation_id,
                result: result.clone(),
            },
        ))
    }

    /// Lists tool invocations for one run, in `seq` order.
    pub fn list_tool_invocations(
        &self,
        run_id: &OpaqueId,
    ) -> Result<Vec<ToolInvocation>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT invocation_id, run_id, realm_id, authority_scope_id, turn_seq, kind, arguments_json, status, refusal_reason, seq, schema_version
             FROM medagent_tool_invocations WHERE run_id = ?1 ORDER BY seq",
        )?;
        let mut rows = stmt.query(params![run_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_tool_invocation_row(row)?);
        }
        Ok(out)
    }

    /// Reads the run receipt for one terminated run.
    pub fn get_run_receipt(&self, run_id: &OpaqueId) -> Result<RunReceipt, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT run_id, realm_id, authority_scope_id, pack_id, pack_version, context_id, context_revision, tool_invocation_ids_json, final_state, failure_reason, schema_version
             FROM medagent_run_receipts WHERE run_id = ?1",
        )?;
        let mut rows = stmt.query(params![run_id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        let id: String = row.get(0).map_err(MetaError::Sqlite)?;
        let realm: String = row.get(1).map_err(MetaError::Sqlite)?;
        let scope: String = row.get(2).map_err(MetaError::Sqlite)?;
        let pack_id: String = row.get(3).map_err(MetaError::Sqlite)?;
        let pack_version: String = row.get(4).map_err(MetaError::Sqlite)?;
        let context_id: String = row.get(5).map_err(MetaError::Sqlite)?;
        let context_revision: i64 = row.get(6).map_err(MetaError::Sqlite)?;
        let tool_ids_json: String = row.get(7).map_err(MetaError::Sqlite)?;
        let final_state: String = row.get(8).map_err(MetaError::Sqlite)?;
        let failure_reason: Option<String> = row.get(9).map_err(MetaError::Sqlite)?;
        let schema_version: i64 = row.get(10).map_err(MetaError::Sqlite)?;
        let tool_ids: Vec<String> = serde_json::from_str(&tool_ids_json)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        Ok(RunReceipt {
            header: header_of(&id, &realm, &scope, schema_version, 1),
            run_id: OpaqueId::new(id),
            pack_id: OpaqueId::new(pack_id),
            pack_version,
            context_manifest_id: OpaqueId::new(context_id),
            context_manifest_revision: revision_from_i64(context_revision, "run receipt context")?,
            tool_invocation_ids: tool_ids.into_iter().map(OpaqueId::new).collect(),
            final_state: parse_agent_run_state(&final_state)?,
            failure_reason,
        })
    }
}

// ---------------------------------------------------------------------------
// AgentProposal
// ---------------------------------------------------------------------------

impl SqliteMetaStore {
    /// Inserts a new agent-proposal linking record.
    pub fn insert_agent_proposal(&self, proposal: &AgentProposal) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO medagent_proposals(agent_proposal_id, run_id, realm_id, authority_scope_id, proposal_id, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                proposal.header.id.as_str(),
                proposal.run_id.as_str(),
                proposal.header.realm_id.as_opaque().as_str(),
                proposal.header.authority_scope_id.as_opaque().as_str(),
                proposal.proposal_id.as_str(),
                proposal.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate agent proposal {}",
                proposal.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads the agent-proposal linking record for one run, if any.
    pub fn get_agent_proposal_for_run(
        &self,
        run_id: &OpaqueId,
    ) -> Result<Option<AgentProposal>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT agent_proposal_id, run_id, realm_id, authority_scope_id, proposal_id, schema_version
             FROM medagent_proposals WHERE run_id = ?1",
        )?;
        let mut rows = stmt.query(params![run_id.as_str()])?;
        match rows.next()? {
            Some(row) => {
                let id: String = row.get(0).map_err(MetaError::Sqlite)?;
                let rid: String = row.get(1).map_err(MetaError::Sqlite)?;
                let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
                let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
                let proposal_id: String = row.get(4).map_err(MetaError::Sqlite)?;
                let schema_version: i64 = row.get(5).map_err(MetaError::Sqlite)?;
                Ok(Some(AgentProposal {
                    header: header_of(&id, &realm, &scope, schema_version, 1),
                    run_id: OpaqueId::new(rid),
                    proposal_id: OpaqueId::new(proposal_id),
                }))
            }
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Backup/restore: full-table scans + exact-row restore for every family
// (mirrors the exact `list_all_*`/`restore_*_row` pattern
// `collaboration.rs` established for Spec 076, including the lesson its own
// exact-range review taught: `backup.rs`'s restore path must re-verify
// whatever cross-row invariant this spec defines, not just replay rows --
// see `restore_v6`'s run/receipt consistency check).
// ---------------------------------------------------------------------------

impl SqliteMetaStore {
    pub fn list_all_agent_identities(&self) -> Result<Vec<AgentIdentity>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT agent_id, project_id, realm_id, authority_scope_id, pack_id, pack_version, display_name, status, revision, schema_version
             FROM medagent_identities ORDER BY agent_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_agent_identity_row(row)?);
        }
        Ok(out)
    }

    pub fn restore_agent_identity_row(&self, identity: &AgentIdentity) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_identities(agent_id, project_id, realm_id, authority_scope_id, pack_id, pack_version, display_name, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                identity.header.id.as_str(),
                identity.project_id.as_str(),
                identity.header.realm_id.as_opaque().as_str(),
                identity.header.authority_scope_id.as_opaque().as_str(),
                identity.pack_id.as_str(),
                identity.pack_version,
                identity.display_name,
                agent_identity_status_str(identity.status),
                revision_to_i64(identity.revision),
                identity.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_all_capability_manifests(&self) -> Result<Vec<(String, String)>, MetaError> {
        let mut stmt = self
            .conn()
            .prepare("SELECT agent_id, granted_tool_kinds_json FROM medagent_capability_manifests ORDER BY agent_id")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let agent_id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let json: String = row.get(1).map_err(MetaError::Sqlite)?;
            out.push((agent_id, json));
        }
        Ok(out)
    }

    pub fn restore_capability_manifest_row(
        &self,
        manifest: &AgentCapabilityManifest,
    ) -> Result<(), MetaError> {
        let granted_json = serde_json::to_string(&manifest.granted_tool_kinds)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_capability_manifests(agent_id, granted_tool_kinds_json)
             VALUES (?1, ?2)",
            params![manifest.agent_identity_id.as_str(), granted_json],
        )?;
        Ok(())
    }

    pub fn list_all_context_manifests(&self) -> Result<Vec<ContextManifest>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT context_id, project_id, realm_id, authority_scope_id, selected_artifacts_json, revision, schema_version
             FROM medagent_context_manifests ORDER BY context_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_context_manifest_row(row)?);
        }
        Ok(out)
    }

    pub fn restore_context_manifest_row(
        &self,
        manifest: &ContextManifest,
    ) -> Result<(), MetaError> {
        let artifacts_json = serde_json::to_string(&manifest.selected_artifacts)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_context_manifests(context_id, project_id, realm_id, authority_scope_id, selected_artifacts_json, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                manifest.header.id.as_str(),
                manifest.project_id.as_str(),
                manifest.header.realm_id.as_opaque().as_str(),
                manifest.header.authority_scope_id.as_opaque().as_str(),
                artifacts_json,
                revision_to_i64(manifest.revision),
                manifest.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_all_agent_runs(&self) -> Result<Vec<AgentRun>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version
             FROM medagent_runs ORDER BY run_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_agent_run_row(row)?);
        }
        Ok(out)
    }

    pub fn restore_agent_run_row(&self, run: &AgentRun) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_runs(run_id, project_id, realm_id, authority_scope_id, agent_id, context_id, prompt, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                run.header.id.as_str(),
                run.project_id.as_str(),
                run.header.realm_id.as_opaque().as_str(),
                run.header.authority_scope_id.as_opaque().as_str(),
                run.agent_identity_id.as_str(),
                run.context_manifest_id.as_str(),
                run.prompt,
                agent_run_state_str(run.status),
                revision_to_i64(run.revision),
                run.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_all_agent_turns(&self) -> Result<Vec<AgentTurn>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT turn_id, run_id, realm_id, authority_scope_id, kind, payload_json, seq, schema_version
             FROM medagent_turns ORDER BY run_id, seq",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let rid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let kind: String = row.get(4).map_err(MetaError::Sqlite)?;
            let payload_json: String = row.get(5).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(6).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
            let payload: serde_json::Value = serde_json::from_str(&payload_json)
                .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
            out.push(AgentTurn {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                run_id: OpaqueId::new(rid),
                seq: u64::try_from(seq).unwrap_or(0),
                kind: parse_agent_turn_kind(&kind)?,
                payload,
            });
        }
        Ok(out)
    }

    pub fn restore_agent_turn_row(&self, turn: &AgentTurn) -> Result<(), MetaError> {
        let payload_json = serde_json::to_string(&turn.payload)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_turns(turn_id, run_id, realm_id, authority_scope_id, kind, payload_json, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                turn.header.id.as_str(),
                turn.run_id.as_str(),
                turn.header.realm_id.as_opaque().as_str(),
                turn.header.authority_scope_id.as_opaque().as_str(),
                agent_turn_kind_str(turn.kind),
                payload_json,
                turn.seq as i64,
                turn.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_all_tool_invocations(&self) -> Result<Vec<ToolInvocation>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT invocation_id, run_id, realm_id, authority_scope_id, turn_seq, kind, arguments_json, status, refusal_reason, seq, schema_version
             FROM medagent_tool_invocations ORDER BY run_id, seq",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_tool_invocation_row(row)?);
        }
        Ok(out)
    }

    pub fn restore_tool_invocation_row(
        &self,
        invocation: &ToolInvocation,
    ) -> Result<(), MetaError> {
        let arguments_json = serde_json::to_string(&invocation.arguments)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_tool_invocations(invocation_id, run_id, realm_id, authority_scope_id, turn_seq, kind, arguments_json, status, refusal_reason, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                invocation.header.id.as_str(),
                invocation.run_id.as_str(),
                invocation.header.realm_id.as_opaque().as_str(),
                invocation.header.authority_scope_id.as_opaque().as_str(),
                invocation.turn_seq as i64,
                tool_kind_str(invocation.kind),
                arguments_json,
                tool_invocation_status_str(invocation.status),
                invocation.refusal_reason,
                invocation.seq as i64,
                invocation.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_all_tool_receipts(&self) -> Result<Vec<ToolReceipt>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT receipt_id, invocation_id, realm_id, authority_scope_id, result_json, schema_version
             FROM medagent_tool_receipts ORDER BY receipt_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let invocation_id: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let result_json: String = row.get(4).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(5).map_err(MetaError::Sqlite)?;
            let result: serde_json::Value = serde_json::from_str(&result_json)
                .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
            out.push(ToolReceipt {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                invocation_id: OpaqueId::new(invocation_id),
                result,
            });
        }
        Ok(out)
    }

    pub fn restore_tool_receipt_row(&self, receipt: &ToolReceipt) -> Result<(), MetaError> {
        let result_json = serde_json::to_string(&receipt.result)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_tool_receipts(receipt_id, invocation_id, realm_id, authority_scope_id, result_json, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                receipt.header.id.as_str(),
                receipt.invocation_id.as_str(),
                receipt.header.realm_id.as_opaque().as_str(),
                receipt.header.authority_scope_id.as_opaque().as_str(),
                result_json,
                receipt.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_all_run_receipts(&self) -> Result<Vec<RunReceipt>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT run_id, realm_id, authority_scope_id, pack_id, pack_version, context_id, context_revision, tool_invocation_ids_json, final_state, failure_reason, schema_version
             FROM medagent_run_receipts ORDER BY run_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(1).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(2).map_err(MetaError::Sqlite)?;
            let pack_id: String = row.get(3).map_err(MetaError::Sqlite)?;
            let pack_version: String = row.get(4).map_err(MetaError::Sqlite)?;
            let context_id: String = row.get(5).map_err(MetaError::Sqlite)?;
            let context_revision: i64 = row.get(6).map_err(MetaError::Sqlite)?;
            let tool_ids_json: String = row.get(7).map_err(MetaError::Sqlite)?;
            let final_state: String = row.get(8).map_err(MetaError::Sqlite)?;
            let failure_reason: Option<String> = row.get(9).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(10).map_err(MetaError::Sqlite)?;
            let tool_ids: Vec<String> = serde_json::from_str(&tool_ids_json)
                .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
            out.push(RunReceipt {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                run_id: OpaqueId::new(id),
                pack_id: OpaqueId::new(pack_id),
                pack_version,
                context_manifest_id: OpaqueId::new(context_id),
                context_manifest_revision: revision_from_i64(
                    context_revision,
                    "run receipt context",
                )?,
                tool_invocation_ids: tool_ids.into_iter().map(OpaqueId::new).collect(),
                final_state: parse_agent_run_state(&final_state)?,
                failure_reason,
            });
        }
        Ok(out)
    }

    pub fn restore_run_receipt_row(&self, receipt: &RunReceipt) -> Result<(), MetaError> {
        let tool_ids_json = serde_json::to_string(
            &receipt
                .tool_invocation_ids
                .iter()
                .map(OpaqueId::as_str)
                .collect::<Vec<_>>(),
        )
        .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_run_receipts(run_id, realm_id, authority_scope_id, pack_id, pack_version, context_id, context_revision, tool_invocation_ids_json, final_state, failure_reason, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                receipt.run_id.as_str(),
                receipt.header.realm_id.as_opaque().as_str(),
                receipt.header.authority_scope_id.as_opaque().as_str(),
                receipt.pack_id.as_str(),
                receipt.pack_version,
                receipt.context_manifest_id.as_str(),
                revision_to_i64(receipt.context_manifest_revision),
                tool_ids_json,
                agent_run_state_str(receipt.final_state),
                receipt.failure_reason,
                receipt.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_all_agent_proposals(&self) -> Result<Vec<AgentProposal>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT agent_proposal_id, run_id, realm_id, authority_scope_id, proposal_id, schema_version
             FROM medagent_proposals ORDER BY agent_proposal_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let run_id: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let proposal_id: String = row.get(4).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(5).map_err(MetaError::Sqlite)?;
            out.push(AgentProposal {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                run_id: OpaqueId::new(run_id),
                proposal_id: OpaqueId::new(proposal_id),
            });
        }
        Ok(out)
    }

    pub fn restore_agent_proposal_row(&self, proposal: &AgentProposal) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO medagent_proposals(agent_proposal_id, run_id, realm_id, authority_scope_id, proposal_id, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                proposal.header.id.as_str(),
                proposal.run_id.as_str(),
                proposal.header.realm_id.as_opaque().as_str(),
                proposal.header.authority_scope_id.as_opaque().as_str(),
                proposal.proposal_id.as_str(),
                proposal.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Re-verifies every terminal run in this vault has exactly one
    /// `RunReceipt` and vice versa -- the invariant `migration.md` section
    /// 5 requires, checked explicitly at restore time rather than merely
    /// assumed from a successful row replay (the exact class of gap Spec
    /// 076's exact-range review found in its own restore path).
    pub fn verify_run_receipt_consistency(&self) -> Result<(), MetaError> {
        let runs = self.list_all_agent_runs()?;
        let receipts = self.list_all_run_receipts()?;
        let receipt_run_ids: std::collections::HashSet<&str> =
            receipts.iter().map(|r| r.run_id.as_str()).collect();
        for run in &runs {
            let has_receipt = receipt_run_ids.contains(run.header.id.as_str());
            if run.status.is_terminal() && !has_receipt {
                return Err(MetaError::CorruptObjectBody(format!(
                    "terminal run {} has no RunReceipt",
                    run.header.id.as_str()
                )));
            }
            if !run.status.is_terminal() && has_receipt {
                return Err(MetaError::CorruptObjectBody(format!(
                    "non-terminal run {} has a RunReceipt",
                    run.header.id.as_str()
                )));
            }
        }
        let run_ids: std::collections::HashSet<&str> =
            runs.iter().map(|r| r.header.id.as_str()).collect();
        for receipt in &receipts {
            if !run_ids.contains(receipt.run_id.as_str()) {
                return Err(MetaError::CorruptObjectBody(format!(
                    "RunReceipt {} has no matching run",
                    receipt.run_id.as_str()
                )));
            }
        }
        Ok(())
    }
}

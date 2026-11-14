//! Model Fleet + Compare durable rows (Spec 078, storage schema v7).
//!
//! Dedicated tables inside the existing encrypted metadata DB: no second
//! database. `LanePolicy` travels as validated JSON on the owning
//! `model_fleet_lanes` row (mirroring Spec 077's
//! `medagent_context_manifests.selected_artifacts_json` precedent), not a
//! separate child table -- there is exactly one policy per lane, never a
//! partially-visible list. `ComparisonReport.observations` travels the same
//! way on `model_fleet_comparison_reports`.
//!
//! Every reference into a Spec 077 row (`agent_identity_id`,
//! `context_manifest_id`, `agent_run_id`) is stored as a plain `OpaqueId`
//! string column, never a SQL foreign key into a `medagent_*` table
//! (`migration.md` section 4) -- no 078 table modifies or cascades into any
//! Spec 077 table.
//!
//! `model_fleet_lane_run_refs.agent_run_id` carries a UNIQUE index
//! (`security.md` T4: no `AgentRun` id may be bound to more than one lane).
//! Restore of this table therefore uses a plain `INSERT`, never
//! `INSERT OR REPLACE`: replacing on conflict would silently squash a
//! tampered duplicate-binding snapshot instead of failing closed, which is
//! exactly the class of restore-time gap Spec 076's and 077's own
//! exact-range reviews found and fixed in their own restore paths.

use std::collections::{HashMap, HashSet};

use medscale_contracts::medagent::{AgentRun, AgentRunState};
use medscale_contracts::model_fleet::{
    AgentLane, AgentLaneStatus, ComparisonObservation, ComparisonReport, FLEET_RUN_MAX_LANES,
    FleetRun, FleetRunState, LanePolicy, LaneRunRef,
};
use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId};
use rusqlite::{OptionalExtension, params};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v7 DDL, executed inside `begin/finish_migration(7)`.
pub(crate) const V7_DDL: &str = r"
CREATE TABLE IF NOT EXISTS model_fleet_lanes (
  lane_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  agent_identity_id TEXT NOT NULL,
  context_manifest_id TEXT NOT NULL,
  role_label TEXT NOT NULL,
  policy_json TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_model_fleet_lanes_project
  ON model_fleet_lanes(project_id, status);
CREATE INDEX IF NOT EXISTS idx_model_fleet_lanes_identity
  ON model_fleet_lanes(agent_identity_id);
CREATE TABLE IF NOT EXISTS model_fleet_runs (
  fleet_run_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  task_prompt TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_model_fleet_runs_project
  ON model_fleet_runs(project_id, status);
CREATE TABLE IF NOT EXISTS model_fleet_lane_run_refs (
  lane_run_ref_id TEXT PRIMARY KEY,
  fleet_run_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  agent_lane_id TEXT NOT NULL,
  agent_run_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_model_fleet_lane_run_refs_fleet
  ON model_fleet_lane_run_refs(fleet_run_id);
CREATE INDEX IF NOT EXISTS idx_model_fleet_lane_run_refs_lane
  ON model_fleet_lane_run_refs(agent_lane_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_model_fleet_lane_run_refs_agent_run
  ON model_fleet_lane_run_refs(agent_run_id);
CREATE TABLE IF NOT EXISTS model_fleet_comparison_reports (
  report_id TEXT PRIMARY KEY,
  fleet_run_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  observations_json TEXT NOT NULL,
  participating_lane_ids_json TEXT NOT NULL,
  excluded_lane_ids_json TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_model_fleet_comparison_reports_fleet
  ON model_fleet_comparison_reports(fleet_run_id);
";

// ---------------------------------------------------------------------------
// small helpers (deliberately duplicated from medagent.rs rather than
// exported from it, so this module never depends on Spec 077's closed file)
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

fn agent_lane_status_str(status: AgentLaneStatus) -> &'static str {
    status.as_str()
}

fn parse_agent_lane_status(value: &str) -> Result<AgentLaneStatus, MetaError> {
    AgentLaneStatus::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn fleet_run_state_str(state: FleetRunState) -> &'static str {
    state.as_str()
}

fn parse_fleet_run_state(value: &str) -> Result<FleetRunState, MetaError> {
    FleetRunState::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn ids_to_json(ids: &[OpaqueId]) -> Result<String, MetaError> {
    serde_json::to_string(&ids.iter().map(OpaqueId::as_str).collect::<Vec<_>>())
        .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))
}

fn ids_from_json(json: &str) -> Result<Vec<OpaqueId>, MetaError> {
    let raw: Vec<String> =
        serde_json::from_str(json).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
    Ok(raw.into_iter().map(OpaqueId::new).collect())
}

// ---------------------------------------------------------------------------
// id allocation (reuses the durable sqlite-backed sequence pattern)
// ---------------------------------------------------------------------------

impl SqliteMetaStore {
    /// Allocates one `prefix-N` Model Fleet id from a durable sqlite-backed
    /// sequence, independent of every other spec's id sequence space.
    pub fn alloc_model_fleet_id(&self, seq_key: &str, prefix: &str) -> Result<OpaqueId, MetaError> {
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
}

// ---------------------------------------------------------------------------
// AgentLane
// ---------------------------------------------------------------------------

fn lane_policy_to_json(policy: &LanePolicy) -> Result<String, MetaError> {
    serde_json::to_string(policy).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))
}

fn lane_policy_from_json(json: &str) -> Result<LanePolicy, MetaError> {
    serde_json::from_str(json).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))
}

fn map_agent_lane_row(row: &rusqlite::Row<'_>) -> Result<AgentLane, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let project_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let agent_identity_id: String = row.get(4).map_err(MetaError::Sqlite)?;
    let context_manifest_id: String = row.get(5).map_err(MetaError::Sqlite)?;
    let role_label: String = row.get(6).map_err(MetaError::Sqlite)?;
    let policy_json: String = row.get(7).map_err(MetaError::Sqlite)?;
    let status: String = row.get(8).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(9).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(10).map_err(MetaError::Sqlite)?;
    Ok(AgentLane {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "agent lane")?,
        project_id: OpaqueId::new(project_id),
        agent_identity_id: OpaqueId::new(agent_identity_id),
        context_manifest_id: OpaqueId::new(context_manifest_id),
        role_label,
        policy: lane_policy_from_json(&policy_json)?,
        status: parse_agent_lane_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new agent lane.
    pub fn insert_agent_lane(&self, lane: &AgentLane) -> Result<(), MetaError> {
        let policy_json = lane_policy_to_json(&lane.policy)?;
        let result = self.conn().execute(
            "INSERT INTO model_fleet_lanes(lane_id, project_id, realm_id, authority_scope_id, agent_identity_id, context_manifest_id, role_label, policy_json, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                lane.header.id.as_str(),
                lane.project_id.as_str(),
                lane.header.realm_id.as_opaque().as_str(),
                lane.header.authority_scope_id.as_opaque().as_str(),
                lane.agent_identity_id.as_str(),
                lane.context_manifest_id.as_str(),
                lane.role_label,
                policy_json,
                agent_lane_status_str(lane.status),
                revision_to_i64(lane.revision),
                lane.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate agent lane {}",
                lane.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one agent lane.
    pub fn get_agent_lane(&self, id: &OpaqueId) -> Result<AgentLane, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT lane_id, project_id, realm_id, authority_scope_id, agent_identity_id, context_manifest_id, role_label, policy_json, status, revision, schema_version
             FROM model_fleet_lanes WHERE lane_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_agent_lane_row(row)
    }

    /// Lists agent lanes in one Project, newest-id-last, optionally
    /// filtered by `AgentLaneStatus`.
    pub fn list_agent_lanes(
        &self,
        project_id: &OpaqueId,
        status: Option<AgentLaneStatus>,
        limit: u32,
    ) -> Result<Vec<AgentLane>, MetaError> {
        let limit = i64::from(limit.clamp(1, 200));
        let status_str: Option<&'static str> = status.map(agent_lane_status_str);
        let mut out = Vec::new();
        let mut stmt = match status_str {
            Some(_) => self.conn().prepare(
                "SELECT lane_id, project_id, realm_id, authority_scope_id, agent_identity_id, context_manifest_id, role_label, policy_json, status, revision, schema_version
                 FROM model_fleet_lanes WHERE project_id = ?1 AND status = ?2 ORDER BY lane_id LIMIT ?3",
            )?,
            None => self.conn().prepare(
                "SELECT lane_id, project_id, realm_id, authority_scope_id, agent_identity_id, context_manifest_id, role_label, policy_json, status, revision, schema_version
                 FROM model_fleet_lanes WHERE project_id = ?1 ORDER BY lane_id LIMIT ?2",
            )?,
        };
        let mut rows = match status_str {
            Some(status_str) => stmt.query(params![project_id.as_str(), status_str, limit])?,
            None => stmt.query(params![project_id.as_str(), limit])?,
        };
        while let Some(row) = rows.next()? {
            out.push(map_agent_lane_row(row)?);
        }
        Ok(out)
    }

    /// Retires an agent lane (status only; CAS on `revision`).
    pub fn retire_agent_lane(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<AgentLane, MetaError> {
        let current = self.get_agent_lane(id)?;
        if current.revision != expected_revision {
            return Err(MetaError::Conflict(format!(
                "stale revision: expected {expected_revision}, current is {}",
                current.revision
            )));
        }
        let next_revision = current.revision + 1;
        let updated = self.conn().execute(
            "UPDATE model_fleet_lanes SET status = ?1, revision = ?2 WHERE lane_id = ?3 AND revision = ?4",
            params![
                agent_lane_status_str(AgentLaneStatus::Retired),
                revision_to_i64(next_revision),
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent lane retire".to_owned()));
        }
        self.get_agent_lane(id)
    }
}

// ---------------------------------------------------------------------------
// FleetRun
// ---------------------------------------------------------------------------

fn map_fleet_run_row(row: &rusqlite::Row<'_>) -> Result<FleetRun, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let project_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let task_prompt: String = row.get(4).map_err(MetaError::Sqlite)?;
    let status: String = row.get(5).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(6).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
    Ok(FleetRun {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "fleet run")?,
        project_id: OpaqueId::new(project_id),
        task_prompt,
        status: parse_fleet_run_state(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new `Pending` fleet run.
    pub fn insert_fleet_run(&self, run: &FleetRun) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO model_fleet_runs(fleet_run_id, project_id, realm_id, authority_scope_id, task_prompt, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                run.header.id.as_str(),
                run.project_id.as_str(),
                run.header.realm_id.as_opaque().as_str(),
                run.header.authority_scope_id.as_opaque().as_str(),
                run.task_prompt,
                fleet_run_state_str(run.status),
                revision_to_i64(run.revision),
                run.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate fleet run {}",
                run.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one fleet run.
    pub fn get_fleet_run(&self, id: &OpaqueId) -> Result<FleetRun, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT fleet_run_id, project_id, realm_id, authority_scope_id, task_prompt, status, revision, schema_version
             FROM model_fleet_runs WHERE fleet_run_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_fleet_run_row(row)
    }

    /// Lists fleet runs in one Project, newest-id-last, optionally filtered
    /// by `FleetRunState`.
    pub fn list_fleet_runs(
        &self,
        project_id: &OpaqueId,
        status: Option<FleetRunState>,
        limit: u32,
    ) -> Result<Vec<FleetRun>, MetaError> {
        let limit = i64::from(limit.clamp(1, 200));
        let status_str: Option<&'static str> = status.map(fleet_run_state_str);
        let mut out = Vec::new();
        let mut stmt = match status_str {
            Some(_) => self.conn().prepare(
                "SELECT fleet_run_id, project_id, realm_id, authority_scope_id, task_prompt, status, revision, schema_version
                 FROM model_fleet_runs WHERE project_id = ?1 AND status = ?2 ORDER BY fleet_run_id LIMIT ?3",
            )?,
            None => self.conn().prepare(
                "SELECT fleet_run_id, project_id, realm_id, authority_scope_id, task_prompt, status, revision, schema_version
                 FROM model_fleet_runs WHERE project_id = ?1 ORDER BY fleet_run_id LIMIT ?2",
            )?,
        };
        let mut rows = match status_str {
            Some(status_str) => stmt.query(params![project_id.as_str(), status_str, limit])?,
            None => stmt.query(params![project_id.as_str(), limit])?,
        };
        while let Some(row) = rows.next()? {
            out.push(map_fleet_run_row(row)?);
        }
        Ok(out)
    }

    /// Transitions a fleet run to `next_state` per the frozen table
    /// (`FleetRun::check_transition`, enforced by the Core caller before
    /// this is invoked). CAS on `revision`.
    pub fn transition_fleet_run(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
        next_state: FleetRunState,
    ) -> Result<FleetRun, MetaError> {
        let current = self.get_fleet_run(id)?;
        if current.revision != expected_revision {
            return Err(MetaError::Conflict(format!(
                "stale revision: expected {expected_revision}, current is {}",
                current.revision
            )));
        }
        let next_revision = current.revision + 1;
        let updated = self.conn().execute(
            "UPDATE model_fleet_runs SET status = ?1, revision = ?2 WHERE fleet_run_id = ?3 AND revision = ?4",
            params![
                fleet_run_state_str(next_state),
                revision_to_i64(next_revision),
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict(
                "concurrent fleet run transition".to_owned(),
            ));
        }
        self.get_fleet_run(id)
    }
}

// ---------------------------------------------------------------------------
// LaneRunRef
// ---------------------------------------------------------------------------

fn map_lane_run_ref_row(row: &rusqlite::Row<'_>) -> Result<LaneRunRef, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let fleet_run_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let agent_lane_id: String = row.get(4).map_err(MetaError::Sqlite)?;
    let agent_run_id: String = row.get(5).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(6).map_err(MetaError::Sqlite)?;
    Ok(LaneRunRef {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        fleet_run_id: OpaqueId::new(fleet_run_id),
        agent_lane_id: OpaqueId::new(agent_lane_id),
        agent_run_id: OpaqueId::new(agent_run_id),
    })
}

impl SqliteMetaStore {
    /// Inserts one `LaneRunRef` binding a fleet run's lane to the real,
    /// independent Spec 077 `AgentRun` it dispatched. `agent_run_id` is
    /// UNIQUE across the whole table (`security.md` T4): a duplicate
    /// binding attempt is a `Conflict`, never a silent second reference to
    /// the same run.
    pub fn insert_lane_run_ref(&self, lane_run_ref: &LaneRunRef) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO model_fleet_lane_run_refs(lane_run_ref_id, fleet_run_id, realm_id, authority_scope_id, agent_lane_id, agent_run_id, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                lane_run_ref.header.id.as_str(),
                lane_run_ref.fleet_run_id.as_str(),
                lane_run_ref.header.realm_id.as_opaque().as_str(),
                lane_run_ref.header.authority_scope_id.as_opaque().as_str(),
                lane_run_ref.agent_lane_id.as_str(),
                lane_run_ref.agent_run_id.as_str(),
                lane_run_ref.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "agent run {} is already bound to a lane run ref (or duplicate id {})",
                lane_run_ref.agent_run_id.as_str(),
                lane_run_ref.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// The lane binding of one Spec 077 `AgentRun`, if any (served by the
    /// UNIQUE `agent_run_id` index; at most one row can match).
    pub fn get_lane_run_ref_for_agent_run(
        &self,
        agent_run_id: &OpaqueId,
    ) -> Result<Option<LaneRunRef>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT lane_run_ref_id, fleet_run_id, realm_id, authority_scope_id, agent_lane_id, agent_run_id, schema_version
             FROM model_fleet_lane_run_refs WHERE agent_run_id = ?1",
        )?;
        let mut rows = stmt.query(params![agent_run_id.as_str()])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_lane_run_ref_row(row)?)),
            None => Ok(None),
        }
    }

    /// Lists lane run refs for one fleet run.
    pub fn list_lane_run_refs(
        &self,
        fleet_run_id: &OpaqueId,
    ) -> Result<Vec<LaneRunRef>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT lane_run_ref_id, fleet_run_id, realm_id, authority_scope_id, agent_lane_id, agent_run_id, schema_version
             FROM model_fleet_lane_run_refs WHERE fleet_run_id = ?1 ORDER BY lane_run_ref_id",
        )?;
        let mut rows = stmt.query(params![fleet_run_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_lane_run_ref_row(row)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// ComparisonReport
// ---------------------------------------------------------------------------

fn observations_to_json(observations: &[ComparisonObservation]) -> Result<String, MetaError> {
    serde_json::to_string(observations).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))
}

fn observations_from_json(json: &str) -> Result<Vec<ComparisonObservation>, MetaError> {
    serde_json::from_str(json).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))
}

fn map_comparison_report_row(row: &rusqlite::Row<'_>) -> Result<ComparisonReport, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let fleet_run_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let observations_json: String = row.get(4).map_err(MetaError::Sqlite)?;
    let participating_json: String = row.get(5).map_err(MetaError::Sqlite)?;
    let excluded_json: String = row.get(6).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
    Ok(ComparisonReport {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        fleet_run_id: OpaqueId::new(fleet_run_id),
        observations: observations_from_json(&observations_json)?,
        participating_lane_ids: ids_from_json(&participating_json)?,
        excluded_lane_ids: ids_from_json(&excluded_json)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new, immutable comparison report. Recomputation inserts a
    /// new row with a new id; there is no update path (`contracts.md`
    /// section 4).
    pub fn insert_comparison_report(&self, report: &ComparisonReport) -> Result<(), MetaError> {
        let observations_json = observations_to_json(&report.observations)?;
        let participating_json = ids_to_json(&report.participating_lane_ids)?;
        let excluded_json = ids_to_json(&report.excluded_lane_ids)?;
        let result = self.conn().execute(
            "INSERT INTO model_fleet_comparison_reports(report_id, fleet_run_id, realm_id, authority_scope_id, observations_json, participating_lane_ids_json, excluded_lane_ids_json, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                report.header.id.as_str(),
                report.fleet_run_id.as_str(),
                report.header.realm_id.as_opaque().as_str(),
                report.header.authority_scope_id.as_opaque().as_str(),
                observations_json,
                participating_json,
                excluded_json,
                report.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate comparison report {}",
                report.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Lists comparison reports for one fleet run, oldest-first (a
    /// `FleetRun` may have more than one report over time if recomputed;
    /// the caller decides which is "current", storage exposes the full
    /// history).
    pub fn list_comparison_reports(
        &self,
        fleet_run_id: &OpaqueId,
    ) -> Result<Vec<ComparisonReport>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT report_id, fleet_run_id, realm_id, authority_scope_id, observations_json, participating_lane_ids_json, excluded_lane_ids_json, schema_version
             FROM model_fleet_comparison_reports WHERE fleet_run_id = ?1 ORDER BY report_id",
        )?;
        let mut rows = stmt.query(params![fleet_run_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_comparison_report_row(row)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Cross-family consistency check + backup/restore full-table scans
// ---------------------------------------------------------------------------

fn corrupt(message: String) -> MetaError {
    MetaError::CorruptObjectBody(message)
}

/// Maps a restore-time insert `Conflict` (duplicate primary key, or a second
/// binding of the same `agent_run_id`) to a corruption signal: a restore
/// target is empty, so any conflict means the snapshot itself is tampered.
fn restore_conflict_is_corrupt(result: Result<(), MetaError>) -> Result<(), MetaError> {
    match result {
        Err(MetaError::Conflict(message)) => Err(corrupt(format!("tampered backup: {message}"))),
        other => other,
    }
}

impl SqliteMetaStore {
    /// Cross-family consistency check, run after restore (`backup.rs`,
    /// `migration.md` sections 6 and 11). Re-verified at restore time, not
    /// assumed from insert-time checks a hand-edited backup could bypass:
    ///
    /// - every `LaneRunRef` names an existing `FleetRun`, `AgentLane` and
    ///   Spec 077 `AgentRun`, and no `AgentRun` is bound twice (T4);
    /// - the bound `AgentRun` was dispatched for exactly that lane's
    ///   identity/context and the fleet's Project (T3);
    /// - no fleet run binds more than `FLEET_RUN_MAX_LANES` lanes, or the
    ///   same lane twice;
    /// - a terminal fleet run has at least one bound lane (unless it was
    ///   cancelled before dispatch), every bound run is terminal, a
    ///   `Cancelled` fleet binds only `Cancelled` runs, and any other
    ///   terminal state equals the aggregate of its bound runs' states (no
    ///   fleet visible `Completed` over a failed lane);
    /// - every `ComparisonReport` names an existing `Completed`/
    ///   `PartiallyFailed` fleet run, passes its contract validation, and
    ///   names only lanes actually bound to that fleet run.
    pub fn verify_model_fleet_consistency(&self) -> Result<(), MetaError> {
        let lanes: HashMap<String, AgentLane> = self
            .list_all_agent_lanes()?
            .into_iter()
            .map(|l| (l.header.id.as_str().to_owned(), l))
            .collect();
        let runs: HashMap<String, AgentRun> = self
            .list_all_agent_runs()?
            .into_iter()
            .map(|r| (r.header.id.as_str().to_owned(), r))
            .collect();
        let fleet_runs: HashMap<String, FleetRun> = self
            .list_all_fleet_runs()?
            .into_iter()
            .map(|f| (f.header.id.as_str().to_owned(), f))
            .collect();

        let mut seen_run_ids = HashSet::new();
        let mut bound: HashMap<String, Vec<(String, &AgentRun)>> = HashMap::new();
        for lane_run_ref in self.list_all_lane_run_refs()? {
            let ref_id = lane_run_ref.header.id.as_str();
            let fleet = fleet_runs
                .get(lane_run_ref.fleet_run_id.as_str())
                .ok_or_else(|| {
                    corrupt(format!(
                        "lane run ref {ref_id} names a non-existent fleet run {}",
                        lane_run_ref.fleet_run_id.as_str()
                    ))
                })?;
            let lane = lanes
                .get(lane_run_ref.agent_lane_id.as_str())
                .ok_or_else(|| {
                    corrupt(format!(
                        "lane run ref {ref_id} names a non-existent agent lane {}",
                        lane_run_ref.agent_lane_id.as_str()
                    ))
                })?;
            let run = runs
                .get(lane_run_ref.agent_run_id.as_str())
                .ok_or_else(|| {
                    corrupt(format!(
                        "lane run ref {ref_id} names a non-existent agent run {}",
                        lane_run_ref.agent_run_id.as_str()
                    ))
                })?;
            if !seen_run_ids.insert(lane_run_ref.agent_run_id.as_str().to_owned()) {
                return Err(corrupt(format!(
                    "agent run {} is bound to more than one lane run ref",
                    lane_run_ref.agent_run_id.as_str()
                )));
            }
            if run.agent_identity_id != lane.agent_identity_id
                || run.context_manifest_id != lane.context_manifest_id
            {
                return Err(corrupt(format!(
                    "lane run ref {ref_id} binds an agent run not dispatched for its lane identity/context"
                )));
            }
            if run.project_id != fleet.project_id || lane.project_id != fleet.project_id {
                return Err(corrupt(format!(
                    "lane run ref {ref_id} crosses Project boundaries"
                )));
            }
            bound
                .entry(lane_run_ref.fleet_run_id.as_str().to_owned())
                .or_default()
                .push((lane_run_ref.agent_lane_id.as_str().to_owned(), run));
        }

        for (fleet_id, fleet) in &fleet_runs {
            let entries = bound.get(fleet_id).map(Vec::as_slice).unwrap_or_default();
            if entries.len() > FLEET_RUN_MAX_LANES {
                return Err(corrupt(format!(
                    "fleet run {fleet_id} binds more than {FLEET_RUN_MAX_LANES} lanes"
                )));
            }
            let distinct_lanes: HashSet<&str> = entries.iter().map(|(l, _)| l.as_str()).collect();
            if distinct_lanes.len() != entries.len() {
                return Err(corrupt(format!(
                    "fleet run {fleet_id} binds the same lane more than once"
                )));
            }
            if !fleet.status.is_terminal() {
                continue;
            }
            if entries.is_empty() && fleet.status != FleetRunState::Cancelled {
                return Err(corrupt(format!(
                    "terminal fleet run {fleet_id} has no bound lane runs"
                )));
            }
            if entries.iter().any(|(_, run)| !run.status.is_terminal()) {
                return Err(corrupt(format!(
                    "terminal fleet run {fleet_id} binds a non-terminal agent run"
                )));
            }
            if fleet.status == FleetRunState::Cancelled {
                // `contracts.md` section 3: a fleet is `Cancelled` only when
                // it was cancelled before any lane reached another terminal
                // state, so every bound lane run is itself `Cancelled`.
                if entries
                    .iter()
                    .any(|(_, run)| run.status != AgentRunState::Cancelled)
                {
                    return Err(corrupt(format!(
                        "cancelled fleet run {fleet_id} binds a lane run that is not cancelled"
                    )));
                }
            } else {
                let states: Vec<_> = entries.iter().map(|(_, run)| run.status).collect();
                let aggregate = FleetRun::aggregate_state(&states);
                if aggregate != fleet.status {
                    return Err(corrupt(format!(
                        "fleet run {fleet_id} is {:?} but its bound runs aggregate to {aggregate:?}",
                        fleet.status
                    )));
                }
            }
        }

        for report in self.list_all_comparison_reports()? {
            let report_id = report.header.id.as_str();
            let fleet = fleet_runs
                .get(report.fleet_run_id.as_str())
                .ok_or_else(|| {
                    corrupt(format!(
                        "comparison report {report_id} names a non-existent fleet run {}",
                        report.fleet_run_id.as_str()
                    ))
                })?;
            if !matches!(
                fleet.status,
                FleetRunState::Completed | FleetRunState::PartiallyFailed
            ) {
                return Err(corrupt(format!(
                    "comparison report {report_id} exists for a {:?} fleet run",
                    fleet.status
                )));
            }
            report
                .validate()
                .map_err(|e| corrupt(format!("comparison report {report_id}: {e}")))?;
            let bound_lanes: HashSet<&str> = bound
                .get(report.fleet_run_id.as_str())
                .map(|v| v.iter().map(|(l, _)| l.as_str()).collect())
                .unwrap_or_default();
            let named = report
                .participating_lane_ids
                .iter()
                .chain(&report.excluded_lane_ids)
                .chain(
                    report
                        .observations
                        .iter()
                        .flat_map(|o| o.participating_lane_ids.iter()),
                );
            for lane_id in named {
                if !bound_lanes.contains(lane_id.as_str()) {
                    return Err(corrupt(format!(
                        "comparison report {report_id} names lane {} not bound to its fleet run",
                        lane_id.as_str()
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn list_all_agent_lanes(&self) -> Result<Vec<AgentLane>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT lane_id, project_id, realm_id, authority_scope_id, agent_identity_id, context_manifest_id, role_label, policy_json, status, revision, schema_version
             FROM model_fleet_lanes ORDER BY lane_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_agent_lane_row(row)?);
        }
        Ok(out)
    }

    pub fn list_all_fleet_runs(&self) -> Result<Vec<FleetRun>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT fleet_run_id, project_id, realm_id, authority_scope_id, task_prompt, status, revision, schema_version
             FROM model_fleet_runs ORDER BY fleet_run_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_fleet_run_row(row)?);
        }
        Ok(out)
    }

    pub fn list_all_lane_run_refs(&self) -> Result<Vec<LaneRunRef>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT lane_run_ref_id, fleet_run_id, realm_id, authority_scope_id, agent_lane_id, agent_run_id, schema_version
             FROM model_fleet_lane_run_refs ORDER BY lane_run_ref_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_lane_run_ref_row(row)?);
        }
        Ok(out)
    }

    pub fn list_all_comparison_reports(&self) -> Result<Vec<ComparisonReport>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT report_id, fleet_run_id, realm_id, authority_scope_id, observations_json, participating_lane_ids_json, excluded_lane_ids_json, schema_version
             FROM model_fleet_comparison_reports ORDER BY report_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_comparison_report_row(row)?);
        }
        Ok(out)
    }

    // Restore replays rows exactly (ids/revisions preserved) through the
    // same plain-INSERT paths as normal writes, never `INSERT OR REPLACE`:
    // a tampered snapshot carrying a duplicate id or a second binding of
    // one `agent_run_id` must fail closed, not be silently squashed.

    pub fn restore_agent_lane_row(&self, lane: &AgentLane) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_agent_lane(lane))
    }

    pub fn restore_fleet_run_row(&self, run: &FleetRun) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_fleet_run(run))
    }

    pub fn restore_lane_run_ref_row(&self, lane_run_ref: &LaneRunRef) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_lane_run_ref(lane_run_ref))
    }

    pub fn restore_comparison_report_row(
        &self,
        report: &ComparisonReport,
    ) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_comparison_report(report))
    }
}

//! Model Fleet + Compare Core authority paths (Spec 078).
//!
//! T078-03 scope: `AgentLane` + `LanePolicy` create/get/list/retire.
//! T078-04 scope: `FleetRun` create/get/list/dispatch/execute-lane/cancel,
//! `LaneRunRef` binding, `FleetRunState` aggregation, and the lane-policy
//! guard on Spec 077's direct tool path.
//! T078-05/06 scope: `ComparisonReport` compute (over committed lane output,
//! see `model_fleet_compare`) and per-fleet report history.
//!
//! A lane is a policy wrapper around an existing, unmodified Spec 077
//! `AgentIdentity`/`ContextManifest` pair. Every Spec 077 object this module
//! touches is read through Spec 077's own public, scope-checked `MedAgent`
//! entry points (`get_agent_identity`, `get_context_manifest`), never through
//! its storage rows or private helpers (`security.md` section 1).
//!
//! `LanePolicy` narrows, never widens, what the bound identity's
//! `AgentCapabilityManifest` and the bound `ContextManifest` already allow
//! (`security.md` T2): `LanePolicy::validate_within` runs against the
//! live-resolved manifests before any row is written, and a superset request
//! is `InvalidArgument` with nothing persisted.
//!
//! This module has no path to proposal promotion, amendment, effects, or
//! external actions (`security.md` T1); it imports none of them.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::medagent::{
    AgentIdentityStatus, AgentProposal, AgentRun, AgentRunState, ToolKind,
};
use medscale_contracts::model_fleet::{
    AgentLane, AgentLaneStatus, ComparisonReport, FLEET_RUN_MAX_LANES, FleetRun, FleetRunState,
    LanePolicy, LaneRunRef, MODEL_FLEET_SCHEMA_VERSION,
};
use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId, VaultId};
use medscale_storage::{MetaError, SqliteMetaStore};

use super::medagent::MedAgent;
use super::model_fleet_compare::{LaneOutput, LaneProposal, compute_observations};
use super::store::{InMemoryAuthorityStore, ScopeError, StoredObject};
use crate::process::{LeaseRegistry, SessionRegistry};

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

/// A fleet compares lanes, so dispatch needs at least two (`contracts.md`
/// section 3, T078-04 decision); the upper bound is the frozen
/// `FLEET_RUN_MAX_LANES`.
const FLEET_RUN_MIN_LANES: usize = 2;

/// Fixed, content-free classification recorded as a failed lane run's
/// `RunReceipt.failure_reason`. The raw error is never copied in: its text
/// can carry local paths or runtime detail (`security.md` T9).
fn bounded_failure_reason(err: &AuthorityError) -> String {
    let kind = match err {
        AuthorityError::DigestMismatch => "pack_identity_mismatch",
        AuthorityError::Unauthorized => "pack_not_admitted",
        AuthorityError::InvalidArgument { .. } => "execution_refused",
        AuthorityError::ExternalGateRequired { .. } => "external_gate_required",
        AuthorityError::Conflict { .. } => "conflict",
        AuthorityError::StaleReference { .. } => "no_current_evidence",
        _ => "execution_failed",
    };
    format!("fleet lane execution failed: {kind}")
}

fn invalid(message: String) -> AuthorityError {
    AuthorityError::InvalidArgument { message }
}

fn header(realm: &RealmId, scope: &AuthorityScopeId, id: OpaqueId) -> ObjectHeader {
    ObjectHeader {
        id,
        schema_version: MODEL_FLEET_SCHEMA_VERSION,
        realm_id: realm.clone(),
        authority_scope_id: scope.clone(),
    }
}

/// Authenticated Model Fleet authority view for one request.
pub struct ModelFleet<'a> {
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

impl ModelFleet<'_> {
    /// A Spec 077 `MedAgent` view over the same request authority. This is
    /// the only way this module reaches Spec 077 behavior.
    fn medagent(&mut self) -> MedAgent<'_> {
        MedAgent {
            store: &mut *self.store,
            meta: self.meta,
            packs: self.packs,
            sessions: self.sessions,
            leases: self.leases,
            vault_id: self.vault_id,
            realm: self.realm.clone(),
            scope: self.scope.clone(),
            session_id: self.session_id.clone(),
        }
    }

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

    /// Appends one scope-level audit row to the existing trail (the same
    /// `ActionAuditRecord` mechanism Spec 077 uses for its scope-level rows).
    fn audit(&mut self, action: &str, targets: Vec<OpaqueId>) -> Result<(), AuthorityError> {
        let actor = self.actor()?;
        let id = self.store.alloc_id("audit");
        let record = medscale_contracts::objects::ActionAuditRecord {
            header: ObjectHeader {
                id,
                schema_version: medscale_contracts::AUTHORITY_SCHEMA_VERSION,
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

    fn in_scope(&self, header: &ObjectHeader) -> Result<(), AuthorityError> {
        if header.realm_id != self.realm || header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(())
    }

    fn scoped_project(&self, id: &OpaqueId) -> Result<(), AuthorityError> {
        let project = self.meta.get_project(id).map_err(meta_err)?;
        self.in_scope(&project.header)
    }

    fn scoped_lane(&self, id: &OpaqueId) -> Result<AgentLane, AuthorityError> {
        let lane = self.meta.get_agent_lane(id).map_err(meta_err)?;
        self.in_scope(&lane.header)?;
        Ok(lane)
    }

    // ----- AgentLane + LanePolicy -----

    /// Creates an `Active` agent lane bound to an existing Spec 077
    /// identity/context pair in `project_id`. Refused before any write when
    /// the identity is revoked, either object belongs to another Project,
    /// or `policy` is not a genuine subset of the bound identity's
    /// capability manifest and the bound context manifest (`security.md` T2).
    pub fn create_agent_lane(
        &mut self,
        project_id: OpaqueId,
        agent_identity_id: OpaqueId,
        context_manifest_id: OpaqueId,
        role_label: String,
        granted_tool_kinds: Option<Vec<ToolKind>>,
        context_artifact_ids: Option<Vec<OpaqueId>>,
    ) -> Result<AgentLane, AuthorityError> {
        self.scoped_project(&project_id)?;
        let policy = LanePolicy::new(granted_tool_kinds, context_artifact_ids).map_err(invalid)?;
        let (identity, capabilities) = self.medagent().get_agent_identity(&agent_identity_id)?;
        if identity.status != AgentIdentityStatus::Active {
            return Err(invalid("agent identity is revoked".to_owned()));
        }
        if identity.project_id != project_id {
            return Err(invalid(
                "agent identity does not belong to this project".to_owned(),
            ));
        }
        let (context, _resolutions) = self.medagent().get_context_manifest(&context_manifest_id)?;
        if context.project_id != project_id {
            return Err(invalid(
                "context manifest does not belong to this project".to_owned(),
            ));
        }
        policy
            .validate_within(&capabilities, &context)
            .map_err(invalid)?;
        let id = self
            .meta
            .alloc_model_fleet_id("model-fleet-lane-seq", "lane")
            .map_err(meta_err)?;
        let lane = AgentLane::new(
            header(&self.realm, &self.scope, id.clone()),
            project_id,
            agent_identity_id,
            context_manifest_id,
            role_label,
            policy,
        )
        .map_err(invalid)?;
        self.meta.insert_agent_lane(&lane).map_err(meta_err)?;
        self.audit("model_fleet_lane.create", vec![id])?;
        Ok(lane)
    }

    /// Scoped lane read.
    pub fn get_agent_lane(&self, id: &OpaqueId) -> Result<AgentLane, AuthorityError> {
        self.scoped_lane(id)
    }

    /// Lists lanes in one Project, optionally by status. The Project is
    /// scope-checked first, so no cross-scope row can be named.
    pub fn list_agent_lanes(
        &self,
        project_id: &OpaqueId,
        status: Option<AgentLaneStatus>,
        limit: u32,
    ) -> Result<Vec<AgentLane>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta
            .list_agent_lanes(project_id, status, limit)
            .map_err(meta_err)
    }

    /// Retires an active lane (status tombstone; CAS on `expected_revision`).
    /// Past fleet history bound to the lane is never rewritten.
    pub fn retire_agent_lane(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<AgentLane, AuthorityError> {
        let lane = self.scoped_lane(id)?;
        lane.check_mutation(expected_revision)
            .map_err(|message| AuthorityError::Conflict { message })?;
        if lane.status == AgentLaneStatus::Retired {
            return Err(AuthorityError::Conflict {
                message: "agent lane is already retired".to_owned(),
            });
        }
        let retired = self
            .meta
            .retire_agent_lane(id, expected_revision)
            .map_err(meta_err)?;
        self.audit("model_fleet_lane.retire", vec![id.clone()])?;
        Ok(retired)
    }

    // ----- FleetRun lifecycle (T078-04) -----

    fn scoped_fleet_run(&self, id: &OpaqueId) -> Result<FleetRun, AuthorityError> {
        let run = self.meta.get_fleet_run(id).map_err(meta_err)?;
        self.in_scope(&run.header)?;
        Ok(run)
    }

    /// Re-resolves a lane's identity/context through Spec 077 and re-checks
    /// its policy (`security.md` T2: re-run at every dispatch, never cached).
    fn require_dispatchable_lane(
        &mut self,
        lane_id: &OpaqueId,
        project_id: &OpaqueId,
    ) -> Result<AgentLane, AuthorityError> {
        let lane = self.scoped_lane(lane_id)?;
        if lane.status != AgentLaneStatus::Active {
            return Err(invalid(format!(
                "agent lane {} is retired",
                lane_id.as_str()
            )));
        }
        if &lane.project_id != project_id {
            return Err(invalid(format!(
                "agent lane {} does not belong to this fleet's project",
                lane_id.as_str()
            )));
        }
        let (identity, capabilities) = self
            .medagent()
            .get_agent_identity(&lane.agent_identity_id)?;
        if identity.status != AgentIdentityStatus::Active {
            return Err(invalid(format!(
                "agent lane {} is bound to a revoked identity",
                lane_id.as_str()
            )));
        }
        // Spec 077's create/start re-check the bound Pack is admitted in
        // this Core process; check it here too, so a lane that would fail
        // there refuses the whole dispatch before the fleet leaves `Pending`.
        let admitted = self
            .packs
            .get(&identity.pack_id)
            .is_some_and(|pack| pack.version == identity.pack_version);
        if !admitted {
            return Err(invalid(format!(
                "agent lane {} is bound to a Pack that is not admitted",
                lane_id.as_str()
            )));
        }
        let (context, _resolutions) = self
            .medagent()
            .get_context_manifest(&lane.context_manifest_id)?;
        lane.policy
            .validate_within(&capabilities, &context)
            .map_err(invalid)?;
        Ok(lane)
    }

    /// Creates a `Pending` fleet run in `project_id`.
    pub fn create_fleet_run(
        &mut self,
        project_id: OpaqueId,
        task_prompt: String,
    ) -> Result<FleetRun, AuthorityError> {
        self.scoped_project(&project_id)?;
        let id = self
            .meta
            .alloc_model_fleet_id("model-fleet-run-seq", "fleet")
            .map_err(meta_err)?;
        let run = FleetRun::new(
            header(&self.realm, &self.scope, id.clone()),
            project_id,
            task_prompt,
        )
        .map_err(invalid)?;
        self.meta.insert_fleet_run(&run).map_err(meta_err)?;
        self.audit("model_fleet_run.create", vec![id])?;
        Ok(run)
    }

    /// Scoped fleet run read with its lane bindings.
    pub fn get_fleet_run(
        &self,
        id: &OpaqueId,
    ) -> Result<(FleetRun, Vec<LaneRunRef>), AuthorityError> {
        let run = self.scoped_fleet_run(id)?;
        let refs = self.meta.list_lane_run_refs(id).map_err(meta_err)?;
        Ok((run, refs))
    }

    /// Lists fleet runs in one Project, optionally by state.
    pub fn list_fleet_runs(
        &self,
        project_id: &OpaqueId,
        status: Option<FleetRunState>,
        limit: u32,
    ) -> Result<Vec<FleetRun>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta
            .list_fleet_runs(project_id, status, limit)
            .map_err(meta_err)
    }

    /// Dispatches a `Pending` fleet run over `lane_ids` (2..=
    /// `FLEET_RUN_MAX_LANES` distinct active lanes of the fleet's Project).
    ///
    /// Every lane is validated before anything is written. The fleet then
    /// moves `Pending -> Running` under CAS, which also makes a second
    /// dispatch a `Conflict`. Per lane, in the order `migration.md` section
    /// 5 freezes: Spec 077 `create_agent_run` (the fleet's task prompt,
    /// unchanged, against exactly the lane's bound identity/context), then
    /// the `LaneRunRef` binding, then Spec 077 `start_agent_run` -- a lane
    /// run is never started before its binding is durable. If a binding is
    /// refused, the just-created unbound run gets a compensating Spec 077
    /// `cancel_agent_run` before the error is returned.
    pub fn dispatch_fleet_run(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
        lane_ids: Vec<OpaqueId>,
    ) -> Result<(FleetRun, Vec<LaneRunRef>), AuthorityError> {
        let fleet = self.scoped_fleet_run(id)?;
        fleet
            .check_mutation(expected_revision)
            .map_err(|message| AuthorityError::Conflict { message })?;
        fleet
            .check_transition(FleetRunState::Running)
            .map_err(|message| AuthorityError::Conflict { message })?;
        if lane_ids.len() < FLEET_RUN_MIN_LANES || lane_ids.len() > FLEET_RUN_MAX_LANES {
            return Err(invalid(format!(
                "a fleet run dispatches {FLEET_RUN_MIN_LANES}..={FLEET_RUN_MAX_LANES} lanes"
            )));
        }
        let distinct: std::collections::HashSet<&str> =
            lane_ids.iter().map(OpaqueId::as_str).collect();
        if distinct.len() != lane_ids.len() {
            return Err(invalid(
                "a lane may be dispatched only once per fleet run".to_owned(),
            ));
        }
        let mut lanes = Vec::with_capacity(lane_ids.len());
        for lane_id in &lane_ids {
            lanes.push(self.require_dispatchable_lane(lane_id, &fleet.project_id)?);
        }

        let running = self
            .meta
            .transition_fleet_run(id, expected_revision, FleetRunState::Running)
            .map_err(meta_err)?;
        let mut refs = Vec::with_capacity(lanes.len());
        for lane in &lanes {
            let run = self.medagent().create_agent_run(
                running.project_id.clone(),
                lane.agent_identity_id.clone(),
                lane.context_manifest_id.clone(),
                running.task_prompt.clone(),
            )?;
            // security.md T4, Core half: never bind one AgentRun twice
            // (storage enforces the same with a UNIQUE index).
            if refs
                .iter()
                .any(|r: &LaneRunRef| r.agent_run_id == run.header.id)
                || self
                    .meta
                    .get_lane_run_ref_for_agent_run(&run.header.id)
                    .map_err(meta_err)?
                    .is_some()
            {
                let _ = self
                    .medagent()
                    .cancel_agent_run(&run.header.id, run.revision);
                return Err(AuthorityError::Conflict {
                    message: "agent run is already bound to a lane".to_owned(),
                });
            }
            let ref_id = self
                .meta
                .alloc_model_fleet_id("model-fleet-lane-run-ref-seq", "lref")
                .map_err(meta_err)?;
            let lane_run_ref = LaneRunRef {
                header: header(&self.realm, &self.scope, ref_id),
                fleet_run_id: id.clone(),
                agent_lane_id: lane.header.id.clone(),
                agent_run_id: run.header.id.clone(),
            };
            if let Err(err) = self.meta.insert_lane_run_ref(&lane_run_ref) {
                let _ = self
                    .medagent()
                    .cancel_agent_run(&run.header.id, run.revision);
                return Err(meta_err(err));
            }
            self.medagent()
                .start_agent_run(&run.header.id, run.revision)?;
            refs.push(lane_run_ref);
        }
        self.audit("model_fleet_run.dispatch", vec![id.clone()])?;
        Ok((running, refs))
    }

    /// The fleet's bound lane runs, freshly read through Spec 077.
    fn bound_lane_runs(
        &mut self,
        fleet_id: &OpaqueId,
    ) -> Result<Vec<(LaneRunRef, AgentRun)>, AuthorityError> {
        let refs = self.meta.list_lane_run_refs(fleet_id).map_err(meta_err)?;
        let mut out = Vec::with_capacity(refs.len());
        for lane_run_ref in refs {
            let run = self.medagent().get_agent_run(&lane_run_ref.agent_run_id)?;
            out.push((lane_run_ref, run));
        }
        Ok(out)
    }

    /// Recomputes a `Running` fleet's state from its lanes' live Spec 077
    /// states and writes it under CAS when it changed (read-then-write,
    /// `migration.md` section 5). Never selects a lane or a proposal.
    fn refresh_fleet_state(&mut self, fleet_id: &OpaqueId) -> Result<FleetRun, AuthorityError> {
        let fleet = self.scoped_fleet_run(fleet_id)?;
        if fleet.status != FleetRunState::Running {
            return Ok(fleet);
        }
        let states: Vec<AgentRunState> = self
            .bound_lane_runs(fleet_id)?
            .into_iter()
            .map(|(_, run)| run.status)
            .collect();
        let aggregate = FleetRun::aggregate_state(&states);
        if aggregate == fleet.status || !aggregate.is_terminal() {
            return Ok(fleet);
        }
        let updated = self
            .meta
            .transition_fleet_run(fleet_id, fleet.revision, aggregate)
            .map_err(meta_err)?;
        self.audit(
            &format!("model_fleet_run.{}", aggregate.as_str()),
            vec![fleet_id.clone()],
        )?;
        Ok(updated)
    }

    /// Executes one dispatched lane of a `Running` fleet through Spec 077's
    /// unmodified `execute_agent_run` against that lane's own run only, then
    /// closes the lane run through Spec 077: `complete_agent_run` on
    /// success, `fail_agent_run` (bounded reason) when execution is refused
    /// or fails. The fleet's state is then re-aggregated. Returns the
    /// fleet, the lane's terminal run, and its proposal when one exists.
    pub fn execute_fleet_lane(
        &mut self,
        fleet_id: &OpaqueId,
        lane_id: &OpaqueId,
        local_path: &str,
        max_tokens: usize,
        synthetic_only: bool,
    ) -> Result<(FleetRun, AgentRun, Option<AgentProposal>), AuthorityError> {
        // Gate first, exactly as Spec 077 does: a real-PHI request is
        // refused without touching the lane run.
        if !synthetic_only {
            return Err(AuthorityError::ExternalGateRequired {
                gate: "REAL_PHI_MODEL_RUNTIME".to_owned(),
            });
        }
        let fleet = self.scoped_fleet_run(fleet_id)?;
        if fleet.status != FleetRunState::Running {
            return Err(AuthorityError::Conflict {
                message: "fleet run is not Running".to_owned(),
            });
        }
        let lane_run_ref = self
            .meta
            .list_lane_run_refs(fleet_id)
            .map_err(meta_err)?
            .into_iter()
            .find(|r| &r.agent_lane_id == lane_id)
            .ok_or(AuthorityError::NotFound)?;
        let run_id = lane_run_ref.agent_run_id;
        let run = self.medagent().get_agent_run(&run_id)?;
        if run.status != AgentRunState::Running {
            return Err(AuthorityError::Conflict {
                message: "lane run is not Running".to_owned(),
            });
        }
        let executed =
            self.medagent()
                .execute_agent_run(&run_id, local_path, max_tokens, synthetic_only);
        let (terminal, proposal) = match executed {
            Ok((_turn, proposal)) => {
                let current = self.medagent().get_agent_run(&run_id)?;
                let (terminal, _receipt) = self
                    .medagent()
                    .complete_agent_run(&run_id, current.revision)?;
                (terminal, Some(proposal))
            }
            Err(err) => {
                let reason = bounded_failure_reason(&err);
                let current = self.medagent().get_agent_run(&run_id)?;
                let (terminal, _receipt) =
                    self.medagent()
                        .fail_agent_run(&run_id, current.revision, reason)?;
                (terminal, None)
            }
        };
        let fleet = self.refresh_fleet_state(fleet_id)?;
        Ok((fleet, terminal, proposal))
    }

    /// Cancels a `Pending` or `Running` fleet (`security.md` T6). Every
    /// still-in-flight lane run is cancelled through Spec 077's own
    /// `cancel_agent_run`; a lane that reached a terminal state first keeps
    /// it (Spec 077's transition check refuses the late cancel). The fleet
    /// then lands `Cancelled` only when every bound lane ended `Cancelled`
    /// (`contracts.md` section 3); otherwise it takes the aggregate of its
    /// lanes' real terminal states. First terminal write wins under CAS.
    pub fn cancel_fleet_run(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<(FleetRun, Vec<LaneRunRef>), AuthorityError> {
        let fleet = self.scoped_fleet_run(id)?;
        fleet
            .check_mutation(expected_revision)
            .map_err(|message| AuthorityError::Conflict { message })?;
        fleet
            .check_transition(FleetRunState::Cancelled)
            .map_err(|message| AuthorityError::Conflict { message })?;
        for (_, run) in self.bound_lane_runs(id)? {
            if run.status.is_terminal() {
                continue;
            }
            match self
                .medagent()
                .cancel_agent_run(&run.header.id, run.revision)
            {
                Ok(_) => {}
                // Lost the race to a concurrent terminal transition: the
                // lane keeps the state it actually reached.
                Err(AuthorityError::Conflict { .. }) => {
                    let current = self.medagent().get_agent_run(&run.header.id)?;
                    if !current.status.is_terminal() {
                        return Err(AuthorityError::Conflict {
                            message: "lane run changed during fleet cancel".to_owned(),
                        });
                    }
                }
                Err(other) => return Err(other),
            }
        }
        let lanes = self.bound_lane_runs(id)?;
        let states: Vec<AgentRunState> = lanes.iter().map(|(_, run)| run.status).collect();
        let final_state = if states.iter().all(|s| *s == AgentRunState::Cancelled) {
            FleetRunState::Cancelled
        } else {
            FleetRun::aggregate_state(&states)
        };
        let updated = self
            .meta
            .transition_fleet_run(id, expected_revision, final_state)
            .map_err(meta_err)?;
        self.audit(
            &format!("model_fleet_run.cancel.{}", final_state.as_str()),
            vec![id.clone()],
        )?;
        Ok((updated, lanes.into_iter().map(|(r, _)| r).collect()))
    }

    /// Lane-policy guard for Spec 077's direct tool path (`security.md` T2).
    /// A run bound to a lane may only use the lane's effective tool kinds;
    /// `ReadContextArtifact` may only name the lane's narrowed artifacts;
    /// and `SearchContextArtifacts`, which scans the whole bound context
    /// manifest inside Spec 077, is refused outright for a lane whose policy
    /// narrows the context (fail closed rather than leak outside the subset).
    /// Runs not bound to any lane are left to Spec 077's own checks.
    pub fn require_lane_policy_allows_tool(
        &mut self,
        run_id: &OpaqueId,
        kind: ToolKind,
        arguments: &serde_json::Value,
    ) -> Result<(), AuthorityError> {
        let Some(lane_run_ref) = self
            .meta
            .get_lane_run_ref_for_agent_run(run_id)
            .map_err(meta_err)?
        else {
            return Ok(());
        };
        let lane = self.scoped_lane(&lane_run_ref.agent_lane_id)?;
        let (_identity, capabilities) = self
            .medagent()
            .get_agent_identity(&lane.agent_identity_id)?;
        if !lane
            .policy
            .effective_tool_kinds(&capabilities)
            .contains(&kind)
        {
            return Err(AuthorityError::Unauthorized);
        }
        if let Some(allowed) = &lane.policy.context_artifact_ids {
            match kind {
                ToolKind::SearchContextArtifacts => return Err(AuthorityError::Unauthorized),
                ToolKind::ReadContextArtifact => {
                    let named = arguments
                        .get("object_id")
                        .and_then(serde_json::Value::as_str);
                    if !named.is_some_and(|id| allowed.iter().any(|a| a.as_str() == id)) {
                        return Err(AuthorityError::Unauthorized);
                    }
                }
            }
        }
        Ok(())
    }
    // ----- Comparison (T078-05) + history (T078-06) -----

    /// Computes and persists one immutable `ComparisonReport` over a
    /// `Completed`/`PartiallyFailed` fleet run (`contracts.md` section 4).
    ///
    /// A pure read of committed state: each completed lane's `RunReceipt`,
    /// `AgentProposal` and `Proposal` (read-only, after the lane run is
    /// scope-checked through Spec 077's public `get_agent_run`), plus the
    /// lane's effective context. Failed/cancelled lanes are named in
    /// `excluded_lane_ids`, never dropped. The only write is the new report
    /// row; recomputation always creates a new report (`security.md` T1/T8).
    pub fn compute_comparison(
        &mut self,
        fleet_run_id: &OpaqueId,
    ) -> Result<ComparisonReport, AuthorityError> {
        let fleet = self.scoped_fleet_run(fleet_run_id)?;
        if !matches!(
            fleet.status,
            FleetRunState::Completed | FleetRunState::PartiallyFailed
        ) {
            return Err(invalid(format!(
                "a {} fleet run has no comparable lane output",
                fleet.status.as_str()
            )));
        }
        let mut participating = Vec::new();
        let mut excluded = Vec::new();
        let mut outputs = Vec::new();
        for (lane_run_ref, run) in self.bound_lane_runs(fleet_run_id)? {
            match run.status {
                AgentRunState::Completed => {}
                AgentRunState::Failed | AgentRunState::Cancelled => {
                    excluded.push(lane_run_ref.agent_lane_id);
                    continue;
                }
                AgentRunState::Pending | AgentRunState::Running => {
                    return Err(AuthorityError::Corrupt {
                        message: "a terminal fleet run binds a non-terminal lane run".to_owned(),
                    });
                }
            }
            let lane = self.scoped_lane(&lane_run_ref.agent_lane_id)?;
            let receipt = self
                .meta
                .get_run_receipt(&run.header.id)
                .map_err(meta_err)?;
            let proposal = match self
                .meta
                .get_agent_proposal_for_run(&run.header.id)
                .map_err(meta_err)?
            {
                Some(agent_proposal) => {
                    let proposal = self
                        .store
                        .get_proposal(&agent_proposal.proposal_id, &self.realm, &self.scope)
                        .map_err(|err| match err {
                            ScopeError::NotFound => AuthorityError::NotFound,
                            ScopeError::WrongScope => AuthorityError::WrongScope,
                        })?;
                    Some(LaneProposal {
                        agent_proposal_id: agent_proposal.header.id,
                        payload: proposal.payload.clone(),
                        evidence_refs: proposal.evidence_refs.clone(),
                    })
                }
                None => None,
            };
            let effective_context = match &lane.policy.context_artifact_ids {
                Some(ids) => ids.iter().cloned().collect(),
                None => self
                    .medagent()
                    .get_context_manifest(&lane.context_manifest_id)?
                    .0
                    .selected_artifacts
                    .into_iter()
                    .map(|artifact| artifact.object_id)
                    .collect(),
            };
            participating.push(lane.header.id.clone());
            outputs.push(LaneOutput {
                lane_id: lane.header.id,
                run_receipt_id: receipt.header.id,
                pack_id: receipt.pack_id,
                pack_version: receipt.pack_version,
                tool_invocation_count: receipt.tool_invocation_ids.len(),
                proposal,
                effective_context,
            });
        }
        let id = self
            .meta
            .alloc_model_fleet_id("model-fleet-report-seq", "report")
            .map_err(meta_err)?;
        let report = ComparisonReport {
            header: header(&self.realm, &self.scope, id.clone()),
            fleet_run_id: fleet_run_id.clone(),
            observations: compute_observations(&outputs),
            participating_lane_ids: participating,
            excluded_lane_ids: excluded,
        };
        report
            .validate()
            .map_err(|message| AuthorityError::Internal {
                message: format!("comparison produced an invalid report: {message}"),
            })?;
        self.meta
            .insert_comparison_report(&report)
            .map_err(meta_err)?;
        self.audit("model_fleet_comparison.compute", vec![id])?;
        Ok(report)
    }

    /// Every comparison report computed over one fleet run, oldest first.
    pub fn list_comparison_reports(
        &self,
        fleet_run_id: &OpaqueId,
    ) -> Result<Vec<ComparisonReport>, AuthorityError> {
        self.scoped_fleet_run(fleet_run_id)?;
        self.meta
            .list_comparison_reports(fleet_run_id)
            .map_err(meta_err)
    }
}

#[cfg(test)]
mod tests {
    /// `security.md` T1: the non-test part of this module has no path to
    /// proposal promotion, amendment, effects, or external actions.
    #[test]
    fn module_has_no_path_to_promotion_amendment_effects_or_actions() {
        for (name, source) in [
            ("model_fleet", include_str!("model_fleet.rs")),
            (
                "model_fleet_compare",
                include_str!("model_fleet_compare.rs"),
            ),
        ] {
            let production = source.split("#[cfg(test)]").next().expect("module source");
            for forbidden in [
                "promote::",
                "amend::",
                "actions::",
                "PromoteProposal",
                "TransitionEffect",
                "ExternalActionIntent",
                "Outbox",
                "ClinicalAssertion",
            ] {
                assert!(
                    !production.contains(forbidden),
                    "authority::{name} must not reference `{forbidden}`"
                );
            }
        }
    }
}

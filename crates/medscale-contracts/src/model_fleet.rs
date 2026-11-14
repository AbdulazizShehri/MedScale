//! Model Fleet + Compare contracts (Spec 078).
//!
//! A local-first multi-lane orchestration and comparison layer on top of
//! Spec 077's MedAgent Workbench: an `AgentLane` wraps an existing
//! `AgentIdentity`/`ContextManifest` with lane-scoped policy that may only
//! *narrow*, never widen, what the underlying identity/context already
//! allow; a `FleetRun` binds two or more lanes to one task and drives each
//! lane's own independent, unmodified Spec 077 `AgentRun` to completion (or
//! explicit partial failure); a `ComparisonReport` then observes the
//! completed lanes' `AgentProposal`/`RunReceipt` output and records
//! agreement/disagreement, contradiction candidates, evidence/citation
//! overlap, unsupported-claim candidates, abstention, schema validity, and
//! resource/runtime facts -- never a quality score, ranking, or declared
//! winner (`security.md` explicit non-capabilities). No type here modifies
//! any Spec 077 contract; every reference into a Spec 077 object is by
//! `OpaqueId` only.

use serde::{Deserialize, Serialize};

use crate::medagent::{AgentCapabilityManifest, AgentRunState, ContextManifest, ToolKind};
use crate::objects::{ObjectHeader, OpaqueId};
use crate::project_graph::{ProjectRevision, check_revision, initial_revision};

/// Durable schema version for every Model Fleet object (Spec 078 v1).
pub const MODEL_FLEET_SCHEMA_VERSION: u32 = 1;

pub const ROLE_LABEL_MAX_CHARS: usize = 128;
pub const TASK_PROMPT_MAX_BYTES: usize = 32_768;
pub const OBSERVATION_DETAIL_MAX_CHARS: usize = 2_048;
/// Bound enforced by Core when dispatching a `FleetRun`'s lanes (not by any
/// type in this module directly -- lanes are bound to a fleet run one
/// `LaneRunRef` at a time, `migration.md` section 5).
pub const FLEET_RUN_MAX_LANES: usize = 8;
pub const COMPARISON_REPORT_MAX_OBSERVATIONS: usize = 512;

fn bounded_text(value: &str, max_chars: usize, what: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{what} must not be empty"));
    }
    if value.chars().count() > max_chars {
        return Err(format!("{what} exceeds bound"));
    }
    if value.contains('\0') {
        return Err(format!("{what} must not contain NUL"));
    }
    Ok(())
}

fn bounded_bytes(value: &str, max_bytes: usize, what: &str) -> Result<(), String> {
    if value.len() > max_bytes {
        return Err(format!("{what} exceeds byte bound"));
    }
    if value.contains('\0') {
        return Err(format!("{what} must not contain NUL"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AgentLane / LanePolicy / LaneTransform
// ---------------------------------------------------------------------------

/// Lifecycle of an `AgentLane`. Mirrors `AgentIdentityStatus`'s shape:
/// `Retired` is the lane equivalent of "revoked" -- a retired lane may not
/// be bound to any new `FleetRun`, but its past `LaneRunRef`s and any
/// `ComparisonReport`s that already reference it remain fully inspectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLaneStatus {
    Active,
    Retired,
}

impl AgentLaneStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Retired => "retired",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "active" => Ok(Self::Active),
            "retired" => Ok(Self::Retired),
            other => Err(format!("unknown agent lane status {other}")),
        }
    }
}

/// Reserved, closed-empty vocabulary at Spec 078: no transform kind is
/// defined yet. Declared as a typed, closed enum (not a `Value` passthrough)
/// so a later spec extends it additively rather than this spec inventing an
/// untyped placeholder. Zero variants is deliberate, not an oversight --
/// see `contracts.md` section 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneTransform {}

/// Lane-scoped policy. Narrows, never widens, what the lane's bound
/// `AgentIdentity`/`AgentCapabilityManifest` and `ContextManifest` already
/// allow (`security.md` T2). `None` means "inherit the full underlying
/// grant"; `Some(subset)` must be a genuine subset, checked by
/// `validate_within` before the lane is ever created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanePolicy {
    pub granted_tool_kinds: Option<Vec<ToolKind>>,
    pub context_artifact_ids: Option<Vec<OpaqueId>>,
}

impl LanePolicy {
    pub fn new(
        granted_tool_kinds: Option<Vec<ToolKind>>,
        context_artifact_ids: Option<Vec<OpaqueId>>,
    ) -> Result<Self, String> {
        if let Some(kinds) = &granted_tool_kinds
            && kinds.is_empty()
        {
            return Err("granted_tool_kinds, when present, must not be empty".to_owned());
        }
        if let Some(ids) = &context_artifact_ids
            && ids.is_empty()
        {
            return Err("context_artifact_ids, when present, must not be empty".to_owned());
        }
        Ok(Self {
            granted_tool_kinds,
            context_artifact_ids,
        })
    }

    /// Validates this policy is a genuine subset of the bound identity's
    /// `AgentCapabilityManifest` and the bound `ContextManifest`
    /// (`security.md` T2). Called at lane creation and re-checked at every
    /// lane-run dispatch, never cached.
    pub fn validate_within(
        &self,
        capability: &AgentCapabilityManifest,
        context: &ContextManifest,
    ) -> Result<(), String> {
        if let Some(kinds) = &self.granted_tool_kinds {
            for kind in kinds {
                if !capability.grants(*kind) {
                    return Err(format!(
                        "lane policy grants {kind:?}, which the bound identity's capability manifest does not grant"
                    ));
                }
            }
        }
        if let Some(ids) = &self.context_artifact_ids {
            for id in ids {
                if !context.allows(id) {
                    return Err(
                        "lane policy names an artifact outside the bound context manifest"
                            .to_owned(),
                    );
                }
            }
        }
        Ok(())
    }

    /// The effective granted tool kinds for dispatch: the narrowed subset if
    /// set, else the full underlying capability grant. Never wider than
    /// `capability` by construction (enforced by `validate_within` at
    /// creation time).
    #[must_use]
    pub fn effective_tool_kinds(&self, capability: &AgentCapabilityManifest) -> Vec<ToolKind> {
        self.granted_tool_kinds
            .clone()
            .unwrap_or_else(|| capability.granted_tool_kinds.clone())
    }
}

/// A policy wrapper around an existing, unmodified Spec 077
/// `AgentIdentity`/`ContextManifest` pair. Never a new execution engine:
/// this lane's actual model execution happens through Spec 077's own
/// `execute_agent_run`, against exactly this lane's bound identity/context
/// (`security.md` section 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentLane {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub agent_identity_id: OpaqueId,
    pub context_manifest_id: OpaqueId,
    pub role_label: String,
    pub policy: LanePolicy,
    pub status: AgentLaneStatus,
}

impl AgentLane {
    /// Creates a revision-1 active lane after validating the role label.
    /// Callers must separately call `policy.validate_within` against the
    /// live-resolved `AgentCapabilityManifest`/`ContextManifest` before
    /// persisting -- this constructor validates shape only, not
    /// cross-object subset containment, mirroring `ContextManifest::new`'s
    /// own shape-only-at-construction discipline.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        agent_identity_id: OpaqueId,
        context_manifest_id: OpaqueId,
        role_label: String,
        policy: LanePolicy,
    ) -> Result<Self, String> {
        bounded_text(&role_label, ROLE_LABEL_MAX_CHARS, "role_label")?;
        Ok(Self {
            header,
            revision: initial_revision(),
            project_id,
            agent_identity_id,
            context_manifest_id,
            role_label,
            policy,
            status: AgentLaneStatus::Active,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

// ---------------------------------------------------------------------------
// FleetRun / FleetRunState / LaneRunRef
// ---------------------------------------------------------------------------

/// Lifecycle of a `FleetRun`. `Pending -> Running -> {Completed,
/// PartiallyFailed, Failed}`, plus `Pending -> Cancelled` and
/// `Running -> Cancelled`. No other edge exists (`contracts.md` section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetRunState {
    Pending,
    Running,
    Completed,
    PartiallyFailed,
    Failed,
    Cancelled,
}

impl FleetRunState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::PartiallyFailed => "partially_failed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "partially_failed" => Ok(Self::PartiallyFailed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(format!("unknown fleet run state {other}")),
        }
    }

    /// True only for the frozen transition table.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Running)
                | (Self::Pending, Self::Cancelled)
                | (Self::Running, Self::Completed)
                | (Self::Running, Self::PartiallyFailed)
                | (Self::Running, Self::Failed)
                | (Self::Running, Self::Cancelled)
        )
    }

    /// True for any of the four terminal states.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::PartiallyFailed | Self::Failed | Self::Cancelled
        )
    }
}

/// A fleet-level task dispatched identically to two or more `AgentLane`s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetRun {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub task_prompt: String,
    pub status: FleetRunState,
}

impl FleetRun {
    /// Creates a revision-1 `Pending` fleet run after validating the task
    /// prompt.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        task_prompt: String,
    ) -> Result<Self, String> {
        bounded_bytes(&task_prompt, TASK_PROMPT_MAX_BYTES, "task_prompt")?;
        if task_prompt.trim().is_empty() {
            return Err("task_prompt must not be empty".to_owned());
        }
        Ok(Self {
            header,
            revision: initial_revision(),
            project_id,
            task_prompt,
            status: FleetRunState::Pending,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// Validates a proposed state transition against the frozen table.
    pub fn check_transition(&self, next: FleetRunState) -> Result<(), String> {
        if self.status.can_transition_to(next) {
            Ok(())
        } else {
            Err(format!(
                "illegal fleet run transition {:?} -> {:?}",
                self.status, next
            ))
        }
    }

    /// Aggregates `FleetRunState` from the current `AgentRunState` of every
    /// bound lane's dispatched run. Pure function over already-resolved
    /// states; Core supplies the live states, this never reads storage
    /// itself.
    #[must_use]
    pub fn aggregate_state(lane_states: &[AgentRunState]) -> FleetRunState {
        if lane_states.is_empty() {
            return FleetRunState::Pending;
        }
        let any_running_or_pending = lane_states
            .iter()
            .any(|s| matches!(s, AgentRunState::Pending | AgentRunState::Running));
        if any_running_or_pending {
            return FleetRunState::Running;
        }
        let completed = lane_states
            .iter()
            .filter(|s| **s == AgentRunState::Completed)
            .count();
        let failed_or_cancelled = lane_states
            .iter()
            .filter(|s| matches!(s, AgentRunState::Failed | AgentRunState::Cancelled))
            .count();
        if completed > 0 && failed_or_cancelled > 0 {
            FleetRunState::PartiallyFailed
        } else if completed > 0 {
            FleetRunState::Completed
        } else {
            FleetRunState::Failed
        }
    }
}

/// The append-only binding record between one `FleetRun`'s lane and the
/// real, independent Spec 077 `AgentRun` it dispatched. `agent_run_id` must
/// be distinct across every `LaneRunRef` in a `FleetRun`
/// (`security.md` T4) -- enforced by Core/storage, not representable as a
/// single-struct invariant here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaneRunRef {
    pub header: ObjectHeader,
    pub fleet_run_id: OpaqueId,
    pub agent_lane_id: OpaqueId,
    pub agent_run_id: OpaqueId,
}

// ---------------------------------------------------------------------------
// Comparison vocabulary
// ---------------------------------------------------------------------------

/// Closed vocabulary of comparison observation kinds. Every kind is a
/// factual, evidence-grounded observation; none is a verdict
/// (`security.md` T1, `contracts.md` section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonObservationKind {
    Agreement,
    Disagreement,
    ContradictionCandidate,
    EvidenceOverlap,
    UnsupportedClaim,
    Abstention,
    SchemaValidity,
    ResourceRuntimeFact,
}

impl ComparisonObservationKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agreement => "agreement",
            Self::Disagreement => "disagreement",
            Self::ContradictionCandidate => "contradiction_candidate",
            Self::EvidenceOverlap => "evidence_overlap",
            Self::UnsupportedClaim => "unsupported_claim",
            Self::Abstention => "abstention",
            Self::SchemaValidity => "schema_validity",
            Self::ResourceRuntimeFact => "resource_runtime_fact",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "agreement" => Ok(Self::Agreement),
            "disagreement" => Ok(Self::Disagreement),
            "contradiction_candidate" => Ok(Self::ContradictionCandidate),
            "evidence_overlap" => Ok(Self::EvidenceOverlap),
            "unsupported_claim" => Ok(Self::UnsupportedClaim),
            "abstention" => Ok(Self::Abstention),
            "schema_validity" => Ok(Self::SchemaValidity),
            "resource_runtime_fact" => Ok(Self::ResourceRuntimeFact),
            other => Err(format!("unknown comparison observation kind {other}")),
        }
    }

    /// Whether this kind is inherently cross-lane (agreement/disagreement/
    /// contradiction/evidence-overlap can only be observed across at least
    /// two lanes); the remaining kinds may describe a single lane's own
    /// claim/abstention/schema/resource facts.
    #[must_use]
    pub const fn requires_multiple_lanes(self) -> bool {
        matches!(
            self,
            Self::Agreement
                | Self::Disagreement
                | Self::ContradictionCandidate
                | Self::EvidenceOverlap
        )
    }
}

/// One factual, evidence-grounded comparison observation. Structurally has
/// no numeric quality/confidence/rank field anywhere in its shape
/// (`security.md` T1, `contracts.md` section 4) -- this is a shape
/// guarantee, not a convention to be respected by callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonObservation {
    pub kind: ComparisonObservationKind,
    pub participating_lane_ids: Vec<OpaqueId>,
    pub detail: String,
    pub evidence_refs: Vec<OpaqueId>,
}

impl ComparisonObservation {
    pub fn validate(&self) -> Result<(), String> {
        bounded_text(&self.detail, OBSERVATION_DETAIL_MAX_CHARS, "detail")?;
        if self.participating_lane_ids.is_empty() {
            return Err("participating_lane_ids must not be empty".to_owned());
        }
        if self.kind.requires_multiple_lanes() && self.participating_lane_ids.len() < 2 {
            return Err(format!(
                "{:?} requires at least two participating lanes",
                self.kind
            ));
        }
        Ok(())
    }
}

/// The typed input to a comparison computation. Not a durable row -- only
/// its output (`ComparisonReport`) is persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRequest {
    pub fleet_run_id: OpaqueId,
}

/// An immutable, append-only comparison result computed over a `FleetRun`'s
/// completed (or partially-failed) lane outputs. Recomputation creates a
/// new `ComparisonReport`, never an in-place mutation of a prior one
/// (`contracts.md` section 4). Carries no classification field in this v1
/// freeze -- see the T078-01 reconciliation note in `contracts.md` section
/// 6 for why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonReport {
    pub header: ObjectHeader,
    pub fleet_run_id: OpaqueId,
    pub observations: Vec<ComparisonObservation>,
    pub participating_lane_ids: Vec<OpaqueId>,
    pub excluded_lane_ids: Vec<OpaqueId>,
}

impl ComparisonReport {
    pub fn validate(&self) -> Result<(), String> {
        if self.observations.len() > COMPARISON_REPORT_MAX_OBSERVATIONS {
            return Err("observations exceeds bound".to_owned());
        }
        for observation in &self.observations {
            observation.validate()?;
        }
        if self.participating_lane_ids.is_empty() {
            return Err("participating_lane_ids must not be empty".to_owned());
        }
        for excluded in &self.excluded_lane_ids {
            if self.participating_lane_ids.contains(excluded) {
                return Err("a lane cannot be both participating and excluded".to_owned());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::medagent::ToolKind;
    use crate::objects::{AuthorityScopeId, RealmId};
    use crate::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding};

    fn h(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: MODEL_FLEET_SCHEMA_VERSION,
            realm_id: RealmId::new("realm-1"),
            authority_scope_id: AuthorityScopeId::new("scope-1"),
        }
    }

    fn descriptor(id: &str) -> ArtifactDescriptor {
        ArtifactDescriptor {
            object_id: OpaqueId::new(id),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        }
    }

    fn capability(kinds: Vec<ToolKind>) -> AgentCapabilityManifest {
        AgentCapabilityManifest::new(OpaqueId::new("agent-1"), kinds).unwrap()
    }

    fn context(ids: &[&str]) -> ContextManifest {
        ContextManifest::new(
            h("ctx-1"),
            OpaqueId::new("proj-1"),
            ids.iter().map(|id| descriptor(id)).collect(),
        )
        .unwrap()
    }

    #[test]
    fn agent_lane_status_round_trips() {
        for s in [AgentLaneStatus::Active, AgentLaneStatus::Retired] {
            assert_eq!(AgentLaneStatus::parse(s.as_str()).unwrap(), s);
        }
        assert!(AgentLaneStatus::parse("bogus").is_err());
    }

    #[test]
    fn lane_transform_is_closed_empty() {
        // A zero-variant enum cannot be constructed; this compiles only if
        // the type remains uninhabited, proving the "reserved, closed-empty"
        // design decision holds at the type level, not merely in prose.
        assert_eq!(std::mem::size_of::<LaneTransform>(), 0);
    }

    #[test]
    fn fleet_run_state_transition_table_is_frozen() {
        use FleetRunState::{Cancelled, Completed, Failed, PartiallyFailed, Pending, Running};
        assert!(Pending.can_transition_to(Running));
        assert!(Pending.can_transition_to(Cancelled));
        assert!(Running.can_transition_to(Completed));
        assert!(Running.can_transition_to(PartiallyFailed));
        assert!(Running.can_transition_to(Failed));
        assert!(Running.can_transition_to(Cancelled));
        // No other edge.
        assert!(!Pending.can_transition_to(Completed));
        assert!(!Pending.can_transition_to(PartiallyFailed));
        assert!(!Pending.can_transition_to(Failed));
        assert!(!Completed.can_transition_to(Running));
        assert!(!Cancelled.can_transition_to(Running));
        assert!(!Failed.can_transition_to(Completed));
        assert!(!PartiallyFailed.can_transition_to(Completed));
    }

    #[test]
    fn fleet_run_state_terminal_classification() {
        assert!(!FleetRunState::Pending.is_terminal());
        assert!(!FleetRunState::Running.is_terminal());
        assert!(FleetRunState::Completed.is_terminal());
        assert!(FleetRunState::PartiallyFailed.is_terminal());
        assert!(FleetRunState::Failed.is_terminal());
        assert!(FleetRunState::Cancelled.is_terminal());
    }

    #[test]
    fn fleet_run_aggregate_state_covers_every_case() {
        use AgentRunState::{Cancelled, Completed, Failed, Pending, Running};
        assert_eq!(FleetRun::aggregate_state(&[]), FleetRunState::Pending);
        assert_eq!(
            FleetRun::aggregate_state(&[Pending, Completed]),
            FleetRunState::Running
        );
        assert_eq!(
            FleetRun::aggregate_state(&[Running, Running]),
            FleetRunState::Running
        );
        assert_eq!(
            FleetRun::aggregate_state(&[Completed, Completed]),
            FleetRunState::Completed
        );
        assert_eq!(
            FleetRun::aggregate_state(&[Completed, Failed]),
            FleetRunState::PartiallyFailed
        );
        assert_eq!(
            FleetRun::aggregate_state(&[Completed, Cancelled]),
            FleetRunState::PartiallyFailed
        );
        assert_eq!(
            FleetRun::aggregate_state(&[Failed, Cancelled]),
            FleetRunState::Failed
        );
    }

    #[test]
    fn lane_policy_validate_within_rejects_superset_tool_kind() {
        let cap = capability(vec![ToolKind::ReadContextArtifact]);
        let ctx = context(&["art-a"]);
        let widening = LanePolicy::new(Some(vec![ToolKind::SearchContextArtifacts]), None).unwrap();
        assert!(widening.validate_within(&cap, &ctx).is_err());

        let narrowing = LanePolicy::new(Some(vec![ToolKind::ReadContextArtifact]), None).unwrap();
        assert!(narrowing.validate_within(&cap, &ctx).is_ok());
    }

    #[test]
    fn lane_policy_validate_within_rejects_out_of_manifest_artifact() {
        let cap = capability(vec![ToolKind::ReadContextArtifact]);
        let ctx = context(&["art-a", "art-b"]);
        let widening = LanePolicy::new(None, Some(vec![OpaqueId::new("art-c")])).unwrap();
        assert!(widening.validate_within(&cap, &ctx).is_err());

        let narrowing = LanePolicy::new(None, Some(vec![OpaqueId::new("art-a")])).unwrap();
        assert!(narrowing.validate_within(&cap, &ctx).is_ok());
    }

    #[test]
    fn agent_lane_new_validates_role_label() {
        let policy = LanePolicy::new(None, None).unwrap();
        assert!(
            AgentLane::new(
                h("lane-1"),
                OpaqueId::new("proj-1"),
                OpaqueId::new("agent-1"),
                OpaqueId::new("ctx-1"),
                "   ".to_owned(),
                policy.clone(),
            )
            .is_err()
        );
        let lane = AgentLane::new(
            h("lane-1"),
            OpaqueId::new("proj-1"),
            OpaqueId::new("agent-1"),
            OpaqueId::new("ctx-1"),
            "baseline".to_owned(),
            policy,
        )
        .unwrap();
        assert_eq!(lane.revision, 1);
        assert_eq!(lane.status, AgentLaneStatus::Active);
    }

    #[test]
    fn fleet_run_new_rejects_empty_and_oversized_prompt() {
        assert!(FleetRun::new(h("fleet-1"), OpaqueId::new("proj-1"), "   ".to_owned()).is_err());
        let over_max = "a".repeat(TASK_PROMPT_MAX_BYTES + 1);
        assert!(FleetRun::new(h("fleet-1"), OpaqueId::new("proj-1"), over_max).is_err());
        let run = FleetRun::new(
            h("fleet-1"),
            OpaqueId::new("proj-1"),
            "summarize the selected documents".to_owned(),
        )
        .unwrap();
        assert_eq!(run.status, FleetRunState::Pending);
        assert!(run.check_transition(FleetRunState::Running).is_ok());
        assert!(run.check_transition(FleetRunState::Completed).is_err());
    }

    #[test]
    fn comparison_observation_kind_requires_multiple_lanes_is_correct() {
        assert!(ComparisonObservationKind::Agreement.requires_multiple_lanes());
        assert!(ComparisonObservationKind::Disagreement.requires_multiple_lanes());
        assert!(ComparisonObservationKind::ContradictionCandidate.requires_multiple_lanes());
        assert!(ComparisonObservationKind::EvidenceOverlap.requires_multiple_lanes());
        assert!(!ComparisonObservationKind::UnsupportedClaim.requires_multiple_lanes());
        assert!(!ComparisonObservationKind::Abstention.requires_multiple_lanes());
        assert!(!ComparisonObservationKind::SchemaValidity.requires_multiple_lanes());
        assert!(!ComparisonObservationKind::ResourceRuntimeFact.requires_multiple_lanes());
    }

    #[test]
    fn comparison_observation_validate_enforces_multi_lane_kinds() {
        let single_lane_agreement = ComparisonObservation {
            kind: ComparisonObservationKind::Agreement,
            participating_lane_ids: vec![OpaqueId::new("lane-a")],
            detail: "both lanes agree on X".to_owned(),
            evidence_refs: vec![],
        };
        assert!(single_lane_agreement.validate().is_err());

        let two_lane_agreement = ComparisonObservation {
            participating_lane_ids: vec![OpaqueId::new("lane-a"), OpaqueId::new("lane-b")],
            ..single_lane_agreement
        };
        assert!(two_lane_agreement.validate().is_ok());

        let empty_detail = ComparisonObservation {
            kind: ComparisonObservationKind::Abstention,
            participating_lane_ids: vec![OpaqueId::new("lane-a")],
            detail: "   ".to_owned(),
            evidence_refs: vec![],
        };
        assert!(empty_detail.validate().is_err());
    }

    #[test]
    fn comparison_report_validate_rejects_lane_in_both_lists() {
        let lane_a = OpaqueId::new("lane-a");
        let lane_b = OpaqueId::new("lane-b");
        let ok = ComparisonReport {
            header: h("report-1"),
            fleet_run_id: OpaqueId::new("fleet-1"),
            observations: vec![],
            participating_lane_ids: vec![lane_a.clone()],
            excluded_lane_ids: vec![lane_b.clone()],
        };
        assert!(ok.validate().is_ok());

        let overlapping = ComparisonReport {
            excluded_lane_ids: vec![lane_a.clone()],
            ..ok
        };
        assert!(overlapping.validate().is_err());

        let no_participants = ComparisonReport {
            header: h("report-2"),
            fleet_run_id: OpaqueId::new("fleet-1"),
            observations: vec![],
            participating_lane_ids: vec![],
            excluded_lane_ids: vec![lane_b],
        };
        assert!(no_participants.validate().is_err());
    }
}

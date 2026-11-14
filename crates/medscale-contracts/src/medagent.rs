//! MedAgent Workbench contracts (Spec 077).
//!
//! A governed, project-aware agent execution model: an `AgentIdentity`
//! bound to one admitted local model Pack, an explicit `ContextManifest`
//! that is the *only* data a run may read, a persisted/cancellable
//! `AgentRun` state machine, typed/receipted `ToolInvocation`s, and
//! evidence-only `AgentProposal` output. Reuses `OpaqueId`, `ObjectHeader`,
//! `ProjectRevision`/`check_revision`/`initial_revision`,
//! `ArtifactDescriptor`/`ArtifactVersionBinding`. Model output is data,
//! never code: no type here or in the authority layer that consumes it may
//! be used to execute a shell command, SQL fragment, or Rust expression
//! (`security.md` T3). No 077 code path may create a `ClinicalAssertion`
//! or call `PromoteProposal`/`TransitionEffect`/`contracts::actions`
//! (`security.md` T1).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::objects::{ObjectHeader, OpaqueId};
use crate::project_graph::{ArtifactDescriptor, ProjectRevision, check_revision, initial_revision};

/// Durable schema version for every MedAgent object (Spec 077 v1).
pub const MEDAGENT_SCHEMA_VERSION: u32 = 1;

pub const AGENT_DISPLAY_NAME_MAX_CHARS: usize = 128;
pub const PROMPT_MAX_BYTES: usize = 32_768;
pub const TOOL_ARGUMENT_MAX_BYTES: usize = 8_192;
pub const TOOL_RESULT_MAX_BYTES: usize = 32_768;
pub const FAILURE_REASON_MAX_CHARS: usize = 1_024;
pub const CONTEXT_MANIFEST_MAX_ARTIFACTS: usize = 64;
pub const CAPABILITY_MANIFEST_MAX_TOOL_KINDS: usize = 32;

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

fn bounded_optional_text(value: &str, max_chars: usize, what: &str) -> Result<(), String> {
    if value.chars().count() > max_chars {
        return Err(format!("{what} exceeds bound"));
    }
    if value.contains('\0') {
        return Err(format!("{what} must not contain NUL"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AgentIdentity
// ---------------------------------------------------------------------------

/// Lifecycle of an `AgentIdentity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentIdentityStatus {
    Active,
    Revoked,
}

impl AgentIdentityStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            other => Err(format!("unknown agent identity status {other}")),
        }
    }
}

/// A registered execution actor bound to exactly one admitted local model
/// Pack. Carries no clinical/research authority by itself (`RESEARCH_OS_DECISIONS.md`
/// D10: agents are first-class participants, not first-class authorities).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIdentity {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    /// The admitted `PackManifestV0.pack_id` this identity is bound to.
    pub pack_id: OpaqueId,
    /// The exact Pack version captured at registration time. A later
    /// promotion/supersession of the admitted Pack never silently upgrades
    /// an existing identity (`security.md` T5).
    pub pack_version: String,
    pub display_name: String,
    pub status: AgentIdentityStatus,
}

impl AgentIdentity {
    /// Creates a revision-1 active identity after validating metadata.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        pack_id: OpaqueId,
        pack_version: String,
        display_name: String,
    ) -> Result<Self, String> {
        bounded_text(&display_name, AGENT_DISPLAY_NAME_MAX_CHARS, "display_name")?;
        bounded_text(&pack_version, 64, "pack_version")?;
        Ok(Self {
            header,
            revision: initial_revision(),
            project_id,
            pack_id,
            pack_version,
            display_name,
            status: AgentIdentityStatus::Active,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

// ---------------------------------------------------------------------------
// Tool vocabulary + AgentCapabilityManifest
// ---------------------------------------------------------------------------

/// Closed vocabulary of tool kinds an agent run may ever request. Every
/// member is provably boundable to the run's `ContextManifest`; no browser/
/// network/external-provider tool is ever a member of this spec's
/// vocabulary (`security.md` explicit non-capabilities).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Read one artifact named in the run's `ContextManifest`.
    ReadContextArtifact,
    /// Bounded lexical search over the run's `ContextManifest` artifacts only.
    SearchContextArtifacts,
}

impl ToolKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadContextArtifact => "read_context_artifact",
            Self::SearchContextArtifacts => "search_context_artifacts",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "read_context_artifact" => Ok(Self::ReadContextArtifact),
            "search_context_artifacts" => Ok(Self::SearchContextArtifacts),
            other => Err(format!("unknown tool kind {other}")),
        }
    }
}

/// The fixed, immutable-once-set set of tool kinds one `AgentIdentity` may
/// ever request. Widening capabilities requires revoking and re-registering
/// -- an explicit, auditable action, never an in-place grant expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentCapabilityManifest {
    pub agent_identity_id: OpaqueId,
    pub granted_tool_kinds: Vec<ToolKind>,
}

impl AgentCapabilityManifest {
    pub fn new(
        agent_identity_id: OpaqueId,
        granted_tool_kinds: Vec<ToolKind>,
    ) -> Result<Self, String> {
        if granted_tool_kinds.is_empty() {
            return Err("granted_tool_kinds must not be empty".to_owned());
        }
        if granted_tool_kinds.len() > CAPABILITY_MANIFEST_MAX_TOOL_KINDS {
            return Err("granted_tool_kinds exceeds bound".to_owned());
        }
        Ok(Self {
            agent_identity_id,
            granted_tool_kinds,
        })
    }

    /// Whether this manifest grants `kind`.
    #[must_use]
    pub fn grants(&self, kind: ToolKind) -> bool {
        self.granted_tool_kinds.contains(&kind)
    }
}

// ---------------------------------------------------------------------------
// ContextManifest
// ---------------------------------------------------------------------------

/// An explicit, revision-bound set of selected Project artifacts. This is
/// the *only* data an agent run bound to it may read; there is no ambient
/// vault access path for agent-run code (`security.md` T4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextManifest {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub selected_artifacts: Vec<ArtifactDescriptor>,
}

impl ContextManifest {
    /// Creates a revision-1 manifest after validating the artifact list.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        selected_artifacts: Vec<ArtifactDescriptor>,
    ) -> Result<Self, String> {
        if selected_artifacts.is_empty() {
            return Err("selected_artifacts must not be empty".to_owned());
        }
        if selected_artifacts.len() > CONTEXT_MANIFEST_MAX_ARTIFACTS {
            return Err("selected_artifacts exceeds bound".to_owned());
        }
        Ok(Self {
            header,
            revision: initial_revision(),
            project_id,
            selected_artifacts,
        })
    }

    /// Whether `object_id` is named by this manifest (the read-boundary
    /// check every tool dispatch must consult).
    #[must_use]
    pub fn allows(&self, object_id: &OpaqueId) -> bool {
        self.selected_artifacts
            .iter()
            .any(|a| &a.object_id == object_id)
    }
}

// ---------------------------------------------------------------------------
// AgentRun / AgentRunState / AgentTurn
// ---------------------------------------------------------------------------

/// Lifecycle of an `AgentRun`. `Pending -> Running -> {Cancelled, Completed,
/// Failed}`, plus early `Pending -> Cancelled` (cancel before start). No
/// other edge exists; every other transition is a `Conflict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunState {
    Pending,
    Running,
    Cancelled,
    Completed,
    Failed,
}

impl AgentRunState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "cancelled" => Ok(Self::Cancelled),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            other => Err(format!("unknown agent run state {other}")),
        }
    }

    /// True only for the frozen transition table (`contracts.md` section 4).
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Running)
                | (Self::Pending, Self::Cancelled)
                | (Self::Running, Self::Cancelled)
                | (Self::Running, Self::Completed)
                | (Self::Running, Self::Failed)
        )
    }

    /// True for any of the three terminal states.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Completed | Self::Failed)
    }
}

/// A local Project-grounded, cancellable agent execution bound to exactly
/// one `AgentIdentity` and one `ContextManifest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRun {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub agent_identity_id: OpaqueId,
    pub context_manifest_id: OpaqueId,
    pub prompt: String,
    pub status: AgentRunState,
}

impl AgentRun {
    /// Creates a revision-1 `Pending` run after validating the prompt.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        agent_identity_id: OpaqueId,
        context_manifest_id: OpaqueId,
        prompt: String,
    ) -> Result<Self, String> {
        bounded_bytes(&prompt, PROMPT_MAX_BYTES, "prompt")?;
        if prompt.trim().is_empty() {
            return Err("prompt must not be empty".to_owned());
        }
        Ok(Self {
            header,
            revision: initial_revision(),
            project_id,
            agent_identity_id,
            context_manifest_id,
            prompt,
            status: AgentRunState::Pending,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// Validates a proposed state transition against the frozen table.
    pub fn check_transition(&self, next: AgentRunState) -> Result<(), String> {
        if self.status.can_transition_to(next) {
            Ok(())
        } else {
            Err(format!(
                "illegal agent run transition {:?} -> {:?}",
                self.status, next
            ))
        }
    }
}

/// One append-only step within a run (prompt/response/tool-call boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTurnKind {
    PromptSubmitted,
    ToolRequested,
    ToolResult,
    ModelOutput,
}

impl AgentTurnKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PromptSubmitted => "prompt_submitted",
            Self::ToolRequested => "tool_requested",
            Self::ToolResult => "tool_result",
            Self::ModelOutput => "model_output",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "prompt_submitted" => Ok(Self::PromptSubmitted),
            "tool_requested" => Ok(Self::ToolRequested),
            "tool_result" => Ok(Self::ToolResult),
            "model_output" => Ok(Self::ModelOutput),
            other => Err(format!("unknown agent turn kind {other}")),
        }
    }
}

/// One append-only record within a run's `seq`-ordered history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTurn {
    pub header: ObjectHeader,
    pub run_id: OpaqueId,
    pub seq: u64,
    pub kind: AgentTurnKind,
    pub payload: Value,
}

// ---------------------------------------------------------------------------
// Tool invocation + receipt
// ---------------------------------------------------------------------------

/// Outcome of dispatching one `ToolInvocation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInvocationStatus {
    /// Refused before execution (not granted, or outside `ContextManifest`).
    Refused,
    /// Executed by Core; a `ToolReceipt` exists for this invocation.
    Executed,
}

impl ToolInvocationStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Executed => "executed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "refused" => Ok(Self::Refused),
            "executed" => Ok(Self::Executed),
            other => Err(format!("unknown tool invocation status {other}")),
        }
    }
}

/// A typed, policy-checked request to call one tool. Dispatched and
/// executed entirely by Core; the model may only *request* a tool by name
/// and typed arguments, never author the execution itself
/// (`security.md` T2/T3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolInvocation {
    pub header: ObjectHeader,
    pub run_id: OpaqueId,
    pub turn_seq: u64,
    pub seq: u64,
    pub kind: ToolKind,
    pub arguments: Value,
    pub status: ToolInvocationStatus,
    /// Set only when `status == Refused`.
    pub refusal_reason: Option<String>,
}

impl ToolInvocation {
    pub fn validate(&self) -> Result<(), String> {
        let arguments_bytes =
            serde_json::to_vec(&self.arguments).map_err(|e| format!("bad arguments: {e}"))?;
        if arguments_bytes.len() > TOOL_ARGUMENT_MAX_BYTES {
            return Err("tool arguments exceed bound".to_owned());
        }
        if let Some(reason) = &self.refusal_reason {
            bounded_optional_text(reason, FAILURE_REASON_MAX_CHARS, "refusal_reason")?;
        }
        match self.status {
            ToolInvocationStatus::Refused if self.refusal_reason.is_none() => {
                Err("a refused invocation must carry a refusal_reason".to_owned())
            }
            ToolInvocationStatus::Executed if self.refusal_reason.is_some() => {
                Err("an executed invocation must not carry a refusal_reason".to_owned())
            }
            _ => Ok(()),
        }
    }
}

/// The typed result of one `Executed` `ToolInvocation`. 1:1 with its
/// invocation; a `Refused` invocation never has one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolReceipt {
    pub header: ObjectHeader,
    pub invocation_id: OpaqueId,
    pub result: Value,
}

impl ToolReceipt {
    pub fn validate(&self) -> Result<(), String> {
        let result_bytes =
            serde_json::to_vec(&self.result).map_err(|e| format!("bad result: {e}"))?;
        if result_bytes.len() > TOOL_RESULT_MAX_BYTES {
            return Err("tool result exceeds bound".to_owned());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RunReceipt + AgentProposal
// ---------------------------------------------------------------------------

/// Durable, inspectable provenance for one terminated run. Committed
/// atomically with the run's terminal transition; never exists for a
/// non-terminal run, and a terminal run never lacks one (`migration.md`
/// section 5, `security.md` T11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReceipt {
    pub header: ObjectHeader,
    pub run_id: OpaqueId,
    pub pack_id: OpaqueId,
    pub pack_version: String,
    pub context_manifest_id: OpaqueId,
    pub context_manifest_revision: ProjectRevision,
    pub tool_invocation_ids: Vec<OpaqueId>,
    pub final_state: AgentRunState,
    pub failure_reason: Option<String>,
}

impl RunReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if !self.final_state.is_terminal() {
            return Err("final_state must be a terminal AgentRunState".to_owned());
        }
        match (self.final_state, &self.failure_reason) {
            (AgentRunState::Failed, None) => {
                Err("a Failed run's RunReceipt must carry a failure_reason".to_owned())
            }
            (state, Some(_)) if state != AgentRunState::Failed => {
                Err("only a Failed run's RunReceipt may carry a failure_reason".to_owned())
            }
            _ => Ok(()),
        }?;
        if let Some(reason) = &self.failure_reason {
            bounded_optional_text(reason, FAILURE_REASON_MAX_CHARS, "failure_reason")?;
        }
        Ok(())
    }
}

/// A thin, 077-owned linking record: the actual claim content lives in the
/// reused `Proposal` object this run's completion submits via the existing
/// `CreateProposal` capability (`contracts.md` section 6). `AgentProposal`
/// itself carries no clinical/research authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProposal {
    pub header: ObjectHeader,
    pub run_id: OpaqueId,
    pub proposal_id: OpaqueId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_graph::{ArtifactKind, ArtifactVersionBinding};

    fn h(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: MEDAGENT_SCHEMA_VERSION,
            realm_id: crate::objects::RealmId::new("realm-1"),
            authority_scope_id: crate::objects::AuthorityScopeId::new("scope-1"),
        }
    }

    fn descriptor(id: &str) -> ArtifactDescriptor {
        ArtifactDescriptor {
            object_id: OpaqueId::new(id),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        }
    }

    #[test]
    fn agent_identity_status_round_trips() {
        for s in [AgentIdentityStatus::Active, AgentIdentityStatus::Revoked] {
            assert_eq!(AgentIdentityStatus::parse(s.as_str()).unwrap(), s);
        }
        assert!(AgentIdentityStatus::parse("bogus").is_err());
    }

    #[test]
    fn tool_kind_round_trips_and_rejects_unknown() {
        for k in [
            ToolKind::ReadContextArtifact,
            ToolKind::SearchContextArtifacts,
        ] {
            assert_eq!(ToolKind::parse(k.as_str()).unwrap(), k);
        }
        assert!(ToolKind::parse("browse_web").is_err());
    }

    #[test]
    fn agent_run_state_transition_table_is_frozen() {
        use AgentRunState::{Cancelled, Completed, Failed, Pending, Running};
        assert!(Pending.can_transition_to(Running));
        assert!(Pending.can_transition_to(Cancelled));
        assert!(Running.can_transition_to(Cancelled));
        assert!(Running.can_transition_to(Completed));
        assert!(Running.can_transition_to(Failed));
        // No other edge.
        assert!(!Pending.can_transition_to(Completed));
        assert!(!Pending.can_transition_to(Failed));
        assert!(!Completed.can_transition_to(Running));
        assert!(!Cancelled.can_transition_to(Running));
        assert!(!Failed.can_transition_to(Completed));
        assert!(!Running.can_transition_to(Pending));
    }

    #[test]
    fn agent_run_state_terminal_classification() {
        assert!(!AgentRunState::Pending.is_terminal());
        assert!(!AgentRunState::Running.is_terminal());
        assert!(AgentRunState::Cancelled.is_terminal());
        assert!(AgentRunState::Completed.is_terminal());
        assert!(AgentRunState::Failed.is_terminal());
    }

    #[test]
    fn agent_identity_new_validates_display_name() {
        assert!(
            AgentIdentity::new(
                h("agent-1"),
                OpaqueId::new("proj-1"),
                OpaqueId::new("pack-1"),
                "1.0.0".to_owned(),
                "   ".to_owned(),
            )
            .is_err()
        );
        let over_max = "a".repeat(AGENT_DISPLAY_NAME_MAX_CHARS + 1);
        assert!(
            AgentIdentity::new(
                h("agent-1"),
                OpaqueId::new("proj-1"),
                OpaqueId::new("pack-1"),
                "1.0.0".to_owned(),
                over_max,
            )
            .is_err()
        );
        let ok = AgentIdentity::new(
            h("agent-1"),
            OpaqueId::new("proj-1"),
            OpaqueId::new("pack-1"),
            "1.0.0".to_owned(),
            "Research Assistant".to_owned(),
        )
        .unwrap();
        assert_eq!(ok.revision, 1);
        assert_eq!(ok.status, AgentIdentityStatus::Active);
    }

    #[test]
    fn agent_capability_manifest_rejects_empty_grants() {
        assert!(AgentCapabilityManifest::new(OpaqueId::new("agent-1"), vec![]).is_err());
        let manifest = AgentCapabilityManifest::new(
            OpaqueId::new("agent-1"),
            vec![ToolKind::ReadContextArtifact],
        )
        .unwrap();
        assert!(manifest.grants(ToolKind::ReadContextArtifact));
        assert!(!manifest.grants(ToolKind::SearchContextArtifacts));
    }

    #[test]
    fn context_manifest_rejects_empty_and_checks_membership() {
        assert!(ContextManifest::new(h("ctx-1"), OpaqueId::new("proj-1"), vec![]).is_err());
        let manifest = ContextManifest::new(
            h("ctx-1"),
            OpaqueId::new("proj-1"),
            vec![descriptor("art-a"), descriptor("art-b")],
        )
        .unwrap();
        assert!(manifest.allows(&OpaqueId::new("art-a")));
        assert!(!manifest.allows(&OpaqueId::new("art-c")));
    }

    #[test]
    fn agent_run_new_rejects_empty_and_oversized_prompt() {
        assert!(
            AgentRun::new(
                h("run-1"),
                OpaqueId::new("proj-1"),
                OpaqueId::new("agent-1"),
                OpaqueId::new("ctx-1"),
                "   ".to_owned(),
            )
            .is_err()
        );
        let over_max = "a".repeat(PROMPT_MAX_BYTES + 1);
        assert!(
            AgentRun::new(
                h("run-1"),
                OpaqueId::new("proj-1"),
                OpaqueId::new("agent-1"),
                OpaqueId::new("ctx-1"),
                over_max,
            )
            .is_err()
        );
        let run = AgentRun::new(
            h("run-1"),
            OpaqueId::new("proj-1"),
            OpaqueId::new("agent-1"),
            OpaqueId::new("ctx-1"),
            "summarize the selected documents".to_owned(),
        )
        .unwrap();
        assert_eq!(run.status, AgentRunState::Pending);
        assert!(run.check_transition(AgentRunState::Running).is_ok());
        assert!(run.check_transition(AgentRunState::Completed).is_err());
    }

    #[test]
    fn tool_invocation_validate_enforces_refusal_reason_pairing() {
        let refused_without_reason = ToolInvocation {
            header: h("inv-1"),
            run_id: OpaqueId::new("run-1"),
            turn_seq: 1,
            seq: 1,
            kind: ToolKind::ReadContextArtifact,
            arguments: serde_json::json!({}),
            status: ToolInvocationStatus::Refused,
            refusal_reason: None,
        };
        assert!(refused_without_reason.validate().is_err());

        let executed_with_reason = ToolInvocation {
            status: ToolInvocationStatus::Executed,
            refusal_reason: Some("x".to_owned()),
            ..refused_without_reason.clone()
        };
        assert!(executed_with_reason.validate().is_err());

        let refused_ok = ToolInvocation {
            refusal_reason: Some("tool kind not granted".to_owned()),
            ..refused_without_reason
        };
        assert!(refused_ok.validate().is_ok());
    }

    #[test]
    fn run_receipt_validate_requires_terminal_state_and_failure_reason_pairing() {
        let base = RunReceipt {
            header: h("receipt-1"),
            run_id: OpaqueId::new("run-1"),
            pack_id: OpaqueId::new("pack-1"),
            pack_version: "1.0.0".to_owned(),
            context_manifest_id: OpaqueId::new("ctx-1"),
            context_manifest_revision: 1,
            tool_invocation_ids: vec![],
            final_state: AgentRunState::Pending,
            failure_reason: None,
        };
        // Non-terminal final_state is rejected.
        assert!(base.validate().is_err());

        let completed = RunReceipt {
            final_state: AgentRunState::Completed,
            ..base.clone()
        };
        assert!(completed.validate().is_ok());

        let completed_with_reason = RunReceipt {
            final_state: AgentRunState::Completed,
            failure_reason: Some("should not be set".to_owned()),
            ..base.clone()
        };
        assert!(completed_with_reason.validate().is_err());

        let failed_without_reason = RunReceipt {
            final_state: AgentRunState::Failed,
            ..base
        };
        assert!(failed_without_reason.validate().is_err());

        let failed_ok = RunReceipt {
            final_state: AgentRunState::Failed,
            failure_reason: Some("model execution failed".to_owned()),
            ..failed_without_reason
        };
        assert!(failed_ok.validate().is_ok());
    }
}

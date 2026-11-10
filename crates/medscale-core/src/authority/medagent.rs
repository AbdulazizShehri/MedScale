//! MedAgent Workbench Core authority paths (Spec 077).
//!
//! T077-03 through T077-06 scope: `AgentIdentity` + `AgentCapabilityManifest`
//! register/get/list/revoke, `ContextManifest` create/get plus the single
//! read-boundary check (`require_artifact_in_context`), the full
//! `AgentRun`/`AgentTurn` lifecycle, and typed `ToolInvocation` dispatch
//! (`invoke_tool`). `AgentProposal`/Model-Pack Core paths land in T077-07
//! and T077-08.
//!
//! `AgentTurn`s are never externally appendable: `PromptSubmitted` is
//! appended automatically, once, by `start_agent_run` (the run's own
//! `prompt` field is already known at creation, so no caller input is
//! trusted); `invoke_tool` likewise appends its own `ToolRequested`/
//! `ToolResult` turns as side effects, never a directly callable "append
//! arbitrary turn" capability -- that would let an external caller
//! fabricate conversation history. `ModelOutput` turns (T077-07) will be
//! appended the same way.
//!
//! `invoke_tool` is the sole tool-dispatch path and the first production
//! caller of `require_artifact_in_context` (added at T077-04): every
//! `ReadContextArtifact`/`SearchContextArtifacts` execution consults it
//! before touching any artifact content (`security.md` T4). A tool kind
//! not present in the run's bound `AgentCapabilityManifest`, an argument
//! shape that fails typed parsing, or an artifact outside the bound
//! `ContextManifest` are all recorded as a `Refused` `ToolInvocation` with
//! a reason -- never a silent drop and never partial execution on a
//! malformed argument.
//!
//! `AgentIdentity.status`/admitted-Pack state is re-checked on every
//! `create_agent_run` and `start_agent_run` call, never cached across a
//! run's lifetime (`security.md` T5): a revoked identity or an
//! un-admitted Pack refuses both starting a new run and advancing an
//! existing `Pending` one.
//!
//! `pack_version` is never client-supplied: it is captured here from the
//! currently admitted `PackManifestV0` for the given `pack_id`, so a caller
//! cannot register an identity against a version string that does not
//! match what is actually admitted (`security.md` T5). Registration against
//! a `pack_id` the local `PackStore` does not recognize fails closed.
//!
//! `ContextManifest.selected_artifacts` are validated for shape only at
//! write time; their live resolution is always recomputed at read time
//! through `resolve_context_artifact` (`migration.md` section 4), never
//! cached -- an artifact deleted or superseded after a `ContextManifest`
//! was created shows up as `Missing`/`Stale` on the next read, it does not
//! silently keep resolving `Current`.
//!
//! Audit split (mirrors `collaboration.rs`'s own T076-03 precedent):
//! `AgentIdentity`/`ContextManifest` are scope-level, not run-level, and
//! have no run to chain an activity record into, so their mutations reuse
//! the existing in-memory `ActionAuditRecord` trail rather than a
//! dedicated hash chain.

use medscale_clinical::summary_empty_evidence::{
    Approval, SUMMARY_EMPTY_EVIDENCE_REFUSAL_CODE, require_approved_evidence,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::medagent::{
    AgentCapabilityManifest, AgentIdentity, AgentIdentityStatus, AgentProposal, AgentRun,
    AgentRunState, AgentTurn, AgentTurnKind, ContextManifest, MEDAGENT_SCHEMA_VERSION, RunReceipt,
    TOOL_ARGUMENT_MAX_BYTES, TOOL_RESULT_MAX_BYTES, ToolInvocation, ToolKind, ToolReceipt,
};
use medscale_contracts::objects::{
    AuthorityScopeId, ObjectHeader, OpaqueId, ProducerKind, Proposal, RealmId, VaultId,
};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, ReferenceResolution,
};
use medscale_storage::{MetaError, SqliteMetaStore};
use serde::Deserialize;

use super::store::{InMemoryAuthorityStore, ScopeError, StoredObject};
use crate::process::{LeaseRegistry, SessionRegistry};

/// Typed, `deny_unknown_fields` argument shapes per `ToolKind`
/// (`security.md` T3: arguments are deserialized into closed typed structs
/// before any use, never a raw `Value` passthrough). Kept Core-internal:
/// the wire envelope still carries `arguments: Value` (mirroring
/// `ToolInvocation.arguments`'s own frozen shape), but `invoke_tool` never
/// touches that `Value` before parsing it into one of these.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadContextArtifactArgs {
    object_id: OpaqueId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchContextArtifactsArgs {
    query: String,
}

/// Nearest byte index `<= idx` that lands on a UTF-8 char boundary in `s`
/// (stable-Rust equivalent of the unstable `str::floor_char_boundary`).
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    let mut idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Nearest byte index `>= idx` that lands on a UTF-8 char boundary in `s`.
fn ceil_char_boundary(s: &str, idx: usize) -> usize {
    let mut idx = idx.min(s.len());
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

const SEARCH_QUERY_MAX_CHARS: usize = 256;
const SEARCH_MAX_MATCHES: usize = 20;
const SEARCH_SNIPPET_RADIUS_BYTES: usize = 80;
/// Conservative cap on raw content bytes copied into a tool result,
/// leaving ample room under `TOOL_RESULT_MAX_BYTES` for JSON structure and
/// UTF-8 lossy-conversion replacement-character growth.
const READ_CONTENT_MAX_BYTES: usize = TOOL_RESULT_MAX_BYTES / 2;
/// Per-artifact byte cap `execute_search_context_artifacts` will actually
/// scan. `SourceRecord`/`DerivedSourceArtifact.bytes` carries no size
/// bound of its own (Spec 002/074, out of this spec's authority to add
/// one), so without this cap a `ContextManifest` naming up to
/// `CONTEXT_MANIFEST_MAX_ARTIFACTS` large artifacts would make every
/// search call do unbounded lowercase-copy-and-scan work per artifact --
/// only the *output* (match count, snippet radius) was bounded before
/// this cap existed, never the input work. 256 KiB keeps a single search
/// call's per-artifact cost small and constant regardless of how large
/// the underlying canonical object actually is; a search that needs to
/// see further into a large artifact should narrow via `ReadContextArtifact`
/// on a specific object instead.
const SEARCH_ARTIFACT_SCAN_MAX_BYTES: usize = 256 * 1024;

/// Mirrors `project_graph::stored_class_matches` (deliberately duplicated,
/// not imported, per this spec's own module-independence convention -- see
/// `crates/medscale-storage/src/medagent.rs`'s header comment).
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

fn header(realm: &RealmId, scope: &AuthorityScopeId, id: OpaqueId) -> ObjectHeader {
    ObjectHeader {
        id,
        schema_version: MEDAGENT_SCHEMA_VERSION,
        realm_id: realm.clone(),
        authority_scope_id: scope.clone(),
    }
}

/// Authenticated authority view for one request.
pub struct MedAgent<'a> {
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

impl MedAgent<'_> {
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

    /// Appends one scope-level audit row to the existing trail (identity
    /// lifecycle only; see module docs).
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

    fn scoped_agent_identity(&self, id: &OpaqueId) -> Result<AgentIdentity, AuthorityError> {
        let identity = self.meta.get_agent_identity(id).map_err(meta_err)?;
        if identity.header.realm_id != self.realm
            || identity.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(identity)
    }

    fn scoped_project(&self, id: &OpaqueId) -> Result<(), AuthorityError> {
        let project = self.meta.get_project(id).map_err(meta_err)?;
        if project.header.realm_id != self.realm || project.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(())
    }

    // ----- AgentIdentity + AgentCapabilityManifest -----

    /// Registers a new `AgentIdentity` bound to `pack_id`. The identity's
    /// `pack_version` is captured from the currently admitted
    /// `PackManifestV0`, never from caller input; a `pack_id` the local
    /// `PackStore` does not recognize fails closed.
    pub fn register_agent_identity(
        &mut self,
        project_id: OpaqueId,
        pack_id: OpaqueId,
        display_name: String,
        granted_tool_kinds: Vec<ToolKind>,
    ) -> Result<(AgentIdentity, AgentCapabilityManifest), AuthorityError> {
        self.scoped_project(&project_id)?;
        let manifest = self
            .packs
            .get(&pack_id)
            .ok_or_else(|| AuthorityError::InvalidArgument {
                message: format!("pack {} is not admitted", pack_id.as_str()),
            })?;
        let id = self
            .meta
            .alloc_medagent_id("medagent-identity-seq", "agent")
            .map_err(meta_err)?;
        let identity = AgentIdentity::new(
            header(&self.realm, &self.scope, id.clone()),
            project_id,
            pack_id,
            manifest.version,
            display_name,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let capabilities = AgentCapabilityManifest::new(id.clone(), granted_tool_kinds)
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta
            .insert_agent_identity_with_capabilities(&identity, &capabilities)
            .map_err(meta_err)?;
        self.audit("medagent_identity.register", vec![id])?;
        Ok((identity, capabilities))
    }

    /// Scoped identity + capability-manifest read.
    pub fn get_agent_identity(
        &self,
        id: &OpaqueId,
    ) -> Result<(AgentIdentity, AgentCapabilityManifest), AuthorityError> {
        let identity = self.scoped_agent_identity(id)?;
        let capabilities = self.meta.get_capability_manifest(id).map_err(meta_err)?;
        Ok((identity, capabilities))
    }

    /// Lists agent identities in one Project. The Project itself is
    /// scope-checked before the storage read, so no cross-scope row can be
    /// named by a valid `project_id`.
    pub fn list_agent_identities(
        &self,
        project_id: &OpaqueId,
        limit: u32,
    ) -> Result<Vec<AgentIdentity>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta
            .list_agent_identities(project_id, limit)
            .map_err(meta_err)
    }

    /// Revokes an agent identity (status only; `pack_id`/`pack_version`
    /// never change once registered).
    pub fn revoke_agent_identity(
        &mut self,
        id: &OpaqueId,
        expected: u64,
    ) -> Result<(AgentIdentity, AgentCapabilityManifest), AuthorityError> {
        let current = self.scoped_agent_identity(id)?;
        if current.status != AgentIdentityStatus::Active {
            return Err(AuthorityError::Conflict {
                message: "agent identity is not active".to_owned(),
            });
        }
        let revoked = self
            .meta
            .revoke_agent_identity(id, expected)
            .map_err(meta_err)?;
        let capabilities = self.meta.get_capability_manifest(id).map_err(meta_err)?;
        self.audit("medagent_identity.revoke", vec![id.clone()])?;
        Ok((revoked, capabilities))
    }

    // ----- ContextManifest -----

    fn scoped_context_manifest(&self, id: &OpaqueId) -> Result<ContextManifest, AuthorityError> {
        let manifest = self.meta.get_context_manifest(id).map_err(meta_err)?;
        if manifest.header.realm_id != self.realm
            || manifest.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(manifest)
    }

    /// Resolves one artifact descriptor against canonical owners in this
    /// realm/scope (mirrors `project_graph::ProjectGraph::resolve_descriptor`
    /// and `collaboration::Collab`'s own duplicated `resolve_anchor` --
    /// deliberately not imported, per this spec's module-independence
    /// convention). Always live; never cached (`migration.md` section 4).
    fn resolve_context_artifact(&self, descriptor: &ArtifactDescriptor) -> ReferenceResolution {
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
                match &descriptor.binding {
                    ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                    _ => ReferenceResolution::Stale,
                }
            }
        }
    }

    /// Creates a new `ContextManifest`. Artifacts are validated for shape
    /// only (`ArtifactDescriptor::validate`); existence/staleness is never
    /// checked here -- it is always recomputed live on read
    /// (`migration.md` section 4), so a `ContextManifest` may legitimately
    /// name an artifact that does not exist yet or has since gone stale.
    pub fn create_context_manifest(
        &mut self,
        project_id: OpaqueId,
        selected_artifacts: Vec<ArtifactDescriptor>,
    ) -> Result<ContextManifest, AuthorityError> {
        self.scoped_project(&project_id)?;
        for artifact in &selected_artifacts {
            artifact
                .validate()
                .map_err(|message| AuthorityError::InvalidArgument { message })?;
        }
        let id = self
            .meta
            .alloc_medagent_id("medagent-context-seq", "context")
            .map_err(meta_err)?;
        let manifest = ContextManifest::new(
            header(&self.realm, &self.scope, id.clone()),
            project_id,
            selected_artifacts,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta
            .insert_context_manifest(&manifest)
            .map_err(meta_err)?;
        self.audit("medagent_context_manifest.create", vec![id])?;
        Ok(manifest)
    }

    /// Scoped context-manifest read, with every selected artifact's
    /// `ReferenceResolution` recomputed live (never cached) alongside it, in
    /// the same order as `selected_artifacts`.
    pub fn get_context_manifest(
        &self,
        id: &OpaqueId,
    ) -> Result<(ContextManifest, Vec<ReferenceResolution>), AuthorityError> {
        let manifest = self.scoped_context_manifest(id)?;
        let resolutions = manifest
            .selected_artifacts
            .iter()
            .map(|artifact| self.resolve_context_artifact(artifact))
            .collect();
        Ok((manifest, resolutions))
    }

    /// THE single Core-internal check for whether `object_id` is readable
    /// under `context_id`'s bound `ContextManifest` (`security.md` T4).
    /// Every tool-dispatch path must call this before resolving any
    /// artifact content; there is no second, unchecked read path for
    /// agent-run code. Returns the manifest itself on success so the
    /// caller never needs a second, separately-scoped fetch.
    ///
    /// First (and, as of T077-06, only) production caller:
    /// `execute_read_context_artifact`.
    pub fn require_artifact_in_context(
        &self,
        context_id: &OpaqueId,
        object_id: &OpaqueId,
    ) -> Result<ContextManifest, AuthorityError> {
        let context = self.scoped_context_manifest(context_id)?;
        if !context.allows(object_id) {
            return Err(AuthorityError::Unauthorized);
        }
        Ok(context)
    }

    // ----- AgentRun + AgentTurn -----

    fn scoped_agent_run(&self, id: &OpaqueId) -> Result<AgentRun, AuthorityError> {
        let run = self.meta.get_agent_run(id).map_err(meta_err)?;
        if run.header.realm_id != self.realm || run.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(run)
    }

    /// Re-checks `agent_id`'s identity is `Active` and its captured
    /// `pack_version` still matches the currently admitted Pack -- never
    /// cached across a run's lifetime (`security.md` T5). Called by both
    /// `create_agent_run` and `start_agent_run`.
    fn require_active_identity(
        &self,
        agent_id: &OpaqueId,
    ) -> Result<AgentIdentity, AuthorityError> {
        let identity = self.scoped_agent_identity(agent_id)?;
        if identity.status != AgentIdentityStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        let admitted = self
            .packs
            .get(&identity.pack_id)
            .ok_or(AuthorityError::Unauthorized)?;
        if admitted.version != identity.pack_version {
            return Err(AuthorityError::Unauthorized);
        }
        Ok(identity)
    }

    /// Creates a new `Pending` `AgentRun`. `agent_identity_id` and
    /// `context_manifest_id` must both already belong to `project_id`
    /// (never merely to the caller's realm/scope) -- an identity or
    /// context from a different Project is refused, not silently
    /// cross-wired.
    pub fn create_agent_run(
        &mut self,
        project_id: OpaqueId,
        agent_identity_id: OpaqueId,
        context_manifest_id: OpaqueId,
        prompt: String,
    ) -> Result<AgentRun, AuthorityError> {
        self.scoped_project(&project_id)?;
        let identity = self.require_active_identity(&agent_identity_id)?;
        if identity.project_id != project_id {
            return Err(AuthorityError::InvalidArgument {
                message: "agent identity does not belong to this project".to_owned(),
            });
        }
        let context = self.scoped_context_manifest(&context_manifest_id)?;
        if context.project_id != project_id {
            return Err(AuthorityError::InvalidArgument {
                message: "context manifest does not belong to this project".to_owned(),
            });
        }
        let id = self
            .meta
            .alloc_medagent_id("medagent-run-seq", "run")
            .map_err(meta_err)?;
        let run = AgentRun::new(
            header(&self.realm, &self.scope, id.clone()),
            project_id,
            agent_identity_id,
            context_manifest_id,
            prompt,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta.insert_agent_run(&run).map_err(meta_err)?;
        self.audit("medagent_run.create", vec![id])?;
        Ok(run)
    }

    /// Scoped run read.
    pub fn get_agent_run(&self, id: &OpaqueId) -> Result<AgentRun, AuthorityError> {
        self.scoped_agent_run(id)
    }

    /// Lists runs in one Project, optionally filtered by agent identity.
    /// Bounded, filterable run-history read: by Project (always), and
    /// optionally by agent identity and/or `AgentRunState` (T077-08).
    pub fn list_agent_runs(
        &self,
        project_id: &OpaqueId,
        agent_id: Option<&OpaqueId>,
        status: Option<AgentRunState>,
        limit: u32,
    ) -> Result<Vec<AgentRun>, AuthorityError> {
        self.scoped_project(project_id)?;
        self.meta
            .list_agent_runs(project_id, agent_id, status, limit)
            .map_err(meta_err)
    }

    /// Transitions a `Pending` run to `Running` and appends its initial
    /// `PromptSubmitted` turn in the same call (the prompt is already
    /// known from the run's own record; no caller input is trusted for
    /// turn content). Re-checks the bound identity is still `Active` and
    /// its Pack still admitted (`security.md` T5) before starting.
    pub fn start_agent_run(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<(AgentRun, AgentTurn), AuthorityError> {
        let current = self.scoped_agent_run(id)?;
        current
            .check_transition(AgentRunState::Running)
            .map_err(|message| AuthorityError::Conflict { message })?;
        self.require_active_identity(&current.agent_identity_id)?;
        let updated = self
            .meta
            .set_agent_run_running(id, expected_revision)
            .map_err(meta_err)?;
        let turn_id = self
            .meta
            .alloc_medagent_id("medagent-turn-id-seq", "turn")
            .map_err(meta_err)?;
        let turn = self
            .meta
            .insert_agent_turn(
                header(&self.realm, &self.scope, turn_id),
                id,
                AgentTurnKind::PromptSubmitted,
                &serde_json::json!({ "prompt": updated.prompt }),
            )
            .map_err(meta_err)?;
        self.audit("medagent_run.start", vec![id.clone()])?;
        Ok((updated, turn))
    }

    /// Commits any terminal transition (`Cancelled`/`Completed`/`Failed`)
    /// with a `RunReceipt` threading the run's *real* tool-invocation
    /// history (in `seq` order, executed and refused alike -- "exact
    /// provenance" per T077-08's acceptance bullet means the full record,
    /// not merely the successes). Shared by `cancel_agent_run`,
    /// `complete_agent_run`, and `fail_agent_run` so none of them can
    /// drift into recording a different, weaker receipt than the others.
    fn commit_terminal_run(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
        final_state: AgentRunState,
        failure_reason: Option<String>,
    ) -> Result<(AgentRun, RunReceipt), AuthorityError> {
        let current = self.scoped_agent_run(id)?;
        current
            .check_transition(final_state)
            .map_err(|message| AuthorityError::Conflict { message })?;
        let identity = self.scoped_agent_identity(&current.agent_identity_id)?;
        let context = self.scoped_context_manifest(&current.context_manifest_id)?;
        let tool_invocation_ids: Vec<OpaqueId> = self
            .meta
            .list_tool_invocations(id)
            .map_err(meta_err)?
            .into_iter()
            .map(|invocation| invocation.header.id)
            .collect();
        let receipt_id = self
            .meta
            .alloc_medagent_id("medagent-receipt-id-seq", "receipt")
            .map_err(meta_err)?;
        let receipt = RunReceipt {
            header: header(&self.realm, &self.scope, receipt_id),
            run_id: id.clone(),
            pack_id: identity.pack_id,
            pack_version: identity.pack_version,
            context_manifest_id: current.context_manifest_id,
            context_manifest_revision: context.revision,
            tool_invocation_ids,
            final_state,
            failure_reason,
        };
        receipt
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let (updated_run, stored_receipt) = self
            .meta
            .commit_terminal_transition_with_receipt(id, expected_revision, final_state, &receipt)
            .map_err(meta_err)?;
        self.audit(
            &format!("medagent_run.{}", final_state.as_str()),
            vec![id.clone()],
        )?;
        Ok((updated_run, stored_receipt))
    }

    /// Cancels a `Pending` or `Running` run.
    pub fn cancel_agent_run(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<(AgentRun, RunReceipt), AuthorityError> {
        self.commit_terminal_run(id, expected_revision, AgentRunState::Cancelled, None)
    }

    /// Marks a `Running` run `Completed`. Callers decide when a run's work
    /// is done (e.g. after `execute_agent_run` produced a satisfactory
    /// `AgentProposal`) -- a run may have multiple `ModelOutput`/tool turns
    /// before that decision, so completion is never automatic.
    pub fn complete_agent_run(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<(AgentRun, RunReceipt), AuthorityError> {
        self.commit_terminal_run(id, expected_revision, AgentRunState::Completed, None)
    }

    /// Marks a `Running` run `Failed` with a bounded, human-readable
    /// reason.
    pub fn fail_agent_run(
        &mut self,
        id: &OpaqueId,
        expected_revision: u64,
        failure_reason: String,
    ) -> Result<(AgentRun, RunReceipt), AuthorityError> {
        self.commit_terminal_run(
            id,
            expected_revision,
            AgentRunState::Failed,
            Some(failure_reason),
        )
    }

    /// Lists a run's turns in `seq` order.
    pub fn list_agent_turns(&self, run_id: &OpaqueId) -> Result<Vec<AgentTurn>, AuthorityError> {
        self.scoped_agent_run(run_id)?;
        self.meta.list_agent_turns(run_id).map_err(meta_err)
    }

    // ----- Tool invocation -----

    /// Fetches one artifact's raw content bytes, only for the object kinds
    /// this spec's tool vocabulary supports (`SourceRecord`/
    /// `DerivedSourceArtifact`). Returns `Err` with a human-readable reason
    /// on any failure; the caller always turns that into a `Refused`
    /// `ToolInvocation`, never a hard `AuthorityError`.
    fn fetch_artifact_bytes(&self, object_id: &OpaqueId) -> Result<Vec<u8>, String> {
        let stored = self
            .store
            .get_scoped(object_id, &self.realm, &self.scope)
            .map_err(|_| "artifact not found".to_owned())?;
        match stored {
            StoredObject::Source(record) => Ok(record.bytes.clone()),
            StoredObject::Derived(artifact) => Ok(artifact.bytes.clone()),
            _ => Err("artifact kind does not carry readable content".to_owned()),
        }
    }

    /// Dispatches one typed tool invocation for a `Running` run, appending
    /// its `ToolRequested` and `ToolResult` turns as side effects. THE
    /// only path from an agent run to artifact content: every branch that
    /// touches storage first calls `require_artifact_in_context`.
    pub fn invoke_tool(
        &mut self,
        run_id: &OpaqueId,
        kind: ToolKind,
        arguments: serde_json::Value,
    ) -> Result<(ToolInvocation, Option<ToolReceipt>), AuthorityError> {
        let run = self.scoped_agent_run(run_id)?;
        if run.status != AgentRunState::Running {
            return Err(AuthorityError::Conflict {
                message: "run is not Running".to_owned(),
            });
        }
        self.require_active_identity(&run.agent_identity_id)?;
        let capabilities = self
            .meta
            .get_capability_manifest(&run.agent_identity_id)
            .map_err(meta_err)?;

        let arguments_size = serde_json::to_vec(&arguments)
            .map(|b| b.len())
            .unwrap_or(usize::MAX);

        let requested_turn = self.append_turn(
            run_id,
            AgentTurnKind::ToolRequested,
            serde_json::json!({ "kind": kind.as_str(), "arguments": arguments }),
        )?;
        let turn_seq = requested_turn.seq;

        let refusal_reason: Option<String> = if arguments_size > TOOL_ARGUMENT_MAX_BYTES {
            Some("tool arguments exceed bound".to_owned())
        } else if !capabilities.grants(kind) {
            Some(format!("tool kind {} is not granted", kind.as_str()))
        } else {
            None
        };

        if let Some(reason) = refusal_reason {
            let invocation_header =
                self.next_medagent_header("medagent-invocation-id-seq", "inv")?;
            let invocation = self
                .meta
                .insert_refused_tool_invocation(
                    invocation_header,
                    run_id,
                    turn_seq,
                    kind,
                    &arguments,
                    &reason,
                )
                .map_err(meta_err)?;
            self.append_turn(
                run_id,
                AgentTurnKind::ToolResult,
                serde_json::json!({ "status": "refused", "reason": reason }),
            )?;
            self.audit("medagent_tool.refuse", vec![run_id.clone()])?;
            return Ok((invocation, None));
        }

        let execution = match kind {
            ToolKind::ReadContextArtifact => self.execute_read_context_artifact(&run, &arguments),
            ToolKind::SearchContextArtifacts => {
                self.execute_search_context_artifacts(&run, &arguments)
            }
        };
        let result = match execution {
            Ok(result) => result,
            Err(reason) => {
                let invocation_header =
                    self.next_medagent_header("medagent-invocation-id-seq", "inv")?;
                let invocation = self
                    .meta
                    .insert_refused_tool_invocation(
                        invocation_header,
                        run_id,
                        turn_seq,
                        kind,
                        &arguments,
                        &reason,
                    )
                    .map_err(meta_err)?;
                self.append_turn(
                    run_id,
                    AgentTurnKind::ToolResult,
                    serde_json::json!({ "status": "refused", "reason": reason }),
                )?;
                self.audit("medagent_tool.refuse", vec![run_id.clone()])?;
                return Ok((invocation, None));
            }
        };

        let invocation_header = self.next_medagent_header("medagent-invocation-id-seq", "inv")?;
        let receipt_header =
            self.next_medagent_header("medagent-tool-receipt-id-seq", "tool-receipt")?;
        let (invocation, receipt) = self
            .meta
            .insert_executed_tool_invocation_with_receipt(
                invocation_header,
                receipt_header,
                run_id,
                turn_seq,
                kind,
                &arguments,
                &result,
            )
            .map_err(meta_err)?;
        self.append_turn(
            run_id,
            AgentTurnKind::ToolResult,
            serde_json::json!({ "status": "executed", "result": result }),
        )?;
        self.audit("medagent_tool.execute", vec![run_id.clone()])?;
        Ok((invocation, Some(receipt)))
    }

    fn append_turn(
        &mut self,
        run_id: &OpaqueId,
        kind: AgentTurnKind,
        payload: serde_json::Value,
    ) -> Result<AgentTurn, AuthorityError> {
        let header = self.next_medagent_header("medagent-turn-id-seq", "turn")?;
        self.meta
            .insert_agent_turn(header, run_id, kind, &payload)
            .map_err(meta_err)
    }

    fn next_medagent_header(
        &self,
        seq_key: &str,
        prefix: &str,
    ) -> Result<ObjectHeader, AuthorityError> {
        let id = self
            .meta
            .alloc_medagent_id(seq_key, prefix)
            .map_err(meta_err)?;
        Ok(header(&self.realm, &self.scope, id))
    }

    /// Executes `ReadContextArtifact`: parses typed arguments, enforces the
    /// context boundary via `require_artifact_in_context`, and returns the
    /// artifact's content bytes (bounded, lossily UTF-8 decoded).
    fn execute_read_context_artifact(
        &self,
        run: &AgentRun,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let args: ReadContextArtifactArgs = serde_json::from_value(arguments.clone())
            .map_err(|e| format!("malformed arguments: {e}"))?;
        self.require_artifact_in_context(&run.context_manifest_id, &args.object_id)
            .map_err(|_| "artifact is outside the bound ContextManifest".to_owned())?;
        let bytes = self.fetch_artifact_bytes(&args.object_id)?;
        let truncated = bytes.len() > READ_CONTENT_MAX_BYTES;
        let content = String::from_utf8_lossy(&bytes[..bytes.len().min(READ_CONTENT_MAX_BYTES)]);
        Ok(serde_json::json!({
            "object_id": args.object_id.as_str(),
            "content": content,
            "truncated": truncated,
        }))
    }

    /// Executes `SearchContextArtifacts`: a bounded, case-insensitive
    /// substring search over only the run's bound `ContextManifest`
    /// artifacts -- never a broader vault query.
    fn execute_search_context_artifacts(
        &self,
        run: &AgentRun,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let args: SearchContextArtifactsArgs = serde_json::from_value(arguments.clone())
            .map_err(|e| format!("malformed arguments: {e}"))?;
        if args.query.trim().is_empty() || args.query.chars().count() > SEARCH_QUERY_MAX_CHARS {
            return Err("query is empty or exceeds bound".to_owned());
        }
        let context = self
            .scoped_context_manifest(&run.context_manifest_id)
            .map_err(|_| "context manifest is unavailable".to_owned())?;
        let query_lower = args.query.to_lowercase();
        let mut matches = Vec::new();
        for artifact in &context.selected_artifacts {
            if matches.len() >= SEARCH_MAX_MATCHES {
                break;
            }
            let Ok(bytes) = self.fetch_artifact_bytes(&artifact.object_id) else {
                continue;
            };
            let scan_len = bytes.len().min(SEARCH_ARTIFACT_SCAN_MAX_BYTES);
            let text = String::from_utf8_lossy(&bytes[..scan_len]);
            let text_lower = text.to_lowercase();
            if let Some(pos) = text_lower.find(&query_lower) {
                // Slice text_lower (not the original text): lowercasing can
                // change a character's UTF-8 byte length, so byte offsets
                // found in text_lower are only guaranteed valid against
                // text_lower itself. Both ends are snapped to a char
                // boundary -- pos and pos + query_lower.len() already are
                // (str::find/len respect char boundaries), but the radius
                // arithmetic can land mid-character.
                let start = floor_char_boundary(
                    &text_lower,
                    pos.saturating_sub(SEARCH_SNIPPET_RADIUS_BYTES),
                );
                let end = ceil_char_boundary(
                    &text_lower,
                    (pos + query_lower.len() + SEARCH_SNIPPET_RADIUS_BYTES).min(text_lower.len()),
                );
                matches.push(serde_json::json!({
                    "object_id": artifact.object_id.as_str(),
                    "snippet": text_lower[start..end].to_owned(),
                }));
            }
        }
        Ok(serde_json::json!({
            "query": args.query,
            "matches": matches,
        }))
    }

    // ----- Model Pack lane + AgentProposal -----

    /// Runs a `Running` run's `prompt` through the exact admitted local
    /// model Pack its `AgentIdentity` is bound to (zero network), appends
    /// the `ModelOutput` turn, and persists the result as an
    /// `AgentProposal` linking to a real `Proposal` row with
    /// `producer: ProducerKind::Agent(agent_identity_id)`.
    ///
    /// `local_path` is caller-supplied on every call, exactly like
    /// `RequestBody::PacksEvaluateLocal` -- `PackManifestV0` is purely
    /// content-addressed and never stores an on-disk path, so there is no
    /// path to cache here either. This method never keeps a prepared
    /// model session across calls (`OnnxTokenClassifierRuntime::run`'s own
    /// "convenience one-shot path" -- a run's model execution happens once,
    /// not in a hot loop, so the `CoreFacade`-level prepared-model cache
    /// `PacksEvaluateLocal` uses is a deliberately out-of-scope
    /// optimization here, not an oversight).
    pub fn execute_agent_run(
        &mut self,
        run_id: &OpaqueId,
        local_path: &str,
        max_tokens: usize,
        synthetic_only: bool,
    ) -> Result<(AgentTurn, AgentProposal), AuthorityError> {
        // Mirrors PacksEvaluateLocal's exact same gate (facade.rs): real
        // PHI flowing through a local model runtime requires a later,
        // explicit authority this spec does not grant. MedAgent runs are
        // Project-grounded synthetic/test data only until that exists.
        if !synthetic_only {
            return Err(AuthorityError::ExternalGateRequired {
                gate: "REAL_PHI_MODEL_RUNTIME".to_owned(),
            });
        }
        let run = self.scoped_agent_run(run_id)?;
        if run.status != AgentRunState::Running {
            return Err(AuthorityError::Conflict {
                message: "run is not Running".to_owned(),
            });
        }
        let identity = self.require_active_identity(&run.agent_identity_id)?;

        // Clinical empty-evidence gate: generation runs only on context that
        // still resolves to its current version now. Artifacts that went
        // missing, stale or out of scope since the manifest was made are not
        // cited, and with none left the model is not run at all.
        let context = self.scoped_context_manifest(&run.context_manifest_id)?;
        let candidates = context
            .selected_artifacts
            .iter()
            .map(|artifact| {
                let approval = match self.resolve_context_artifact(artifact) {
                    ReferenceResolution::Current => Approval::Approved,
                    ReferenceResolution::Stale
                    | ReferenceResolution::Missing
                    | ReferenceResolution::Denied => Approval::Unapproved,
                    ReferenceResolution::UnsupportedKind | ReferenceResolution::Corrupt => {
                        Approval::Invalid
                    }
                };
                (artifact.object_id.clone(), approval)
            })
            .collect();
        let evidence_refs = require_approved_evidence(candidates).map_err(|refusal| {
            AuthorityError::StaleReference {
                message: format!(
                    "{SUMMARY_EMPTY_EVIDENCE_REFUSAL_CODE}: none of the {} context artifacts \
                     resolves to its current version; nothing was generated",
                    refusal.input_evidence_count
                ),
            }
        })?;

        let path = std::path::Path::new(local_path);
        let manifest =
            medscale_pack::admit_pack_dir(path).map_err(|err| AuthorityError::InvalidArgument {
                message: format!("pack admission failed: {err}"),
            })?;
        // Exact model Pack identity/version: the directory at `local_path`
        // must be precisely the Pack this identity was registered against,
        // never merely "a" pack with a matching id.
        if manifest.pack_id != identity.pack_id || manifest.version != identity.pack_version {
            return Err(AuthorityError::DigestMismatch);
        }
        // Exact runtime identity: the directory must also match what is
        // actually admitted in this vault's PackStore right now, not a
        // caller-supplied directory merely claiming the same identity.
        let admitted = self
            .packs
            .get(&manifest.pack_id)
            .ok_or(AuthorityError::Unauthorized)?;
        if admitted.content_digest != manifest.content_digest
            || admitted.pack_epoch != manifest.pack_epoch
            || admitted.version != manifest.version
        {
            return Err(AuthorityError::DigestMismatch);
        }

        // The Pack's declared runtime (tract ONNX or candle safetensors,
        // Spec 104) is used; nothing is inferred from file names.
        let evaluation =
            medscale_pack::evaluate_admitted_pack(path, &manifest, &run.prompt, max_tokens)
                .map_err(|message| AuthorityError::InvalidArgument { message })?;

        let turn = self.append_turn(
            run_id,
            AgentTurnKind::ModelOutput,
            evaluation.output.proposal_payload.clone(),
        )?;

        // The actual claim content lives in the reused Proposal object
        // (contracts.md section 6); AgentProposal is a thin 077-owned
        // linking record. Constructed directly here (not through
        // RequestBody::CreateProposal) because that capability's frozen
        // request shape has no way to carry `producer:
        // ProducerKind::Agent(..)` -- it always sets `ProducerKind::Rule`
        // (facade.rs's CreateProposal arm) -- so this mirrors that arm's
        // exact construction pattern with the one field this spec's own
        // contracts.md section 7 addition exists to carry.
        let proposal_id = self.store.alloc_id("proposal");
        let proposal = Proposal {
            header: ObjectHeader {
                id: proposal_id.clone(),
                schema_version: medscale_contracts::AUTHORITY_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            subject_ref: None,
            claim_kind: "medagent_run_output".to_owned(),
            payload: evaluation.output.proposal_payload,
            confidence: None,
            evidence_refs,
            producer: ProducerKind::Agent(run.agent_identity_id.clone()),
        };
        self.store.insert(StoredObject::Proposal(proposal));

        let agent_proposal_header =
            self.next_medagent_header("medagent-proposal-id-seq", "agent-proposal")?;
        let agent_proposal = AgentProposal {
            header: agent_proposal_header,
            run_id: run_id.clone(),
            proposal_id,
        };
        self.meta
            .insert_agent_proposal(&agent_proposal)
            .map_err(meta_err)?;
        self.audit("medagent_run.execute", vec![run_id.clone()])?;
        Ok((turn, agent_proposal))
    }
}

#[cfg(test)]
mod tests {
    use super::super::source_ops::create_source_record;
    use super::*;
    use medscale_contracts::project_graph::Project;

    #[allow(clippy::too_many_arguments)]
    fn harness<'a>(
        meta: &'a SqliteMetaStore,
        store: &'a mut InMemoryAuthorityStore,
        packs: &'a medscale_pack::PackStore,
        sessions: &'a SessionRegistry,
        leases: &'a LeaseRegistry,
        vault_id: &'a VaultId,
        realm: &str,
        scope: &str,
    ) -> MedAgent<'a> {
        MedAgent {
            store,
            meta,
            packs,
            sessions,
            leases,
            vault_id,
            realm: RealmId::new(realm),
            scope: AuthorityScopeId::new(scope),
            session_id: None,
        }
    }

    fn temp_meta(name: &str) -> SqliteMetaStore {
        let dir =
            std::env::temp_dir().join(format!("medscale-077cu-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        SqliteMetaStore::open_at(&dir.join("meta.sqlite3")).unwrap()
    }

    fn project_header(realm: &str, scope: &str, id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: 1,
            realm_id: RealmId::new(realm),
            authority_scope_id: AuthorityScopeId::new(scope),
        }
    }

    #[test]
    fn resolve_context_artifact_matrix_matches_project_graph_semantics() {
        let meta = temp_meta("resolve");
        let mut store = InMemoryAuthorityStore::default();
        let packs = medscale_pack::PackStore::default();
        let sessions = SessionRegistry::new();
        let leases = LeaseRegistry::new();
        let vault_id = VaultId::new("vault-1");
        leases
            .acquire(&vault_id, OpaqueId::new("actor-a"), None)
            .unwrap();
        let record = create_source_record(
            &mut store,
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            "text/plain".to_owned(),
            b"hello".to_vec(),
        );
        let medagent = harness(
            &meta, &mut store, &packs, &sessions, &leases, &vault_id, "realm-a", "scope-a",
        );

        assert_eq!(
            medagent.resolve_context_artifact(&ArtifactDescriptor {
                object_id: record.header.id.clone(),
                kind: ArtifactKind::SourceRecord,
                binding: ArtifactVersionBinding::IdentityOnly,
            }),
            ReferenceResolution::Current
        );
        assert_eq!(
            medagent.resolve_context_artifact(&ArtifactDescriptor {
                object_id: OpaqueId::new("src-nope"),
                kind: ArtifactKind::SourceRecord,
                binding: ArtifactVersionBinding::IdentityOnly,
            }),
            ReferenceResolution::Missing
        );
        assert_eq!(
            medagent.resolve_context_artifact(&ArtifactDescriptor {
                object_id: record.header.id.clone(),
                kind: ArtifactKind::EvidenceDocument,
                binding: ArtifactVersionBinding::IdentityOnly,
            }),
            ReferenceResolution::UnsupportedKind
        );
    }

    #[test]
    fn create_and_get_context_manifest_resolves_live_never_cached() {
        let meta = temp_meta("create-get");
        meta.insert_project(
            &Project::new(
                project_header("realm-a", "scope-a", "proj-1"),
                "proj-1".to_owned(),
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let mut store = InMemoryAuthorityStore::default();
        let packs = medscale_pack::PackStore::default();
        let sessions = SessionRegistry::new();
        let leases = LeaseRegistry::new();
        let vault_id = VaultId::new("vault-1");
        leases
            .acquire(&vault_id, OpaqueId::new("actor-a"), None)
            .unwrap();
        let record = create_source_record(
            &mut store,
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            "text/plain".to_owned(),
            b"hello".to_vec(),
        );
        let mut medagent = harness(
            &meta, &mut store, &packs, &sessions, &leases, &vault_id, "realm-a", "scope-a",
        );

        let manifest = medagent
            .create_context_manifest(
                OpaqueId::new("proj-1"),
                vec![
                    ArtifactDescriptor {
                        object_id: record.header.id.clone(),
                        kind: ArtifactKind::SourceRecord,
                        binding: ArtifactVersionBinding::IdentityOnly,
                    },
                    // Not yet existing at creation time -- shape-only
                    // validation must still accept this (migration.md
                    // section 4: existence is never checked at write time).
                    ArtifactDescriptor {
                        object_id: OpaqueId::new("src-not-yet-created"),
                        kind: ArtifactKind::SourceRecord,
                        binding: ArtifactVersionBinding::IdentityOnly,
                    },
                ],
            )
            .unwrap();

        let (read_back, resolutions) = medagent.get_context_manifest(&manifest.header.id).unwrap();
        assert_eq!(read_back, manifest);
        assert_eq!(
            resolutions,
            vec![ReferenceResolution::Current, ReferenceResolution::Missing,]
        );
    }

    #[test]
    fn require_artifact_in_context_allows_named_denies_everything_else() {
        let meta = temp_meta("boundary");
        meta.insert_project(
            &Project::new(
                project_header("realm-a", "scope-a", "proj-1"),
                "proj-1".to_owned(),
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let mut store = InMemoryAuthorityStore::default();
        let packs = medscale_pack::PackStore::default();
        let sessions = SessionRegistry::new();
        let leases = LeaseRegistry::new();
        let vault_id = VaultId::new("vault-1");
        leases
            .acquire(&vault_id, OpaqueId::new("actor-a"), None)
            .unwrap();
        let allowed = create_source_record(
            &mut store,
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            "text/plain".to_owned(),
            b"allowed".to_vec(),
        );
        let outside = create_source_record(
            &mut store,
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            "text/plain".to_owned(),
            b"outside".to_vec(),
        );
        let mut medagent = harness(
            &meta, &mut store, &packs, &sessions, &leases, &vault_id, "realm-a", "scope-a",
        );
        let manifest = medagent
            .create_context_manifest(
                OpaqueId::new("proj-1"),
                vec![ArtifactDescriptor {
                    object_id: allowed.header.id.clone(),
                    kind: ArtifactKind::SourceRecord,
                    binding: ArtifactVersionBinding::IdentityOnly,
                }],
            )
            .unwrap();

        // The named artifact resolves through the boundary check.
        assert!(
            medagent
                .require_artifact_in_context(&manifest.header.id, &allowed.header.id)
                .is_ok()
        );
        // A real, existing artifact that simply was never named by this
        // ContextManifest is refused -- direct access outside the bound
        // context is denied even though the object itself is perfectly
        // readable to a human operator with full Project access
        // (security.md T4).
        let err = medagent
            .require_artifact_in_context(&manifest.header.id, &outside.header.id)
            .unwrap_err();
        assert!(matches!(err, AuthorityError::Unauthorized));
        // A fabricated id that never existed anywhere is refused the same
        // way -- the boundary check never needs to consult storage at all.
        let err = medagent
            .require_artifact_in_context(&manifest.header.id, &OpaqueId::new("src-fabricated"))
            .unwrap_err();
        assert!(matches!(err, AuthorityError::Unauthorized));
    }

    #[test]
    fn context_manifest_is_scope_isolated() {
        let meta = temp_meta("scope-isolation");
        meta.insert_project(
            &Project::new(
                project_header("realm-a", "scope-a", "proj-1"),
                "proj-1".to_owned(),
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let mut store = InMemoryAuthorityStore::default();
        let packs = medscale_pack::PackStore::default();
        let sessions = SessionRegistry::new();
        let leases = LeaseRegistry::new();
        let vault_id = VaultId::new("vault-1");
        leases
            .acquire(&vault_id, OpaqueId::new("actor-a"), None)
            .unwrap();
        let record = create_source_record(
            &mut store,
            RealmId::new("realm-a"),
            AuthorityScopeId::new("scope-a"),
            "text/plain".to_owned(),
            b"hello".to_vec(),
        );
        let context_id = {
            let mut medagent_a = harness(
                &meta, &mut store, &packs, &sessions, &leases, &vault_id, "realm-a", "scope-a",
            );
            medagent_a
                .create_context_manifest(
                    OpaqueId::new("proj-1"),
                    vec![ArtifactDescriptor {
                        object_id: record.header.id.clone(),
                        kind: ArtifactKind::SourceRecord,
                        binding: ArtifactVersionBinding::IdentityOnly,
                    }],
                )
                .unwrap()
                .header
                .id
        };

        // A second caller scoped to a different realm/scope cannot read
        // scope-a's ContextManifest at all -- WrongScope, not NotFound
        // (which would leak existence) and not a successful read.
        let medagent_b = harness(
            &meta, &mut store, &packs, &sessions, &leases, &vault_id, "realm-b", "scope-b",
        );
        let err = medagent_b.get_context_manifest(&context_id).unwrap_err();
        assert!(matches!(err, AuthorityError::WrongScope));
        let err = medagent_b
            .require_artifact_in_context(&context_id, &record.header.id)
            .unwrap_err();
        assert!(matches!(err, AuthorityError::WrongScope));
    }
}

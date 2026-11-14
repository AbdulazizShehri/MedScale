//! Spec 077 MedAgent Workbench Core authority integration tests (T077-03).
//!
//! Every mutation travels request -> session -> realm/scope -> `MedAgent`
//! authority -> validation -> revision -> transaction -> typed result
//! through `CoreFacade::dispatch`. Synthetic data only. T077-03 scope:
//! `AgentIdentity` + `AgentCapabilityManifest` register/get/list/revoke.

use std::fs;
use std::path::PathBuf;

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::medagent::{AgentRunState, AgentTurnKind, ToolKind};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding};
use medscale_core::CoreFacade;

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-077c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0")
}

/// The Spec 069 fixture pack, `runtime_requirements:
/// tract_onnx_token_classification_v1` -- the only pack in this repository
/// admitted for real local model execution (`OnnxTokenClassifierRuntime`).
/// `fixture_pack()` above (Spec 008's `pack-fixture-ner-v0`) declares
/// `runtime_requirements: fixture_runtime_v0` and carries no ONNX
/// model/tokenizer/labels artifacts at all -- it is admissible for
/// identity registration but cannot be executed as a real model, which is
/// exactly why T077-07 uses this pack instead.
fn onnx_fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0",
    )
}

const REALM: &str = "realm-a";
const SCOPE: &str = "scope-a";
const VAULT: &str = "vault-1";

/// Single-actor Core harness (mirrors `collaboration_076.rs`'s `Harness`,
/// trimmed to what T077-03 needs: no multi-actor switching required since
/// agent identity registration/revocation is scope-level, not
/// membership-gated).
struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    dir: std::path::PathBuf,
    next_req: u64,
}

fn base_req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new(VAULT),
        RealmId::new(REALM),
        AuthorityScopeId::new(SCOPE),
        capability,
        body,
    )
}

impl Harness {
    fn setup(name: &str) -> Self {
        let dir = tmp_dir(name);
        let facade = CoreFacade::new();
        let mut h = Harness {
            facade,
            session: OpaqueId::new("pending"),
            dir,
            next_req: 0,
        };
        let mut vault_req = base_req(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: h.dir.to_str().unwrap().to_owned(),
            },
        );
        // Bootstrap a lease + session before the vault open so later calls
        // carry a valid session_id (mirrors collaboration_076.rs).
        let lease = h
            .facade
            .dispatch(base_req(
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new("actor-a"),
                    holder_id_hint: Some(OpaqueId::new("actor-a")),
                },
            ))
            .result
            .expect("lease");
        let holder_id = match lease {
            ResponseBody::Lease { holder_id, .. } => holder_id,
            other => panic!("{other:?}"),
        };
        let session = h
            .facade
            .dispatch(base_req(
                Capability::OpenSession,
                RequestBody::OpenSession {
                    holder_id,
                    granted: Capability::operator_grants(),
                    ttl_ticks: 1_000_000,
                },
            ))
            .result
            .expect("session");
        h.session = match session {
            ResponseBody::Session { session_id, .. } => session_id,
            other => panic!("{other:?}"),
        };
        vault_req.session_id = Some(h.session.clone());
        h.facade.dispatch(vault_req).result.expect("vault");
        h
    }

    fn req_id(&mut self) -> OpaqueId {
        self.next_req += 1;
        OpaqueId::new(format!("req-{}", self.next_req))
    }

    fn call(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        let id = self.req_id();
        let mut request = AuthorityRequest::new(
            id,
            VaultId::new(VAULT),
            RealmId::new(REALM),
            AuthorityScopeId::new(SCOPE),
            capability,
            body,
        );
        request.session_id = Some(self.session.clone());
        self.facade.dispatch(request).result
    }

    fn project(&mut self, name: &str) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: name.to_owned(),
                    description: None,
                },
            )
            .expect("project")
        {
            ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    /// Installs the Spec 008 fixture pack and returns its admitted
    /// `pack_id`.
    fn install_fixture_pack(&mut self) -> OpaqueId {
        let path = fixture_pack().display().to_string();
        match self
            .call(
                Capability::PacksInstallLocal,
                RequestBody::PacksInstallLocal { local_path: path },
            )
            .expect("pack install")
        {
            ResponseBody::PackAdmit { result } => {
                assert!(result.admitted, "fixture pack must admit cleanly");
                result.pack_id.expect("admitted pack carries a pack_id")
            }
            other => panic!("{other:?}"),
        }
    }

    /// Installs the Spec 069 ONNX-runtime-admitted fixture pack.
    fn install_onnx_fixture_pack(&mut self) -> OpaqueId {
        let path = onnx_fixture_pack().display().to_string();
        match self
            .call(
                Capability::PacksInstallLocal,
                RequestBody::PacksInstallLocal { local_path: path },
            )
            .expect("onnx pack install")
        {
            ResponseBody::PackAdmit { result } => {
                assert!(result.admitted, "onnx fixture pack must admit cleanly");
                result.pack_id.expect("admitted pack carries a pack_id")
            }
            other => panic!("{other:?}"),
        }
    }

    fn register_identity(&mut self, project_id: OpaqueId, pack_id: OpaqueId) -> OpaqueId {
        match self
            .call(
                Capability::AgentIdentityRegister,
                RequestBody::AgentIdentityRegister {
                    project_id,
                    pack_id,
                    display_name: "Research Assistant".to_owned(),
                    granted_tool_kinds: vec![ToolKind::ReadContextArtifact],
                },
            )
            .expect("register identity")
        {
            ResponseBody::MedAgentIdentity { identity, .. } => identity.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn create_context(&mut self, project_id: OpaqueId, artifact_id: &str) -> OpaqueId {
        match self
            .call(
                Capability::ContextManifestCreate,
                RequestBody::ContextManifestCreate {
                    project_id,
                    selected_artifacts: vec![ArtifactDescriptor {
                        object_id: OpaqueId::new(artifact_id),
                        kind: ArtifactKind::SourceRecord,
                        binding: ArtifactVersionBinding::IdentityOnly,
                    }],
                },
            )
            .expect("create context")
        {
            ResponseBody::MedAgentContextManifest { manifest, .. } => manifest.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn create_run(
        &mut self,
        project_id: OpaqueId,
        agent_id: OpaqueId,
        context_id: OpaqueId,
        prompt: &str,
    ) -> OpaqueId {
        match self
            .call(
                Capability::AgentRunCreate,
                RequestBody::AgentRunCreate {
                    project_id,
                    agent_identity_id: agent_id,
                    context_manifest_id: context_id,
                    prompt: prompt.to_owned(),
                },
            )
            .expect("create run")
        {
            ResponseBody::MedAgentRun { run } => run.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn source_record(&mut self, bytes: &[u8]) -> OpaqueId {
        match self
            .call(
                Capability::CreateSourceRecord,
                RequestBody::CreateSourceRecord {
                    media_type: "text/plain".to_owned(),
                    bytes: bytes.to_vec(),
                },
            )
            .expect("create source record")
        {
            ResponseBody::Created { object_id } => object_id,
            other => panic!("{other:?}"),
        }
    }

    fn start_run(&mut self, run_id: OpaqueId) {
        self.call(
            Capability::AgentRunStart,
            RequestBody::AgentRunStart {
                run_id,
                expected_revision: 1,
            },
        )
        .expect("start run");
    }
}

#[test]
fn register_get_list_revoke_roundtrip_through_core() {
    let mut h = Harness::setup("lifecycle");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();

    let resp = h
        .call(
            Capability::AgentIdentityRegister,
            RequestBody::AgentIdentityRegister {
                project_id: project_id.clone(),
                pack_id: pack_id.clone(),
                display_name: "Research Assistant".to_owned(),
                granted_tool_kinds: vec![ToolKind::ReadContextArtifact],
            },
        )
        .expect("register");
    let (agent_id, pack_version) = match resp {
        ResponseBody::MedAgentIdentity {
            identity,
            capabilities,
        } => {
            assert_eq!(identity.project_id, project_id);
            assert_eq!(identity.pack_id, pack_id);
            assert!(!identity.pack_version.is_empty());
            assert!(capabilities.grants(ToolKind::ReadContextArtifact));
            assert!(!capabilities.grants(ToolKind::SearchContextArtifacts));
            (identity.header.id, identity.pack_version)
        }
        other => panic!("{other:?}"),
    };

    let resp = h
        .call(
            Capability::AgentIdentityRead,
            RequestBody::AgentIdentityGet {
                agent_id: agent_id.clone(),
            },
        )
        .expect("get");
    match resp {
        ResponseBody::MedAgentIdentity { identity, .. } => {
            assert_eq!(identity.header.id, agent_id);
            assert_eq!(identity.pack_version, pack_version);
            assert_eq!(identity.revision, 1);
        }
        other => panic!("{other:?}"),
    }

    let resp = h
        .call(
            Capability::AgentIdentityRead,
            RequestBody::AgentIdentityList {
                project_id: project_id.clone(),
                limit: None,
            },
        )
        .expect("list");
    match resp {
        ResponseBody::MedAgentIdentityList { identities } => {
            assert_eq!(identities.len(), 1);
            assert_eq!(identities[0].header.id, agent_id);
        }
        other => panic!("{other:?}"),
    }

    let resp = h
        .call(
            Capability::AgentIdentityRevoke,
            RequestBody::AgentIdentityRevoke {
                agent_id: agent_id.clone(),
                expected_revision: 1,
            },
        )
        .expect("revoke");
    match resp {
        ResponseBody::MedAgentIdentity { identity, .. } => {
            assert_eq!(identity.status.as_str(), "revoked");
            assert_eq!(identity.revision, 2);
        }
        other => panic!("{other:?}"),
    }

    // Revoking again with the now-stale expected_revision fails closed.
    let err = h
        .call(
            Capability::AgentIdentityRevoke,
            RequestBody::AgentIdentityRevoke {
                agent_id,
                expected_revision: 1,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Conflict { .. }));
}

/// `security.md` T5: registration against a `pack_id` the local
/// `PackStore` does not recognize must fail closed, never silently create
/// an identity with a fabricated `pack_version`.
#[test]
fn registration_against_non_admitted_pack_fails_closed() {
    let mut h = Harness::setup("non-admitted-pack");
    let project_id = h.project("agent-project");

    let err = h
        .call(
            Capability::AgentIdentityRegister,
            RequestBody::AgentIdentityRegister {
                project_id,
                pack_id: OpaqueId::new("never-admitted-pack"),
                display_name: "Rogue Agent".to_owned(),
                granted_tool_kinds: vec![ToolKind::ReadContextArtifact],
            },
        )
        .unwrap_err();
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "expected InvalidArgument for a non-admitted pack, got {err:?}"
    );
}

/// Capability manifest is immutable once set: there is no update path, only
/// revoke-and-re-register. Re-registering the same holder concept (a fresh
/// `AgentIdentity`) is a distinct object, never an in-place capability
/// widening of the first.
#[test]
fn capability_manifest_is_immutable_and_registration_is_scoped_to_its_project() {
    let mut h = Harness::setup("scoped");
    let project_id = h.project("agent-project");
    let other_project_id = h.project("other-project");
    let pack_id = h.install_fixture_pack();

    let resp = h
        .call(
            Capability::AgentIdentityRegister,
            RequestBody::AgentIdentityRegister {
                project_id,
                pack_id: pack_id.clone(),
                display_name: "Agent One".to_owned(),
                granted_tool_kinds: vec![ToolKind::ReadContextArtifact],
            },
        )
        .expect("register");
    let agent_id = match resp {
        ResponseBody::MedAgentIdentity { identity, .. } => identity.header.id,
        other => panic!("{other:?}"),
    };

    // A second identity registered against a different Project, even with
    // the same pack, is a distinct object with its own id and manifest.
    let resp = h
        .call(
            Capability::AgentIdentityRegister,
            RequestBody::AgentIdentityRegister {
                project_id: other_project_id,
                pack_id,
                display_name: "Agent Two".to_owned(),
                granted_tool_kinds: vec![
                    ToolKind::ReadContextArtifact,
                    ToolKind::SearchContextArtifacts,
                ],
            },
        )
        .expect("register second");
    let other_agent_id = match resp {
        ResponseBody::MedAgentIdentity {
            identity,
            capabilities,
        } => {
            assert!(capabilities.grants(ToolKind::SearchContextArtifacts));
            identity.header.id
        }
        other => panic!("{other:?}"),
    };
    assert_ne!(agent_id, other_agent_id);

    // The first identity's own capability manifest is unaffected by the
    // second registration.
    let resp = h
        .call(
            Capability::AgentIdentityRead,
            RequestBody::AgentIdentityGet { agent_id },
        )
        .expect("get first");
    match resp {
        ResponseBody::MedAgentIdentity { capabilities, .. } => {
            assert!(!capabilities.grants(ToolKind::SearchContextArtifacts));
        }
        other => panic!("{other:?}"),
    }
}

/// Reopen durability: an agent identity registered before closing the
/// synthetic vault must read back identically after a fresh `CoreFacade`
/// reopens it (matches `collaboration_076.rs`'s own reopen precedent).
#[test]
fn agent_identity_survives_vault_reopen() {
    let mut h = Harness::setup("reopen");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let resp = h
        .call(
            Capability::AgentIdentityRegister,
            RequestBody::AgentIdentityRegister {
                project_id,
                pack_id,
                display_name: "Durable Agent".to_owned(),
                granted_tool_kinds: vec![ToolKind::ReadContextArtifact],
            },
        )
        .expect("register");
    let (agent_id, pack_version) = match resp {
        ResponseBody::MedAgentIdentity { identity, .. } => {
            (identity.header.id, identity.pack_version)
        }
        other => panic!("{other:?}"),
    };
    // Close through the real capability path (matches durable_restart_016.rs's
    // two-process precedent) rather than relying on Drop timing.
    h.call(Capability::CloseVault, RequestBody::CloseVault)
        .expect("close vault");
    let dir = h.dir;

    // Fresh facade/session/vault-open against the same on-disk root,
    // simulating a real process restart.
    let facade = CoreFacade::new();
    let lease = facade
        .dispatch(base_req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("actor-b"),
                holder_id_hint: Some(OpaqueId::new("actor-b")),
            },
        ))
        .result
        .expect("lease");
    let holder_id = match lease {
        ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };
    let session = facade
        .dispatch(base_req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id,
                granted: Capability::operator_grants(),
                ttl_ticks: 1_000_000,
            },
        ))
        .result
        .expect("session");
    let session_id = match session {
        ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };
    let mut vault_req = base_req(
        Capability::OpenSyntheticVault,
        RequestBody::OpenSyntheticVault {
            vault_root: dir.to_str().unwrap().to_owned(),
        },
    );
    vault_req.session_id = Some(session_id.clone());
    facade.dispatch(vault_req).result.expect("reopen vault");

    let mut get_req = base_req(
        Capability::AgentIdentityRead,
        RequestBody::AgentIdentityGet { agent_id },
    );
    get_req.session_id = Some(session_id);
    let resp = facade.dispatch(get_req).result.expect("get after reopen");
    match resp {
        ResponseBody::MedAgentIdentity { identity, .. } => {
            assert_eq!(identity.pack_version, pack_version);
            assert_eq!(identity.revision, 1);
        }
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------
// T077-05: AgentRun lifecycle
// ---------------------------------------------------------------------------

#[test]
fn full_run_lifecycle_start_then_cancel_through_core() {
    let mut h = Harness::setup("run-lifecycle");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");
    let run_id = h.create_run(
        project_id,
        agent_id,
        context_id,
        "summarize the bound context",
    );

    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunGet {
                run_id: run_id.clone(),
            },
        )
        .expect("get pending run");
    match resp {
        ResponseBody::MedAgentRun { run } => {
            assert_eq!(run.status, AgentRunState::Pending);
            assert_eq!(run.revision, 1);
        }
        other => panic!("{other:?}"),
    }

    // Starting appends the initial PromptSubmitted turn automatically --
    // no caller-supplied turn content is ever trusted.
    let resp = h
        .call(
            Capability::AgentRunStart,
            RequestBody::AgentRunStart {
                run_id: run_id.clone(),
                expected_revision: 1,
            },
        )
        .expect("start run");
    match resp {
        ResponseBody::MedAgentRunStarted { run, turn } => {
            assert_eq!(run.status, AgentRunState::Running);
            assert_eq!(run.revision, 2);
            assert_eq!(turn.kind, AgentTurnKind::PromptSubmitted);
            assert_eq!(turn.seq, 1);
            assert_eq!(
                turn.payload.get("prompt").and_then(|v| v.as_str()),
                Some("summarize the bound context")
            );
        }
        other => panic!("{other:?}"),
    }

    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunTurnList {
                run_id: run_id.clone(),
            },
        )
        .expect("list turns");
    match resp {
        ResponseBody::MedAgentTurnList { turns } => assert_eq!(turns.len(), 1),
        other => panic!("{other:?}"),
    }

    // Cancel a Running run: terminal transition + RunReceipt committed
    // atomically, with no tool invocations yet (T077-06 threads real ones).
    let resp = h
        .call(
            Capability::AgentRunCancel,
            RequestBody::AgentRunCancel {
                run_id: run_id.clone(),
                expected_revision: 2,
            },
        )
        .expect("cancel run");
    match resp {
        ResponseBody::MedAgentRunTerminal { run, receipt } => {
            assert_eq!(run.status, AgentRunState::Cancelled);
            assert_eq!(run.revision, 3);
            assert_eq!(receipt.run_id, run_id);
            assert_eq!(receipt.final_state, AgentRunState::Cancelled);
            assert!(receipt.failure_reason.is_none());
            assert!(receipt.tool_invocation_ids.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn pending_run_can_be_cancelled_directly_without_starting() {
    let mut h = Harness::setup("cancel-pending");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");
    let run_id = h.create_run(project_id, agent_id, context_id, "go");

    let resp = h
        .call(
            Capability::AgentRunCancel,
            RequestBody::AgentRunCancel {
                run_id: run_id.clone(),
                expected_revision: 1,
            },
        )
        .expect("cancel pending run");
    match resp {
        ResponseBody::MedAgentRunTerminal { run, .. } => {
            assert_eq!(run.status, AgentRunState::Cancelled);
        }
        other => panic!("{other:?}"),
    }

    // No turns were ever appended for a run cancelled before starting.
    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunTurnList { run_id },
        )
        .expect("list turns");
    match resp {
        ResponseBody::MedAgentTurnList { turns } => assert!(turns.is_empty()),
        other => panic!("{other:?}"),
    }
}

/// Cancellation race: two callers who both observed the run at revision 1
/// (e.g. a UI cancel click racing an operator's CLI cancel) both submit a
/// cancel request. Only one may win; the other must fail closed on the now
/// -stale revision, never silently re-apply or double-commit a second
/// `RunReceipt` (`plan.md` 077-E gate: "no transition outside the frozen
/// table is reachable").
#[test]
fn cancellation_race_only_one_request_wins() {
    let mut h = Harness::setup("cancel-race");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");
    let run_id = h.create_run(project_id, agent_id, context_id, "go");

    let first = h.call(
        Capability::AgentRunCancel,
        RequestBody::AgentRunCancel {
            run_id: run_id.clone(),
            expected_revision: 1,
        },
    );
    let second = h.call(
        Capability::AgentRunCancel,
        RequestBody::AgentRunCancel {
            run_id: run_id.clone(),
            expected_revision: 1,
        },
    );
    let results = [first, second];
    let wins = results.iter().filter(|r| r.is_ok()).count();
    let conflicts = results
        .iter()
        .filter(|r| matches!(r, Err(AuthorityError::Conflict { .. })))
        .count();
    assert_eq!(wins, 1, "exactly one cancel request must win the race");
    assert_eq!(
        conflicts, 1,
        "the loser must fail closed with Conflict, never silently re-apply"
    );

    // The run itself shows exactly one terminal transition (revision 2,
    // not 3): the loser never mutated anything.
    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunGet { run_id },
        )
        .expect("get after race");
    match resp {
        ResponseBody::MedAgentRun { run } => {
            assert_eq!(run.status, AgentRunState::Cancelled);
            assert_eq!(run.revision, 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn illegal_transitions_are_rejected() {
    let mut h = Harness::setup("illegal-transitions");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");
    let run_id = h.create_run(project_id, agent_id, context_id, "go");

    h.call(
        Capability::AgentRunCancel,
        RequestBody::AgentRunCancel {
            run_id: run_id.clone(),
            expected_revision: 1,
        },
    )
    .expect("cancel");

    // Cancelled -> Running is not in the frozen table.
    let err = h
        .call(
            Capability::AgentRunStart,
            RequestBody::AgentRunStart {
                run_id: run_id.clone(),
                expected_revision: 2,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Conflict { .. }));

    // Cancelled -> Cancelled again is not in the frozen table either.
    let err = h
        .call(
            Capability::AgentRunCancel,
            RequestBody::AgentRunCancel {
                run_id,
                expected_revision: 2,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Conflict { .. }));
}

#[test]
fn run_creation_rejects_cross_project_identity_and_context() {
    let mut h = Harness::setup("cross-project");
    let project_a = h.project("project-a");
    let project_b = h.project("project-b");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_a.clone(), pack_id);
    let context_id = h.create_context(project_a.clone(), "artifact-1");

    // agent_id belongs to project_a, but the run is created against
    // project_b.
    let err = h
        .call(
            Capability::AgentRunCreate,
            RequestBody::AgentRunCreate {
                project_id: project_b.clone(),
                agent_identity_id: agent_id.clone(),
                context_manifest_id: context_id.clone(),
                prompt: "go".to_owned(),
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::InvalidArgument { .. }));

    // context_id also belongs to project_a; naming project_a correctly for
    // the identity but pairing it with a context from a different project
    // must fail too (build a second context under project_b to prove it's
    // the context check, not just the identity check, that fires).
    let context_b = h.create_context(project_b.clone(), "artifact-2");
    let err = h
        .call(
            Capability::AgentRunCreate,
            RequestBody::AgentRunCreate {
                project_id: project_a,
                agent_identity_id: agent_id,
                context_manifest_id: context_b,
                prompt: "go".to_owned(),
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::InvalidArgument { .. }));
}

/// `security.md` T5: a revoked identity must refuse both starting a new
/// run and (via `create_agent_run`'s own re-check) creating one -- the
/// check is never cached across the run's lifetime.
#[test]
fn revoked_identity_fails_closed_at_create_and_at_start() {
    let mut h = Harness::setup("revoked-identity");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");

    // A run created while the identity is still Active, then the identity
    // is revoked before the run ever starts.
    let run_id = h.create_run(
        project_id.clone(),
        agent_id.clone(),
        context_id.clone(),
        "go",
    );
    h.call(
        Capability::AgentIdentityRevoke,
        RequestBody::AgentIdentityRevoke {
            agent_id: agent_id.clone(),
            expected_revision: 1,
        },
    )
    .expect("revoke");

    let err = h
        .call(
            Capability::AgentRunStart,
            RequestBody::AgentRunStart {
                run_id,
                expected_revision: 1,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Unauthorized));

    // A brand new run creation attempt against the now-revoked identity is
    // refused at create time too.
    let err = h
        .call(
            Capability::AgentRunCreate,
            RequestBody::AgentRunCreate {
                project_id,
                agent_identity_id: agent_id,
                context_manifest_id: context_id,
                prompt: "go".to_owned(),
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Unauthorized));
}

// ---------------------------------------------------------------------------
// T077-06: Tool invocation
// ---------------------------------------------------------------------------

/// Sets up a Running run bound to a ContextManifest naming exactly one
/// real SourceRecord, with the agent granted only `granted`. Returns
/// (run_id, source_id).
fn running_run_with_one_source(
    h: &mut Harness,
    granted: Vec<ToolKind>,
    content: &[u8],
) -> (OpaqueId, OpaqueId) {
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let source_id = h.source_record(content);
    let resp = h
        .call(
            Capability::AgentIdentityRegister,
            RequestBody::AgentIdentityRegister {
                project_id: project_id.clone(),
                pack_id,
                display_name: "Research Assistant".to_owned(),
                granted_tool_kinds: granted,
            },
        )
        .expect("register identity");
    let agent_id = match resp {
        ResponseBody::MedAgentIdentity { identity, .. } => identity.header.id,
        other => panic!("{other:?}"),
    };
    let resp = h
        .call(
            Capability::ContextManifestCreate,
            RequestBody::ContextManifestCreate {
                project_id: project_id.clone(),
                selected_artifacts: vec![ArtifactDescriptor {
                    object_id: source_id.clone(),
                    kind: ArtifactKind::SourceRecord,
                    binding: ArtifactVersionBinding::IdentityOnly,
                }],
            },
        )
        .expect("create context");
    let context_id = match resp {
        ResponseBody::MedAgentContextManifest { manifest, .. } => manifest.header.id,
        other => panic!("{other:?}"),
    };
    let run_id = h.create_run(project_id, agent_id, context_id, "read the source");
    h.start_run(run_id.clone());
    (run_id, source_id)
}

#[test]
fn granted_read_context_artifact_executes_and_returns_content() {
    let mut h = Harness::setup("tool-read-ok");
    let (run_id, source_id) =
        running_run_with_one_source(&mut h, vec![ToolKind::ReadContextArtifact], b"hello world");

    let resp = h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id: run_id.clone(),
                kind: ToolKind::ReadContextArtifact,
                arguments: serde_json::json!({ "object_id": source_id.as_str() }),
            },
        )
        .expect("invoke tool");
    match resp {
        ResponseBody::MedAgentToolInvocation {
            invocation,
            receipt,
        } => {
            assert_eq!(invocation.status.as_str(), "executed");
            assert!(invocation.refusal_reason.is_none());
            let receipt = receipt.expect("executed invocation carries a receipt");
            assert_eq!(
                receipt.result.get("content").and_then(|v| v.as_str()),
                Some("hello world")
            );
        }
        other => panic!("{other:?}"),
    }

    // Both ToolRequested and ToolResult turns were appended, after the
    // initial PromptSubmitted from start_agent_run.
    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunTurnList { run_id },
        )
        .expect("list turns");
    match resp {
        ResponseBody::MedAgentTurnList { turns } => {
            assert_eq!(turns.len(), 3);
            assert_eq!(turns[0].kind.as_str(), "prompt_submitted");
            assert_eq!(turns[1].kind.as_str(), "tool_requested");
            assert_eq!(turns[2].kind.as_str(), "tool_result");
        }
        other => panic!("{other:?}"),
    }
}

/// `security.md` T2: an ungranted tool kind is refused before execution
/// and recorded as a refusal, never silently dropped or executed.
#[test]
fn ungranted_tool_kind_is_refused_before_execution() {
    let mut h = Harness::setup("tool-ungranted");
    // Only SearchContextArtifacts is granted; ReadContextArtifact is not.
    let (run_id, source_id) =
        running_run_with_one_source(&mut h, vec![ToolKind::SearchContextArtifacts], b"secret");

    let resp = h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id,
                kind: ToolKind::ReadContextArtifact,
                arguments: serde_json::json!({ "object_id": source_id.as_str() }),
            },
        )
        .expect("invoke tool");
    match resp {
        ResponseBody::MedAgentToolInvocation {
            invocation,
            receipt,
        } => {
            assert_eq!(invocation.status.as_str(), "refused");
            assert!(invocation.refusal_reason.is_some());
            assert!(
                receipt.is_none(),
                "a refused invocation never has a receipt"
            );
        }
        other => panic!("{other:?}"),
    }
}

/// `security.md` T4: a granted tool kind whose arguments name an artifact
/// outside the run's bound ContextManifest is refused, never executed --
/// even though the object itself really exists and is readable to a human
/// operator with full Project access.
#[test]
fn granted_tool_refuses_artifact_outside_bound_context() {
    let mut h = Harness::setup("tool-outside-context");
    let (run_id, _source_id) =
        running_run_with_one_source(&mut h, vec![ToolKind::ReadContextArtifact], b"in context");
    // A second, real source record that exists but was never named by this
    // run's ContextManifest.
    let outside_id = h.source_record(b"outside context");

    let resp = h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id,
                kind: ToolKind::ReadContextArtifact,
                arguments: serde_json::json!({ "object_id": outside_id.as_str() }),
            },
        )
        .expect("invoke tool");
    match resp {
        ResponseBody::MedAgentToolInvocation {
            invocation,
            receipt,
        } => {
            assert_eq!(invocation.status.as_str(), "refused");
            assert!(
                invocation
                    .refusal_reason
                    .as_deref()
                    .unwrap_or_default()
                    .contains("ContextManifest")
            );
            assert!(receipt.is_none());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn search_context_artifacts_finds_only_bound_content() {
    let mut h = Harness::setup("tool-search");
    let (run_id, _source_id) = running_run_with_one_source(
        &mut h,
        vec![ToolKind::SearchContextArtifacts],
        b"the quick brown fox jumps over the lazy dog",
    );

    let resp = h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id,
                kind: ToolKind::SearchContextArtifacts,
                arguments: serde_json::json!({ "query": "brown fox" }),
            },
        )
        .expect("invoke tool");
    match resp {
        ResponseBody::MedAgentToolInvocation {
            invocation,
            receipt,
        } => {
            assert_eq!(invocation.status.as_str(), "executed");
            let receipt = receipt.expect("executed invocation carries a receipt");
            let matches = receipt
                .result
                .get("matches")
                .and_then(|v| v.as_array())
                .expect("matches array");
            assert_eq!(matches.len(), 1);
            let snippet = matches[0]
                .get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            assert!(snippet.contains("brown fox"));
        }
        other => panic!("{other:?}"),
    }
}

/// `execute_search_context_artifacts` bounds per-artifact scan work (this
/// session's OpenCodeReview finding): content placed past the scan cap
/// must not be found, proving the cap is actually enforced, not merely
/// documented. 300 KiB of filler comfortably exceeds the
/// 256 KiB `SEARCH_ARTIFACT_SCAN_MAX_BYTES` cap.
#[test]
fn search_context_artifacts_does_not_scan_past_the_size_cap() {
    let mut h = Harness::setup("tool-search-cap");
    let filler = "x".repeat(300 * 1024);
    let content = format!("{filler}unique-marker-beyond-cap");
    let (run_id, _source_id) = running_run_with_one_source(
        &mut h,
        vec![ToolKind::SearchContextArtifacts],
        content.as_bytes(),
    );

    let resp = h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id,
                kind: ToolKind::SearchContextArtifacts,
                arguments: serde_json::json!({ "query": "unique-marker-beyond-cap" }),
            },
        )
        .expect("invoke tool");
    match resp {
        ResponseBody::MedAgentToolInvocation {
            invocation,
            receipt,
        } => {
            assert_eq!(invocation.status.as_str(), "executed");
            let receipt = receipt.expect("executed invocation carries a receipt");
            let matches = receipt
                .result
                .get("matches")
                .and_then(|v| v.as_array())
                .expect("matches array");
            assert!(
                matches.is_empty(),
                "content beyond the scan cap must not be found, got {matches:?}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn tool_invocation_refused_with_malformed_arguments() {
    let mut h = Harness::setup("tool-malformed");
    let (run_id, _source_id) =
        running_run_with_one_source(&mut h, vec![ToolKind::ReadContextArtifact], b"content");

    // object_id is required by ReadContextArtifactArgs; this payload omits
    // it entirely, so typed parsing must fail, not partially execute.
    let resp = h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id,
                kind: ToolKind::ReadContextArtifact,
                arguments: serde_json::json!({ "unexpected_field": "x" }),
            },
        )
        .expect("invoke tool");
    match resp {
        ResponseBody::MedAgentToolInvocation {
            invocation,
            receipt,
        } => {
            assert_eq!(invocation.status.as_str(), "refused");
            assert!(receipt.is_none());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn tool_invocation_on_a_non_running_run_is_refused_closed() {
    let mut h = Harness::setup("tool-not-running");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let source_id = h.source_record(b"content");
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), source_id.as_str());
    // Pending, never started.
    let run_id = h.create_run(project_id, agent_id, context_id, "go");

    let err = h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id,
                kind: ToolKind::ReadContextArtifact,
                arguments: serde_json::json!({ "object_id": source_id.as_str() }),
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Conflict { .. }));
}

// ---------------------------------------------------------------------------
// T077-07: Model Pack lane + AgentProposal
// ---------------------------------------------------------------------------

/// Sets up a Running run bound to the real ONNX-admitted fixture pack,
/// with `prompt` short enough to fit the fixture's `fixed_sequence_length
/// == 4` token budget (matches `real_local_model_runtime_069.rs`'s own
/// input exactly, a known-good value for this fixture's tokenizer).
fn running_run_on_onnx_pack(h: &mut Harness) -> OpaqueId {
    let project_id = h.project("agent-project");
    let pack_id = h.install_onnx_fixture_pack();
    let source_id = h.source_record(b"synthetic clinical note");
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), source_id.as_str());
    let run_id = h.create_run(
        project_id,
        agent_id,
        context_id,
        "alice visited clinic today",
    );
    h.start_run(run_id.clone());
    run_id
}

#[test]
fn real_local_model_execution_produces_agent_proposal() {
    let mut h = Harness::setup("model-execute-ok");
    let run_id = running_run_on_onnx_pack(&mut h);

    let resp = h
        .call(
            Capability::AgentRunExecute,
            RequestBody::AgentRunExecute {
                run_id: run_id.clone(),
                local_path: onnx_fixture_pack().display().to_string(),
                max_tokens: 4,
                synthetic_only: true,
            },
        )
        .expect("execute run");
    let (turn, proposal) = match resp {
        ResponseBody::MedAgentRunExecuted { turn, proposal } => (turn, proposal),
        other => panic!("{other:?}"),
    };
    assert_eq!(turn.kind.as_str(), "model_output");
    assert_eq!(
        turn.payload.get("runtime").and_then(|v| v.as_str()),
        Some("tract_onnx_token_classification_v1")
    );
    assert_eq!(proposal.run_id, run_id);

    // The reused Proposal object itself carries producer =
    // ProducerKind::Agent(agent_identity_id) -- contracts.md section 7's
    // whole reason for existing.
    let resp = h
        .call(
            Capability::ReadObject,
            RequestBody::ReadObject {
                object_id: proposal.proposal_id.clone(),
            },
        )
        .expect("read proposal object");
    match resp {
        ResponseBody::Object { value } => {
            assert_eq!(
                value.get("claim_kind").and_then(|v| v.as_str()),
                Some("medagent_run_output")
            );
            // ProducerKind carries #[serde(rename_all = "snake_case")], so
            // the Agent(OpaqueId) variant serializes as {"agent": "<id>"}.
            assert!(
                value.get("producer").and_then(|p| p.get("agent")).is_some(),
                "expected producer: agent(..), got {value:?}"
            );
            assert!(
                value
                    .get("evidence_refs")
                    .and_then(|v| v.as_array())
                    .is_some_and(|refs| !refs.is_empty()),
                "expected non-empty evidence_refs linking the run's bound context artifacts"
            );
        }
        other => panic!("{other:?}"),
    }

    // The turn sequence: prompt_submitted (from start), model_output.
    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunTurnList { run_id },
        )
        .expect("list turns");
    match resp {
        ResponseBody::MedAgentTurnList { turns } => {
            assert_eq!(turns.len(), 2);
            assert_eq!(turns[0].kind.as_str(), "prompt_submitted");
            assert_eq!(turns[1].kind.as_str(), "model_output");
        }
        other => panic!("{other:?}"),
    }
}

/// `RELEASE_READY`/real-PHI claims require an explicit gate this spec does
/// not grant; `synthetic_only: false` must refuse before touching the
/// runtime at all.
#[test]
fn non_synthetic_execution_is_refused_closed() {
    let mut h = Harness::setup("model-execute-real-phi-gate");
    let run_id = running_run_on_onnx_pack(&mut h);

    let err = h
        .call(
            Capability::AgentRunExecute,
            RequestBody::AgentRunExecute {
                run_id,
                local_path: onnx_fixture_pack().display().to_string(),
                max_tokens: 4,
                synthetic_only: false,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::ExternalGateRequired { .. }));
}

/// Exact model Pack identity: a `local_path` pointing to a *different*,
/// really-admitted pack than the one this run's `AgentIdentity` was
/// registered against must fail closed, never silently execute against
/// the wrong model.
#[test]
fn execution_against_mismatched_pack_directory_fails_closed() {
    let mut h = Harness::setup("model-execute-mismatch");
    let run_id = running_run_on_onnx_pack(&mut h);
    // A second, real, admitted pack -- but not the one this identity is
    // bound to.
    h.install_fixture_pack();

    let err = h
        .call(
            Capability::AgentRunExecute,
            RequestBody::AgentRunExecute {
                run_id,
                local_path: fixture_pack().display().to_string(),
                max_tokens: 4,
                synthetic_only: true,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::DigestMismatch));
}

#[test]
fn execution_on_a_non_running_run_is_refused_closed() {
    let mut h = Harness::setup("model-execute-not-running");
    let project_id = h.project("agent-project");
    let pack_id = h.install_onnx_fixture_pack();
    let source_id = h.source_record(b"synthetic clinical note");
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), source_id.as_str());
    // Pending, never started.
    let run_id = h.create_run(
        project_id,
        agent_id,
        context_id,
        "alice visited clinic today",
    );

    let err = h
        .call(
            Capability::AgentRunExecute,
            RequestBody::AgentRunExecute {
                run_id,
                local_path: onnx_fixture_pack().display().to_string(),
                max_tokens: 4,
                synthetic_only: true,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Conflict { .. }));
}

// ---------------------------------------------------------------------------
// T077-08: RunReceipt + run history
// ---------------------------------------------------------------------------

#[test]
fn completed_run_receipt_threads_real_tool_invocation_ids() {
    let mut h = Harness::setup("receipt-tool-ids");
    let (run_id, source_id) =
        running_run_with_one_source(&mut h, vec![ToolKind::ReadContextArtifact], b"hello world");

    let invocation_id = match h
        .call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id: run_id.clone(),
                kind: ToolKind::ReadContextArtifact,
                arguments: serde_json::json!({ "object_id": source_id.as_str() }),
            },
        )
        .expect("invoke tool")
    {
        ResponseBody::MedAgentToolInvocation { invocation, .. } => invocation.header.id,
        other => panic!("{other:?}"),
    };

    // Pending(1) -> Running(2, start) -> after one tool call, still
    // Running(2) (tool invocation does not itself bump run revision).
    let resp = h
        .call(
            Capability::AgentRunComplete,
            RequestBody::AgentRunComplete {
                run_id: run_id.clone(),
                expected_revision: 2,
            },
        )
        .expect("complete run");
    match resp {
        ResponseBody::MedAgentRunTerminal { run, receipt } => {
            assert_eq!(run.status, AgentRunState::Completed);
            assert!(receipt.failure_reason.is_none());
            assert_eq!(receipt.tool_invocation_ids, vec![invocation_id]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn fail_agent_run_records_failure_reason_in_receipt() {
    let mut h = Harness::setup("receipt-fail");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");
    let run_id = h.create_run(project_id, agent_id, context_id, "go");
    h.start_run(run_id.clone());

    let resp = h
        .call(
            Capability::AgentRunFail,
            RequestBody::AgentRunFail {
                run_id: run_id.clone(),
                expected_revision: 2,
                failure_reason: "pack runtime evaluation failed: token bound exceeded".to_owned(),
            },
        )
        .expect("fail run");
    match resp {
        ResponseBody::MedAgentRunTerminal { run, receipt } => {
            assert_eq!(run.status, AgentRunState::Failed);
            assert_eq!(receipt.final_state, AgentRunState::Failed);
            assert_eq!(
                receipt.failure_reason.as_deref(),
                Some("pack runtime evaluation failed: token bound exceeded")
            );
        }
        other => panic!("{other:?}"),
    }
}

/// Frozen table: `Pending -> Completed` is not an edge (only
/// `Running -> Completed`). Completing a never-started run must fail
/// closed, not silently promote it.
#[test]
fn completing_a_pending_run_is_rejected() {
    let mut h = Harness::setup("complete-pending-rejected");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");
    let run_id = h.create_run(project_id, agent_id, context_id, "go");

    let err = h
        .call(
            Capability::AgentRunComplete,
            RequestBody::AgentRunComplete {
                run_id,
                expected_revision: 1,
            },
        )
        .unwrap_err();
    assert!(matches!(err, AuthorityError::Conflict { .. }));
}

#[test]
fn run_history_is_filterable_by_status() {
    let mut h = Harness::setup("history-filter");
    let project_id = h.project("agent-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(project_id.clone(), pack_id);
    let context_id = h.create_context(project_id.clone(), "artifact-1");

    let pending_run = h.create_run(
        project_id.clone(),
        agent_id.clone(),
        context_id.clone(),
        "stays pending",
    );
    let cancelled_run = h.create_run(
        project_id.clone(),
        agent_id.clone(),
        context_id.clone(),
        "gets cancelled",
    );
    h.call(
        Capability::AgentRunCancel,
        RequestBody::AgentRunCancel {
            run_id: cancelled_run.clone(),
            expected_revision: 1,
        },
    )
    .expect("cancel");

    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunList {
                project_id: project_id.clone(),
                agent_id: None,
                status: Some(AgentRunState::Cancelled),
                limit: None,
            },
        )
        .expect("list cancelled");
    match resp {
        ResponseBody::MedAgentRunList { runs } => {
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].header.id, cancelled_run);
        }
        other => panic!("{other:?}"),
    }

    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunList {
                project_id: project_id.clone(),
                agent_id: None,
                status: Some(AgentRunState::Pending),
                limit: None,
            },
        )
        .expect("list pending");
    match resp {
        ResponseBody::MedAgentRunList { runs } => {
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].header.id, pending_run);
        }
        other => panic!("{other:?}"),
    }

    let resp = h
        .call(
            Capability::AgentRunRead,
            RequestBody::AgentRunList {
                project_id,
                agent_id: None,
                status: None,
                limit: None,
            },
        )
        .expect("list all");
    match resp {
        ResponseBody::MedAgentRunList { runs } => assert_eq!(runs.len(), 2),
        other => panic!("{other:?}"),
    }
}

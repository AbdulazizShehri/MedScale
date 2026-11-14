//! Spec 078 Model Fleet + Compare Core authority integration tests.
//!
//! Every mutation travels request -> session -> realm/scope -> `ModelFleet`
//! authority -> Spec 077 public reads -> validation -> revision -> typed
//! result through `CoreFacade::dispatch`. Synthetic data only.
//!
//! T078-03 scope: `AgentLane` + `LanePolicy` create/get/list/retire,
//! `security.md` T2 (subset-only policy), T5 (stale revision) and T7
//! (bounded text), plus reopen durability.
//!
//! T078-04 scope: `FleetRun` create/dispatch/execute-lane/cancel over real
//! local ONNX execution, the frozen state machine incl. partial failure and
//! cancel semantics, dispatch refusals, T3 lane isolation, the lane-policy
//! guard on Spec 077's direct tool path, the real-PHI gate, and reopen.
//!
//! T078-05/06 scope: comparison over two real, independent ONNX lane runs,
//! partial-failure exclusion, refusal for non-comparable fleets, append-only
//! recompute with no side effect on lane runs, and report history.

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::medagent::{
    AgentProposal, AgentRun, AgentRunState, ToolInvocation, ToolInvocationStatus, ToolKind,
};
use medscale_contracts::model_fleet::{
    AgentLane, AgentLaneStatus, ComparisonObservation, ComparisonObservationKind, ComparisonReport,
    FleetRun, FleetRunState, LaneRunRef, ROLE_LABEL_MAX_CHARS,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding};
use medscale_core::CoreFacade;

const REALM: &str = "realm-a";
const SCOPE: &str = "scope-a";
const VAULT: &str = "vault-1";

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-078c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/008-local-ai-capability-fabric/fixtures/pack-fixture-ner-v0")
}

fn request(
    id: OpaqueId,
    scope: &str,
    capability: Capability,
    body: RequestBody,
) -> AuthorityRequest {
    AuthorityRequest::new(
        id,
        VaultId::new(VAULT),
        RealmId::new(REALM),
        AuthorityScopeId::new(scope),
        capability,
        body,
    )
}

/// Single-actor Core harness over one synthetic vault directory.
struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    dir: PathBuf,
    next_req: u64,
}

impl Harness {
    fn setup(name: &str) -> Self {
        Self::open_at(tmp_dir(name))
    }

    /// Opens a fresh `CoreFacade` (a new process, as far as Core state is
    /// concerned) over an existing vault directory.
    fn open_at(dir: PathBuf) -> Self {
        let mut h = Harness {
            facade: CoreFacade::new(),
            session: OpaqueId::new("pending"),
            dir,
            next_req: 0,
        };
        let lease = h
            .facade
            .dispatch(request(
                OpaqueId::new("req-lease"),
                SCOPE,
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new("actor-a"),
                    holder_id_hint: Some(OpaqueId::new("actor-a")),
                },
            ))
            .result
            .expect("lease");
        let ResponseBody::Lease { holder_id, .. } = lease else {
            panic!("{lease:?}");
        };
        let session = h
            .facade
            .dispatch(request(
                OpaqueId::new("req-session"),
                SCOPE,
                Capability::OpenSession,
                RequestBody::OpenSession {
                    holder_id,
                    granted: Capability::operator_grants(),
                    ttl_ticks: 1_000_000,
                },
            ))
            .result
            .expect("session");
        let ResponseBody::Session { session_id, .. } = session else {
            panic!("{session:?}");
        };
        h.session = session_id;
        let vault_root = h.dir.to_str().unwrap().to_owned();
        h.call(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault { vault_root },
        )
        .expect("vault");
        h
    }

    fn reopen(self) -> Self {
        let dir = self.dir.clone();
        drop(self);
        Self::open_at(dir)
    }

    fn call_in(
        &mut self,
        scope: &str,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next_req += 1;
        let mut req = request(
            OpaqueId::new(format!("req-{}", self.next_req)),
            scope,
            capability,
            body,
        );
        req.session_id = Some(self.session.clone());
        self.facade.dispatch(req).result
    }

    fn call(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.call_in(SCOPE, capability, body)
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

    fn install_fixture_pack(&mut self) -> OpaqueId {
        let local_path = fixture_pack().display().to_string();
        match self
            .call(
                Capability::PacksInstallLocal,
                RequestBody::PacksInstallLocal { local_path },
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

    fn register_identity(
        &mut self,
        project_id: &OpaqueId,
        pack_id: &OpaqueId,
        granted_tool_kinds: Vec<ToolKind>,
    ) -> OpaqueId {
        match self
            .call(
                Capability::AgentIdentityRegister,
                RequestBody::AgentIdentityRegister {
                    project_id: project_id.clone(),
                    pack_id: pack_id.clone(),
                    display_name: "Lane Agent".to_owned(),
                    granted_tool_kinds,
                },
            )
            .expect("register identity")
        {
            ResponseBody::MedAgentIdentity { identity, .. } => identity.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn create_context(&mut self, project_id: &OpaqueId, artifact_ids: &[&str]) -> OpaqueId {
        let selected_artifacts = artifact_ids
            .iter()
            .map(|id| ArtifactDescriptor {
                object_id: OpaqueId::new(*id),
                kind: ArtifactKind::SourceRecord,
                binding: ArtifactVersionBinding::IdentityOnly,
            })
            .collect();
        match self
            .call(
                Capability::ContextManifestCreate,
                RequestBody::ContextManifestCreate {
                    project_id: project_id.clone(),
                    selected_artifacts,
                },
            )
            .expect("create context")
        {
            ResponseBody::MedAgentContextManifest { manifest, .. } => manifest.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn create_lane(
        &mut self,
        project_id: &OpaqueId,
        agent_id: &OpaqueId,
        context_id: &OpaqueId,
        granted_tool_kinds: Option<Vec<ToolKind>>,
        context_artifact_ids: Option<Vec<OpaqueId>>,
    ) -> Result<AgentLane, AuthorityError> {
        self.call(
            Capability::AgentLaneCreate,
            RequestBody::AgentLaneCreate {
                project_id: project_id.clone(),
                agent_identity_id: agent_id.clone(),
                context_manifest_id: context_id.clone(),
                role_label: "reviewer".to_owned(),
                granted_tool_kinds,
                context_artifact_ids,
            },
        )
        .map(|resp| match resp {
            ResponseBody::ModelFleetLane { lane } => *lane,
            other => panic!("{other:?}"),
        })
    }

    fn list_lanes(
        &mut self,
        project_id: &OpaqueId,
        status: Option<AgentLaneStatus>,
    ) -> Vec<AgentLane> {
        match self
            .call(
                Capability::AgentLaneRead,
                RequestBody::AgentLaneList {
                    project_id: project_id.clone(),
                    status,
                    limit: None,
                },
            )
            .expect("list lanes")
        {
            ResponseBody::ModelFleetLaneList { lanes } => lanes,
            other => panic!("{other:?}"),
        }
    }

    fn get_lane(&mut self, lane_id: &OpaqueId) -> Result<AgentLane, AuthorityError> {
        self.call(
            Capability::AgentLaneRead,
            RequestBody::AgentLaneGet {
                lane_id: lane_id.clone(),
            },
        )
        .map(|resp| match resp {
            ResponseBody::ModelFleetLane { lane } => *lane,
            other => panic!("{other:?}"),
        })
    }

    fn retire_lane(
        &mut self,
        lane_id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<AgentLane, AuthorityError> {
        self.call(
            Capability::AgentLaneRetire,
            RequestBody::AgentLaneRetire {
                lane_id: lane_id.clone(),
                expected_revision,
            },
        )
        .map(|resp| match resp {
            ResponseBody::ModelFleetLane { lane } => *lane,
            other => panic!("{other:?}"),
        })
    }
}

/// A Project with one identity (granting read + search) and one context
/// manifest selecting two artifacts.
struct LaneFixture {
    project_id: OpaqueId,
    pack_id: OpaqueId,
    agent_id: OpaqueId,
    context_id: OpaqueId,
}

fn lane_fixture(h: &mut Harness) -> LaneFixture {
    let project_id = h.project("fleet-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(
        &project_id,
        &pack_id,
        vec![
            ToolKind::ReadContextArtifact,
            ToolKind::SearchContextArtifacts,
        ],
    );
    let context_id = h.create_context(&project_id, &["artifact-1", "artifact-2"]);
    LaneFixture {
        project_id,
        pack_id,
        agent_id,
        context_id,
    }
}

fn assert_invalid(result: Result<AgentLane, AuthorityError>, why: &str) {
    match result {
        Err(AuthorityError::InvalidArgument { .. }) => {}
        other => panic!("{why}: expected InvalidArgument, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// T078-03: lane vertical slice + reopen durability
// ---------------------------------------------------------------------------

#[test]
fn lane_create_get_list_retire_roundtrip_and_survives_reopen() {
    let mut h = Harness::setup("lane-lifecycle");
    let f = lane_fixture(&mut h);

    let narrowed = h
        .create_lane(
            &f.project_id,
            &f.agent_id,
            &f.context_id,
            Some(vec![ToolKind::ReadContextArtifact]),
            Some(vec![OpaqueId::new("artifact-1")]),
        )
        .expect("narrowed lane");
    assert_eq!(narrowed.status, AgentLaneStatus::Active);
    assert_eq!(narrowed.revision, 1);
    assert_eq!(narrowed.agent_identity_id, f.agent_id);
    assert_eq!(narrowed.context_manifest_id, f.context_id);
    assert_eq!(narrowed.header.realm_id.as_opaque().as_str(), REALM);

    // `None` inherits the full underlying grant.
    let inherited = h
        .create_lane(&f.project_id, &f.agent_id, &f.context_id, None, None)
        .expect("inheriting lane");
    assert_ne!(inherited.header.id, narrowed.header.id);

    assert_eq!(h.get_lane(&narrowed.header.id).unwrap(), narrowed);
    assert_eq!(h.list_lanes(&f.project_id, None).len(), 2);

    let retired = h.retire_lane(&narrowed.header.id, 1).expect("retire");
    assert_eq!(retired.status, AgentLaneStatus::Retired);
    assert_eq!(retired.revision, 2);
    assert_eq!(
        retired.policy, narrowed.policy,
        "retire never rewrites policy"
    );

    let mut h = h.reopen();
    assert_eq!(h.get_lane(&narrowed.header.id).unwrap(), retired);
    assert_eq!(h.get_lane(&inherited.header.id).unwrap(), inherited);
    let active = h.list_lanes(&f.project_id, Some(AgentLaneStatus::Active));
    assert_eq!(active, vec![inherited.clone()]);
    let retired_list = h.list_lanes(&f.project_id, Some(AgentLaneStatus::Retired));
    assert_eq!(retired_list, vec![retired]);

    // Ids keep allocating from the durable sequence after reopen.
    let third = h
        .create_lane(&f.project_id, &f.agent_id, &f.context_id, None, None)
        .expect("lane after reopen");
    assert_ne!(third.header.id, narrowed.header.id);
    assert_ne!(third.header.id, inherited.header.id);
}

// ---------------------------------------------------------------------------
// security.md T2: policy narrows, never widens
// ---------------------------------------------------------------------------

#[test]
fn lane_policy_naming_an_ungranted_tool_kind_is_refused_before_any_write() {
    let mut h = Harness::setup("lane-t2-tool");
    let project_id = h.project("fleet-project");
    let pack_id = h.install_fixture_pack();
    let agent_id = h.register_identity(&project_id, &pack_id, vec![ToolKind::ReadContextArtifact]);
    let context_id = h.create_context(&project_id, &["artifact-1"]);

    assert_invalid(
        h.create_lane(
            &project_id,
            &agent_id,
            &context_id,
            Some(vec![ToolKind::SearchContextArtifacts]),
            None,
        ),
        "a tool kind the identity does not grant",
    );
    assert_invalid(
        h.create_lane(
            &project_id,
            &agent_id,
            &context_id,
            Some(vec![
                ToolKind::ReadContextArtifact,
                ToolKind::SearchContextArtifacts,
            ]),
            None,
        ),
        "a superset of the identity's grant",
    );
    assert!(h.list_lanes(&project_id, None).is_empty());
}

#[test]
fn lane_policy_naming_an_out_of_manifest_artifact_is_refused_before_any_write() {
    let mut h = Harness::setup("lane-t2-context");
    let f = lane_fixture(&mut h);
    assert_invalid(
        h.create_lane(
            &f.project_id,
            &f.agent_id,
            &f.context_id,
            None,
            Some(vec![OpaqueId::new("artifact-outside")]),
        ),
        "an artifact outside the bound context manifest",
    );
    assert_invalid(
        h.create_lane(
            &f.project_id,
            &f.agent_id,
            &f.context_id,
            None,
            Some(vec![
                OpaqueId::new("artifact-1"),
                OpaqueId::new("artifact-outside"),
            ]),
        ),
        "a partial overlap with the bound context manifest",
    );
    assert_invalid(
        h.create_lane(
            &f.project_id,
            &f.agent_id,
            &f.context_id,
            Some(vec![]),
            None,
        ),
        "an empty tool-kind subset",
    );
    assert_invalid(
        h.create_lane(
            &f.project_id,
            &f.agent_id,
            &f.context_id,
            None,
            Some(vec![]),
        ),
        "an empty artifact subset",
    );
    assert!(h.list_lanes(&f.project_id, None).is_empty());
}

#[test]
fn lane_binding_to_revoked_missing_or_foreign_objects_is_refused() {
    let mut h = Harness::setup("lane-binding");
    let f = lane_fixture(&mut h);
    let other_project = h.project("other-project");
    let foreign_agent = h.register_identity(
        &other_project,
        &f.pack_id,
        vec![ToolKind::ReadContextArtifact],
    );
    let foreign_context = h.create_context(&other_project, &["artifact-9"]);

    assert_invalid(
        h.create_lane(&f.project_id, &foreign_agent, &f.context_id, None, None),
        "an identity from another Project",
    );
    assert_invalid(
        h.create_lane(&f.project_id, &f.agent_id, &foreign_context, None, None),
        "a context manifest from another Project",
    );
    assert!(matches!(
        h.create_lane(
            &f.project_id,
            &OpaqueId::new("agent-missing"),
            &f.context_id,
            None,
            None
        ),
        Err(AuthorityError::NotFound)
    ));
    assert!(matches!(
        h.create_lane(
            &f.project_id,
            &f.agent_id,
            &OpaqueId::new("ctx-missing"),
            None,
            None
        ),
        Err(AuthorityError::NotFound)
    ));

    h.call(
        Capability::AgentIdentityRevoke,
        RequestBody::AgentIdentityRevoke {
            agent_id: f.agent_id.clone(),
            expected_revision: 1,
        },
    )
    .expect("revoke identity");
    assert_invalid(
        h.create_lane(&f.project_id, &f.agent_id, &f.context_id, None, None),
        "a revoked identity",
    );
    assert!(h.list_lanes(&f.project_id, None).is_empty());
}

// ---------------------------------------------------------------------------
// security.md T5 / T7 and scope
// ---------------------------------------------------------------------------

#[test]
fn lane_retire_is_stale_revision_safe_and_not_repeatable() {
    let mut h = Harness::setup("lane-cas");
    let f = lane_fixture(&mut h);
    let lane = h
        .create_lane(&f.project_id, &f.agent_id, &f.context_id, None, None)
        .unwrap();
    assert!(matches!(
        h.retire_lane(&lane.header.id, 7),
        Err(AuthorityError::Conflict { .. })
    ));
    h.retire_lane(&lane.header.id, 1).unwrap();
    assert!(matches!(
        h.retire_lane(&lane.header.id, 1),
        Err(AuthorityError::Conflict { .. })
    ));
    assert!(matches!(
        h.retire_lane(&lane.header.id, 2),
        Err(AuthorityError::Conflict { .. })
    ));
    assert!(matches!(
        h.retire_lane(&OpaqueId::new("lane-missing"), 1),
        Err(AuthorityError::NotFound)
    ));
}

#[test]
fn lane_role_label_is_bounded_and_stored_as_inert_text() {
    let mut h = Harness::setup("lane-t7");
    let f = lane_fixture(&mut h);
    for role_label in [
        String::new(),
        "   ".to_owned(),
        "x".repeat(ROLE_LABEL_MAX_CHARS + 1),
    ] {
        let result = h
            .call(
                Capability::AgentLaneCreate,
                RequestBody::AgentLaneCreate {
                    project_id: f.project_id.clone(),
                    agent_identity_id: f.agent_id.clone(),
                    context_manifest_id: f.context_id.clone(),
                    role_label: role_label.clone(),
                    granted_tool_kinds: None,
                    context_artifact_ids: None,
                },
            )
            .map(|_| ());
        assert!(
            matches!(result, Err(AuthorityError::InvalidArgument { .. })),
            "role label of {} chars must be refused, got {result:?}",
            role_label.chars().count()
        );
    }
    let hostile = "'); DROP TABLE model_fleet_lanes; -- <script>\u{202e}";
    let lane = match h
        .call(
            Capability::AgentLaneCreate,
            RequestBody::AgentLaneCreate {
                project_id: f.project_id.clone(),
                agent_identity_id: f.agent_id.clone(),
                context_manifest_id: f.context_id.clone(),
                role_label: hostile.to_owned(),
                granted_tool_kinds: None,
                context_artifact_ids: None,
            },
        )
        .expect("hostile-but-bounded label is stored verbatim")
    {
        ResponseBody::ModelFleetLane { lane } => *lane,
        other => panic!("{other:?}"),
    };
    assert_eq!(h.get_lane(&lane.header.id).unwrap().role_label, hostile);
    assert_eq!(h.list_lanes(&f.project_id, None).len(), 1);
}

#[test]
fn lane_reads_are_scope_checked() {
    let mut h = Harness::setup("lane-scope");
    let f = lane_fixture(&mut h);
    let lane = h
        .create_lane(&f.project_id, &f.agent_id, &f.context_id, None, None)
        .unwrap();
    let foreign = h.call_in(
        "scope-b",
        Capability::AgentLaneRead,
        RequestBody::AgentLaneGet {
            lane_id: lane.header.id.clone(),
        },
    );
    assert!(
        foreign.is_err(),
        "a lane must not be readable from another scope: {foreign:?}"
    );
    let foreign_list = h.call_in(
        "scope-b",
        Capability::AgentLaneRead,
        RequestBody::AgentLaneList {
            project_id: f.project_id.clone(),
            status: None,
            limit: None,
        },
    );
    assert!(
        foreign_list.is_err(),
        "a Project's lanes must not be listable from another scope: {foreign_list:?}"
    );
}

#[test]
fn lane_capabilities_are_distinct_from_spec_077_capabilities() {
    let mut h = Harness::setup("lane-capability");
    let f = lane_fixture(&mut h);
    // A request whose declared capability does not match its body is
    // refused by the facade's capability matrix before any authority code.
    let result = h.call(
        Capability::AgentRunCreate,
        RequestBody::AgentLaneCreate {
            project_id: f.project_id.clone(),
            agent_identity_id: f.agent_id.clone(),
            context_manifest_id: f.context_id.clone(),
            role_label: "reviewer".to_owned(),
            granted_tool_kinds: None,
            context_artifact_ids: None,
        },
    );
    assert!(
        result.is_err(),
        "capability/body mismatch must be refused: {result:?}"
    );
    assert!(h.list_lanes(&f.project_id, None).is_empty());
}

// ===========================================================================
// T078-04: FleetRun lifecycle
// ===========================================================================

/// The Spec 069 pack admitted for real local model execution.
fn onnx_fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../evidence/069-real-local-model-runtime-hf-pack-path/fixtures/pack-tiny-token-classifier-v0",
    )
}

impl Harness {
    fn install_onnx_pack(&mut self) -> OpaqueId {
        let local_path = onnx_fixture_pack().display().to_string();
        match self
            .call(
                Capability::PacksInstallLocal,
                RequestBody::PacksInstallLocal { local_path },
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

    fn fleet(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<(FleetRun, Vec<LaneRunRef>), AuthorityError> {
        self.call(capability, body).map(|resp| match resp {
            ResponseBody::ModelFleetRun { run, lane_run_refs } => (*run, lane_run_refs),
            other => panic!("{other:?}"),
        })
    }

    fn create_fleet(&mut self, project_id: &OpaqueId) -> FleetRun {
        self.fleet(
            Capability::FleetRunCreate,
            RequestBody::FleetRunCreate {
                project_id: project_id.clone(),
                task_prompt: "Summarize the bound context for review.".to_owned(),
            },
        )
        .expect("create fleet")
        .0
    }

    fn dispatch(
        &mut self,
        fleet_id: &OpaqueId,
        expected_revision: u64,
        lane_ids: &[&OpaqueId],
    ) -> Result<(FleetRun, Vec<LaneRunRef>), AuthorityError> {
        self.fleet(
            Capability::FleetRunDispatch,
            RequestBody::FleetRunDispatch {
                fleet_run_id: fleet_id.clone(),
                expected_revision,
                lane_ids: lane_ids.iter().map(|id| (*id).clone()).collect(),
            },
        )
    }

    fn get_fleet(&mut self, fleet_id: &OpaqueId) -> (FleetRun, Vec<LaneRunRef>) {
        self.fleet(
            Capability::FleetRunRead,
            RequestBody::FleetRunGet {
                fleet_run_id: fleet_id.clone(),
            },
        )
        .expect("get fleet")
    }

    fn cancel_fleet(
        &mut self,
        fleet_id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<(FleetRun, Vec<LaneRunRef>), AuthorityError> {
        self.fleet(
            Capability::FleetRunCancel,
            RequestBody::FleetRunCancel {
                fleet_run_id: fleet_id.clone(),
                expected_revision,
            },
        )
    }

    fn execute_lane(
        &mut self,
        fleet_id: &OpaqueId,
        lane_id: &OpaqueId,
        local_path: &Path,
        synthetic_only: bool,
    ) -> Result<(FleetRun, AgentRun, Option<AgentProposal>), AuthorityError> {
        self.call(
            Capability::FleetRunExecuteLane,
            RequestBody::FleetRunExecuteLane {
                fleet_run_id: fleet_id.clone(),
                lane_id: lane_id.clone(),
                local_path: local_path.display().to_string(),
                max_tokens: 4,
                synthetic_only,
            },
        )
        .map(|resp| match resp {
            ResponseBody::ModelFleetLaneExecuted {
                run,
                lane_run,
                proposal,
            } => (*run, *lane_run, proposal.map(|p| *p)),
            other => panic!("{other:?}"),
        })
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

    fn agent_run(&mut self, run_id: &OpaqueId) -> AgentRun {
        match self
            .call(
                Capability::AgentRunRead,
                RequestBody::AgentRunGet {
                    run_id: run_id.clone(),
                },
            )
            .expect("agent run")
        {
            ResponseBody::MedAgentRun { run } => *run,
            other => panic!("{other:?}"),
        }
    }

    fn invoke_tool(
        &mut self,
        run_id: &OpaqueId,
        kind: ToolKind,
        arguments: serde_json::Value,
    ) -> Result<ToolInvocation, AuthorityError> {
        self.call(
            Capability::AgentToolInvoke,
            RequestBody::AgentToolInvoke {
                run_id: run_id.clone(),
                kind,
                arguments,
            },
        )
        .map(|resp| match resp {
            ResponseBody::MedAgentToolInvocation { invocation, .. } => *invocation,
            other => panic!("{other:?}"),
        })
    }
}

/// Two lanes over two distinct identities, both bound to the one real
/// admitted ONNX Pack, with disjoint context manifests and deliberately
/// different policies (`security.md` T4 closure-fixture requirement).
struct FleetFixture {
    project_id: OpaqueId,
    pack_id: OpaqueId,
    lane_a: AgentLane,
    lane_b: AgentLane,
    a1: OpaqueId,
    a2: OpaqueId,
    b1: OpaqueId,
}

fn fleet_fixture(h: &mut Harness) -> FleetFixture {
    let project_id = h.project("fleet-project");
    let pack_id = h.install_onnx_pack();
    let agent_a = h.register_identity(
        &project_id,
        &pack_id,
        vec![
            ToolKind::ReadContextArtifact,
            ToolKind::SearchContextArtifacts,
        ],
    );
    let agent_b = h.register_identity(&project_id, &pack_id, vec![ToolKind::ReadContextArtifact]);
    let a1 = h.source_record(b"synthetic note a1: lane a may read this");
    let a2 = h.source_record(b"synthetic note a2: outside lane a policy");
    let b1 = h.source_record(b"synthetic note b1: lane b context only");
    let ctx_a = h.create_context(&project_id, &[a1.as_str(), a2.as_str()]);
    let ctx_b = h.create_context(&project_id, &[b1.as_str()]);
    let lane_a = h
        .create_lane(
            &project_id,
            &agent_a,
            &ctx_a,
            Some(vec![ToolKind::ReadContextArtifact]),
            Some(vec![a1.clone()]),
        )
        .expect("lane a");
    let lane_b = h
        .create_lane(&project_id, &agent_b, &ctx_b, None, None)
        .expect("lane b");
    FleetFixture {
        project_id,
        pack_id,
        lane_a,
        lane_b,
        a1,
        a2,
        b1,
    }
}

fn ref_for<'a>(refs: &'a [LaneRunRef], lane: &AgentLane) -> &'a LaneRunRef {
    refs.iter()
        .find(|r| r.agent_lane_id == lane.header.id)
        .expect("lane is bound")
}

#[test]
fn dispatch_binds_and_starts_one_independent_run_per_lane() {
    let mut h = Harness::setup("fleet-dispatch");
    let f = fleet_fixture(&mut h);
    let fleet = h.create_fleet(&f.project_id);
    assert_eq!((fleet.status, fleet.revision), (FleetRunState::Pending, 1));
    assert!(h.get_fleet(&fleet.header.id).1.is_empty());

    let (running, refs) = h
        .dispatch(
            &fleet.header.id,
            1,
            &[&f.lane_a.header.id, &f.lane_b.header.id],
        )
        .expect("dispatch");
    assert_eq!(
        (running.status, running.revision),
        (FleetRunState::Running, 2)
    );
    assert_eq!(refs.len(), 2);
    let ref_a = ref_for(&refs, &f.lane_a);
    let ref_b = ref_for(&refs, &f.lane_b);
    assert_ne!(ref_a.agent_run_id, ref_b.agent_run_id, "security.md T4");

    for (lane, lane_ref) in [(&f.lane_a, ref_a), (&f.lane_b, ref_b)] {
        let run = h.agent_run(&lane_ref.agent_run_id);
        // security.md T2: the dispatched run binds exactly the lane's own
        // identity/context and the fleet's prompt, never a substitute.
        assert_eq!(run.agent_identity_id, lane.agent_identity_id);
        assert_eq!(run.context_manifest_id, lane.context_manifest_id);
        assert_eq!(run.prompt, running.task_prompt);
        assert_eq!(run.project_id, f.project_id);
        // Started only after binding: Running with its prompt turn.
        assert_eq!(run.status, AgentRunState::Running);
    }
    assert_eq!(h.get_fleet(&fleet.header.id), (running, refs));
}

#[test]
fn fleet_reaches_completed_and_partially_failed_from_real_lane_outcomes() {
    let mut h = Harness::setup("fleet-outcomes");
    let f = fleet_fixture(&mut h);

    // Every lane completes -> Completed.
    let fleet = h.create_fleet(&f.project_id);
    h.dispatch(
        &fleet.header.id,
        1,
        &[&f.lane_a.header.id, &f.lane_b.header.id],
    )
    .unwrap();
    let (after_a, run_a, proposal_a) = h
        .execute_lane(
            &fleet.header.id,
            &f.lane_a.header.id,
            &onnx_fixture_pack(),
            true,
        )
        .expect("execute lane a");
    assert_eq!(
        after_a.status,
        FleetRunState::Running,
        "lane b still in flight"
    );
    assert_eq!(run_a.status, AgentRunState::Completed);
    let proposal_a = proposal_a.expect("a completed lane carries its proposal");
    assert_eq!(proposal_a.run_id, run_a.header.id);
    let (after_b, run_b, proposal_b) = h
        .execute_lane(
            &fleet.header.id,
            &f.lane_b.header.id,
            &onnx_fixture_pack(),
            true,
        )
        .expect("execute lane b");
    assert_eq!(after_b.status, FleetRunState::Completed);
    assert_eq!(run_b.status, AgentRunState::Completed);
    assert_ne!(proposal_b.unwrap().proposal_id, proposal_a.proposal_id);

    // One lane fails (wrong Pack directory) -> PartiallyFailed.
    let fleet = h.create_fleet(&f.project_id);
    h.dispatch(
        &fleet.header.id,
        1,
        &[&f.lane_a.header.id, &f.lane_b.header.id],
    )
    .unwrap();
    h.execute_lane(
        &fleet.header.id,
        &f.lane_a.header.id,
        &onnx_fixture_pack(),
        true,
    )
    .unwrap();
    let (partial, failed_run, none) = h
        .execute_lane(&fleet.header.id, &f.lane_b.header.id, &fixture_pack(), true)
        .expect("a failed execution closes the lane, it is not an error");
    assert_eq!(partial.status, FleetRunState::PartiallyFailed);
    assert_eq!(failed_run.status, AgentRunState::Failed);
    assert!(none.is_none());

    // Every lane fails -> Failed.
    let fleet = h.create_fleet(&f.project_id);
    h.dispatch(
        &fleet.header.id,
        1,
        &[&f.lane_a.header.id, &f.lane_b.header.id],
    )
    .unwrap();
    h.execute_lane(&fleet.header.id, &f.lane_a.header.id, &fixture_pack(), true)
        .unwrap();
    let (failed, _, _) = h
        .execute_lane(&fleet.header.id, &f.lane_b.header.id, &fixture_pack(), true)
        .unwrap();
    assert_eq!(failed.status, FleetRunState::Failed);

    // A terminal fleet accepts no further lane execution or cancel.
    assert!(matches!(
        h.execute_lane(
            &fleet.header.id,
            &f.lane_a.header.id,
            &onnx_fixture_pack(),
            true
        ),
        Err(AuthorityError::Conflict { .. })
    ));
    assert!(matches!(
        h.cancel_fleet(&fleet.header.id, failed.revision),
        Err(AuthorityError::Conflict { .. })
    ));
}

#[test]
fn fleet_cancel_semantics_follow_the_frozen_contract() {
    let mut h = Harness::setup("fleet-cancel");
    let f = fleet_fixture(&mut h);

    // Pending -> Cancelled directly, nothing dispatched.
    let fleet = h.create_fleet(&f.project_id);
    let (cancelled, refs) = h.cancel_fleet(&fleet.header.id, 1).unwrap();
    assert_eq!(cancelled.status, FleetRunState::Cancelled);
    assert!(refs.is_empty());
    assert!(matches!(
        h.dispatch(
            &fleet.header.id,
            cancelled.revision,
            &[&f.lane_a.header.id, &f.lane_b.header.id]
        ),
        Err(AuthorityError::Conflict { .. })
    ));

    // Running, no lane terminal yet -> every lane Cancelled, fleet Cancelled.
    let fleet = h.create_fleet(&f.project_id);
    let (running, _) = h
        .dispatch(
            &fleet.header.id,
            1,
            &[&f.lane_a.header.id, &f.lane_b.header.id],
        )
        .unwrap();
    let (cancelled, refs) = h.cancel_fleet(&fleet.header.id, running.revision).unwrap();
    assert_eq!(cancelled.status, FleetRunState::Cancelled);
    for lane_ref in &refs {
        assert_eq!(
            h.agent_run(&lane_ref.agent_run_id).status,
            AgentRunState::Cancelled
        );
    }

    // security.md T6: a lane that completed first keeps Completed; only the
    // in-flight lane is cancelled; the fleet takes the aggregate, never
    // `Cancelled` over a completed lane.
    let fleet = h.create_fleet(&f.project_id);
    h.dispatch(
        &fleet.header.id,
        1,
        &[&f.lane_a.header.id, &f.lane_b.header.id],
    )
    .unwrap();
    let (after_a, _, _) = h
        .execute_lane(
            &fleet.header.id,
            &f.lane_a.header.id,
            &onnx_fixture_pack(),
            true,
        )
        .unwrap();
    let (landed, refs) = h.cancel_fleet(&fleet.header.id, after_a.revision).unwrap();
    assert_eq!(landed.status, FleetRunState::PartiallyFailed);
    assert_eq!(
        h.agent_run(&ref_for(&refs, &f.lane_a).agent_run_id).status,
        AgentRunState::Completed
    );
    assert_eq!(
        h.agent_run(&ref_for(&refs, &f.lane_b).agent_run_id).status,
        AgentRunState::Cancelled
    );

    // A lane closed directly through Spec 077 before the fleet cancel is
    // left as it is (exactly one terminal state per lane).
    let fleet = h.create_fleet(&f.project_id);
    let (running, refs) = h
        .dispatch(
            &fleet.header.id,
            1,
            &[&f.lane_a.header.id, &f.lane_b.header.id],
        )
        .unwrap();
    let run_b = ref_for(&refs, &f.lane_b).agent_run_id.clone();
    let rev_b = h.agent_run(&run_b).revision;
    h.call(
        Capability::AgentRunFail,
        RequestBody::AgentRunFail {
            run_id: run_b.clone(),
            expected_revision: rev_b,
            failure_reason: "closed outside the fleet".to_owned(),
        },
    )
    .expect("direct Spec 077 fail");
    let (landed, _) = h.cancel_fleet(&fleet.header.id, running.revision).unwrap();
    assert_eq!(h.agent_run(&run_b).status, AgentRunState::Failed);
    assert_eq!(
        landed.status,
        FleetRunState::Failed,
        "cancelled + failed lanes, none completed"
    );

    // Stale revision is a conflict.
    let fleet = h.create_fleet(&f.project_id);
    assert!(matches!(
        h.cancel_fleet(&fleet.header.id, 9),
        Err(AuthorityError::Conflict { .. })
    ));
}

#[test]
fn dispatch_refusals_write_nothing() {
    let mut h = Harness::setup("fleet-refusals");
    let f = fleet_fixture(&mut h);
    let fleet = h.create_fleet(&f.project_id);
    let id = fleet.header.id.clone();
    let other_project = h.project("other-project");
    let foreign_agent = h.register_identity(
        &other_project,
        &f.pack_id,
        vec![ToolKind::ReadContextArtifact],
    );
    let foreign_ctx = h.create_context(&other_project, &["artifact-x"]);
    let foreign_lane = h
        .create_lane(&other_project, &foreign_agent, &foreign_ctx, None, None)
        .unwrap();
    let retired = h
        .create_lane(
            &f.project_id,
            &f.lane_b.agent_identity_id,
            &f.lane_b.context_manifest_id,
            None,
            None,
        )
        .unwrap();
    h.retire_lane(&retired.header.id, 1).unwrap();

    let a = &f.lane_a.header.id;
    let b = &f.lane_b.header.id;
    for (why, lanes, rev) in [
        ("a single lane", vec![a], 1),
        ("a duplicated lane", vec![a, a], 1),
        ("a retired lane", vec![a, &retired.header.id], 1),
        (
            "a lane of another project",
            vec![a, &foreign_lane.header.id],
            1,
        ),
        ("a stale revision", vec![a, b], 7),
    ] {
        let result = h.dispatch(&id, rev, &lanes);
        assert!(
            matches!(
                result,
                Err(AuthorityError::InvalidArgument { .. } | AuthorityError::Conflict { .. })
            ),
            "{why} must be refused, got {result:?}"
        );
        let (unchanged, refs) = h.get_fleet(&id);
        assert_eq!(
            (unchanged.status, unchanged.revision),
            (FleetRunState::Pending, 1),
            "{why}"
        );
        assert!(refs.is_empty(), "{why}: nothing may be bound");
    }
    let too_many: Vec<OpaqueId> = (0..=medscale_contracts::model_fleet::FLEET_RUN_MAX_LANES)
        .map(|n| OpaqueId::new(format!("lane-{n}")))
        .collect();
    let too_many: Vec<&OpaqueId> = too_many.iter().collect();
    assert!(matches!(
        h.dispatch(&id, 1, &too_many),
        Err(AuthorityError::InvalidArgument { .. })
    ));

    // A lane whose identity was revoked after lane creation is refused at
    // dispatch: the lane's bindings are re-resolved, never cached.
    h.call(
        Capability::AgentIdentityRevoke,
        RequestBody::AgentIdentityRevoke {
            agent_id: f.lane_b.agent_identity_id.clone(),
            expected_revision: 1,
        },
    )
    .unwrap();
    assert!(matches!(
        h.dispatch(&id, 1, &[a, b]),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    assert!(h.get_fleet(&id).1.is_empty());

    // A lane whose Pack is not admitted in this Core process is refused
    // before the fleet leaves Pending (fresh Core after reopen: no Packs).
    let mut h = h.reopen();
    assert!(matches!(
        h.dispatch(&id, 1, &[a, b]),
        Err(AuthorityError::InvalidArgument { .. })
    ));
    let (unchanged, refs) = h.get_fleet(&id);
    assert_eq!(
        (unchanged.status, unchanged.revision),
        (FleetRunState::Pending, 1)
    );
    assert!(refs.is_empty());
    h.install_onnx_pack();

    // Dispatching twice is a conflict (the second sees Running).
    let lane_c = h
        .create_lane(
            &f.project_id,
            &f.lane_a.agent_identity_id,
            &f.lane_a.context_manifest_id,
            None,
            None,
        )
        .unwrap();
    let (running, _) = h.dispatch(&id, 1, &[a, &lane_c.header.id]).unwrap();
    assert!(matches!(
        h.dispatch(&id, running.revision, &[a, &lane_c.header.id]),
        Err(AuthorityError::Conflict { .. })
    ));
    assert_eq!(h.get_fleet(&id).1.len(), 2);
}

#[test]
fn lanes_cannot_reach_each_others_context_and_lane_policy_holds_on_the_direct_tool_path() {
    let mut h = Harness::setup("fleet-t3");
    let f = fleet_fixture(&mut h);
    let fleet = h.create_fleet(&f.project_id);
    let (_, refs) = h
        .dispatch(
            &fleet.header.id,
            1,
            &[&f.lane_a.header.id, &f.lane_b.header.id],
        )
        .unwrap();
    let run_a = ref_for(&refs, &f.lane_a).agent_run_id.clone();
    let run_b = ref_for(&refs, &f.lane_b).agent_run_id.clone();

    // security.md T3: lane b's run cannot resolve lane a's artifact; Spec
    // 077's own context check refuses and records it.
    let refused = h
        .invoke_tool(
            &run_b,
            ToolKind::ReadContextArtifact,
            serde_json::json!({ "object_id": f.a1.as_str() }),
        )
        .expect("Spec 077 records the refusal");
    assert_eq!(refused.status, ToolInvocationStatus::Refused);

    // security.md T2 on the direct path: lane a's policy narrows both the
    // tool kinds (read only) and the artifacts (a1 only).
    for (why, kind, arguments) in [
        (
            "a sibling lane's artifact",
            ToolKind::ReadContextArtifact,
            serde_json::json!({ "object_id": f.b1.as_str() }),
        ),
        (
            "an in-manifest artifact the lane policy excludes",
            ToolKind::ReadContextArtifact,
            serde_json::json!({ "object_id": f.a2.as_str() }),
        ),
        (
            "a tool kind the lane policy excludes",
            ToolKind::SearchContextArtifacts,
            serde_json::json!({ "query": "anything" }),
        ),
        (
            "a malformed argument shape",
            ToolKind::ReadContextArtifact,
            serde_json::json!({ "object": f.a1.as_str() }),
        ),
    ] {
        let result = h.invoke_tool(&run_a, kind, arguments);
        assert!(
            matches!(result, Err(AuthorityError::Unauthorized)),
            "{why} must be refused by the lane guard, got {result:?}"
        );
    }
    // Within the lane policy the call reaches Spec 077 and executes.
    let executed = h
        .invoke_tool(
            &run_a,
            ToolKind::ReadContextArtifact,
            serde_json::json!({ "object_id": f.a1.as_str() }),
        )
        .expect("reaches Spec 077");
    assert_eq!(executed.status, ToolInvocationStatus::Executed);
    assert_eq!(executed.run_id, run_a);
    // Lane b (no narrowing) reads its own artifact normally.
    let own = h
        .invoke_tool(
            &run_b,
            ToolKind::ReadContextArtifact,
            serde_json::json!({ "object_id": f.b1.as_str() }),
        )
        .expect("lane b own read");
    assert_eq!(own.status, ToolInvocationStatus::Executed);
}

#[test]
fn real_phi_gate_and_foreign_lanes_leave_the_lane_run_untouched() {
    let mut h = Harness::setup("fleet-gate");
    let f = fleet_fixture(&mut h);
    let fleet = h.create_fleet(&f.project_id);
    let (running, refs) = h
        .dispatch(
            &fleet.header.id,
            1,
            &[&f.lane_a.header.id, &f.lane_b.header.id],
        )
        .unwrap();
    assert!(matches!(
        h.execute_lane(
            &fleet.header.id,
            &f.lane_a.header.id,
            &onnx_fixture_pack(),
            false
        ),
        Err(AuthorityError::ExternalGateRequired { .. })
    ));
    assert!(matches!(
        h.execute_lane(
            &fleet.header.id,
            &OpaqueId::new("lane-unbound"),
            &onnx_fixture_pack(),
            true
        ),
        Err(AuthorityError::NotFound)
    ));
    let run_a = h.agent_run(&ref_for(&refs, &f.lane_a).agent_run_id);
    assert_eq!(run_a.status, AgentRunState::Running);
    assert_eq!(h.get_fleet(&fleet.header.id).0, running);
}

#[test]
fn fleet_state_and_bindings_survive_reopen() {
    let mut h = Harness::setup("fleet-reopen");
    let f = fleet_fixture(&mut h);
    let fleet = h.create_fleet(&f.project_id);
    h.dispatch(
        &fleet.header.id,
        1,
        &[&f.lane_a.header.id, &f.lane_b.header.id],
    )
    .unwrap();
    h.execute_lane(
        &fleet.header.id,
        &f.lane_a.header.id,
        &onnx_fixture_pack(),
        true,
    )
    .unwrap();
    let before = h.get_fleet(&fleet.header.id);
    assert_eq!(before.0.status, FleetRunState::Running);

    let mut h = h.reopen();
    // The reopened Core has no admitted Packs in memory; re-admit the
    // same Pack so lane b can execute.
    h.install_onnx_pack();
    assert_eq!(h.get_fleet(&fleet.header.id), before);
    let (done, _, _) = h
        .execute_lane(
            &fleet.header.id,
            &f.lane_b.header.id,
            &onnx_fixture_pack(),
            true,
        )
        .expect("lane b after reopen");
    assert_eq!(done.status, FleetRunState::Completed);
    let listed = match h
        .call(
            Capability::FleetRunRead,
            RequestBody::FleetRunList {
                project_id: f.project_id.clone(),
                status: Some(FleetRunState::Completed),
                limit: None,
            },
        )
        .unwrap()
    {
        ResponseBody::ModelFleetRunList { runs } => runs,
        other => panic!("{other:?}"),
    };
    assert_eq!(listed, vec![done]);
}

// ===========================================================================
// T078-05 / T078-06: comparison engine + history
// ===========================================================================

impl Harness {
    fn compare(&mut self, fleet_id: &OpaqueId) -> Result<ComparisonReport, AuthorityError> {
        self.call(
            Capability::ComparisonCompute,
            RequestBody::ComparisonCompute {
                fleet_run_id: fleet_id.clone(),
            },
        )
        .map(|resp| match resp {
            ResponseBody::ModelFleetComparisonReport { report } => *report,
            other => panic!("{other:?}"),
        })
    }

    fn reports(&mut self, fleet_id: &OpaqueId) -> Vec<ComparisonReport> {
        match self
            .call(
                Capability::ComparisonRead,
                RequestBody::ComparisonReportList {
                    fleet_run_id: fleet_id.clone(),
                },
            )
            .expect("list reports")
        {
            ResponseBody::ModelFleetComparisonReportList { reports } => reports,
            other => panic!("{other:?}"),
        }
    }

    /// Dispatches `lanes`, executes every lane in `succeed` with the real
    /// ONNX Pack and every other lane with a mismatched Pack (a real
    /// execution failure), and returns the terminal fleet.
    fn run_fleet(
        &mut self,
        project_id: &OpaqueId,
        lanes: &[&AgentLane],
        succeed: &[&AgentLane],
    ) -> (FleetRun, Vec<LaneRunRef>) {
        let fleet = self.create_fleet(project_id);
        let ids: Vec<&OpaqueId> = lanes.iter().map(|l| &l.header.id).collect();
        self.dispatch(&fleet.header.id, 1, &ids).expect("dispatch");
        for lane in lanes {
            let pack = if succeed.iter().any(|s| s.header.id == lane.header.id) {
                onnx_fixture_pack()
            } else {
                fixture_pack()
            };
            self.execute_lane(&fleet.header.id, &lane.header.id, &pack, true)
                .expect("execute lane");
        }
        self.get_fleet(&fleet.header.id)
    }
}

fn of_kind(
    report: &ComparisonReport,
    kind: ComparisonObservationKind,
) -> Vec<&ComparisonObservation> {
    report
        .observations
        .iter()
        .filter(|o| o.kind == kind)
        .collect()
}

#[test]
fn two_real_lanes_produce_a_factual_grounded_report() {
    let mut h = Harness::setup("compare-real");
    let f = fleet_fixture(&mut h);
    let (fleet, refs) = h.run_fleet(
        &f.project_id,
        &[&f.lane_a, &f.lane_b],
        &[&f.lane_a, &f.lane_b],
    );
    assert_eq!(fleet.status, FleetRunState::Completed);
    let run_a = ref_for(&refs, &f.lane_a).agent_run_id.clone();
    let run_b = ref_for(&refs, &f.lane_b).agent_run_id.clone();
    assert_ne!(run_a, run_b, "two genuinely independent runs");

    let report = h.compare(&fleet.header.id).expect("compare");
    assert_eq!(report.fleet_run_id, fleet.header.id);
    assert_eq!(
        report.participating_lane_ids,
        vec![f.lane_a.header.id.clone(), f.lane_b.header.id.clone()]
    );
    assert!(report.excluded_lane_ids.is_empty());
    report.validate().expect("valid report");

    // Same prompt through the same deterministic Pack: one Agreement over
    // both lanes, and no Disagreement is invented.
    let agreement = of_kind(&report, ComparisonObservationKind::Agreement);
    assert_eq!(agreement.len(), 1);
    assert_eq!(agreement[0].participating_lane_ids.len(), 2);
    assert!(of_kind(&report, ComparisonObservationKind::Disagreement).is_empty());
    assert_eq!(
        of_kind(&report, ComparisonObservationKind::SchemaValidity).len(),
        2
    );
    assert_eq!(
        of_kind(&report, ComparisonObservationKind::ResourceRuntimeFact).len(),
        2
    );
    // Disjoint lane contexts: no evidence overlap.
    assert!(of_kind(&report, ComparisonObservationKind::EvidenceOverlap).is_empty());
    // Lane a's policy narrows its context to a1, but Spec 077 cites the
    // whole bound manifest (a1 + a2) as proposal evidence: reported as an
    // out-of-policy citation, never silently accepted.
    let unsupported = of_kind(&report, ComparisonObservationKind::UnsupportedClaim);
    assert!(
        unsupported.iter().any(
            |o| o.participating_lane_ids == vec![f.lane_a.header.id.clone()]
                && o.detail.contains(f.a2.as_str())
        ),
        "{unsupported:?}"
    );
    assert!(
        unsupported
            .iter()
            .all(|o| o.participating_lane_ids != vec![f.lane_b.header.id.clone()]),
        "lane b cites only its own context"
    );

    // Both lanes really executed the model (a ModelOutput turn each), and
    // every observation is grounded and names only participating lanes.
    for run_id in [&run_a, &run_b] {
        let turns = match h
            .call(
                Capability::AgentRunRead,
                RequestBody::AgentRunTurnList {
                    run_id: run_id.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::MedAgentTurnList { turns } => turns,
            other => panic!("{other:?}"),
        };
        assert!(turns.iter().any(|t| t.kind.as_str() == "model_output"));
    }
    for observation in &report.observations {
        assert!(!observation.evidence_refs.is_empty(), "{observation:?}");
        for lane_id in &observation.participating_lane_ids {
            assert!(report.participating_lane_ids.contains(lane_id));
        }
    }
    let serialized = serde_json::to_string(&report).unwrap().to_lowercase();
    for forbidden in ["score", "rank", "winner", "confidence", "best"] {
        assert!(
            !serialized.contains(forbidden),
            "report must carry no `{forbidden}`: {serialized}"
        );
    }
}

#[test]
fn partial_failure_reports_name_the_excluded_lane_and_bad_states_are_refused() {
    let mut h = Harness::setup("compare-partial");
    let f = fleet_fixture(&mut h);
    let (fleet, _) = h.run_fleet(&f.project_id, &[&f.lane_a, &f.lane_b], &[&f.lane_a]);
    assert_eq!(fleet.status, FleetRunState::PartiallyFailed);
    let report = h.compare(&fleet.header.id).expect("compare partial");
    assert_eq!(
        report.participating_lane_ids,
        vec![f.lane_a.header.id.clone()]
    );
    assert_eq!(report.excluded_lane_ids, vec![f.lane_b.header.id.clone()]);
    assert!(of_kind(&report, ComparisonObservationKind::Agreement).is_empty());
    for observation in &report.observations {
        assert!(
            !observation
                .participating_lane_ids
                .contains(&f.lane_b.header.id)
        );
    }

    // Failed, cancelled, pending and running fleets have nothing to compare.
    let (failed, _) = h.run_fleet(&f.project_id, &[&f.lane_a, &f.lane_b], &[]);
    assert_eq!(failed.status, FleetRunState::Failed);
    let pending = h.create_fleet(&f.project_id);
    let running = h.create_fleet(&f.project_id);
    h.dispatch(
        &running.header.id,
        1,
        &[&f.lane_a.header.id, &f.lane_b.header.id],
    )
    .unwrap();
    let cancelled = h.create_fleet(&f.project_id);
    h.cancel_fleet(&cancelled.header.id, 1).unwrap();
    for id in [
        &failed.header.id,
        &pending.header.id,
        &running.header.id,
        &cancelled.header.id,
    ] {
        assert!(
            matches!(h.compare(id), Err(AuthorityError::InvalidArgument { .. })),
            "fleet {} must not be comparable",
            id.as_str()
        );
        assert!(h.reports(id).is_empty());
    }
    assert!(matches!(
        h.compare(&OpaqueId::new("fleet-missing")),
        Err(AuthorityError::NotFound)
    ));
}

#[test]
fn recompute_appends_a_new_report_and_never_touches_lane_runs() {
    let mut h = Harness::setup("compare-history");
    let f = fleet_fixture(&mut h);
    let (fleet, refs) = h.run_fleet(
        &f.project_id,
        &[&f.lane_a, &f.lane_b],
        &[&f.lane_a, &f.lane_b],
    );
    let runs_before: Vec<AgentRun> = refs.iter().map(|r| h.agent_run(&r.agent_run_id)).collect();
    let fleet_before = h.get_fleet(&fleet.header.id);

    let first = h.compare(&fleet.header.id).unwrap();
    let second = h.compare(&fleet.header.id).unwrap();
    assert_ne!(first.header.id, second.header.id);
    assert_eq!(
        first.observations, second.observations,
        "same committed input, same facts"
    );
    assert_eq!(
        h.reports(&fleet.header.id),
        vec![first.clone(), second.clone()]
    );

    // security.md T1: comparison writes only its own report row.
    let runs_after: Vec<AgentRun> = refs.iter().map(|r| h.agent_run(&r.agent_run_id)).collect();
    assert_eq!(runs_before, runs_after);
    assert_eq!(h.get_fleet(&fleet.header.id), fleet_before);

    // History survives reopen, oldest first, exactly as written.
    let mut h = h.reopen();
    assert_eq!(h.reports(&fleet.header.id), vec![first, second]);
    let foreign = h.call_in(
        "scope-b",
        Capability::ComparisonRead,
        RequestBody::ComparisonReportList {
            fleet_run_id: fleet.header.id.clone(),
        },
    );
    assert!(foreign.is_err(), "reports are scope-checked: {foreign:?}");
}

// ---------------------------------------------------------------------------
// Spec 103: a model admitted from the catalog (verified Hub snapshot -> signed
// Pack -> PacksInstallLocal) runs as Model Fleet lanes like any other Pack.

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn git_blob_sha1(bytes: &[u8]) -> String {
    use sha1::Digest as _;
    let mut h = sha1::Sha1::new();
    h.update(format!("blob {}\0", bytes.len()).as_bytes());
    h.update(bytes);
    hex_digest(&h.finalize())
}

/// Builds a synthetic Hub snapshot around the Spec 069 fixture model plus a
/// catalog row bound to it, then a signed Pack via the Core catalog surface.
fn catalog_snapshot_pack(name: &str) -> PathBuf {
    use sha2::Digest as _;
    const CATALOG_COMMIT: &str = "ea920f36fadd7b45935247d639f0ffa1ef493b23";
    const HUB_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const REPO: &str = "Synthetic/Fleet-Fixture-onnx";
    let root = tmp_dir(name);
    let snapshot = root.join("snapshot");
    fs::create_dir_all(&snapshot).unwrap();
    let model = fs::read(onnx_fixture_pack().join("model.onnx")).unwrap();
    let tokenizer = fs::read(onnx_fixture_pack().join("tokenizer.json")).unwrap();
    let config = br#"{"id2label":{"0":"O","1":"ENTITY"}}"#.to_vec();
    fs::write(snapshot.join("model.onnx"), &model).unwrap();
    fs::write(snapshot.join("tokenizer.json"), &tokenizer).unwrap();
    fs::write(snapshot.join("config.json"), &config).unwrap();
    let metadata = format!(
        r#"{{"id":"{REPO}","sha":"{HUB_SHA}","lastModified":"2026-10-09T00:00:00.000Z","gated":false,"private":false,"siblings":[{{"rfilename":"config.json","size":{},"blobId":"{}"}},{{"rfilename":"model.onnx","size":{},"lfs":{{"sha256":"{}","size":{}}}}},{{"rfilename":"tokenizer.json","size":{},"blobId":"{}"}}]}}"#,
        config.len(),
        git_blob_sha1(&config),
        model.len(),
        hex_digest(&sha2::Sha256::digest(&model)),
        model.len(),
        tokenizer.len(),
        git_blob_sha1(&tokenizer),
    );
    fs::write(snapshot.join("metadata.json"), &metadata).unwrap();
    let parsed: medscale_pack::HfRepoMetadata = serde_json::from_str(&metadata).unwrap();
    let hash = medscale_pack::reproducibility_hash(&parsed);
    let catalog = root.join("models.jsonl");
    fs::write(
        &catalog,
        format!(
            r#"{{"repo_id":"{REPO}","family":"NER","task":"token-classification","languages":["en"],"formats":["onnx"],"license":"apache-2.0","reproducibility_hash":"{hash}"}}"#
        ),
    )
    .unwrap();
    let pack = root.join("pack");
    medscale_core::model_catalog::build_pack_from_snapshot(
        &catalog,
        "synthetic/catalog",
        CATALOG_COMMIT,
        &snapshot,
        "model.onnx",
        4,
        &pack,
    )
    .expect("verified snapshot builds a signed Pack");
    pack
}

#[test]
fn catalog_admitted_pack_runs_as_fleet_lanes() {
    let mut h = Harness::setup("fleet-catalog-pack");
    let pack_dir = catalog_snapshot_pack("catalog-pack");
    let pack_id = match h
        .call(
            Capability::PacksInstallLocal,
            RequestBody::PacksInstallLocal {
                local_path: pack_dir.display().to_string(),
            },
        )
        .expect("catalog pack install")
    {
        ResponseBody::PackAdmit { result } => {
            assert!(result.admitted, "catalog-built pack must admit: {result:?}");
            result.pack_id.expect("admitted pack carries a pack_id")
        }
        other => panic!("{other:?}"),
    };

    let project_id = h.project("fleet-catalog-project");
    let agent_a = h.register_identity(&project_id, &pack_id, vec![ToolKind::ReadContextArtifact]);
    let agent_b = h.register_identity(&project_id, &pack_id, vec![ToolKind::ReadContextArtifact]);
    let a1 = h.source_record(b"synthetic note: catalog lane a");
    let b1 = h.source_record(b"synthetic note: catalog lane b");
    let ctx_a = h.create_context(&project_id, &[a1.as_str()]);
    let ctx_b = h.create_context(&project_id, &[b1.as_str()]);
    let lane_a = h
        .create_lane(&project_id, &agent_a, &ctx_a, None, None)
        .expect("lane a");
    let lane_b = h
        .create_lane(&project_id, &agent_b, &ctx_b, None, None)
        .expect("lane b");

    let fleet = h.create_fleet(&project_id);
    h.dispatch(&fleet.header.id, 1, &[&lane_a.header.id, &lane_b.header.id])
        .expect("dispatch");
    let (_, run_a, proposal_a) = h
        .execute_lane(&fleet.header.id, &lane_a.header.id, &pack_dir, true)
        .expect("execute catalog lane a");
    assert_eq!(run_a.status, AgentRunState::Completed);
    assert!(
        proposal_a.is_some(),
        "a completed lane carries its proposal"
    );
    let (after_b, run_b, proposal_b) = h
        .execute_lane(&fleet.header.id, &lane_b.header.id, &pack_dir, true)
        .expect("execute catalog lane b");
    assert_eq!(run_b.status, AgentRunState::Completed);
    assert!(proposal_b.is_some());
    assert_eq!(after_b.status, FleetRunState::Completed);
}

/// Spec 104: one fleet compares lanes running on different local runtimes:
/// lane A on a tract ONNX Pack, lane B on a candle safetensors Pack. Each lane
/// runs its Pack's declared runtime; the comparison is factual, not a ranking.
#[test]
fn fleet_compares_lanes_on_different_local_runtimes() {
    let mut h = Harness::setup("fleet-mixed-runtime");
    let onnx_dir = catalog_snapshot_pack("mixed-onnx");
    let candle_dir = support::dir("mixed-candle");
    support::candle_pack(&candle_dir);
    let mut install = |dir: &Path| -> OpaqueId {
        match h
            .call(
                Capability::PacksInstallLocal,
                RequestBody::PacksInstallLocal {
                    local_path: dir.display().to_string(),
                },
            )
            .expect("pack install")
        {
            ResponseBody::PackAdmit { result } => {
                assert!(result.admitted, "{result:?}");
                result.pack_id.expect("admitted pack carries a pack_id")
            }
            other => panic!("{other:?}"),
        }
    };
    let onnx_pack = install(&onnx_dir);
    let candle_pack = install(&candle_dir);
    assert_ne!(onnx_pack, candle_pack);

    let project_id = h.project("fleet-mixed-project");
    let agent_a = h.register_identity(&project_id, &onnx_pack, vec![ToolKind::ReadContextArtifact]);
    let agent_b = h.register_identity(
        &project_id,
        &candle_pack,
        vec![ToolKind::ReadContextArtifact],
    );
    let a1 = h.source_record(b"synthetic note: onnx lane");
    let b1 = h.source_record(b"synthetic note: candle lane");
    let ctx_a = h.create_context(&project_id, &[a1.as_str()]);
    let ctx_b = h.create_context(&project_id, &[b1.as_str()]);
    let lane_a = h
        .create_lane(&project_id, &agent_a, &ctx_a, None, None)
        .expect("lane a");
    let lane_b = h
        .create_lane(&project_id, &agent_b, &ctx_b, None, None)
        .expect("lane b");
    let fleet = h.create_fleet(&project_id);
    h.dispatch(&fleet.header.id, 1, &[&lane_a.header.id, &lane_b.header.id])
        .expect("dispatch");
    let (_, run_a, proposal_a) = h
        .execute_lane(&fleet.header.id, &lane_a.header.id, &onnx_dir, true)
        .expect("onnx lane");
    assert_eq!(run_a.status, AgentRunState::Completed);
    assert!(proposal_a.is_some());
    let (after, run_b, proposal_b) = h
        .execute_lane(&fleet.header.id, &lane_b.header.id, &candle_dir, true)
        .expect("candle lane");
    assert_eq!(run_b.status, AgentRunState::Completed);
    assert!(proposal_b.is_some());
    assert_eq!(after.status, FleetRunState::Completed);

    let report = h
        .compare(&fleet.header.id)
        .expect("compare across runtimes");
    assert_eq!(report.fleet_run_id, fleet.header.id);
    assert_eq!(
        report.participating_lane_ids,
        vec![lane_a.header.id.clone(), lane_b.header.id.clone()]
    );
    assert!(report.excluded_lane_ids.is_empty());
    report.validate().expect("valid report");
}

#[test]
fn a_lane_cites_only_current_evidence_and_is_refused_when_none_remains() {
    // Clinical empty-evidence gate: a run re-resolves its context before the
    // model runs, cites only what is current, and refuses with nothing left.
    let mut h = Harness::setup("fleet-evidence-gate");
    let project_id = h.project("evidence-gate-project");
    let pack_id = h.install_onnx_pack();
    let agent_a = h.register_identity(&project_id, &pack_id, vec![ToolKind::ReadContextArtifact]);
    let agent_b = h.register_identity(&project_id, &pack_id, vec![ToolKind::ReadContextArtifact]);
    let real = h.source_record(b"synthetic note: still present");
    let mixed = h.create_context(&project_id, &[real.as_str(), "artifact-never-created"]);
    let empty = h.create_context(&project_id, &["artifact-never-created"]);
    let lane_mixed = h
        .create_lane(&project_id, &agent_a, &mixed, None, None)
        .expect("lane with mixed context");
    let lane_empty = h
        .create_lane(&project_id, &agent_b, &empty, None, None)
        .expect("lane with no current context");
    let fleet = h.create_fleet(&project_id);
    h.dispatch(
        &fleet.header.id,
        1,
        &[&lane_mixed.header.id, &lane_empty.header.id],
    )
    .unwrap();

    let (_, run, proposal) = h
        .execute_lane(
            &fleet.header.id,
            &lane_mixed.header.id,
            &onnx_fixture_pack(),
            true,
        )
        .expect("execute mixed lane");
    assert_eq!(run.status, AgentRunState::Completed);
    let proposal = proposal.expect("a completed lane carries its proposal");
    let ResponseBody::Object { value } = h
        .call(
            Capability::ReadObject,
            RequestBody::ReadObject {
                object_id: proposal.proposal_id,
            },
        )
        .expect("read proposal")
    else {
        panic!("expected the proposal object");
    };
    let stored = value.to_string();
    assert!(stored.contains(real.as_str()), "{stored}");
    assert!(
        !stored.contains("artifact-never-created"),
        "a missing artifact is not cited: {stored}"
    );

    let (_, refused, none) = h
        .execute_lane(
            &fleet.header.id,
            &lane_empty.header.id,
            &onnx_fixture_pack(),
            true,
        )
        .expect("a refused execution closes the lane, it is not an error");
    assert_eq!(refused.status, AgentRunState::Failed);
    assert!(none.is_none(), "no proposal without current evidence");
}

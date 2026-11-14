//! Spec 074 Core authority integration tests (074-C).
//!
//! Every mutation travels request -> session -> realm/scope -> authority ->
//! validation -> revision -> reference check -> transaction -> audit -> typed
//! result through `CoreFacade::dispatch`. Synthetic data only.

use std::fs;

use medscale_contracts::envelopes::{AuthorityError, AuthorityRequest, Capability, RequestBody};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::project_graph::{
    ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, GraphEndpoint, ProjectGraphPredicate,
};
use medscale_core::CoreFacade;

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-074c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn project_grants() -> Vec<Capability> {
    vec![
        Capability::OpenSyntheticVault,
        Capability::ProjectCreate,
        Capability::ProjectRead,
        Capability::ProjectUpdate,
        Capability::ProjectArchive,
        Capability::ExperimentCreate,
        Capability::ExperimentRead,
        Capability::ExperimentUpdate,
        Capability::ExperimentArchive,
        Capability::ProjectArtifactAttach,
        Capability::ProjectArtifactDetach,
        Capability::ProjectGraphRead,
        Capability::ProjectGraphMutate,
        Capability::CreateSourceRecord,
        Capability::CreateProposal,
    ]
}

struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    dir: std::path::PathBuf,
}

fn setup(name: &str) -> Harness {
    let dir = tmp_dir(name);
    let facade = CoreFacade::new();
    let holder = match facade
        .dispatch(base_req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("client-a"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease")
    {
        medscale_contracts::envelopes::ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };
    let session = match facade
        .dispatch(base_req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id: holder,
                granted: project_grants(),
                ttl_ticks: 1_000_000,
            },
        ))
        .result
        .expect("session")
    {
        medscale_contracts::envelopes::ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };
    let mut vault_req = base_req(
        Capability::OpenSyntheticVault,
        RequestBody::OpenSyntheticVault {
            vault_root: dir.to_str().unwrap().to_owned(),
        },
    );
    vault_req.session_id = Some(session.clone());
    facade.dispatch(vault_req).result.expect("vault");
    Harness {
        facade,
        session,
        dir,
    }
}

fn base_req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new("scope-a"),
        capability,
        body,
    )
}

impl Harness {
    fn call(&self, capability: Capability, body: RequestBody) -> AuthorityRequestResult {
        let mut request = base_req(capability, body);
        request.session_id = Some(self.session.clone());
        AuthorityRequestResult(self.facade.dispatch(request).result)
    }

    fn call_scoped(
        &self,
        capability: Capability,
        body: RequestBody,
        realm: &str,
        scope: &str,
    ) -> AuthorityRequestResult {
        let mut request = AuthorityRequest::new(
            OpaqueId::new("req"),
            VaultId::new("vault-1"),
            RealmId::new(realm),
            AuthorityScopeId::new(scope),
            capability,
            body,
        );
        request.session_id = Some(self.session.clone());
        AuthorityRequestResult(self.facade.dispatch(request).result)
    }

    fn create_source(&self, bytes: &[u8]) -> OpaqueId {
        match self
            .call(
                Capability::CreateSourceRecord,
                RequestBody::CreateSourceRecord {
                    media_type: "text/plain".to_owned(),
                    bytes: bytes.to_vec(),
                },
            )
            .0
            .expect("source")
        {
            medscale_contracts::envelopes::ResponseBody::Created { object_id } => object_id,
            other => panic!("{other:?}"),
        }
    }

    fn create_project(&self, name: &str) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: name.to_owned(),
                    description: None,
                },
            )
            .0
            .expect("project")
        {
            medscale_contracts::envelopes::ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn audit_count(&self) -> usize {
        let meta = medscale_storage::SqliteMetaStore::open_at(&self.dir.join("meta.sqlite3"))
            .expect("reopen meta");
        meta.list_authority_objects()
            .expect("objects")
            .into_iter()
            .filter(|o| o.object_class == "audit")
            .count()
    }

    fn snapshot_bytes(&self) -> Vec<u8> {
        let meta = medscale_storage::SqliteMetaStore::open_at(&self.dir.join("meta.sqlite3"))
            .expect("reopen meta");
        meta.snapshot_bytes().expect("snapshot")
    }
}

struct AuthorityRequestResult(Result<medscale_contracts::envelopes::ResponseBody, AuthorityError>);

fn descriptor(object_id: OpaqueId) -> ArtifactDescriptor {
    ArtifactDescriptor {
        object_id,
        kind: ArtifactKind::SourceRecord,
        binding: ArtifactVersionBinding::IdentityOnly,
    }
}

#[test]
fn project_lifecycle_end_to_end_with_revisions() {
    let h = setup("lifecycle");
    let id = h.create_project("study");
    // Get echoes revision 1.
    match h
        .call(
            Capability::ProjectRead,
            RequestBody::ProjectGet {
                project_id: id.clone(),
            },
        )
        .0
        .expect("get")
    {
        medscale_contracts::envelopes::ResponseBody::Project { project } => {
            assert_eq!(project.revision, 1);
        }
        other => panic!("{other:?}"),
    }
    // Update -> 2, archive -> 3, restore -> 4.
    for (body, capability, want) in [
        (
            RequestBody::ProjectUpdate {
                project_id: id.clone(),
                expected_revision: 1,
                name: Some("renamed".to_owned()),
                description: None,
            },
            Capability::ProjectUpdate,
            2,
        ),
        (
            RequestBody::ProjectArchive {
                project_id: id.clone(),
                expected_revision: 2,
            },
            Capability::ProjectArchive,
            3,
        ),
        (
            RequestBody::ProjectRestore {
                project_id: id.clone(),
                expected_revision: 3,
            },
            Capability::ProjectArchive,
            4,
        ),
    ] {
        match h.call(capability, body).0.expect("mutation") {
            medscale_contracts::envelopes::ResponseBody::Project { project } => {
                assert_eq!(project.revision, want);
            }
            other => panic!("{other:?}"),
        }
    }
    // Every mutation appended exactly one audit row.
    assert_eq!(h.audit_count(), 4);
}

#[test]
fn mutation_without_session_is_rejected() {
    let h = setup("nosession");
    let request = base_req(
        Capability::ProjectCreate,
        RequestBody::ProjectCreate {
            name: "x".to_owned(),
            description: None,
        },
    );
    assert!(matches!(
        h.facade.dispatch(request).result,
        Err(AuthorityError::SessionRequired)
    ));
}

#[test]
fn stale_write_conflicts_with_zero_write() {
    let h = setup("stale");
    let id = h.create_project("study");
    let err = h
        .call(
            Capability::ProjectUpdate,
            RequestBody::ProjectUpdate {
                project_id: id.clone(),
                expected_revision: 99,
                name: Some("evil".to_owned()),
                description: None,
            },
        )
        .0
        .expect_err("must conflict");
    assert!(matches!(err, AuthorityError::Conflict { .. }), "{err:?}");
    match h
        .call(
            Capability::ProjectRead,
            RequestBody::ProjectGet { project_id: id },
        )
        .0
        .expect("get")
    {
        medscale_contracts::envelopes::ResponseBody::Project { project } => {
            assert_eq!(project.revision, 1);
            assert_eq!(project.name, "study");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(h.audit_count(), 1);
}

#[test]
fn cross_scope_read_is_denied_without_leak() {
    let h = setup("xscope");
    let id = h.create_project("study");
    let err = h
        .call_scoped(
            Capability::ProjectRead,
            RequestBody::ProjectGet { project_id: id },
            "realm-b",
            "scope-b",
        )
        .0
        .expect_err("must deny");
    assert!(matches!(err, AuthorityError::WrongScope), "{err:?}");
}

#[test]
fn capability_gate_denies_ungranted_project_ops() {
    let dir = tmp_dir("grants");
    let facade = CoreFacade::new();
    let holder = match facade
        .dispatch(base_req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("client-a"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease")
    {
        medscale_contracts::envelopes::ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };
    let session = match facade
        .dispatch(base_req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id: holder.clone(),
                granted: vec![Capability::Ping, Capability::OpenSyntheticVault],
                ttl_ticks: 1000,
            },
        ))
        .result
        .expect("session")
    {
        medscale_contracts::envelopes::ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };
    let mut vault_req = base_req(
        Capability::OpenSyntheticVault,
        RequestBody::OpenSyntheticVault {
            vault_root: dir.to_str().unwrap().to_owned(),
        },
    );
    vault_req.session_id = Some(session);
    facade.dispatch(vault_req).result.expect("vault");
    // A session without the project grant is denied the operation.
    let ping_session = match facade
        .dispatch(base_req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id: holder,
                granted: vec![Capability::Ping],
                ttl_ticks: 1000,
            },
        ))
        .result
        .expect("ping session")
    {
        medscale_contracts::envelopes::ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };
    let mut request = base_req(
        Capability::ProjectCreate,
        RequestBody::ProjectCreate {
            name: "x".to_owned(),
            description: None,
        },
    );
    request.session_id = Some(ping_session);
    assert!(matches!(
        facade.dispatch(request).result,
        Err(AuthorityError::SessionDenied)
    ));
}

#[test]
fn project_op_without_vault_is_unavailable() {
    let facade = CoreFacade::new();
    let holder = match facade
        .dispatch(base_req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("client-a"),
                holder_id_hint: None,
            },
        ))
        .result
        .expect("lease")
    {
        medscale_contracts::envelopes::ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };
    let session = match facade
        .dispatch(base_req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id: holder,
                granted: project_grants(),
                ttl_ticks: 1000,
            },
        ))
        .result
        .expect("session")
    {
        medscale_contracts::envelopes::ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };
    let mut request = base_req(
        Capability::ProjectCreate,
        RequestBody::ProjectCreate {
            name: "x".to_owned(),
            description: None,
        },
    );
    request.session_id = Some(session);
    assert!(matches!(
        facade.dispatch(request).result,
        Err(AuthorityError::VaultRequired)
    ));
}

#[test]
fn attach_admits_current_and_rejects_the_rest() {
    let h = setup("admit");
    let project = h.create_project("study");
    let source = h.create_source(b"hello");
    // Current target attaches.
    match h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: descriptor(source.clone()),
            },
        )
        .0
        .expect("attach")
    {
        medscale_contracts::envelopes::ResponseBody::ProjectRef { reference } => {
            assert_eq!(reference.revision, 1);
        }
        other => panic!("{other:?}"),
    }
    // Missing target is NotFound.
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: descriptor(OpaqueId::new("src-nope")),
            },
        )
        .0
        .expect_err("missing");
    assert!(matches!(err, AuthorityError::NotFound), "{err:?}");
    // Cross-scope target is denied.
    let other_scope_artifact = {
        let mut request = AuthorityRequest::new(
            OpaqueId::new("req"),
            VaultId::new("vault-1"),
            RealmId::new("realm-b"),
            AuthorityScopeId::new("scope-b"),
            Capability::CreateSourceRecord,
            RequestBody::CreateSourceRecord {
                media_type: "text/plain".to_owned(),
                bytes: b"other".to_vec(),
            },
        );
        request.session_id = Some(h.session.clone());
        match h.facade.dispatch(request).result.expect("source-b") {
            medscale_contracts::envelopes::ResponseBody::Created { object_id } => object_id,
            other => panic!("{other:?}"),
        }
    };
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: descriptor(other_scope_artifact),
            },
        )
        .0
        .expect_err("denied");
    assert!(matches!(err, AuthorityError::WrongScope), "{err:?}");
    // Stale digest binding is StaleReference.
    let stale = ArtifactDescriptor {
        object_id: source.clone(),
        kind: ArtifactKind::SourceRecord,
        binding: ArtifactVersionBinding::Digest(medscale_contracts::objects::DigestSha256::of(
            b"other",
        )),
    };
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: stale,
            },
        )
        .0
        .expect_err("stale");
    assert!(
        matches!(err, AuthorityError::StaleReference { .. }),
        "{err:?}"
    );
    // Evidence kinds have no Core owner: fail closed.
    let evidence = ArtifactDescriptor {
        object_id: source.clone(),
        kind: ArtifactKind::EvidenceDocument,
        binding: ArtifactVersionBinding::IdentityOnly,
    };
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: evidence,
            },
        )
        .0
        .expect_err("unsupported");
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "{err:?}"
    );
    // Missing pack reads as NotFound (Missing resolution).
    let pack = ArtifactDescriptor {
        object_id: OpaqueId::new("pack-nope"),
        kind: ArtifactKind::PackManifest,
        binding: ArtifactVersionBinding::IdentityOnly,
    };
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project,
                experiment_id: None,
                artifact: pack,
            },
        )
        .0
        .expect_err("pack missing");
    assert!(matches!(err, AuthorityError::NotFound), "{err:?}");
}

#[test]
fn attach_rejects_kind_mismatch_as_corrupt() {
    let h = setup("kindmismatch");
    let project = h.create_project("study");
    // Real proposal, but attached under the source kind: fail closed.
    let proposal = match h
        .call(
            Capability::CreateProposal,
            RequestBody::CreateProposal {
                subject_ref: None,
                claim_kind: "test".to_owned(),
                payload: serde_json::json!({}),
                evidence_refs: vec![],
            },
        )
        .0
        .expect("proposal")
    {
        medscale_contracts::envelopes::ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };
    let mismatched = ArtifactDescriptor {
        object_id: proposal,
        kind: ArtifactKind::SourceRecord,
        binding: ArtifactVersionBinding::IdentityOnly,
    };
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project,
                experiment_id: None,
                artifact: mismatched,
            },
        )
        .0
        .expect_err("kind mismatch");
    assert!(matches!(err, AuthorityError::Corrupt { .. }), "{err:?}");
}

#[test]
fn attach_rejects_experiment_from_another_project() {
    let h = setup("foreign-exp");
    let project = h.create_project("study");
    let other = h.create_project("other");
    let source = h.create_source(b"hello");
    let foreign = match h
        .call(
            Capability::ExperimentCreate,
            RequestBody::ExperimentCreate {
                project_id: other,
                name: "foreign".to_owned(),
                description: None,
            },
        )
        .0
        .expect("exp")
    {
        medscale_contracts::envelopes::ResponseBody::Experiment { experiment } => {
            experiment.header.id
        }
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project,
                experiment_id: Some(foreign),
                artifact: descriptor(source),
            },
        )
        .0
        .expect_err("foreign experiment");
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "{err:?}"
    );
}

#[test]
fn high_frequency_ops_return_receipts_without_audit_growth() {
    // Frozen performance contract: attach/detach/edge ops persist revisioned
    // receipts in sqlite and must NOT append per-op memory audit rows (which
    // would make every op O(full history) through the snapshot sync).
    let h = setup("receipts");
    let project = h.create_project("study");
    let source = h.create_source(b"hello");
    let reference = match h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: descriptor(source.clone()),
            },
        )
        .0
        .expect("attach")
    {
        medscale_contracts::envelopes::ResponseBody::ProjectRef { reference } => reference,
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::ProjectGraphMutate,
            RequestBody::GraphEdgeCreate {
                project_id: project.clone(),
                subject: GraphEndpoint::Artifact(descriptor(source)),
                predicate: ProjectGraphPredicate::References,
                object: GraphEndpoint::Artifact(descriptor(OpaqueId::new("src-y"))),
            },
        )
        .0
        .expect_err("missing endpoint");
    // src-y does not exist: endpoint admission fails, still no audit.
    assert!(
        matches!(err, medscale_contracts::envelopes::AuthorityError::NotFound),
        "{err:?}"
    );
    h.call(
        Capability::ProjectArtifactDetach,
        RequestBody::ProjectDetach {
            ref_id: reference.header.id,
            expected_revision: 1,
        },
    )
    .0
    .expect("detach");
    // Only the project-create lifecycle audit exists.
    assert_eq!(h.audit_count(), 1);
}

#[test]
fn high_frequency_ops_leave_memory_snapshot_identical() {
    // Locks RequestBody::preserves_memory_snapshot: attach/detach/edge ops
    // must not change one byte of the memory-derived authority snapshot,
    // which is what makes skipping the full-snapshot rewrite output-identical
    // (and per-op cost constant instead of O(full history)).
    //
    // `snapshot_bytes` serializes the DURABLE sqlite file, whose `edges` and
    // `refs` sections legitimately grow: the revisioned sqlite rows ARE the
    // durability receipts for these ops. So this lock compares every section
    // EXCEPT `edges`/`refs` for exact equality (memory-derived authority
    // state untouched, no audit growth), then pins the receipt rows exactly
    // (one detached ref and one removed edge at revision 2).
    let h = setup("snapshot-stable");
    let project = h.create_project("study");
    let first = h.create_source(b"one");
    let second = h.create_source(b"two");
    let before_raw = h.snapshot_bytes();
    let reference = match h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: descriptor(first.clone()),
            },
        )
        .0
        .expect("attach")
    {
        medscale_contracts::envelopes::ResponseBody::ProjectRef { reference } => reference,
        other => panic!("{other:?}"),
    };
    let edge = match h
        .call(
            Capability::ProjectGraphMutate,
            RequestBody::GraphEdgeCreate {
                project_id: project.clone(),
                subject: GraphEndpoint::Artifact(descriptor(first)),
                predicate: ProjectGraphPredicate::References,
                object: GraphEndpoint::Artifact(descriptor(second)),
            },
        )
        .0
        .expect("edge")
    {
        medscale_contracts::envelopes::ResponseBody::GraphEdge { edge } => edge,
        other => panic!("{other:?}"),
    };
    h.call(
        Capability::ProjectArtifactDetach,
        RequestBody::ProjectDetach {
            ref_id: reference.header.id.clone(),
            expected_revision: 1,
        },
    )
    .0
    .expect("detach");
    h.call(
        Capability::ProjectGraphMutate,
        RequestBody::GraphEdgeRemove {
            edge_id: edge.header.id.clone(),
            expected_revision: 1,
        },
    )
    .0
    .expect("remove");
    let before: serde_json::Value =
        serde_json::from_slice(&before_raw).expect("before snapshot is json");
    let after: serde_json::Value =
        serde_json::from_slice(&h.snapshot_bytes()).expect("after snapshot is json");
    // Receipt proof: exactly the detached ref and the removed edge, at
    // revision 2, carrying the ids from the op receipts.
    let refs = after
        .get("refs")
        .and_then(|v| v.as_array())
        .expect("refs section");
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(
        refs[0].get("status").and_then(|v| v.as_str()),
        Some("detached"),
        "{refs:?}"
    );
    assert_eq!(
        refs[0].get("revision").and_then(|v| v.as_u64()),
        Some(2),
        "{refs:?}"
    );
    assert_eq!(
        refs[0].pointer("/header/id").and_then(|v| v.as_str()),
        Some(reference.header.id.as_str()),
        "{refs:?}"
    );
    let edges = after
        .get("edges")
        .and_then(|v| v.as_array())
        .expect("edges section");
    assert_eq!(edges.len(), 1, "{edges:?}");
    assert_eq!(
        edges[0].get("status").and_then(|v| v.as_str()),
        Some("removed"),
        "{edges:?}"
    );
    assert_eq!(
        edges[0].get("revision").and_then(|v| v.as_u64()),
        Some(2),
        "{edges:?}"
    );
    assert_eq!(
        edges[0].pointer("/header/id").and_then(|v| v.as_str()),
        Some(edge.header.id.as_str()),
        "{edges:?}"
    );
    // Authority proof: strip the receipt sections and the durable id
    // sequences they allocate from (sqlite rows, not memory state);
    // everything else (memory-derived authority state: objects, projects,
    // sources, experiments, the `next_seq` counter) must be byte-identical
    // through all four high-frequency ops.
    let mut before_stripped = before;
    let mut after_stripped = after;
    for snapshot in [&mut before_stripped, &mut after_stripped] {
        let obj = snapshot.as_object_mut().expect("snapshot object");
        obj.remove("refs");
        obj.remove("edges");
        obj.remove("id_sequences");
    }
    assert_eq!(after_stripped, before_stripped);
}

#[test]
fn duplicate_attach_is_explicit_conflict() {
    let h = setup("dupattach");
    let project = h.create_project("study");
    let source = h.create_source(b"hello");
    h.call(
        Capability::ProjectArtifactAttach,
        RequestBody::ProjectAttach {
            project_id: project.clone(),
            experiment_id: None,
            artifact: descriptor(source.clone()),
        },
    )
    .0
    .expect("first");
    let err = h
        .call(
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project,
                experiment_id: None,
                artifact: descriptor(source),
            },
        )
        .0
        .expect_err("duplicate");
    assert!(matches!(err, AuthorityError::Conflict { .. }), "{err:?}");
}

#[test]
fn archive_freezes_project_mutations() {
    let h = setup("frozen");
    let project = h.create_project("study");
    let source = h.create_source(b"hello");
    h.call(
        Capability::ProjectArchive,
        RequestBody::ProjectArchive {
            project_id: project.clone(),
            expected_revision: 1,
        },
    )
    .0
    .expect("archive");
    for (capability, body) in [
        (
            Capability::ProjectUpdate,
            RequestBody::ProjectUpdate {
                project_id: project.clone(),
                expected_revision: 2,
                name: Some("late".to_owned()),
                description: None,
            },
        ),
        (
            Capability::ProjectArtifactAttach,
            RequestBody::ProjectAttach {
                project_id: project.clone(),
                experiment_id: None,
                artifact: descriptor(source),
            },
        ),
        (
            Capability::ProjectGraphMutate,
            RequestBody::GraphEdgeCreate {
                project_id: project.clone(),
                subject: GraphEndpoint::Artifact(descriptor(OpaqueId::new("src-1"))),
                predicate: ProjectGraphPredicate::References,
                object: GraphEndpoint::Artifact(descriptor(OpaqueId::new("src-2"))),
            },
        ),
    ] {
        let err = h.call(capability, body).0.expect_err("frozen");
        assert!(matches!(err, AuthorityError::Unauthorized), "{err:?}");
    }
}

#[test]
fn edge_guards_reject_self_and_foreign_endpoints() {
    let h = setup("edgeguards");
    let project = h.create_project("study");
    let source = h.create_source(b"hello");
    let endpoint = GraphEndpoint::Artifact(descriptor(source));
    // Self-edge denied.
    let err = h
        .call(
            Capability::ProjectGraphMutate,
            RequestBody::GraphEdgeCreate {
                project_id: project.clone(),
                subject: endpoint.clone(),
                predicate: ProjectGraphPredicate::References,
                object: endpoint,
            },
        )
        .0
        .expect_err("self edge");
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "{err:?}"
    );
    // Experiment endpoint from another project denied.
    let other = h.create_project("other");
    let foreign = match h
        .call(
            Capability::ExperimentCreate,
            RequestBody::ExperimentCreate {
                project_id: other,
                name: "foreign".to_owned(),
                description: None,
            },
        )
        .0
        .expect("exp")
    {
        medscale_contracts::envelopes::ResponseBody::Experiment { experiment } => {
            experiment.header.id
        }
        other => panic!("{other:?}"),
    };
    let err = h
        .call(
            Capability::ProjectGraphMutate,
            RequestBody::GraphEdgeCreate {
                project_id: project,
                subject: GraphEndpoint::Experiment(foreign),
                predicate: ProjectGraphPredicate::Contains,
                object: GraphEndpoint::Experiment(OpaqueId::new("exp-nope")),
            },
        )
        .0
        .expect_err("foreign");
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "{err:?}"
    );
}

#[test]
fn neighbors_reject_unbounded_queries() {
    let h = setup("bounds");
    let project = h.create_project("study");
    let err = h
        .call(
            Capability::ProjectGraphRead,
            RequestBody::GraphNeighbors {
                project_id: project,
                start: GraphEndpoint::Experiment(OpaqueId::new("exp-1")),
                predicates: None,
                direction: None,
                limit: Some(101),
                cursor: None,
            },
        )
        .0
        .expect_err("unbounded");
    assert!(
        matches!(err, AuthorityError::InvalidArgument { .. }),
        "{err:?}"
    );
}

#[test]
fn context_and_summary_report_authorized_state() {
    let h = setup("context");
    let project = h.create_project("study");
    let source = h.create_source(b"hello");
    let experiment = match h
        .call(
            Capability::ExperimentCreate,
            RequestBody::ExperimentCreate {
                project_id: project.clone(),
                name: "exp".to_owned(),
                description: None,
            },
        )
        .0
        .expect("exp")
    {
        medscale_contracts::envelopes::ResponseBody::Experiment { experiment } => experiment,
        other => panic!("{other:?}"),
    };
    h.call(
        Capability::ProjectArtifactAttach,
        RequestBody::ProjectAttach {
            project_id: project.clone(),
            experiment_id: Some(experiment.header.id.clone()),
            artifact: descriptor(source.clone()),
        },
    )
    .0
    .expect("attach");
    h.call(
        Capability::ProjectGraphMutate,
        RequestBody::GraphEdgeCreate {
            project_id: project.clone(),
            subject: GraphEndpoint::Experiment(experiment.header.id.clone()),
            predicate: ProjectGraphPredicate::ExperimentInput,
            object: GraphEndpoint::Artifact(descriptor(source)),
        },
    )
    .0
    .expect("edge");
    match h
        .call(
            Capability::ProjectRead,
            RequestBody::ProjectSummaryQuery {
                project_id: project.clone(),
            },
        )
        .0
        .expect("summary")
    {
        medscale_contracts::envelopes::ResponseBody::ProjectSummary { summary } => {
            assert_eq!(summary.experiment_count, 1);
            assert_eq!(summary.active_ref_count, 1);
            assert_eq!(summary.active_edge_count, 1);
        }
        other => panic!("{other:?}"),
    }
    match h
        .call(
            Capability::ProjectRead,
            RequestBody::ProjectContextResolve {
                project_id: project,
                experiment_id: Some(experiment.header.id),
                refs_limit: None,
                graph_limit: None,
            },
        )
        .0
        .expect("context")
    {
        medscale_contracts::envelopes::ResponseBody::ProjectContext { context } => {
            assert_eq!(context.authorized_artifact_refs.len(), 1);
            assert_eq!(context.graph_slice.len(), 1);
            assert!(context.experiment.is_some());
        }
        other => panic!("{other:?}"),
    }
}

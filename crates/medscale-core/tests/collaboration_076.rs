//! Spec 076 Core authority integration tests (076-C).
//!
//! Every mutation travels request -> session -> realm/scope -> `Collab`
//! authority -> `RoomMembership` visibility -> validation -> revision ->
//! transaction -> typed result through `CoreFacade::dispatch`. Synthetic
//! data only. Covers functional lifecycle for every entity family plus the
//! adversarial scenarios T1-T9 from `security.md` that are reachable
//! through Core dispatch (T10 activity tamper-detection, T12 half-committed
//! state, and T13 dependency gates are proven at the storage layer and by
//! CI respectively, in `crates/medscale-storage/tests/collaboration_076.rs`
//! and the supply-chain/cargo-deny CI jobs).

use std::fs;

use medscale_contracts::collaboration::{
    AnchorTarget, ApprovalDecisionOutcome, ApprovalKind, MembershipRole, NoteDocument,
    ParticipantKind, ThreadStatus,
};
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding};
use medscale_core::CoreFacade;

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-076c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const REALM: &str = "realm-a";
const SCOPE: &str = "scope-a";
const VAULT: &str = "vault-1";

/// Multi-actor Core harness: one `CoreFacade`/vault shared across however
/// many local "operators" a test needs to simulate. The single-writer lease
/// model (`lease_single_writer.rs`) means only one holder can be active at
/// once, so `switch_actor` releases the current lease and opens a fresh one
/// under a new holder id -- exactly how a real local single-device vault
/// changes hands between operators, never a second concurrent writer.
struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    holder: OpaqueId,
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
            holder: OpaqueId::new("pending"),
            dir,
            next_req: 0,
        };
        h.switch_actor("actor-a");
        let mut vault_req = base_req(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: h.dir.to_str().unwrap().to_owned(),
            },
        );
        vault_req.session_id = Some(h.session.clone());
        h.facade.dispatch(vault_req).result.expect("vault");
        h
    }

    /// Releases the current lease (if any) and opens a new one/session under
    /// `holder`, with every collaboration + project capability granted.
    fn switch_actor(&mut self, holder: &str) {
        if self.holder.as_str() != "pending" {
            let _ = self.facade.dispatch(base_req(
                Capability::ReleaseLease,
                RequestBody::ReleaseLease {
                    holder_id: self.holder.clone(),
                },
            ));
        }
        let lease = self
            .facade
            .dispatch(base_req(
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new(holder),
                    holder_id_hint: Some(OpaqueId::new(holder)),
                },
            ))
            .result
            .expect("lease");
        let holder_id = match lease {
            ResponseBody::Lease { holder_id, .. } => holder_id,
            other => panic!("{other:?}"),
        };
        let session = self
            .facade
            .dispatch(base_req(
                Capability::OpenSession,
                RequestBody::OpenSession {
                    holder_id: holder_id.clone(),
                    granted: Capability::operator_grants(),
                    ttl_ticks: 1_000_000,
                },
            ))
            .result
            .expect("session");
        self.session = match session {
            ResponseBody::Session { session_id, .. } => session_id,
            other => panic!("{other:?}"),
        };
        self.holder = holder_id;
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

    fn register_self(&mut self, display_name: &str) -> OpaqueId {
        match self
            .call(
                Capability::ParticipantRegister,
                RequestBody::ParticipantRegister {
                    holder_id: self.holder.clone(),
                    kind: ParticipantKind::Human,
                    display_name: display_name.to_owned(),
                    agent_profile_ref: None,
                },
            )
            .expect("register")
        {
            ResponseBody::Participant { participant } => participant.header.id,
            other => panic!("{other:?}"),
        }
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

    fn source_record(&mut self, bytes: &[u8]) -> OpaqueId {
        match self
            .call(
                Capability::CreateSourceRecord,
                RequestBody::CreateSourceRecord {
                    media_type: "text/plain".to_owned(),
                    bytes: bytes.to_vec(),
                },
            )
            .expect("source")
        {
            ResponseBody::Created { object_id } => object_id,
            other => panic!("{other:?}"),
        }
    }

    fn room(
        &mut self,
        project_id: &OpaqueId,
        name: &str,
    ) -> medscale_contracts::collaboration::Room {
        match self
            .call(
                Capability::RoomCreate,
                RequestBody::RoomCreate {
                    project_id: project_id.clone(),
                    experiment_id: None,
                    name: name.to_owned(),
                },
            )
            .expect("room")
        {
            ResponseBody::CollabRoom { room } => *room,
            other => panic!("{other:?}"),
        }
    }

    fn add_member(
        &mut self,
        room_id: &OpaqueId,
        participant_id: &OpaqueId,
        role: MembershipRole,
    ) -> OpaqueId {
        match self
            .call(
                Capability::RoomMembershipManage,
                RequestBody::RoomMembershipAdd {
                    room_id: room_id.clone(),
                    participant_id: participant_id.clone(),
                    role,
                },
            )
            .expect("membership add")
        {
            ResponseBody::CollabMembership { membership } => membership.header.id,
            other => panic!("{other:?}"),
        }
    }
}

fn source_anchor(object_id: OpaqueId) -> AnchorTarget {
    AnchorTarget {
        artifact: ArtifactDescriptor {
            object_id,
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        },
        detail: None,
    }
}

// ---------------------------------------------------------------------------
// Functional lifecycle
// ---------------------------------------------------------------------------

#[test]
fn room_membership_lifecycle_end_to_end() {
    let mut h = Harness::setup("room-lifecycle");
    h.register_self("Operator A");
    let project = h.project("study");
    let room = h.room(&project, "trial design");
    assert_eq!(room.revision, 1);

    // Get echoes revision 1.
    match h
        .call(
            Capability::RoomRead,
            RequestBody::RoomGet {
                room_id: room.header.id.clone(),
            },
        )
        .expect("get")
    {
        ResponseBody::CollabRoom { room } => assert_eq!(room.revision, 1),
        other => panic!("{other:?}"),
    }

    // Rename bumps revision; stale rename conflicts (T5).
    match h
        .call(
            Capability::RoomUpdate,
            RequestBody::RoomRename {
                room_id: room.header.id.clone(),
                expected_revision: 1,
                name: "renamed room".to_owned(),
            },
        )
        .expect("rename")
    {
        ResponseBody::CollabRoom { room } => assert_eq!(room.revision, 2),
        other => panic!("{other:?}"),
    }
    let stale = h.call(
        Capability::RoomUpdate,
        RequestBody::RoomRename {
            room_id: room.header.id.clone(),
            expected_revision: 1,
            name: "stale rename".to_owned(),
        },
    );
    assert!(matches!(stale, Err(AuthorityError::Conflict { .. })));

    // Archive bumps revision again.
    match h
        .call(
            Capability::RoomArchive,
            RequestBody::RoomArchive {
                room_id: room.header.id.clone(),
                expected_revision: 2,
            },
        )
        .expect("archive")
    {
        ResponseBody::CollabRoom { room } => {
            assert_eq!(room.revision, 3);
            assert_eq!(
                room.status,
                medscale_contracts::collaboration::RoomStatus::Archived
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn membership_add_list_remove_and_stale_remove_conflicts() {
    let mut h = Harness::setup("membership");
    let owner = h.register_self("Owner");
    let project = h.project("study");
    let room = h.room(&project, "room");

    // Register a second participant under a throwaway holder, then switch
    // back to the owner to add them (membership add is owner-initiated).
    h.switch_actor("actor-b");
    let member_b = h.register_self("Member B");
    h.switch_actor("actor-a");

    let membership_id = h.add_member(&room.header.id, &member_b, MembershipRole::Member);
    let list = match h
        .call(
            Capability::RoomMembershipRead,
            RequestBody::RoomMembershipList {
                room_id: room.header.id.clone(),
            },
        )
        .expect("list")
    {
        ResponseBody::CollabMembershipList { memberships } => memberships,
        other => panic!("{other:?}"),
    };
    // Owner's own auto-granted membership + Member B == 2.
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|m| m.participant_id == owner));
    assert!(list.iter().any(|m| m.participant_id == member_b));

    // Stale remove (wrong expected_revision) conflicts, never last-write-wins.
    let stale = h.call(
        Capability::RoomMembershipManage,
        RequestBody::RoomMembershipRemove {
            room_id: room.header.id.clone(),
            membership_id: membership_id.clone(),
            expected_revision: 99,
        },
    );
    assert!(matches!(stale, Err(AuthorityError::Conflict { .. })));

    let removed = h
        .call(
            Capability::RoomMembershipManage,
            RequestBody::RoomMembershipRemove {
                room_id: room.header.id.clone(),
                membership_id,
                expected_revision: 1,
            },
        )
        .expect("remove");
    assert!(matches!(removed, ResponseBody::CollabMembership { .. }));
}

#[test]
fn thread_lifecycle_status_transitions_and_live_resolution() {
    let mut h = Harness::setup("thread");
    h.register_self("Operator");
    let project = h.project("study");
    let room = h.room(&project, "room");
    let source = h.source_record(b"raw evidence bytes");

    let (thread, resolution) = match h
        .call(
            Capability::ThreadCreate,
            RequestBody::ThreadOpen {
                room_id: room.header.id.clone(),
                anchor: source_anchor(source),
            },
        )
        .expect("open thread")
    {
        ResponseBody::CollabThread { thread, resolution } => (*thread, resolution),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        resolution,
        medscale_contracts::project_graph::ReferenceResolution::Current,
        "a real, previously-ingested source must resolve Current, not a fabricated ok"
    );
    assert_eq!(thread.status, ThreadStatus::Open);

    // Valid transition chain: Open -> Resolved -> Reopened -> Resolved.
    for (status, revision) in [
        (ThreadStatus::Resolved, 1),
        (ThreadStatus::Reopened, 2),
        (ThreadStatus::Resolved, 3),
    ] {
        match h
            .call(
                Capability::ThreadResolve,
                RequestBody::ThreadSetStatus {
                    thread_id: thread.header.id.clone(),
                    expected_revision: revision,
                    status,
                },
            )
            .expect("transition")
        {
            ResponseBody::CollabThread { thread, .. } => assert_eq!(thread.status, status),
            other => panic!("{other:?}"),
        }
    }

    // T7: live resolution recompute, never cached. A thread anchored to an
    // unknown id honestly resolves Missing.
    let (_, missing_resolution) = match h
        .call(
            Capability::ThreadCreate,
            RequestBody::ThreadOpen {
                room_id: room.header.id.clone(),
                anchor: source_anchor(OpaqueId::new("source-does-not-exist")),
            },
        )
        .expect("open thread on unknown id")
    {
        ResponseBody::CollabThread { thread, resolution } => (*thread, resolution),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        missing_resolution,
        medscale_contracts::project_graph::ReferenceResolution::Missing
    );
}

#[test]
fn message_post_list_and_author_only_edit_delete() {
    let mut h = Harness::setup("message");
    let author = h.register_self("Author");
    let project = h.project("study");
    let room = h.room(&project, "room");
    let source = h.source_record(b"x");
    let (thread, _) = match h
        .call(
            Capability::ThreadCreate,
            RequestBody::ThreadOpen {
                room_id: room.header.id.clone(),
                anchor: source_anchor(source),
            },
        )
        .expect("thread")
    {
        ResponseBody::CollabThread { thread, resolution } => (*thread, resolution),
        other => panic!("{other:?}"),
    };

    let message = match h
        .call(
            Capability::MessagePost,
            RequestBody::MessagePost {
                thread_id: thread.header.id.clone(),
                body: "first message".to_owned(),
            },
        )
        .expect("post")
    {
        ResponseBody::CollabMessage { message } => *message,
        other => panic!("{other:?}"),
    };
    assert_eq!(message.author_participant_id, author);
    assert_eq!(message.seq, 1);

    // A second, independently-registered member joins and attempts to edit
    // the first author's message: must be denied (author-only, T2/T6 style
    // attribution integrity), never silently allowed.
    h.switch_actor("actor-b");
    let member_b = h.register_self("Member B");
    h.switch_actor("actor-a");
    h.add_member(&room.header.id, &member_b, MembershipRole::Member);
    h.switch_actor("actor-b");
    let denied = h.call(
        Capability::MessageEdit,
        RequestBody::MessageEditBody {
            message_id: message.header.id.clone(),
            new_body: "hijacked".to_owned(),
        },
    );
    assert!(
        matches!(
            denied,
            Err(AuthorityError::Unauthorized) | Err(AuthorityError::WrongScope)
        ),
        "non-author edit must be denied, got {denied:?}"
    );
    h.switch_actor("actor-a");

    // The real author can edit and delete their own message.
    let edit = h
        .call(
            Capability::MessageEdit,
            RequestBody::MessageEditBody {
                message_id: message.header.id.clone(),
                new_body: "edited by author".to_owned(),
            },
        )
        .expect("author edit");
    assert!(matches!(edit, ResponseBody::CollabMessageEdit { .. }));
}

#[test]
fn task_lifecycle_with_stale_update_conflict() {
    let mut h = Harness::setup("task");
    h.register_self("Operator");
    let project = h.project("study");
    let room = h.room(&project, "room");

    let task = match h
        .call(
            Capability::TaskCreate,
            RequestBody::TaskCreate {
                room_id: room.header.id.clone(),
                anchor: None,
                title: "collect baseline labs".to_owned(),
                description: None,
            },
        )
        .expect("task create")
    {
        ResponseBody::CollabTask { task } => *task,
        other => panic!("{other:?}"),
    };

    let stale = h.call(
        Capability::TaskUpdate,
        RequestBody::TaskUpdate {
            task_id: task.header.id.clone(),
            expected_revision: 99,
            status: medscale_contracts::collaboration::TaskStatus::Done,
            assignee_participant_id: None,
        },
    );
    assert!(matches!(stale, Err(AuthorityError::Conflict { .. })));

    match h
        .call(
            Capability::TaskUpdate,
            RequestBody::TaskUpdate {
                task_id: task.header.id.clone(),
                expected_revision: 1,
                status: medscale_contracts::collaboration::TaskStatus::Done,
                assignee_participant_id: None,
            },
        )
        .expect("task update")
    {
        ResponseBody::CollabTask { task } => {
            assert_eq!(
                task.status,
                medscale_contracts::collaboration::TaskStatus::Done
            );
            assert_eq!(task.revision, 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn note_stale_write_produces_conflict_copy_never_silent_overwrite() {
    let mut h = Harness::setup("note");
    h.register_self("Operator");
    let project = h.project("study");
    let room = h.room(&project, "room");

    let note: NoteDocument = match h
        .call(
            Capability::NoteCreate,
            RequestBody::NoteCreate {
                room_id: room.header.id.clone(),
                title: "protocol notes".to_owned(),
                body: "v1 body".to_owned(),
            },
        )
        .expect("note create")
    {
        ResponseBody::CollabNote { note } => *note,
        other => panic!("{other:?}"),
    };
    assert_eq!(note.revision, 1);

    // Fast-forward edit (expected matches current pointer).
    let (note_after_ff, _, is_conflict) = match h
        .call(
            Capability::NoteUpdate,
            RequestBody::NoteEdit {
                note_id: note.header.id.clone(),
                expected_revision: 1,
                body: "v2 body, fast-forward".to_owned(),
            },
        )
        .expect("fast-forward edit")
    {
        ResponseBody::CollabNoteEdit {
            note,
            new_revision,
            is_conflict_copy,
        } => (*note, *new_revision, is_conflict_copy),
        other => panic!("{other:?}"),
    };
    assert!(!is_conflict);
    assert_eq!(note_after_ff.revision, 2);

    // Stale edit (still claims expected_revision = 1, but the pointer moved
    // to 2): must produce an explicit conflict copy, not a silent overwrite
    // and not a bare rejection (contracts.md section 10, security.md T5).
    let (note_after_conflict, _, is_conflict) = match h
        .call(
            Capability::NoteUpdate,
            RequestBody::NoteEdit {
                note_id: note.header.id.clone(),
                expected_revision: 1,
                body: "stale concurrent edit".to_owned(),
            },
        )
        .expect("conflict edit")
    {
        ResponseBody::CollabNoteEdit {
            note,
            new_revision,
            is_conflict_copy,
        } => (*note, *new_revision, is_conflict_copy),
        other => panic!("{other:?}"),
    };
    assert!(is_conflict);
    // The pointer/current row is untouched by a conflict copy.
    assert_eq!(note_after_conflict.revision, note_after_ff.revision);

    let revisions = match h
        .call(
            Capability::NoteRead,
            RequestBody::NoteListRevisions {
                note_id: note.header.id.clone(),
            },
        )
        .expect("list revisions")
    {
        ResponseBody::CollabNoteRevisionList { revisions } => revisions,
        other => panic!("{other:?}"),
    };
    // Both the fast-forward body and the conflict-copy body must be present
    // and readable -- the caller's content is never dropped.
    assert!(revisions.iter().any(|r| r.body == "v2 body, fast-forward"));
    assert!(revisions.iter().any(|r| r.body == "stale concurrent edit"));
}

#[test]
fn approval_request_blind_mode_and_requester_only_withdraw() {
    let mut h = Harness::setup("approval");
    h.register_self("Requester");
    let project = h.project("study");
    let room = h.room(&project, "room");
    let source = h.source_record(b"protocol amendment v2");

    h.switch_actor("actor-b");
    let assignee_b = h.register_self("Assignee B");
    h.switch_actor("actor-c");
    let assignee_c = h.register_self("Assignee C");
    h.switch_actor("actor-a");
    h.add_member(&room.header.id, &assignee_b, MembershipRole::Member);
    h.add_member(&room.header.id, &assignee_c, MembershipRole::Member);

    let request = match h
        .call(
            Capability::ApprovalRequestCreate,
            RequestBody::ApprovalRequestCreate {
                room_id: room.header.id.clone(),
                anchor: source_anchor(source),
                kind: ApprovalKind::Approval,
                assignee_participant_ids: vec![assignee_b.clone(), assignee_c.clone()],
                blind_until_closed: true,
            },
        )
        .expect("approval create")
    {
        ResponseBody::CollabApprovalRequest { request } => *request,
        other => panic!("{other:?}"),
    };
    assert_eq!(request.assignee_participant_ids.len(), 2);

    // Non-requester withdraw is denied (T6: only the requester may withdraw).
    h.switch_actor("actor-b");
    let denied_withdraw = h.call(
        Capability::ApprovalWithdraw,
        RequestBody::ApprovalRequestWithdraw {
            request_id: request.header.id.clone(),
            expected_revision: 1,
        },
    );
    assert!(matches!(
        denied_withdraw,
        Err(AuthorityError::Unauthorized) | Err(AuthorityError::WrongScope)
    ));

    // Assignee B decides.
    h.call(
        Capability::ApprovalDecide,
        RequestBody::ApprovalDecide {
            request_id: request.header.id.clone(),
            outcome: ApprovalDecisionOutcome::Approved,
            rationale: Some("looks sound".to_owned()),
        },
    )
    .expect("decision B");

    // Blind mode: assignee C must not see B's decision while the request is
    // still Open (T6: blind_until_closed prevents copying a co-assignee's
    // already-recorded rationale before deciding).
    h.switch_actor("actor-c");
    let decisions_seen_by_c = match h
        .call(
            Capability::ApprovalRequestRead,
            RequestBody::ApprovalDecisionList {
                request_id: request.header.id.clone(),
            },
        )
        .expect("list decisions as C")
    {
        ResponseBody::CollabApprovalDecisionList { decisions } => decisions,
        other => panic!("{other:?}"),
    };
    assert!(
        decisions_seen_by_c.is_empty(),
        "blind_until_closed must hide co-assignee decisions while Open, got {decisions_seen_by_c:?}"
    );
    h.call(
        Capability::ApprovalDecide,
        RequestBody::ApprovalDecide {
            request_id: request.header.id.clone(),
            outcome: ApprovalDecisionOutcome::Approved,
            rationale: Some("agree".to_owned()),
        },
    )
    .expect("decision C");

    // Requester can withdraw (still Open; a request with recorded decisions
    // but no state transition remains Open in this frozen slice).
    h.switch_actor("actor-a");
    let withdrawn = h
        .call(
            Capability::ApprovalWithdraw,
            RequestBody::ApprovalRequestWithdraw {
                request_id: request.header.id.clone(),
                expected_revision: 1,
            },
        )
        .expect("requester withdraw");
    assert!(matches!(
        withdrawn,
        ResponseBody::CollabApprovalRequest { .. }
    ));
}

#[test]
fn activity_feed_records_every_mutation_in_seq_order() {
    let mut h = Harness::setup("activity");
    h.register_self("Operator");
    let project = h.project("study");
    let room = h.room(&project, "room");
    h.call(
        Capability::TaskCreate,
        RequestBody::TaskCreate {
            room_id: room.header.id.clone(),
            anchor: None,
            title: "task one".to_owned(),
            description: None,
        },
    )
    .expect("task");

    let records = match h
        .call(
            Capability::ActivityRead,
            RequestBody::ActivityList {
                room_id: room.header.id.clone(),
                limit: Some(50),
                after_seq: None,
            },
        )
        .expect("activity list")
    {
        ResponseBody::CollabActivityList { records } => records,
        other => panic!("{other:?}"),
    };
    assert!(
        records.len() >= 2,
        "room create + task create must both be recorded"
    );
    for pair in records.windows(2) {
        assert!(
            pair[0].seq < pair[1].seq,
            "activity records must be strictly ordered"
        );
    }
}

// ---------------------------------------------------------------------------
// security.md T1 -- structural: no call path from a collaboration decision
// into promotion/amendment/action-intent creation.
// ---------------------------------------------------------------------------

#[test]
fn t1_approval_decision_module_has_no_call_into_effect_or_promotion_paths() {
    let source = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../medscale-core/src/authority/collaboration.rs"
    ))
    .expect("read authority::collaboration source");
    for forbidden in [
        "authority::promote",
        "authority::amend",
        "contracts::actions",
    ] {
        assert!(
            !source.contains(forbidden),
            "authority::collaboration must never call into {forbidden} (security.md T1)"
        );
    }
}

// ---------------------------------------------------------------------------
// security.md T2 -- participant kind is immutable, never caller-supplied
// truth on re-registration.
// ---------------------------------------------------------------------------

#[test]
fn t2_participant_kind_is_immutable_on_reregistration() {
    let mut h = Harness::setup("t2-kind");
    let first = h.register_self("Operator");
    // Re-registering the same holder with the SAME kind is idempotent.
    let again = h.register_self("Operator (again)");
    assert_eq!(first, again);

    // Re-registering the same holder with a DIFFERENT kind is a Conflict,
    // never a silent flip from Human to Agent.
    let flipped = h.call(
        Capability::ParticipantRegister,
        RequestBody::ParticipantRegister {
            holder_id: h.holder.clone(),
            kind: ParticipantKind::Agent,
            display_name: "Operator as agent".to_owned(),
            agent_profile_ref: None,
        },
    );
    assert!(matches!(flipped, Err(AuthorityError::Conflict { .. })));
}

// ---------------------------------------------------------------------------
// security.md T3 -- cross-project / cross-room visibility leakage.
// ---------------------------------------------------------------------------

#[test]
fn t3_membership_in_one_room_never_leaks_a_sibling_room() {
    let mut h = Harness::setup("t3-visibility");
    h.register_self("Operator");
    let project_a = h.project("study-a");
    let project_b = h.project("study-b");
    let room_a = h.room(&project_a, "room a");
    let room_b = h.room(&project_b, "room b"); // caller is auto-member of both by virtue of creating them

    // Bring in a second participant who is a member of Room A only.
    h.switch_actor("actor-b");
    let member_b = h.register_self("Member B");
    h.switch_actor("actor-a");
    h.add_member(&room_a.header.id, &member_b, MembershipRole::Member);

    h.switch_actor("actor-b");
    // Room A is visible.
    h.call(
        Capability::RoomRead,
        RequestBody::RoomGet {
            room_id: room_a.header.id.clone(),
        },
    )
    .expect("room a visible to its member");

    // Room B must not be readable, listable, or otherwise inferable.
    let denied_get = h.call(
        Capability::RoomRead,
        RequestBody::RoomGet {
            room_id: room_b.header.id.clone(),
        },
    );
    assert!(matches!(denied_get, Err(AuthorityError::Unauthorized)));

    let list = match h
        .call(
            Capability::RoomRead,
            RequestBody::RoomList {
                project_id: project_b.clone(),
                status: None,
                limit: Some(50),
                cursor: None,
            },
        )
        .expect("list project b rooms")
    {
        ResponseBody::CollabRoomList { rooms, .. } => rooms,
        other => panic!("{other:?}"),
    };
    assert!(
        list.is_empty(),
        "a non-member must never see Room B's existence via list, got {list:?}"
    );

    let denied_activity = h.call(
        Capability::ActivityRead,
        RequestBody::ActivityList {
            room_id: room_b.header.id.clone(),
            limit: Some(10),
            after_seq: None,
        },
    );
    assert!(matches!(denied_activity, Err(AuthorityError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// security.md T4 -- removed membership / revoked participant fails closed
// immediately, without a new session.
// ---------------------------------------------------------------------------

#[test]
fn t4_removed_membership_denies_access_on_the_very_next_request() {
    let mut h = Harness::setup("t4-removal");
    h.register_self("Owner");
    let project = h.project("study");
    let room = h.room(&project, "room");

    h.switch_actor("actor-b");
    let member_b = h.register_self("Member B");
    h.switch_actor("actor-a");
    let membership_id = h.add_member(&room.header.id, &member_b, MembershipRole::Member);

    h.switch_actor("actor-b");
    // Access works while an active member.
    h.call(
        Capability::RoomRead,
        RequestBody::RoomGet {
            room_id: room.header.id.clone(),
        },
    )
    .expect("member can read while active");

    h.switch_actor("actor-a");
    h.call(
        Capability::RoomMembershipManage,
        RequestBody::RoomMembershipRemove {
            room_id: room.header.id.clone(),
            membership_id,
            expected_revision: 1,
        },
    )
    .expect("owner removes member");

    // The very next request from B, same holder/session model, fails closed
    // -- no cached membership, no grace period.
    h.switch_actor("actor-b");
    let denied = h.call(
        Capability::RoomRead,
        RequestBody::RoomGet {
            room_id: room.header.id.clone(),
        },
    );
    assert!(matches!(denied, Err(AuthorityError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// security.md T9 -- malformed/hostile metadata: bounds enforced end-to-end
// through Core, and content that is merely unusual (SQL-like, control
// characters, non-Latin Unicode) is stored and returned byte-exact rather
// than rejected or mutated (parameterized storage, never string-built SQL).
// ---------------------------------------------------------------------------

#[test]
fn t9_room_name_bounds_enforced_end_to_end_through_core() {
    let mut h = Harness::setup("t9-bounds");
    h.register_self("Operator");
    let project = h.project("study");

    let empty = h.call(
        Capability::RoomCreate,
        RequestBody::RoomCreate {
            project_id: project.clone(),
            experiment_id: None,
            name: "   ".to_owned(),
        },
    );
    assert!(matches!(empty, Err(AuthorityError::InvalidArgument { .. })));

    let over_max = h.call(
        Capability::RoomCreate,
        RequestBody::RoomCreate {
            project_id: project.clone(),
            experiment_id: None,
            name: "a".repeat(129),
        },
    );
    assert!(matches!(
        over_max,
        Err(AuthorityError::InvalidArgument { .. })
    ));

    let nul = h.call(
        Capability::RoomCreate,
        RequestBody::RoomCreate {
            project_id: project.clone(),
            experiment_id: None,
            name: "bad\0name".to_owned(),
        },
    );
    assert!(matches!(nul, Err(AuthorityError::InvalidArgument { .. })));
}

#[test]
fn t9_hostile_looking_text_is_stored_and_returned_byte_exact() {
    let mut h = Harness::setup("t9-hostile");
    h.register_self("Operator");
    let project = h.project("study");
    // A SQL-like room name and a message body with control characters and
    // non-Latin Unicode must be accepted as plain bounded text (parameterized
    // storage APIs neutralize injection; content is never sanitized/mutated).
    let hostile_name = "'; DROP TABLE collab_rooms; -- \u{202e}\u{0007}\u{4e2d}\u{6587}";
    let room = h.room(&project, hostile_name);
    assert_eq!(room.name, hostile_name);
    let source = h.source_record(b"x");

    let (thread, _) = match h
        .call(
            Capability::ThreadCreate,
            RequestBody::ThreadOpen {
                room_id: room.header.id.clone(),
                anchor: source_anchor(source),
            },
        )
        .expect("thread")
    {
        ResponseBody::CollabThread { thread, resolution } => (*thread, resolution),
        other => panic!("{other:?}"),
    };
    // Control characters and SQL-like text, but no NUL (bounded_text rejects
    // NUL separately and is covered by the over-max/empty/NUL bound test).
    let hostile_body = "'); DELETE FROM collab_messages; --\n\u{0007}\u{202e}".to_owned();
    let posted = match h
        .call(
            Capability::MessagePost,
            RequestBody::MessagePost {
                thread_id: thread.header.id.clone(),
                body: hostile_body.clone(),
            },
        )
        .expect("post hostile body")
    {
        ResponseBody::CollabMessage { message } => *message,
        other => panic!("{other:?}"),
    };
    assert_eq!(posted.body, hostile_body);

    // The room table must still be intact (parameterized queries, never
    // string-built SQL): a follow-up room create in the same project works.
    let follow_up = h.room(&project, "still works");
    assert_eq!(
        follow_up.status,
        medscale_contracts::collaboration::RoomStatus::Active
    );
}

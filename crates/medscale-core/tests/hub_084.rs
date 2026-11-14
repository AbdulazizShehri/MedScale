//! Spec 084 MedScale Hub foundation Core integration tests.
//!
//! Three synthetic vaults in one process: a Hub and two clients. Every call
//! goes through `CoreFacade::dispatch`; Hub calls from clients travel as
//! serialized `AuthorityRequest`s (in-process JSON round trip, and once over
//! the Spec 024 local-socket IPC).

use std::fs;
use std::path::PathBuf;

use medscale_contracts::collaboration::{
    AnchorTarget, MembershipRole, ParticipantKind, TaskStatus,
};
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::hub::{
    DeviceStatus, HubEvent, HubEventKind, HubInvitationCode, HubLink, HubSession, SyncEnvelope,
    SyncIntent, SyncOutcome, SyncRefusal,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding};
use medscale_core::CoreFacade;
use medscale_core::hub_sync::{self, ClientContext, InProcessHubTransport, IpcHubTransport};
use medscale_core::ipc::HostIpcServer;

const ENDPOINT: &str = "medscale-hub-test";

struct Node {
    facade: CoreFacade,
    vault: String,
    realm: String,
    scope: String,
    session: OpaqueId,
    holder: OpaqueId,
    dir: PathBuf,
    next: u64,
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-084c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

impl Node {
    fn open(dir: PathBuf, vault: &str, realm: &str, scope: &str) -> Self {
        Self::open_on(CoreFacade::new(), dir, vault, realm, scope)
    }

    fn open_on(facade: CoreFacade, dir: PathBuf, vault: &str, realm: &str, scope: &str) -> Self {
        let mut n = Node {
            facade,
            vault: vault.to_owned(),
            realm: realm.to_owned(),
            scope: scope.to_owned(),
            session: OpaqueId::new("pending"),
            holder: OpaqueId::new("pending"),
            dir,
            next: 0,
        };
        let ResponseBody::Lease { holder_id, .. } = n
            .raw(
                None,
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new(format!("{vault}-operator")),
                    holder_id_hint: Some(OpaqueId::new(format!("{vault}-operator"))),
                },
            )
            .unwrap()
        else {
            panic!()
        };
        n.holder = holder_id.clone();
        let ResponseBody::Session { session_id, .. } = n
            .raw(
                None,
                Capability::OpenSession,
                RequestBody::OpenSession {
                    holder_id,
                    granted: Capability::operator_grants(),
                    ttl_ticks: 1_000_000,
                },
            )
            .unwrap()
        else {
            panic!()
        };
        n.session = session_id;
        let vault_root = n.dir.join("vault").display().to_string();
        n.call(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault { vault_root },
        )
        .unwrap();
        n
    }

    fn raw(
        &mut self,
        session: Option<OpaqueId>,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next += 1;
        let mut r = AuthorityRequest::new(
            OpaqueId::new(format!("req-{}", self.next)),
            VaultId::new(&self.vault),
            RealmId::new(&self.realm),
            AuthorityScopeId::new(&self.scope),
            capability,
            body,
        );
        r.session_id = session;
        self.facade.dispatch(r).result
    }

    fn call(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        let s = Some(self.session.clone());
        self.raw(s, capability, body)
    }

    fn ctx(&self) -> ClientContext<'_> {
        ClientContext {
            facade: &self.facade,
            vault_id: VaultId::new(&self.vault),
            realm_id: RealmId::new(&self.realm),
            scope_id: AuthorityScopeId::new(&self.scope),
            session_id: self.session.clone(),
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
            .unwrap()
        {
            ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn register_operator(&mut self) -> OpaqueId {
        let holder = self.holder.clone();
        match self
            .call(
                Capability::ParticipantRegister,
                RequestBody::ParticipantRegister {
                    holder_id: holder,
                    kind: ParticipantKind::Human,
                    display_name: "Hub operator".to_owned(),
                    agent_profile_ref: None,
                },
            )
            .unwrap()
        {
            ResponseBody::Participant { participant } => participant.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn room(&mut self, project: &OpaqueId, name: &str) -> OpaqueId {
        match self
            .call(
                Capability::RoomCreate,
                RequestBody::RoomCreate {
                    project_id: project.clone(),
                    experiment_id: None,
                    name: name.to_owned(),
                },
            )
            .unwrap()
        {
            ResponseBody::CollabRoom { room } => room.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn member(&mut self, room: &OpaqueId, participant: &OpaqueId) {
        self.call(
            Capability::RoomMembershipManage,
            RequestBody::RoomMembershipAdd {
                room_id: room.clone(),
                participant_id: participant.clone(),
                role: MembershipRole::Member,
            },
        )
        .unwrap();
    }

    fn thread(&mut self, room: &OpaqueId) -> OpaqueId {
        match self
            .call(
                Capability::ThreadCreate,
                RequestBody::ThreadOpen {
                    room_id: room.clone(),
                    anchor: AnchorTarget {
                        artifact: ArtifactDescriptor {
                            object_id: OpaqueId::new("source-1"),
                            kind: ArtifactKind::SourceRecord,
                            binding: ArtifactVersionBinding::IdentityOnly,
                        },
                        detail: None,
                    },
                },
            )
            .unwrap()
        {
            ResponseBody::CollabThread { thread, .. } => thread.header.id,
            other => panic!("{other:?}"),
        }
    }

    fn init_hub(&mut self) {
        self.call(Capability::HubAdmin, RequestBody::HubInit)
            .unwrap();
    }

    fn invite(&mut self, project: &OpaqueId, name: &str) -> HubInvitationCode {
        match self
            .call(
                Capability::HubAdmin,
                RequestBody::HubInvite {
                    project_id: project.clone(),
                    display_name: name.to_owned(),
                },
            )
            .unwrap()
        {
            ResponseBody::HubInvited { code, .. } => *code,
            other => panic!("{other:?}"),
        }
    }

    fn queue(&mut self, link: &OpaqueId, intent: SyncIntent) -> SyncEnvelope {
        match self
            .call(
                Capability::HubClient,
                RequestBody::HubQueue {
                    link_id: link.clone(),
                    intent,
                },
            )
            .unwrap()
        {
            ResponseBody::HubQueued { entry } => entry.envelope,
            other => panic!("{other:?}"),
        }
    }

    fn mirror(&mut self, link: &OpaqueId) -> Vec<HubEvent> {
        match self
            .call(
                Capability::HubRead,
                RequestBody::HubMirrorList {
                    link_id: link.clone(),
                    after: 0,
                    limit: 100,
                },
            )
            .unwrap()
        {
            ResponseBody::HubMirror { events } => events,
            other => panic!("{other:?}"),
        }
    }

    fn link(&mut self, link: &OpaqueId) -> HubLink {
        match self
            .call(
                Capability::HubRead,
                RequestBody::HubLinkGet {
                    link_id: link.clone(),
                },
            )
            .unwrap()
        {
            ResponseBody::HubLink { link } => *link,
            other => panic!("{other:?}"),
        }
    }

    fn status(&mut self) -> medscale_contracts::hub::HubStatus {
        match self
            .call(Capability::HubRead, RequestBody::HubStatus)
            .unwrap()
        {
            ResponseBody::HubStatus { status } => *status,
            other => panic!("{other:?}"),
        }
    }
}

fn join(client: &Node, hub: &Node, code: HubInvitationCode) -> HubLink {
    let mut t = InProcessHubTransport { hub: &hub.facade };
    hub_sync::join(&client.ctx(), &mut t, ENDPOINT.to_owned(), code).unwrap()
}

fn sync(client: &Node, hub: &Node, link: &OpaqueId) -> medscale_contracts::hub::SyncReport {
    let mut t = InProcessHubTransport { hub: &hub.facade };
    hub_sync::sync(&client.ctx(), &mut t, link).unwrap()
}

/// A Hub with one Project, a room, a thread, and two joined clients that
/// are members of the room.
struct Lab {
    hub: Node,
    a: Node,
    b: Node,
    project: OpaqueId,
    room: OpaqueId,
    thread: OpaqueId,
    link_a: HubLink,
    link_b: HubLink,
}

fn lab(name: &str) -> Lab {
    let root = temp(name);
    let mut hub = Node::open(root.join("hub"), "vault-hub", "realm-h", "scope-h");
    let a = Node::open(root.join("a"), "vault-a", "realm-a", "scope-a");
    let b = Node::open(root.join("b"), "vault-b", "realm-b", "scope-b");
    let project = hub.project("Shared study");
    hub.register_operator();
    let room = hub.room(&project, "Analysis");
    let thread = hub.thread(&room);
    hub.init_hub();
    let code_a = hub.invite(&project, "Laptop A");
    let code_b = hub.invite(&project, "Laptop B");
    let link_a = join(&a, &hub, code_a);
    let link_b = join(&b, &hub, code_b);
    hub.member(&room, &link_a.participant_id);
    hub.member(&room, &link_b.participant_id);
    Lab {
        hub,
        a,
        b,
        project,
        room,
        thread,
        link_a,
        link_b,
    }
}

fn handshake(client: &mut Node, hub: &mut Node, link: &HubLink) -> HubSession {
    let ResponseBody::HubChallenge { challenge } = hub
        .raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubChallenge {
                device_id: link.device_id.clone(),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let ResponseBody::HubHandshakeSigned { handshake } = client
        .call(
            Capability::HubClient,
            RequestBody::HubSignHandshake {
                link_id: link.header.id.clone(),
                challenge: *challenge,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    match hub
        .raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubHandshake {
                handshake: *handshake,
            },
        )
        .unwrap()
    {
        ResponseBody::HubSession { session } => *session,
        other => panic!("{other:?}"),
    }
}

fn submit(
    hub: &mut Node,
    session: &HubSession,
    envelopes: Vec<SyncEnvelope>,
) -> Result<Vec<SyncOutcome>, AuthorityError> {
    hub.raw(
        Some(session.session_id.clone()),
        Capability::HubSync,
        RequestBody::HubSubmit { envelopes },
    )
    .map(|r| match r {
        ResponseBody::HubOutcomes { outcomes } => outcomes,
        other => panic!("{other:?}"),
    })
}

fn hub_events(hub: &mut Node, session: &HubSession) -> Vec<HubEvent> {
    match hub
        .raw(
            Some(session.session_id.clone()),
            Capability::HubSync,
            RequestBody::HubPull {
                after: 0,
                limit: 100,
            },
        )
        .unwrap()
    {
        ResponseBody::HubEvents { page } => page.events,
        other => panic!("{other:?}"),
    }
}

fn message(thread: &OpaqueId, body: &str) -> SyncIntent {
    SyncIntent::MessagePost {
        thread_id: thread.clone(),
        body: body.to_owned(),
    }
}

#[test]
fn invitations_and_handshakes_fail_closed() {
    let root = temp("enroll");
    let mut hub = Node::open(root.join("hub"), "vault-hub", "realm-h", "scope-h");
    let mut c = Node::open(root.join("c"), "vault-c", "realm-c", "scope-c");
    let project = hub.project("Study");
    // No Hub role yet.
    assert!(matches!(
        hub.call(
            Capability::HubAdmin,
            RequestBody::HubInvite {
                project_id: project.clone(),
                display_name: "x".to_owned()
            }
        ),
        Err(AuthorityError::NotFound)
    ));
    hub.init_hub();
    assert!(matches!(
        hub.call(Capability::HubAdmin, RequestBody::HubInit),
        Err(AuthorityError::Conflict { .. })
    ));
    let code = hub.invite(&project, "Laptop");

    // A tampered token is refused and the invitation stays open.
    let mut tampered = code.clone();
    tampered.token_hex = "0".repeat(64);
    let mut t = InProcessHubTransport { hub: &hub.facade };
    assert!(matches!(
        hub_sync::join(&c.ctx(), &mut t, ENDPOINT.to_owned(), tampered),
        Err(AuthorityError::Unauthorized)
    ));
    // An enrollment signed by another key is refused.
    let (_, other_public) = medscale_keys::generate_device_key();
    assert!(matches!(
        hub.raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubEnroll {
                token_hex: code.token_hex.clone(),
                public_key_hex: other_public,
                signature_hex: "ab".repeat(64),
            }
        ),
        Err(AuthorityError::Unauthorized)
    ));
    // Malformed hex is refused before any lookup.
    assert!(matches!(
        hub.raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubEnroll {
                token_hex: code.token_hex.to_uppercase(),
                public_key_hex: "a".repeat(64),
                signature_hex: "ab".repeat(64),
            }
        ),
        Err(AuthorityError::InvalidArgument { .. })
    ));

    let link = join(&c, &hub, code.clone());
    // Reuse is refused.
    let mut t = InProcessHubTransport { hub: &hub.facade };
    assert!(matches!(
        hub_sync::join(&c.ctx(), &mut t, ENDPOINT.to_owned(), code),
        Err(AuthorityError::Unauthorized)
    ));
    // A revoked invitation is refused.
    let revoked = hub.invite(&project, "Spare");
    hub.call(
        Capability::HubAdmin,
        RequestBody::HubInvitationRevoke {
            invitation_id: revoked.invitation_id.clone(),
        },
    )
    .unwrap();
    let mut t = InProcessHubTransport { hub: &hub.facade };
    assert!(matches!(
        hub_sync::join(&c.ctx(), &mut t, ENDPOINT.to_owned(), revoked),
        Err(AuthorityError::Unauthorized)
    ));

    // Handshakes: a good one works once; the nonce cannot be replayed.
    let ResponseBody::HubChallenge { challenge } = hub
        .raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubChallenge {
                device_id: link.device_id.clone(),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let ResponseBody::HubHandshakeSigned { handshake: signed } = c
        .call(
            Capability::HubClient,
            RequestBody::HubSignHandshake {
                link_id: link.header.id.clone(),
                challenge: *challenge,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let good = *signed;
    let mut wrong_version = good.clone();
    wrong_version.protocol_version = 2;
    assert!(matches!(
        hub.raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubHandshake {
                handshake: wrong_version
            }
        ),
        Err(AuthorityError::UnsupportedSchema { .. })
    ));
    let mut bad_sig = good.clone();
    bad_sig.signature_hex = "ab".repeat(64);
    // A bad signature consumes the nonce, so the good answer then fails too.
    assert!(matches!(
        hub.raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubHandshake { handshake: bad_sig }
        ),
        Err(AuthorityError::Unauthorized)
    ));
    assert!(matches!(
        hub.raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubHandshake {
                handshake: good.clone()
            }
        ),
        Err(AuthorityError::Unauthorized)
    ));
    // A nonce never issued is refused.
    let mut foreign = good;
    foreign.nonce_hex = "1".repeat(64);
    assert!(matches!(
        hub.raw(
            None,
            Capability::HubBootstrap,
            RequestBody::HubHandshake { handshake: foreign }
        ),
        Err(AuthorityError::Unauthorized)
    ));
    // A fresh handshake works and grants only Hub sync.
    let session = handshake(&mut c, &mut hub, &link);
    assert!(matches!(
        hub.raw(
            Some(session.session_id.clone()),
            Capability::ProjectCreate,
            RequestBody::ProjectCreate {
                name: "sneaky".to_owned(),
                description: None
            }
        ),
        Err(AuthorityError::SessionDenied)
    ));
    // Another tenant scope sees nothing.
    let mut wrong_scope = hub.raw(
        None,
        Capability::HubBootstrap,
        RequestBody::HubChallenge {
            device_id: link.device_id.clone(),
        },
    );
    assert!(wrong_scope.is_ok());
    hub.scope = "scope-other".to_owned();
    wrong_scope = hub.raw(
        None,
        Capability::HubBootstrap,
        RequestBody::HubChallenge {
            device_id: link.device_id.clone(),
        },
    );
    assert!(matches!(wrong_scope, Err(AuthorityError::WrongScope)));
}

#[test]
fn two_clients_edit_offline_and_converge() {
    let mut l = lab("converge");
    // Both clients work while disconnected.
    l.a.queue(&l.link_a.header.id, message(&l.thread, "A: first look"));
    l.a.queue(
        &l.link_a.header.id,
        SyncIntent::TaskCreate {
            room_id: l.room.clone(),
            title: "Check cohort size".to_owned(),
            description: None,
        },
    );
    l.b.queue(&l.link_b.header.id, message(&l.thread, "B: agreed"));
    l.b.queue(
        &l.link_b.header.id,
        SyncIntent::NoteCreate {
            room_id: l.room.clone(),
            title: "Plan".to_owned(),
            body: "Draft".to_owned(),
        },
    );
    let ra = sync(&l.a, &l.hub, &l.link_a.header.id);
    assert_eq!((ra.submitted, ra.applied, ra.refused), (2, 2, 0));
    let rb = sync(&l.b, &l.hub, &l.link_b.header.id);
    assert_eq!((rb.submitted, rb.applied, rb.refused), (2, 2, 0));
    sync(&l.a, &l.hub, &l.link_a.header.id);
    // Both mirrors hold the same ordered, verified event list: two
    // enrollments and four submissions.
    let ma = l.a.mirror(&l.link_a.header.id);
    let mb = l.b.mirror(&l.link_b.header.id);
    assert_eq!(ma, mb);
    assert_eq!(ma.len(), 6);
    assert_eq!(
        ma.iter().map(|e| e.cursor).collect::<Vec<_>>(),
        (1..=6).collect::<Vec<u64>>()
    );
    // The Hub applied them as each device's own participant.
    let ResponseBody::CollabMessageList { messages } = l
        .hub
        .call(
            Capability::MessageRead,
            RequestBody::MessageList {
                thread_id: l.thread.clone(),
                limit: None,
                after_seq: None,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let authors: Vec<_> = messages
        .iter()
        .map(|m| m.author_participant_id.clone())
        .collect();
    assert!(authors.contains(&l.link_a.participant_id));
    assert!(authors.contains(&l.link_b.participant_id));
    // Every queued entry is done with its outcome.
    let ResponseBody::HubOutbox { entries } =
        l.a.call(
            Capability::HubRead,
            RequestBody::HubOutboxList {
                link_id: l.link_a.header.id.clone(),
                pending_only: true,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(entries.is_empty());
    // A second sync with nothing queued changes nothing.
    let again = sync(&l.a, &l.hub, &l.link_a.header.id);
    assert_eq!((again.submitted, again.pulled), (0, 0));
    let status = l.hub.status();
    assert_eq!(status.checkpoints.len(), 1);
    assert_eq!(status.checkpoints[0].cursor, 6);
    let _ = l.project;
}

#[test]
fn conflicts_follow_q07() {
    let mut l = lab("conflicts");
    l.a.queue(
        &l.link_a.header.id,
        SyncIntent::TaskCreate {
            room_id: l.room.clone(),
            title: "Review".to_owned(),
            description: None,
        },
    );
    l.a.queue(
        &l.link_a.header.id,
        SyncIntent::NoteCreate {
            room_id: l.room.clone(),
            title: "Notes".to_owned(),
            body: "v1".to_owned(),
        },
    );
    sync(&l.a, &l.hub, &l.link_a.header.id);
    sync(&l.b, &l.hub, &l.link_b.header.id);
    let created: Vec<OpaqueId> =
        l.b.mirror(&l.link_b.header.id)
            .iter()
            .filter_map(|e| match &e.kind {
                HubEventKind::Submission {
                    outcome: SyncOutcome::Applied { object_id, .. },
                    ..
                } => Some(object_id.clone()),
                _ => None,
            })
            .collect();
    let (task, note) = (created[0].clone(), created[1].clone());
    // Both edit the same revision offline.
    for (node, link, who) in [(&mut l.a, &l.link_a, "A"), (&mut l.b, &l.link_b, "B")] {
        node.queue(
            &link.header.id,
            SyncIntent::TaskUpdate {
                task_id: task.clone(),
                expected_revision: 1,
                status: TaskStatus::InProgress,
                assignee_participant_id: None,
            },
        );
        node.queue(
            &link.header.id,
            SyncIntent::NoteEdit {
                note_id: note.clone(),
                expected_revision: 1,
                body: format!("{who} edit"),
            },
        );
        node.queue(&link.header.id, message(&l.thread, &format!("{who} here")));
    }
    let ra = sync(&l.a, &l.hub, &l.link_a.header.id);
    assert_eq!((ra.applied, ra.conflicts), (3, 0));
    let rb = sync(&l.b, &l.hub, &l.link_b.header.id);
    // The stale task update conflicts; the stale note edit becomes a
    // conflict copy (counted as applied); the message appends.
    assert_eq!((rb.applied, rb.conflicts, rb.refused), (2, 1, 0));
    let ResponseBody::HubOutbox { entries } =
        l.b.call(
            Capability::HubRead,
            RequestBody::HubOutboxList {
                link_id: l.link_b.header.id.clone(),
                pending_only: false,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let outcomes: Vec<_> = entries.iter().filter_map(|e| e.outcome.clone()).collect();
    assert!(outcomes.iter().any(|o| matches!(
        o,
        SyncOutcome::Conflict { conflict } if conflict.current_revision == Some(2)
    )));
    assert!(
        outcomes
            .iter()
            .any(|o| matches!(o, SyncOutcome::ConflictCopy { .. }))
    );
    // The Hub task kept A's update: never last-writer-wins.
    let ResponseBody::CollabTask { task: t } = l
        .hub
        .call(Capability::TaskRead, RequestBody::TaskGet { task_id: task })
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(t.revision, 2);
}

#[test]
fn replays_forgeries_and_sequence_breaks_are_refused() {
    let mut l = lab("replay");
    let e1 = l.a.queue(&l.link_a.header.id, message(&l.thread, "one"));
    let session = handshake(&mut l.a, &mut l.hub, &l.link_a);
    let first = submit(&mut l.hub, &session, vec![e1.clone()]).unwrap();
    assert!(matches!(first[0], SyncOutcome::Applied { .. }));
    let before = hub_events(&mut l.hub, &session).len();
    // Byte-identical replay: same outcome, nothing recorded or applied.
    let replay = submit(&mut l.hub, &session, vec![e1.clone()]).unwrap();
    assert_eq!(replay, first);
    assert_eq!(hub_events(&mut l.hub, &session).len(), before);

    let e2 = l.a.queue(&l.link_a.header.id, message(&l.thread, "two"));
    // Forged signature: refused and recorded.
    let mut forged = e2.clone();
    forged.signature_hex = e1.signature_hex.clone();
    // Reused sequence with another body (validly signed by the device).
    let reused = SyncEnvelope {
        body: e1.body.clone(),
        signature_hex: e2.signature_hex.clone(),
    };
    // Gap: sequence 3 before 2.
    let e3 = l.a.queue(&l.link_a.header.id, message(&l.thread, "three"));
    // Another hub's and another device's envelopes.
    let mut other_hub = e2.clone();
    other_hub.body.hub_id = OpaqueId::new("hub-elsewhere");
    let mut other_device = e2.clone();
    other_device.body.device_id = l.link_b.device_id.clone();
    let outcomes = submit(
        &mut l.hub,
        &session,
        vec![forged, reused, e3.clone(), other_hub, other_device],
    )
    .unwrap();
    let reasons: Vec<_> = outcomes
        .iter()
        .map(|o| match o {
            SyncOutcome::Refused { reason } => *reason,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        reasons,
        vec![
            SyncRefusal::BadSignature,
            SyncRefusal::BadSignature,
            SyncRefusal::SequenceGap,
            SyncRefusal::WrongHub,
            SyncRefusal::WrongDevice,
        ]
    );
    // Three were attributable to the device and recorded; nothing applied.
    assert_eq!(hub_events(&mut l.hub, &session).len(), before + 3);
    // The real sequence 2 and 3 still apply in order.
    let ok = submit(&mut l.hub, &session, vec![e2, e3]).unwrap();
    assert!(ok.iter().all(|o| matches!(o, SyncOutcome::Applied { .. })));
    // A different body validly signed by the device key at a sequence it
    // already used (as a stolen key could produce) is refused, not applied.
    let secret: String = rusqlite::Connection::open(l.a.dir.join("vault").join("meta.sqlite3"))
        .unwrap()
        .query_row(
            "SELECT secret_hex FROM hub_link_secrets WHERE link_id = ?1",
            [l.link_a.header.id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    let mut reused_body = e1.body.clone();
    reused_body.intent = message(&l.thread, "rewritten history");
    let reused_sig =
        medscale_keys::sign_device_payload(&secret, &reused_body.signing_payload().unwrap())
            .unwrap();
    let outcome = submit(
        &mut l.hub,
        &session,
        vec![SyncEnvelope {
            body: reused_body,
            signature_hex: reused_sig,
        }],
    )
    .unwrap();
    assert!(matches!(
        outcome[0],
        SyncOutcome::Refused {
            reason: SyncRefusal::SequenceReused
        }
    ));
    // The whole chain still verifies.
    assert_eq!(l.hub.status().checkpoints.len(), 1);
}

#[test]
fn revocation_blocks_the_device_and_propagates() {
    let mut l = lab("revoke");
    let old_session = handshake(&mut l.b, &mut l.hub, &l.link_b);
    l.b.queue(&l.link_b.header.id, message(&l.thread, "before"));
    l.hub
        .call(
            Capability::HubAdmin,
            RequestBody::HubDeviceRevoke {
                device_id: l.link_b.device_id.clone(),
            },
        )
        .unwrap();
    // The old session is dead; a new handshake is refused; nothing applies.
    let pending = l.b.queue(&l.link_b.header.id, message(&l.thread, "after"));
    assert!(matches!(
        submit(&mut l.hub, &old_session, vec![pending]),
        Err(AuthorityError::SessionRevoked)
    ));
    let mut t = InProcessHubTransport { hub: &l.hub.facade };
    assert!(matches!(
        hub_sync::sync(&l.b.ctx(), &mut t, &l.link_b.header.id),
        Err(AuthorityError::Unauthorized)
    ));
    assert!(matches!(
        l.hub.call(
            Capability::HubAdmin,
            RequestBody::HubDeviceRevoke {
                device_id: l.link_b.device_id.clone()
            }
        ),
        Err(AuthorityError::Conflict { .. })
    ));
    let status = l.hub.status();
    let device = status
        .devices
        .iter()
        .find(|d| d.header.id == l.link_b.device_id)
        .unwrap();
    assert_eq!(device.status, DeviceStatus::Revoked);
    // Client A learns of the revocation through the event log.
    sync(&l.a, &l.hub, &l.link_a.header.id);
    assert!(l.a.mirror(&l.link_a.header.id).iter().any(|e| matches!(
        &e.kind,
        HubEventKind::DeviceRevoked { device_id } if *device_id == l.link_b.device_id
    )));
    // No message from B ever reached the Hub.
    let ResponseBody::CollabMessageList { messages } = l
        .hub
        .call(
            Capability::MessageRead,
            RequestBody::MessageList {
                thread_id: l.thread.clone(),
                limit: None,
                after_seq: None,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(messages.is_empty());
}

#[test]
fn projects_and_rooms_stay_isolated() {
    let mut l = lab("isolation");
    // A room in another Project of the same Hub.
    let other_project = l.hub.project("Other study");
    let other_room = l.hub.room(&other_project, "Private");
    l.hub.member(&other_room, &l.link_a.participant_id);
    // Membership alone does not cross the device's enrolled Project.
    l.a.queue(
        &l.link_a.header.id,
        SyncIntent::TaskCreate {
            room_id: other_room,
            title: "Leak".to_owned(),
            description: None,
        },
    );
    // A room of the right Project the device is not a member of.
    let closed_room = l.hub.room(&l.project, "Closed");
    l.a.queue(
        &l.link_a.header.id,
        SyncIntent::NoteCreate {
            room_id: closed_room,
            title: "x".to_owned(),
            body: "y".to_owned(),
        },
    );
    l.a.queue(
        &l.link_a.header.id,
        SyncIntent::TaskUpdate {
            task_id: OpaqueId::new("task-missing"),
            expected_revision: 1,
            status: TaskStatus::Done,
            assignee_participant_id: None,
        },
    );
    let r = sync(&l.a, &l.hub, &l.link_a.header.id);
    assert_eq!((r.applied, r.refused), (0, 3));
    let ResponseBody::HubOutbox { entries } =
        l.a.call(
            Capability::HubRead,
            RequestBody::HubOutboxList {
                link_id: l.link_a.header.id.clone(),
                pending_only: false,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let reasons: Vec<_> = entries
        .iter()
        .map(|e| match &e.outcome {
            Some(SyncOutcome::Refused { reason }) => *reason,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        reasons,
        vec![
            SyncRefusal::WrongProject,
            SyncRefusal::NotMember,
            SyncRefusal::NotFound
        ]
    );
    // Refusals still claim their sequences, so the next intent applies.
    l.a.queue(&l.link_a.header.id, message(&l.thread, "fine"));
    assert_eq!(sync(&l.a, &l.hub, &l.link_a.header.id).applied, 1);
    // An operator session cannot submit: it is not a device.
    let operator = Some(l.hub.session.clone());
    assert!(
        l.hub
            .raw(
                operator,
                Capability::HubSync,
                RequestBody::HubPull {
                    after: 0,
                    limit: 10
                }
            )
            .is_err()
    );
    // A client's links are bound to its own tenant scope.
    l.b.scope = "scope-other".to_owned();
    assert!(matches!(
        l.b.call(
            Capability::HubRead,
            RequestBody::HubLinkGet {
                link_id: l.link_b.header.id.clone()
            }
        ),
        Err(AuthorityError::WrongScope)
    ));
}

#[test]
fn hub_and_client_state_survive_reopen_and_restore_needs_re_enrollment() {
    let mut l = lab("reopen");
    l.a.queue(&l.link_a.header.id, message(&l.thread, "persisted"));
    sync(&l.a, &l.hub, &l.link_a.header.id);
    let hub_dir = l.hub.dir.clone();
    let a_dir = l.a.dir.clone();
    drop(l.hub);
    drop(l.a);
    let mut hub = Node::open(hub_dir, "vault-hub", "realm-h", "scope-h");
    let mut a = Node::open(a_dir.clone(), "vault-a", "realm-a", "scope-a");
    let status = hub.status();
    assert_eq!(status.checkpoints[0].cursor, 3);
    // Sessions are in memory: a new handshake is required and works.
    a.queue(&l.link_a.header.id, message(&l.thread, "after reopen"));
    let r = sync(&a, &hub, &l.link_a.header.id);
    assert_eq!((r.applied, r.cursor), (1, 4));
    assert_eq!(a.link(&l.link_a.header.id).cursor, 4);

    // Backup and restore the client: its link and mirror return, its
    // device secret does not, so syncing fails closed until re-enrollment.
    let backup = a_dir.join("backup").display().to_string();
    a.call(
        Capability::BackupVault,
        RequestBody::BackupVault {
            destination: backup.clone(),
        },
    )
    .unwrap();
    let restored_dir = temp("reopen-restored");
    a.call(
        Capability::RestoreVault,
        RequestBody::RestoreVault {
            source: backup,
            destination: restored_dir.join("vault").display().to_string(),
        },
    )
    .unwrap();
    drop(a);
    let restored = Node::open(restored_dir, "vault-a", "realm-a", "scope-a");
    let mut t = InProcessHubTransport { hub: &hub.facade };
    assert!(matches!(
        hub_sync::sync(&restored.ctx(), &mut t, &l.link_a.header.id),
        Err(AuthorityError::Unavailable { .. })
    ));
    let mut restored = restored;
    assert_eq!(restored.mirror(&l.link_a.header.id).len(), 4);
}

#[test]
fn a_hub_served_over_local_ipc_syncs_a_client() {
    let root = temp("ipc");
    let host_lock = root.join("host");
    fs::create_dir_all(&host_lock).unwrap();
    let endpoint = format!("medscale-hub-084-{}", std::process::id());
    let server = HostIpcServer::bind(&host_lock, &endpoint).expect("bind");
    let (endpoint, _facade, _join) = server.into_background();
    // The Hub operator uses the served facade through its own IPC client.
    let mut hub = IpcNode::connect(&endpoint, &host_lock, root.join("hub"));
    let project = hub.project();
    hub.call(Capability::HubAdmin, RequestBody::HubInit)
        .unwrap();
    let ResponseBody::HubInvited { code, .. } = hub
        .call(
            Capability::HubAdmin,
            RequestBody::HubInvite {
                project_id: project,
                display_name: "Laptop".to_owned(),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let c = Node::open(root.join("c"), "vault-c", "realm-c", "scope-c");
    // This test serves the full endpoint, which needs the host's token; a
    // real Hub serves devices on its limited endpoint, which doesn't.
    let device = || {
        IpcHubTransport::from_client(
            medscale_core::ipc::HostIpcClient::connect_authenticated(&endpoint, &host_lock)
                .unwrap(),
        )
    };
    let mut t = device();
    let link = hub_sync::join(&c.ctx(), &mut t, endpoint.clone(), *code).unwrap();
    let mut t = device();
    let report = hub_sync::sync(&c.ctx(), &mut t, &link.header.id).unwrap();
    assert_eq!((report.pulled, report.cursor), (1, 1));
}

/// A Hub operator talking to an IPC-hosted facade.
struct IpcNode {
    client: medscale_core::ipc::HostIpcClient,
    session: Option<OpaqueId>,
    next: u64,
}

impl IpcNode {
    fn connect(endpoint: &str, host_root: &std::path::Path, vault_dir: PathBuf) -> Self {
        let mut n = IpcNode {
            client: medscale_core::ipc::HostIpcClient::connect_authenticated(endpoint, host_root)
                .unwrap(),
            session: None,
            next: 0,
        };
        let ResponseBody::Lease { holder_id, .. } = n
            .call(
                Capability::AcquireLease,
                RequestBody::AcquireLease {
                    client_id: OpaqueId::new("hub-operator"),
                    holder_id_hint: Some(OpaqueId::new("hub-operator")),
                },
            )
            .unwrap()
        else {
            panic!()
        };
        let ResponseBody::Session { session_id, .. } = n
            .call(
                Capability::OpenSession,
                RequestBody::OpenSession {
                    holder_id,
                    granted: Capability::operator_grants(),
                    ttl_ticks: 1_000_000,
                },
            )
            .unwrap()
        else {
            panic!()
        };
        n.session = Some(session_id);
        n.call(
            Capability::OpenSyntheticVault,
            RequestBody::OpenSyntheticVault {
                vault_root: vault_dir.display().to_string(),
            },
        )
        .unwrap();
        n
    }

    fn call(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next += 1;
        let mut r = AuthorityRequest::new(
            OpaqueId::new(format!("ipc-req-{}", self.next)),
            VaultId::new("vault-hub"),
            RealmId::new("realm-h"),
            AuthorityScopeId::new("scope-h"),
            capability,
            body,
        );
        r.session_id = self.session.clone();
        self.client.dispatch(r).unwrap().result
    }

    fn project(&mut self) -> OpaqueId {
        match self
            .call(
                Capability::ProjectCreate,
                RequestBody::ProjectCreate {
                    name: "IPC study".to_owned(),
                    description: None,
                },
            )
            .unwrap()
        {
            ResponseBody::Project { project } => project.header.id,
            other => panic!("{other:?}"),
        }
    }
}

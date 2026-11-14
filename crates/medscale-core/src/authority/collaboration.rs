//! Collaboration Substrate Core authority paths (Spec 076).
//!
//! Every mutation flows: actor/session -> realm/scope -> RoomMembership
//! visibility (collaboration-scope, independent of Core `Capability`) ->
//! validation -> expected revision -> transaction (primary row + its
//! `ActivityRecord`, same commit) -> typed result. Surfaces never touch
//! storage.
//!
//! T076-03 scope only: `ParticipantIdentity`, `Room`, `RoomMembership`.
//! Thread/message/task/note/approval/activity-read Core paths land in
//! T076-04 through T076-09.
//!
//! Audit split (see `contracts.md` section 14): `ParticipantIdentity` is
//! scope-level, not room-level, and has no room to chain an `ActivityRecord`
//! into, so participant register/revoke reuse the *existing* in-memory
//! `ActionAuditRecord` trail (same mechanism Spec 074 uses for
//! `project.create`). Every room-scoped mutation (room, membership, and
//! everything built on them in later slices) appends to the new
//! `collab_activity_records` hash chain instead, in the same transaction as
//! its primary row write.

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::collaboration::{
    COLLAB_SCHEMA_VERSION, MembershipRole, ParticipantIdentity, ParticipantKind, ParticipantStatus,
    Room, RoomMembership, RoomStatus, Task, ThreadRef,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId, VaultId};
use medscale_storage::{MetaError, SqliteMetaStore};

use super::store::{InMemoryAuthorityStore, StoredObject};
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

fn header(realm: &RealmId, scope: &AuthorityScopeId, id: OpaqueId) -> ObjectHeader {
    ObjectHeader {
        id,
        schema_version: COLLAB_SCHEMA_VERSION,
        realm_id: realm.clone(),
        authority_scope_id: scope.clone(),
    }
}

/// A thread paired with its live `ReferenceResolution`, recomputed at read
/// time (`contracts.md` section 6). Named to keep `list_threads`'s return
/// type under clippy's `type_complexity` threshold.
type ResolvedThread = (
    ThreadRef,
    medscale_contracts::project_graph::ReferenceResolution,
);

/// Authenticated authority view for one request.
pub struct Collab<'a> {
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

impl Collab<'_> {
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

    /// Appends one scope-level audit row to the existing trail (participant
    /// lifecycle only; see module docs).
    fn audit(&mut self, action: &str, targets: Vec<OpaqueId>) -> Result<(), AuthorityError> {
        let actor = self.actor()?;
        let id = self.store.alloc_id("audit");
        let record = medscale_contracts::objects::ActionAuditRecord {
            header: ObjectHeader {
                id,
                schema_version: AUTHORITY_SCHEMA_VERSION,
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

    fn scoped_room(&self, id: &OpaqueId) -> Result<Room, AuthorityError> {
        let room = self.meta.get_room(id).map_err(meta_err)?;
        if room.header.realm_id != self.realm || room.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(room)
    }

    fn scoped_participant(&self, id: &OpaqueId) -> Result<ParticipantIdentity, AuthorityError> {
        let participant = self.meta.get_participant(id).map_err(meta_err)?;
        if participant.header.realm_id != self.realm
            || participant.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(participant)
    }

    /// Resolves the caller's own `ParticipantIdentity` via their session/
    /// lease holder id. `Unauthorized` when the caller has never registered
    /// a participant in this scope: every collaboration action requires
    /// registration first, including creating the first room.
    fn caller_participant(&self) -> Result<ParticipantIdentity, AuthorityError> {
        let holder_id = self.actor()?;
        self.meta
            .get_participant_by_holder(&self.scope, &holder_id)
            .map_err(meta_err)?
            .ok_or(AuthorityError::Unauthorized)
    }

    /// Resolves the caller's participant identity and proves they hold an
    /// active membership in `room_id`, scoped to this realm/scope. This is
    /// the collaboration-visibility check (`security.md` T3): independent
    /// of, and never a substitute for, the Core `Capability` gate already
    /// enforced by the facade before any `Collab` method runs.
    fn require_membership(
        &self,
        room_id: &OpaqueId,
    ) -> Result<ParticipantIdentity, AuthorityError> {
        self.scoped_room(room_id)?;
        let participant = self.caller_participant()?;
        let membership = self
            .meta
            .find_active_membership(room_id, &participant.header.id)
            .map_err(meta_err)?;
        if membership.is_none() {
            // Denied resolution must not reveal room existence beyond what
            // the scope check above already necessarily reveals via
            // NotFound/WrongScope; a member-less caller gets the same
            // Unauthorized a nonexistent room would not otherwise imply.
            return Err(AuthorityError::Unauthorized);
        }
        Ok(participant)
    }

    // ----- participants -----

    /// Registers a participant for `holder_id`, or returns the existing one
    /// (idempotent on `(scope, holder_id)`). `kind` is immutable once set;
    /// re-registering an existing holder with a different `kind` is a
    /// conflict, not a silent overwrite.
    pub fn register_participant(
        &mut self,
        holder_id: OpaqueId,
        kind: ParticipantKind,
        display_name: String,
        agent_profile_ref: Option<OpaqueId>,
    ) -> Result<ParticipantIdentity, AuthorityError> {
        if let Some(existing) = self
            .meta
            .get_participant_by_holder(&self.scope, &holder_id)
            .map_err(meta_err)?
        {
            if existing.kind != kind {
                return Err(AuthorityError::Conflict {
                    message: "participant kind is immutable once registered".to_owned(),
                });
            }
            return Ok(existing);
        }
        if kind != ParticipantKind::Agent && agent_profile_ref.is_some() {
            return Err(AuthorityError::InvalidArgument {
                message: "agent_profile_ref is only admitted for kind = agent".to_owned(),
            });
        }
        let id = self
            .meta
            .alloc_collab_id("collab-participant", "participant")
            .map_err(meta_err)?;
        let participant = ParticipantIdentity::new(
            header(&self.realm, &self.scope, id.clone()),
            holder_id,
            kind,
            display_name,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta
            .insert_participant(&participant, agent_profile_ref.as_ref())
            .map_err(meta_err)?;
        self.audit("participant.register", vec![id])?;
        Ok(participant)
    }

    /// Scoped participant read.
    pub fn get_participant(&self, id: &OpaqueId) -> Result<ParticipantIdentity, AuthorityError> {
        self.scoped_participant(id)
    }

    /// Revokes a participant (status only; `kind`/`holder_id` never change).
    pub fn revoke_participant(
        &mut self,
        id: &OpaqueId,
        expected: u64,
    ) -> Result<ParticipantIdentity, AuthorityError> {
        let current = self.scoped_participant(id)?;
        if current.status != ParticipantStatus::Active {
            return Err(AuthorityError::Conflict {
                message: "participant is not active".to_owned(),
            });
        }
        let revoked = self
            .meta
            .set_participant_status(id, expected, ParticipantStatus::Revoked)
            .map_err(meta_err)?;
        self.audit("participant.revoke", vec![id.clone()])?;
        Ok(revoked)
    }

    // ----- rooms -----

    /// Creates a Room scoped to an existing Project and grants the caller
    /// `Owner` membership. The caller must already be a registered
    /// participant (`register_participant` first).
    pub fn create_room(
        &mut self,
        project_id: OpaqueId,
        experiment_id: Option<OpaqueId>,
        name: String,
    ) -> Result<Room, AuthorityError> {
        let project = self.meta.get_project(&project_id).map_err(meta_err)?;
        if project.header.realm_id != self.realm || project.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        let creator = self.caller_participant()?;
        let room_id = self
            .meta
            .alloc_collab_id("collab-room", "room")
            .map_err(meta_err)?;
        let room = Room::new(
            header(&self.realm, &self.scope, room_id.clone()),
            project_id,
            experiment_id,
            name,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        self.meta
            .insert_room_with_activity(
                &room,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        let membership_id = self
            .meta
            .alloc_collab_id("collab-membership", "membership")
            .map_err(meta_err)?;
        let membership = RoomMembership::new(
            header(&self.realm, &self.scope, membership_id),
            room_id,
            creator.header.id,
            MembershipRole::Owner,
        );
        let owner_activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        self.meta
            .insert_membership_with_activity(
                &membership,
                header(&self.realm, &self.scope, owner_activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        self.meta.get_room(&room.header.id).map_err(meta_err)
    }

    /// Scoped, membership-gated Room read (`security.md` T3).
    pub fn get_room(&self, id: &OpaqueId) -> Result<Room, AuthorityError> {
        self.require_membership(id)?;
        self.scoped_room(id)
    }

    /// Lists rooms in one Project that the caller is an active member of.
    /// Visibility is intersected with membership, not merely Project scope
    /// (`security.md` T3): a caller with Project-level access but no
    /// `RoomMembership` sees nothing.
    pub fn list_rooms(
        &self,
        project_id: &OpaqueId,
        status: Option<RoomStatus>,
        limit: u32,
        after: Option<String>,
    ) -> Result<(Vec<Room>, Option<String>), AuthorityError> {
        let caller = self.caller_participant()?;
        let (rooms, next) = self
            .meta
            .list_rooms(&self.scope, project_id, status, limit, after.as_deref())
            .map_err(meta_err)?;
        let mut visible = Vec::with_capacity(rooms.len());
        for room in rooms {
            let member = self
                .meta
                .find_active_membership(&room.header.id, &caller.header.id)
                .map_err(meta_err)?;
            if member.is_some() {
                visible.push(room);
            }
        }
        Ok((visible, next))
    }

    /// Compare-and-swap room name (membership-gated; no dedicated activity
    /// event exists for rename in the frozen 076 vocabulary).
    pub fn rename_room(
        &self,
        id: &OpaqueId,
        expected: u64,
        name: String,
    ) -> Result<Room, AuthorityError> {
        self.require_membership(id)?;
        self.meta
            .update_room_name(id, expected, &name)
            .map_err(meta_err)
    }

    /// Archives a Room (membership-gated; requires `Owner` role).
    pub fn archive_room(&self, id: &OpaqueId, expected: u64) -> Result<Room, AuthorityError> {
        let caller = self.require_membership(id)?;
        let membership = self
            .meta
            .find_active_membership(id, &caller.header.id)
            .map_err(meta_err)?
            .ok_or(AuthorityError::Unauthorized)?;
        if membership.role != MembershipRole::Owner {
            return Err(AuthorityError::Unauthorized);
        }
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (room, _activity) = self
            .meta
            .archive_room_with_activity(
                id,
                expected,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        Ok(room)
    }

    // ----- room memberships -----

    /// Adds a member to a room (membership-gated: only an existing member
    /// may add another). Rejects a duplicate *active* `(room, participant)`
    /// pair rather than silently duplicating it.
    pub fn add_membership(
        &self,
        room_id: &OpaqueId,
        participant_id: &OpaqueId,
        role: MembershipRole,
    ) -> Result<RoomMembership, AuthorityError> {
        self.require_membership(room_id)?;
        let target = self.scoped_participant(participant_id)?;
        if target.status != ParticipantStatus::Active {
            return Err(AuthorityError::Conflict {
                message: "participant is not active".to_owned(),
            });
        }
        if self
            .meta
            .find_active_membership(room_id, participant_id)
            .map_err(meta_err)?
            .is_some()
        {
            return Err(AuthorityError::Conflict {
                message: "participant already has an active membership in this room".to_owned(),
            });
        }
        let membership_id = self
            .meta
            .alloc_collab_id("collab-membership", "membership")
            .map_err(meta_err)?;
        let membership = RoomMembership::new(
            header(&self.realm, &self.scope, membership_id),
            room_id.clone(),
            participant_id.clone(),
            role,
        );
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        self.meta
            .insert_membership_with_activity(
                &membership,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        self.meta
            .find_active_membership(room_id, participant_id)
            .map_err(meta_err)?
            .ok_or(AuthorityError::Internal {
                message: "membership vanished immediately after insert".to_owned(),
            })
    }

    /// Lists active memberships for one room (membership-gated).
    pub fn list_memberships(
        &self,
        room_id: &OpaqueId,
    ) -> Result<Vec<RoomMembership>, AuthorityError> {
        self.require_membership(room_id)?;
        self.meta.list_room_memberships(room_id).map_err(meta_err)
    }

    /// Removes a member (membership-gated). A member may always remove
    /// themself; removing another participant requires the caller to hold
    /// `Owner` role.
    pub fn remove_membership(
        &self,
        room_id: &OpaqueId,
        membership_id: &OpaqueId,
        expected: u64,
    ) -> Result<RoomMembership, AuthorityError> {
        let caller = self.require_membership(room_id)?;
        let memberships = self.meta.list_room_memberships(room_id).map_err(meta_err)?;
        let target = memberships
            .iter()
            .find(|m| m.header.id == *membership_id)
            .cloned()
            .ok_or(AuthorityError::NotFound)?;
        if target.participant_id != caller.header.id {
            let caller_membership = memberships
                .iter()
                .find(|m| m.participant_id == caller.header.id)
                .ok_or(AuthorityError::Unauthorized)?;
            if caller_membership.role != MembershipRole::Owner {
                return Err(AuthorityError::Unauthorized);
            }
        }
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (membership, _activity) = self
            .meta
            .remove_membership_with_activity(
                membership_id,
                expected,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        Ok(membership)
    }

    // ----- threads -----

    fn scoped_thread(&self, id: &OpaqueId) -> Result<ThreadRef, AuthorityError> {
        let thread = self.meta.get_thread(id).map_err(meta_err)?;
        if thread.header.realm_id != self.realm || thread.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(thread)
    }

    /// Resolves one anchor's target `ArtifactDescriptor` against canonical
    /// owners in this realm/scope, live, at read time (`contracts.md`
    /// section 6): never cached, never trusted from a prior write. This
    /// deliberately mirrors `project_graph::ProjectGraph::resolve_descriptor`
    /// field-for-field rather than sharing it: that method takes `&self` on
    /// a different struct shape, and 076 does not modify Spec 074's closed
    /// file to extract a shared helper. A future refactor could unify them.
    ///
    /// Coverage is exactly what `ArtifactKind`'s frozen vocabulary supports
    /// today: `PackManifest` and the H0-era in-memory `StoredObject` kinds.
    /// `ArtifactKind` has no variant yet for Spec 075 data-source/snapshot
    /// objects, so an anchor pointing at those resolves as `UnsupportedKind`
    /// -- an honest fail-closed answer, not a fabricated `Current`.
    fn resolve_anchor_artifact(
        &self,
        descriptor: &medscale_contracts::project_graph::ArtifactDescriptor,
    ) -> medscale_contracts::project_graph::ReferenceResolution {
        use medscale_contracts::project_graph::{
            ArtifactKind, ArtifactVersionBinding, ReferenceResolution,
        };
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
                        Err(super::store::ScopeError::NotFound) => {
                            return ReferenceResolution::Missing;
                        }
                        Err(super::store::ScopeError::WrongScope) => {
                            return ReferenceResolution::Denied;
                        }
                    };
                let class_matches = matches!(
                    (stored, &descriptor.kind),
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
                );
                if !class_matches {
                    return ReferenceResolution::Corrupt;
                }
                match stored {
                    StoredObject::Source(record) => {
                        if !record.digest_valid() {
                            return ReferenceResolution::Corrupt;
                        }
                        match &descriptor.binding {
                            ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                            ArtifactVersionBinding::Digest(digest) => {
                                if record.content_digest == *digest {
                                    ReferenceResolution::Current
                                } else {
                                    ReferenceResolution::Stale
                                }
                            }
                            _ => ReferenceResolution::Stale,
                        }
                    }
                    StoredObject::Derived(artifact) => match &descriptor.binding {
                        ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                        ArtifactVersionBinding::Digest(digest) => {
                            if artifact.content_digest == *digest {
                                ReferenceResolution::Current
                            } else {
                                ReferenceResolution::Stale
                            }
                        }
                        _ => ReferenceResolution::Stale,
                    },
                    _ => match &descriptor.binding {
                        ArtifactVersionBinding::IdentityOnly => ReferenceResolution::Current,
                        _ => ReferenceResolution::Stale,
                    },
                }
            }
        }
    }

    /// Opens a thread anchored to an exact artifact revision (membership-
    /// gated). Anchor shape is validated by `ThreadRef::new`. Returns the
    /// live resolution computed immediately after creation (normally
    /// `Current`, but honestly reported like any other read since nothing
    /// prevents the target from already being stale/missing at open time).
    pub fn open_thread(
        &self,
        room_id: OpaqueId,
        anchor: medscale_contracts::collaboration::AnchorTarget,
    ) -> Result<ResolvedThread, AuthorityError> {
        self.require_membership(&room_id)?;
        let thread_id = self
            .meta
            .alloc_collab_id("collab-thread", "thread")
            .map_err(meta_err)?;
        let thread = ThreadRef::new(header(&self.realm, &self.scope, thread_id), room_id, anchor)
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        self.meta
            .insert_thread_with_activity(
                &thread,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        let resolution = self.resolve_anchor_artifact(&thread.anchor.artifact);
        Ok((thread, resolution))
    }

    /// Membership-gated thread read with live `ReferenceResolution`
    /// (`contracts.md` section 6): recomputed on every read, never cached.
    pub fn get_thread(&self, id: &OpaqueId) -> Result<ResolvedThread, AuthorityError> {
        let thread = self.scoped_thread(id)?;
        self.require_membership(&thread.room_id)?;
        let resolution = self.resolve_anchor_artifact(&thread.anchor.artifact);
        Ok((thread, resolution))
    }

    /// Lists threads in one room with each thread's live `ReferenceResolution`
    /// (membership-gated).
    pub fn list_threads(
        &self,
        room_id: &OpaqueId,
        limit: u32,
        cursor: Option<String>,
    ) -> Result<(Vec<ResolvedThread>, Option<String>), AuthorityError> {
        self.require_membership(room_id)?;
        let (threads, next) = self
            .meta
            .list_threads(room_id, limit, cursor.as_deref())
            .map_err(meta_err)?;
        let resolved = threads
            .into_iter()
            .map(|thread| {
                let resolution = self.resolve_anchor_artifact(&thread.anchor.artifact);
                (thread, resolution)
            })
            .collect();
        Ok((resolved, next))
    }

    /// Transitions a thread `Open -> Resolved` or `Resolved -> Reopened`
    /// (membership-gated). Any other transition is a conflict. Returns the
    /// live resolution alongside the updated thread: resolving/reopening a
    /// thread is independent of whether its anchor is still current
    /// (`contracts.md` section 7) -- both facts are surfaced together, never
    /// collapsed.
    pub fn set_thread_status(
        &self,
        id: &OpaqueId,
        expected: u64,
        target: medscale_contracts::collaboration::ThreadStatus,
    ) -> Result<ResolvedThread, AuthorityError> {
        use medscale_contracts::collaboration::ThreadStatus;
        let current = self.scoped_thread(id)?;
        self.require_membership(&current.room_id)?;
        let allowed = matches!(
            (current.status, target),
            (ThreadStatus::Open, ThreadStatus::Resolved)
                | (ThreadStatus::Resolved, ThreadStatus::Reopened)
                | (ThreadStatus::Reopened, ThreadStatus::Resolved)
        );
        if !allowed {
            return Err(AuthorityError::Conflict {
                message: format!(
                    "cannot transition thread from {:?} to {target:?}",
                    current.status
                ),
            });
        }
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (thread, _activity) = self
            .meta
            .set_thread_status_with_activity(
                id,
                expected,
                target,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        let resolution = self.resolve_anchor_artifact(&thread.anchor.artifact);
        Ok((thread, resolution))
    }

    // ----- messages -----

    fn scoped_message(
        &self,
        id: &OpaqueId,
    ) -> Result<medscale_contracts::collaboration::Message, AuthorityError> {
        let message = self.meta.get_message(id).map_err(meta_err)?;
        if message.header.realm_id != self.realm || message.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(message)
    }

    /// Posts a message to a thread (membership-gated via the thread's room).
    pub fn post_message(
        &self,
        thread_id: &OpaqueId,
        body: String,
    ) -> Result<medscale_contracts::collaboration::Message, AuthorityError> {
        let thread = self.scoped_thread(thread_id)?;
        let caller = self.require_membership(&thread.room_id)?;
        let candidate = medscale_contracts::collaboration::Message {
            header: header(&self.realm, &self.scope, OpaqueId::new("validate-only")),
            thread_id: thread_id.clone(),
            author_participant_id: caller.header.id.clone(),
            body: body.clone(),
            seq: 0,
        };
        candidate
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let message_id = self
            .meta
            .alloc_collab_id("collab-message", "message")
            .map_err(meta_err)?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (message, _activity) = self
            .meta
            .insert_message_with_activity(
                header(&self.realm, &self.scope, message_id),
                &thread.room_id,
                thread_id,
                &caller.header.id,
                &body,
                header(&self.realm, &self.scope, activity_id),
            )
            .map_err(meta_err)?;
        Ok(message)
    }

    /// Lists messages in one thread (membership-gated).
    pub fn list_messages(
        &self,
        thread_id: &OpaqueId,
        limit: u32,
        after_seq: u64,
    ) -> Result<Vec<medscale_contracts::collaboration::Message>, AuthorityError> {
        let thread = self.scoped_thread(thread_id)?;
        self.require_membership(&thread.room_id)?;
        self.meta
            .list_messages(thread_id, limit, after_seq)
            .map_err(meta_err)
    }

    /// Edits a message's body. Only the original author may edit or delete
    /// their own message; membership alone is not sufficient (this is a
    /// stricter check than the generic room-visibility gate).
    pub fn edit_message(
        &self,
        message_id: &OpaqueId,
        new_body: String,
    ) -> Result<medscale_contracts::collaboration::MessageEdit, AuthorityError> {
        self.edit_message_inner(
            message_id,
            medscale_contracts::collaboration::MessageEditKind::BodyReplace { new_body },
        )
    }

    /// Deletes a message (append-only tombstone; the original row and its
    /// full edit history remain intact for audit).
    pub fn delete_message(
        &self,
        message_id: &OpaqueId,
    ) -> Result<medscale_contracts::collaboration::MessageEdit, AuthorityError> {
        self.edit_message_inner(
            message_id,
            medscale_contracts::collaboration::MessageEditKind::Delete,
        )
    }

    fn edit_message_inner(
        &self,
        message_id: &OpaqueId,
        kind: medscale_contracts::collaboration::MessageEditKind,
    ) -> Result<medscale_contracts::collaboration::MessageEdit, AuthorityError> {
        let message = self.scoped_message(message_id)?;
        let thread = self.scoped_thread(&message.thread_id)?;
        let caller = self.require_membership(&thread.room_id)?;
        if message.author_participant_id != caller.header.id {
            return Err(AuthorityError::Unauthorized);
        }
        kind.validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let edit_id = self
            .meta
            .alloc_collab_id("collab-message-edit", "msgedit")
            .map_err(meta_err)?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (edit, _activity) = self
            .meta
            .insert_message_edit_with_activity(
                header(&self.realm, &self.scope, edit_id),
                &thread.room_id,
                message_id,
                &kind,
                &caller.header.id,
                header(&self.realm, &self.scope, activity_id),
            )
            .map_err(meta_err)?;
        Ok(edit)
    }

    // ----- tasks -----

    fn scoped_task(&self, id: &OpaqueId) -> Result<Task, AuthorityError> {
        let task = self.meta.get_task(id).map_err(meta_err)?;
        if task.header.realm_id != self.realm || task.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(task)
    }

    /// Creates a task, optionally anchored to an artifact (membership-gated).
    pub fn create_task(
        &self,
        room_id: OpaqueId,
        anchor: Option<medscale_contracts::collaboration::AnchorTarget>,
        title: String,
        description: Option<String>,
    ) -> Result<Task, AuthorityError> {
        self.require_membership(&room_id)?;
        let task_id = self
            .meta
            .alloc_collab_id("collab-task", "task")
            .map_err(meta_err)?;
        let task = Task::new(
            header(&self.realm, &self.scope, task_id),
            room_id,
            anchor,
            title,
            description,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        self.meta
            .insert_task_with_activity(
                &task,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        Ok(task)
    }

    /// Membership-gated task read.
    pub fn get_task(&self, id: &OpaqueId) -> Result<Task, AuthorityError> {
        let task = self.scoped_task(id)?;
        self.require_membership(&task.room_id)?;
        Ok(task)
    }

    /// Lists tasks in one room (membership-gated).
    pub fn list_tasks(
        &self,
        room_id: &OpaqueId,
        limit: u32,
        cursor: Option<String>,
    ) -> Result<(Vec<Task>, Option<String>), AuthorityError> {
        self.require_membership(room_id)?;
        self.meta
            .list_tasks(room_id, limit, cursor.as_deref())
            .map_err(meta_err)
    }

    /// Compare-and-swap task status/assignee (membership-gated).
    pub fn update_task(
        &self,
        id: &OpaqueId,
        expected: u64,
        status: medscale_contracts::collaboration::TaskStatus,
        assignee_participant_id: Option<OpaqueId>,
    ) -> Result<Task, AuthorityError> {
        let current = self.scoped_task(id)?;
        self.require_membership(&current.room_id)?;
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (task, _activity) = self
            .meta
            .update_task_with_activity(
                id,
                expected,
                status,
                assignee_participant_id.as_ref(),
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        Ok(task)
    }

    // ----- notes (conflict-copy, not silent merge) -----

    fn scoped_note(
        &self,
        id: &OpaqueId,
    ) -> Result<medscale_contracts::collaboration::NoteDocument, AuthorityError> {
        let note = self.meta.get_note(id).map_err(meta_err)?;
        if note.header.realm_id != self.realm || note.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(note)
    }

    /// Creates a note document with its initial (revision-1) body
    /// (membership-gated).
    pub fn create_note(
        &self,
        room_id: OpaqueId,
        title: String,
        body: String,
    ) -> Result<medscale_contracts::collaboration::NoteDocument, AuthorityError> {
        let caller = self.require_membership(&room_id)?;
        let note_id = self
            .meta
            .alloc_collab_id("collab-note", "note")
            .map_err(meta_err)?;
        let note = medscale_contracts::collaboration::NoteDocument::new(
            header(&self.realm, &self.scope, note_id.clone()),
            room_id,
            title,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let revision_id = self
            .meta
            .alloc_collab_id("collab-note-revision", "noterev")
            .map_err(meta_err)?;
        let initial_revision = medscale_contracts::collaboration::NoteRevision {
            header: header(&self.realm, &self.scope, revision_id),
            note_id: note_id.clone(),
            revision: note.revision,
            parent_revision: None,
            body,
            author_participant_id: caller.header.id,
            conflict_of: None,
        };
        initial_revision
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        self.meta
            .insert_note_with_activity(
                &note,
                &initial_revision,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        Ok(note)
    }

    /// Membership-gated note-document read.
    pub fn get_note(
        &self,
        id: &OpaqueId,
    ) -> Result<medscale_contracts::collaboration::NoteDocument, AuthorityError> {
        let note = self.scoped_note(id)?;
        self.require_membership(&note.room_id)?;
        Ok(note)
    }

    /// Lists the full revision history of one note, including conflict
    /// copies (membership-gated).
    pub fn list_note_revisions(
        &self,
        note_id: &OpaqueId,
    ) -> Result<Vec<medscale_contracts::collaboration::NoteRevision>, AuthorityError> {
        let note = self.scoped_note(note_id)?;
        self.require_membership(&note.room_id)?;
        self.meta.list_note_revisions(note_id).map_err(meta_err)
    }

    /// Edits a note. `expected` matching the current pointer is a fast-
    /// forward edit; a mismatch creates an explicit conflict copy that
    /// preserves the caller's content instead of rejecting the write
    /// (`contracts.md` section 10). Returns
    /// `(note_document, new_revision, is_conflict_copy)`.
    pub fn edit_note(
        &self,
        note_id: &OpaqueId,
        expected: u64,
        body: String,
    ) -> Result<
        (
            medscale_contracts::collaboration::NoteDocument,
            medscale_contracts::collaboration::NoteRevision,
            bool,
        ),
        AuthorityError,
    > {
        let current = self.scoped_note(note_id)?;
        let caller = self.require_membership(&current.room_id)?;
        let revision_id = self
            .meta
            .alloc_collab_id("collab-note-revision", "noterev")
            .map_err(meta_err)?;
        if current.is_fast_forward(expected) {
            let new_revision = medscale_contracts::collaboration::NoteRevision {
                header: header(&self.realm, &self.scope, revision_id),
                note_id: note_id.clone(),
                revision: expected.saturating_add(1),
                parent_revision: Some(expected),
                body,
                author_participant_id: caller.header.id,
                conflict_of: None,
            };
            new_revision
                .validate()
                .map_err(|message| AuthorityError::InvalidArgument { message })?;
            let activity_id = self
                .meta
                .alloc_collab_id("collab-activity", "activity")
                .map_err(meta_err)?;
            let (note, _activity) = self
                .meta
                .apply_note_fast_forward_with_activity(
                    note_id,
                    expected,
                    &new_revision,
                    header(&self.realm, &self.scope, activity_id),
                )
                .map_err(meta_err)?;
            Ok((note, new_revision, false))
        } else {
            // The row currently backing NoteDocument.revision is the unique
            // fast-forward-chain row (conflict_of IS NULL) at that revision
            // number: conflict copies reuse an old, already-superseded
            // revision number but always carry conflict_of = Some(..), so
            // this lookup is unambiguous even after repeated conflicts.
            let history = self.meta.list_note_revisions(note_id).map_err(meta_err)?;
            let current_row = history
                .iter()
                .find(|r| r.conflict_of.is_none() && r.revision == current.revision)
                .ok_or(AuthorityError::Internal {
                    message: "note has no current fast-forward revision row".to_owned(),
                })?;
            let conflict_copy = medscale_contracts::collaboration::NoteRevision {
                header: header(&self.realm, &self.scope, revision_id),
                note_id: note_id.clone(),
                revision: expected,
                parent_revision: Some(expected),
                body,
                author_participant_id: caller.header.id,
                conflict_of: Some(current_row.header.id.clone()),
            };
            conflict_copy
                .validate()
                .map_err(|message| AuthorityError::InvalidArgument { message })?;
            let activity_id = self
                .meta
                .alloc_collab_id("collab-activity", "activity")
                .map_err(meta_err)?;
            self.meta
                .insert_note_conflict_copy_with_activity(
                    &current.room_id,
                    note_id,
                    &conflict_copy,
                    header(&self.realm, &self.scope, activity_id),
                )
                .map_err(meta_err)?;
            Ok((current, conflict_copy, true))
        }
    }

    // ----- approval requests and decisions -----

    fn scoped_approval_request(
        &self,
        id: &OpaqueId,
    ) -> Result<medscale_contracts::collaboration::ApprovalRequest, AuthorityError> {
        let request = self.meta.get_approval_request(id).map_err(meta_err)?;
        if request.header.realm_id != self.realm || request.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(request)
    }

    /// Creates an approval request (membership-gated). Supports multiple
    /// independent assignees (dual/blinded review) and an optional
    /// `blind_until_closed` read-time filter.
    #[allow(clippy::too_many_arguments)]
    pub fn create_approval_request(
        &self,
        room_id: OpaqueId,
        anchor: medscale_contracts::collaboration::AnchorTarget,
        kind: medscale_contracts::collaboration::ApprovalKind,
        assignee_participant_ids: Vec<OpaqueId>,
        blind_until_closed: bool,
    ) -> Result<medscale_contracts::collaboration::ApprovalRequest, AuthorityError> {
        let caller = self.require_membership(&room_id)?;
        for assignee in &assignee_participant_ids {
            let participant = self.scoped_participant(assignee)?;
            if participant.status != ParticipantStatus::Active {
                return Err(AuthorityError::Conflict {
                    message: "assignee participant is not active".to_owned(),
                });
            }
        }
        let request_id = self
            .meta
            .alloc_collab_id("collab-approval-request", "approval")
            .map_err(meta_err)?;
        let request = medscale_contracts::collaboration::ApprovalRequest::new(
            header(&self.realm, &self.scope, request_id),
            room_id,
            anchor,
            kind,
            caller.header.id,
            assignee_participant_ids,
            blind_until_closed,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        self.meta
            .insert_approval_request_with_activity(
                &request,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        Ok(request)
    }

    /// Membership-gated approval-request read.
    pub fn get_approval_request(
        &self,
        id: &OpaqueId,
    ) -> Result<medscale_contracts::collaboration::ApprovalRequest, AuthorityError> {
        let request = self.scoped_approval_request(id)?;
        self.require_membership(&request.room_id)?;
        Ok(request)
    }

    /// Withdraws an open approval request (membership-gated; only the
    /// original requester may withdraw).
    pub fn withdraw_approval_request(
        &self,
        id: &OpaqueId,
        expected: u64,
    ) -> Result<medscale_contracts::collaboration::ApprovalRequest, AuthorityError> {
        let current = self.scoped_approval_request(id)?;
        let caller = self.require_membership(&current.room_id)?;
        if current.requested_by_participant_id != caller.header.id {
            return Err(AuthorityError::Unauthorized);
        }
        if current.status != medscale_contracts::collaboration::ApprovalRequestStatus::Open {
            return Err(AuthorityError::Conflict {
                message: "approval request is not open".to_owned(),
            });
        }
        let actor_id = self.actor()?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (request, _activity) = self
            .meta
            .withdraw_approval_request_with_activity(
                id,
                expected,
                header(&self.realm, &self.scope, activity_id),
                &actor_id,
            )
            .map_err(meta_err)?;
        Ok(request)
    }

    /// Records a decision against an open approval request. The caller must
    /// be an assignee; multiple decisions per request are expected (dual/
    /// independent review) and this never triggers any effect/action/
    /// proposal-promotion path (`security.md` T1) -- it is a pure
    /// collaboration-table write plus the `ActivityRecord` append above.
    pub fn decide_approval_request(
        &self,
        request_id: &OpaqueId,
        outcome: medscale_contracts::collaboration::ApprovalDecisionOutcome,
        rationale: Option<String>,
    ) -> Result<medscale_contracts::collaboration::ApprovalDecision, AuthorityError> {
        let request = self.scoped_approval_request(request_id)?;
        let caller = self.require_membership(&request.room_id)?;
        if request.status != medscale_contracts::collaboration::ApprovalRequestStatus::Open {
            return Err(AuthorityError::Conflict {
                message: "approval request is not open".to_owned(),
            });
        }
        if !request.is_assignee(&caller.header.id) {
            return Err(AuthorityError::Unauthorized);
        }
        let decision_id = self
            .meta
            .alloc_collab_id("collab-approval-decision", "decision")
            .map_err(meta_err)?;
        let activity_id = self
            .meta
            .alloc_collab_id("collab-activity", "activity")
            .map_err(meta_err)?;
        let (decision, _activity) = self
            .meta
            .insert_approval_decision_with_activity(
                header(&self.realm, &self.scope, decision_id),
                &request.room_id,
                request_id,
                &caller.header.id,
                outcome,
                rationale.as_deref(),
                header(&self.realm, &self.scope, activity_id),
            )
            .map_err(meta_err)?;
        Ok(decision)
    }

    /// Lists every decision recorded against one request, filtered by the
    /// request's `blind_until_closed` policy relative to the caller
    /// (`ApprovalRequest::hides_decision_from`; `contracts.md` section 11).
    pub fn list_approval_decisions(
        &self,
        request_id: &OpaqueId,
    ) -> Result<Vec<medscale_contracts::collaboration::ApprovalDecision>, AuthorityError> {
        let request = self.scoped_approval_request(request_id)?;
        let caller = self.require_membership(&request.room_id)?;
        let decisions = self
            .meta
            .list_approval_decisions(request_id)
            .map_err(meta_err)?;
        Ok(decisions
            .into_iter()
            .filter(|d| {
                !request.hides_decision_from(&d.decided_by_participant_id, &caller.header.id)
            })
            .collect())
    }

    // ----- activity (read-only) -----

    /// Lists activity records for one room, in `seq` order (membership-
    /// gated). This is the same durable log `verify_activity_chain` walks;
    /// no separate feed store exists.
    pub fn list_activity(
        &self,
        room_id: &OpaqueId,
        limit: u32,
        after_seq: u64,
    ) -> Result<Vec<medscale_contracts::collaboration::ActivityRecord>, AuthorityError> {
        self.require_membership(room_id)?;
        self.meta
            .list_activity_records(room_id, limit, after_seq)
            .map_err(meta_err)
    }
}

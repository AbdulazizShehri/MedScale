# Contracts — Spec 076 Collaboration Substrate

**Status:** `CONTRACT_FREEZE_INPUT`
**Rule:** the implementer must reconcile these semantics against the exact live Rust types during T076-01, make only compatibility-minimizing adjustments, then mark the resulting field layout `FROZEN_FOR_076` before storage implementation begins.

This file closes semantic ambiguity. It does not authorize creation of a parallel ID, provenance, audit, authority, or clinical-truth system. Collaboration is a new *plane* over existing MedScale objects, never a second store of canonical facts.

## 1. Existing primitives are authoritative

Reuse the current MedScale primitives wherever their semantics fit:

```text
OpaqueId
ObjectHeader
DigestSha256
RealmId
AuthorityScopeId
medscale_contracts::project_graph::{ProjectRevision, check_revision, initial_revision}
medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding, ReferenceResolution}
medscale_contracts::text::{TextSpan, TextRepresentation, CoordinateSystem}
medscale_contracts::envelopes::AuthorityError
```

Do not introduce `RoomId`, `ThreadId`, `MessageId`, `TaskId`, `NoteId`, `ApprovalId` wrappers. Every collaboration object's identity is `header.id: OpaqueId`, exactly like Spec 074/075. Do not introduce a second revision type: mutable 076 rows reuse `ProjectRevision`/`check_revision`/`initial_revision` verbatim, the same reuse 075 already established.

Collaboration never stores clinical/research payload bytes. A comment, task, or approval references a canonical artifact by `ArtifactDescriptor`; it never copies or re-derives the artifact's content.

## 2. One authority path (no second authority system)

```text
CLI/Desktop
   -> typed Core command/query (Capability-checked, session-checked)
      -> RoomMembership visibility check (collaboration-scope, not Core authority)
         -> input + revision validation
            -> encrypted storage transaction
               -> ActivityRecord (audit/activity) append
```

`RoomMembership` governs **collaboration visibility only** (who can read/write collaboration state inside a room). It never grants, widens, or substitutes for a Core `Capability` grant. A participant with `RoomMembership` but no Core session capability still gets `AuthorityError::Unauthorized`/`SessionDenied` from Core; a participant with Core capabilities but no `RoomMembership` in a room gets a collaboration-scope denial. Both checks are required; neither implies the other.

An `ApprovalDecision` is a **collaboration-plane record only**. It never calls `PromoteProposal`, `TransitionEffect`, `CreateExternalActionIntent`, or any existing effect/action path. A human approving something inside Collaboration does not cause any clinical, research-authority, or external-system effect; that always requires the caller to separately invoke the existing, already-gated Core capability for that effect. See `security.md` T1.

## 3. Revision model

Mutable 076 state (`Room`, `RoomMembership`, `ThreadRef`, `Task`, `ApprovalRequest`, the `NoteDocument` pointer) reuses `medscale_contracts::project_graph::ProjectRevision` (`u64`) with the existing `check_revision`/`initial_revision` helpers:

- create starts at revision `1`;
- every successful mutation increments by exactly one;
- mutation supplies `expected_revision`; mismatch returns `AuthorityError::Conflict` and does not write — **except** `NoteDocument`, which uses conflict-copy semantics instead of a bare reject (section 8);
- immutable append rows (`Message`, `MessageEdit`, `NoteRevision`, `ApprovalDecision`, `ActivityRecord`) never carry a mutable revision; their order is a monotonic `seq` assigned by storage, never client-supplied.

## 4. ParticipantIdentity and ParticipantKind

```text
ParticipantKind = Human | Service | Agent

ParticipantIdentity {
    header: ObjectHeader,
    revision: ProjectRevision,
    holder_id: OpaqueId,        // the same audit/session holder identity Spec 074 FR-001 already uses
    kind: ParticipantKind,
    display_name: String,
    status: ParticipantStatus,  // Active | Revoked
}

AgentParticipantIdentity {
    participant_id: OpaqueId,          // -> ParticipantIdentity.header.id where kind == Agent
    agent_profile_ref: Option<OpaqueId>, // opaque forward reference; NOT resolved in 076 (Spec 077 territory)
}
```

Invariants:

- `kind` is set at creation and is immutable; a participant that changes role type is revoked and a new `ParticipantIdentity` is created (no silent kind mutation, so authority history stays honest);
- `AgentParticipantIdentity` exists **only** to carry a forward-compatible, unresolved pointer toward the future Spec 077 `AgentIdentity`. Spec 076 does not read, validate, or dereference `agent_profile_ref`; it is opaque bytes today;
- `kind == Agent` grants **zero** additional Core capability. Agent participation is visible and distinguishable everywhere (messages, tasks, decisions, activity) but never implies human approval or clinical/research authority. Any policy that would treat an `Agent`-authored `ApprovalDecision` as sufficient for a clinical/authority effect is a defect, not a scope question;
- `holder_id` binds a `ParticipantIdentity` to the same actor identity already flowing through `SessionRegistry::holder_of`; 076 does not invent a second actor-identity concept.

## 5. Room and RoomMembership

```text
RoomStatus = Active | Archived

Room {
    header: ObjectHeader,
    revision: ProjectRevision,
    project_id: OpaqueId,         // every Room is scoped to exactly one Spec 074 Project
    experiment_id: Option<OpaqueId>,
    name: String,                 // bounded, non-empty after trim
    status: RoomStatus,
}

MembershipRole = Owner | Member
MembershipStatus = Active | Removed

RoomMembership {
    header: ObjectHeader,
    revision: ProjectRevision,
    room_id: OpaqueId,
    participant_id: OpaqueId,
    role: MembershipRole,
    status: MembershipStatus,
}
```

Invariants:

- `project_id` must resolve to an existing Project in the same realm/authority scope (reuses Spec 074 Project existence check; no duplicate Project concept);
- Room archive retains every thread/message/task/note/approval/activity for inspection; archive never cascades delete;
- `RoomMembership` removal is a status tombstone, never a row delete: past messages/decisions keep their real author identity even after removal;
- duplicate active membership for the same `(room_id, participant_id)` is rejected, not silently merged.

## 6. Anchoring: AnchorTarget, AnchorDetail

```text
AnchorDetail =
    TextRegion(TextSpan)                       // reuses medscale_contracts::text::TextSpan
  | DatasetCell { row_index: u64, column: String }  // forward-compatible with Spec 075 SnapshotRowPage; not resolved/validated against live snapshot rows in 076

AnchorTarget {
    artifact: ArtifactDescriptor,   // reused verbatim from project_graph; exact object_id/kind/binding
    detail: Option<AnchorDetail>,
}
```

This is the **exact artifact-revision anchoring** primitive every comment, thread, task-with-target, and approval request uses. It introduces no new pinning mechanism: `ArtifactDescriptor.binding: ArtifactVersionBinding` is the same `IdentityOnly | Digest | Revision | DigestAndRevision` vocabulary Spec 074 already froze.

Resolution is **never stored as a cached fact**. Every read of a `ThreadRef`/`ApprovalRequest`/anchored `Task` recomputes `ReferenceResolution` (`Current | Stale | Missing | UnsupportedKind | Denied | Corrupt`, reused from `project_graph`) against the artifact's live current version at query time. This is how "a later revision must not silently make an old review appear current" is satisfied with zero new machinery: a thread anchored to `Revision("v1")` is reported `Stale` the instant the artifact's current revision moves past `v1`, exactly like a Spec 074 `ResolvedArtifactRef`.

## 7. ThreadRef

```text
ThreadStatus = Open | Resolved | Reopened

ThreadRef {
    header: ObjectHeader,
    revision: ProjectRevision,
    room_id: OpaqueId,
    anchor: AnchorTarget,
    status: ThreadStatus,
}
```

- `Resolved -> Reopened` and `Open -> Resolved` are explicit status transitions, revision-guarded like any other mutable row;
- resolving a thread does not touch the anchored artifact and has no clinical/external effect;
- a thread's `ReferenceResolution` (section 6) is independent of `ThreadStatus`: a thread can be `Resolved` and simultaneously report `Stale` if the artifact moved after resolution — both facts are shown, never collapsed.

## 8. Message and MessageEdit

```text
MessageStatus computed, not stored: a message with a MessageEdit{kind: Delete} in its
edit log is Deleted; otherwise Active. Storage never overwrites the original row.

Message {
    header: ObjectHeader,           // no revision field: immutable once created
    thread_id: OpaqueId,
    author_participant_id: OpaqueId,
    body: String,                   // bounded byte length, explicit constant
    seq: u64,                       // monotonic append order within the thread
}

MessageEditKind = BodyReplace { new_body: String } | Delete

MessageEdit {
    header: ObjectHeader,           // immutable append row
    message_id: OpaqueId,
    kind: MessageEditKind,
    edited_by_participant_id: OpaqueId,
    seq: u64,                       // monotonic per message
}
```

- messages are append-only; body mutation is never in-place SQL `UPDATE` on the original row;
- the "current" displayed body is the latest `BodyReplace` (or absence, meaning the original body), and `Delete` clears display content while the row and its edit history remain intact for audit;
- deleting a message never deletes the underlying row or edit log; it is a typed status, matching `RoomMembership`'s tombstone discipline.

## 9. Task

```text
TaskStatus = Open | InProgress | Blocked | Done | Cancelled

Task {
    header: ObjectHeader,
    revision: ProjectRevision,
    room_id: OpaqueId,
    anchor: Option<AnchorTarget>,
    title: String,
    description: Option<String>,
    assignee_participant_id: Option<OpaqueId>,
    status: TaskStatus,
}
```

- update supplies `expected_revision`; mismatch -> `Conflict`, no write (satisfies the "stale task update -> Conflict" requirement directly);
- scheduling/due-dates/calendars are explicitly out of scope for this foundation (section 12);
- a `Task` with no `anchor` is a plain room to-do; a `Task` with `anchor` set is how later specs build review/annotation task queues without a new contract.

## 10. NoteDocument and NoteRevision (conflict-copy, not silent merge)

```text
NoteStatus = Active | Archived

NoteDocument {
    header: ObjectHeader,
    revision: ProjectRevision,      // pointer revision: which NoteRevision is "current"
    room_id: OpaqueId,
    title: String,
    status: NoteStatus,
}

NoteRevision {
    header: ObjectHeader,           // immutable append row
    note_id: OpaqueId,
    revision: ProjectRevision,      // the NoteDocument.revision this write produced/targeted
    parent_revision: Option<ProjectRevision>, // revision this edit was based on; None only for revision 1
    body: String,
    author_participant_id: OpaqueId,
    conflict_of: Option<OpaqueId>,  // Some(other NoteRevision id) when this is a conflict copy, not a fast-forward edit
}
```

Update rule (the one deliberate departure from plain `Conflict`-and-reject):

1. caller supplies `expected_revision` (the `parent_revision` they edited from) and `body`;
2. if `expected_revision == NoteDocument.revision`, this is a normal fast-forward edit: a new `NoteRevision` is appended, `conflict_of = None`, and `NoteDocument.revision` advances;
3. if `expected_revision != NoteDocument.revision` (someone else's edit landed first, e.g. after an offline period), Core still appends a **new** `NoteRevision` carrying the caller's `body`, with `conflict_of` set to the `NoteRevision` id that is currently current. `NoteDocument.revision` does **not** advance for a conflict copy. The typed result is `NoteConflictCopyCreated { current: NoteRevision, conflict_copy: NoteRevision }`, never a bare `Conflict` error — **the caller's content is never discarded.**
4. resolving the conflict (making the conflict copy current, or discarding it) is an explicit later mutation by a participant, never automatic.

This is the required "conflict-copy/user-resolution behavior for note documents" and directly proves "offline note conflict -> both revisions preserved" without any CRDT machinery.

## 11. ApprovalRequest and ApprovalDecision

```text
ApprovalKind = Review | Approval | Adjudication
ApprovalRequestStatus = Open | Withdrawn | Closed

ApprovalRequest {
    header: ObjectHeader,
    revision: ProjectRevision,
    room_id: OpaqueId,
    anchor: AnchorTarget,
    kind: ApprovalKind,
    requested_by_participant_id: OpaqueId,
    assignee_participant_ids: Vec<OpaqueId>,
    blind_until_closed: bool,      // when true, one assignee's decisions are not readable by another assignee while status == Open
    status: ApprovalRequestStatus,
}

ApprovalDecisionOutcome = Approved | Rejected | ChangesRequested | Abstained
ApprovalDecision {
    header: ObjectHeader,          // immutable append row; multiple decisions per request are expected (dual review)
    request_id: OpaqueId,
    decided_by_participant_id: OpaqueId,
    outcome: ApprovalDecisionOutcome,
    rationale: Option<String>,
    seq: u64,                      // monotonic per request
}
```

Invariants:

- an `ApprovalDecision` is **never** self-executing: recording it changes only `ApprovalRequest`/`ApprovalDecision` rows and appends `ActivityRecord`. It never mutates the anchored artifact, never calls `PromoteProposal`/`TransitionEffect`/action-intent creation. See section 2 and `security.md` T1;
- multiple `ApprovalDecision` rows for the same `request_id` are how dual independent screening and later adjudication are built: two reviewers each decide once; a later owning spec (systematic-review product) computes agreement/disagreement by reading both and, on disagreement, opens a further `ApprovalRequest{kind: Adjudication}` anchored to the same target. No 076 redesign is needed for this;
- `blind_until_closed` is enforced as a **read-time filter** in Core: while `status == Open`, a `GET`/list caller who is one of the `assignee_participant_ids` but not `requested_by_participant_id` sees only their own decisions, never co-assignees'. This is how blinded review is supported without a contract change later;
- `ApprovalRequestStatus::Withdrawn` is explicit and distinct from a decision outcome; withdrawing never fabricates a decision.

## 12. Explicit non-goals reflected in the contracts

- no scheduling/calendar/due-date fields on `Task`;
- no rich-text/canvas document model for `NoteDocument` beyond a plain bounded `String` body; a structured canvas is a later spec's contract, not 076's;
- no CRDT type anywhere; `NoteRevision` conflict-copy is the only non-reject-on-stale mutation in 076;
- no Hub network client, no actual sync execution behind `SyncCursor` (section 14);
- no free-form RBAC engine: `MembershipRole` is `Owner | Member` only, used solely for room-management operations (rename/archive/remove-member), never for Core capability grants.

## 13. PresenceEvent — ephemeral, never durable

```text
PresenceState = Viewing | Typing | Idle

PresenceEvent {
    participant_id: OpaqueId,
    room_id: OpaqueId,
    thread_id: Option<OpaqueId>,
    state: PresenceState,
    observed_at_tick: u64,   // SessionRegistry-style monotonic tick
}
```

`PresenceEvent` is held only in an in-process registry (same `Mutex<State>` shape as `SessionRegistry`), scoped to the open vault process. It is **never written to encrypted storage**, never migrated, never backed up, and is lost on restart by construction. This satisfies "ephemeral presence separated from durable history" with no new attack surface: there is nothing durable to tamper with.

## 14. ActivityRecord and CollabEventEnvelope (one audit trail, not two)

`CollabEventEnvelope` is the **contract concept**, not a second persisted table. Every 076 mutation Core accepts (`RoomCreate`, `MessagePost`, `TaskUpdate`, `ApprovalDecide`, ...) is normalized into one envelope shape before validation and storage:

```text
CollabEventEnvelope {
    schema_version: u32,
    room_id: OpaqueId,
    actor_participant_id: OpaqueId,
    actor_kind: ParticipantKind,     // must equal the live ParticipantIdentity.kind or Core rejects (Invalid)
    event_kind: CollabEventKind,
    seq: u64,
}

CollabEventKind =
    RoomCreated | RoomArchived
  | MembershipAdded | MembershipRemoved
  | ThreadOpened | ThreadResolved | ThreadReopened
  | MessagePosted | MessageEdited | MessageDeleted
  | TaskCreated | TaskUpdated
  | NoteCreated | NoteRevised | NoteConflictCopyCreated
  | ApprovalRequested | ApprovalDecided | ApprovalWithdrawn
  | ParticipantRegistered | ParticipantRevoked
```

Its durable projection is `ActivityRecord`, which **is** the tamper-evident audit trail and the activity-feed data source at once (one log, not a log plus a duplicate feed):

```text
ActivityRecord {
    header: ObjectHeader,            // immutable append row
    room_id: OpaqueId,
    actor_participant_id: OpaqueId,
    event_kind: CollabEventKind,
    target_object_id: OpaqueId,      // the mutated Room/Thread/Message/Task/Note/ApprovalRequest/Participant id
    checkpoint_digest: DigestSha256, // Sha256(prev_checkpoint_digest_bytes || canonical_encoding(this_record_without_digest))
    seq: u64,                        // monotonic per room
}
```

Tamper-evidence is the same hash-chain idea already used elsewhere for digest-bound evidence: recomputing the chain from `seq = 1` and comparing the stored `checkpoint_digest` at each step detects any row edited or removed out of band. `ActivityRecord` rows are searchable locally (by room, actor, event kind, target) without any new index technology.

**Scope note:** the chain is per-room (`UNIQUE(room_id, seq)`), because every event that appends to it — room lifecycle, membership, threads, messages, tasks, notes, approvals — naturally has one. `ParticipantRegistered`/`ParticipantRevoked` are the one exception: a `ParticipantIdentity` is scoped to `(realm, authority_scope)`, not to any single room, so it has no room to chain into. Those two events are recorded through the *existing* in-memory `ActionAuditRecord` audit trail instead (the same mechanism Spec 074 uses for `project.create`/`project.archive`), not through `collab_activity_records`. This is a deliberate reuse, not an oversight: 076 does not need a second scope-level audit mechanism when one already exists.

## 15. SyncCursor (inert placeholder, no network)

```text
SyncCursor is a computed read, not a stored row: the current value for a room
is simply the room's latest ActivityRecord.seq. Spec 076 defines the name and
the field it will need (last_synced_seq: u64) so a future Spec 084 Hub sync
does not have to renumber history; it stores nothing new, runs no sync, and
opens no network connection. Any RequestBody surfacing it is read-only.
```

This satisfies "later sync through Hub" as a forward-compatible naming/seq decision only. Spec 076 must not implement, schedule, or expose any capability that transmits collaboration data off the local vault.

## 16. Error contract

Reuse `medscale_contracts::envelopes::AuthorityError` as-is. No new variant is required for 076:

```text
Unauthorized / SessionDenied / WrongScope   -> membership or capability denial
NotFound                                    -> unknown room/thread/message/task/note/approval/participant
Conflict { message }                        -> stale expected_revision on Room/RoomMembership/ThreadRef/Task/ApprovalRequest
StaleReference { message }                  -> reserved for anchor resolution reporting at the API edge when a caller demanded Current and got Stale/Missing
InvalidArgument { message }                 -> bounds/charset/empty-field violations, actor_kind mismatch
Internal { message }                        -> unexpected failure only
```

If T076-01 discovers a genuine gap, the addition must be additive and recorded here before storage work begins, exactly like 075's `Cancelled` addition.

## 17. Serialization/versioning

- `#[serde(deny_unknown_fields)]` on every durable/authority-bearing 076 struct, matching current convention;
- `#[serde(rename_all = "snake_case")]` on every enum, matching current convention;
- every durable schema has explicit schema-version ownership (storage schema v5 for 076 rows, see `migration.md`);
- unknown future `CollabEventKind`/`ParticipantKind`/`ApprovalKind` values fail closed, never silently pass through as a known kind;
- JSON CLI output is versionable and tested, matching current `RequestBody`/`ResponseBody` conventions.

## 18. Contract freeze gate

Before T076-02 starts, the implementer must update this file with:

```text
CONTRACT_FREEZE = FROZEN_FOR_076
LIVE_BASE_SHA = <verified canonical ancestor>
CONTRACT_FILES = <exact Rust paths>
REVISION_MODEL = <exact reused type, expected: project_graph::ProjectRevision>
PARTICIPANT_KIND_VOCABULARY = human, service, agent
EVENT_KIND_VOCABULARY = <final bounded CollabEventKind set>
ANCHOR_DETAIL_SET = <final admitted AnchorDetail set>
ERROR_TYPE = medscale_contracts::envelopes::AuthorityError (additive only if proven necessary)
NEW_CAPABILITIES = <exact count and list>
NEW_REQUEST_BODY_VARIANTS = <exact count>
NEW_RESPONSE_BODY_VARIANTS = <exact count>
```

Any later change to those frozen items requires an explicit reason, test-impact review, and migration-impact review before code proceeds.

## 19. Freeze record (T076-01, FROZEN_FOR_076)

```text
CONTRACT_FREEZE = FROZEN_FOR_076
LIVE_BASE_SHA = 6021ff9aad397a8488087cae01e56528370e8211
CONTRACT_FILES = crates/medscale-contracts/src/collaboration.rs,
                 crates/medscale-contracts/src/lib.rs (module registration)
REVISION_MODEL = medscale_contracts::project_graph::ProjectRevision (u64 alias)
                 + check_revision/initial_revision; no new revision type.
                 NoteDocument is the sole exception: check_revision is not
                 used on it directly — NoteDocument::is_fast_forward(expected)
                 tells the caller (Core, in T076-07) whether to fast-forward
                 or create a conflict-copy NoteRevision instead.
PARTICIPANT_KIND_VOCABULARY = human, service, agent (ParticipantKind; immutable
                 once set on a ParticipantIdentity)
EVENT_KIND_VOCABULARY = room_created, room_archived, membership_added,
                 membership_removed, thread_opened, thread_resolved,
                 thread_reopened, message_posted, message_edited,
                 message_deleted, task_created, task_updated, note_created,
                 note_revised, note_conflict_copy_created, approval_requested,
                 approval_decided, approval_withdrawn, participant_registered,
                 participant_revoked (20 variants; CollabEventKind)
ANCHOR_DETAIL_SET = text_region { span: TextSpan } (reuses contracts::text::TextSpan),
                 dataset_cell { row_index: u64, column: String } (forward-compatible
                 with Spec 075 SnapshotRowPage; not resolved/validated in 076)
ERROR_TYPE = medscale_contracts::envelopes::AuthorityError, unchanged — no new
                 variant was required. NoteDocument's conflict-copy path does
                 not use AuthorityError::Conflict at all (see REVISION_MODEL
                 above); every other mutable row (Room, RoomMembership,
                 ThreadRef, Task, ApprovalRequest) uses Conflict exactly like
                 Spec 074/075.
NEW_CAPABILITIES = deferred to T076-03 onward (Core dispatch slices), not
                 required to exist for the contracts module itself to compile
                 and pass its own tests. Planned ~28 across
                 Room/RoomMembership/Participant/Thread/Message/Task/Note/
                 Approval/Presence/Activity families; exact list frozen when
                 crates/medscale-contracts/src/envelopes/mod.rs is amended.
NEW_REQUEST_BODY_VARIANTS = deferred to the same T076-03+ slices, same reason.
NEW_RESPONSE_BODY_VARIANTS = deferred to the same T076-03+ slices, same reason.
TEST_FILE = in-module `#[cfg(test)] mod tests` in collaboration.rs (23 tests):
                 participant kind round-trip and immutability, room revision
                 start/increment/conflict, empty/oversized room name
                 rejection, membership duplicate-identity detection, anchor
                 text-region ordering validation, thread creation without a
                 stored resolution field, message body bound, message-edit
                 append semantics (replace + delete), task stale-revision
                 conflict, note fast-forward-vs-conflict detection, note
                 conflict-copy dual-body preservation, approval request
                 empty-assignee rejection, approval request dual-assignee
                 support, blind-until-closed read-time filtering (three
                 cases), activity hash-chain tamper detection, full
                 CollabEventKind vocabulary round-trip, agent participant
                 identity opaque-forward-ref-only behavior.
LOCAL_VERIFICATION = cargo fmt --check -p medscale-contracts and --all both
                 pass (exit 0). cargo check/test cannot run on this
                 workstation (no MSVC linker; see evidence/076-.../README.md);
                 GitHub Actions CI is the authoritative build/test/clippy
                 verification for this freeze, to be recorded in
                 evidence/076-collaboration-substrate/EXACT_HEAD_QUALIFICATION.md
                 once the exact-head PR run completes.
```

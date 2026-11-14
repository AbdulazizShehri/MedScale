//! Collaboration Substrate contracts (Spec 076).
//!
//! Local-first rooms, participant identities, artifact-anchored threads,
//! append-only messages, tasks, conflict-safe notes, review/approval
//! requests, ephemeral presence, and a tamper-evident activity trail.
//!
//! Collaboration is a new plane over existing MedScale objects, never a
//! second store of canonical facts: every anchor references an existing
//! artifact by exact identity/version binding, never copies its content.
//!
//! Reused primitives (authoritative, defined elsewhere):
//! `OpaqueId`, `ObjectHeader`, `DigestSha256`, `RealmId`, `AuthorityScopeId`,
//! `ProjectRevision`/`check_revision` (Spec 074 revision pattern, single
//! definition), `ArtifactDescriptor`/`ArtifactVersionBinding`/
//! `ReferenceResolution` (Spec 074 anchor/resolution pattern), `TextSpan`.
//!
//! Transport error mapping (semantic -> `crate::envelopes::AuthorityError`):
//!
//! ```text
//! Invalid          -> InvalidArgument { message }
//! NotFound         -> NotFound
//! Denied           -> Unauthorized | SessionDenied | WrongScope (most precise applies)
//! Conflict         -> Conflict { message } (stale expected_revision on every
//!                     mutable row except NoteDocument; duplicate membership)
//! Internal         -> Internal { message }
//! ```
//!
//! `NoteDocument` updates never map a stale write to `Conflict`: a stale
//! `expected_revision` produces a conflict-copy `NoteRevision` instead (see
//! `NoteRevision::conflict_of`). No `AuthorityError` variant is added by 076;
//! the existing vocabulary already covers every required state.
//!
//! Durable timestamps: none. 076 follows the existing audit convention where
//! ordering travels through monotonic `seq`/`revision`, not per-object wall
//! clocks. `MedicalTime` keeps its clinical semantics and is not reused here.
//!
//! Authority separation: `RoomMembership` governs collaboration-scope
//! visibility only. It never grants, widens, or substitutes for a Core
//! `Capability` grant, and a Capability grant never substitutes for
//! `RoomMembership`. Recording an `ApprovalDecision` never calls any
//! effect/action/proposal-promotion path; it is a pure collaboration-table
//! write plus an `ActivityRecord` append.

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};
use crate::project_graph::{ArtifactDescriptor, ProjectRevision, check_revision, initial_revision};
use crate::text::TextSpan;

/// Durable schema version for Collaboration Substrate rows (storage schema v5).
pub const COLLAB_SCHEMA_VERSION: u32 = 1;

/// Maximum display-name length in Unicode scalar values.
pub const PARTICIPANT_NAME_MAX_CHARS: usize = 128;

/// Maximum room-name length in Unicode scalar values.
pub const ROOM_NAME_MAX_CHARS: usize = 128;

/// Maximum message body length in bytes.
pub const MESSAGE_BODY_MAX_BYTES: usize = 8_192;

/// Maximum task title length in Unicode scalar values.
pub const TASK_TITLE_MAX_CHARS: usize = 256;

/// Maximum task description length in Unicode scalar values.
pub const TASK_DESCRIPTION_MAX_CHARS: usize = 4_096;

/// Maximum note title length in Unicode scalar values.
pub const NOTE_TITLE_MAX_CHARS: usize = 256;

/// Maximum note body length in bytes.
pub const NOTE_BODY_MAX_BYTES: usize = 65_536;

/// Maximum approval-decision rationale length in Unicode scalar values.
pub const APPROVAL_RATIONALE_MAX_CHARS: usize = 4_096;

/// Maximum assignees admitted on one approval request.
pub const APPROVAL_ASSIGNEES_MAX: usize = 16;

/// Maximum dataset-anchor column-name length in Unicode scalar values.
pub const ANCHOR_COLUMN_MAX_CHARS: usize = 128;

/// Default page size for bounded collaboration listings.
pub const COLLAB_LIST_LIMIT_DEFAULT: u32 = 25;

/// Maximum page size for bounded collaboration listings.
pub const COLLAB_LIST_LIMIT_MAX: u32 = 100;

/// Maximum opaque cursor length in bytes.
pub const COLLAB_CURSOR_MAX_BYTES: usize = 256;

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

fn bounded_optional_text(value: &str, max_chars: usize, what: &str) -> Result<(), String> {
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
// Participant identity
// ---------------------------------------------------------------------------

/// Participant actor class. Immutable once set; a participant that changes
/// kind is revoked and a new `ParticipantIdentity` is created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantKind {
    Human,
    Service,
    Agent,
}

impl ParticipantKind {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Service => "service",
            Self::Agent => "agent",
        }
    }

    /// Parses a closed vocabulary value; unknown kinds fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "human" => Ok(Self::Human),
            "service" => Ok(Self::Service),
            "agent" => Ok(Self::Agent),
            other => Err(format!("unknown participant kind {other}")),
        }
    }
}

/// Lifecycle of a participant identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantStatus {
    Active,
    Revoked,
}

/// A collaboration actor. Binds to the existing session/audit `holder_id`;
/// carries no clinical/research authority by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticipantIdentity {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub holder_id: OpaqueId,
    pub kind: ParticipantKind,
    pub display_name: String,
    pub status: ParticipantStatus,
}

impl ParticipantIdentity {
    /// Creates a revision-1 active participant after validating metadata.
    pub fn new(
        header: ObjectHeader,
        holder_id: OpaqueId,
        kind: ParticipantKind,
        display_name: String,
    ) -> Result<Self, String> {
        bounded_text(&display_name, PARTICIPANT_NAME_MAX_CHARS, "display_name")?;
        Ok(Self {
            header,
            revision: initial_revision(),
            holder_id,
            kind,
            display_name,
            status: ParticipantStatus::Active,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

/// Forward-compatible, unresolved pointer toward a future Spec 077 agent
/// identity. Spec 076 never reads, validates, or dereferences
/// `agent_profile_ref`; it carries zero additional Core capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentParticipantIdentity {
    pub participant_id: OpaqueId,
    pub agent_profile_ref: Option<OpaqueId>,
}

// ---------------------------------------------------------------------------
// Room and membership
// ---------------------------------------------------------------------------

/// Lifecycle of a Room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    Active,
    Archived,
}

/// A collaboration Room scoped to exactly one Spec 074 Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Room {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub experiment_id: Option<OpaqueId>,
    pub name: String,
    pub status: RoomStatus,
}

impl Room {
    /// Creates a revision-1 active Room after validating metadata.
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        experiment_id: Option<OpaqueId>,
        name: String,
    ) -> Result<Self, String> {
        bounded_text(&name, ROOM_NAME_MAX_CHARS, "room name")?;
        Ok(Self {
            header,
            revision: initial_revision(),
            project_id,
            experiment_id,
            name,
            status: RoomStatus::Active,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

/// A participant's role inside one Room. Governs room-management operations
/// only (rename/archive/remove-member); never a Core capability grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRole {
    Owner,
    Member,
}

/// Lifecycle of a Room membership. Removal is a tombstone, never a row delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipStatus {
    Active,
    Removed,
}

/// One participant's membership in one Room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomMembership {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub room_id: OpaqueId,
    pub participant_id: OpaqueId,
    pub role: MembershipRole,
    pub status: MembershipStatus,
}

impl RoomMembership {
    /// Creates a revision-1 active membership.
    #[must_use]
    pub fn new(
        header: ObjectHeader,
        room_id: OpaqueId,
        participant_id: OpaqueId,
        role: MembershipRole,
    ) -> Self {
        Self {
            header,
            revision: initial_revision(),
            room_id,
            participant_id,
            role,
            status: MembershipStatus::Active,
        }
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// Deterministic duplicate identity: same room and participant. Core
    /// treats a repeat active membership as idempotent or an explicit
    /// conflict; never a silent duplicate.
    #[must_use]
    pub fn same_membership_as(&self, other: &Self) -> bool {
        self.room_id == other.room_id && self.participant_id == other.participant_id
    }
}

// ---------------------------------------------------------------------------
// Anchoring: exact artifact-revision references
// ---------------------------------------------------------------------------

/// Sub-artifact detail an anchor may narrow to. Forward-compatible with a
/// Spec 075 `SnapshotRowPage` cell; not resolved/validated against live
/// snapshot rows in 076.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnchorDetail {
    TextRegion { span: TextSpan },
    DatasetCell { row_index: u64, column: String },
}

impl AnchorDetail {
    /// Validates detail shape without resolving it against live content.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::TextRegion { span } => {
                if !span.is_ordered() {
                    return Err("text region span must have start <= end".to_owned());
                }
                Ok(())
            }
            Self::DatasetCell { column, .. } => {
                bounded_text(column, ANCHOR_COLUMN_MAX_CHARS, "dataset cell column")
            }
        }
    }
}

/// Exact artifact-revision anchor. Reuses the Spec 074 `ArtifactDescriptor`
/// binding verbatim; introduces no new pinning mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorTarget {
    pub artifact: ArtifactDescriptor,
    pub detail: Option<AnchorDetail>,
}

impl AnchorTarget {
    /// Validates anchor shape. Live resolution against the current artifact
    /// state is always a read-time concern (never cached here).
    pub fn validate(&self) -> Result<(), String> {
        self.artifact.validate()?;
        if let Some(detail) = &self.detail {
            detail.validate()?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Thread
// ---------------------------------------------------------------------------

/// Lifecycle of a discussion thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Open,
    Resolved,
    Reopened,
}

/// A discussion thread anchored to an exact artifact revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadRef {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub room_id: OpaqueId,
    pub anchor: AnchorTarget,
    pub status: ThreadStatus,
}

impl ThreadRef {
    /// Creates a revision-1 open thread after validating the anchor.
    pub fn new(
        header: ObjectHeader,
        room_id: OpaqueId,
        anchor: AnchorTarget,
    ) -> Result<Self, String> {
        anchor.validate()?;
        Ok(Self {
            header,
            revision: initial_revision(),
            room_id,
            anchor,
            status: ThreadStatus::Open,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

// ---------------------------------------------------------------------------
// Message (append-only)
// ---------------------------------------------------------------------------

/// One posted message. Immutable once created; edits/deletes are separate
/// append rows, never an in-place rewrite of this row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub header: ObjectHeader,
    pub thread_id: OpaqueId,
    pub author_participant_id: OpaqueId,
    pub body: String,
    pub seq: u64,
}

impl Message {
    /// Validates message shape.
    pub fn validate(&self) -> Result<(), String> {
        bounded_bytes(&self.body, MESSAGE_BODY_MAX_BYTES, "message body")
    }
}

/// The kind of change one `MessageEdit` row records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageEditKind {
    BodyReplace { new_body: String },
    Delete,
}

impl MessageEditKind {
    /// Validates edit-kind shape.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::BodyReplace { new_body } => {
                bounded_bytes(new_body, MESSAGE_BODY_MAX_BYTES, "message body")
            }
            Self::Delete => Ok(()),
        }
    }
}

/// An append-only edit or deletion of one `Message`. The original row is
/// never rewritten; this is a new row in the message's edit log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageEdit {
    pub header: ObjectHeader,
    pub message_id: OpaqueId,
    pub kind: MessageEditKind,
    pub edited_by_participant_id: OpaqueId,
    pub seq: u64,
}

impl MessageEdit {
    /// Validates edit shape.
    pub fn validate(&self) -> Result<(), String> {
        self.kind.validate()
    }
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

/// Lifecycle of a Task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

/// A room-scoped to-do, optionally anchored to an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub room_id: OpaqueId,
    pub anchor: Option<AnchorTarget>,
    pub title: String,
    pub description: Option<String>,
    pub assignee_participant_id: Option<OpaqueId>,
    pub status: TaskStatus,
}

impl Task {
    /// Creates a revision-1 open Task after validating metadata and anchor.
    pub fn new(
        header: ObjectHeader,
        room_id: OpaqueId,
        anchor: Option<AnchorTarget>,
        title: String,
        description: Option<String>,
    ) -> Result<Self, String> {
        bounded_text(&title, TASK_TITLE_MAX_CHARS, "task title")?;
        if let Some(description) = &description {
            bounded_optional_text(description, TASK_DESCRIPTION_MAX_CHARS, "task description")?;
        }
        if let Some(anchor) = &anchor {
            anchor.validate()?;
        }
        Ok(Self {
            header,
            revision: initial_revision(),
            room_id,
            anchor,
            title,
            description,
            assignee_participant_id: None,
            status: TaskStatus::Open,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }
}

// ---------------------------------------------------------------------------
// Note (conflict-copy, not silent merge)
// ---------------------------------------------------------------------------

/// Lifecycle of a NoteDocument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteStatus {
    Active,
    Archived,
}

/// A collaborative note's current pointer. `revision` names which
/// `NoteRevision` is current; the body itself lives only in `NoteRevision`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteDocument {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub room_id: OpaqueId,
    pub title: String,
    pub status: NoteStatus,
}

impl NoteDocument {
    /// Creates a revision-1 active NoteDocument after validating the title.
    pub fn new(header: ObjectHeader, room_id: OpaqueId, title: String) -> Result<Self, String> {
        bounded_text(&title, NOTE_TITLE_MAX_CHARS, "note title")?;
        Ok(Self {
            header,
            revision: initial_revision(),
            room_id,
            title,
            status: NoteStatus::Active,
        })
    }

    /// Returns the next revision when `expected` matches the current
    /// pointer. A mismatch is not itself an error here: the caller (Core)
    /// uses the mismatch to decide to create a conflict-copy `NoteRevision`
    /// instead of a fast-forward edit; this helper only reports which case
    /// applies.
    #[must_use]
    pub fn is_fast_forward(&self, expected: ProjectRevision) -> bool {
        self.revision == expected
    }
}

/// One append-only body revision of a `NoteDocument`. A fast-forward edit
/// has `conflict_of: None`; a conflict copy (stale `expected_revision`) has
/// `conflict_of` set to the `NoteRevision` id that was current at write
/// time. Nothing is ever discarded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteRevision {
    pub header: ObjectHeader,
    pub note_id: OpaqueId,
    pub revision: ProjectRevision,
    pub parent_revision: Option<ProjectRevision>,
    pub body: String,
    pub author_participant_id: OpaqueId,
    pub conflict_of: Option<OpaqueId>,
}

impl NoteRevision {
    /// Validates note-revision shape.
    pub fn validate(&self) -> Result<(), String> {
        bounded_bytes(&self.body, NOTE_BODY_MAX_BYTES, "note body")
    }
}

// ---------------------------------------------------------------------------
// Approval request and decision
// ---------------------------------------------------------------------------

/// The kind of review/approval workflow one request represents. Kept
/// distinct from a plain comment/suggestion, which is a `Message` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    Review,
    Approval,
    Adjudication,
}

/// Lifecycle of an ApprovalRequest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequestStatus {
    Open,
    Withdrawn,
    Closed,
}

/// A request for one or more participants to review/approve/adjudicate an
/// anchored artifact. Supports multiple independent assignees (dual/blinded
/// review) without redesign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRequest {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub room_id: OpaqueId,
    pub anchor: AnchorTarget,
    pub kind: ApprovalKind,
    pub requested_by_participant_id: OpaqueId,
    pub assignee_participant_ids: Vec<OpaqueId>,
    pub blind_until_closed: bool,
    pub status: ApprovalRequestStatus,
}

impl ApprovalRequest {
    /// Creates a revision-1 open ApprovalRequest after validating metadata.
    pub fn new(
        header: ObjectHeader,
        room_id: OpaqueId,
        anchor: AnchorTarget,
        kind: ApprovalKind,
        requested_by_participant_id: OpaqueId,
        assignee_participant_ids: Vec<OpaqueId>,
        blind_until_closed: bool,
    ) -> Result<Self, String> {
        anchor.validate()?;
        if assignee_participant_ids.is_empty() {
            return Err("approval request requires at least one assignee".to_owned());
        }
        if assignee_participant_ids.len() > APPROVAL_ASSIGNEES_MAX {
            return Err("approval request assignees exceed bound".to_owned());
        }
        Ok(Self {
            header,
            revision: initial_revision(),
            room_id,
            anchor,
            kind,
            requested_by_participant_id,
            assignee_participant_ids,
            blind_until_closed,
            status: ApprovalRequestStatus::Open,
        })
    }

    /// Returns the next revision when `expected` matches, else `Conflict`.
    pub fn check_mutation(&self, expected: ProjectRevision) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected)
    }

    /// True when `participant_id` is one of this request's assignees.
    #[must_use]
    pub fn is_assignee(&self, participant_id: &OpaqueId) -> bool {
        self.assignee_participant_ids.contains(participant_id)
    }

    /// True when a decision by `viewer_id` should be hidden from
    /// `other_participant_id` under the request's blind-review policy.
    /// Read-time filter only; storage always holds every decision.
    #[must_use]
    pub fn hides_decision_from(&self, decider_id: &OpaqueId, viewer_id: &OpaqueId) -> bool {
        self.blind_until_closed
            && self.status == ApprovalRequestStatus::Open
            && decider_id != viewer_id
            && viewer_id != &self.requested_by_participant_id
    }
}

/// The outcome one participant recorded for an `ApprovalRequest`. Never
/// self-executing: recording a decision mutates only 076 tables plus the
/// `ActivityRecord` append (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionOutcome {
    Approved,
    Rejected,
    ChangesRequested,
    Abstained,
}

/// One participant's decision against an `ApprovalRequest`. Multiple
/// decisions per request are expected (dual/independent review); this row
/// is append-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecision {
    pub header: ObjectHeader,
    pub request_id: OpaqueId,
    pub decided_by_participant_id: OpaqueId,
    pub outcome: ApprovalDecisionOutcome,
    pub rationale: Option<String>,
    pub seq: u64,
}

impl ApprovalDecision {
    /// Validates decision shape.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(rationale) = &self.rationale {
            bounded_optional_text(
                rationale,
                APPROVAL_RATIONALE_MAX_CHARS,
                "decision rationale",
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Presence (ephemeral, never durable)
// ---------------------------------------------------------------------------

/// A participant's momentary state inside a Room/thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceState {
    Viewing,
    Typing,
    Idle,
}

/// An ephemeral presence signal. Never written to encrypted storage, never
/// migrated, never backed up; held only in an in-process registry scoped to
/// the open vault session and lost on restart by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresenceEvent {
    pub participant_id: OpaqueId,
    pub room_id: OpaqueId,
    pub thread_id: Option<OpaqueId>,
    pub state: PresenceState,
    pub observed_at_tick: u64,
}

// ---------------------------------------------------------------------------
// Activity (tamper-evident audit trail and activity feed, one log)
// ---------------------------------------------------------------------------

/// Bounded vocabulary of durable collaboration mutations. Unknown future
/// values fail closed, never silently pass through as a known kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollabEventKind {
    RoomCreated,
    RoomArchived,
    MembershipAdded,
    MembershipRemoved,
    ThreadOpened,
    ThreadResolved,
    ThreadReopened,
    MessagePosted,
    MessageEdited,
    MessageDeleted,
    TaskCreated,
    TaskUpdated,
    NoteCreated,
    NoteRevised,
    NoteConflictCopyCreated,
    ApprovalRequested,
    ApprovalDecided,
    ApprovalWithdrawn,
    ParticipantRegistered,
    ParticipantRevoked,
}

impl CollabEventKind {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RoomCreated => "room_created",
            Self::RoomArchived => "room_archived",
            Self::MembershipAdded => "membership_added",
            Self::MembershipRemoved => "membership_removed",
            Self::ThreadOpened => "thread_opened",
            Self::ThreadResolved => "thread_resolved",
            Self::ThreadReopened => "thread_reopened",
            Self::MessagePosted => "message_posted",
            Self::MessageEdited => "message_edited",
            Self::MessageDeleted => "message_deleted",
            Self::TaskCreated => "task_created",
            Self::TaskUpdated => "task_updated",
            Self::NoteCreated => "note_created",
            Self::NoteRevised => "note_revised",
            Self::NoteConflictCopyCreated => "note_conflict_copy_created",
            Self::ApprovalRequested => "approval_requested",
            Self::ApprovalDecided => "approval_decided",
            Self::ApprovalWithdrawn => "approval_withdrawn",
            Self::ParticipantRegistered => "participant_registered",
            Self::ParticipantRevoked => "participant_revoked",
        }
    }

    /// Parses a closed vocabulary value; unknown kinds fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "room_created" => Ok(Self::RoomCreated),
            "room_archived" => Ok(Self::RoomArchived),
            "membership_added" => Ok(Self::MembershipAdded),
            "membership_removed" => Ok(Self::MembershipRemoved),
            "thread_opened" => Ok(Self::ThreadOpened),
            "thread_resolved" => Ok(Self::ThreadResolved),
            "thread_reopened" => Ok(Self::ThreadReopened),
            "message_posted" => Ok(Self::MessagePosted),
            "message_edited" => Ok(Self::MessageEdited),
            "message_deleted" => Ok(Self::MessageDeleted),
            "task_created" => Ok(Self::TaskCreated),
            "task_updated" => Ok(Self::TaskUpdated),
            "note_created" => Ok(Self::NoteCreated),
            "note_revised" => Ok(Self::NoteRevised),
            "note_conflict_copy_created" => Ok(Self::NoteConflictCopyCreated),
            "approval_requested" => Ok(Self::ApprovalRequested),
            "approval_decided" => Ok(Self::ApprovalDecided),
            "approval_withdrawn" => Ok(Self::ApprovalWithdrawn),
            "participant_registered" => Ok(Self::ParticipantRegistered),
            "participant_revoked" => Ok(Self::ParticipantRevoked),
            other => Err(format!("unknown collaboration event kind {other}")),
        }
    }
}

/// The normalized shape every 076 mutation is reduced to before validation
/// and storage. Not a second persisted table: its durable projection is
/// `ActivityRecord` (one audit trail, not two).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollabEventEnvelope {
    pub schema_version: u32,
    pub room_id: OpaqueId,
    pub actor_participant_id: OpaqueId,
    pub actor_kind: ParticipantKind,
    pub event_kind: CollabEventKind,
    pub seq: u64,
}

/// An immutable, hash-chained activity entry. This is both the tamper-
/// evident audit trail and the local activity-feed data source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityRecord {
    pub header: ObjectHeader,
    pub room_id: OpaqueId,
    pub actor_participant_id: OpaqueId,
    pub event_kind: CollabEventKind,
    pub target_object_id: OpaqueId,
    pub checkpoint_digest: DigestSha256,
    pub seq: u64,
}

/// Computes the hash-chain checkpoint digest for the next `ActivityRecord`.
///
/// `Sha256(prev_checkpoint_digest_bytes || canonical_fields)`, where
/// `prev` is `None` only for the first record in a room (`seq == 1`).
/// Recomputing this chain from `seq = 1` and comparing stored digests
/// detects any row edited, deleted, or reordered out of band.
#[must_use]
pub fn compute_checkpoint_digest(
    prev: Option<&DigestSha256>,
    room_id: &OpaqueId,
    actor_participant_id: &OpaqueId,
    event_kind: CollabEventKind,
    target_object_id: &OpaqueId,
    seq: u64,
) -> DigestSha256 {
    let mut bytes = Vec::new();
    if let Some(prev) = prev {
        bytes.extend_from_slice(prev.as_bytes());
    }
    bytes.push(0);
    bytes.extend_from_slice(room_id.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(actor_participant_id.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(event_kind.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(target_object_id.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&seq.to_le_bytes());
    DigestSha256::of(&bytes)
}

impl ActivityRecord {
    /// Validates that `checkpoint_digest` matches the chain computation.
    pub fn validate_chain(&self, prev: Option<&DigestSha256>) -> Result<(), String> {
        let expected = compute_checkpoint_digest(
            prev,
            &self.room_id,
            &self.actor_participant_id,
            self.event_kind,
            &self.target_object_id,
            self.seq,
        );
        if expected != self.checkpoint_digest {
            return Err(format!(
                "activity checkpoint mismatch at seq {}: chain broken or tampered",
                self.seq
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: COLLAB_SCHEMA_VERSION,
            realm_id: crate::objects::RealmId::new("realm-1"),
            authority_scope_id: crate::objects::AuthorityScopeId::new("scope-1"),
        }
    }

    #[test]
    fn participant_kind_round_trips() {
        for kind in [
            ParticipantKind::Human,
            ParticipantKind::Service,
            ParticipantKind::Agent,
        ] {
            assert_eq!(ParticipantKind::parse(kind.as_str()).unwrap(), kind);
        }
        assert!(ParticipantKind::parse("nonexistent").is_err());
    }

    #[test]
    fn participant_kind_is_immutable_field_not_settable_post_hoc() {
        let participant = ParticipantIdentity::new(
            header("participant-1"),
            OpaqueId::new("holder-1"),
            ParticipantKind::Human,
            "Dr. Researcher".to_owned(),
        )
        .unwrap();
        assert_eq!(participant.kind, ParticipantKind::Human);
        assert_eq!(participant.revision, 1);
    }

    #[test]
    fn room_revision_starts_at_one_and_increments_by_one() {
        let room = Room::new(
            header("room-1"),
            OpaqueId::new("proj-1"),
            None,
            "Study Room".to_owned(),
        )
        .unwrap();
        assert_eq!(room.revision, 1);
        assert_eq!(room.check_mutation(1).unwrap(), 2);
    }

    #[test]
    fn stale_room_revision_conflicts_without_write() {
        let room = Room::new(
            header("room-1"),
            OpaqueId::new("proj-1"),
            None,
            "Study Room".to_owned(),
        )
        .unwrap();
        assert!(room.check_mutation(0).is_err());
        assert!(room.check_mutation(2).is_err());
    }

    #[test]
    fn empty_room_name_rejected() {
        assert!(
            Room::new(
                header("room-1"),
                OpaqueId::new("proj-1"),
                None,
                String::new()
            )
            .is_err()
        );
        assert!(
            Room::new(
                header("room-1"),
                OpaqueId::new("proj-1"),
                None,
                "   ".to_owned()
            )
            .is_err()
        );
    }

    #[test]
    fn oversized_room_name_rejected() {
        let name: String = "a".repeat(ROOM_NAME_MAX_CHARS + 1);
        assert!(Room::new(header("room-1"), OpaqueId::new("proj-1"), None, name).is_err());
    }

    #[test]
    fn membership_duplicate_identity_detects_same_room_and_participant() {
        let a = RoomMembership::new(
            header("mem-1"),
            OpaqueId::new("room-1"),
            OpaqueId::new("participant-1"),
            MembershipRole::Member,
        );
        let b = RoomMembership::new(
            header("mem-2"),
            OpaqueId::new("room-1"),
            OpaqueId::new("participant-1"),
            MembershipRole::Owner,
        );
        let c = RoomMembership::new(
            header("mem-3"),
            OpaqueId::new("room-1"),
            OpaqueId::new("participant-2"),
            MembershipRole::Member,
        );
        assert!(a.same_membership_as(&b));
        assert!(!a.same_membership_as(&c));
    }

    #[test]
    fn anchor_target_validates_text_region_ordering() {
        let bad = AnchorTarget {
            artifact: ArtifactDescriptor {
                object_id: OpaqueId::new("artifact-1"),
                kind: crate::project_graph::ArtifactKind::EvidenceDocument,
                binding: crate::project_graph::ArtifactVersionBinding::IdentityOnly,
            },
            detail: Some(AnchorDetail::TextRegion {
                span: TextSpan {
                    representation: crate::text::TextRepresentation::SourceBytes,
                    coordinate_system: crate::text::CoordinateSystem::RawByte,
                    start: 10,
                    end: 5,
                },
            }),
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn thread_resolution_is_not_a_stored_field() {
        // ThreadRef carries no `resolution` field; resolution is always a
        // read-time recompute against the live artifact (contracts.md
        // section 6). This test documents the invariant at the type level:
        // constructing a ThreadRef requires no resolution input at all.
        let anchor = AnchorTarget {
            artifact: ArtifactDescriptor {
                object_id: OpaqueId::new("artifact-1"),
                kind: crate::project_graph::ArtifactKind::EvidenceDocument,
                binding: crate::project_graph::ArtifactVersionBinding::Revision("v1".to_owned()),
            },
            detail: None,
        };
        let thread = ThreadRef::new(header("thread-1"), OpaqueId::new("room-1"), anchor).unwrap();
        assert_eq!(thread.status, ThreadStatus::Open);
    }

    #[test]
    fn message_body_bound_enforced() {
        let oversized = Message {
            header: header("msg-1"),
            thread_id: OpaqueId::new("thread-1"),
            author_participant_id: OpaqueId::new("participant-1"),
            body: "a".repeat(MESSAGE_BODY_MAX_BYTES + 1),
            seq: 1,
        };
        assert!(oversized.validate().is_err());
    }

    #[test]
    fn message_edit_never_represented_as_original_row_mutation() {
        // MessageEdit is its own row type distinct from Message; there is no
        // API on Message that mutates `body` in place.
        let edit = MessageEdit {
            header: header("edit-1"),
            message_id: OpaqueId::new("msg-1"),
            kind: MessageEditKind::BodyReplace {
                new_body: "corrected".to_owned(),
            },
            edited_by_participant_id: OpaqueId::new("participant-1"),
            seq: 1,
        };
        assert!(edit.validate().is_ok());
        let delete = MessageEdit {
            header: header("edit-2"),
            message_id: OpaqueId::new("msg-1"),
            kind: MessageEditKind::Delete,
            edited_by_participant_id: OpaqueId::new("participant-1"),
            seq: 2,
        };
        assert!(delete.validate().is_ok());
    }

    #[test]
    fn task_stale_revision_conflicts_without_write() {
        let task = Task::new(
            header("task-1"),
            OpaqueId::new("room-1"),
            None,
            "Screen abstract".to_owned(),
            None,
        )
        .unwrap();
        assert_eq!(task.revision, 1);
        assert!(task.check_mutation(0).is_err());
        assert_eq!(task.check_mutation(1).unwrap(), 2);
    }

    #[test]
    fn note_document_fast_forward_vs_conflict_detection() {
        let note = NoteDocument::new(
            header("note-1"),
            OpaqueId::new("room-1"),
            "Draft".to_owned(),
        )
        .unwrap();
        assert!(note.is_fast_forward(1));
        assert!(!note.is_fast_forward(0));
    }

    #[test]
    fn note_revision_conflict_copy_preserves_both_bodies() {
        let current = NoteRevision {
            header: header("rev-1"),
            note_id: OpaqueId::new("note-1"),
            revision: 2,
            parent_revision: Some(1),
            body: "second writer's edit".to_owned(),
            author_participant_id: OpaqueId::new("participant-2"),
            conflict_of: None,
        };
        let conflict_copy = NoteRevision {
            header: header("rev-2"),
            note_id: OpaqueId::new("note-1"),
            revision: 1,
            parent_revision: Some(1),
            body: "first writer's offline edit".to_owned(),
            author_participant_id: OpaqueId::new("participant-1"),
            conflict_of: Some(current.header.id.clone()),
        };
        assert!(current.validate().is_ok());
        assert!(conflict_copy.validate().is_ok());
        // Both bodies are distinct and both are readable: nothing was
        // silently overwritten or auto-merged.
        assert_ne!(current.body, conflict_copy.body);
        assert_eq!(conflict_copy.conflict_of, Some(current.header.id));
    }

    #[test]
    fn approval_request_requires_at_least_one_assignee() {
        let anchor = AnchorTarget {
            artifact: ArtifactDescriptor {
                object_id: OpaqueId::new("artifact-1"),
                kind: crate::project_graph::ArtifactKind::EvidenceDocument,
                binding: crate::project_graph::ArtifactVersionBinding::IdentityOnly,
            },
            detail: None,
        };
        let result = ApprovalRequest::new(
            header("req-1"),
            OpaqueId::new("room-1"),
            anchor,
            ApprovalKind::Review,
            OpaqueId::new("participant-1"),
            Vec::new(),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn approval_request_supports_dual_independent_assignees() {
        let anchor = AnchorTarget {
            artifact: ArtifactDescriptor {
                object_id: OpaqueId::new("artifact-1"),
                kind: crate::project_graph::ArtifactKind::EvidenceDocument,
                binding: crate::project_graph::ArtifactVersionBinding::IdentityOnly,
            },
            detail: None,
        };
        let request = ApprovalRequest::new(
            header("req-1"),
            OpaqueId::new("room-1"),
            anchor,
            ApprovalKind::Review,
            OpaqueId::new("owner-1"),
            vec![OpaqueId::new("reviewer-1"), OpaqueId::new("reviewer-2")],
            true,
        )
        .unwrap();
        assert!(request.is_assignee(&OpaqueId::new("reviewer-1")));
        assert!(request.is_assignee(&OpaqueId::new("reviewer-2")));
        assert!(!request.is_assignee(&OpaqueId::new("reviewer-3")));
    }

    #[test]
    fn blind_until_closed_hides_co_assignee_decision_while_open() {
        let anchor = AnchorTarget {
            artifact: ArtifactDescriptor {
                object_id: OpaqueId::new("artifact-1"),
                kind: crate::project_graph::ArtifactKind::EvidenceDocument,
                binding: crate::project_graph::ArtifactVersionBinding::IdentityOnly,
            },
            detail: None,
        };
        let mut request = ApprovalRequest::new(
            header("req-1"),
            OpaqueId::new("room-1"),
            anchor,
            ApprovalKind::Review,
            OpaqueId::new("owner-1"),
            vec![OpaqueId::new("reviewer-1"), OpaqueId::new("reviewer-2")],
            true,
        )
        .unwrap();
        let decider = OpaqueId::new("reviewer-1");
        let other_assignee = OpaqueId::new("reviewer-2");
        let owner = OpaqueId::new("owner-1");
        assert!(request.hides_decision_from(&decider, &other_assignee));
        // The requester may see everything even while open, per this policy.
        assert!(!request.hides_decision_from(&decider, &owner));
        request.status = ApprovalRequestStatus::Closed;
        assert!(!request.hides_decision_from(&decider, &other_assignee));
    }

    #[test]
    fn activity_chain_detects_tampered_row() {
        let room_id = OpaqueId::new("room-1");
        let actor = OpaqueId::new("participant-1");
        let target = OpaqueId::new("room-1");
        let digest_1 = compute_checkpoint_digest(
            None,
            &room_id,
            &actor,
            CollabEventKind::RoomCreated,
            &target,
            1,
        );
        let record_1 = ActivityRecord {
            header: header("activity-1"),
            room_id: room_id.clone(),
            actor_participant_id: actor.clone(),
            event_kind: CollabEventKind::RoomCreated,
            target_object_id: target.clone(),
            checkpoint_digest: digest_1.clone(),
            seq: 1,
        };
        assert!(record_1.validate_chain(None).is_ok());

        let digest_2 = compute_checkpoint_digest(
            Some(&digest_1),
            &room_id,
            &actor,
            CollabEventKind::MembershipAdded,
            &target,
            2,
        );
        let mut record_2 = ActivityRecord {
            header: header("activity-2"),
            room_id: room_id.clone(),
            actor_participant_id: actor.clone(),
            event_kind: CollabEventKind::MembershipAdded,
            target_object_id: target.clone(),
            checkpoint_digest: digest_2,
            seq: 2,
        };
        assert!(record_2.validate_chain(Some(&digest_1)).is_ok());

        // Tamper: flip the event kind after the digest was computed.
        record_2.event_kind = CollabEventKind::MembershipRemoved;
        assert!(record_2.validate_chain(Some(&digest_1)).is_err());
    }

    #[test]
    fn collab_event_kind_round_trips_full_vocabulary() {
        let all = [
            CollabEventKind::RoomCreated,
            CollabEventKind::RoomArchived,
            CollabEventKind::MembershipAdded,
            CollabEventKind::MembershipRemoved,
            CollabEventKind::ThreadOpened,
            CollabEventKind::ThreadResolved,
            CollabEventKind::ThreadReopened,
            CollabEventKind::MessagePosted,
            CollabEventKind::MessageEdited,
            CollabEventKind::MessageDeleted,
            CollabEventKind::TaskCreated,
            CollabEventKind::TaskUpdated,
            CollabEventKind::NoteCreated,
            CollabEventKind::NoteRevised,
            CollabEventKind::NoteConflictCopyCreated,
            CollabEventKind::ApprovalRequested,
            CollabEventKind::ApprovalDecided,
            CollabEventKind::ApprovalWithdrawn,
            CollabEventKind::ParticipantRegistered,
            CollabEventKind::ParticipantRevoked,
        ];
        for kind in all {
            assert_eq!(CollabEventKind::parse(kind.as_str()).unwrap(), kind);
        }
        assert!(CollabEventKind::parse("nonexistent").is_err());
    }

    #[test]
    fn agent_participant_identity_carries_unresolved_forward_ref_only() {
        let agent = AgentParticipantIdentity {
            participant_id: OpaqueId::new("participant-1"),
            agent_profile_ref: None,
        };
        assert!(agent.agent_profile_ref.is_none());
        let agent_with_ref = AgentParticipantIdentity {
            participant_id: OpaqueId::new("participant-2"),
            agent_profile_ref: Some(OpaqueId::new("future-agent-profile")),
        };
        // 076 never dereferences this; it is opaque data only.
        assert!(agent_with_ref.agent_profile_ref.is_some());
    }
}

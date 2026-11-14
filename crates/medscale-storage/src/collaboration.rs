//! Collaboration Substrate durable rows (Spec 076, storage schema v5).
//!
//! Dedicated tables inside the existing encrypted metadata DB: no second
//! database, no canonical payload copies. Every 076 contract field is a
//! scalar column except nested enums/structs (anchor, assignee list, message
//! edit kind), which travel as validated JSON. Reads re-validate and report
//! `Corrupt`/`UnsupportedSchema`.
//!
//! Concurrency: compare-and-swap on `revision` inside `unchecked_transaction`
//! for mutable rows (participants, rooms, memberships, threads, tasks,
//! approval requests). `collab_notes.revision` is CAS-updated only on a
//! fast-forward edit; a stale write instead inserts a `collab_note_revisions`
//! conflict-copy row and leaves the pointer untouched (see
//! `insert_note_conflict_copy`). Every other append-only family (messages,
//! message edits, note revisions, approval decisions, activity records) is
//! insert-only with a storage-assigned `seq`.
//!
//! `AgentParticipantIdentity` has no dedicated table: `agent_profile_ref` is
//! a nullable column on `collab_participants`, populated only when
//! `kind = 'agent'`. It is stored as an opaque string and never interpreted.
//!
//! Every mutation function that appends a `collab_activity_records` row does
//! so inside the same transaction as its primary write (`migration.md`
//! section 5): there is no code path that commits one without the other.

use medscale_contracts::collaboration::{
    ActivityRecord, AgentParticipantIdentity, ApprovalDecision, ApprovalDecisionOutcome,
    ApprovalKind, ApprovalRequest, ApprovalRequestStatus, CollabEventKind, MembershipRole,
    MembershipStatus, Message, MessageEdit, MessageEditKind, NoteDocument, NoteRevision,
    ParticipantIdentity, ParticipantKind, ParticipantStatus, Room, RoomMembership, RoomStatus,
    Task, TaskStatus, ThreadRef, ThreadStatus,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId,
};
use rusqlite::{OptionalExtension, Transaction, params};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v5 DDL, executed inside `begin/finish_migration(5)`.
pub(crate) const V5_DDL: &str = r"
CREATE TABLE IF NOT EXISTS collab_participants (
  participant_id TEXT PRIMARY KEY,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  holder_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  display_name TEXT NOT NULL,
  agent_profile_ref TEXT,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_collab_participants_holder
  ON collab_participants(authority_scope_id, holder_id);
CREATE TABLE IF NOT EXISTS collab_rooms (
  room_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  experiment_id TEXT,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_rooms_project
  ON collab_rooms(project_id, status);
CREATE TABLE IF NOT EXISTS collab_room_memberships (
  membership_id TEXT PRIMARY KEY,
  room_id TEXT NOT NULL,
  participant_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  role TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_memberships_room
  ON collab_room_memberships(room_id, status);
CREATE INDEX IF NOT EXISTS idx_collab_memberships_participant
  ON collab_room_memberships(participant_id, status);
CREATE TABLE IF NOT EXISTS collab_threads (
  thread_id TEXT PRIMARY KEY,
  room_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  anchor_json TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_threads_room
  ON collab_threads(room_id);
CREATE TABLE IF NOT EXISTS collab_messages (
  message_id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  author_participant_id TEXT NOT NULL,
  body TEXT NOT NULL,
  seq INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_messages_thread
  ON collab_messages(thread_id, seq);
CREATE TABLE IF NOT EXISTS collab_message_edits (
  edit_id TEXT PRIMARY KEY,
  message_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  kind_json TEXT NOT NULL,
  edited_by_participant_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_message_edits_message
  ON collab_message_edits(message_id, seq);
CREATE TABLE IF NOT EXISTS collab_tasks (
  task_id TEXT PRIMARY KEY,
  room_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  anchor_json TEXT,
  title TEXT NOT NULL,
  description TEXT,
  assignee_participant_id TEXT,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_tasks_room
  ON collab_tasks(room_id);
CREATE INDEX IF NOT EXISTS idx_collab_tasks_assignee
  ON collab_tasks(assignee_participant_id);
CREATE TABLE IF NOT EXISTS collab_notes (
  note_id TEXT PRIMARY KEY,
  room_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_notes_room
  ON collab_notes(room_id);
CREATE TABLE IF NOT EXISTS collab_note_revisions (
  note_revision_id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  parent_revision INTEGER,
  body TEXT NOT NULL,
  author_participant_id TEXT NOT NULL,
  conflict_of TEXT,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_note_revisions_note
  ON collab_note_revisions(note_id, revision);
CREATE TABLE IF NOT EXISTS collab_approval_requests (
  request_id TEXT PRIMARY KEY,
  room_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  anchor_json TEXT NOT NULL,
  kind TEXT NOT NULL,
  requested_by_participant_id TEXT NOT NULL,
  assignees_json TEXT NOT NULL,
  blind_until_closed INTEGER NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_approval_requests_room
  ON collab_approval_requests(room_id);
CREATE TABLE IF NOT EXISTS collab_approval_decisions (
  decision_id TEXT PRIMARY KEY,
  request_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  decided_by_participant_id TEXT NOT NULL,
  outcome TEXT NOT NULL,
  rationale TEXT,
  seq INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collab_approval_decisions_request
  ON collab_approval_decisions(request_id, seq);
CREATE TABLE IF NOT EXISTS collab_activity_records (
  activity_id TEXT PRIMARY KEY,
  room_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  actor_participant_id TEXT NOT NULL,
  event_kind TEXT NOT NULL,
  target_object_id TEXT NOT NULL,
  checkpoint_digest_hex TEXT NOT NULL,
  seq INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_collab_activity_room_seq
  ON collab_activity_records(room_id, seq);
";

// ---------------------------------------------------------------------------
// small helpers (deliberately duplicated from data_sources.rs rather than
// exported from it, so this module never depends on Spec 075's closed file)
// ---------------------------------------------------------------------------

const COLLAB_CURSOR_MAX_BYTES: usize = 256;

fn check_cursor(after: Option<&str>) -> Result<(), MetaError> {
    if let Some(value) = after
        && value.len() > COLLAB_CURSOR_MAX_BYTES
    {
        return Err(MetaError::UnsupportedSchema(
            "cursor exceeds bound".to_owned(),
        ));
    }
    Ok(())
}

fn is_conflict(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(f, _)
        if f.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

fn digest_hex(digest: &DigestSha256) -> String {
    digest.to_hex()
}

fn parse_digest_hex(value: &str, what: &str) -> Result<DigestSha256, MetaError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(MetaError::CorruptObjectBody(format!(
            "invalid {what} digest"
        )));
    }
    let mut out = [0_u8; 32];
    for (i, chunk) in value.as_bytes().chunks(2).enumerate() {
        let text = std::str::from_utf8(chunk)
            .map_err(|_| MetaError::CorruptObjectBody(format!("invalid {what} digest")))?;
        out[i] = u8::from_str_radix(text, 16)
            .map_err(|_| MetaError::CorruptObjectBody(format!("invalid {what} digest")))?;
    }
    Ok(DigestSha256::from_bytes(out))
}

fn revision_to_i64(revision: u64) -> i64 {
    i64::try_from(revision).unwrap_or(i64::MAX)
}

fn revision_from_i64(value: i64, what: &str) -> Result<u64, MetaError> {
    u64::try_from(value).map_err(|_| MetaError::CorruptObjectBody(format!("bad {what} revision")))
}

fn header_of(
    id: &str,
    realm: &str,
    scope: &str,
    schema_version: i64,
    default_schema_version: u32,
) -> ObjectHeader {
    ObjectHeader {
        id: OpaqueId::new(id),
        schema_version: u32::try_from(schema_version).unwrap_or(default_schema_version),
        realm_id: RealmId::new(realm),
        authority_scope_id: AuthorityScopeId::new(scope),
    }
}

// ---------------------------------------------------------------------------
// id allocation (reuses the durable sqlite-backed sequence pattern)
// ---------------------------------------------------------------------------

impl SqliteMetaStore {
    /// Allocates one `prefix-N` collaboration id from a durable sqlite-backed
    /// sequence, independent of the Spec 075 `alloc_data_fabric_id` sequence
    /// space (distinct `seq_key`s never collide across specs).
    pub fn alloc_collab_id(&self, seq_key: &str, prefix: &str) -> Result<OpaqueId, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = ?1",
                params![seq_key],
                |row| row.get(0),
            )
            .optional()?;
        let next: u64 = match current.as_deref() {
            None => 1,
            Some(raw) => {
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("id sequence {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, next.to_string()],
        )?;
        tx.commit()?;
        Ok(OpaqueId::new(format!("{prefix}-{next}")))
    }

    /// Allocates the next `seq` for one append-only family scoped by
    /// `scope_key` (e.g. a thread id for messages, a room id for activity
    /// records). Distinct from `alloc_collab_id`: this is a per-parent
    /// counter, not a global id sequence.
    fn next_collab_seq(&self, seq_key: &str) -> Result<u64, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = ?1",
                params![seq_key],
                |row| row.get(0),
            )
            .optional()?;
        let next: u64 = match current.as_deref() {
            None => 1,
            Some(raw) => {
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("seq counter {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, next.to_string()],
        )?;
        tx.commit()?;
        Ok(next)
    }
}

// ---------------------------------------------------------------------------
// participants
// ---------------------------------------------------------------------------

fn participant_kind_str(kind: ParticipantKind) -> &'static str {
    kind.as_str()
}

fn parse_participant_kind(value: &str) -> Result<ParticipantKind, MetaError> {
    ParticipantKind::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn participant_status_str(status: ParticipantStatus) -> &'static str {
    match status {
        ParticipantStatus::Active => "active",
        ParticipantStatus::Revoked => "revoked",
    }
}

fn parse_participant_status(value: &str) -> Result<ParticipantStatus, MetaError> {
    match value {
        "active" => Ok(ParticipantStatus::Active),
        "revoked" => Ok(ParticipantStatus::Revoked),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown participant status {other}"
        ))),
    }
}

fn map_participant_row(row: &rusqlite::Row<'_>) -> Result<ParticipantIdentity, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(1).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(2).map_err(MetaError::Sqlite)?;
    let holder_id: String = row.get(3).map_err(MetaError::Sqlite)?;
    let kind: String = row.get(4).map_err(MetaError::Sqlite)?;
    let display_name: String = row.get(5).map_err(MetaError::Sqlite)?;
    let status: String = row.get(7).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(8).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(9).map_err(MetaError::Sqlite)?;
    Ok(ParticipantIdentity {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "participant")?,
        holder_id: OpaqueId::new(holder_id),
        kind: parse_participant_kind(&kind)?,
        display_name,
        status: parse_participant_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new participant row. Duplicate `(scope, holder_id)` fails
    /// as `Conflict` (idempotent registration is the caller's job: look up
    /// via `get_participant_by_holder` first).
    pub fn insert_participant(
        &self,
        participant: &ParticipantIdentity,
        agent_profile_ref: Option<&OpaqueId>,
    ) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO collab_participants(participant_id, realm_id, authority_scope_id, holder_id, kind, display_name, agent_profile_ref, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                participant.header.id.as_str(),
                participant.header.realm_id.as_opaque().as_str(),
                participant.header.authority_scope_id.as_opaque().as_str(),
                participant.holder_id.as_str(),
                participant_kind_str(participant.kind),
                participant.display_name,
                agent_profile_ref.map(|r| r.as_str()),
                participant_status_str(participant.status),
                revision_to_i64(participant.revision),
                participant.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate participant {}",
                participant.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one participant row.
    pub fn get_participant(&self, id: &OpaqueId) -> Result<ParticipantIdentity, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT participant_id, realm_id, authority_scope_id, holder_id, kind, display_name, agent_profile_ref, status, revision, schema_version
             FROM collab_participants WHERE participant_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_participant_row(row)
    }

    /// Idempotent lookup by `(scope, holder_id)`; `None` when unregistered.
    pub fn get_participant_by_holder(
        &self,
        scope: &AuthorityScopeId,
        holder_id: &OpaqueId,
    ) -> Result<Option<ParticipantIdentity>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT participant_id, realm_id, authority_scope_id, holder_id, kind, display_name, agent_profile_ref, status, revision, schema_version
             FROM collab_participants WHERE authority_scope_id = ?1 AND holder_id = ?2",
        )?;
        let mut rows = stmt.query(params![scope.as_opaque().as_str(), holder_id.as_str()])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_participant_row(row)?)),
            None => Ok(None),
        }
    }

    /// Returns the `agent_profile_ref` extension for an agent participant,
    /// wrapped as `AgentParticipantIdentity`. `None` when the participant is
    /// not an agent or carries no forward reference.
    pub fn get_agent_participant(
        &self,
        id: &OpaqueId,
    ) -> Result<Option<AgentParticipantIdentity>, MetaError> {
        let participant = self.get_participant(id)?;
        if participant.kind != ParticipantKind::Agent {
            return Ok(None);
        }
        let agent_profile_ref: Option<String> = self
            .conn()
            .query_row(
                "SELECT agent_profile_ref FROM collab_participants WHERE participant_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(Some(AgentParticipantIdentity {
            participant_id: id.clone(),
            agent_profile_ref: agent_profile_ref.map(OpaqueId::new),
        }))
    }

    /// Lists every participant row (backup/restore snapshot only).
    pub fn list_all_participants(&self) -> Result<Vec<ParticipantIdentity>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT participant_id, realm_id, authority_scope_id, holder_id, kind, display_name, agent_profile_ref, status, revision, schema_version
             FROM collab_participants ORDER BY participant_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_participant_row(row)?);
        }
        Ok(out)
    }

    /// Reads every `(participant_id, agent_profile_ref)` pair (backup
    /// snapshot only; `list_all_participants` does not carry this column).
    pub fn list_all_participant_agent_refs(
        &self,
    ) -> Result<std::collections::HashMap<String, Option<String>>, MetaError> {
        let mut stmt = self
            .conn()
            .prepare("SELECT participant_id, agent_profile_ref FROM collab_participants")?;
        let mut rows = stmt.query([])?;
        let mut out = std::collections::HashMap::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let agent_profile_ref: Option<String> = row.get(1).map_err(MetaError::Sqlite)?;
            out.insert(id, agent_profile_ref);
        }
        Ok(out)
    }

    /// Restores one participant row exactly, including its
    /// `agent_profile_ref` (backup/restore path only).
    pub fn restore_participant_row(
        &self,
        participant: &ParticipantIdentity,
        agent_profile_ref: Option<&OpaqueId>,
    ) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_participants(participant_id, realm_id, authority_scope_id, holder_id, kind, display_name, agent_profile_ref, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                participant.header.id.as_str(),
                participant.header.realm_id.as_opaque().as_str(),
                participant.header.authority_scope_id.as_opaque().as_str(),
                participant.holder_id.as_str(),
                participant_kind_str(participant.kind),
                participant.display_name,
                agent_profile_ref.map(|r| r.as_str()),
                participant_status_str(participant.status),
                revision_to_i64(participant.revision.max(1)),
                participant.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Compare-and-swap participant status (revoke). No display-name/kind
    /// mutation path exists: `kind` is immutable per contract.
    pub fn set_participant_status(
        &self,
        id: &OpaqueId,
        expected: u64,
        status: ParticipantStatus,
    ) -> Result<ParticipantIdentity, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM collab_participants WHERE participant_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict("stale participant revision".to_owned()));
        }
        tx.execute(
            "UPDATE collab_participants SET status = ?1, revision = revision + 1 WHERE participant_id = ?2",
            params![participant_status_str(status), id.as_str()],
        )?;
        tx.commit()?;
        self.get_participant(id)
    }
}

// ---------------------------------------------------------------------------
// rooms
// ---------------------------------------------------------------------------

fn room_status_str(status: RoomStatus) -> &'static str {
    match status {
        RoomStatus::Active => "active",
        RoomStatus::Archived => "archived",
    }
}

fn parse_room_status(value: &str) -> Result<RoomStatus, MetaError> {
    match value {
        "active" => Ok(RoomStatus::Active),
        "archived" => Ok(RoomStatus::Archived),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown room status {other}"
        ))),
    }
}

fn map_room_row(row: &rusqlite::Row<'_>) -> Result<Room, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let project_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let experiment_id: Option<String> = row.get(2).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(3).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(4).map_err(MetaError::Sqlite)?;
    let name: String = row.get(5).map_err(MetaError::Sqlite)?;
    let status: String = row.get(6).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(7).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(8).map_err(MetaError::Sqlite)?;
    Ok(Room {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "room")?,
        project_id: OpaqueId::new(project_id),
        experiment_id: experiment_id.map(OpaqueId::new),
        name,
        status: parse_room_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new room row.
    pub fn insert_room(&self, room: &Room) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO collab_rooms(room_id, project_id, experiment_id, realm_id, authority_scope_id, name, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                room.header.id.as_str(),
                room.project_id.as_str(),
                room.experiment_id.as_ref().map(|e| e.as_str()),
                room.header.realm_id.as_opaque().as_str(),
                room.header.authority_scope_id.as_opaque().as_str(),
                room.name,
                room_status_str(room.status),
                revision_to_i64(room.revision),
                room.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate room {}",
                room.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Inserts a new room row and its `RoomCreated` activity entry in one
    /// transaction (`migration.md` section 5).
    pub fn insert_room_with_activity(
        &self,
        room: &Room,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<ActivityRecord, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO collab_rooms(room_id, project_id, experiment_id, realm_id, authority_scope_id, name, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                room.header.id.as_str(),
                room.project_id.as_str(),
                room.experiment_id.as_ref().map(|e| e.as_str()),
                room.header.realm_id.as_opaque().as_str(),
                room.header.authority_scope_id.as_opaque().as_str(),
                room.name,
                room_status_str(room.status),
                revision_to_i64(room.revision),
                room.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!("duplicate room {}", room.header.id.as_str()))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &room.header.id,
            actor_participant_id,
            CollabEventKind::RoomCreated,
            &room.header.id,
        )?;
        tx.commit()?;
        Ok(activity)
    }

    /// Reads one room row.
    pub fn get_room(&self, id: &OpaqueId) -> Result<Room, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT room_id, project_id, experiment_id, realm_id, authority_scope_id, name, status, revision, schema_version
             FROM collab_rooms WHERE room_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_room_row(row)
    }

    /// Lists rooms in one project with deterministic id order.
    pub fn list_rooms(
        &self,
        scope: &AuthorityScopeId,
        project_id: &OpaqueId,
        status: Option<RoomStatus>,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<Room>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = i64::from(limit.clamp(1, 100)) + 1;
        let after = after.unwrap_or_default();
        let status_str = status.map(room_status_str);
        let mut stmt = self.conn().prepare(
            "SELECT room_id, project_id, experiment_id, realm_id, authority_scope_id, name, status, revision, schema_version
             FROM collab_rooms
             WHERE authority_scope_id = ?1 AND project_id = ?2 AND (?3 IS NULL OR status = ?3) AND room_id > ?4
             ORDER BY room_id LIMIT ?5",
        )?;
        let mut rows = stmt.query(params![
            scope.as_opaque().as_str(),
            project_id.as_str(),
            status_str,
            after,
            limit
        ])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_room_row(row)?);
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|r| r.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Lists every room row (backup/restore snapshot only).
    pub fn list_all_rooms(&self) -> Result<Vec<Room>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT room_id, project_id, experiment_id, realm_id, authority_scope_id, name, status, revision, schema_version
             FROM collab_rooms ORDER BY room_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_room_row(row)?);
        }
        Ok(out)
    }

    /// Restores one room row exactly (backup/restore path only).
    pub fn restore_room_row(&self, room: &Room) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_rooms(room_id, project_id, experiment_id, realm_id, authority_scope_id, name, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                room.header.id.as_str(),
                room.project_id.as_str(),
                room.experiment_id.as_ref().map(|e| e.as_str()),
                room.header.realm_id.as_opaque().as_str(),
                room.header.authority_scope_id.as_opaque().as_str(),
                room.name,
                room_status_str(room.status),
                revision_to_i64(room.revision.max(1)),
                room.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Compare-and-swap room name. `CollabEventKind` has no dedicated
    /// "renamed" event in the frozen 076 vocabulary, so this records no
    /// activity entry, only the row mutation itself. Room archive
    /// (`archive_room_with_activity` below) does have a dedicated kind.
    pub fn update_room_name(
        &self,
        id: &OpaqueId,
        expected: u64,
        name: &str,
    ) -> Result<Room, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM collab_rooms WHERE room_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict("stale room revision".to_owned()));
        }
        tx.execute(
            "UPDATE collab_rooms SET name = ?1, revision = revision + 1 WHERE room_id = ?2",
            params![name, id.as_str()],
        )?;
        tx.commit()?;
        self.get_room(id)
    }

    /// Compare-and-swap room status to `Archived`, appending the
    /// `RoomArchived` activity entry in the same transaction
    /// (`migration.md` section 5). Room un-archive is out of 076 scope
    /// (`spec.md` section 5); only the one-way archive transition exists.
    pub fn archive_room_with_activity(
        &self,
        id: &OpaqueId,
        expected: u64,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<(Room, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM collab_rooms WHERE room_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict("stale room revision".to_owned()));
        }
        tx.execute(
            "UPDATE collab_rooms SET status = ?1, revision = revision + 1 WHERE room_id = ?2",
            params![room_status_str(RoomStatus::Archived), id.as_str()],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            id,
            actor_participant_id,
            CollabEventKind::RoomArchived,
            id,
        )?;
        tx.commit()?;
        let room = self.get_room(id)?;
        Ok((room, activity))
    }
}

// ---------------------------------------------------------------------------
// room memberships
// ---------------------------------------------------------------------------

fn membership_role_str(role: MembershipRole) -> &'static str {
    match role {
        MembershipRole::Owner => "owner",
        MembershipRole::Member => "member",
    }
}

fn parse_membership_role(value: &str) -> Result<MembershipRole, MetaError> {
    match value {
        "owner" => Ok(MembershipRole::Owner),
        "member" => Ok(MembershipRole::Member),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown membership role {other}"
        ))),
    }
}

fn membership_status_str(status: MembershipStatus) -> &'static str {
    match status {
        MembershipStatus::Active => "active",
        MembershipStatus::Removed => "removed",
    }
}

fn parse_membership_status(value: &str) -> Result<MembershipStatus, MetaError> {
    match value {
        "active" => Ok(MembershipStatus::Active),
        "removed" => Ok(MembershipStatus::Removed),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown membership status {other}"
        ))),
    }
}

fn map_membership_row(row: &rusqlite::Row<'_>) -> Result<RoomMembership, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let room_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let participant_id: String = row.get(2).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(3).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(4).map_err(MetaError::Sqlite)?;
    let role: String = row.get(5).map_err(MetaError::Sqlite)?;
    let status: String = row.get(6).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(7).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(8).map_err(MetaError::Sqlite)?;
    Ok(RoomMembership {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "membership")?,
        room_id: OpaqueId::new(room_id),
        participant_id: OpaqueId::new(participant_id),
        role: parse_membership_role(&role)?,
        status: parse_membership_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new membership row and its `MembershipAdded` activity entry
    /// in one transaction (`migration.md` section 5). Duplicate
    /// `membership_id` fails as `Conflict`; a duplicate *active*
    /// `(room, participant)` pair is the caller's job to reject via
    /// `find_active_membership` first (`RoomMembership::same_membership_as`).
    pub fn insert_membership_with_activity(
        &self,
        membership: &RoomMembership,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<ActivityRecord, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO collab_room_memberships(membership_id, room_id, participant_id, realm_id, authority_scope_id, role, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                membership.header.id.as_str(),
                membership.room_id.as_str(),
                membership.participant_id.as_str(),
                membership.header.realm_id.as_opaque().as_str(),
                membership.header.authority_scope_id.as_opaque().as_str(),
                membership_role_str(membership.role),
                membership_status_str(membership.status),
                revision_to_i64(membership.revision),
                membership.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!(
                    "duplicate membership {}",
                    membership.header.id.as_str()
                ))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &membership.room_id,
            actor_participant_id,
            CollabEventKind::MembershipAdded,
            &membership.participant_id,
        )?;
        tx.commit()?;
        Ok(activity)
    }

    /// Finds an active membership for `(room, participant)`, if any.
    pub fn find_active_membership(
        &self,
        room_id: &OpaqueId,
        participant_id: &OpaqueId,
    ) -> Result<Option<RoomMembership>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT membership_id, room_id, participant_id, realm_id, authority_scope_id, role, status, revision, schema_version
             FROM collab_room_memberships WHERE room_id = ?1 AND participant_id = ?2 AND status = 'active'",
        )?;
        let mut rows = stmt.query(params![room_id.as_str(), participant_id.as_str()])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_membership_row(row)?)),
            None => Ok(None),
        }
    }

    /// Lists active memberships for one room.
    pub fn list_room_memberships(
        &self,
        room_id: &OpaqueId,
    ) -> Result<Vec<RoomMembership>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT membership_id, room_id, participant_id, realm_id, authority_scope_id, role, status, revision, schema_version
             FROM collab_room_memberships WHERE room_id = ?1 AND status = 'active' ORDER BY membership_id",
        )?;
        let mut rows = stmt.query(params![room_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_membership_row(row)?);
        }
        Ok(out)
    }

    /// Lists every membership row (backup/restore snapshot only).
    pub fn list_all_memberships(&self) -> Result<Vec<RoomMembership>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT membership_id, room_id, participant_id, realm_id, authority_scope_id, role, status, revision, schema_version
             FROM collab_room_memberships ORDER BY membership_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_membership_row(row)?);
        }
        Ok(out)
    }

    /// Restores one membership row exactly (backup/restore path only).
    pub fn restore_membership_row(&self, membership: &RoomMembership) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_room_memberships(membership_id, room_id, participant_id, realm_id, authority_scope_id, role, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                membership.header.id.as_str(),
                membership.room_id.as_str(),
                membership.participant_id.as_str(),
                membership.header.realm_id.as_opaque().as_str(),
                membership.header.authority_scope_id.as_opaque().as_str(),
                membership_role_str(membership.role),
                membership_status_str(membership.status),
                revision_to_i64(membership.revision.max(1)),
                membership.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Compare-and-swap membership to `Removed`, appending the
    /// `MembershipRemoved` activity entry in the same transaction
    /// (`migration.md` section 5). Re-adding a removed participant creates
    /// a new membership row via `insert_membership_with_activity`; this
    /// function never flips `Removed` back to `Active` in place.
    pub fn remove_membership_with_activity(
        &self,
        id: &OpaqueId,
        expected: u64,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<(RoomMembership, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String, String)> = tx
            .query_row(
                "SELECT revision, room_id, participant_id FROM collab_room_memberships WHERE membership_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((current_rev, room_id, participant_id)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict("stale membership revision".to_owned()));
        }
        tx.execute(
            "UPDATE collab_room_memberships SET status = ?1, revision = revision + 1 WHERE membership_id = ?2",
            params![membership_status_str(MembershipStatus::Removed), id.as_str()],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &OpaqueId::new(room_id),
            actor_participant_id,
            CollabEventKind::MembershipRemoved,
            &OpaqueId::new(participant_id),
        )?;
        tx.commit()?;
        let mut stmt = self.conn().prepare(
            "SELECT membership_id, room_id, participant_id, realm_id, authority_scope_id, role, status, revision, schema_version
             FROM collab_room_memberships WHERE membership_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        let membership = map_membership_row(row)?;
        Ok((membership, activity))
    }
}

// ---------------------------------------------------------------------------
// threads
// ---------------------------------------------------------------------------

fn thread_status_str(status: ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Open => "open",
        ThreadStatus::Resolved => "resolved",
        ThreadStatus::Reopened => "reopened",
    }
}

fn parse_thread_status(value: &str) -> Result<ThreadStatus, MetaError> {
    match value {
        "open" => Ok(ThreadStatus::Open),
        "resolved" => Ok(ThreadStatus::Resolved),
        "reopened" => Ok(ThreadStatus::Reopened),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown thread status {other}"
        ))),
    }
}

fn map_thread_row(row: &rusqlite::Row<'_>) -> Result<ThreadRef, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let room_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let anchor_json: String = row.get(4).map_err(MetaError::Sqlite)?;
    let status: String = row.get(5).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(6).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
    let anchor = serde_json::from_str(&anchor_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad anchor json: {e}")))?;
    Ok(ThreadRef {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "thread")?,
        room_id: OpaqueId::new(room_id),
        anchor,
        status: parse_thread_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new thread row and its `ThreadOpened` activity entry in one
    /// transaction (`migration.md` section 5).
    pub fn insert_thread_with_activity(
        &self,
        thread: &ThreadRef,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<ActivityRecord, MetaError> {
        let anchor_json = serde_json::to_string(&thread.anchor)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO collab_threads(thread_id, room_id, realm_id, authority_scope_id, anchor_json, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                thread.header.id.as_str(),
                thread.room_id.as_str(),
                thread.header.realm_id.as_opaque().as_str(),
                thread.header.authority_scope_id.as_opaque().as_str(),
                anchor_json,
                thread_status_str(thread.status),
                revision_to_i64(thread.revision),
                thread.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!("duplicate thread {}", thread.header.id.as_str()))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &thread.room_id,
            actor_participant_id,
            CollabEventKind::ThreadOpened,
            &thread.header.id,
        )?;
        tx.commit()?;
        Ok(activity)
    }

    /// Reads one thread row.
    pub fn get_thread(&self, id: &OpaqueId) -> Result<ThreadRef, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT thread_id, room_id, realm_id, authority_scope_id, anchor_json, status, revision, schema_version
             FROM collab_threads WHERE thread_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_thread_row(row)
    }

    /// Lists threads in one room, newest-first by id.
    pub fn list_threads(
        &self,
        room_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<ThreadRef>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = i64::from(limit.clamp(1, 100)) + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT thread_id, room_id, realm_id, authority_scope_id, anchor_json, status, revision, schema_version
             FROM collab_threads WHERE room_id = ?1 AND thread_id > ?2 ORDER BY thread_id LIMIT ?3",
        )?;
        let mut rows = stmt.query(params![room_id.as_str(), after, limit])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_thread_row(row)?);
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|t| t.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Lists every thread row (backup/restore snapshot only).
    pub fn list_all_threads(&self) -> Result<Vec<ThreadRef>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT thread_id, room_id, realm_id, authority_scope_id, anchor_json, status, revision, schema_version
             FROM collab_threads ORDER BY thread_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_thread_row(row)?);
        }
        Ok(out)
    }

    /// Restores one thread row exactly (backup/restore path only).
    pub fn restore_thread_row(&self, thread: &ThreadRef) -> Result<(), MetaError> {
        let anchor_json = serde_json::to_string(&thread.anchor)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_threads(thread_id, room_id, realm_id, authority_scope_id, anchor_json, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                thread.header.id.as_str(),
                thread.room_id.as_str(),
                thread.header.realm_id.as_opaque().as_str(),
                thread.header.authority_scope_id.as_opaque().as_str(),
                anchor_json,
                thread_status_str(thread.status),
                revision_to_i64(thread.revision.max(1)),
                thread.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Compare-and-swap thread status (resolve/reopen), appending the
    /// matching `ThreadResolved`/`ThreadReopened` activity entry in the same
    /// transaction (`migration.md` section 5).
    pub fn set_thread_status_with_activity(
        &self,
        id: &OpaqueId,
        expected: u64,
        status: ThreadStatus,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<(ThreadRef, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision, room_id FROM collab_threads WHERE thread_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((current_rev, room_id)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict("stale thread revision".to_owned()));
        }
        tx.execute(
            "UPDATE collab_threads SET status = ?1, revision = revision + 1 WHERE thread_id = ?2",
            params![thread_status_str(status), id.as_str()],
        )?;
        let event_kind = match status {
            ThreadStatus::Resolved => CollabEventKind::ThreadResolved,
            ThreadStatus::Reopened => CollabEventKind::ThreadReopened,
            ThreadStatus::Open => CollabEventKind::ThreadOpened,
        };
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &OpaqueId::new(room_id),
            actor_participant_id,
            event_kind,
            id,
        )?;
        tx.commit()?;
        let thread = self.get_thread(id)?;
        Ok((thread, activity))
    }
}

// ---------------------------------------------------------------------------
// messages and message edits
// ---------------------------------------------------------------------------

fn map_message_row(row: &rusqlite::Row<'_>) -> Result<Message, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let thread_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let author: String = row.get(4).map_err(MetaError::Sqlite)?;
    let body: String = row.get(5).map_err(MetaError::Sqlite)?;
    let seq: i64 = row.get(6).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
    Ok(Message {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        thread_id: OpaqueId::new(thread_id),
        author_participant_id: OpaqueId::new(author),
        body,
        seq: u64::try_from(seq).unwrap_or(0),
    })
}

impl SqliteMetaStore {
    /// Appends a new message and its `MessagePosted` activity entry in one
    /// transaction (`migration.md` section 5). `seq` is assigned by storage
    /// (per-thread counter), never client-supplied. The caller supplies
    /// `room_id` (already resolved when the thread was loaded) so this
    /// function does not need an extra lookup to scope the activity chain.
    pub fn insert_message_with_activity(
        &self,
        header: ObjectHeader,
        room_id: &OpaqueId,
        thread_id: &OpaqueId,
        author_participant_id: &OpaqueId,
        body: &str,
        activity_header: ObjectHeader,
    ) -> Result<(Message, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let seq_key = format!("collab-msg-seq-{}", thread_id.as_str());
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = ?1",
                params![seq_key],
                |row| row.get(0),
            )
            .optional()?;
        let seq: u64 = match current.as_deref() {
            None => 1,
            Some(raw) => {
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("seq counter {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, seq.to_string()],
        )?;
        tx.execute(
            "INSERT INTO collab_messages(message_id, thread_id, realm_id, authority_scope_id, author_participant_id, body, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                header.id.as_str(),
                thread_id.as_str(),
                header.realm_id.as_opaque().as_str(),
                header.authority_scope_id.as_opaque().as_str(),
                author_participant_id.as_str(),
                body,
                seq as i64,
                header.schema_version as i64,
            ],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            room_id,
            author_participant_id,
            CollabEventKind::MessagePosted,
            &header.id,
        )?;
        tx.commit()?;
        let message = Message {
            header,
            thread_id: thread_id.clone(),
            author_participant_id: author_participant_id.clone(),
            body: body.to_owned(),
            seq,
        };
        Ok((message, activity))
    }

    /// Lists messages in one thread, in `seq` order.
    pub fn list_messages(
        &self,
        thread_id: &OpaqueId,
        limit: u32,
        after_seq: u64,
    ) -> Result<Vec<Message>, MetaError> {
        let limit = i64::from(limit.clamp(1, 100));
        let mut stmt = self.conn().prepare(
            "SELECT message_id, thread_id, realm_id, authority_scope_id, author_participant_id, body, seq, schema_version
             FROM collab_messages WHERE thread_id = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3",
        )?;
        let mut rows = stmt.query(params![thread_id.as_str(), after_seq as i64, limit])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_message_row(row)?);
        }
        Ok(out)
    }

    /// Reads one message row.
    pub fn get_message(&self, id: &OpaqueId) -> Result<Message, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT message_id, thread_id, realm_id, authority_scope_id, author_participant_id, body, seq, schema_version
             FROM collab_messages WHERE message_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_message_row(row)
    }

    /// Appends a message edit or delete and its `MessageEdited`/
    /// `MessageDeleted` activity entry in one transaction (`migration.md`
    /// section 5). Never rewrites `collab_messages`. The caller supplies
    /// `room_id` (already resolved with the message/thread).
    pub fn insert_message_edit_with_activity(
        &self,
        header: ObjectHeader,
        room_id: &OpaqueId,
        message_id: &OpaqueId,
        kind: &MessageEditKind,
        edited_by_participant_id: &OpaqueId,
        activity_header: ObjectHeader,
    ) -> Result<(MessageEdit, ActivityRecord), MetaError> {
        let kind_json =
            serde_json::to_string(kind).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let tx = self.conn().unchecked_transaction()?;
        let seq_key = format!("collab-msgedit-seq-{}", message_id.as_str());
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = ?1",
                params![seq_key],
                |row| row.get(0),
            )
            .optional()?;
        let seq: u64 = match current.as_deref() {
            None => 1,
            Some(raw) => {
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("seq counter {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, seq.to_string()],
        )?;
        tx.execute(
            "INSERT INTO collab_message_edits(edit_id, message_id, realm_id, authority_scope_id, kind_json, edited_by_participant_id, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                header.id.as_str(),
                message_id.as_str(),
                header.realm_id.as_opaque().as_str(),
                header.authority_scope_id.as_opaque().as_str(),
                kind_json,
                edited_by_participant_id.as_str(),
                seq as i64,
                header.schema_version as i64,
            ],
        )?;
        let event_kind = match kind {
            MessageEditKind::BodyReplace { .. } => CollabEventKind::MessageEdited,
            MessageEditKind::Delete => CollabEventKind::MessageDeleted,
        };
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            room_id,
            edited_by_participant_id,
            event_kind,
            message_id,
        )?;
        tx.commit()?;
        let edit = MessageEdit {
            header,
            message_id: message_id.clone(),
            kind: kind.clone(),
            edited_by_participant_id: edited_by_participant_id.clone(),
            seq,
        };
        Ok((edit, activity))
    }

    /// Lists every message row (backup/restore snapshot only).
    pub fn list_all_messages(&self) -> Result<Vec<Message>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT message_id, thread_id, realm_id, authority_scope_id, author_participant_id, body, seq, schema_version
             FROM collab_messages ORDER BY thread_id, seq",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_message_row(row)?);
        }
        Ok(out)
    }

    /// Restores one message row exactly, including its storage-assigned
    /// `seq` (backup/restore path only).
    pub fn restore_message_row(&self, message: &Message) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_messages(message_id, thread_id, realm_id, authority_scope_id, author_participant_id, body, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                message.header.id.as_str(),
                message.thread_id.as_str(),
                message.header.realm_id.as_opaque().as_str(),
                message.header.authority_scope_id.as_opaque().as_str(),
                message.author_participant_id.as_str(),
                message.body,
                message.seq as i64,
                message.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Lists the edit log of one message, in `seq` order (oldest first).
    pub fn list_message_edits(&self, message_id: &OpaqueId) -> Result<Vec<MessageEdit>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT edit_id, message_id, realm_id, authority_scope_id, kind_json, edited_by_participant_id, seq, schema_version
             FROM collab_message_edits WHERE message_id = ?1 ORDER BY seq",
        )?;
        let mut rows = stmt.query(params![message_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let mid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let kind_json: String = row.get(4).map_err(MetaError::Sqlite)?;
            let editor: String = row.get(5).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(6).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
            let kind = serde_json::from_str(&kind_json)
                .map_err(|e| MetaError::CorruptObjectBody(format!("bad edit kind json: {e}")))?;
            out.push(MessageEdit {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                message_id: OpaqueId::new(mid),
                kind,
                edited_by_participant_id: OpaqueId::new(editor),
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Lists every message-edit row (backup/restore snapshot only).
    pub fn list_all_message_edits(&self) -> Result<Vec<MessageEdit>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT edit_id, message_id, realm_id, authority_scope_id, kind_json, edited_by_participant_id, seq, schema_version
             FROM collab_message_edits ORDER BY message_id, seq",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let mid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let kind_json: String = row.get(4).map_err(MetaError::Sqlite)?;
            let editor: String = row.get(5).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(6).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
            let kind = serde_json::from_str(&kind_json)
                .map_err(|e| MetaError::CorruptObjectBody(format!("bad edit kind json: {e}")))?;
            out.push(MessageEdit {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                message_id: OpaqueId::new(mid),
                kind,
                edited_by_participant_id: OpaqueId::new(editor),
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Restores one message-edit row exactly, including its storage-assigned
    /// `seq` (backup/restore path only).
    pub fn restore_message_edit_row(&self, edit: &MessageEdit) -> Result<(), MetaError> {
        let kind_json = serde_json::to_string(&edit.kind)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_message_edits(edit_id, message_id, realm_id, authority_scope_id, kind_json, edited_by_participant_id, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                edit.header.id.as_str(),
                edit.message_id.as_str(),
                edit.header.realm_id.as_opaque().as_str(),
                edit.header.authority_scope_id.as_opaque().as_str(),
                kind_json,
                edit.edited_by_participant_id.as_str(),
                edit.seq as i64,
                edit.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// tasks
// ---------------------------------------------------------------------------

fn task_status_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "open",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Done => "done",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn parse_task_status(value: &str) -> Result<TaskStatus, MetaError> {
    match value {
        "open" => Ok(TaskStatus::Open),
        "in_progress" => Ok(TaskStatus::InProgress),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        "cancelled" => Ok(TaskStatus::Cancelled),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown task status {other}"
        ))),
    }
}

fn map_task_row(row: &rusqlite::Row<'_>) -> Result<Task, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let room_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let anchor_json: Option<String> = row.get(4).map_err(MetaError::Sqlite)?;
    let title: String = row.get(5).map_err(MetaError::Sqlite)?;
    let description: Option<String> = row.get(6).map_err(MetaError::Sqlite)?;
    let assignee: Option<String> = row.get(7).map_err(MetaError::Sqlite)?;
    let status: String = row.get(8).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(9).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(10).map_err(MetaError::Sqlite)?;
    let anchor = match anchor_json {
        Some(json) => Some(
            serde_json::from_str(&json)
                .map_err(|e| MetaError::CorruptObjectBody(format!("bad anchor json: {e}")))?,
        ),
        None => None,
    };
    Ok(Task {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "task")?,
        room_id: OpaqueId::new(room_id),
        anchor,
        title,
        description,
        assignee_participant_id: assignee.map(OpaqueId::new),
        status: parse_task_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new task row and its `TaskCreated` activity entry in one
    /// transaction (`migration.md` section 5).
    pub fn insert_task_with_activity(
        &self,
        task: &Task,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<ActivityRecord, MetaError> {
        let anchor_json = match &task.anchor {
            Some(anchor) => Some(
                serde_json::to_string(anchor)
                    .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
            ),
            None => None,
        };
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO collab_tasks(task_id, room_id, realm_id, authority_scope_id, anchor_json, title, description, assignee_participant_id, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.header.id.as_str(),
                task.room_id.as_str(),
                task.header.realm_id.as_opaque().as_str(),
                task.header.authority_scope_id.as_opaque().as_str(),
                anchor_json,
                task.title,
                task.description,
                task.assignee_participant_id.as_ref().map(|a| a.as_str()),
                task_status_str(task.status),
                revision_to_i64(task.revision),
                task.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!("duplicate task {}", task.header.id.as_str()))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &task.room_id,
            actor_participant_id,
            CollabEventKind::TaskCreated,
            &task.header.id,
        )?;
        tx.commit()?;
        Ok(activity)
    }

    /// Reads one task row.
    pub fn get_task(&self, id: &OpaqueId) -> Result<Task, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT task_id, room_id, realm_id, authority_scope_id, anchor_json, title, description, assignee_participant_id, status, revision, schema_version
             FROM collab_tasks WHERE task_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_task_row(row)
    }

    /// Lists tasks in one room.
    pub fn list_tasks(
        &self,
        room_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<Task>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = i64::from(limit.clamp(1, 100)) + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT task_id, room_id, realm_id, authority_scope_id, anchor_json, title, description, assignee_participant_id, status, revision, schema_version
             FROM collab_tasks WHERE room_id = ?1 AND task_id > ?2 ORDER BY task_id LIMIT ?3",
        )?;
        let mut rows = stmt.query(params![room_id.as_str(), after, limit])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_task_row(row)?);
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|t| t.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Lists every task row (backup/restore snapshot only).
    pub fn list_all_tasks(&self) -> Result<Vec<Task>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT task_id, room_id, realm_id, authority_scope_id, anchor_json, title, description, assignee_participant_id, status, revision, schema_version
             FROM collab_tasks ORDER BY task_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_task_row(row)?);
        }
        Ok(out)
    }

    /// Restores one task row exactly (backup/restore path only).
    pub fn restore_task_row(&self, task: &Task) -> Result<(), MetaError> {
        let anchor_json = match &task.anchor {
            Some(anchor) => Some(
                serde_json::to_string(anchor)
                    .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
            ),
            None => None,
        };
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_tasks(task_id, room_id, realm_id, authority_scope_id, anchor_json, title, description, assignee_participant_id, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.header.id.as_str(),
                task.room_id.as_str(),
                task.header.realm_id.as_opaque().as_str(),
                task.header.authority_scope_id.as_opaque().as_str(),
                anchor_json,
                task.title,
                task.description,
                task.assignee_participant_id.as_ref().map(|a| a.as_str()),
                task_status_str(task.status),
                revision_to_i64(task.revision.max(1)),
                task.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Compare-and-swap task fields (status/assignee), appending the
    /// `TaskUpdated` activity entry in the same transaction (`migration.md`
    /// section 5).
    pub fn update_task_with_activity(
        &self,
        id: &OpaqueId,
        expected: u64,
        status: TaskStatus,
        assignee_participant_id: Option<&OpaqueId>,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<(Task, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision, room_id FROM collab_tasks WHERE task_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((current_rev, room_id)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict("stale task revision".to_owned()));
        }
        tx.execute(
            "UPDATE collab_tasks SET status = ?1, assignee_participant_id = ?2, revision = revision + 1 WHERE task_id = ?3",
            params![
                task_status_str(status),
                assignee_participant_id.map(|a| a.as_str()),
                id.as_str()
            ],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &OpaqueId::new(room_id),
            actor_participant_id,
            CollabEventKind::TaskUpdated,
            id,
        )?;
        tx.commit()?;
        let task = self.get_task(id)?;
        Ok((task, activity))
    }
}

// ---------------------------------------------------------------------------
// notes (fast-forward edit vs. explicit conflict copy)
// ---------------------------------------------------------------------------

fn note_status_str(status: medscale_contracts::collaboration::NoteStatus) -> &'static str {
    use medscale_contracts::collaboration::NoteStatus;
    match status {
        NoteStatus::Active => "active",
        NoteStatus::Archived => "archived",
    }
}

fn parse_note_status(
    value: &str,
) -> Result<medscale_contracts::collaboration::NoteStatus, MetaError> {
    use medscale_contracts::collaboration::NoteStatus;
    match value {
        "active" => Ok(NoteStatus::Active),
        "archived" => Ok(NoteStatus::Archived),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown note status {other}"
        ))),
    }
}

fn map_note_row(row: &rusqlite::Row<'_>) -> Result<NoteDocument, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let room_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let title: String = row.get(4).map_err(MetaError::Sqlite)?;
    let status: String = row.get(5).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(6).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(7).map_err(MetaError::Sqlite)?;
    Ok(NoteDocument {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "note")?,
        room_id: OpaqueId::new(room_id),
        title,
        status: parse_note_status(&status)?,
    })
}

fn map_note_revision_row(row: &rusqlite::Row<'_>) -> Result<NoteRevision, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let note_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(4).map_err(MetaError::Sqlite)?;
    let parent_revision: Option<i64> = row.get(5).map_err(MetaError::Sqlite)?;
    let body: String = row.get(6).map_err(MetaError::Sqlite)?;
    let author: String = row.get(7).map_err(MetaError::Sqlite)?;
    let conflict_of: Option<String> = row.get(8).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(9).map_err(MetaError::Sqlite)?;
    let parent_revision = match parent_revision {
        Some(v) => Some(revision_from_i64(v, "note revision parent")?),
        None => None,
    };
    Ok(NoteRevision {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        note_id: OpaqueId::new(note_id),
        revision: revision_from_i64(revision, "note revision")?,
        parent_revision,
        body,
        author_participant_id: OpaqueId::new(author),
        conflict_of: conflict_of.map(OpaqueId::new),
    })
}

impl SqliteMetaStore {
    /// Inserts a new note document row plus its initial (revision-1) body
    /// and the `NoteCreated` activity entry, all in one transaction
    /// (`migration.md` section 5).
    pub fn insert_note_with_activity(
        &self,
        note: &NoteDocument,
        initial_revision: &NoteRevision,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<ActivityRecord, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO collab_notes(note_id, room_id, realm_id, authority_scope_id, title, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                note.header.id.as_str(),
                note.room_id.as_str(),
                note.header.realm_id.as_opaque().as_str(),
                note.header.authority_scope_id.as_opaque().as_str(),
                note.title,
                note_status_str(note.status),
                revision_to_i64(note.revision),
                note.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!("duplicate note {}", note.header.id.as_str()))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        tx.execute(
            "INSERT INTO collab_note_revisions(note_revision_id, note_id, realm_id, authority_scope_id, revision, parent_revision, body, author_participant_id, conflict_of, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                initial_revision.header.id.as_str(),
                initial_revision.note_id.as_str(),
                initial_revision.header.realm_id.as_opaque().as_str(),
                initial_revision
                    .header
                    .authority_scope_id
                    .as_opaque()
                    .as_str(),
                revision_to_i64(initial_revision.revision),
                initial_revision.parent_revision.map(revision_to_i64),
                initial_revision.body,
                initial_revision.author_participant_id.as_str(),
                initial_revision.conflict_of.as_ref().map(|c| c.as_str()),
                initial_revision.header.schema_version as i64,
            ],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &note.room_id,
            actor_participant_id,
            CollabEventKind::NoteCreated,
            &note.header.id,
        )?;
        tx.commit()?;
        Ok(activity)
    }

    /// Reads one note document row.
    pub fn get_note(&self, id: &OpaqueId) -> Result<NoteDocument, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT note_id, room_id, realm_id, authority_scope_id, title, status, revision, schema_version
             FROM collab_notes WHERE note_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_note_row(row)
    }

    /// Lists the full revision history of one note (fast-forward edits and
    /// conflict copies alike), oldest first.
    pub fn list_note_revisions(&self, note_id: &OpaqueId) -> Result<Vec<NoteRevision>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT note_revision_id, note_id, realm_id, authority_scope_id, revision, parent_revision, body, author_participant_id, conflict_of, schema_version
             FROM collab_note_revisions WHERE note_id = ?1 ORDER BY note_revision_id",
        )?;
        let mut rows = stmt.query(params![note_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_note_revision_row(row)?);
        }
        Ok(out)
    }

    /// Lists every note document row (backup/restore snapshot only).
    pub fn list_all_notes(&self) -> Result<Vec<NoteDocument>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT note_id, room_id, realm_id, authority_scope_id, title, status, revision, schema_version
             FROM collab_notes ORDER BY note_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_note_row(row)?);
        }
        Ok(out)
    }

    /// Lists every note-revision row across every note, ordered by note then
    /// revision (backup/restore snapshot only).
    pub fn list_all_note_revisions(&self) -> Result<Vec<NoteRevision>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT note_revision_id, note_id, realm_id, authority_scope_id, revision, parent_revision, body, author_participant_id, conflict_of, schema_version
             FROM collab_note_revisions ORDER BY note_id, note_revision_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_note_revision_row(row)?);
        }
        Ok(out)
    }

    /// Restores one note document pointer row exactly (backup/restore path
    /// only; does not touch `collab_note_revisions` — restore that
    /// separately via `restore_note_revision_row`).
    pub fn restore_note_row(&self, note: &NoteDocument) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_notes(note_id, room_id, realm_id, authority_scope_id, title, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                note.header.id.as_str(),
                note.room_id.as_str(),
                note.header.realm_id.as_opaque().as_str(),
                note.header.authority_scope_id.as_opaque().as_str(),
                note.title,
                note_status_str(note.status),
                revision_to_i64(note.revision.max(1)),
                note.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Restores one note-revision row exactly, fast-forward or conflict
    /// copy alike (backup/restore path only).
    pub fn restore_note_revision_row(&self, revision: &NoteRevision) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_note_revisions(note_revision_id, note_id, realm_id, authority_scope_id, revision, parent_revision, body, author_participant_id, conflict_of, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                revision.header.id.as_str(),
                revision.note_id.as_str(),
                revision.header.realm_id.as_opaque().as_str(),
                revision.header.authority_scope_id.as_opaque().as_str(),
                revision_to_i64(revision.revision.max(1)),
                revision.parent_revision.map(revision_to_i64),
                revision.body,
                revision.author_participant_id.as_str(),
                revision.conflict_of.as_ref().map(|c| c.as_str()),
                revision.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Fast-forward edit: `expected_revision` matches the current pointer.
    /// Advances `collab_notes.revision`, appends the new body as a
    /// `NoteRevision` with `conflict_of = NULL`, and appends the
    /// `NoteRevised` activity entry, all in one transaction (`migration.md`
    /// section 5).
    pub fn apply_note_fast_forward_with_activity(
        &self,
        note_id: &OpaqueId,
        expected: u64,
        new_revision: &NoteRevision,
        activity_header: ObjectHeader,
    ) -> Result<(NoteDocument, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision, room_id FROM collab_notes WHERE note_id = ?1",
                params![note_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((current_rev, room_id)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict(
                "note is not at the expected revision; caller must create a conflict copy instead"
                    .to_owned(),
            ));
        }
        tx.execute(
            "INSERT INTO collab_note_revisions(note_revision_id, note_id, realm_id, authority_scope_id, revision, parent_revision, body, author_participant_id, conflict_of, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                new_revision.header.id.as_str(),
                new_revision.note_id.as_str(),
                new_revision.header.realm_id.as_opaque().as_str(),
                new_revision.header.authority_scope_id.as_opaque().as_str(),
                revision_to_i64(new_revision.revision),
                new_revision.parent_revision.map(revision_to_i64),
                new_revision.body,
                new_revision.author_participant_id.as_str(),
                Option::<&str>::None,
                new_revision.header.schema_version as i64,
            ],
        )?;
        tx.execute(
            "UPDATE collab_notes SET revision = ?1 WHERE note_id = ?2",
            params![revision_to_i64(new_revision.revision), note_id.as_str()],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &OpaqueId::new(room_id),
            &new_revision.author_participant_id,
            CollabEventKind::NoteRevised,
            note_id,
        )?;
        tx.commit()?;
        let note = self.get_note(note_id)?;
        Ok((note, activity))
    }

    /// Conflict copy: `expected_revision` did not match the current
    /// pointer. Appends the caller's body as a `NoteRevision` with
    /// `conflict_of` set to the currently-current revision's id, plus the
    /// `NoteConflictCopyCreated` activity entry, in one transaction.
    /// `collab_notes.revision` is left untouched — the caller's content is
    /// preserved, never discarded, and the pointer never moves backward.
    pub fn insert_note_conflict_copy_with_activity(
        &self,
        room_id: &OpaqueId,
        note_id: &OpaqueId,
        conflict_copy: &NoteRevision,
        activity_header: ObjectHeader,
    ) -> Result<ActivityRecord, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO collab_note_revisions(note_revision_id, note_id, realm_id, authority_scope_id, revision, parent_revision, body, author_participant_id, conflict_of, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                conflict_copy.header.id.as_str(),
                note_id.as_str(),
                conflict_copy.header.realm_id.as_opaque().as_str(),
                conflict_copy.header.authority_scope_id.as_opaque().as_str(),
                revision_to_i64(conflict_copy.revision),
                conflict_copy.parent_revision.map(revision_to_i64),
                conflict_copy.body,
                conflict_copy.author_participant_id.as_str(),
                conflict_copy.conflict_of.as_ref().map(|c| c.as_str()),
                conflict_copy.header.schema_version as i64,
            ],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            room_id,
            &conflict_copy.author_participant_id,
            CollabEventKind::NoteConflictCopyCreated,
            note_id,
        )?;
        tx.commit()?;
        Ok(activity)
    }
}

// ---------------------------------------------------------------------------
// approval requests and decisions
// ---------------------------------------------------------------------------

fn approval_kind_str(kind: ApprovalKind) -> &'static str {
    match kind {
        ApprovalKind::Review => "review",
        ApprovalKind::Approval => "approval",
        ApprovalKind::Adjudication => "adjudication",
    }
}

fn parse_approval_kind(value: &str) -> Result<ApprovalKind, MetaError> {
    match value {
        "review" => Ok(ApprovalKind::Review),
        "approval" => Ok(ApprovalKind::Approval),
        "adjudication" => Ok(ApprovalKind::Adjudication),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown approval kind {other}"
        ))),
    }
}

fn approval_status_str(status: ApprovalRequestStatus) -> &'static str {
    match status {
        ApprovalRequestStatus::Open => "open",
        ApprovalRequestStatus::Withdrawn => "withdrawn",
        ApprovalRequestStatus::Closed => "closed",
    }
}

fn parse_approval_status(value: &str) -> Result<ApprovalRequestStatus, MetaError> {
    match value {
        "open" => Ok(ApprovalRequestStatus::Open),
        "withdrawn" => Ok(ApprovalRequestStatus::Withdrawn),
        "closed" => Ok(ApprovalRequestStatus::Closed),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown approval request status {other}"
        ))),
    }
}

fn approval_outcome_str(outcome: ApprovalDecisionOutcome) -> &'static str {
    match outcome {
        ApprovalDecisionOutcome::Approved => "approved",
        ApprovalDecisionOutcome::Rejected => "rejected",
        ApprovalDecisionOutcome::ChangesRequested => "changes_requested",
        ApprovalDecisionOutcome::Abstained => "abstained",
    }
}

fn parse_approval_outcome(value: &str) -> Result<ApprovalDecisionOutcome, MetaError> {
    match value {
        "approved" => Ok(ApprovalDecisionOutcome::Approved),
        "rejected" => Ok(ApprovalDecisionOutcome::Rejected),
        "changes_requested" => Ok(ApprovalDecisionOutcome::ChangesRequested),
        "abstained" => Ok(ApprovalDecisionOutcome::Abstained),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown approval decision outcome {other}"
        ))),
    }
}

fn map_approval_request_row(row: &rusqlite::Row<'_>) -> Result<ApprovalRequest, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let room_id: String = row.get(1).map_err(MetaError::Sqlite)?;
    let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
    let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
    let anchor_json: String = row.get(4).map_err(MetaError::Sqlite)?;
    let kind: String = row.get(5).map_err(MetaError::Sqlite)?;
    let requested_by: String = row.get(6).map_err(MetaError::Sqlite)?;
    let assignees_json: String = row.get(7).map_err(MetaError::Sqlite)?;
    let blind: i64 = row.get(8).map_err(MetaError::Sqlite)?;
    let status: String = row.get(9).map_err(MetaError::Sqlite)?;
    let revision: i64 = row.get(10).map_err(MetaError::Sqlite)?;
    let schema_version: i64 = row.get(11).map_err(MetaError::Sqlite)?;
    let anchor = serde_json::from_str(&anchor_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad anchor json: {e}")))?;
    let assignee_strings: Vec<String> = serde_json::from_str(&assignees_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad assignees json: {e}")))?;
    Ok(ApprovalRequest {
        header: header_of(&id, &realm, &scope, schema_version, 1),
        revision: revision_from_i64(revision, "approval request")?,
        room_id: OpaqueId::new(room_id),
        anchor,
        kind: parse_approval_kind(&kind)?,
        requested_by_participant_id: OpaqueId::new(requested_by),
        assignee_participant_ids: assignee_strings.into_iter().map(OpaqueId::new).collect(),
        blind_until_closed: blind != 0,
        status: parse_approval_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new approval request row and its `ApprovalRequested`
    /// activity entry in one transaction (`migration.md` section 5).
    pub fn insert_approval_request_with_activity(
        &self,
        request: &ApprovalRequest,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<ActivityRecord, MetaError> {
        let anchor_json = serde_json::to_string(&request.anchor)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let assignee_strings: Vec<&str> = request
            .assignee_participant_ids
            .iter()
            .map(OpaqueId::as_str)
            .collect();
        let assignees_json = serde_json::to_string(&assignee_strings)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO collab_approval_requests(request_id, room_id, realm_id, authority_scope_id, anchor_json, kind, requested_by_participant_id, assignees_json, blind_until_closed, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                request.header.id.as_str(),
                request.room_id.as_str(),
                request.header.realm_id.as_opaque().as_str(),
                request.header.authority_scope_id.as_opaque().as_str(),
                anchor_json,
                approval_kind_str(request.kind),
                request.requested_by_participant_id.as_str(),
                assignees_json,
                i64::from(request.blind_until_closed),
                approval_status_str(request.status),
                revision_to_i64(request.revision),
                request.header.schema_version as i64,
            ],
        )
        .map_err(|e| {
            if is_conflict(&e) {
                MetaError::Conflict(format!(
                    "duplicate approval request {}",
                    request.header.id.as_str()
                ))
            } else {
                MetaError::Sqlite(e)
            }
        })?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &request.room_id,
            actor_participant_id,
            CollabEventKind::ApprovalRequested,
            &request.header.id,
        )?;
        tx.commit()?;
        Ok(activity)
    }

    /// Reads one approval request row.
    pub fn get_approval_request(&self, id: &OpaqueId) -> Result<ApprovalRequest, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT request_id, room_id, realm_id, authority_scope_id, anchor_json, kind, requested_by_participant_id, assignees_json, blind_until_closed, status, revision, schema_version
             FROM collab_approval_requests WHERE request_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_approval_request_row(row)
    }

    /// Lists approval requests in one room.
    pub fn list_approval_requests(
        &self,
        room_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<ApprovalRequest>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = i64::from(limit.clamp(1, 100)) + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT request_id, room_id, realm_id, authority_scope_id, anchor_json, kind, requested_by_participant_id, assignees_json, blind_until_closed, status, revision, schema_version
             FROM collab_approval_requests WHERE room_id = ?1 AND request_id > ?2 ORDER BY request_id LIMIT ?3",
        )?;
        let mut rows = stmt.query(params![room_id.as_str(), after, limit])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_approval_request_row(row)?);
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|r| r.header.id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Lists every approval request row (backup/restore snapshot only).
    pub fn list_all_approval_requests(&self) -> Result<Vec<ApprovalRequest>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT request_id, room_id, realm_id, authority_scope_id, anchor_json, kind, requested_by_participant_id, assignees_json, blind_until_closed, status, revision, schema_version
             FROM collab_approval_requests ORDER BY request_id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_approval_request_row(row)?);
        }
        Ok(out)
    }

    /// Restores one approval request row exactly (backup/restore path only).
    pub fn restore_approval_request_row(&self, request: &ApprovalRequest) -> Result<(), MetaError> {
        let anchor_json = serde_json::to_string(&request.anchor)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let assignee_strings: Vec<&str> = request
            .assignee_participant_ids
            .iter()
            .map(OpaqueId::as_str)
            .collect();
        let assignees_json = serde_json::to_string(&assignee_strings)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_approval_requests(request_id, room_id, realm_id, authority_scope_id, anchor_json, kind, requested_by_participant_id, assignees_json, blind_until_closed, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                request.header.id.as_str(),
                request.room_id.as_str(),
                request.header.realm_id.as_opaque().as_str(),
                request.header.authority_scope_id.as_opaque().as_str(),
                anchor_json,
                approval_kind_str(request.kind),
                request.requested_by_participant_id.as_str(),
                assignees_json,
                i64::from(request.blind_until_closed),
                approval_status_str(request.status),
                revision_to_i64(request.revision.max(1)),
                request.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Compare-and-swap approval request status `Open -> Withdrawn`,
    /// appending the `ApprovalWithdrawn` activity entry in the same
    /// transaction (`migration.md` section 5). `Closed` has no dedicated
    /// `CollabEventKind` in the frozen 076 vocabulary and is not a
    /// transition this function performs; it is a deferred product
    /// decision for a later slice (when/how a request closes).
    pub fn withdraw_approval_request_with_activity(
        &self,
        id: &OpaqueId,
        expected: u64,
        activity_header: ObjectHeader,
        actor_participant_id: &OpaqueId,
    ) -> Result<(ApprovalRequest, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<(i64, String)> = tx
            .query_row(
                "SELECT revision, room_id FROM collab_approval_requests WHERE request_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((current_rev, room_id)) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != revision_to_i64(expected) {
            return Err(MetaError::Conflict(
                "stale approval request revision".to_owned(),
            ));
        }
        tx.execute(
            "UPDATE collab_approval_requests SET status = ?1, revision = revision + 1 WHERE request_id = ?2",
            params![
                approval_status_str(ApprovalRequestStatus::Withdrawn),
                id.as_str()
            ],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            &OpaqueId::new(room_id),
            actor_participant_id,
            CollabEventKind::ApprovalWithdrawn,
            id,
        )?;
        tx.commit()?;
        let request = self.get_approval_request(id)?;
        Ok((request, activity))
    }

    /// Appends a decision and its `ApprovalDecided` activity entry in one
    /// transaction (`migration.md` section 5). Multiple decisions per
    /// request are expected (dual/independent review); this row is
    /// insert-only. The caller supplies `room_id` (already resolved with
    /// the request).
    #[allow(clippy::too_many_arguments)]
    pub fn insert_approval_decision_with_activity(
        &self,
        header: ObjectHeader,
        room_id: &OpaqueId,
        request_id: &OpaqueId,
        decided_by_participant_id: &OpaqueId,
        outcome: ApprovalDecisionOutcome,
        rationale: Option<&str>,
        activity_header: ObjectHeader,
    ) -> Result<(ApprovalDecision, ActivityRecord), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let seq_key = format!("collab-decision-seq-{}", request_id.as_str());
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM store_state WHERE key = ?1",
                params![seq_key],
                |row| row.get(0),
            )
            .optional()?;
        let seq: u64 = match current.as_deref() {
            None => 1,
            Some(raw) => {
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("seq counter {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, seq.to_string()],
        )?;
        tx.execute(
            "INSERT INTO collab_approval_decisions(decision_id, request_id, realm_id, authority_scope_id, decided_by_participant_id, outcome, rationale, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                header.id.as_str(),
                request_id.as_str(),
                header.realm_id.as_opaque().as_str(),
                header.authority_scope_id.as_opaque().as_str(),
                decided_by_participant_id.as_str(),
                approval_outcome_str(outcome),
                rationale,
                seq as i64,
                header.schema_version as i64,
            ],
        )?;
        let activity = append_activity_in_tx(
            &tx,
            activity_header,
            room_id,
            decided_by_participant_id,
            CollabEventKind::ApprovalDecided,
            request_id,
        )?;
        tx.commit()?;
        let decision = ApprovalDecision {
            header,
            request_id: request_id.clone(),
            decided_by_participant_id: decided_by_participant_id.clone(),
            outcome,
            rationale: rationale.map(str::to_owned),
            seq,
        };
        Ok((decision, activity))
    }

    /// Lists every decision recorded against one request, in `seq` order.
    /// Read-time blind-review filtering (`ApprovalRequest::hides_decision_from`)
    /// is applied by the caller (Core), not by storage.
    pub fn list_approval_decisions(
        &self,
        request_id: &OpaqueId,
    ) -> Result<Vec<ApprovalDecision>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT decision_id, request_id, realm_id, authority_scope_id, decided_by_participant_id, outcome, rationale, seq, schema_version
             FROM collab_approval_decisions WHERE request_id = ?1 ORDER BY seq",
        )?;
        let mut rows = stmt.query(params![request_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let rid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let decider: String = row.get(4).map_err(MetaError::Sqlite)?;
            let outcome: String = row.get(5).map_err(MetaError::Sqlite)?;
            let rationale: Option<String> = row.get(6).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(7).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(8).map_err(MetaError::Sqlite)?;
            out.push(ApprovalDecision {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                request_id: OpaqueId::new(rid),
                decided_by_participant_id: OpaqueId::new(decider),
                outcome: parse_approval_outcome(&outcome)?,
                rationale,
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Lists every approval-decision row across every request (backup/restore
    /// snapshot only).
    pub fn list_all_approval_decisions(&self) -> Result<Vec<ApprovalDecision>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT decision_id, request_id, realm_id, authority_scope_id, decided_by_participant_id, outcome, rationale, seq, schema_version
             FROM collab_approval_decisions ORDER BY request_id, seq",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let rid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let decider: String = row.get(4).map_err(MetaError::Sqlite)?;
            let outcome: String = row.get(5).map_err(MetaError::Sqlite)?;
            let rationale: Option<String> = row.get(6).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(7).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(8).map_err(MetaError::Sqlite)?;
            out.push(ApprovalDecision {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                request_id: OpaqueId::new(rid),
                decided_by_participant_id: OpaqueId::new(decider),
                outcome: parse_approval_outcome(&outcome)?,
                rationale,
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Restores one approval-decision row exactly, including its
    /// storage-assigned `seq` (backup/restore path only).
    pub fn restore_approval_decision_row(
        &self,
        decision: &ApprovalDecision,
    ) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_approval_decisions(decision_id, request_id, realm_id, authority_scope_id, decided_by_participant_id, outcome, rationale, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                decision.header.id.as_str(),
                decision.request_id.as_str(),
                decision.header.realm_id.as_opaque().as_str(),
                decision.header.authority_scope_id.as_opaque().as_str(),
                decision.decided_by_participant_id.as_str(),
                approval_outcome_str(decision.outcome),
                decision.rationale,
                decision.seq as i64,
                decision.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// activity records (tamper-evident audit trail and activity feed)
// ---------------------------------------------------------------------------

/// Appends one activity record inside a caller-provided transaction, so the
/// primary mutation and its activity entry commit together or not at all
/// (`migration.md` section 5, `security.md` T12). Extends the room's hash
/// chain the same way `append_activity_record` does, but never opens its
/// own transaction. A crash before `tx.commit()` leaves neither the primary
/// row nor this activity record visible; it can never leave one without the
/// other.
fn append_activity_in_tx(
    tx: &Transaction<'_>,
    header: ObjectHeader,
    room_id: &OpaqueId,
    actor_participant_id: &OpaqueId,
    event_kind: CollabEventKind,
    target_object_id: &OpaqueId,
) -> Result<ActivityRecord, MetaError> {
    let prev_digest_hex: Option<String> = tx
        .query_row(
            "SELECT checkpoint_digest_hex FROM collab_activity_records WHERE room_id = ?1 ORDER BY seq DESC LIMIT 1",
            params![room_id.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    let prev_digest = match prev_digest_hex {
        Some(hex) => Some(parse_digest_hex(&hex, "activity checkpoint")?),
        None => None,
    };
    let seq_key = format!("collab-activity-seq-{}", room_id.as_str());
    let current: Option<String> = tx
        .query_row(
            "SELECT value FROM store_state WHERE key = ?1",
            params![seq_key],
            |row| row.get(0),
        )
        .optional()?;
    let seq: u64 = match current.as_deref() {
        None => 1,
        Some(raw) => {
            raw.parse::<u64>().map_err(|_| {
                MetaError::CorruptObjectBody(format!("seq counter {seq_key} is not numeric"))
            })? + 1
        }
    };
    tx.execute(
        "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
        params![seq_key, seq.to_string()],
    )?;
    let checkpoint_digest = medscale_contracts::collaboration::compute_checkpoint_digest(
        prev_digest.as_ref(),
        room_id,
        actor_participant_id,
        event_kind,
        target_object_id,
        seq,
    );
    tx.execute(
        "INSERT INTO collab_activity_records(activity_id, room_id, realm_id, authority_scope_id, actor_participant_id, event_kind, target_object_id, checkpoint_digest_hex, seq, schema_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            header.id.as_str(),
            room_id.as_str(),
            header.realm_id.as_opaque().as_str(),
            header.authority_scope_id.as_opaque().as_str(),
            actor_participant_id.as_str(),
            event_kind.as_str(),
            target_object_id.as_str(),
            digest_hex(&checkpoint_digest),
            seq as i64,
            header.schema_version as i64,
        ],
    )?;
    Ok(ActivityRecord {
        header,
        room_id: room_id.clone(),
        actor_participant_id: actor_participant_id.clone(),
        event_kind,
        target_object_id: target_object_id.clone(),
        checkpoint_digest,
        seq,
    })
}

impl SqliteMetaStore {
    /// Appends the next activity record in one room's hash chain. `seq` and
    /// `checkpoint_digest` are computed here (not client-supplied): the
    /// caller passes the mutation's own `event_kind`/`target_object_id` and
    /// this function reads the room's latest checkpoint to extend the
    /// chain. Must be called inside the same transaction as the mutation it
    /// records — callers achieve this by calling storage's own combined
    /// insert functions above where available; free-standing callers must
    /// wrap both calls in `unchecked_transaction` themselves.
    pub fn append_activity_record(
        &self,
        header: ObjectHeader,
        room_id: &OpaqueId,
        actor_participant_id: &OpaqueId,
        event_kind: CollabEventKind,
        target_object_id: &OpaqueId,
    ) -> Result<ActivityRecord, MetaError> {
        let prev_digest_hex: Option<String> = self
            .conn()
            .query_row(
                "SELECT checkpoint_digest_hex FROM collab_activity_records WHERE room_id = ?1 ORDER BY seq DESC LIMIT 1",
                params![room_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let prev_digest = match prev_digest_hex {
            Some(hex) => Some(parse_digest_hex(&hex, "activity checkpoint")?),
            None => None,
        };
        let seq = self.next_collab_seq(&format!("collab-activity-seq-{}", room_id.as_str()))?;
        let checkpoint_digest = medscale_contracts::collaboration::compute_checkpoint_digest(
            prev_digest.as_ref(),
            room_id,
            actor_participant_id,
            event_kind,
            target_object_id,
            seq,
        );
        self.conn().execute(
            "INSERT INTO collab_activity_records(activity_id, room_id, realm_id, authority_scope_id, actor_participant_id, event_kind, target_object_id, checkpoint_digest_hex, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                header.id.as_str(),
                room_id.as_str(),
                header.realm_id.as_opaque().as_str(),
                header.authority_scope_id.as_opaque().as_str(),
                actor_participant_id.as_str(),
                event_kind.as_str(),
                target_object_id.as_str(),
                digest_hex(&checkpoint_digest),
                seq as i64,
                header.schema_version as i64,
            ],
        )?;
        Ok(ActivityRecord {
            header,
            room_id: room_id.clone(),
            actor_participant_id: actor_participant_id.clone(),
            event_kind,
            target_object_id: target_object_id.clone(),
            checkpoint_digest,
            seq,
        })
    }

    /// Lists activity records for one room, in `seq` order (oldest first).
    pub fn list_activity_records(
        &self,
        room_id: &OpaqueId,
        limit: u32,
        after_seq: u64,
    ) -> Result<Vec<ActivityRecord>, MetaError> {
        let limit = i64::from(limit.clamp(1, 200));
        let mut stmt = self.conn().prepare(
            "SELECT activity_id, room_id, realm_id, authority_scope_id, actor_participant_id, event_kind, target_object_id, checkpoint_digest_hex, seq, schema_version
             FROM collab_activity_records WHERE room_id = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3",
        )?;
        let mut rows = stmt.query(params![room_id.as_str(), after_seq as i64, limit])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let rid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let actor: String = row.get(4).map_err(MetaError::Sqlite)?;
            let event_kind: String = row.get(5).map_err(MetaError::Sqlite)?;
            let target: String = row.get(6).map_err(MetaError::Sqlite)?;
            let digest_hex_value: String = row.get(7).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(8).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(9).map_err(MetaError::Sqlite)?;
            out.push(ActivityRecord {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                room_id: OpaqueId::new(rid),
                actor_participant_id: OpaqueId::new(actor),
                event_kind: CollabEventKind::parse(&event_kind)
                    .map_err(MetaError::UnsupportedSchema)?,
                target_object_id: OpaqueId::new(target),
                checkpoint_digest: parse_digest_hex(&digest_hex_value, "activity checkpoint")?,
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Lists every activity record across every room, ordered by room then
    /// `seq` (backup/restore snapshot only).
    pub fn list_all_activity_records(&self) -> Result<Vec<ActivityRecord>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT activity_id, room_id, realm_id, authority_scope_id, actor_participant_id, event_kind, target_object_id, checkpoint_digest_hex, seq, schema_version
             FROM collab_activity_records ORDER BY room_id, seq",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0).map_err(MetaError::Sqlite)?;
            let rid: String = row.get(1).map_err(MetaError::Sqlite)?;
            let realm: String = row.get(2).map_err(MetaError::Sqlite)?;
            let scope: String = row.get(3).map_err(MetaError::Sqlite)?;
            let actor: String = row.get(4).map_err(MetaError::Sqlite)?;
            let event_kind: String = row.get(5).map_err(MetaError::Sqlite)?;
            let target: String = row.get(6).map_err(MetaError::Sqlite)?;
            let digest_hex_value: String = row.get(7).map_err(MetaError::Sqlite)?;
            let seq: i64 = row.get(8).map_err(MetaError::Sqlite)?;
            let schema_version: i64 = row.get(9).map_err(MetaError::Sqlite)?;
            out.push(ActivityRecord {
                header: header_of(&id, &realm, &scope, schema_version, 1),
                room_id: OpaqueId::new(rid),
                actor_participant_id: OpaqueId::new(actor),
                event_kind: CollabEventKind::parse(&event_kind)
                    .map_err(MetaError::UnsupportedSchema)?,
                target_object_id: OpaqueId::new(target),
                checkpoint_digest: parse_digest_hex(&digest_hex_value, "activity checkpoint")?,
                seq: u64::try_from(seq).unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Restores one activity record exactly, preserving its stored
    /// `checkpoint_digest` verbatim (never recomputed) so the restored
    /// chain matches the backed-up chain bit-for-bit (backup/restore path
    /// only).
    pub fn restore_activity_record_row(&self, record: &ActivityRecord) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO collab_activity_records(activity_id, room_id, realm_id, authority_scope_id, actor_participant_id, event_kind, target_object_id, checkpoint_digest_hex, seq, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.header.id.as_str(),
                record.room_id.as_str(),
                record.header.realm_id.as_opaque().as_str(),
                record.header.authority_scope_id.as_opaque().as_str(),
                record.actor_participant_id.as_str(),
                record.event_kind.as_str(),
                record.target_object_id.as_str(),
                digest_hex(&record.checkpoint_digest),
                record.seq as i64,
                record.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }

    /// Verifies the full hash chain for one room from `seq = 1`. Returns
    /// `Err` naming the first `seq` whose stored digest does not match the
    /// recomputed chain (edited, deleted, or reordered row).
    pub fn verify_activity_chain(&self, room_id: &OpaqueId) -> Result<(), MetaError> {
        let records = self.list_activity_records(room_id, 10_000, 0)?;
        let mut prev: Option<DigestSha256> = None;
        for record in &records {
            record
                .validate_chain(prev.as_ref())
                .map_err(MetaError::CorruptObjectBody)?;
            prev = Some(record.checkpoint_digest.clone());
        }
        Ok(())
    }
}

//! Collaboration Substrate view-models (Spec 076).
//!
//! Desktop reads and mutates collaboration state only through the Core-owned
//! `CliSession` (facade authority; never storage). Every function below maps
//! one typed Core result to plain view-model rows plus an explicit status
//! string. No fake product data is ever synthesized. Reuses the same
//! `desktop-projects` session Spec 074/075 already open: Rooms are scoped to
//! Projects, so no second vault/session is needed.

use medscale_contracts::collaboration::ParticipantKind;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactKind, ArtifactVersionBinding};
use medscale_core::CliSession;

/// One Room list row.
#[derive(Debug, Clone, Default)]
pub struct RoomRowVm {
    pub id: String,
    pub name: String,
    pub revision: u64,
}

/// One Thread list row, including its live artifact resolution.
#[derive(Debug, Clone, Default)]
pub struct ThreadRowVm {
    pub id: String,
    pub status: String,
    pub resolution: String,
    pub artifact_id: String,
}

/// One Message row within a thread.
#[derive(Debug, Clone, Default)]
pub struct MessageRowVm {
    pub author: String,
    pub body: String,
    pub seq: u64,
}

/// One Task list row.
#[derive(Debug, Clone, Default)]
pub struct TaskRowVm {
    pub id: String,
    pub title: String,
    pub status: String,
    pub revision: u64,
}

/// Maps a typed Core error to an explicit collaboration status (no payload leak).
#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::Conflict { .. } => "Conflict: stale revision, nothing written",
        AuthorityError::InvalidArgument { .. } => "Invalid: rejected before any write",
        AuthorityError::Corrupt { .. } => "Corrupt: integrity check failed",
        AuthorityError::UnsupportedSchema { .. } => "Unsupported: unknown schema value",
        AuthorityError::Cancelled { .. } => "Cancelled: no write was performed",
        _ => "Unavailable: vault or Core not ready",
    }
}

/// Registers (or reuses) the Human participant backing the current session's
/// *real* holder id (`CliSession::holder_id`), so the first collaboration
/// action in a fresh vault does not fail with "not a registered
/// participant". Idempotent. Must register under the session's actual bound
/// holder, not an arbitrary display string: `Collab::caller_participant`
/// resolves the caller via the session's real actor id, so a mismatched
/// `holder_id` would register a participant Core can never recognize as the
/// caller (every subsequent room/thread/message/task action would fail
/// closed with `Unauthorized`).
fn ensure_self_participant(
    session: &mut CliSession,
    display_name: &str,
) -> Result<OpaqueId, AuthorityError> {
    let holder_id = session.holder_id();
    let participant = session.collab_participant_register(
        holder_id,
        ParticipantKind::Human,
        display_name.to_owned(),
        None,
    )?;
    Ok(participant.header.id)
}

/// Lists Rooms for one Project through Core, registering the desktop
/// operator as a participant first if needed.
pub fn refresh_rooms(
    session: &mut CliSession,
    project_id: &str,
) -> Result<Vec<RoomRowVm>, AuthorityError> {
    ensure_self_participant(session, "Desktop Operator")?;
    let (rooms, _) = session.collab_room_list(OpaqueId::new(project_id), None, Some(100), None)?;
    Ok(rooms
        .into_iter()
        .map(|room| RoomRowVm {
            id: room.header.id.as_str().to_owned(),
            name: room.name,
            revision: room.revision,
        })
        .collect())
}

/// Creates a Room in one Project (registers the operator participant first).
pub fn create_room(
    session: &mut CliSession,
    project_id: &str,
    name: String,
) -> Result<RoomRowVm, AuthorityError> {
    ensure_self_participant(session, "Desktop Operator")?;
    let room = session.collab_room_create(OpaqueId::new(project_id), None, name)?;
    Ok(RoomRowVm {
        id: room.header.id.as_str().to_owned(),
        name: room.name,
        revision: room.revision,
    })
}

fn resolution_label(resolution: medscale_contracts::project_graph::ReferenceResolution) -> String {
    use medscale_contracts::project_graph::ReferenceResolution;
    match resolution {
        ReferenceResolution::Current => "current",
        ReferenceResolution::Stale => "stale",
        ReferenceResolution::Missing => "missing",
        ReferenceResolution::UnsupportedKind => "unsupported_kind",
        ReferenceResolution::Denied => "denied",
        ReferenceResolution::Corrupt => "corrupt",
    }
    .to_owned()
}

/// Lists threads in one Room, each with its live artifact resolution.
pub fn refresh_threads(
    session: &mut CliSession,
    room_id: &str,
) -> Result<Vec<ThreadRowVm>, AuthorityError> {
    let (threads, resolutions, _) =
        session.collab_thread_list(OpaqueId::new(room_id), Some(100), None)?;
    Ok(threads
        .into_iter()
        .zip(resolutions)
        .map(|(thread, resolution)| ThreadRowVm {
            id: thread.header.id.as_str().to_owned(),
            status: format!("{:?}", thread.status),
            resolution: resolution_label(resolution),
            artifact_id: thread.anchor.artifact.object_id.as_str().to_owned(),
        })
        .collect())
}

/// Opens a thread anchored to a `SourceRecord`-kind artifact by id. This is
/// the operator-facing default because `SourceRecord` is one of the kinds
/// `Collab::resolve_anchor_artifact` can actually resolve against the live
/// in-memory store (a real, previously-ingested source id will show
/// `current`; an unknown id honestly shows `missing`). `EvidenceDocument`
/// and `OtherExplicit` always report `UnsupportedKind` regardless of the id
/// (`contracts.md` section 6), so they would make every thread opened from
/// this quick-entry field look identically "unsupported" -- a misleading
/// default this module deliberately avoids.
pub fn open_thread(
    session: &mut CliSession,
    room_id: &str,
    artifact_id: &str,
) -> Result<ThreadRowVm, AuthorityError> {
    let anchor = medscale_contracts::collaboration::AnchorTarget {
        artifact: ArtifactDescriptor {
            object_id: OpaqueId::new(artifact_id),
            kind: ArtifactKind::SourceRecord,
            binding: ArtifactVersionBinding::IdentityOnly,
        },
        detail: None,
    };
    let (thread, resolution) = session.collab_thread_open(OpaqueId::new(room_id), anchor)?;
    Ok(ThreadRowVm {
        id: thread.header.id.as_str().to_owned(),
        status: format!("{:?}", thread.status),
        resolution: resolution_label(resolution),
        artifact_id: thread.anchor.artifact.object_id.as_str().to_owned(),
    })
}

/// Lists messages in one thread.
pub fn refresh_messages(
    session: &mut CliSession,
    thread_id: &str,
) -> Result<Vec<MessageRowVm>, AuthorityError> {
    let messages = session.collab_message_list(OpaqueId::new(thread_id), Some(200), None)?;
    Ok(messages
        .into_iter()
        .map(|message| MessageRowVm {
            author: message.author_participant_id.as_str().to_owned(),
            body: message.body,
            seq: message.seq,
        })
        .collect())
}

/// Posts a message to a thread.
pub fn post_message(
    session: &mut CliSession,
    thread_id: &str,
    body: String,
) -> Result<(), AuthorityError> {
    session.collab_message_post(OpaqueId::new(thread_id), body)?;
    Ok(())
}

/// Lists tasks in one Room.
pub fn refresh_tasks(
    session: &mut CliSession,
    room_id: &str,
) -> Result<Vec<TaskRowVm>, AuthorityError> {
    let (tasks, _) = session.collab_task_list(OpaqueId::new(room_id), Some(100), None)?;
    Ok(tasks
        .into_iter()
        .map(|task| TaskRowVm {
            id: task.header.id.as_str().to_owned(),
            title: task.title,
            status: format!("{:?}", task.status),
            revision: task.revision,
        })
        .collect())
}

/// Creates a task in one Room (no anchor; a plain room to-do).
pub fn create_task(
    session: &mut CliSession,
    room_id: &str,
    title: String,
) -> Result<TaskRowVm, AuthorityError> {
    let task = session.collab_task_create(OpaqueId::new(room_id), None, title, None)?;
    Ok(TaskRowVm {
        id: task.header.id.as_str().to_owned(),
        title: task.title,
        status: format!("{:?}", task.status),
        revision: task.revision,
    })
}

/// Marks a task `Done` (revision-guarded).
pub fn complete_task(
    session: &mut CliSession,
    task_id: &str,
    expected_revision: u64,
) -> Result<TaskRowVm, AuthorityError> {
    let task = session.collab_task_update(
        OpaqueId::new(task_id),
        expected_revision,
        medscale_contracts::collaboration::TaskStatus::Done,
        None,
    )?;
    Ok(TaskRowVm {
        id: task.header.id.as_str().to_owned(),
        title: task.title,
        status: format!("{:?}", task.status),
        revision: task.revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session(name: &str) -> CliSession {
        let dir = std::env::temp_dir().join(format!("medscale-076d-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut session = CliSession::connect("desktop-collab-test").expect("operator session");
        session
            .open_synthetic_vault(&dir.display().to_string())
            .expect("open vault");
        session
    }

    /// Mirrors Spec 075's `workbench_flows_through_real_core_session`: every
    /// view-model function in this module is exercised against a real
    /// `CliSession` (Core), never synthetic/fake data, so the Desktop
    /// collaboration panel's data path is proven even though this
    /// workstation cannot render the Slint UI locally or in CI.
    #[test]
    fn collab_workspace_flows_through_real_core_session() {
        let mut session = test_session("flows");
        let project = session
            .project_create("study".to_owned(), None)
            .expect("project");
        let project_id = project.header.id.as_str().to_owned();

        let room =
            create_room(&mut session, &project_id, "trial design".to_owned()).expect("create room");
        assert_eq!(room.name, "trial design");
        let rooms = refresh_rooms(&mut session, &project_id).expect("refresh rooms");
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, room.id);

        // No CliSession convenience method creates a SourceRecord (only the
        // raw Capability::CreateSourceRecord facade path does, which
        // CliSession does not expose publicly), so this proves the honest
        // fail-closed side of live resolution: an unregistered artifact id
        // resolves Missing, never a fabricated Current.
        let thread = open_thread(&mut session, &room.id, "source-unknown").expect("open thread");
        assert_eq!(thread.resolution, "missing");
        let threads = refresh_threads(&mut session, &room.id).expect("refresh threads");
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].artifact_id, "source-unknown");

        post_message(&mut session, &thread.id, "first message".to_owned()).expect("post message");
        let messages = refresh_messages(&mut session, &thread.id).expect("refresh messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].body, "first message");

        let task = create_task(&mut session, &room.id, "collect baseline labs".to_owned())
            .expect("create task");
        assert_eq!(task.status, "Open");
        let tasks = refresh_tasks(&mut session, &room.id).expect("refresh tasks");
        assert_eq!(tasks.len(), 1);
        let done = complete_task(&mut session, &task.id, task.revision).expect("complete task");
        assert_eq!(done.status, "Done");

        // Statuses stay explicit for every error class (never a payload leak).
        assert_eq!(
            status_message(&AuthorityError::NotFound),
            "Missing: not found in this vault"
        );
    }

    #[test]
    fn status_message_never_empty() {
        assert!(!status_message(&AuthorityError::NotFound).is_empty());
        assert!(
            !status_message(&AuthorityError::Conflict {
                message: "x".to_owned()
            })
            .is_empty()
        );
    }

    #[test]
    fn resolution_label_covers_every_variant() {
        use medscale_contracts::project_graph::ReferenceResolution;
        for resolution in [
            ReferenceResolution::Current,
            ReferenceResolution::Stale,
            ReferenceResolution::Missing,
            ReferenceResolution::UnsupportedKind,
            ReferenceResolution::Denied,
            ReferenceResolution::Corrupt,
        ] {
            assert!(!resolution_label(resolution).is_empty());
        }
    }
}

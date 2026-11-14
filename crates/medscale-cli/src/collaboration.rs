//! Spec 076 collaboration substrate commands (CLI vertical slice through Core).
//!
//! Every command opens the session scope, dispatches one typed Core request
//! via `CliSession`, and renders the typed result as human lines or stable
//! JSON. The CLI never writes collaboration storage directly.
//!
//! Anchor input is deliberately simplified here to `IdentityOnly` bindings
//! (`--artifact-id` + `--artifact-kind`): a CLI operator surface does not
//! need every `ArtifactVersionBinding` precision Core supports internally.
//! This is a CLI scope simplification, not a Core limitation.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::collaboration::{
    ApprovalDecisionOutcome, ApprovalKind, MembershipRole, ParticipantKind, RoomStatus,
    ThreadStatus,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_contracts::project_graph::{ArtifactDescriptor, ArtifactVersionBinding};

use super::{fail_json, print_json_or_debug};

fn collab_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => ("denied", debug),
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::StaleReference { message } => ("stale_reference", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::UnsupportedSchema { message } => ("unsupported_schema", message.clone()),
        AuthorityError::Unavailable { message } => ("unavailable", message.clone()),
        AuthorityError::Cancelled { message } => ("cancelled", message.clone()),
        AuthorityError::LeaseRequired
        | AuthorityError::VaultRequired
        | AuthorityError::MissingKeyMaterial => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

fn open_collab_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| collab_fail(&err, json))
}

fn parse_participant_kind(value: &str) -> Result<ParticipantKind, String> {
    ParticipantKind::parse(value)
}

fn parse_room_status(value: &str) -> Result<RoomStatus, String> {
    match value {
        "active" => Ok(RoomStatus::Active),
        "archived" => Ok(RoomStatus::Archived),
        other => Err(format!("unknown room status {other}")),
    }
}

fn parse_thread_status(value: &str) -> Result<ThreadStatus, String> {
    match value {
        "open" => Ok(ThreadStatus::Open),
        "resolved" => Ok(ThreadStatus::Resolved),
        "reopened" => Ok(ThreadStatus::Reopened),
        other => Err(format!("unknown thread status {other}")),
    }
}

fn parse_task_status(value: &str) -> Result<medscale_contracts::collaboration::TaskStatus, String> {
    use medscale_contracts::collaboration::TaskStatus;
    match value {
        "open" => Ok(TaskStatus::Open),
        "in_progress" => Ok(TaskStatus::InProgress),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        "cancelled" => Ok(TaskStatus::Cancelled),
        other => Err(format!("unknown task status {other}")),
    }
}

fn parse_membership_role(value: &str) -> Result<MembershipRole, String> {
    match value {
        "owner" => Ok(MembershipRole::Owner),
        "member" => Ok(MembershipRole::Member),
        other => Err(format!("unknown membership role {other}")),
    }
}

fn parse_approval_kind(value: &str) -> Result<ApprovalKind, String> {
    match value {
        "review" => Ok(ApprovalKind::Review),
        "approval" => Ok(ApprovalKind::Approval),
        "adjudication" => Ok(ApprovalKind::Adjudication),
        other => Err(format!("unknown approval kind {other}")),
    }
}

fn parse_approval_outcome(value: &str) -> Result<ApprovalDecisionOutcome, String> {
    match value {
        "approved" => Ok(ApprovalDecisionOutcome::Approved),
        "rejected" => Ok(ApprovalDecisionOutcome::Rejected),
        "changes_requested" => Ok(ApprovalDecisionOutcome::ChangesRequested),
        "abstained" => Ok(ApprovalDecisionOutcome::Abstained),
        other => Err(format!("unknown approval outcome {other}")),
    }
}

fn parse_artifact_kind(
    value: &str,
) -> Result<medscale_contracts::project_graph::ArtifactKind, String> {
    crate::project::parse_kind(value)
}

/// Builds an `IdentityOnly`-bound anchor from CLI-simple inputs (see module
/// docs for why the CLI does not expose every `ArtifactVersionBinding`).
fn simple_anchor(
    artifact_id: String,
    artifact_kind: &str,
) -> Result<medscale_contracts::collaboration::AnchorTarget, String> {
    let kind = parse_artifact_kind(artifact_kind)?;
    let artifact = ArtifactDescriptor {
        object_id: OpaqueId::new(artifact_id),
        kind,
        binding: ArtifactVersionBinding::IdentityOnly,
    };
    Ok(medscale_contracts::collaboration::AnchorTarget {
        artifact,
        detail: None,
    })
}

fn print_participant_human(p: &medscale_contracts::collaboration::ParticipantIdentity) {
    println!("participant_id: {}", p.header.id.as_str());
    println!("holder_id: {}", p.holder_id.as_str());
    println!("kind: {}", p.kind.as_str());
    println!("display_name: {}", p.display_name);
    println!("revision: {}", p.revision);
}

fn print_room_human(r: &medscale_contracts::collaboration::Room) {
    println!("room_id: {}", r.header.id.as_str());
    println!("project_id: {}", r.project_id.as_str());
    println!("name: {}", r.name);
    println!("revision: {}", r.revision);
}

fn print_thread_human(
    t: &medscale_contracts::collaboration::ThreadRef,
    resolution: medscale_contracts::project_graph::ReferenceResolution,
) {
    println!("thread_id: {}", t.header.id.as_str());
    println!("room_id: {}", t.room_id.as_str());
    println!("status: {:?}", t.status);
    println!("revision: {}", t.revision);
    println!("resolution: {resolution:?}");
}

fn print_task_human(t: &medscale_contracts::collaboration::Task) {
    println!("task_id: {}", t.header.id.as_str());
    println!("room_id: {}", t.room_id.as_str());
    println!("title: {}", t.title);
    println!("status: {:?}", t.status);
    println!("revision: {}", t.revision);
}

/// Spec 076 collaboration commands (participant / room / membership).
#[derive(Debug, Subcommand)]
pub enum CollabCmd {
    /// Register a participant (idempotent on holder_id).
    ParticipantRegister {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        holder_id: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        display_name: String,
        #[arg(long)]
        agent_profile_ref: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one participant.
    ParticipantShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        participant_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke a participant.
    ParticipantRevoke {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        participant_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Create a Room scoped to a Project.
    RoomCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        experiment_id: Option<String>,
        #[arg(long)]
        name: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one Room.
    RoomShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List Rooms in a Project the caller is a member of.
    RoomList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Rename a Room.
    RoomRename {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        name: String,
        #[arg(long)]
        json: bool,
    },
    /// Archive a Room (Owner role required).
    RoomArchive {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Add a member to a Room.
    MembershipAdd {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        participant_id: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        json: bool,
    },
    /// List active memberships for a Room.
    MembershipList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Remove a member (self, or Owner removing another).
    MembershipRemove {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        membership_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
}

/// Spec 076 collaboration commands (thread / message / task / note / approval / activity).
#[derive(Debug, Subcommand)]
pub enum CollabWorkCmd {
    /// Open a thread anchored to an exact artifact revision.
    ThreadOpen {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        artifact_id: String,
        #[arg(long)]
        artifact_kind: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one thread with its live artifact resolution.
    ThreadShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        thread_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List threads in a Room with live resolution.
    ThreadList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Transition a thread's status (Open<->Resolved<->Reopened only).
    ThreadSetStatus {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        thread_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        status: String,
        #[arg(long)]
        json: bool,
    },
    /// Post a message to a thread.
    MessagePost {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        thread_id: String,
        #[arg(long)]
        body: String,
        #[arg(long)]
        json: bool,
    },
    /// List messages in a thread.
    MessageList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        thread_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        after_seq: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Edit a message's body (author-only).
    MessageEdit {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        message_id: String,
        #[arg(long)]
        new_body: String,
        #[arg(long)]
        json: bool,
    },
    /// Delete a message (append-only tombstone; author-only).
    MessageDelete {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        message_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Create a task, optionally anchored to an artifact.
    TaskCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        artifact_id: Option<String>,
        #[arg(long)]
        artifact_kind: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one task.
    TaskShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List tasks in a Room.
    TaskList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Update task status/assignee (revision-guarded).
    TaskUpdate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        status: String,
        #[arg(long)]
        assignee_participant_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create a note with its initial body.
    NoteCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one note document (pointer only).
    NoteShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        note_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List a note's full revision history (including conflict copies).
    NoteRevisions {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        note_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Edit a note (fast-forward, or explicit conflict copy on stale write).
    NoteEdit {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        note_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        body: String,
        #[arg(long)]
        json: bool,
    },
    /// Create an approval request (supports multiple assignees + blind mode).
    ApprovalCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        artifact_id: String,
        #[arg(long)]
        artifact_kind: String,
        #[arg(long)]
        kind: String,
        #[arg(long, value_delimiter = ',')]
        assignee_participant_ids: Vec<String>,
        #[arg(long)]
        blind_until_closed: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show one approval request.
    ApprovalShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        request_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Withdraw an open approval request (requester-only).
    ApprovalWithdraw {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        request_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
    /// Record a decision against an open approval request (assignee-only).
    ApprovalDecide {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        request_id: String,
        #[arg(long)]
        outcome: String,
        #[arg(long)]
        rationale: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List decisions recorded against a request (blind-mode filtered).
    ApprovalDecisions {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        request_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List a room's tamper-evident activity trail.
    ActivityList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        after_seq: Option<u64>,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_collab(action: CollabCmd) -> anyhow::Result<()> {
    match action {
        CollabCmd::ParticipantRegister {
            vault_id,
            vault_root,
            holder_id,
            kind,
            display_name,
            agent_profile_ref,
            json,
        } => {
            let kind = parse_participant_kind(&kind).map_err(|m| invalid(m, json))?;
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let participant = session
                .collab_participant_register(
                    OpaqueId::new(holder_id),
                    kind,
                    display_name,
                    agent_profile_ref.map(OpaqueId::new),
                )
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&participant, true)?;
            } else {
                print_participant_human(&participant);
            }
            Ok(())
        }
        CollabCmd::ParticipantShow {
            vault_id,
            vault_root,
            participant_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let participant = session
                .collab_participant_get(OpaqueId::new(participant_id))
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&participant, true)?;
            } else {
                print_participant_human(&participant);
            }
            Ok(())
        }
        CollabCmd::ParticipantRevoke {
            vault_id,
            vault_root,
            participant_id,
            expected_revision,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let participant = session
                .collab_participant_revoke(OpaqueId::new(participant_id), expected_revision)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&participant, true)?;
            } else {
                print_participant_human(&participant);
            }
            Ok(())
        }
        CollabCmd::RoomCreate {
            vault_id,
            vault_root,
            project_id,
            experiment_id,
            name,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let room = session
                .collab_room_create(
                    OpaqueId::new(project_id),
                    experiment_id.map(OpaqueId::new),
                    name,
                )
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&room, true)?;
            } else {
                print_room_human(&room);
            }
            Ok(())
        }
        CollabCmd::RoomShow {
            vault_id,
            vault_root,
            room_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let room = session
                .collab_room_get(OpaqueId::new(room_id))
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&room, true)?;
            } else {
                print_room_human(&room);
            }
            Ok(())
        }
        CollabCmd::RoomList {
            vault_id,
            vault_root,
            project_id,
            status,
            limit,
            cursor,
            json,
        } => {
            let status = status
                .map(|s| parse_room_status(&s))
                .transpose()
                .map_err(|m| invalid(m, json))?;
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let (rooms, next_cursor) = session
                .collab_room_list(OpaqueId::new(project_id), status, limit, cursor)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"rooms": rooms, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if rooms.is_empty() {
                println!("rooms: empty");
            } else {
                for room in &rooms {
                    println!(
                        "{} | {} | rev={}",
                        room.header.id.as_str(),
                        room.name,
                        room.revision
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        CollabCmd::RoomRename {
            vault_id,
            vault_root,
            room_id,
            expected_revision,
            name,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let room = session
                .collab_room_rename(OpaqueId::new(room_id), expected_revision, name)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&room, true)?;
            } else {
                print_room_human(&room);
            }
            Ok(())
        }
        CollabCmd::RoomArchive {
            vault_id,
            vault_root,
            room_id,
            expected_revision,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let room = session
                .collab_room_archive(OpaqueId::new(room_id), expected_revision)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&room, true)?;
            } else {
                print_room_human(&room);
            }
            Ok(())
        }
        CollabCmd::MembershipAdd {
            vault_id,
            vault_root,
            room_id,
            participant_id,
            role,
            json,
        } => {
            let role = parse_membership_role(&role).map_err(|m| invalid(m, json))?;
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let membership = session
                .collab_membership_add(OpaqueId::new(room_id), OpaqueId::new(participant_id), role)
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&membership, json)
        }
        CollabCmd::MembershipList {
            vault_id,
            vault_root,
            room_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let memberships = session
                .collab_membership_list(OpaqueId::new(room_id))
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&memberships, true)
            } else {
                if memberships.is_empty() {
                    println!("memberships: empty");
                }
                for m in &memberships {
                    println!(
                        "{} | participant={} | role={:?} | rev={}",
                        m.header.id.as_str(),
                        m.participant_id.as_str(),
                        m.role,
                        m.revision
                    );
                }
                Ok(())
            }
        }
        CollabCmd::MembershipRemove {
            vault_id,
            vault_root,
            room_id,
            membership_id,
            expected_revision,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let membership = session
                .collab_membership_remove(
                    OpaqueId::new(room_id),
                    OpaqueId::new(membership_id),
                    expected_revision,
                )
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&membership, json)
        }
    }
}

pub fn run_collab_work(action: CollabWorkCmd) -> anyhow::Result<()> {
    match action {
        CollabWorkCmd::ThreadOpen {
            vault_id,
            vault_root,
            room_id,
            artifact_id,
            artifact_kind,
            json,
        } => {
            let anchor =
                simple_anchor(artifact_id, &artifact_kind).map_err(|m| invalid(m, json))?;
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let (thread, resolution) = session
                .collab_thread_open(OpaqueId::new(room_id), anchor)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"thread": thread, "resolution": resolution}),
                    true,
                )
            } else {
                print_thread_human(&thread, resolution);
                Ok(())
            }
        }
        CollabWorkCmd::ThreadShow {
            vault_id,
            vault_root,
            thread_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let (thread, resolution) = session
                .collab_thread_get(OpaqueId::new(thread_id))
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"thread": thread, "resolution": resolution}),
                    true,
                )
            } else {
                print_thread_human(&thread, resolution);
                Ok(())
            }
        }
        CollabWorkCmd::ThreadList {
            vault_id,
            vault_root,
            room_id,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let (threads, resolutions, next_cursor) = session
                .collab_thread_list(OpaqueId::new(room_id), limit, cursor)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"threads": threads, "resolutions": resolutions, "next_cursor": next_cursor}),
                    true,
                )
            } else {
                if threads.is_empty() {
                    println!("threads: empty");
                }
                for (thread, resolution) in threads.iter().zip(resolutions.iter()) {
                    println!(
                        "{} | status={:?} | resolution={resolution:?}",
                        thread.header.id.as_str(),
                        thread.status
                    );
                }
                Ok(())
            }
        }
        CollabWorkCmd::ThreadSetStatus {
            vault_id,
            vault_root,
            thread_id,
            expected_revision,
            status,
            json,
        } => {
            let status = parse_thread_status(&status).map_err(|m| invalid(m, json))?;
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let (thread, resolution) = session
                .collab_thread_set_status(OpaqueId::new(thread_id), expected_revision, status)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"thread": thread, "resolution": resolution}),
                    true,
                )
            } else {
                print_thread_human(&thread, resolution);
                Ok(())
            }
        }
        CollabWorkCmd::MessagePost {
            vault_id,
            vault_root,
            thread_id,
            body,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let message = session
                .collab_message_post(OpaqueId::new(thread_id), body)
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&message, json)
        }
        CollabWorkCmd::MessageList {
            vault_id,
            vault_root,
            thread_id,
            limit,
            after_seq,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let messages = session
                .collab_message_list(OpaqueId::new(thread_id), limit, after_seq)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&messages, true)
            } else {
                if messages.is_empty() {
                    println!("messages: empty");
                }
                for m in &messages {
                    println!(
                        "[{}] {}: {}",
                        m.seq,
                        m.author_participant_id.as_str(),
                        m.body
                    );
                }
                Ok(())
            }
        }
        CollabWorkCmd::MessageEdit {
            vault_id,
            vault_root,
            message_id,
            new_body,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let edit = session
                .collab_message_edit(OpaqueId::new(message_id), new_body)
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&edit, json)
        }
        CollabWorkCmd::MessageDelete {
            vault_id,
            vault_root,
            message_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let edit = session
                .collab_message_delete(OpaqueId::new(message_id))
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&edit, json)
        }
        CollabWorkCmd::TaskCreate {
            vault_id,
            vault_root,
            room_id,
            artifact_id,
            artifact_kind,
            title,
            description,
            json,
        } => {
            let anchor = match (artifact_id, artifact_kind) {
                (Some(id), Some(kind)) => {
                    Some(simple_anchor(id, &kind).map_err(|m| invalid(m, json))?)
                }
                (None, None) => None,
                _ => {
                    return Err(invalid(
                        "artifact_id and artifact_kind must both be set or both omitted".to_owned(),
                        json,
                    ));
                }
            };
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let task = session
                .collab_task_create(OpaqueId::new(room_id), anchor, title, description)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&task, true)
            } else {
                print_task_human(&task);
                Ok(())
            }
        }
        CollabWorkCmd::TaskShow {
            vault_id,
            vault_root,
            task_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let task = session
                .collab_task_get(OpaqueId::new(task_id))
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&task, true)
            } else {
                print_task_human(&task);
                Ok(())
            }
        }
        CollabWorkCmd::TaskList {
            vault_id,
            vault_root,
            room_id,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let (tasks, next_cursor) = session
                .collab_task_list(OpaqueId::new(room_id), limit, cursor)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"tasks": tasks, "next_cursor": next_cursor}),
                    true,
                )
            } else {
                if tasks.is_empty() {
                    println!("tasks: empty");
                }
                for t in &tasks {
                    println!(
                        "{} | {} | status={:?} | rev={}",
                        t.header.id.as_str(),
                        t.title,
                        t.status,
                        t.revision
                    );
                }
                Ok(())
            }
        }
        CollabWorkCmd::TaskUpdate {
            vault_id,
            vault_root,
            task_id,
            expected_revision,
            status,
            assignee_participant_id,
            json,
        } => {
            let status = parse_task_status(&status).map_err(|m| invalid(m, json))?;
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let task = session
                .collab_task_update(
                    OpaqueId::new(task_id),
                    expected_revision,
                    status,
                    assignee_participant_id.map(OpaqueId::new),
                )
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&task, true)
            } else {
                print_task_human(&task);
                Ok(())
            }
        }
        CollabWorkCmd::NoteCreate {
            vault_id,
            vault_root,
            room_id,
            title,
            body,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let note = session
                .collab_note_create(OpaqueId::new(room_id), title, body)
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&note, json)
        }
        CollabWorkCmd::NoteShow {
            vault_id,
            vault_root,
            note_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let note = session
                .collab_note_get(OpaqueId::new(note_id))
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&note, json)
        }
        CollabWorkCmd::NoteRevisions {
            vault_id,
            vault_root,
            note_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let revisions = session
                .collab_note_revisions(OpaqueId::new(note_id))
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&revisions, true)
            } else {
                if revisions.is_empty() {
                    println!("revisions: empty");
                }
                for r in &revisions {
                    let conflict = r
                        .conflict_of
                        .as_ref()
                        .map(|c| format!(" (conflict copy of {})", c.as_str()))
                        .unwrap_or_default();
                    println!(
                        "rev {} by {}{conflict}: {}",
                        r.revision,
                        r.author_participant_id.as_str(),
                        r.body
                    );
                }
                Ok(())
            }
        }
        CollabWorkCmd::NoteEdit {
            vault_id,
            vault_root,
            note_id,
            expected_revision,
            body,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let (note, new_revision, is_conflict_copy) = session
                .collab_note_edit(OpaqueId::new(note_id), expected_revision, body)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"note": note, "new_revision": new_revision, "is_conflict_copy": is_conflict_copy}),
                    true,
                )
            } else {
                println!("note_id: {}", note.header.id.as_str());
                println!("pointer_revision: {}", note.revision);
                println!("is_conflict_copy: {is_conflict_copy}");
                println!("written_revision: {}", new_revision.revision);
                println!("body: {}", new_revision.body);
                Ok(())
            }
        }
        CollabWorkCmd::ApprovalCreate {
            vault_id,
            vault_root,
            room_id,
            artifact_id,
            artifact_kind,
            kind,
            assignee_participant_ids,
            blind_until_closed,
            json,
        } => {
            let anchor =
                simple_anchor(artifact_id, &artifact_kind).map_err(|m| invalid(m, json))?;
            let kind = parse_approval_kind(&kind).map_err(|m| invalid(m, json))?;
            let assignees = assignee_participant_ids
                .into_iter()
                .map(OpaqueId::new)
                .collect();
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let request = session
                .collab_approval_request_create(
                    OpaqueId::new(room_id),
                    anchor,
                    kind,
                    assignees,
                    blind_until_closed,
                )
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&request, json)
        }
        CollabWorkCmd::ApprovalShow {
            vault_id,
            vault_root,
            request_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let request = session
                .collab_approval_request_get(OpaqueId::new(request_id))
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&request, json)
        }
        CollabWorkCmd::ApprovalWithdraw {
            vault_id,
            vault_root,
            request_id,
            expected_revision,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let request = session
                .collab_approval_request_withdraw(OpaqueId::new(request_id), expected_revision)
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&request, json)
        }
        CollabWorkCmd::ApprovalDecide {
            vault_id,
            vault_root,
            request_id,
            outcome,
            rationale,
            json,
        } => {
            let outcome = parse_approval_outcome(&outcome).map_err(|m| invalid(m, json))?;
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let decision = session
                .collab_approval_decide(OpaqueId::new(request_id), outcome, rationale)
                .map_err(|err| collab_fail(&err, json))?;
            print_json_or_debug(&decision, json)
        }
        CollabWorkCmd::ApprovalDecisions {
            vault_id,
            vault_root,
            request_id,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let decisions = session
                .collab_approval_decision_list(OpaqueId::new(request_id))
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&decisions, true)
            } else {
                if decisions.is_empty() {
                    println!("decisions: empty");
                }
                for d in &decisions {
                    println!(
                        "[{}] {} -> {:?}",
                        d.seq,
                        d.decided_by_participant_id.as_str(),
                        d.outcome
                    );
                }
                Ok(())
            }
        }
        CollabWorkCmd::ActivityList {
            vault_id,
            vault_root,
            room_id,
            limit,
            after_seq,
            json,
        } => {
            let mut session = open_collab_session(&vault_id, &vault_root, json)?;
            let records = session
                .collab_activity_list(OpaqueId::new(room_id), limit, after_seq)
                .map_err(|err| collab_fail(&err, json))?;
            if json {
                print_json_or_debug(&records, true)
            } else {
                if records.is_empty() {
                    println!("activity: empty");
                }
                for r in &records {
                    println!(
                        "[{}] {} by {} -> {}",
                        r.seq,
                        r.event_kind.as_str(),
                        r.actor_participant_id.as_str(),
                        r.target_object_id.as_str()
                    );
                }
                Ok(())
            }
        }
    }
}

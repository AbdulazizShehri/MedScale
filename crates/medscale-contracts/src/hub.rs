//! MedScale Hub foundation contracts (Spec 084).
//!
//! A Hub is a MedScale vault whose Core accepts signed submissions from
//! enrolled devices of other vaults and applies them through its own Spec
//! 076 collaboration authority, running as the submitting device's
//! participant. Core stays the only authority on both sides: the Hub never
//! writes a collaboration row except through that path, and a client never
//! writes Hub state. A client keeps an outbox of its own signed submissions
//! and a read-only mirror of the Hub's ordered events; the mirror is a
//! projection, not a second collaboration store.
//!
//! Every request travels as an ordinary `AuthorityRequest`, so the same
//! bytes work in-process and over the Spec 024 local-socket IPC. There is no
//! network transport in this foundation.

use serde::{Deserialize, Serialize};

use crate::collaboration::TaskStatus;
use crate::objects::{AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId, VaultId};

/// Durable schema version for every Hub object (Spec 084 v1).
pub const HUB_SCHEMA_VERSION: u32 = 1;
/// Handshake protocol version this build speaks.
pub const HUB_PROTOCOL_VERSION: u32 = 1;

pub const DEVICE_NAME_MAX_CHARS: usize = 128;
/// Envelopes per submission and events per pull.
pub const HUB_BATCH_MAX: u32 = 100;
/// Hex lengths of an ed25519 public key, an ed25519 signature, a 32-byte
/// nonce and a 32-byte invitation token.
pub const PUBLIC_KEY_HEX_LEN: usize = 64;
pub const SIGNATURE_HEX_LEN: usize = 128;
pub const NONCE_HEX_LEN: usize = 64;
pub const TOKEN_HEX_LEN: usize = 64;
/// Upper bound on a local IPC endpoint name kept in a link.
pub const ENDPOINT_MAX_CHARS: usize = 128;

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

fn bounded_text(value: &str, max_chars: usize, what: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{what} must not be empty"));
    }
    if value.chars().count() > max_chars {
        return Err(format!("{what} exceeds {max_chars} characters"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{what} must not contain control characters"));
    }
    Ok(())
}

/// Checks a lowercase hex string of an exact length.
pub fn check_hex(value: &str, len: usize, what: &str) -> Result<(), String> {
    if value.len() != len
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(format!("{what} must be {len} lowercase hex characters"));
    }
    Ok(())
}

/// Checks a local IPC endpoint name: short, ASCII letters, digits, `-` and
/// `_` only (the Spec 024 host accepts the same set).
pub fn check_endpoint(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > ENDPOINT_MAX_CHARS
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err("endpoint must be a short local socket name".to_owned());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Hub side
// ---------------------------------------------------------------------------

/// The Hub role of one vault. `header.id` is the Hub id; realm and scope
/// are the tenant scope every Hub read and write is bound to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubIdentity {
    pub header: ObjectHeader,
    pub vault_id: VaultId,
    pub protocol_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    Open,
    Redeemed,
    Revoked,
}

closed_vocabulary!(InvitationStatus, "invitation status", {
    Open => "open",
    Redeemed => "redeemed",
    Revoked => "revoked",
});

/// A one-time enrollment invitation to one Project. Only the token's
/// digest is stored; the token itself is shown to the operator once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubInvitation {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub display_name: String,
    pub token_digest: DigestSha256,
    pub status: InvitationStatus,
    pub device_id: Option<OpaqueId>,
}

impl HubInvitation {
    pub fn validate(&self) -> Result<(), String> {
        bounded_text(&self.display_name, DEVICE_NAME_MAX_CHARS, "display name")?;
        if (self.status == InvitationStatus::Redeemed) != self.device_id.is_some() {
            return Err("only a redeemed invitation names its device".to_owned());
        }
        Ok(())
    }
}

/// What the Hub operator hands to the person enrolling a device: where the
/// Hub is and the one-time token. Shown once; only the token digest is
/// stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubInvitationCode {
    pub hub_id: OpaqueId,
    pub hub_vault_id: VaultId,
    pub hub_realm_id: RealmId,
    pub hub_scope_id: AuthorityScopeId,
    pub invitation_id: OpaqueId,
    pub token_hex: String,
}

impl HubInvitationCode {
    pub fn validate(&self) -> Result<(), String> {
        check_hex(&self.token_hex, TOKEN_HEX_LEN, "invitation token")
    }
}

/// The digest an invitation stores for its token.
#[must_use]
pub fn invitation_token_digest(hub_id: &OpaqueId, token_hex: &str) -> DigestSha256 {
    let mut bytes = b"medscale-hub-invitation-v1\0".to_vec();
    bytes.extend_from_slice(hub_id.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(token_hex.as_bytes());
    DigestSha256::of(&bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Active,
    Revoked,
}

closed_vocabulary!(DeviceStatus, "device status", {
    Active => "active",
    Revoked => "revoked",
});

/// An enrolled device: its public key, its Hub participant and the one
/// Project it may sync (its Project membership). `header.id` is the device
/// id; realm and scope are the Hub's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceIdentity {
    pub header: ObjectHeader,
    pub hub_id: OpaqueId,
    pub project_id: OpaqueId,
    pub participant_id: OpaqueId,
    /// The Spec 076 participant holder id the Hub runs this device's
    /// submissions as.
    pub holder_id: OpaqueId,
    pub display_name: String,
    pub public_key_hex: String,
    pub invitation_id: OpaqueId,
    pub status: DeviceStatus,
}

impl DeviceIdentity {
    pub fn validate(&self) -> Result<(), String> {
        bounded_text(&self.display_name, DEVICE_NAME_MAX_CHARS, "display name")?;
        check_hex(&self.public_key_hex, PUBLIC_KEY_HEX_LEN, "public key")
    }
}

/// Bytes a device signs to redeem an invitation (proves key possession).
#[must_use]
pub fn enrollment_payload(hub_id: &OpaqueId, token_hex: &str, public_key_hex: &str) -> Vec<u8> {
    let mut bytes = b"medscale-hub-enroll-v1\0".to_vec();
    for part in [hub_id.as_str(), token_hex, public_key_hex] {
        bytes.extend_from_slice(part.as_bytes());
        bytes.push(0);
    }
    bytes
}

/// A single-use handshake challenge the Hub issued to one device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubChallenge {
    pub hub_id: OpaqueId,
    pub device_id: OpaqueId,
    pub nonce_hex: String,
    pub protocol_version: u32,
}

/// A device's answer to a challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubHandshake {
    pub device_id: OpaqueId,
    pub nonce_hex: String,
    pub protocol_version: u32,
    pub signature_hex: String,
}

/// Bytes a device signs to answer a challenge.
#[must_use]
pub fn handshake_payload(
    hub_id: &OpaqueId,
    device_id: &OpaqueId,
    nonce_hex: &str,
    protocol_version: u32,
) -> Vec<u8> {
    let mut bytes = b"medscale-hub-handshake-v1\0".to_vec();
    for part in [hub_id.as_str(), device_id.as_str(), nonce_hex] {
        bytes.extend_from_slice(part.as_bytes());
        bytes.push(0);
    }
    bytes.extend_from_slice(&protocol_version.to_le_bytes());
    bytes
}

/// A live Hub session for one device (a Core session bound to the device's
/// participant holder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubSession {
    pub session_id: OpaqueId,
    pub hub_id: OpaqueId,
    pub device_id: OpaqueId,
    pub participant_id: OpaqueId,
    pub project_id: OpaqueId,
    /// The Project's latest event cursor at handshake time.
    pub head_cursor: u64,
}

// ---------------------------------------------------------------------------
// Envelopes and outcomes
// ---------------------------------------------------------------------------

/// One collaboration intent. The Hub applies it through the Spec 076
/// authority as the device's participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum SyncIntent {
    MessagePost {
        thread_id: OpaqueId,
        body: String,
    },
    TaskCreate {
        room_id: OpaqueId,
        title: String,
        description: Option<String>,
    },
    TaskUpdate {
        task_id: OpaqueId,
        expected_revision: u64,
        status: TaskStatus,
        assignee_participant_id: Option<OpaqueId>,
    },
    NoteCreate {
        room_id: OpaqueId,
        title: String,
        body: String,
    },
    NoteEdit {
        note_id: OpaqueId,
        expected_revision: u64,
        body: String,
    },
}

/// The signed part of an envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncEnvelopeBody {
    pub hub_id: OpaqueId,
    pub device_id: OpaqueId,
    /// Device-scoped, starting at 1, without gaps.
    pub seq: u64,
    pub intent: SyncIntent,
}

impl SyncEnvelopeBody {
    /// The exact bytes a device signs: a domain tag and the canonical JSON
    /// of this body (struct fields serialize in declaration order).
    pub fn signing_payload(&self) -> Result<Vec<u8>, String> {
        let mut bytes = b"medscale-hub-envelope-v1\0".to_vec();
        bytes.extend_from_slice(&serde_json::to_vec(self).map_err(|e| e.to_string())?);
        Ok(bytes)
    }
}

/// One signed submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncEnvelope {
    pub body: SyncEnvelopeBody,
    pub signature_hex: String,
}

impl SyncEnvelope {
    pub fn validate(&self) -> Result<(), String> {
        if self.body.seq == 0 {
            return Err("envelope sequence starts at 1".to_owned());
        }
        check_hex(&self.signature_hex, SIGNATURE_HEX_LEN, "signature")
    }

    /// Digest of the signed payload and the signature: two envelopes with
    /// the same digest are byte-identical submissions.
    pub fn digest(&self) -> Result<DigestSha256, String> {
        let mut bytes = self.body.signing_payload()?;
        bytes.extend_from_slice(self.signature_hex.as_bytes());
        Ok(DigestSha256::of(&bytes))
    }
}

/// Why the Hub refused a submission. Closed vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncRefusal {
    WrongHub,
    WrongDevice,
    BadSignature,
    DeviceRevoked,
    SequenceGap,
    SequenceReused,
    NotMember,
    WrongProject,
    NotFound,
    InvalidIntent,
}

closed_vocabulary!(SyncRefusal, "sync refusal", {
    WrongHub => "wrong_hub",
    WrongDevice => "wrong_device",
    BadSignature => "bad_signature",
    DeviceRevoked => "device_revoked",
    SequenceGap => "sequence_gap",
    SequenceReused => "sequence_reused",
    NotMember => "not_member",
    WrongProject => "wrong_project",
    NotFound => "not_found",
    InvalidIntent => "invalid_intent",
});

/// A task update at a stale revision (Q07: optimistic concurrency; never
/// last-writer-wins).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncConflict {
    pub object_id: OpaqueId,
    pub expected_revision: u64,
    pub current_revision: Option<u64>,
}

/// What the Hub did with one submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome", deny_unknown_fields)]
pub enum SyncOutcome {
    /// Applied; `object_id` is the Hub object written, `revision` its new
    /// revision.
    Applied {
        object_id: OpaqueId,
        revision: u64,
    },
    /// A stale note edit kept as a conflict copy on the Hub (Q07).
    ConflictCopy {
        note_id: OpaqueId,
        revision: u64,
    },
    Conflict {
        conflict: SyncConflict,
    },
    Refused {
        reason: SyncRefusal,
    },
}

// ---------------------------------------------------------------------------
// Ordered events and audit
// ---------------------------------------------------------------------------

/// What one Hub event records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum HubEventKind {
    /// Binds the device's public key into the chain, so a device row whose
    /// key was swapped disagrees with its enrollment event.
    DeviceEnrolled {
        device_id: OpaqueId,
        participant_id: OpaqueId,
        public_key_hex: String,
    },
    DeviceRevoked {
        device_id: OpaqueId,
    },
    Submission {
        envelope: SyncEnvelope,
        outcome: SyncOutcome,
    },
}

/// One entry of a Project's ordered, hash-chained Hub event log. The chain
/// digest at each cursor is the Hub audit checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubEvent {
    pub hub_id: OpaqueId,
    pub project_id: OpaqueId,
    /// 1-based, contiguous per Project.
    pub cursor: u64,
    pub kind: HubEventKind,
    pub checkpoint_digest: DigestSha256,
}

/// `Sha256(prev || domain || hub || project || cursor || kind json)`.
pub fn hub_event_digest(
    prev: Option<&DigestSha256>,
    hub_id: &OpaqueId,
    project_id: &OpaqueId,
    cursor: u64,
    kind: &HubEventKind,
) -> Result<DigestSha256, String> {
    let mut bytes = Vec::new();
    if let Some(prev) = prev {
        bytes.extend_from_slice(prev.as_bytes());
    }
    bytes.extend_from_slice(b"medscale-hub-event-v1\0");
    bytes.extend_from_slice(hub_id.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(project_id.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&cursor.to_le_bytes());
    bytes.extend_from_slice(&serde_json::to_vec(kind).map_err(|e| e.to_string())?);
    Ok(DigestSha256::of(&bytes))
}

impl HubEvent {
    /// Checks this event follows `prev` (the event at `cursor - 1`).
    pub fn validate_after(&self, prev: Option<&HubEvent>) -> Result<(), String> {
        let expected_cursor = prev.map_or(1, |p| p.cursor + 1);
        if self.cursor != expected_cursor {
            return Err(format!(
                "hub event cursor {} does not follow {}",
                self.cursor,
                expected_cursor - 1
            ));
        }
        if let Some(prev) = prev
            && (prev.hub_id != self.hub_id || prev.project_id != self.project_id)
        {
            return Err("hub event chain changes hub or project".to_owned());
        }
        if let HubEventKind::Submission { envelope, .. } = &self.kind {
            envelope.validate()?;
            if envelope.body.hub_id != self.hub_id {
                return Err("hub event names another hub's envelope".to_owned());
            }
        }
        let expected = hub_event_digest(
            prev.map(|p| &p.checkpoint_digest),
            &self.hub_id,
            &self.project_id,
            self.cursor,
            &self.kind,
        )?;
        if expected != self.checkpoint_digest {
            return Err(format!(
                "hub checkpoint mismatch at cursor {}: chain broken or tampered",
                self.cursor
            ));
        }
        Ok(())
    }
}

/// The verified head of a Project's Hub event chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubAuditCheckpoint {
    pub hub_id: OpaqueId,
    pub project_id: OpaqueId,
    pub cursor: u64,
    pub digest: Option<DigestSha256>,
}

/// A page of events after a cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubEventPage {
    pub events: Vec<HubEvent>,
    pub head: HubAuditCheckpoint,
}

/// Hub status for its operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubStatus {
    pub hub: HubIdentity,
    pub devices: Vec<DeviceIdentity>,
    pub invitations: Vec<HubInvitation>,
    pub checkpoints: Vec<HubAuditCheckpoint>,
}

// ---------------------------------------------------------------------------
// Client side
// ---------------------------------------------------------------------------

/// A client vault's link to one Hub. The device secret key is held by the
/// client's storage and never appears in this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubLink {
    pub header: ObjectHeader,
    pub endpoint: String,
    pub hub_id: OpaqueId,
    pub hub_vault_id: VaultId,
    pub hub_realm_id: RealmId,
    pub hub_scope_id: AuthorityScopeId,
    pub device_id: OpaqueId,
    pub participant_id: OpaqueId,
    pub project_id: OpaqueId,
    pub public_key_hex: String,
    /// Last sequence queued (0 before the first).
    pub last_seq: u64,
    /// Last Hub event mirrored (0 before the first).
    pub cursor: u64,
    pub head_digest: Option<DigestSha256>,
    pub revoked: bool,
}

impl HubLink {
    pub fn validate(&self) -> Result<(), String> {
        check_endpoint(&self.endpoint)?;
        check_hex(&self.public_key_hex, PUBLIC_KEY_HEX_LEN, "public key")?;
        if (self.cursor == 0) != self.head_digest.is_none() {
            return Err("a link names a head digest exactly when it has a cursor".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxState {
    Pending,
    Done,
}

closed_vocabulary!(OutboxState, "outbox state", {
    Pending => "pending",
    Done => "done",
});

/// One queued submission of a client and, once sent, the Hub's outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubOutboxEntry {
    pub link_id: OpaqueId,
    pub envelope: SyncEnvelope,
    pub state: OutboxState,
    pub outcome: Option<SyncOutcome>,
}

impl HubOutboxEntry {
    pub fn validate(&self) -> Result<(), String> {
        self.envelope.validate()?;
        if (self.state == OutboxState::Done) != self.outcome.is_some() {
            return Err("an outbox entry has an outcome exactly when it is done".to_owned());
        }
        Ok(())
    }
}

/// What one sync run did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncReport {
    pub link_id: OpaqueId,
    pub submitted: u32,
    pub applied: u32,
    pub conflicts: u32,
    pub refused: u32,
    pub pulled: u32,
    pub cursor: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: HUB_SCHEMA_VERSION,
            realm_id: RealmId::new("realm-1"),
            authority_scope_id: AuthorityScopeId::new("scope-1"),
        }
    }

    fn envelope(seq: u64) -> SyncEnvelope {
        SyncEnvelope {
            body: SyncEnvelopeBody {
                hub_id: OpaqueId::new("hub-1"),
                device_id: OpaqueId::new("device-1"),
                seq,
                intent: SyncIntent::MessagePost {
                    thread_id: OpaqueId::new("thread-1"),
                    body: "hello".to_owned(),
                },
            },
            signature_hex: "ab".repeat(64),
        }
    }

    fn chain(n: u64) -> Vec<HubEvent> {
        let mut out: Vec<HubEvent> = Vec::new();
        for cursor in 1..=n {
            let kind = HubEventKind::Submission {
                envelope: envelope(cursor),
                outcome: SyncOutcome::Applied {
                    object_id: OpaqueId::new(format!("message-{cursor}")),
                    revision: 1,
                },
            };
            let digest = hub_event_digest(
                out.last().map(|e| &e.checkpoint_digest),
                &OpaqueId::new("hub-1"),
                &OpaqueId::new("proj-1"),
                cursor,
                &kind,
            )
            .unwrap();
            out.push(HubEvent {
                hub_id: OpaqueId::new("hub-1"),
                project_id: OpaqueId::new("proj-1"),
                cursor,
                kind,
                checkpoint_digest: digest,
            });
        }
        out
    }

    #[test]
    fn vocabularies_round_trip_and_are_closed() {
        for v in SyncRefusal::ALL {
            assert_eq!(SyncRefusal::parse(v.as_str()).unwrap(), *v);
        }
        for v in InvitationStatus::ALL {
            assert_eq!(InvitationStatus::parse(v.as_str()).unwrap(), *v);
        }
        for v in DeviceStatus::ALL {
            assert_eq!(DeviceStatus::parse(v.as_str()).unwrap(), *v);
        }
        for v in OutboxState::ALL {
            assert_eq!(OutboxState::parse(v.as_str()).unwrap(), *v);
        }
        assert!(SyncRefusal::parse("maybe").is_err());
        let unknown = serde_json::json!({"kind": "room_delete", "room_id": "room-1"});
        assert!(serde_json::from_value::<SyncIntent>(unknown).is_err());
    }

    #[test]
    fn hex_and_endpoint_checks_are_strict() {
        assert!(check_hex(&"a".repeat(64), 64, "x").is_ok());
        assert!(check_hex(&"A".repeat(64), 64, "x").is_err());
        assert!(check_hex(&"g".repeat(64), 64, "x").is_err());
        assert!(check_hex(&"a".repeat(63), 64, "x").is_err());
        assert!(check_hex(&format!("{}\u{e9}", "a".repeat(62)), 64, "x").is_err());
        assert!(check_endpoint("medscale-hub_1").is_ok());
        assert!(check_endpoint("medscale.hub").is_err());
        assert!(check_endpoint("../hub").is_err());
        assert!(check_endpoint("").is_err());
        assert!(check_endpoint(&"a".repeat(ENDPOINT_MAX_CHARS + 1)).is_err());
    }

    #[test]
    fn signing_payloads_are_domain_separated_and_stable() {
        let e = envelope(1);
        let a = e.body.signing_payload().unwrap();
        assert_eq!(a, e.body.signing_payload().unwrap());
        assert!(a.starts_with(b"medscale-hub-envelope-v1\0"));
        let mut other = e.clone();
        other.body.seq = 2;
        assert_ne!(a, other.body.signing_payload().unwrap());
        let hub = OpaqueId::new("hub-1");
        let device = OpaqueId::new("device-1");
        assert_ne!(
            handshake_payload(&hub, &device, &"0".repeat(64), 1),
            handshake_payload(&hub, &device, &"0".repeat(64), 2)
        );
        assert_ne!(
            enrollment_payload(&hub, "ab", "cd"),
            enrollment_payload(&hub, "a", "bcd")
        );
        assert_ne!(
            invitation_token_digest(&hub, "ab"),
            invitation_token_digest(&OpaqueId::new("hub-2"), "ab")
        );
        assert_ne!(e.digest().unwrap(), other.digest().unwrap());
    }

    #[test]
    fn envelopes_entries_and_links_validate() {
        assert!(envelope(1).validate().is_ok());
        assert!(envelope(0).validate().is_err());
        let mut bad = envelope(1);
        bad.signature_hex = "zz".to_owned();
        assert!(bad.validate().is_err());
        let pending = HubOutboxEntry {
            link_id: OpaqueId::new("link-1"),
            envelope: envelope(1),
            state: OutboxState::Pending,
            outcome: None,
        };
        assert!(pending.validate().is_ok());
        let mut done_without_outcome = pending.clone();
        done_without_outcome.state = OutboxState::Done;
        assert!(done_without_outcome.validate().is_err());
        let invitation = HubInvitation {
            header: header("invite-1"),
            project_id: OpaqueId::new("proj-1"),
            display_name: "Lab laptop".to_owned(),
            token_digest: DigestSha256::of(b"t"),
            status: InvitationStatus::Redeemed,
            device_id: None,
        };
        assert!(invitation.validate().is_err());
        let link = HubLink {
            header: header("link-1"),
            endpoint: "hub-1".to_owned(),
            hub_id: OpaqueId::new("hub-1"),
            hub_vault_id: VaultId::new("vault-hub"),
            hub_realm_id: RealmId::new("realm-1"),
            hub_scope_id: AuthorityScopeId::new("scope-1"),
            device_id: OpaqueId::new("device-1"),
            participant_id: OpaqueId::new("participant-1"),
            project_id: OpaqueId::new("proj-1"),
            public_key_hex: "a".repeat(64),
            last_seq: 0,
            cursor: 1,
            head_digest: None,
            revoked: false,
        };
        assert!(link.validate().is_err());
    }

    #[test]
    fn event_chains_detect_edits_gaps_and_reorders() {
        let events = chain(3);
        let mut prev: Option<&HubEvent> = None;
        for e in &events {
            e.validate_after(prev).unwrap();
            prev = Some(e);
        }
        // Edited payload.
        let mut edited = events[1].clone();
        if let HubEventKind::Submission { envelope, .. } = &mut edited.kind {
            envelope.body.seq = 9;
        }
        assert!(edited.validate_after(Some(&events[0])).is_err());
        // Gap.
        assert!(events[2].validate_after(Some(&events[0])).is_err());
        // Reorder.
        assert!(events[1].validate_after(None).is_err());
        // Another hub's envelope.
        let mut foreign = events[0].clone();
        if let HubEventKind::Submission { envelope, .. } = &mut foreign.kind {
            envelope.body.hub_id = OpaqueId::new("hub-2");
        }
        assert!(foreign.validate_after(None).is_err());
    }
}

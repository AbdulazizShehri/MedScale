//! MedScale Hub foundation (Spec 084): Hub-side and client-side authority.
//!
//! The Hub applies a device's signed intents through the Spec 076
//! collaboration authority (`Collab`) running under that device's own Core
//! session, so the existing membership, scope and revision checks and the
//! hash-chained activity trail apply unchanged; the Hub adds enrollment,
//! signature, sequence and ordering checks around it and records every
//! authenticated submission in a per-Project hash-chained event log.
//!
//! The client side keeps its device secret inside its own vault: it signs
//! envelopes and handshakes here and never returns the secret.

use medscale_contracts::collaboration::ParticipantKind;
use medscale_contracts::envelopes::{AuthorityError, Capability};
use medscale_contracts::hub::{
    DeviceIdentity, DeviceStatus, HUB_BATCH_MAX, HUB_PROTOCOL_VERSION, HUB_SCHEMA_VERSION,
    HubChallenge, HubEvent, HubEventKind, HubEventPage, HubHandshake, HubIdentity, HubInvitation,
    HubInvitationCode, HubLink, HubOutboxEntry, HubSession, HubStatus, InvitationStatus,
    NONCE_HEX_LEN, OutboxState, PUBLIC_KEY_HEX_LEN, SIGNATURE_HEX_LEN, SyncConflict, SyncEnvelope,
    SyncEnvelopeBody, SyncIntent, SyncOutcome, SyncRefusal, TOKEN_HEX_LEN, check_endpoint,
    check_hex, enrollment_payload, handshake_payload, invitation_token_digest,
};
use medscale_contracts::objects::{AuthorityScopeId, ObjectHeader, OpaqueId, RealmId, VaultId};
use medscale_storage::{MetaError, SeqClaim, SqliteMetaStore};

use super::collaboration::Collab;
use super::store::InMemoryAuthorityStore;
use crate::process::{LeaseRegistry, SessionRegistry};

/// Ticks a device session lives (sessions are in memory; a restarted Hub
/// requires a new handshake).
pub const HUB_SESSION_TTL_TICKS: u64 = 10_000;

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

fn invalid(message: impl Into<String>) -> AuthorityError {
    AuthorityError::InvalidArgument {
        message: message.into(),
    }
}

fn hex32() -> String {
    medscale_contracts::objects::DigestSha256::from_bytes(medscale_keys::generate_key32()).to_hex()
}

/// Authenticated Hub-side view for one request.
pub struct Hub<'a> {
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

impl Hub<'_> {
    fn header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: HUB_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    fn collab(&mut self, session_id: Option<OpaqueId>) -> Collab<'_> {
        Collab {
            store: &mut *self.store,
            meta: self.meta,
            packs: self.packs,
            sessions: self.sessions,
            leases: self.leases,
            vault_id: self.vault_id,
            realm: self.realm.clone(),
            scope: self.scope.clone(),
            session_id,
        }
    }

    /// The Hub identity; `NotFound` before `init`, `WrongScope` when the
    /// request names another realm or scope (tenant scope before lookup).
    fn hub(&self) -> Result<HubIdentity, AuthorityError> {
        let hub = self
            .meta
            .get_hub_identity()
            .map_err(meta_err)?
            .ok_or(AuthorityError::NotFound)?;
        if hub.header.realm_id != self.realm || hub.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(hub)
    }

    fn scoped_project(&self, project_id: &OpaqueId) -> Result<(), AuthorityError> {
        let project = self.meta.get_project(project_id).map_err(meta_err)?;
        if project.header.realm_id != self.realm || project.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(())
    }

    /// Gives this vault its Hub role (once).
    pub fn init(&self) -> Result<HubIdentity, AuthorityError> {
        let hub = HubIdentity {
            header: self.header(OpaqueId::new(format!("hub-{}", &hex32()[..16]))),
            vault_id: self.vault_id.clone(),
            protocol_version: HUB_PROTOCOL_VERSION,
        };
        self.meta.insert_hub_identity(&hub).map_err(meta_err)?;
        Ok(hub)
    }

    /// Issues a one-time invitation to one Project.
    pub fn invite(
        &self,
        project_id: OpaqueId,
        display_name: String,
    ) -> Result<(HubInvitation, HubInvitationCode), AuthorityError> {
        let hub = self.hub()?;
        self.scoped_project(&project_id)?;
        let token_hex = hex32();
        let id = self
            .meta
            .alloc_collab_id("hub-invitation", "invitation")
            .map_err(meta_err)?;
        let invitation = HubInvitation {
            header: self.header(id.clone()),
            project_id,
            display_name,
            token_digest: invitation_token_digest(&hub.header.id, &token_hex),
            status: InvitationStatus::Open,
            device_id: None,
        };
        invitation.validate().map_err(invalid)?;
        self.meta.insert_invitation(&invitation).map_err(meta_err)?;
        let code = HubInvitationCode {
            hub_id: hub.header.id,
            hub_vault_id: hub.vault_id,
            hub_realm_id: hub.header.realm_id,
            hub_scope_id: hub.header.authority_scope_id,
            invitation_id: id,
            token_hex,
        };
        Ok((invitation, code))
    }

    pub fn revoke_invitation(&self, id: &OpaqueId) -> Result<HubInvitation, AuthorityError> {
        self.hub()?;
        let inv = self.meta.get_invitation(id).map_err(meta_err)?;
        if inv.header.realm_id != self.realm || inv.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        self.meta.revoke_invitation(id).map_err(meta_err)
    }

    /// Redeems an invitation for a device key. The signature over the
    /// enrollment payload proves the caller holds the key's secret.
    pub fn enroll(
        &mut self,
        token_hex: &str,
        public_key_hex: &str,
        signature_hex: &str,
    ) -> Result<DeviceIdentity, AuthorityError> {
        let hub = self.hub()?;
        check_hex(token_hex, TOKEN_HEX_LEN, "invitation token").map_err(invalid)?;
        check_hex(public_key_hex, PUBLIC_KEY_HEX_LEN, "public key").map_err(invalid)?;
        check_hex(signature_hex, SIGNATURE_HEX_LEN, "signature").map_err(invalid)?;
        let digest = invitation_token_digest(&hub.header.id, token_hex);
        // An unknown, used or revoked token is indistinguishable to the
        // caller.
        let invitation = self
            .meta
            .get_invitation_by_token_digest(&digest)
            .map_err(meta_err)?
            .filter(|i| i.status == InvitationStatus::Open)
            .ok_or(AuthorityError::Unauthorized)?;
        medscale_keys::verify_device_signature(
            public_key_hex,
            &enrollment_payload(&hub.header.id, token_hex, public_key_hex),
            signature_hex,
        )
        .map_err(|_| AuthorityError::Unauthorized)?;
        let device_id = self
            .meta
            .alloc_collab_id("hub-device", "device")
            .map_err(meta_err)?;
        let holder_id = OpaqueId::new(format!("hub-{}", device_id.as_str()));
        let participant = self.collab(None).register_participant(
            holder_id.clone(),
            ParticipantKind::Human,
            invitation.display_name.clone(),
            None,
        )?;
        let device = DeviceIdentity {
            header: self.header(device_id),
            hub_id: hub.header.id,
            project_id: invitation.project_id.clone(),
            participant_id: participant.header.id,
            holder_id,
            display_name: invitation.display_name.clone(),
            public_key_hex: public_key_hex.to_owned(),
            invitation_id: invitation.header.id.clone(),
            status: DeviceStatus::Active,
        };
        self.meta
            .enroll_device(&invitation.header.id, &device)
            .map_err(meta_err)?;
        Ok(device)
    }

    fn scoped_device(&self, id: &OpaqueId) -> Result<DeviceIdentity, AuthorityError> {
        let device = self.meta.get_device(id).map_err(meta_err)?;
        if device.header.realm_id != self.realm || device.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(device)
    }

    /// Issues a single-use nonce to an active device.
    pub fn challenge(&self, device_id: &OpaqueId) -> Result<HubChallenge, AuthorityError> {
        let hub = self.hub()?;
        let device = self
            .scoped_device(device_id)
            .map_err(|_| AuthorityError::Unauthorized)?;
        if device.status != DeviceStatus::Active {
            return Err(AuthorityError::Unauthorized);
        }
        let nonce_hex = hex32();
        self.meta
            .insert_hub_nonce(device_id, &nonce_hex)
            .map_err(meta_err)?;
        Ok(HubChallenge {
            hub_id: hub.header.id,
            device_id: device_id.clone(),
            nonce_hex,
            protocol_version: HUB_PROTOCOL_VERSION,
        })
    }

    /// Answers a challenge with a device session. The nonce is consumed
    /// before the signature is checked, so a failed attempt cannot be
    /// retried with the same nonce.
    pub fn handshake(&self, hs: &HubHandshake) -> Result<HubSession, AuthorityError> {
        let hub = self.hub()?;
        if hs.protocol_version != HUB_PROTOCOL_VERSION {
            return Err(AuthorityError::UnsupportedSchema {
                message: format!(
                    "hub protocol {} is not {HUB_PROTOCOL_VERSION}",
                    hs.protocol_version
                ),
            });
        }
        check_hex(&hs.nonce_hex, NONCE_HEX_LEN, "nonce").map_err(invalid)?;
        let device = self
            .scoped_device(&hs.device_id)
            .map_err(|_| AuthorityError::Unauthorized)?;
        if !self
            .meta
            .take_hub_nonce(&hs.device_id, &hs.nonce_hex)
            .map_err(meta_err)?
            || device.status != DeviceStatus::Active
        {
            return Err(AuthorityError::Unauthorized);
        }
        medscale_keys::verify_device_signature(
            &device.public_key_hex,
            &handshake_payload(
                &hub.header.id,
                &device.header.id,
                &hs.nonce_hex,
                hs.protocol_version,
            ),
            &hs.signature_hex,
        )
        .map_err(|_| AuthorityError::Unauthorized)?;
        let (session_id, _) = self.sessions.open(
            self.vault_id,
            device.holder_id.clone(),
            vec![Capability::HubSync],
            HUB_SESSION_TTL_TICKS,
        );
        let head = self
            .meta
            .hub_head(&hub.header.id, &device.project_id)
            .map_err(meta_err)?;
        Ok(HubSession {
            session_id,
            hub_id: hub.header.id,
            device_id: device.header.id,
            participant_id: device.participant_id,
            project_id: device.project_id,
            head_cursor: head.cursor,
        })
    }

    /// The active device behind this request's session.
    fn session_device(&self) -> Result<(HubIdentity, DeviceIdentity), AuthorityError> {
        let hub = self.hub()?;
        let holder = self
            .session_id
            .as_ref()
            .and_then(|s| self.sessions.holder_of(s))
            .ok_or(AuthorityError::SessionRequired)?;
        let device = self
            .meta
            .get_device_by_holder(&holder)
            .map_err(meta_err)?
            .ok_or(AuthorityError::Unauthorized)?;
        if device.header.realm_id != self.realm || device.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok((hub, device))
    }

    /// The Project an intent's room belongs to.
    fn intent_project(&self, intent: &SyncIntent) -> Result<OpaqueId, AuthorityError> {
        let room_id = match intent {
            SyncIntent::MessagePost { thread_id, .. } => {
                self.meta.get_thread(thread_id).map_err(meta_err)?.room_id
            }
            SyncIntent::TaskCreate { room_id, .. } | SyncIntent::NoteCreate { room_id, .. } => {
                room_id.clone()
            }
            SyncIntent::TaskUpdate { task_id, .. } => {
                self.meta.get_task(task_id).map_err(meta_err)?.room_id
            }
            SyncIntent::NoteEdit { note_id, .. } => {
                self.meta.get_note(note_id).map_err(meta_err)?.room_id
            }
        };
        let room = self.meta.get_room(&room_id).map_err(meta_err)?;
        if room.header.realm_id != self.realm || room.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(room.project_id)
    }

    /// Applies one intent through the Spec 076 authority as the device.
    fn apply(
        &mut self,
        device: &DeviceIdentity,
        intent: SyncIntent,
    ) -> Result<SyncOutcome, AuthorityError> {
        let refused = |reason| Ok(SyncOutcome::Refused { reason });
        match self.intent_project(&intent) {
            Ok(project) if project == device.project_id => {}
            Ok(_) | Err(AuthorityError::WrongScope) => return refused(SyncRefusal::WrongProject),
            Err(AuthorityError::NotFound) => return refused(SyncRefusal::NotFound),
            Err(other) => return Err(other),
        }
        let session = self.session_id.clone();
        let collab = self.collab(session);
        let result = match &intent {
            SyncIntent::MessagePost { thread_id, body } => collab
                .post_message(thread_id, body.clone())
                .map(|m| SyncOutcome::Applied {
                    object_id: m.header.id,
                    revision: 1,
                }),
            SyncIntent::TaskCreate {
                room_id,
                title,
                description,
            } => collab
                .create_task(room_id.clone(), None, title.clone(), description.clone())
                .map(|t| SyncOutcome::Applied {
                    object_id: t.header.id,
                    revision: t.revision,
                }),
            SyncIntent::TaskUpdate {
                task_id,
                expected_revision,
                status,
                assignee_participant_id,
            } => collab
                .update_task(
                    task_id,
                    *expected_revision,
                    *status,
                    assignee_participant_id.clone(),
                )
                .map(|t| SyncOutcome::Applied {
                    object_id: t.header.id,
                    revision: t.revision,
                }),
            SyncIntent::NoteCreate {
                room_id,
                title,
                body,
            } => collab
                .create_note(room_id.clone(), title.clone(), body.clone())
                .map(|n| SyncOutcome::Applied {
                    object_id: n.header.id,
                    revision: n.revision,
                }),
            SyncIntent::NoteEdit {
                note_id,
                expected_revision,
                body,
            } => collab
                .edit_note(note_id, *expected_revision, body.clone())
                .map(|(note, rev, copy)| {
                    if copy {
                        SyncOutcome::ConflictCopy {
                            note_id: note.header.id,
                            revision: rev.revision,
                        }
                    } else {
                        SyncOutcome::Applied {
                            object_id: note.header.id,
                            revision: note.revision,
                        }
                    }
                }),
        };
        match result {
            Ok(outcome) => Ok(outcome),
            Err(AuthorityError::Conflict { .. }) => {
                if let SyncIntent::TaskUpdate {
                    task_id,
                    expected_revision,
                    ..
                } = &intent
                {
                    let current = self.meta.get_task(task_id).ok().map(|t| t.revision);
                    Ok(SyncOutcome::Conflict {
                        conflict: SyncConflict {
                            object_id: task_id.clone(),
                            expected_revision: *expected_revision,
                            current_revision: current,
                        },
                    })
                } else {
                    refused(SyncRefusal::InvalidIntent)
                }
            }
            Err(AuthorityError::Unauthorized) => refused(SyncRefusal::NotMember),
            Err(AuthorityError::WrongScope) => refused(SyncRefusal::WrongProject),
            Err(AuthorityError::NotFound) => refused(SyncRefusal::NotFound),
            Err(AuthorityError::InvalidArgument { .. }) => refused(SyncRefusal::InvalidIntent),
            Err(other) => Err(other),
        }
    }

    fn record(
        &self,
        hub: &HubIdentity,
        device: &DeviceIdentity,
        envelope: SyncEnvelope,
        outcome: SyncOutcome,
        claim: Option<SeqClaim>,
    ) -> Result<SyncOutcome, AuthorityError> {
        self.meta
            .append_hub_event(
                &hub.header.id,
                &device.project_id,
                HubEventKind::Submission {
                    envelope,
                    outcome: outcome.clone(),
                },
                claim.as_ref(),
            )
            .map_err(meta_err)?;
        Ok(outcome)
    }

    /// Applies a batch of envelopes in order. Every authenticated envelope
    /// is recorded as an event; a byte-identical resubmission returns its
    /// stored outcome and records nothing.
    pub fn submit(
        &mut self,
        envelopes: Vec<SyncEnvelope>,
    ) -> Result<Vec<SyncOutcome>, AuthorityError> {
        if envelopes.len() > HUB_BATCH_MAX as usize {
            return Err(invalid(format!(
                "at most {HUB_BATCH_MAX} envelopes per submission"
            )));
        }
        let (hub, device) = self.session_device()?;
        let mut outcomes = Vec::with_capacity(envelopes.len());
        for envelope in envelopes {
            outcomes.push(self.submit_one(&hub, &device, envelope)?);
        }
        Ok(outcomes)
    }

    fn submit_one(
        &mut self,
        hub: &HubIdentity,
        device: &DeviceIdentity,
        envelope: SyncEnvelope,
    ) -> Result<SyncOutcome, AuthorityError> {
        let refused = |reason| SyncOutcome::Refused { reason };
        // Not attributable to this Hub or device: answered, never recorded.
        if envelope.validate().is_err() {
            return Ok(refused(SyncRefusal::InvalidIntent));
        }
        if envelope.body.hub_id != hub.header.id {
            return Ok(refused(SyncRefusal::WrongHub));
        }
        if envelope.body.device_id != device.header.id {
            return Ok(refused(SyncRefusal::WrongDevice));
        }
        let payload = envelope.body.signing_payload().map_err(invalid)?;
        if medscale_keys::verify_device_signature(
            &device.public_key_hex,
            &payload,
            &envelope.signature_hex,
        )
        .is_err()
        {
            return self.record(
                hub,
                device,
                envelope,
                refused(SyncRefusal::BadSignature),
                None,
            );
        }
        // Re-read: a revocation between handshake and submission wins.
        let current = self.meta.get_device(&device.header.id).map_err(meta_err)?;
        if current.status != DeviceStatus::Active {
            return self.record(
                hub,
                device,
                envelope,
                refused(SyncRefusal::DeviceRevoked),
                None,
            );
        }
        let digest = envelope.digest().map_err(invalid)?;
        let seq = envelope.body.seq;
        let last = self
            .meta
            .last_claimed_seq(&device.header.id)
            .map_err(meta_err)?;
        if seq <= last {
            let prior = self
                .meta
                .find_claimed_event(&device.header.id, seq)
                .map_err(meta_err)?;
            if let Some(HubEvent {
                kind:
                    HubEventKind::Submission {
                        envelope: stored,
                        outcome,
                    },
                ..
            }) = prior
                && stored.digest().map_err(invalid)? == digest
            {
                return Ok(outcome);
            }
            return self.record(
                hub,
                device,
                envelope,
                refused(SyncRefusal::SequenceReused),
                None,
            );
        }
        if seq != last + 1 {
            return self.record(
                hub,
                device,
                envelope,
                refused(SyncRefusal::SequenceGap),
                None,
            );
        }
        let outcome = self.apply(device, envelope.body.intent.clone())?;
        let claim = SeqClaim {
            device_id: device.header.id.clone(),
            seq,
            envelope_digest: digest,
        };
        self.record(hub, device, envelope, outcome, Some(claim))
    }

    /// Events of the device's Project after `after`.
    pub fn pull(&self, after: u64, limit: u32) -> Result<HubEventPage, AuthorityError> {
        let (hub, device) = self.session_device()?;
        if limit == 0 || limit > HUB_BATCH_MAX {
            return Err(invalid(format!("limit must be 1..={HUB_BATCH_MAX}")));
        }
        let head = self
            .meta
            .hub_head(&hub.header.id, &device.project_id)
            .map_err(meta_err)?;
        if after > head.cursor {
            return Err(AuthorityError::Conflict {
                message: "the cursor is ahead of the Hub's history".to_owned(),
            });
        }
        let events = self
            .meta
            .list_hub_events(&device.project_id, after, limit)
            .map_err(meta_err)?;
        Ok(HubEventPage { events, head })
    }

    /// Revokes a device: storage status and event, its Hub participant, and
    /// its live sessions.
    pub fn revoke_device(
        &mut self,
        device_id: &OpaqueId,
    ) -> Result<DeviceIdentity, AuthorityError> {
        self.hub()?;
        self.scoped_device(device_id)?;
        let (device, _) = self.meta.revoke_device(device_id).map_err(meta_err)?;
        let participant = self
            .meta
            .get_participant(&device.participant_id)
            .map_err(meta_err)?;
        if participant.status == medscale_contracts::collaboration::ParticipantStatus::Active {
            self.collab(None)
                .revoke_participant(&device.participant_id, participant.revision)?;
        }
        self.sessions.revoke_holder(&device.holder_id);
        Ok(device)
    }

    /// Hub status for its operator; every chain is verified.
    pub fn status(&self) -> Result<HubStatus, AuthorityError> {
        let hub = self.hub()?;
        let mut checkpoints = Vec::new();
        for project in self.meta.list_hub_event_projects().map_err(meta_err)? {
            checkpoints.push(
                self.meta
                    .verify_hub_chain(&hub.header.id, &project)
                    .map_err(meta_err)?,
            );
        }
        Ok(HubStatus {
            hub,
            devices: self.meta.list_devices().map_err(meta_err)?,
            invitations: self.meta.list_invitations().map_err(meta_err)?,
            checkpoints,
        })
    }
}

/// Client-side view for one request (the client vault's own authority).
pub struct HubClient<'a> {
    pub meta: &'a SqliteMetaStore,
    pub realm: RealmId,
    pub scope: AuthorityScopeId,
}

/// A prepared join: the new link id and the enrollment proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinPrepared {
    pub link_id: OpaqueId,
    pub public_key_hex: String,
    pub signature_hex: String,
}

impl HubClient<'_> {
    fn scoped_link(&self, id: &OpaqueId) -> Result<HubLink, AuthorityError> {
        let link = self.meta.get_hub_link(id).map_err(meta_err)?;
        if link.header.realm_id != self.realm || link.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(link)
    }

    fn secret(&self, link_id: &OpaqueId) -> Result<String, AuthorityError> {
        self.meta
            .get_hub_link_secret(link_id)
            .map_err(meta_err)?
            .ok_or_else(|| AuthorityError::Unavailable {
                message: "the device key is not in this vault (re-enroll after a restore)"
                    .to_owned(),
            })
    }

    /// Generates a device key, stores its secret, and signs the enrollment
    /// payload for `code`.
    pub fn join_prepare(&self, code: &HubInvitationCode) -> Result<JoinPrepared, AuthorityError> {
        code.validate().map_err(invalid)?;
        let link_id = self
            .meta
            .alloc_collab_id("hub-link", "link")
            .map_err(meta_err)?;
        let (secret, public) = medscale_keys::generate_device_key();
        let signature = medscale_keys::sign_device_payload(
            &secret,
            &enrollment_payload(&code.hub_id, &code.token_hex, &public),
        )
        .map_err(|e| AuthorityError::Internal {
            message: e.to_string(),
        })?;
        self.meta
            .insert_pending_hub_secret(&link_id, &secret)
            .map_err(meta_err)?;
        Ok(JoinPrepared {
            link_id,
            public_key_hex: public,
            signature_hex: signature,
        })
    }

    /// Records the link once the Hub enrolled the device; the enrolled key
    /// must be the one this vault generated.
    pub fn join_complete(
        &self,
        link_id: &OpaqueId,
        endpoint: String,
        code: &HubInvitationCode,
        device: &DeviceIdentity,
    ) -> Result<HubLink, AuthorityError> {
        check_endpoint(&endpoint).map_err(invalid)?;
        let secret = self.secret(link_id)?;
        let public =
            medscale_keys::device_public_key(&secret).map_err(|e| AuthorityError::Corrupt {
                message: e.to_string(),
            })?;
        if public != device.public_key_hex
            || device.hub_id != code.hub_id
            || device.invitation_id != code.invitation_id
        {
            return Err(AuthorityError::Conflict {
                message: "the enrolled device does not match this join".to_owned(),
            });
        }
        let link = HubLink {
            header: ObjectHeader {
                id: link_id.clone(),
                schema_version: HUB_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            endpoint,
            hub_id: code.hub_id.clone(),
            hub_vault_id: code.hub_vault_id.clone(),
            hub_realm_id: code.hub_realm_id.clone(),
            hub_scope_id: code.hub_scope_id.clone(),
            device_id: device.header.id.clone(),
            participant_id: device.participant_id.clone(),
            project_id: device.project_id.clone(),
            public_key_hex: public,
            last_seq: 0,
            cursor: 0,
            head_digest: None,
            revoked: false,
        };
        self.meta.insert_hub_link(&link).map_err(meta_err)?;
        Ok(link)
    }

    pub fn link(&self, id: &OpaqueId) -> Result<HubLink, AuthorityError> {
        self.scoped_link(id)
    }

    pub fn links(&self) -> Result<Vec<HubLink>, AuthorityError> {
        Ok(self
            .meta
            .list_hub_links()
            .map_err(meta_err)?
            .into_iter()
            .filter(|l| {
                l.header.realm_id == self.realm && l.header.authority_scope_id == self.scope
            })
            .collect())
    }

    /// Signs and queues one intent at the link's next sequence (offline).
    pub fn queue(
        &self,
        link_id: &OpaqueId,
        intent: SyncIntent,
    ) -> Result<HubOutboxEntry, AuthorityError> {
        let link = self.scoped_link(link_id)?;
        if link.revoked {
            return Err(AuthorityError::Unauthorized);
        }
        let secret = self.secret(link_id)?;
        let body = SyncEnvelopeBody {
            hub_id: link.hub_id.clone(),
            device_id: link.device_id.clone(),
            seq: link.last_seq + 1,
            intent,
        };
        let signature_hex =
            medscale_keys::sign_device_payload(&secret, &body.signing_payload().map_err(invalid)?)
                .map_err(|e| AuthorityError::Corrupt {
                    message: e.to_string(),
                })?;
        let entry = HubOutboxEntry {
            link_id: link_id.clone(),
            envelope: SyncEnvelope {
                body,
                signature_hex,
            },
            state: OutboxState::Pending,
            outcome: None,
        };
        self.meta.queue_hub_outbox(&entry).map_err(meta_err)?;
        Ok(entry)
    }

    /// Answers a Hub challenge for this link's device.
    pub fn sign_handshake(
        &self,
        link_id: &OpaqueId,
        challenge: &HubChallenge,
    ) -> Result<HubHandshake, AuthorityError> {
        let link = self.scoped_link(link_id)?;
        if challenge.hub_id != link.hub_id || challenge.device_id != link.device_id {
            return Err(AuthorityError::Conflict {
                message: "the challenge is for another hub or device".to_owned(),
            });
        }
        if challenge.protocol_version != HUB_PROTOCOL_VERSION {
            return Err(AuthorityError::UnsupportedSchema {
                message: "unsupported hub protocol".to_owned(),
            });
        }
        check_hex(&challenge.nonce_hex, NONCE_HEX_LEN, "nonce").map_err(invalid)?;
        let secret = self.secret(link_id)?;
        let signature_hex = medscale_keys::sign_device_payload(
            &secret,
            &handshake_payload(
                &link.hub_id,
                &link.device_id,
                &challenge.nonce_hex,
                challenge.protocol_version,
            ),
        )
        .map_err(|e| AuthorityError::Corrupt {
            message: e.to_string(),
        })?;
        Ok(HubHandshake {
            device_id: link.device_id,
            nonce_hex: challenge.nonce_hex.clone(),
            protocol_version: challenge.protocol_version,
            signature_hex,
        })
    }

    pub fn outbox(
        &self,
        link_id: &OpaqueId,
        pending_only: bool,
    ) -> Result<Vec<HubOutboxEntry>, AuthorityError> {
        self.scoped_link(link_id)?;
        self.meta
            .list_hub_outbox(link_id, pending_only)
            .map_err(meta_err)
    }

    pub fn record_outcomes(
        &self,
        link_id: &OpaqueId,
        outcomes: &[(u64, SyncOutcome)],
    ) -> Result<(), AuthorityError> {
        self.scoped_link(link_id)?;
        for (seq, outcome) in outcomes {
            self.meta
                .record_hub_outcome(link_id, *seq, outcome)
                .map_err(meta_err)?;
        }
        Ok(())
    }

    /// Mirrors a pulled page; a Hub whose head is behind this link's cursor
    /// has lost or rewritten history and is refused.
    pub fn mirror_append(
        &self,
        link_id: &OpaqueId,
        page: &HubEventPage,
    ) -> Result<HubLink, AuthorityError> {
        let link = self.scoped_link(link_id)?;
        if page.head.hub_id != link.hub_id
            || page.head.project_id != link.project_id
            || page.head.cursor < link.cursor
        {
            return Err(AuthorityError::Conflict {
                message: "the Hub's head is behind this link".to_owned(),
            });
        }
        self.meta
            .append_hub_mirror(link_id, &page.events)
            .map_err(meta_err)
    }

    pub fn mirror(
        &self,
        link_id: &OpaqueId,
        after: u64,
        limit: u32,
    ) -> Result<Vec<HubEvent>, AuthorityError> {
        self.scoped_link(link_id)?;
        if limit == 0 || limit > HUB_BATCH_MAX {
            return Err(invalid(format!("limit must be 1..={HUB_BATCH_MAX}")));
        }
        self.meta
            .list_hub_mirror(link_id, after, limit)
            .map_err(meta_err)
    }
}

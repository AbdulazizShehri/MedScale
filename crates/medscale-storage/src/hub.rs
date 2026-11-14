//! MedScale Hub durable rows (Spec 084, storage schema v13).
//!
//! Same pattern as Specs 079-083: each row keeps the validated contract
//! value as JSON (`body_json`) plus the columns queries need; reads re-check
//! columns against the body and fail closed on disagreement.
//!
//! Hub side: the Hub identity, invitations (token digests only), enrolled
//! devices (public keys only), single-use handshake nonces, and a per-Project
//! hash-chained event log. An event that claims a device sequence carries
//! `(device_id, seq)` columns with a unique index: that index is the
//! idempotency and replay record.
//!
//! Client side: links to a Hub, the device secret of each link in its own
//! table (never exported to backups), the outbox of signed submissions, and
//! the mirror of Hub events, whose chain is re-verified on every append.

use std::collections::HashMap;

use medscale_contracts::hub::{
    DeviceIdentity, DeviceStatus, HubAuditCheckpoint, HubEvent, HubEventKind, HubIdentity,
    HubInvitation, HubLink, HubOutboxEntry, InvitationStatus, OutboxState, SyncOutcome,
    hub_event_digest,
};
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v13 DDL, executed inside `begin/finish_migration(13)`.
pub(crate) const V13_DDL: &str = r"
CREATE TABLE IF NOT EXISTS hub_identity (
  hub_id TEXT PRIMARY KEY,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hub_invitations (
  invitation_id TEXT PRIMARY KEY,
  token_digest TEXT NOT NULL UNIQUE,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hub_devices (
  device_id TEXT PRIMARY KEY,
  holder_id TEXT NOT NULL UNIQUE,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hub_nonces (
  nonce_hex TEXT PRIMARY KEY,
  device_id TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hub_events (
  project_id TEXT NOT NULL,
  cursor INTEGER NOT NULL,
  device_id TEXT,
  seq INTEGER,
  envelope_digest TEXT,
  body_json TEXT NOT NULL,
  PRIMARY KEY (project_id, cursor),
  UNIQUE (device_id, seq)
);
CREATE TABLE IF NOT EXISTS hub_links (
  link_id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hub_link_secrets (
  link_id TEXT PRIMARY KEY,
  secret_hex TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hub_outbox (
  link_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  state TEXT NOT NULL,
  body_json TEXT NOT NULL,
  PRIMARY KEY (link_id, seq)
);
CREATE TABLE IF NOT EXISTS hub_mirror (
  link_id TEXT NOT NULL,
  cursor INTEGER NOT NULL,
  body_json TEXT NOT NULL,
  PRIMARY KEY (link_id, cursor)
);
";

/// A device sequence claim recorded with an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeqClaim {
    pub device_id: OpaqueId,
    pub seq: u64,
    pub envelope_digest: DigestSha256,
}

fn corrupt(message: String) -> MetaError {
    MetaError::CorruptObjectBody(message)
}

fn map_insert(result: rusqlite::Result<usize>, what: &str, id: &str) -> Result<(), MetaError> {
    match result {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(f, _))
            if f.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(MetaError::Conflict(format!("duplicate {what} {id}")))
        }
        Err(e) => Err(MetaError::Sqlite(e)),
    }
}

fn restore_conflict_is_corrupt(result: Result<(), MetaError>) -> Result<(), MetaError> {
    match result {
        Err(MetaError::Conflict(message)) => Err(corrupt(format!("tampered backup: {message}"))),
        Err(MetaError::UnsupportedSchema(message)) => Err(corrupt(message)),
        other => other,
    }
}

fn to_json<T: Serialize>(value: &T) -> Result<String, MetaError> {
    serde_json::to_string(value).map_err(|e| corrupt(e.to_string()))
}

fn from_json<T: DeserializeOwned>(json: &str, what: &str) -> Result<T, MetaError> {
    serde_json::from_str(json).map_err(|e| corrupt(format!("{what}: {e}")))
}

fn check_column(column: &str, body: &str, what: &str) -> Result<(), MetaError> {
    if column == body {
        Ok(())
    } else {
        Err(corrupt(format!(
            "{what} row columns disagree with its body"
        )))
    }
}

fn to_i64(value: u64, what: &str) -> Result<i64, MetaError> {
    i64::try_from(value).map_err(|_| MetaError::UnsupportedSchema(format!("{what} out of range")))
}

fn to_u64(value: i64, what: &str) -> Result<u64, MetaError> {
    u64::try_from(value).map_err(|_| corrupt(format!("{what} out of range")))
}

fn invalid(what: &str, e: String) -> MetaError {
    MetaError::UnsupportedSchema(format!("{what}: {e}"))
}

/// The sequence claim an event carries, if any: a submission whose outcome
/// is not a refusal, or a refusal the Hub made after verifying the device's
/// signature at the next sequence (`not_member`, `wrong_project`,
/// `not_found`, `invalid_intent`).
#[must_use]
pub fn event_claims_seq(event: &HubEvent) -> Option<(OpaqueId, u64)> {
    use medscale_contracts::hub::SyncRefusal;
    let HubEventKind::Submission { envelope, outcome } = &event.kind else {
        return None;
    };
    let claims = match outcome {
        SyncOutcome::Refused { reason } => matches!(
            reason,
            SyncRefusal::NotMember
                | SyncRefusal::WrongProject
                | SyncRefusal::NotFound
                | SyncRefusal::InvalidIntent
        ),
        _ => true,
    };
    claims.then(|| (envelope.body.device_id.clone(), envelope.body.seq))
}

fn decode_event(row: &rusqlite::Row<'_>) -> Result<HubEvent, MetaError> {
    let project_id: String = row.get(0)?;
    let cursor: i64 = row.get(1)?;
    let device_id: Option<String> = row.get(2)?;
    let seq: Option<i64> = row.get(3)?;
    let digest: Option<String> = row.get(4)?;
    let body: String = row.get(5)?;
    let event: HubEvent = from_json(&body, "hub event")?;
    check_column(&project_id, event.project_id.as_str(), "hub event")?;
    if to_u64(cursor, "hub event cursor")? != event.cursor {
        return Err(corrupt(
            "hub event row columns disagree with its body".to_owned(),
        ));
    }
    let claim = event_claims_seq(&event);
    let columns = match (device_id, seq) {
        (Some(d), Some(s)) => Some((d, to_u64(s, "hub event seq")?)),
        (None, None) => None,
        _ => return Err(corrupt("hub event claim columns are partial".to_owned())),
    };
    let expected = claim.as_ref().map(|(d, s)| (d.as_str().to_owned(), *s));
    if columns != expected {
        return Err(corrupt(
            "hub event claim columns disagree with its body".to_owned(),
        ));
    }
    let expected_digest = match (&claim, &event.kind) {
        (Some(_), HubEventKind::Submission { envelope, .. }) => {
            Some(envelope.digest().map_err(corrupt)?.to_hex())
        }
        _ => None,
    };
    if digest != expected_digest {
        return Err(corrupt(
            "hub event digest column disagrees with its body".to_owned(),
        ));
    }
    Ok(event)
}

const EVENT_COLUMNS: &str = "project_id, cursor, device_id, seq, envelope_digest, body_json";

impl SqliteMetaStore {
    fn hub_rows<T>(
        &self,
        sql: &str,
        args: &[&dyn rusqlite::ToSql],
        decode: fn(&rusqlite::Row<'_>) -> Result<T, MetaError>,
    ) -> Result<Vec<T>, MetaError> {
        let mut stmt = self.conn().prepare(sql)?;
        let mut rows = stmt.query(args)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(decode(row)?);
        }
        Ok(out)
    }

    fn hub_bodies<T: DeserializeOwned>(
        &self,
        sql: &str,
        args: &[&dyn rusqlite::ToSql],
        what: &str,
    ) -> Result<Vec<(String, T)>, MetaError> {
        let mut stmt = self.conn().prepare(sql)?;
        let mut rows = stmt.query(args)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let body: String = row.get(1)?;
            out.push((key, from_json(&body, what)?));
        }
        Ok(out)
    }

    // ----- Hub identity -----

    fn hub_insert_identity_on(
        conn: &rusqlite::Connection,
        hub: &HubIdentity,
    ) -> Result<(), MetaError> {
        let existing: i64 =
            conn.query_row("SELECT COUNT(*) FROM hub_identity", [], |row| row.get(0))?;
        if existing > 0 {
            return Err(MetaError::Conflict(
                "this vault already has a Hub identity".to_owned(),
            ));
        }
        map_insert(
            conn.execute(
                "INSERT INTO hub_identity(hub_id, body_json) VALUES (?1, ?2)",
                params![hub.header.id.as_str(), to_json(hub)?],
            ),
            "hub identity",
            hub.header.id.as_str(),
        )
    }

    /// Records this vault's Hub role (at most one).
    pub fn insert_hub_identity(&self, hub: &HubIdentity) -> Result<(), MetaError> {
        Self::hub_insert_identity_on(self.conn(), hub)
    }

    pub fn get_hub_identity(&self) -> Result<Option<HubIdentity>, MetaError> {
        let rows: Vec<(String, HubIdentity)> = self.hub_bodies(
            "SELECT hub_id, body_json FROM hub_identity",
            &[],
            "hub identity",
        )?;
        if rows.len() > 1 {
            return Err(corrupt("more than one Hub identity".to_owned()));
        }
        match rows.into_iter().next() {
            Some((id, hub)) => {
                check_column(&id, hub.header.id.as_str(), "hub identity")?;
                Ok(Some(hub))
            }
            None => Ok(None),
        }
    }

    // ----- invitations -----

    fn hub_insert_invitation_on(
        conn: &rusqlite::Connection,
        inv: &HubInvitation,
    ) -> Result<(), MetaError> {
        inv.validate().map_err(|e| invalid("invitation", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO hub_invitations(invitation_id, token_digest, project_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    inv.header.id.as_str(),
                    inv.token_digest.to_hex(),
                    inv.project_id.as_str(),
                    to_json(inv)?
                ],
            ),
            "invitation",
            inv.header.id.as_str(),
        )
    }

    pub fn insert_invitation(&self, inv: &HubInvitation) -> Result<(), MetaError> {
        if inv.status != InvitationStatus::Open {
            return Err(MetaError::UnsupportedSchema(
                "a new invitation must be open".to_owned(),
            ));
        }
        Self::hub_insert_invitation_on(self.conn(), inv)
    }

    fn decode_invitation(key: &str, digest: &str, inv: &HubInvitation) -> Result<(), MetaError> {
        check_column(key, inv.header.id.as_str(), "invitation")?;
        check_column(digest, &inv.token_digest.to_hex(), "invitation")?;
        inv.validate()
            .map_err(|e| corrupt(format!("invitation: {e}")))
    }

    fn invitations_where(
        &self,
        clause: &str,
        args: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<HubInvitation>, MetaError> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT invitation_id, token_digest, body_json FROM hub_invitations {clause}"
        ))?;
        let mut rows = stmt.query(args)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let digest: String = row.get(1)?;
            let body: String = row.get(2)?;
            let inv: HubInvitation = from_json(&body, "invitation")?;
            Self::decode_invitation(&key, &digest, &inv)?;
            out.push(inv);
        }
        Ok(out)
    }

    pub fn get_invitation_by_token_digest(
        &self,
        digest: &DigestSha256,
    ) -> Result<Option<HubInvitation>, MetaError> {
        Ok(self
            .invitations_where("WHERE token_digest = ?1", &[&digest.to_hex()])?
            .into_iter()
            .next())
    }

    pub fn get_invitation(&self, id: &OpaqueId) -> Result<HubInvitation, MetaError> {
        self.invitations_where("WHERE invitation_id = ?1", &[&id.as_str()])?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)
    }

    pub fn list_invitations(&self) -> Result<Vec<HubInvitation>, MetaError> {
        self.invitations_where("ORDER BY rowid", &[])
    }

    /// Revokes an open invitation.
    pub fn revoke_invitation(&self, id: &OpaqueId) -> Result<HubInvitation, MetaError> {
        let mut inv = self.get_invitation(id)?;
        if inv.status != InvitationStatus::Open {
            return Err(MetaError::Conflict(format!(
                "invitation {} is {}",
                id.as_str(),
                inv.status.as_str()
            )));
        }
        inv.status = InvitationStatus::Revoked;
        self.conn().execute(
            "UPDATE hub_invitations SET body_json = ?2 WHERE invitation_id = ?1",
            params![id.as_str(), to_json(&inv)?],
        )?;
        Ok(inv)
    }

    // ----- devices -----

    fn hub_insert_device_on(
        conn: &rusqlite::Connection,
        d: &DeviceIdentity,
    ) -> Result<(), MetaError> {
        d.validate().map_err(|e| invalid("device", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO hub_devices(device_id, holder_id, project_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    d.header.id.as_str(),
                    d.holder_id.as_str(),
                    d.project_id.as_str(),
                    to_json(d)?
                ],
            ),
            "device",
            d.header.id.as_str(),
        )
    }

    fn devices_where(
        &self,
        clause: &str,
        args: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<DeviceIdentity>, MetaError> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT device_id, holder_id, project_id, body_json FROM hub_devices {clause}"
        ))?;
        let mut rows = stmt.query(args)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let holder: String = row.get(1)?;
            let project: String = row.get(2)?;
            let body: String = row.get(3)?;
            let d: DeviceIdentity = from_json(&body, "device")?;
            check_column(&id, d.header.id.as_str(), "device")?;
            check_column(&holder, d.holder_id.as_str(), "device")?;
            check_column(&project, d.project_id.as_str(), "device")?;
            d.validate().map_err(|e| corrupt(format!("device: {e}")))?;
            out.push(d);
        }
        Ok(out)
    }

    pub fn get_device(&self, id: &OpaqueId) -> Result<DeviceIdentity, MetaError> {
        self.devices_where("WHERE device_id = ?1", &[&id.as_str()])?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)
    }

    pub fn get_device_by_holder(
        &self,
        holder: &OpaqueId,
    ) -> Result<Option<DeviceIdentity>, MetaError> {
        Ok(self
            .devices_where("WHERE holder_id = ?1", &[&holder.as_str()])?
            .into_iter()
            .next())
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceIdentity>, MetaError> {
        self.devices_where("ORDER BY rowid", &[])
    }

    /// Redeems an open invitation: marks it redeemed, inserts the device and
    /// appends its `device_enrolled` event, in one transaction.
    pub fn enroll_device(
        &self,
        invitation_id: &OpaqueId,
        device: &DeviceIdentity,
    ) -> Result<HubEvent, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let mut inv = self.get_invitation(invitation_id)?;
        if inv.status != InvitationStatus::Open {
            return Err(MetaError::Conflict(format!(
                "invitation {} is {}",
                invitation_id.as_str(),
                inv.status.as_str()
            )));
        }
        if inv.project_id != device.project_id || device.invitation_id != *invitation_id {
            return Err(MetaError::UnsupportedSchema(
                "a device must bind its invitation's Project".to_owned(),
            ));
        }
        inv.status = InvitationStatus::Redeemed;
        inv.device_id = Some(device.header.id.clone());
        tx.execute(
            "UPDATE hub_invitations SET body_json = ?2 WHERE invitation_id = ?1",
            params![invitation_id.as_str(), to_json(&inv)?],
        )?;
        Self::hub_insert_device_on(&tx, device)?;
        let event = Self::hub_append_event_on(
            &tx,
            &device.hub_id,
            &device.project_id,
            HubEventKind::DeviceEnrolled {
                device_id: device.header.id.clone(),
                participant_id: device.participant_id.clone(),
                public_key_hex: device.public_key_hex.clone(),
            },
            None,
        )?;
        tx.commit()?;
        Ok(event)
    }

    /// Revokes a device and appends its `device_revoked` event; drops any
    /// outstanding handshake nonces.
    pub fn revoke_device(&self, id: &OpaqueId) -> Result<(DeviceIdentity, HubEvent), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let mut d = self.get_device(id)?;
        if d.status == DeviceStatus::Revoked {
            return Err(MetaError::Conflict(format!(
                "device {} is revoked",
                id.as_str()
            )));
        }
        d.status = DeviceStatus::Revoked;
        tx.execute(
            "UPDATE hub_devices SET body_json = ?2 WHERE device_id = ?1",
            params![id.as_str(), to_json(&d)?],
        )?;
        tx.execute(
            "DELETE FROM hub_nonces WHERE device_id = ?1",
            params![id.as_str()],
        )?;
        let event = Self::hub_append_event_on(
            &tx,
            &d.hub_id,
            &d.project_id,
            HubEventKind::DeviceRevoked {
                device_id: id.clone(),
            },
            None,
        )?;
        tx.commit()?;
        Ok((d, event))
    }

    // ----- nonces -----

    pub fn insert_hub_nonce(&self, device_id: &OpaqueId, nonce_hex: &str) -> Result<(), MetaError> {
        map_insert(
            self.conn().execute(
                "INSERT INTO hub_nonces(nonce_hex, device_id) VALUES (?1, ?2)",
                params![nonce_hex, device_id.as_str()],
            ),
            "nonce",
            nonce_hex,
        )
    }

    /// Consumes a nonce issued to `device_id`; true only the first time.
    pub fn take_hub_nonce(&self, device_id: &OpaqueId, nonce_hex: &str) -> Result<bool, MetaError> {
        let n = self.conn().execute(
            "DELETE FROM hub_nonces WHERE nonce_hex = ?1 AND device_id = ?2",
            params![nonce_hex, device_id.as_str()],
        )?;
        Ok(n == 1)
    }

    // ----- events -----

    fn hub_last_event_on(
        conn: &rusqlite::Connection,
        project_id: &OpaqueId,
    ) -> Result<Option<HubEvent>, MetaError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {EVENT_COLUMNS} FROM hub_events WHERE project_id = ?1 ORDER BY cursor DESC LIMIT 1"
        ))?;
        let mut rows = stmt.query(params![project_id.as_str()])?;
        match rows.next()? {
            Some(row) => Ok(Some(decode_event(row)?)),
            None => Ok(None),
        }
    }

    fn hub_insert_event_on(
        conn: &rusqlite::Connection,
        event: &HubEvent,
        claim: Option<&SeqClaim>,
    ) -> Result<(), MetaError> {
        let expected = event_claims_seq(event);
        let given = claim.map(|c| (c.device_id.clone(), c.seq));
        if expected != given {
            return Err(MetaError::UnsupportedSchema(
                "hub event claim disagrees with its outcome".to_owned(),
            ));
        }
        if let (Some(c), HubEventKind::Submission { envelope, .. }) = (claim, &event.kind)
            && envelope.digest().map_err(|e| invalid("hub event", e))? != c.envelope_digest
        {
            return Err(MetaError::UnsupportedSchema(
                "hub event claim digest disagrees with its envelope".to_owned(),
            ));
        }
        let seq = match claim {
            Some(c) => Some(to_i64(c.seq, "seq")?),
            None => None,
        };
        map_insert(
            conn.execute(
                "INSERT INTO hub_events(project_id, cursor, device_id, seq, envelope_digest, body_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event.project_id.as_str(),
                    to_i64(event.cursor, "cursor")?,
                    claim.map(|c| c.device_id.as_str().to_owned()),
                    seq,
                    claim.map(|c| c.envelope_digest.to_hex()),
                    to_json(event)?
                ],
            ),
            "hub event",
            &format!("{}#{}", event.project_id.as_str(), event.cursor),
        )
    }

    fn hub_append_event_on(
        conn: &rusqlite::Connection,
        hub_id: &OpaqueId,
        project_id: &OpaqueId,
        kind: HubEventKind,
        claim: Option<&SeqClaim>,
    ) -> Result<HubEvent, MetaError> {
        let prev = Self::hub_last_event_on(conn, project_id)?;
        let cursor = prev.as_ref().map_or(1, |p| p.cursor + 1);
        let checkpoint_digest = hub_event_digest(
            prev.as_ref().map(|p| &p.checkpoint_digest),
            hub_id,
            project_id,
            cursor,
            &kind,
        )
        .map_err(|e| invalid("hub event", e))?;
        let event = HubEvent {
            hub_id: hub_id.clone(),
            project_id: project_id.clone(),
            cursor,
            kind,
            checkpoint_digest,
        };
        event
            .validate_after(prev.as_ref())
            .map_err(|e| invalid("hub event", e))?;
        Self::hub_insert_event_on(conn, &event, claim)?;
        Ok(event)
    }

    /// Appends one event to a Project's chain atomically.
    pub fn append_hub_event(
        &self,
        hub_id: &OpaqueId,
        project_id: &OpaqueId,
        kind: HubEventKind,
        claim: Option<&SeqClaim>,
    ) -> Result<HubEvent, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let event = Self::hub_append_event_on(&tx, hub_id, project_id, kind, claim)?;
        tx.commit()?;
        Ok(event)
    }

    /// The event that claimed `(device, seq)`, if any.
    pub fn find_claimed_event(
        &self,
        device_id: &OpaqueId,
        seq: u64,
    ) -> Result<Option<HubEvent>, MetaError> {
        Ok(self
            .hub_rows(
                &format!(
                    "SELECT {EVENT_COLUMNS} FROM hub_events WHERE device_id = ?1 AND seq = ?2"
                ),
                &[&device_id.as_str(), &to_i64(seq, "seq")?],
                decode_event,
            )?
            .into_iter()
            .next())
    }

    /// The highest sequence a device has claimed (0 before the first).
    pub fn last_claimed_seq(&self, device_id: &OpaqueId) -> Result<u64, MetaError> {
        let max: Option<i64> = self.conn().query_row(
            "SELECT MAX(seq) FROM hub_events WHERE device_id = ?1",
            params![device_id.as_str()],
            |row| row.get(0),
        )?;
        max.map_or(Ok(0), |m| to_u64(m, "seq"))
    }

    /// Events after `after` (at most `limit`), each checked against its
    /// predecessor, so an edited, removed or reordered row fails closed.
    pub fn list_hub_events(
        &self,
        project_id: &OpaqueId,
        after: u64,
        limit: u32,
    ) -> Result<Vec<HubEvent>, MetaError> {
        let from = after.saturating_sub(1);
        let rows = self.hub_rows(
            &format!(
                "SELECT {EVENT_COLUMNS} FROM hub_events WHERE project_id = ?1 AND cursor > ?2 ORDER BY cursor LIMIT ?3"
            ),
            &[
                &project_id.as_str(),
                &to_i64(from, "cursor")?,
                &(i64::from(limit) + 1),
            ],
            decode_event,
        )?;
        let mut prev: Option<&HubEvent> = None;
        let mut out = Vec::new();
        for (i, event) in rows.iter().enumerate() {
            if i == 0 && after > 0 {
                if event.cursor != after {
                    return Err(corrupt("hub event chain is missing a row".to_owned()));
                }
                prev = Some(event);
                continue;
            }
            event
                .validate_after(prev)
                .map_err(|e| corrupt(format!("hub event: {e}")))?;
            prev = Some(event);
            out.push(event.clone());
        }
        out.truncate(limit as usize);
        Ok(out)
    }

    /// The head of a Project's chain (its last event), without re-verifying
    /// the whole chain.
    pub fn hub_head(
        &self,
        hub_id: &OpaqueId,
        project_id: &OpaqueId,
    ) -> Result<HubAuditCheckpoint, MetaError> {
        let last = Self::hub_last_event_on(self.conn(), project_id)?;
        Ok(HubAuditCheckpoint {
            hub_id: hub_id.clone(),
            project_id: project_id.clone(),
            cursor: last.as_ref().map_or(0, |e| e.cursor),
            digest: last.map(|e| e.checkpoint_digest),
        })
    }

    /// Verifies a Project's whole chain and returns its head.
    pub fn verify_hub_chain(
        &self,
        hub_id: &OpaqueId,
        project_id: &OpaqueId,
    ) -> Result<HubAuditCheckpoint, MetaError> {
        let events = self.hub_rows(
            &format!(
                "SELECT {EVENT_COLUMNS} FROM hub_events WHERE project_id = ?1 ORDER BY cursor"
            ),
            &[&project_id.as_str()],
            decode_event,
        )?;
        let mut prev: Option<&HubEvent> = None;
        for event in &events {
            event
                .validate_after(prev)
                .map_err(|e| corrupt(format!("hub event: {e}")))?;
            if event.hub_id != *hub_id {
                return Err(corrupt("hub event names another hub".to_owned()));
            }
            prev = Some(event);
        }
        Ok(HubAuditCheckpoint {
            hub_id: hub_id.clone(),
            project_id: project_id.clone(),
            cursor: prev.map_or(0, |e| e.cursor),
            digest: prev.map(|e| e.checkpoint_digest.clone()),
        })
    }

    pub fn list_hub_event_projects(&self) -> Result<Vec<OpaqueId>, MetaError> {
        let mut stmt = self
            .conn()
            .prepare("SELECT DISTINCT project_id FROM hub_events ORDER BY project_id")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let p: String = row.get(0)?;
            out.push(OpaqueId::new(p));
        }
        Ok(out)
    }

    pub fn list_all_hub_events(&self) -> Result<Vec<HubEvent>, MetaError> {
        self.hub_rows(
            &format!("SELECT {EVENT_COLUMNS} FROM hub_events ORDER BY project_id, cursor"),
            &[],
            decode_event,
        )
    }

    // ----- client links -----

    fn hub_insert_link_on(conn: &rusqlite::Connection, link: &HubLink) -> Result<(), MetaError> {
        link.validate().map_err(|e| invalid("hub link", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO hub_links(link_id, device_id, body_json) VALUES (?1, ?2, ?3)",
                params![
                    link.header.id.as_str(),
                    link.device_id.as_str(),
                    to_json(link)?
                ],
            ),
            "hub link",
            link.header.id.as_str(),
        )
    }

    /// Stores the device secret of a link being joined (before the Hub has
    /// enrolled it).
    pub fn insert_pending_hub_secret(
        &self,
        link_id: &OpaqueId,
        secret_hex: &str,
    ) -> Result<(), MetaError> {
        map_insert(
            self.conn().execute(
                "INSERT INTO hub_link_secrets(link_id, secret_hex) VALUES (?1, ?2)",
                params![link_id.as_str(), secret_hex],
            ),
            "hub link secret",
            link_id.as_str(),
        )
    }

    /// Completes a join: the link's secret must already be stored and the
    /// link must start empty.
    pub fn insert_hub_link(&self, link: &HubLink) -> Result<(), MetaError> {
        if link.last_seq != 0 || link.cursor != 0 || link.revoked {
            return Err(MetaError::UnsupportedSchema(
                "a new hub link starts empty".to_owned(),
            ));
        }
        if self.get_hub_link_secret(&link.header.id)?.is_none() {
            return Err(MetaError::NotFound);
        }
        Self::hub_insert_link_on(self.conn(), link)
    }

    fn links_where(
        &self,
        clause: &str,
        args: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<HubLink>, MetaError> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT link_id, device_id, body_json FROM hub_links {clause}"
        ))?;
        let mut rows = stmt.query(args)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let device: String = row.get(1)?;
            let body: String = row.get(2)?;
            let link: HubLink = from_json(&body, "hub link")?;
            check_column(&id, link.header.id.as_str(), "hub link")?;
            check_column(&device, link.device_id.as_str(), "hub link")?;
            link.validate()
                .map_err(|e| corrupt(format!("hub link: {e}")))?;
            out.push(link);
        }
        Ok(out)
    }

    pub fn get_hub_link(&self, id: &OpaqueId) -> Result<HubLink, MetaError> {
        self.links_where("WHERE link_id = ?1", &[&id.as_str()])?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)
    }

    pub fn list_hub_links(&self) -> Result<Vec<HubLink>, MetaError> {
        self.links_where("ORDER BY rowid", &[])
    }

    /// The device secret of a link; `None` after a restore (secrets are
    /// never exported).
    pub fn get_hub_link_secret(&self, id: &OpaqueId) -> Result<Option<String>, MetaError> {
        Ok(self
            .conn()
            .query_row(
                "SELECT secret_hex FROM hub_link_secrets WHERE link_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn hub_write_link_on(conn: &rusqlite::Connection, link: &HubLink) -> Result<(), MetaError> {
        link.validate().map_err(|e| invalid("hub link", e))?;
        let n = conn.execute(
            "UPDATE hub_links SET body_json = ?2 WHERE link_id = ?1",
            params![link.header.id.as_str(), to_json(link)?],
        )?;
        if n == 1 {
            Ok(())
        } else {
            Err(MetaError::NotFound)
        }
    }

    // ----- outbox -----

    fn hub_insert_outbox_on(
        conn: &rusqlite::Connection,
        e: &HubOutboxEntry,
    ) -> Result<(), MetaError> {
        e.validate().map_err(|err| invalid("outbox entry", err))?;
        map_insert(
            conn.execute(
                "INSERT INTO hub_outbox(link_id, seq, state, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    e.link_id.as_str(),
                    to_i64(e.envelope.body.seq, "seq")?,
                    e.state.as_str(),
                    to_json(e)?
                ],
            ),
            "outbox entry",
            e.link_id.as_str(),
        )
    }

    /// Appends a pending entry at `link.last_seq + 1` and advances the link.
    pub fn queue_hub_outbox(&self, entry: &HubOutboxEntry) -> Result<HubLink, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let mut link = self.get_hub_link(&entry.link_id)?;
        if link.revoked {
            return Err(MetaError::Conflict("the hub link is revoked".to_owned()));
        }
        if entry.state != OutboxState::Pending
            || entry.envelope.body.seq != link.last_seq + 1
            || entry.envelope.body.device_id != link.device_id
            || entry.envelope.body.hub_id != link.hub_id
        {
            return Err(MetaError::Conflict(
                "an outbox entry must be the link's next pending sequence".to_owned(),
            ));
        }
        Self::hub_insert_outbox_on(&tx, entry)?;
        link.last_seq += 1;
        Self::hub_write_link_on(&tx, &link)?;
        tx.commit()?;
        Ok(link)
    }

    fn decode_outbox(row: &rusqlite::Row<'_>) -> Result<HubOutboxEntry, MetaError> {
        let link: String = row.get(0)?;
        let seq: i64 = row.get(1)?;
        let state: String = row.get(2)?;
        let body: String = row.get(3)?;
        let e: HubOutboxEntry = from_json(&body, "outbox entry")?;
        check_column(&link, e.link_id.as_str(), "outbox entry")?;
        check_column(&state, e.state.as_str(), "outbox entry")?;
        if to_u64(seq, "seq")? != e.envelope.body.seq {
            return Err(corrupt(
                "outbox entry row columns disagree with its body".to_owned(),
            ));
        }
        e.validate()
            .map_err(|err| corrupt(format!("outbox entry: {err}")))?;
        Ok(e)
    }

    pub fn list_hub_outbox(
        &self,
        link_id: &OpaqueId,
        pending_only: bool,
    ) -> Result<Vec<HubOutboxEntry>, MetaError> {
        let filter = if pending_only {
            " AND state = 'pending'"
        } else {
            ""
        };
        self.hub_rows(
            &format!(
                "SELECT link_id, seq, state, body_json FROM hub_outbox WHERE link_id = ?1{filter} ORDER BY seq LIMIT 1000"
            ),
            &[&link_id.as_str()],
            Self::decode_outbox,
        )
    }

    fn list_all_hub_outbox(&self) -> Result<Vec<HubOutboxEntry>, MetaError> {
        self.hub_rows(
            "SELECT link_id, seq, state, body_json FROM hub_outbox ORDER BY link_id, seq",
            &[],
            Self::decode_outbox,
        )
    }

    /// Records the Hub's outcome for one pending entry.
    pub fn record_hub_outcome(
        &self,
        link_id: &OpaqueId,
        seq: u64,
        outcome: &SyncOutcome,
    ) -> Result<(), MetaError> {
        let mut entry = self
            .hub_rows(
                "SELECT link_id, seq, state, body_json FROM hub_outbox WHERE link_id = ?1 AND seq = ?2",
                &[&link_id.as_str(), &to_i64(seq, "seq")?],
                Self::decode_outbox,
            )?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)?;
        if entry.state == OutboxState::Done {
            return if entry.outcome.as_ref() == Some(outcome) {
                Ok(())
            } else {
                Err(MetaError::Conflict(format!(
                    "outbox entry {seq} already has another outcome"
                )))
            };
        }
        entry.state = OutboxState::Done;
        entry.outcome = Some(outcome.clone());
        self.conn().execute(
            "UPDATE hub_outbox SET state = ?3, body_json = ?4 WHERE link_id = ?1 AND seq = ?2",
            params![
                link_id.as_str(),
                to_i64(seq, "seq")?,
                entry.state.as_str(),
                to_json(&entry)?
            ],
        )?;
        Ok(())
    }

    // ----- mirror -----

    fn hub_insert_mirror_on(
        conn: &rusqlite::Connection,
        link_id: &OpaqueId,
        event: &HubEvent,
    ) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO hub_mirror(link_id, cursor, body_json) VALUES (?1, ?2, ?3)",
                params![
                    link_id.as_str(),
                    to_i64(event.cursor, "cursor")?,
                    to_json(event)?
                ],
            ),
            "mirrored event",
            link_id.as_str(),
        )
    }

    /// Appends pulled events after the link's head: each must follow the
    /// previous one in the Hub's chain, so a Hub that rewrote history is
    /// refused. A `device_revoked` event for the link's own device marks the
    /// link revoked.
    pub fn append_hub_mirror(
        &self,
        link_id: &OpaqueId,
        events: &[HubEvent],
    ) -> Result<HubLink, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let mut link = self.get_hub_link(link_id)?;
        let mut prev = self.last_mirrored(link_id)?;
        if prev.as_ref().map(|p| p.cursor).unwrap_or(0) != link.cursor {
            return Err(corrupt("the hub mirror disagrees with its link".to_owned()));
        }
        for event in events {
            if event.hub_id != link.hub_id || event.project_id != link.project_id {
                return Err(MetaError::UnsupportedSchema(
                    "a pulled event names another hub or project".to_owned(),
                ));
            }
            event
                .validate_after(prev.as_ref())
                .map_err(|e| MetaError::UnsupportedSchema(format!("pulled event: {e}")))?;
            Self::hub_insert_mirror_on(&tx, link_id, event)?;
            if let HubEventKind::DeviceRevoked { device_id } = &event.kind
                && *device_id == link.device_id
            {
                link.revoked = true;
            }
            link.cursor = event.cursor;
            link.head_digest = Some(event.checkpoint_digest.clone());
            prev = Some(event.clone());
        }
        Self::hub_write_link_on(&tx, &link)?;
        tx.commit()?;
        Ok(link)
    }

    fn decode_mirror(row: &rusqlite::Row<'_>) -> Result<(OpaqueId, HubEvent), MetaError> {
        let link: String = row.get(0)?;
        let cursor: i64 = row.get(1)?;
        let body: String = row.get(2)?;
        let event: HubEvent = from_json(&body, "mirrored event")?;
        if to_u64(cursor, "cursor")? != event.cursor {
            return Err(corrupt(
                "mirrored event row columns disagree with its body".to_owned(),
            ));
        }
        Ok((OpaqueId::new(link), event))
    }

    fn last_mirrored(&self, link_id: &OpaqueId) -> Result<Option<HubEvent>, MetaError> {
        Ok(self
            .hub_rows(
                "SELECT link_id, cursor, body_json FROM hub_mirror WHERE link_id = ?1 ORDER BY cursor DESC LIMIT 1",
                &[&link_id.as_str()],
                Self::decode_mirror,
            )?
            .into_iter()
            .next()
            .map(|(_, e)| e))
    }

    /// Mirrored events after `after`, each checked against its predecessor.
    pub fn list_hub_mirror(
        &self,
        link_id: &OpaqueId,
        after: u64,
        limit: u32,
    ) -> Result<Vec<HubEvent>, MetaError> {
        let rows = self.hub_rows(
            "SELECT link_id, cursor, body_json FROM hub_mirror WHERE link_id = ?1 AND cursor >= ?2 ORDER BY cursor LIMIT ?3",
            &[
                &link_id.as_str(),
                &to_i64(after.max(1), "cursor")?,
                &(i64::from(limit) + 1),
            ],
            Self::decode_mirror,
        )?;
        let mut prev: Option<HubEvent> = None;
        let mut out = Vec::new();
        for (_, event) in rows {
            if after > 0 && prev.is_none() && event.cursor == after {
                prev = Some(event);
                continue;
            }
            event
                .validate_after(prev.as_ref())
                .map_err(|e| corrupt(format!("mirrored event: {e}")))?;
            prev = Some(event.clone());
            out.push(event);
        }
        out.truncate(limit as usize);
        Ok(out)
    }

    fn list_all_hub_mirror(&self) -> Result<Vec<MirrorRow>, MetaError> {
        Ok(self
            .hub_rows(
                "SELECT link_id, cursor, body_json FROM hub_mirror ORDER BY link_id, cursor",
                &[],
                Self::decode_mirror,
            )?
            .into_iter()
            .map(|(link_id, event)| MirrorRow { link_id, event })
            .collect())
    }

    // ----- backup / restore / consistency -----

    /// Every Hub-side and client-side row for a backup. Device secrets are
    /// deliberately absent.
    pub fn hub_backup_families(&self) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        Ok(vec![
            (
                "hub_identity",
                v(serde_json::to_value(
                    self.get_hub_identity()?.into_iter().collect::<Vec<_>>(),
                ))?,
            ),
            (
                "hub_invitations",
                v(serde_json::to_value(self.list_invitations()?))?,
            ),
            (
                "hub_devices",
                v(serde_json::to_value(self.list_devices()?))?,
            ),
            (
                "hub_events",
                v(serde_json::to_value(self.list_all_hub_events()?))?,
            ),
            (
                "hub_links",
                v(serde_json::to_value(self.list_hub_links()?))?,
            ),
            (
                "hub_outbox",
                v(serde_json::to_value(self.list_all_hub_outbox()?))?,
            ),
            (
                "hub_mirror",
                v(serde_json::to_value(self.list_all_hub_mirror()?))?,
            ),
        ])
    }

    pub fn restore_hub_identity_row(&self, hub: &HubIdentity) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hub_insert_identity_on(self.conn(), hub))
    }

    /// Replays an invitation. An invitation that was open at backup time is
    /// restored revoked: a restored Hub never accepts a token issued before
    /// the backup (the file could have been edited to plant one).
    pub fn restore_invitation_row(&self, inv: &HubInvitation) -> Result<(), MetaError> {
        let mut inv = inv.clone();
        if inv.status == InvitationStatus::Open {
            inv.status = InvitationStatus::Revoked;
        }
        restore_conflict_is_corrupt(Self::hub_insert_invitation_on(self.conn(), &inv))
    }

    pub fn restore_device_row(&self, d: &DeviceIdentity) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hub_insert_device_on(self.conn(), d))
    }

    /// Replays one event; it must follow the Project's current last event.
    pub fn restore_hub_event_row(&self, event: &HubEvent) -> Result<(), MetaError> {
        let prev = Self::hub_last_event_on(self.conn(), &event.project_id)?;
        event
            .validate_after(prev.as_ref())
            .map_err(|e| corrupt(format!("tampered backup: {e}")))?;
        let claim = match (event_claims_seq(event), &event.kind) {
            (Some((device_id, seq)), HubEventKind::Submission { envelope, .. }) => Some(SeqClaim {
                device_id,
                seq,
                envelope_digest: envelope.digest().map_err(corrupt)?,
            }),
            _ => None,
        };
        restore_conflict_is_corrupt(Self::hub_insert_event_on(
            self.conn(),
            event,
            claim.as_ref(),
        ))
    }

    /// Replays a link without its secret (the restored device must
    /// re-enroll).
    pub fn restore_hub_link_row(&self, link: &HubLink) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hub_insert_link_on(self.conn(), link))
    }

    pub fn restore_hub_outbox_row(&self, e: &HubOutboxEntry) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hub_insert_outbox_on(self.conn(), e))
    }

    pub fn restore_hub_mirror_row(&self, row: &MirrorRow) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hub_insert_mirror_on(
            self.conn(),
            &row.link_id,
            &row.event,
        ))
    }

    /// Cross-row invariants of every Hub family (restore and diagnostics).
    pub fn verify_hub_consistency(&self) -> Result<(), MetaError> {
        let hub = self.get_hub_identity()?;
        let invitations = self.list_invitations()?;
        let devices = self.list_devices()?;
        let events = self.list_all_hub_events()?;
        if hub.is_none() && !(invitations.is_empty() && devices.is_empty() && events.is_empty()) {
            return Err(corrupt("Hub rows exist without a Hub identity".to_owned()));
        }
        if let Some(hub) = &hub {
            let scope_ok = |h: &medscale_contracts::objects::ObjectHeader| {
                h.realm_id == hub.header.realm_id
                    && h.authority_scope_id == hub.header.authority_scope_id
            };
            let by_invitation: HashMap<&str, &HubInvitation> = invitations
                .iter()
                .map(|i| (i.header.id.as_str(), i))
                .collect();
            for inv in &invitations {
                if !scope_ok(&inv.header) {
                    return Err(corrupt(
                        "an invitation is outside the Hub's scope".to_owned(),
                    ));
                }
            }
            for d in &devices {
                let inv = by_invitation
                    .get(d.invitation_id.as_str())
                    .ok_or_else(|| corrupt("a device names a missing invitation".to_owned()))?;
                if !scope_ok(&d.header)
                    || d.hub_id != hub.header.id
                    || inv.device_id.as_ref() != Some(&d.header.id)
                    || inv.project_id != d.project_id
                {
                    return Err(corrupt("a device disagrees with its invitation".to_owned()));
                }
            }
            for inv in &invitations {
                if let Some(device) = &inv.device_id
                    && !devices.iter().any(|d| d.header.id == *device)
                {
                    return Err(corrupt(
                        "a redeemed invitation names a missing device".to_owned(),
                    ));
                }
            }
            for project in self.list_hub_event_projects()? {
                self.verify_hub_chain(&hub.header.id, &project)?;
            }
            let device_ids: HashMap<&str, &DeviceIdentity> =
                devices.iter().map(|d| (d.header.id.as_str(), d)).collect();
            for d in &devices {
                let enrolled = events.iter().any(|e| {
                    matches!(
                        &e.kind,
                        HubEventKind::DeviceEnrolled { device_id, participant_id, public_key_hex }
                            if *device_id == d.header.id
                                && *participant_id == d.participant_id
                                && *public_key_hex == d.public_key_hex
                    )
                });
                if !enrolled {
                    return Err(corrupt(
                        "a device disagrees with its enrollment event".to_owned(),
                    ));
                }
                let revoked = events.iter().any(|e| {
                    matches!(&e.kind, HubEventKind::DeviceRevoked { device_id } if *device_id == d.header.id)
                });
                if revoked != (d.status == DeviceStatus::Revoked) {
                    return Err(corrupt(
                        "a device status disagrees with its events".to_owned(),
                    ));
                }
            }
            for e in &events {
                let named = match &e.kind {
                    HubEventKind::DeviceEnrolled { device_id, .. }
                    | HubEventKind::DeviceRevoked { device_id } => device_id,
                    HubEventKind::Submission { envelope, .. } => &envelope.body.device_id,
                };
                let d = device_ids
                    .get(named.as_str())
                    .ok_or_else(|| corrupt("an event names a missing device".to_owned()))?;
                if d.project_id != e.project_id {
                    return Err(corrupt(
                        "an event names another Project's device".to_owned(),
                    ));
                }
            }
        }
        // Client side: each link's outbox is contiguous 1..=last_seq and its
        // mirror is a verified chain ending at the link's head.
        let outbox = self.list_all_hub_outbox()?;
        let mirror = self.list_all_hub_mirror()?;
        for link in self.list_hub_links()? {
            let seqs: Vec<u64> = outbox
                .iter()
                .filter(|e| e.link_id == link.header.id)
                .map(|e| e.envelope.body.seq)
                .collect();
            if seqs.len() as u64 != link.last_seq
                || seqs.iter().enumerate().any(|(i, s)| *s != i as u64 + 1)
            {
                return Err(corrupt(
                    "a hub outbox is not contiguous with its link".to_owned(),
                ));
            }
            let mut prev: Option<&HubEvent> = None;
            for row in mirror.iter().filter(|r| r.link_id == link.header.id) {
                row.event
                    .validate_after(prev)
                    .map_err(|e| corrupt(format!("mirrored event: {e}")))?;
                prev = Some(&row.event);
            }
            if prev.map_or(0, |e| e.cursor) != link.cursor
                || prev.map(|e| &e.checkpoint_digest) != link.head_digest.as_ref()
            {
                return Err(corrupt("a hub mirror disagrees with its link".to_owned()));
            }
        }
        if outbox
            .iter()
            .any(|e| self.get_hub_link(&e.link_id).is_err())
            || mirror
                .iter()
                .any(|r| self.get_hub_link(&r.link_id).is_err())
        {
            return Err(corrupt("a hub row names a missing link".to_owned()));
        }
        Ok(())
    }
}

/// One mirrored event with its link (backup form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorRow {
    pub link_id: OpaqueId,
    pub event: HubEvent,
}

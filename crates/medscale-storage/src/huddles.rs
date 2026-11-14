//! AudioFlow Advanced huddle rows (Spec 088, storage schema v17).
//!
//! Same pattern as Specs 079-087: validated contract values as JSON with
//! the columns queries need; reads re-check columns and fail closed.
//! Huddles and participants change by compare-and-set on their revision,
//! committed together with their receipt. Media deletion removes the Spec
//! 081 audio source row (with its bytes), its transcript revisions and
//! their receipts, marks the media `deleted`, and records the removed
//! digests, in one transaction. SQLite `secure_delete` is enabled for that
//! transaction so freed pages are overwritten.

use std::collections::HashMap;

use medscale_contracts::huddles::{
    Huddle, HuddleMedia, HuddleParticipant, HuddleProposal, HuddleReceipt, MediaState,
};
use medscale_contracts::objects::OpaqueId;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v17 DDL, executed inside `begin/finish_migration(17)`.
pub(crate) const V17_DDL: &str = r"
CREATE TABLE IF NOT EXISTS hud_huddles (
  huddle_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hud_participants (
  participant_id TEXT PRIMARY KEY,
  huddle_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hud_media (
  media_id TEXT PRIMARY KEY,
  huddle_id TEXT NOT NULL,
  source_id TEXT NOT NULL UNIQUE,
  state TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hud_proposals (
  proposal_id TEXT PRIMARY KEY,
  huddle_id TEXT NOT NULL,
  state TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hud_receipts (
  receipt_id TEXT PRIMARY KEY,
  huddle_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
";

/// Names of the v17 tables (for rewind tests of earlier specs).
pub const HUDDLE_TABLES: [&str; 5] = [
    "hud_huddles",
    "hud_participants",
    "hud_media",
    "hud_proposals",
    "hud_receipts",
];

fn corrupt(message: String) -> MetaError {
    MetaError::CorruptObjectBody(message)
}

fn invalid(what: &str, e: String) -> MetaError {
    MetaError::UnsupportedSchema(format!("{what}: {e}"))
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

fn check(column: &str, body: &str, what: &str) -> Result<(), MetaError> {
    if column == body {
        Ok(())
    } else {
        Err(corrupt(format!(
            "{what} row columns disagree with its body"
        )))
    }
}

fn revision_i64(r: u64, what: &str) -> Result<i64, MetaError> {
    i64::try_from(r).map_err(|_| invalid(what, "revision overflow".to_owned()))
}

fn decode_huddle(row: &rusqlite::Row<'_>) -> Result<Huddle, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let v: Huddle = from_json(&body, "huddle")?;
    check(&id, v.header.id.as_str(), "huddle")?;
    check(&project_id, v.project_id.as_str(), "huddle")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("huddle revision disagrees".to_owned()));
    }
    v.validate().map_err(|e| corrupt(format!("huddle: {e}")))?;
    Ok(v)
}

fn decode_participant(row: &rusqlite::Row<'_>) -> Result<HuddleParticipant, MetaError> {
    let id: String = row.get(0)?;
    let huddle_id: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let v: HuddleParticipant = from_json(&body, "huddle participant")?;
    check(&id, v.header.id.as_str(), "huddle participant")?;
    check(&huddle_id, v.huddle_id.as_str(), "huddle participant")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("participant revision disagrees".to_owned()));
    }
    v.validate()
        .map_err(|e| corrupt(format!("huddle participant: {e}")))?;
    Ok(v)
}

fn decode_media(row: &rusqlite::Row<'_>) -> Result<HuddleMedia, MetaError> {
    let id: String = row.get(0)?;
    let huddle_id: String = row.get(1)?;
    let source_id: String = row.get(2)?;
    let state: String = row.get(3)?;
    let body: String = row.get(4)?;
    let v: HuddleMedia = from_json(&body, "huddle media")?;
    check(&id, v.header.id.as_str(), "huddle media")?;
    check(&huddle_id, v.huddle_id.as_str(), "huddle media")?;
    check(&source_id, v.source_id.as_str(), "huddle media")?;
    check(&state, v.state.as_str(), "huddle media")?;
    v.validate()
        .map_err(|e| corrupt(format!("huddle media: {e}")))?;
    Ok(v)
}

fn decode_proposal(row: &rusqlite::Row<'_>) -> Result<HuddleProposal, MetaError> {
    let id: String = row.get(0)?;
    let huddle_id: String = row.get(1)?;
    let state: String = row.get(2)?;
    let body: String = row.get(3)?;
    let v: HuddleProposal = from_json(&body, "huddle proposal")?;
    check(&id, v.header.id.as_str(), "huddle proposal")?;
    check(&huddle_id, v.huddle_id.as_str(), "huddle proposal")?;
    check(&state, v.state.as_str(), "huddle proposal")?;
    v.validate()
        .map_err(|e| corrupt(format!("huddle proposal: {e}")))?;
    Ok(v)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<HuddleReceipt, MetaError> {
    let id: String = row.get(0)?;
    let huddle_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let v: HuddleReceipt = from_json(&body, "huddle receipt")?;
    check(&id, v.header.id.as_str(), "huddle receipt")?;
    check(&huddle_id, v.huddle_id.as_str(), "huddle receipt")?;
    Ok(v)
}

const HUDDLE_COLUMNS: &str = "huddle_id, project_id, revision, body_json";
const PARTICIPANT_COLUMNS: &str = "participant_id, huddle_id, revision, body_json";
const MEDIA_COLUMNS: &str = "media_id, huddle_id, source_id, state, body_json";
const PROPOSAL_COLUMNS: &str = "proposal_id, huddle_id, state, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, huddle_id, body_json";

/// One huddle change and its receipt, committed together.
#[derive(Debug, Default)]
pub struct HuddleChange<'a> {
    /// `(value, expected_revision)`; `None` expected inserts.
    pub huddle: Option<(&'a Huddle, Option<u64>)>,
    pub participant: Option<(&'a HuddleParticipant, Option<u64>)>,
    pub new_media: Option<&'a HuddleMedia>,
    /// A proposal to insert (`None` expected state) or to move from
    /// `proposed` (`Some(())`).
    pub proposal: Option<(&'a HuddleProposal, bool)>,
    /// Media to delete with its Spec 081 source and transcripts.
    pub delete_media: Vec<&'a HuddleMedia>,
    pub receipt: Option<&'a HuddleReceipt>,
}

impl SqliteMetaStore {
    fn hud_rows<T>(
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

    /// Allocates one `prefix-N` huddle id from a durable sequence.
    pub fn alloc_huddle_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("hud_seq_{prefix}");
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
                raw.parse::<u64>()
                    .map_err(|_| corrupt(format!("id sequence {seq_key} is not numeric")))?
                    + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, next.to_string()],
        )?;
        tx.commit()?;
        Ok(OpaqueId::new(format!("{prefix}-{next}")))
    }

    fn hud_write_huddle_on(
        conn: &rusqlite::Connection,
        h: &Huddle,
        expected: Option<u64>,
    ) -> Result<(), MetaError> {
        h.validate().map_err(|e| invalid("huddle", e))?;
        let rev = revision_i64(h.revision, "huddle")?;
        match expected {
            None => {
                if h.revision != 1 {
                    return Err(invalid("huddle", "a new huddle has revision 1".to_owned()));
                }
                map_insert(
                    conn.execute(
                        "INSERT INTO hud_huddles(huddle_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                        params![h.header.id.as_str(), h.project_id.as_str(), rev, to_json(h)?],
                    ),
                    "huddle",
                    h.header.id.as_str(),
                )
            }
            Some(e) => {
                if h.revision != e + 1 {
                    return Err(invalid("huddle", "revision must advance by one".to_owned()));
                }
                let changed = conn.execute(
                    "UPDATE hud_huddles SET revision = ?1, body_json = ?2 WHERE huddle_id = ?3 AND revision = ?4",
                    params![rev, to_json(h)?, h.header.id.as_str(), revision_i64(e, "huddle")?],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict(
                        "huddle changed concurrently".to_owned(),
                    ))
                }
            }
        }
    }

    fn hud_write_participant_on(
        conn: &rusqlite::Connection,
        p: &HuddleParticipant,
        expected: Option<u64>,
    ) -> Result<(), MetaError> {
        p.validate().map_err(|e| invalid("huddle participant", e))?;
        let rev = revision_i64(p.revision, "participant")?;
        match expected {
            None => {
                if p.revision != 1 {
                    return Err(invalid(
                        "huddle participant",
                        "a new participant has revision 1".to_owned(),
                    ));
                }
                map_insert(
                    conn.execute(
                        "INSERT INTO hud_participants(participant_id, huddle_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                        params![p.header.id.as_str(), p.huddle_id.as_str(), rev, to_json(p)?],
                    ),
                    "huddle participant",
                    p.header.id.as_str(),
                )
            }
            Some(e) => {
                if p.revision != e + 1 {
                    return Err(invalid(
                        "huddle participant",
                        "revision must advance by one".to_owned(),
                    ));
                }
                let changed = conn.execute(
                    "UPDATE hud_participants SET revision = ?1, body_json = ?2 WHERE participant_id = ?3 AND revision = ?4",
                    params![rev, to_json(p)?, p.header.id.as_str(), revision_i64(e, "participant")?],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict(
                        "participant changed concurrently".to_owned(),
                    ))
                }
            }
        }
    }

    fn hud_insert_media_on(conn: &rusqlite::Connection, m: &HuddleMedia) -> Result<(), MetaError> {
        m.validate().map_err(|e| invalid("huddle media", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO hud_media(media_id, huddle_id, source_id, state, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    m.header.id.as_str(),
                    m.huddle_id.as_str(),
                    m.source_id.as_str(),
                    m.state.as_str(),
                    to_json(m)?
                ],
            ),
            "huddle media",
            m.header.id.as_str(),
        )
    }

    fn hud_write_proposal_on(
        conn: &rusqlite::Connection,
        p: &HuddleProposal,
        update: bool,
    ) -> Result<(), MetaError> {
        p.validate().map_err(|e| invalid("huddle proposal", e))?;
        if update {
            let changed = conn.execute(
                "UPDATE hud_proposals SET state = ?1, body_json = ?2 WHERE proposal_id = ?3 AND state = 'proposed'",
                params![p.state.as_str(), to_json(p)?, p.header.id.as_str()],
            )?;
            if changed == 1 {
                Ok(())
            } else {
                Err(MetaError::Conflict(
                    "proposal was already reviewed".to_owned(),
                ))
            }
        } else {
            map_insert(
                conn.execute(
                    "INSERT INTO hud_proposals(proposal_id, huddle_id, state, body_json) VALUES (?1, ?2, ?3, ?4)",
                    params![p.header.id.as_str(), p.huddle_id.as_str(), p.state.as_str(), to_json(p)?],
                ),
                "huddle proposal",
                p.header.id.as_str(),
            )
        }
    }

    fn hud_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &HuddleReceipt,
    ) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO hud_receipts(receipt_id, huddle_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.huddle_id.as_str(), to_json(r)?],
            ),
            "huddle receipt",
            r.header.id.as_str(),
        )
    }

    fn hud_delete_media_on(conn: &rusqlite::Connection, m: &HuddleMedia) -> Result<(), MetaError> {
        let source = m.source_id.as_str();
        conn.execute(
            "DELETE FROM audio_transcript_receipts WHERE source_id = ?1",
            params![source],
        )?;
        conn.execute(
            "DELETE FROM audio_transcripts WHERE source_id = ?1",
            params![source],
        )?;
        let removed = conn.execute(
            "DELETE FROM audio_sources WHERE source_id = ?1",
            params![source],
        )?;
        if removed != 1 {
            return Err(MetaError::Conflict(format!(
                "audio source {source} is already gone"
            )));
        }
        let mut deleted = m.clone();
        deleted.state = MediaState::Deleted;
        let changed = conn.execute(
            "UPDATE hud_media SET state = ?1, body_json = ?2 WHERE media_id = ?3 AND state = 'present'",
            params![deleted.state.as_str(), to_json(&deleted)?, m.header.id.as_str()],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(MetaError::Conflict("media was already deleted".to_owned()))
        }
    }

    fn hud_apply(
        &self,
        change: &HuddleChange<'_>,
        receipt: &HuddleReceipt,
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        if let Some((h, expected)) = change.huddle {
            Self::hud_write_huddle_on(&tx, h, expected)?;
        }
        if let Some((p, expected)) = change.participant {
            Self::hud_write_participant_on(&tx, p, expected)?;
        }
        if let Some(m) = change.new_media {
            Self::hud_insert_media_on(&tx, m)?;
        }
        if let Some((p, update)) = change.proposal {
            Self::hud_write_proposal_on(&tx, p, update)?;
        }
        for m in &change.delete_media {
            Self::hud_delete_media_on(&tx, m)?;
        }
        Self::hud_insert_receipt_on(&tx, receipt)?;
        tx.commit()?;
        Ok(())
    }

    /// Commits one huddle change and its receipt in a single transaction.
    pub fn commit_huddle_change(&self, change: &HuddleChange<'_>) -> Result<(), MetaError> {
        let Some(receipt) = change.receipt else {
            return Err(invalid(
                "huddle change",
                "a change carries its receipt".to_owned(),
            ));
        };
        let deleting = !change.delete_media.is_empty();
        if deleting {
            self.conn().execute_batch("PRAGMA secure_delete = ON;")?;
        }
        let result = self.hud_apply(change, receipt);
        if deleting {
            self.conn().execute_batch("PRAGMA secure_delete = OFF;")?;
        }
        result
    }

    /// Records a refused act (receipt only).
    pub fn insert_huddle_refusal(&self, r: &HuddleReceipt) -> Result<(), MetaError> {
        if r.refusal.is_none() {
            return Err(invalid("huddle receipt", "not a refusal".to_owned()));
        }
        Self::hud_insert_receipt_on(self.conn(), r)
    }

    pub fn get_huddle(&self, id: &OpaqueId) -> Result<Huddle, MetaError> {
        self.hud_rows(
            &format!("SELECT {HUDDLE_COLUMNS} FROM hud_huddles WHERE huddle_id = ?1"),
            &[&id.as_str()],
            decode_huddle,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_huddles(&self) -> Result<Vec<Huddle>, MetaError> {
        self.hud_rows(
            &format!("SELECT {HUDDLE_COLUMNS} FROM hud_huddles ORDER BY rowid"),
            &[],
            decode_huddle,
        )
    }

    pub fn list_huddle_participants(
        &self,
        huddle_id: Option<&OpaqueId>,
    ) -> Result<Vec<HuddleParticipant>, MetaError> {
        match huddle_id {
            Some(h) => self.hud_rows(
                &format!("SELECT {PARTICIPANT_COLUMNS} FROM hud_participants WHERE huddle_id = ?1 ORDER BY rowid"),
                &[&h.as_str()],
                decode_participant,
            ),
            None => self.hud_rows(
                &format!("SELECT {PARTICIPANT_COLUMNS} FROM hud_participants ORDER BY rowid"),
                &[],
                decode_participant,
            ),
        }
    }

    pub fn list_huddle_media(
        &self,
        huddle_id: Option<&OpaqueId>,
    ) -> Result<Vec<HuddleMedia>, MetaError> {
        match huddle_id {
            Some(h) => self.hud_rows(
                &format!(
                    "SELECT {MEDIA_COLUMNS} FROM hud_media WHERE huddle_id = ?1 ORDER BY rowid"
                ),
                &[&h.as_str()],
                decode_media,
            ),
            None => self.hud_rows(
                &format!("SELECT {MEDIA_COLUMNS} FROM hud_media ORDER BY rowid"),
                &[],
                decode_media,
            ),
        }
    }

    pub fn list_huddle_proposals(
        &self,
        huddle_id: Option<&OpaqueId>,
    ) -> Result<Vec<HuddleProposal>, MetaError> {
        match huddle_id {
            Some(h) => self.hud_rows(
                &format!("SELECT {PROPOSAL_COLUMNS} FROM hud_proposals WHERE huddle_id = ?1 ORDER BY rowid"),
                &[&h.as_str()],
                decode_proposal,
            ),
            None => self.hud_rows(
                &format!("SELECT {PROPOSAL_COLUMNS} FROM hud_proposals ORDER BY rowid"),
                &[],
                decode_proposal,
            ),
        }
    }

    pub fn list_huddle_receipts(
        &self,
        huddle_id: Option<&OpaqueId>,
    ) -> Result<Vec<HuddleReceipt>, MetaError> {
        match huddle_id {
            Some(h) => self.hud_rows(
                &format!(
                    "SELECT {RECEIPT_COLUMNS} FROM hud_receipts WHERE huddle_id = ?1 ORDER BY rowid"
                ),
                &[&h.as_str()],
                decode_receipt,
            ),
            None => self.hud_rows(
                &format!("SELECT {RECEIPT_COLUMNS} FROM hud_receipts ORDER BY rowid"),
                &[],
                decode_receipt,
            ),
        }
    }

    /// Cross-row invariants, re-verified after restore:
    /// - every huddle names an existing Project in its scope;
    /// - participants, media, proposals and receipts name existing huddles;
    /// - present media names an existing Spec 081 source of the huddle's
    ///   Project with the recorded digest; deleted media has no source row.
    pub fn verify_huddle_consistency(&self) -> Result<(), MetaError> {
        let huddles: HashMap<String, Huddle> = self
            .list_huddles()?
            .into_iter()
            .map(|h| (h.header.id.as_str().to_owned(), h))
            .collect();
        for h in huddles.values() {
            let project = match self.get_project(&h.project_id) {
                Ok(p) => p,
                Err(MetaError::NotFound) => {
                    return Err(corrupt("huddle names a missing project".to_owned()));
                }
                Err(e) => return Err(e),
            };
            if project.header.realm_id != h.header.realm_id
                || project.header.authority_scope_id != h.header.authority_scope_id
            {
                return Err(corrupt("huddle is outside its project's scope".to_owned()));
            }
        }
        let known = |id: &OpaqueId| {
            huddles
                .get(id.as_str())
                .ok_or_else(|| corrupt("row names a missing huddle".to_owned()))
        };
        for p in self.list_huddle_participants(None)? {
            known(&p.huddle_id)?;
        }
        for p in self.list_huddle_proposals(None)? {
            known(&p.huddle_id)?;
        }
        for r in self.list_huddle_receipts(None)? {
            known(&r.huddle_id)?;
        }
        for m in self.list_huddle_media(None)? {
            let h = known(&m.huddle_id)?;
            let source = self.get_audio_source(&m.source_id);
            match (m.state, source) {
                (MediaState::Present, Ok((s, _))) => {
                    if s.project_id != h.project_id || s.content_digest != m.source_digest {
                        return Err(corrupt("huddle media does not match its source".to_owned()));
                    }
                }
                (MediaState::Present, Err(_)) => {
                    return Err(corrupt("present huddle media has no source".to_owned()));
                }
                (MediaState::Deleted, Ok(_)) => {
                    return Err(corrupt(
                        "deleted huddle media still has its source".to_owned(),
                    ));
                }
                (MediaState::Deleted, Err(_)) => {}
            }
        }
        Ok(())
    }

    /// Backup families.
    pub fn huddle_backup_families(
        &self,
    ) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        Ok(vec![
            (
                "hud_huddles",
                v(serde_json::to_value(self.list_huddles()?))?,
            ),
            (
                "hud_participants",
                v(serde_json::to_value(self.list_huddle_participants(None)?))?,
            ),
            (
                "hud_media",
                v(serde_json::to_value(self.list_huddle_media(None)?))?,
            ),
            (
                "hud_proposals",
                v(serde_json::to_value(self.list_huddle_proposals(None)?))?,
            ),
            (
                "hud_receipts",
                v(serde_json::to_value(self.list_huddle_receipts(None)?))?,
            ),
        ])
    }

    pub fn restore_huddle_row(&self, h: &Huddle) -> Result<(), MetaError> {
        h.validate().map_err(|e| corrupt(format!("huddle: {e}")))?;
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO hud_huddles(huddle_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    h.header.id.as_str(),
                    h.project_id.as_str(),
                    revision_i64(h.revision, "huddle")?,
                    to_json(h)?
                ],
            ),
            "huddle",
            h.header.id.as_str(),
        ))
    }

    pub fn restore_huddle_participant_row(&self, p: &HuddleParticipant) -> Result<(), MetaError> {
        p.validate()
            .map_err(|e| corrupt(format!("huddle participant: {e}")))?;
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO hud_participants(participant_id, huddle_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    p.header.id.as_str(),
                    p.huddle_id.as_str(),
                    revision_i64(p.revision, "participant")?,
                    to_json(p)?
                ],
            ),
            "huddle participant",
            p.header.id.as_str(),
        ))
    }

    pub fn restore_huddle_media_row(&self, m: &HuddleMedia) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hud_insert_media_on(self.conn(), m))
    }

    pub fn restore_huddle_proposal_row(&self, p: &HuddleProposal) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hud_write_proposal_on(self.conn(), p, false))
    }

    pub fn restore_huddle_receipt_row(&self, r: &HuddleReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::hud_insert_receipt_on(self.conn(), r))
    }
}

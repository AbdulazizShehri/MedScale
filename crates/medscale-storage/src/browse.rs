//! Governed Browse durable rows (Spec 080, storage schema v9).
//!
//! Same pattern as Spec 079: each row keeps the validated contract value as
//! JSON (`body_json`) plus the columns queries and uniqueness need; reads
//! re-check columns against the body and fail closed on disagreement.
//! Evidence and download bytes live in a `content` BLOB whose SHA-256 must
//! equal the recorded digest on every read.
//!
//! A browse session and everything it produced (evidence, downloads,
//! receipt) are written in one transaction. Restore uses plain `INSERT`.

use std::collections::{HashMap, HashSet};

use medscale_contracts::browse::{
    BrowseAllowlistEntry, BrowseDownloadCandidate, BrowseEvidenceItem, BrowseReceipt,
    BrowseSession, BrowseSessionState, request_digest,
};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v9 DDL, executed inside `begin/finish_migration(9)`.
pub(crate) const V9_DDL: &str = r"
CREATE TABLE IF NOT EXISTS browse_allowlist (
  entry_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  host TEXT NOT NULL,
  path_prefix TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_browse_allowlist_target
  ON browse_allowlist(project_id, host, path_prefix);
CREATE TABLE IF NOT EXISTS browse_sessions (
  session_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_browse_sessions_project
  ON browse_sessions(project_id);
CREATE TABLE IF NOT EXISTS browse_evidence (
  evidence_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  content BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_browse_evidence_session
  ON browse_evidence(session_id);
CREATE TABLE IF NOT EXISTS browse_downloads (
  download_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  content BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_browse_downloads_session
  ON browse_downloads(session_id);
CREATE TABLE IF NOT EXISTS browse_receipts (
  receipt_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_browse_receipts_session
  ON browse_receipts(session_id);
";

/// Evidence row plus its raw bytes (backup form: bytes as lowercase hex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseEvidenceRow {
    pub evidence: BrowseEvidenceItem,
    pub content_hex: String,
}

/// Download candidate plus its quarantined bytes (hex in backups).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowseDownloadRow {
    pub download: BrowseDownloadCandidate,
    pub content_hex: String,
}

/// Everything one browse session writes, in one transaction.
#[derive(Debug, Clone)]
pub struct BrowseSessionCommit {
    pub session: BrowseSession,
    pub evidence: Vec<(BrowseEvidenceItem, Vec<u8>)>,
    pub downloads: Vec<(BrowseDownloadCandidate, Vec<u8>)>,
    pub receipt: BrowseReceipt,
}

fn corrupt(message: String) -> MetaError {
    MetaError::CorruptObjectBody(message)
}

fn is_conflict(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(f, _)
        if f.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

fn map_insert(result: rusqlite::Result<usize>, what: &str, id: &str) -> Result<(), MetaError> {
    match result {
        Ok(_) => Ok(()),
        Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!("duplicate {what} {id}"))),
        Err(e) => Err(MetaError::Sqlite(e)),
    }
}

fn restore_conflict_is_corrupt(result: Result<(), MetaError>) -> Result<(), MetaError> {
    match result {
        Err(MetaError::Conflict(message)) => Err(corrupt(format!("tampered backup: {message}"))),
        other => other,
    }
}

fn revision_to_i64(revision: u64) -> i64 {
    i64::try_from(revision).unwrap_or(i64::MAX)
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

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Strict hex decoding. Checking the characters first keeps a tampered
/// backup from reaching a non-ASCII slice boundary (a panic) or
/// `from_str_radix`'s sign handling.
fn from_hex(hex: &str) -> Result<Vec<u8>, MetaError> {
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(corrupt("bad hex".to_owned()));
    }
    if !hex.len().is_multiple_of(2) {
        return Err(corrupt("odd hex length".to_owned()));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| corrupt("bad hex".to_owned())))
        .collect()
}

fn check_content(
    digest: &DigestSha256,
    length: u64,
    bytes: &[u8],
    what: &str,
) -> Result<(), MetaError> {
    if &DigestSha256::of(bytes) != digest || bytes.len() as u64 != length {
        return Err(corrupt(format!("{what} content does not match its digest")));
    }
    Ok(())
}

fn decode_allowlist(row: &rusqlite::Row<'_>) -> Result<BrowseAllowlistEntry, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let host: String = row.get(2)?;
    let path_prefix: String = row.get(3)?;
    let revision: i64 = row.get(4)?;
    let body: String = row.get(5)?;
    let value: BrowseAllowlistEntry = from_json(&body, "browse allowlist entry")?;
    check_column(&id, value.header.id.as_str(), "browse allowlist entry")?;
    check_column(
        &project_id,
        value.project_id.as_str(),
        "browse allowlist entry",
    )?;
    check_column(&host, &value.host, "browse allowlist entry")?;
    check_column(&path_prefix, &value.path_prefix, "browse allowlist entry")?;
    if revision != revision_to_i64(value.revision) {
        return Err(corrupt("browse allowlist revision disagrees".to_owned()));
    }
    value
        .validate()
        .map_err(|e| corrupt(format!("browse allowlist entry: {e}")))?;
    Ok(value)
}

fn decode_session(row: &rusqlite::Row<'_>) -> Result<BrowseSession, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let value: BrowseSession = from_json(&body, "browse session")?;
    check_column(&id, value.header.id.as_str(), "browse session")?;
    check_column(&project_id, value.project_id.as_str(), "browse session")?;
    if revision != revision_to_i64(value.revision) {
        return Err(corrupt("browse session revision disagrees".to_owned()));
    }
    value
        .validate()
        .map_err(|e| corrupt(format!("browse session: {e}")))?;
    Ok(value)
}

fn decode_evidence(row: &rusqlite::Row<'_>) -> Result<(BrowseEvidenceItem, Vec<u8>), MetaError> {
    let id: String = row.get(0)?;
    let session_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let content: Vec<u8> = row.get(3)?;
    let value: BrowseEvidenceItem = from_json(&body, "browse evidence")?;
    check_column(&id, value.header.id.as_str(), "browse evidence")?;
    check_column(&session_id, value.session_id.as_str(), "browse evidence")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("browse evidence: {e}")))?;
    check_content(
        &value.content_digest,
        value.byte_length,
        &content,
        "browse evidence",
    )?;
    Ok((value, content))
}

fn decode_download(
    row: &rusqlite::Row<'_>,
) -> Result<(BrowseDownloadCandidate, Vec<u8>), MetaError> {
    let id: String = row.get(0)?;
    let session_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let content: Vec<u8> = row.get(3)?;
    let value: BrowseDownloadCandidate = from_json(&body, "browse download")?;
    check_column(&id, value.header.id.as_str(), "browse download")?;
    check_column(&session_id, value.session_id.as_str(), "browse download")?;
    check_content(
        &value.content_digest,
        value.byte_length,
        &content,
        "browse download",
    )?;
    Ok((value, content))
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<BrowseReceipt, MetaError> {
    let id: String = row.get(0)?;
    let session_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let value: BrowseReceipt = from_json(&body, "browse receipt")?;
    check_column(&id, value.header.id.as_str(), "browse receipt")?;
    check_column(&session_id, value.session_id.as_str(), "browse receipt")?;
    Ok(value)
}

const ALLOWLIST_COLUMNS: &str = "entry_id, project_id, host, path_prefix, revision, body_json";
const SESSION_COLUMNS: &str = "session_id, project_id, revision, body_json";
const EVIDENCE_COLUMNS: &str = "evidence_id, session_id, body_json, content";
const DOWNLOAD_COLUMNS: &str = "download_id, session_id, body_json, content";
const RECEIPT_COLUMNS: &str = "receipt_id, session_id, body_json";

impl SqliteMetaStore {
    fn browse_rows<T>(
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

    /// Allocates one `prefix-N` Browse id from a durable sequence.
    pub fn alloc_browse_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("browse_seq_{prefix}");
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

    // -- allowlist -------------------------------------------------------------

    pub fn insert_browse_allowlist_entry(
        &self,
        value: &BrowseAllowlistEntry,
    ) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("browse allowlist entry: {e}")))?;
        map_insert(
            self.conn().execute(
                "INSERT INTO browse_allowlist(entry_id, project_id, host, path_prefix, revision, body_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    value.host,
                    value.path_prefix,
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "browse allowlist entry",
            value.header.id.as_str(),
        )
    }

    pub fn get_browse_allowlist_entry(
        &self,
        id: &OpaqueId,
    ) -> Result<BrowseAllowlistEntry, MetaError> {
        self.browse_rows(
            &format!("SELECT {ALLOWLIST_COLUMNS} FROM browse_allowlist WHERE entry_id = ?1"),
            &[&id.as_str()],
            decode_allowlist,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_browse_allowlist(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<BrowseAllowlistEntry>, MetaError> {
        self.browse_rows(
            &format!(
                "SELECT {ALLOWLIST_COLUMNS} FROM browse_allowlist WHERE project_id = ?1 ORDER BY entry_id LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_allowlist,
        )
    }

    /// Disables an entry (CAS). Disabling is terminal for that entry.
    pub fn disable_browse_allowlist_entry(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<BrowseAllowlistEntry, MetaError> {
        let mut value = self.get_browse_allowlist_entry(id)?;
        if value.revision != expected_revision {
            return Err(MetaError::Conflict(format!(
                "stale revision: expected {expected_revision}, current is {}",
                value.revision
            )));
        }
        if !value.enabled {
            return Err(MetaError::Conflict("entry already disabled".to_owned()));
        }
        value.enabled = false;
        value.revision = expected_revision + 1;
        let updated = self.conn().execute(
            "UPDATE browse_allowlist SET revision = ?1, body_json = ?2 WHERE entry_id = ?3 AND revision = ?4",
            params![
                revision_to_i64(value.revision),
                to_json(&value)?,
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict(
                "concurrent allowlist update".to_owned(),
            ));
        }
        Ok(value)
    }

    // -- sessions --------------------------------------------------------------

    fn browse_insert_session_on(
        conn: &rusqlite::Connection,
        s: &BrowseSession,
    ) -> Result<(), MetaError> {
        s.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("browse session: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO browse_sessions(session_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    s.header.id.as_str(),
                    s.project_id.as_str(),
                    revision_to_i64(s.revision),
                    to_json(s)?,
                ],
            ),
            "browse session",
            s.header.id.as_str(),
        )
    }

    fn browse_insert_evidence_on(
        conn: &rusqlite::Connection,
        e: &BrowseEvidenceItem,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        e.validate()
            .map_err(|m| MetaError::UnsupportedSchema(format!("browse evidence: {m}")))?;
        check_content(&e.content_digest, e.byte_length, bytes, "browse evidence")
            .map_err(|_| MetaError::UnsupportedSchema("evidence digest mismatch".to_owned()))?;
        map_insert(
            conn.execute(
                "INSERT INTO browse_evidence(evidence_id, session_id, body_json, content) VALUES (?1, ?2, ?3, ?4)",
                params![e.header.id.as_str(), e.session_id.as_str(), to_json(e)?, bytes],
            ),
            "browse evidence",
            e.header.id.as_str(),
        )
    }

    fn browse_insert_download_on(
        conn: &rusqlite::Connection,
        d: &BrowseDownloadCandidate,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        check_content(&d.content_digest, d.byte_length, bytes, "browse download")
            .map_err(|_| MetaError::UnsupportedSchema("download digest mismatch".to_owned()))?;
        map_insert(
            conn.execute(
                "INSERT INTO browse_downloads(download_id, session_id, body_json, content) VALUES (?1, ?2, ?3, ?4)",
                params![d.header.id.as_str(), d.session_id.as_str(), to_json(d)?, bytes],
            ),
            "browse download",
            d.header.id.as_str(),
        )
    }

    fn browse_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &BrowseReceipt,
    ) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO browse_receipts(receipt_id, session_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.session_id.as_str(), to_json(r)?],
            ),
            "browse receipt",
            r.header.id.as_str(),
        )
    }

    /// Writes one session with its evidence, downloads and receipt, all or
    /// nothing.
    pub fn commit_browse_session(&self, commit: &BrowseSessionCommit) -> Result<(), MetaError> {
        let session_id = &commit.session.header.id;
        let receipt = &commit.receipt;
        if &receipt.session_id != session_id
            || receipt.final_state != commit.session.state
            || receipt.evidence_ids.len() != commit.evidence.len()
            || receipt.download_ids.len() != commit.downloads.len()
        {
            return Err(MetaError::UnsupportedSchema(
                "receipt does not match its session".to_owned(),
            ));
        }
        let tx = self.conn().unchecked_transaction()?;
        Self::browse_insert_session_on(&tx, &commit.session)?;
        for (e, bytes) in &commit.evidence {
            if &e.session_id != session_id {
                return Err(MetaError::UnsupportedSchema(
                    "evidence names another session".to_owned(),
                ));
            }
            Self::browse_insert_evidence_on(&tx, e, bytes)?;
        }
        for (d, bytes) in &commit.downloads {
            if &d.session_id != session_id {
                return Err(MetaError::UnsupportedSchema(
                    "download names another session".to_owned(),
                ));
            }
            Self::browse_insert_download_on(&tx, d, bytes)?;
        }
        Self::browse_insert_receipt_on(&tx, receipt)?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_browse_session(&self, id: &OpaqueId) -> Result<BrowseSession, MetaError> {
        self.browse_rows(
            &format!("SELECT {SESSION_COLUMNS} FROM browse_sessions WHERE session_id = ?1"),
            &[&id.as_str()],
            decode_session,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_browse_sessions(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<BrowseSession>, MetaError> {
        self.browse_rows(
            &format!(
                "SELECT {SESSION_COLUMNS} FROM browse_sessions WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_session,
        )
    }

    /// Cancels a session awaiting human takeover (CAS).
    pub fn cancel_browse_session(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<BrowseSession, MetaError> {
        let mut s = self.get_browse_session(id)?;
        if s.revision != expected_revision {
            return Err(MetaError::Conflict(format!(
                "stale revision: expected {expected_revision}, current is {}",
                s.revision
            )));
        }
        if !s.state.can_transition_to(BrowseSessionState::Cancelled) {
            return Err(MetaError::Conflict("session is final".to_owned()));
        }
        s.state = BrowseSessionState::Cancelled;
        s.revision = expected_revision + 1;
        let updated = self.conn().execute(
            "UPDATE browse_sessions SET revision = ?1, body_json = ?2 WHERE session_id = ?3 AND revision = ?4",
            params![
                revision_to_i64(s.revision),
                to_json(&s)?,
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent session update".to_owned()));
        }
        Ok(s)
    }

    pub fn list_browse_evidence(
        &self,
        session_id: &OpaqueId,
    ) -> Result<Vec<(BrowseEvidenceItem, Vec<u8>)>, MetaError> {
        self.browse_rows(
            &format!("SELECT {EVIDENCE_COLUMNS} FROM browse_evidence WHERE session_id = ?1 ORDER BY evidence_id"),
            &[&session_id.as_str()],
            decode_evidence,
        )
    }

    pub fn list_browse_downloads(
        &self,
        session_id: &OpaqueId,
    ) -> Result<Vec<(BrowseDownloadCandidate, Vec<u8>)>, MetaError> {
        self.browse_rows(
            &format!("SELECT {DOWNLOAD_COLUMNS} FROM browse_downloads WHERE session_id = ?1 ORDER BY download_id"),
            &[&session_id.as_str()],
            decode_download,
        )
    }

    pub fn get_browse_receipt_for_session(
        &self,
        session_id: &OpaqueId,
    ) -> Result<BrowseReceipt, MetaError> {
        self.browse_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM browse_receipts WHERE session_id = ?1"),
            &[&session_id.as_str()],
            decode_receipt,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    // -- full scans, consistency, restore --------------------------------------

    pub fn list_all_browse_allowlist(&self) -> Result<Vec<BrowseAllowlistEntry>, MetaError> {
        self.browse_rows(
            &format!("SELECT {ALLOWLIST_COLUMNS} FROM browse_allowlist ORDER BY entry_id"),
            &[],
            decode_allowlist,
        )
    }

    pub fn list_all_browse_sessions(&self) -> Result<Vec<BrowseSession>, MetaError> {
        self.browse_rows(
            &format!("SELECT {SESSION_COLUMNS} FROM browse_sessions ORDER BY rowid"),
            &[],
            decode_session,
        )
    }

    pub fn list_all_browse_evidence(&self) -> Result<Vec<BrowseEvidenceRow>, MetaError> {
        Ok(self
            .browse_rows(
                &format!("SELECT {EVIDENCE_COLUMNS} FROM browse_evidence ORDER BY evidence_id"),
                &[],
                decode_evidence,
            )?
            .into_iter()
            .map(|(evidence, bytes)| BrowseEvidenceRow {
                evidence,
                content_hex: to_hex(&bytes),
            })
            .collect())
    }

    pub fn list_all_browse_downloads(&self) -> Result<Vec<BrowseDownloadRow>, MetaError> {
        Ok(self
            .browse_rows(
                &format!("SELECT {DOWNLOAD_COLUMNS} FROM browse_downloads ORDER BY download_id"),
                &[],
                decode_download,
            )?
            .into_iter()
            .map(|(download, bytes)| BrowseDownloadRow {
                download,
                content_hex: to_hex(&bytes),
            })
            .collect())
    }

    pub fn list_all_browse_receipts(&self) -> Result<Vec<BrowseReceipt>, MetaError> {
        self.browse_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM browse_receipts ORDER BY rowid"),
            &[],
            decode_receipt,
        )
    }

    /// A Browse row must name an existing Project in its own realm and scope,
    /// as the Core write path requires.
    fn browse_project_in_scope(
        &self,
        project_id: &OpaqueId,
        header: &ObjectHeader,
        what: &str,
    ) -> Result<(), MetaError> {
        let project = match self.get_project(project_id) {
            Ok(project) => project,
            Err(MetaError::NotFound) => {
                return Err(corrupt(format!("{what} names a missing project")));
            }
            Err(other) => return Err(other),
        };
        if project.header.realm_id != header.realm_id
            || project.header.authority_scope_id != header.authority_scope_id
        {
            return Err(corrupt(format!("{what} is outside its project's scope")));
        }
        Ok(())
    }

    /// Cross-row invariants, re-verified after restore:
    /// - every allowlist entry and session names an existing Project in the
    ///   same realm and scope;
    /// - every session has exactly one receipt, whose route, request digest,
    ///   final state, step count and evidence/download ids match the stored
    ///   rows;
    /// - every evidence/download row belongs to an existing session.
    pub fn verify_browse_consistency(&self) -> Result<(), MetaError> {
        for entry in self.list_all_browse_allowlist()? {
            self.browse_project_in_scope(&entry.project_id, &entry.header, "allowlist entry")?;
        }
        let sessions: HashMap<String, BrowseSession> = self
            .list_all_browse_sessions()?
            .into_iter()
            .map(|s| (s.header.id.as_str().to_owned(), s))
            .collect();
        for session in sessions.values() {
            self.browse_project_in_scope(&session.project_id, &session.header, "browse session")?;
        }
        let mut evidence: HashMap<String, HashSet<String>> = HashMap::new();
        for row in self.list_all_browse_evidence()? {
            let sid = row.evidence.session_id.as_str().to_owned();
            if !sessions.contains_key(&sid) {
                return Err(corrupt("evidence names a missing session".to_owned()));
            }
            evidence
                .entry(sid)
                .or_default()
                .insert(row.evidence.header.id.as_str().to_owned());
        }
        let mut downloads: HashMap<String, HashSet<String>> = HashMap::new();
        for row in self.list_all_browse_downloads()? {
            let sid = row.download.session_id.as_str().to_owned();
            if !sessions.contains_key(&sid) {
                return Err(corrupt("download names a missing session".to_owned()));
            }
            downloads
                .entry(sid)
                .or_default()
                .insert(row.download.header.id.as_str().to_owned());
        }
        let mut with_receipt = HashSet::new();
        for r in self.list_all_browse_receipts()? {
            let sid = r.session_id.as_str().to_owned();
            let session = sessions
                .get(&sid)
                .ok_or_else(|| corrupt("receipt names a missing session".to_owned()))?;
            // A cancelled session was awaiting takeover when its receipt was written.
            let state_ok = r.final_state == session.state
                || (session.state == BrowseSessionState::Cancelled
                    && r.final_state == BrowseSessionState::AwaitingHumanTakeover);
            let ids = |v: &[OpaqueId]| {
                v.iter()
                    .map(|i| i.as_str().to_owned())
                    .collect::<HashSet<_>>()
            };
            if !state_ok
                || r.route != session.route
                || r.request_digest != request_digest(&session.request)
                || r.step_count as usize != session.steps.len()
                || r.project_id != session.project_id
                || ids(&r.evidence_ids) != evidence.get(&sid).cloned().unwrap_or_default()
                || ids(&r.download_ids) != downloads.get(&sid).cloned().unwrap_or_default()
            {
                return Err(corrupt(format!(
                    "receipt for session {sid} disagrees with its rows"
                )));
            }
            with_receipt.insert(sid);
        }
        if with_receipt.len() != sessions.len() {
            return Err(corrupt("a browse session has no receipt".to_owned()));
        }
        Ok(())
    }

    pub fn restore_browse_allowlist_row(
        &self,
        value: &BrowseAllowlistEntry,
    ) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| corrupt(format!("browse allowlist entry: {e}")))?;
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO browse_allowlist(entry_id, project_id, host, path_prefix, revision, body_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    value.host,
                    value.path_prefix,
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "browse allowlist entry",
            value.header.id.as_str(),
        ))
    }

    pub fn restore_browse_session_row(&self, value: &BrowseSession) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| corrupt(format!("browse session: {e}")))?;
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO browse_sessions(session_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "browse session",
            value.header.id.as_str(),
        ))
    }

    pub fn restore_browse_evidence_row(&self, row: &BrowseEvidenceRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(
            Self::browse_insert_evidence_on(self.conn(), &row.evidence, &bytes).map_err(
                |e| match e {
                    MetaError::UnsupportedSchema(m) => corrupt(m),
                    other => other,
                },
            ),
        )
    }

    pub fn restore_browse_download_row(&self, row: &BrowseDownloadRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(
            Self::browse_insert_download_on(self.conn(), &row.download, &bytes).map_err(
                |e| match e {
                    MetaError::UnsupportedSchema(m) => corrupt(m),
                    other => other,
                },
            ),
        )
    }

    pub fn restore_browse_receipt_row(&self, value: &BrowseReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::browse_insert_receipt_on(self.conn(), value))
    }
}

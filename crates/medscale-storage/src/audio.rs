//! AudioFlow durable rows (Spec 081, storage schema v10).
//!
//! Same pattern as Specs 079/080: each row keeps the validated contract
//! value as JSON (`body_json`) plus the columns queries and uniqueness need;
//! reads re-check columns against the body and fail closed on disagreement.
//! Source audio (a WAV file) lives in a `content` BLOB whose SHA-256 must
//! equal the recorded digest on every read. Capture frames live in
//! `audio_capture_chunks` only while a capture is open; stopping moves them
//! into one source, cancelling or interruption deletes them.

use std::collections::{HashMap, HashSet};

use medscale_contracts::audio::{
    AudioSession, AudioSource, AudioSourceKind, CaptureState, TranscriptOrigin, TranscriptReceipt,
    TranscriptRevision,
};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v10 DDL, executed inside `begin/finish_migration(10)`.
pub(crate) const V10_DDL: &str = r"
CREATE TABLE IF NOT EXISTS audio_sources (
  source_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  content BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audio_sources_project
  ON audio_sources(project_id);
CREATE TABLE IF NOT EXISTS audio_capture_sessions (
  session_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audio_capture_sessions_project
  ON audio_capture_sessions(project_id);
CREATE TABLE IF NOT EXISTS audio_capture_chunks (
  session_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  content BLOB NOT NULL,
  PRIMARY KEY (session_id, seq)
);
CREATE TABLE IF NOT EXISTS audio_transcripts (
  revision_id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  revision_no INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_audio_transcripts_source_no
  ON audio_transcripts(source_id, revision_no);
CREATE TABLE IF NOT EXISTS audio_transcript_receipts (
  receipt_id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  revision_id TEXT,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audio_transcript_receipts_source
  ON audio_transcript_receipts(source_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_audio_transcript_receipts_revision
  ON audio_transcript_receipts(revision_id) WHERE revision_id IS NOT NULL;
";

/// Source row plus its WAV bytes (backup form: bytes as lowercase hex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSourceRow {
    pub source: AudioSource,
    pub content_hex: String,
}

/// One open-capture chunk (backup form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioChunkRow {
    pub session_id: OpaqueId,
    pub seq: u32,
    pub content_hex: String,
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

fn to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
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

fn check_content(source: &AudioSource, bytes: &[u8]) -> Result<(), MetaError> {
    if DigestSha256::of(bytes) != source.content_digest || bytes.len() as u64 != source.byte_length
    {
        return Err(corrupt(
            "audio source content does not match its digest".to_owned(),
        ));
    }
    Ok(())
}

const SOURCE_COLUMNS: &str = "source_id, project_id, body_json, content";
const SESSION_COLUMNS: &str = "session_id, project_id, revision, body_json";
const TRANSCRIPT_COLUMNS: &str = "revision_id, source_id, project_id, revision_no, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, source_id, revision_id, body_json";

fn decode_source(row: &rusqlite::Row<'_>) -> Result<(AudioSource, Vec<u8>), MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let content: Vec<u8> = row.get(3)?;
    let value: AudioSource = from_json(&body, "audio source")?;
    check_column(&id, value.header.id.as_str(), "audio source")?;
    check_column(&project_id, value.project_id.as_str(), "audio source")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("audio source: {e}")))?;
    check_content(&value, &content)?;
    Ok((value, content))
}

fn decode_session(row: &rusqlite::Row<'_>) -> Result<AudioSession, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let value: AudioSession = from_json(&body, "capture session")?;
    check_column(&id, value.header.id.as_str(), "capture session")?;
    check_column(&project_id, value.project_id.as_str(), "capture session")?;
    if revision != to_i64(value.revision) {
        return Err(corrupt("capture session revision disagrees".to_owned()));
    }
    value
        .validate()
        .map_err(|e| corrupt(format!("capture session: {e}")))?;
    Ok(value)
}

fn decode_transcript(row: &rusqlite::Row<'_>) -> Result<TranscriptRevision, MetaError> {
    let id: String = row.get(0)?;
    let source_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let revision_no: i64 = row.get(3)?;
    let body: String = row.get(4)?;
    let value: TranscriptRevision = from_json(&body, "transcript revision")?;
    check_column(&id, value.header.id.as_str(), "transcript revision")?;
    check_column(&source_id, value.source_id.as_str(), "transcript revision")?;
    check_column(
        &project_id,
        value.project_id.as_str(),
        "transcript revision",
    )?;
    if revision_no != i64::from(value.revision_no) {
        return Err(corrupt("transcript revision number disagrees".to_owned()));
    }
    Ok(value)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<TranscriptReceipt, MetaError> {
    let id: String = row.get(0)?;
    let source_id: String = row.get(1)?;
    let revision_id: Option<String> = row.get(2)?;
    let body: String = row.get(3)?;
    let value: TranscriptReceipt = from_json(&body, "transcript receipt")?;
    check_column(&id, value.header.id.as_str(), "transcript receipt")?;
    check_column(&source_id, value.source_id.as_str(), "transcript receipt")?;
    if revision_id.as_deref() != value.transcript_revision_id.as_ref().map(OpaqueId::as_str) {
        return Err(corrupt(
            "transcript receipt row columns disagree with its body".to_owned(),
        ));
    }
    value
        .validate()
        .map_err(|e| corrupt(format!("transcript receipt: {e}")))?;
    Ok(value)
}

impl SqliteMetaStore {
    fn audio_rows<T>(
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

    /// Allocates one `prefix-N` AudioFlow id from a durable sequence.
    pub fn alloc_audio_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("audio_seq_{prefix}");
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

    // -- sources ---------------------------------------------------------------

    fn audio_insert_source_on(
        conn: &rusqlite::Connection,
        source: &AudioSource,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        source
            .validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("audio source: {e}")))?;
        check_content(source, bytes)
            .map_err(|_| MetaError::UnsupportedSchema("audio source digest mismatch".to_owned()))?;
        map_insert(
            conn.execute(
                "INSERT INTO audio_sources(source_id, project_id, body_json, content) VALUES (?1, ?2, ?3, ?4)",
                params![
                    source.header.id.as_str(),
                    source.project_id.as_str(),
                    to_json(source)?,
                    bytes
                ],
            ),
            "audio source",
            source.header.id.as_str(),
        )
    }

    /// Inserts an imported source. Capture sources are written only by
    /// `stop_capture_session`.
    pub fn insert_imported_audio_source(
        &self,
        source: &AudioSource,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        if source.kind != AudioSourceKind::ImportedFile {
            return Err(MetaError::UnsupportedSchema(
                "capture sources come only from a stopped capture".to_owned(),
            ));
        }
        Self::audio_insert_source_on(self.conn(), source, bytes)
    }

    /// Source metadata plus its digest-checked WAV bytes.
    pub fn get_audio_source(&self, id: &OpaqueId) -> Result<(AudioSource, Vec<u8>), MetaError> {
        self.audio_rows(
            &format!("SELECT {SOURCE_COLUMNS} FROM audio_sources WHERE source_id = ?1"),
            &[&id.as_str()],
            decode_source,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    /// Remove one unreferenced imported WAV from the *current* vault only.
    ///
    /// This intentionally does not expose an end-user erasure action. A future
    /// authenticated Core command must apply policy, collect explicit consent
    /// and record retention evidence before calling it. Existing backups,
    /// snapshots, SQLite WAL pages and physical storage are NOT wiped.
    ///
    /// An imported source with transcript revisions, even denied-route audit
    /// receipts, or a capture-session reference must never be removed. The
    /// checks and row removal share a single SQLite transaction; any refusal
    /// rolls back without modifying another source or its provenance.
    pub fn remove_unreferenced_imported_audio_source(
        &self,
        project_id: &OpaqueId,
        source_id: &OpaqueId,
        expected_digest: &DigestSha256,
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let row: Option<(String, Vec<u8>)> = tx
            .query_row(
                "SELECT body_json, content FROM audio_sources WHERE source_id = ?1",
                params![source_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (body, bytes) = row.ok_or(MetaError::NotFound)?;
        let source: AudioSource = from_json(&body, "audio source")?;
        source
            .validate()
            .map_err(|e| corrupt(format!("audio source: {e}")))?;
        check_content(&source, &bytes)?;
        if source.header.id != *source_id || source.project_id != *project_id {
            return Err(MetaError::NotFound);
        }
        if source.content_digest != *expected_digest {
            return Err(MetaError::Conflict(
                "audio source content has changed since confirmation".to_owned(),
            ));
        }
        if source.kind != AudioSourceKind::ImportedFile || source.capture_session_id.is_some() {
            return Err(MetaError::Conflict(
                "capture recordings require a separate retention protocol".to_owned(),
            ));
        }

        let linked: i64 = tx.query_row(
            "SELECT (SELECT COUNT(*) FROM audio_transcripts WHERE source_id = ?1)
                  + (SELECT COUNT(*) FROM audio_transcript_receipts WHERE source_id = ?1)",
            params![source_id.as_str()],
            |row| row.get(0),
        )?;
        if linked != 0 {
            return Err(MetaError::Conflict(
                "audio recording has transcript revisions or immutable receipts".to_owned(),
            ));
        }

        // Capture sessions store their source link inside validated JSON,
        // rather than a separate indexed column. Check every session and
        // refuse removal if any session references this source.
        {
            let mut stmt = tx.prepare("SELECT body_json FROM audio_capture_sessions")?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let body: String = row.get(0)?;
                let session: AudioSession = from_json(&body, "capture session")?;
                if session.source_id.as_ref() == Some(source_id) {
                    return Err(MetaError::Conflict(
                        "audio recording is still referenced by a capture session".to_owned(),
                    ));
                }
            }
        }

        let removed = tx.execute(
            "DELETE FROM audio_sources
             WHERE source_id = ?1 AND project_id = ?2 AND body_json = ?3",
            params![source_id.as_str(), project_id.as_str(), body],
        )?;
        if removed != 1 {
            return Err(MetaError::Conflict(
                "audio source changed before removal".to_owned(),
            ));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_audio_sources(&self, project_id: &OpaqueId) -> Result<Vec<AudioSource>, MetaError> {
        Ok(self
            .audio_rows(
                &format!(
                    "SELECT {SOURCE_COLUMNS} FROM audio_sources WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
                ),
                &[&project_id.as_str()],
                decode_source,
            )?
            .into_iter()
            .map(|(s, _)| s)
            .collect())
    }

    pub fn list_all_audio_sources(&self) -> Result<Vec<AudioSourceRow>, MetaError> {
        Ok(self
            .audio_rows(
                &format!("SELECT {SOURCE_COLUMNS} FROM audio_sources ORDER BY rowid"),
                &[],
                decode_source,
            )?
            .into_iter()
            .map(|(source, bytes)| AudioSourceRow {
                source,
                content_hex: to_hex(&bytes),
            })
            .collect())
    }

    // -- capture sessions ------------------------------------------------------

    fn audio_insert_session_on(
        conn: &rusqlite::Connection,
        s: &AudioSession,
    ) -> Result<(), MetaError> {
        s.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("capture session: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO audio_capture_sessions(session_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    s.header.id.as_str(),
                    s.project_id.as_str(),
                    to_i64(s.revision),
                    to_json(s)?
                ],
            ),
            "capture session",
            s.header.id.as_str(),
        )
    }

    pub fn insert_capture_session(&self, s: &AudioSession) -> Result<(), MetaError> {
        if s.state != CaptureState::Recording || s.captured_bytes != 0 || s.chunk_count != 0 {
            return Err(MetaError::UnsupportedSchema(
                "a capture session starts empty and recording".to_owned(),
            ));
        }
        Self::audio_insert_session_on(self.conn(), s)
    }

    pub fn get_capture_session(&self, id: &OpaqueId) -> Result<AudioSession, MetaError> {
        Self::get_capture_session_on(self.conn(), id)
    }

    fn get_capture_session_on(
        conn: &rusqlite::Connection,
        id: &OpaqueId,
    ) -> Result<AudioSession, MetaError> {
        conn.query_row(
            &format!("SELECT {SESSION_COLUMNS} FROM audio_capture_sessions WHERE session_id = ?1"),
            params![id.as_str()],
            |row| Ok(decode_session(row)),
        )
        .optional()?
        .ok_or(MetaError::NotFound)?
    }

    pub fn list_capture_sessions(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<AudioSession>, MetaError> {
        self.audio_rows(
            &format!(
                "SELECT {SESSION_COLUMNS} FROM audio_capture_sessions WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_session,
        )
    }

    pub fn list_all_capture_sessions(&self) -> Result<Vec<AudioSession>, MetaError> {
        self.audio_rows(
            &format!("SELECT {SESSION_COLUMNS} FROM audio_capture_sessions ORDER BY rowid"),
            &[],
            decode_session,
        )
    }

    fn audio_update_session_on(
        conn: &rusqlite::Connection,
        next: &AudioSession,
        expected_revision: u64,
    ) -> Result<(), MetaError> {
        next.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("capture session: {e}")))?;
        let updated = conn.execute(
            "UPDATE audio_capture_sessions SET revision = ?1, body_json = ?2 WHERE session_id = ?3 AND revision = ?4",
            params![
                to_i64(next.revision),
                to_json(next)?,
                next.header.id.as_str(),
                to_i64(expected_revision)
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict(
                "concurrent capture session update".to_owned(),
            ));
        }
        Ok(())
    }

    /// Loads a session for a CAS mutation.
    fn capture_for_update(
        conn: &rusqlite::Connection,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<AudioSession, MetaError> {
        let current = Self::get_capture_session_on(conn, id)?;
        current
            .check_mutation(expected_revision)
            .map_err(MetaError::Conflict)?;
        Ok(current)
    }

    /// Appends PCM frames to a recording session (CAS on revision).
    pub fn append_capture_chunk(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
        frames: &[u8],
    ) -> Result<AudioSession, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let mut next = Self::capture_for_update(&tx, id, expected_revision)?;
        if next.state != CaptureState::Recording {
            return Err(MetaError::Conflict(format!(
                "capture is {}, not recording",
                next.state.as_str()
            )));
        }
        next.chunk_count += 1;
        next.captured_bytes += frames.len() as u64;
        next.revision = expected_revision + 1;
        tx.execute(
            "INSERT INTO audio_capture_chunks(session_id, seq, content) VALUES (?1, ?2, ?3)",
            params![id.as_str(), next.chunk_count, frames],
        )?;
        Self::audio_update_session_on(&tx, &next, expected_revision)?;
        tx.commit()?;
        Ok(next)
    }

    /// Captured frames in order (open sessions only).
    pub fn capture_chunks(&self, id: &OpaqueId) -> Result<Vec<Vec<u8>>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT seq, content FROM audio_capture_chunks WHERE session_id = ?1 ORDER BY seq",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let seq: i64 = row.get(0)?;
            if seq != i64::try_from(out.len() + 1).unwrap_or(i64::MAX) {
                return Err(corrupt("capture chunks are not contiguous".to_owned()));
            }
            out.push(row.get::<_, Vec<u8>>(1)?);
        }
        Ok(out)
    }

    pub fn list_all_capture_chunks(&self) -> Result<Vec<AudioChunkRow>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT session_id, seq, content FROM audio_capture_chunks ORDER BY session_id, seq",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let session_id: String = row.get(0)?;
            let seq: i64 = row.get(1)?;
            let content: Vec<u8> = row.get(2)?;
            out.push(AudioChunkRow {
                session_id: OpaqueId::new(session_id),
                seq: u32::try_from(seq).map_err(|_| corrupt("bad chunk seq".to_owned()))?,
                content_hex: to_hex(&content),
            });
        }
        Ok(out)
    }

    /// Pause, resume, cancel or mark interrupted (CAS). Leaving the open
    /// states deletes every captured chunk in the same transaction.
    pub fn transition_capture_session(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
        to: CaptureState,
    ) -> Result<AudioSession, MetaError> {
        if to == CaptureState::Stopped {
            return Err(MetaError::UnsupportedSchema(
                "stopping goes through stop_capture_session".to_owned(),
            ));
        }
        let tx = self.conn().unchecked_transaction()?;
        let mut next = Self::capture_for_update(&tx, id, expected_revision)?;
        if !next.state.can_become(to) {
            return Err(MetaError::Conflict(format!(
                "capture cannot go from {} to {}",
                next.state.as_str(),
                to.as_str()
            )));
        }
        next.state = to;
        next.revision = expected_revision + 1;
        if !to.is_open() {
            tx.execute(
                "DELETE FROM audio_capture_chunks WHERE session_id = ?1",
                params![id.as_str()],
            )?;
        }
        Self::audio_update_session_on(&tx, &next, expected_revision)?;
        tx.commit()?;
        Ok(next)
    }

    /// Stops a capture: the source (built from exactly the stored chunks by
    /// the caller) is inserted, the session names it, and the chunks are
    /// deleted, all in one transaction.
    pub fn stop_capture_session(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
        source: &AudioSource,
        wav: &[u8],
    ) -> Result<AudioSession, MetaError> {
        if source.kind != AudioSourceKind::Capture || source.capture_session_id.as_ref() != Some(id)
        {
            return Err(MetaError::UnsupportedSchema(
                "a capture source names its own session".to_owned(),
            ));
        }
        let tx = self.conn().unchecked_transaction()?;
        let mut next = Self::capture_for_update(&tx, id, expected_revision)?;
        if !next.state.can_become(CaptureState::Stopped) {
            return Err(MetaError::Conflict(format!(
                "capture cannot stop from {}",
                next.state.as_str()
            )));
        }
        if source.project_id != next.project_id || source.format != next.format {
            return Err(MetaError::UnsupportedSchema(
                "capture source disagrees with its session".to_owned(),
            ));
        }
        Self::audio_insert_source_on(&tx, source, wav)?;
        next.state = CaptureState::Stopped;
        next.source_id = Some(source.header.id.clone());
        next.revision = expected_revision + 1;
        tx.execute(
            "DELETE FROM audio_capture_chunks WHERE session_id = ?1",
            params![id.as_str()],
        )?;
        Self::audio_update_session_on(&tx, &next, expected_revision)?;
        tx.commit()?;
        Ok(next)
    }

    // -- transcripts -----------------------------------------------------------

    fn audio_insert_transcript_on(
        conn: &rusqlite::Connection,
        t: &TranscriptRevision,
    ) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO audio_transcripts(revision_id, source_id, project_id, revision_no, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    t.header.id.as_str(),
                    t.source_id.as_str(),
                    t.project_id.as_str(),
                    t.revision_no,
                    to_json(t)?
                ],
            ),
            "transcript revision",
            t.header.id.as_str(),
        )
    }

    fn audio_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &TranscriptReceipt,
    ) -> Result<(), MetaError> {
        r.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("transcript receipt: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO audio_transcript_receipts(receipt_id, source_id, revision_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    r.header.id.as_str(),
                    r.source_id.as_str(),
                    r.transcript_revision_id.as_ref().map(OpaqueId::as_str),
                    to_json(r)?
                ],
            ),
            "transcript receipt",
            r.header.id.as_str(),
        )
    }

    /// Checks a revision against its stored source and earlier revisions.
    fn check_transcript_on(
        conn: &rusqlite::Connection,
        t: &TranscriptRevision,
    ) -> Result<(), MetaError> {
        let source: Option<String> = conn
            .query_row(
                "SELECT body_json FROM audio_sources WHERE source_id = ?1",
                params![t.source_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let source: AudioSource = match source {
            Some(body) => from_json(&body, "audio source")?,
            None => {
                return Err(MetaError::UnsupportedSchema(
                    "transcript names a missing source".to_owned(),
                ));
            }
        };
        if source.content_digest != t.source_digest
            || source.project_id != t.project_id
            || source.header.realm_id != t.header.realm_id
            || source.header.authority_scope_id != t.header.authority_scope_id
        {
            return Err(MetaError::UnsupportedSchema(
                "transcript disagrees with its source".to_owned(),
            ));
        }
        t.validate(source.duration_ms)
            .map_err(|e| MetaError::UnsupportedSchema(format!("transcript revision: {e}")))?;
        let latest: i64 = conn.query_row(
            "SELECT COALESCE(MAX(revision_no), 0) FROM audio_transcripts WHERE source_id = ?1",
            params![t.source_id.as_str()],
            |row| row.get(0),
        )?;
        if i64::from(t.revision_no) != latest + 1 {
            return Err(MetaError::Conflict(format!(
                "transcript revision {} does not follow revision {latest}",
                t.revision_no
            )));
        }
        if let TranscriptOrigin::HumanCorrection {
            parent_revision_id, ..
        } = &t.origin
        {
            let parent: Option<String> = conn
                .query_row(
                    "SELECT source_id FROM audio_transcripts WHERE revision_id = ?1",
                    params![parent_revision_id.as_str()],
                    |row| row.get(0),
                )
                .optional()?;
            if parent.as_deref() != Some(t.source_id.as_str()) {
                return Err(MetaError::UnsupportedSchema(
                    "a correction's parent is an earlier revision of the same source".to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// Writes one transcription outcome: the receipt, and for an allowed
    /// request the new revision, in one transaction.
    pub fn commit_transcript(
        &self,
        revision: Option<&TranscriptRevision>,
        receipt: &TranscriptReceipt,
    ) -> Result<(), MetaError> {
        if receipt.transcript_revision_id.as_ref() != revision.map(|t| &t.header.id) {
            return Err(MetaError::UnsupportedSchema(
                "receipt does not name its revision".to_owned(),
            ));
        }
        let tx = self.conn().unchecked_transaction()?;
        if let Some(t) = revision {
            if t.source_id != receipt.source_id
                || u32::try_from(t.segments.len()).ok() != Some(receipt.segment_count)
            {
                return Err(MetaError::UnsupportedSchema(
                    "receipt disagrees with its revision".to_owned(),
                ));
            }
            Self::check_transcript_on(&tx, t)?;
            Self::audio_insert_transcript_on(&tx, t)?;
        }
        Self::audio_insert_receipt_on(&tx, receipt)?;
        tx.commit()?;
        Ok(())
    }

    /// Stores a human correction (no receipt: the origin records who and why).
    pub fn insert_transcript_correction(&self, t: &TranscriptRevision) -> Result<(), MetaError> {
        if !matches!(t.origin, TranscriptOrigin::HumanCorrection { .. }) {
            return Err(MetaError::UnsupportedSchema(
                "engine revisions are written with their receipt".to_owned(),
            ));
        }
        let tx = self.conn().unchecked_transaction()?;
        Self::check_transcript_on(&tx, t)?;
        Self::audio_insert_transcript_on(&tx, t)?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_transcript_revision(&self, id: &OpaqueId) -> Result<TranscriptRevision, MetaError> {
        let t = self
            .audio_rows(
                &format!(
                    "SELECT {TRANSCRIPT_COLUMNS} FROM audio_transcripts WHERE revision_id = ?1"
                ),
                &[&id.as_str()],
                decode_transcript,
            )?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)?;
        self.revalidate_transcript(&t)?;
        Ok(t)
    }

    fn revalidate_transcript(&self, t: &TranscriptRevision) -> Result<(), MetaError> {
        let (source, _) = self.get_audio_source(&t.source_id).map_err(|e| match e {
            MetaError::NotFound => corrupt("transcript names a missing source".to_owned()),
            other => other,
        })?;
        if source.content_digest != t.source_digest {
            return Err(corrupt("transcript source digest disagrees".to_owned()));
        }
        t.validate(source.duration_ms)
            .map_err(|e| corrupt(format!("transcript revision: {e}")))
    }

    pub fn list_transcript_revisions(
        &self,
        source_id: &OpaqueId,
    ) -> Result<Vec<TranscriptRevision>, MetaError> {
        let all = self.audio_rows(
            &format!(
                "SELECT {TRANSCRIPT_COLUMNS} FROM audio_transcripts WHERE source_id = ?1 ORDER BY revision_no LIMIT 500"
            ),
            &[&source_id.as_str()],
            decode_transcript,
        )?;
        for t in &all {
            self.revalidate_transcript(t)?;
        }
        Ok(all)
    }

    pub fn list_all_transcript_revisions(&self) -> Result<Vec<TranscriptRevision>, MetaError> {
        self.audio_rows(
            &format!("SELECT {TRANSCRIPT_COLUMNS} FROM audio_transcripts ORDER BY rowid"),
            &[],
            decode_transcript,
        )
    }

    pub fn list_transcript_receipts(
        &self,
        source_id: &OpaqueId,
    ) -> Result<Vec<TranscriptReceipt>, MetaError> {
        self.audio_rows(
            &format!(
                "SELECT {RECEIPT_COLUMNS} FROM audio_transcript_receipts WHERE source_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&source_id.as_str()],
            decode_receipt,
        )
    }

    pub fn get_transcript_receipt(&self, id: &OpaqueId) -> Result<TranscriptReceipt, MetaError> {
        self.audio_rows(
            &format!(
                "SELECT {RECEIPT_COLUMNS} FROM audio_transcript_receipts WHERE receipt_id = ?1"
            ),
            &[&id.as_str()],
            decode_receipt,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_all_transcript_receipts(&self) -> Result<Vec<TranscriptReceipt>, MetaError> {
        self.audio_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM audio_transcript_receipts ORDER BY rowid"),
            &[],
            decode_receipt,
        )
    }

    // -- consistency and restore ------------------------------------------------

    fn audio_project_in_scope(
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
    /// - every source, session, revision and receipt names an existing
    ///   Project in the same realm and scope;
    /// - a stopped session names a capture source that names it back, whose
    ///   byte size matches the session; capture sources belong to a stopped
    ///   session;
    /// - chunks exist only for open sessions, contiguous, summing to the
    ///   session's captured bytes;
    /// - revisions are contiguous per source, match the source digest and
    ///   duration, and corrections cite an earlier revision of the source;
    /// - every engine revision has exactly one allowed receipt that matches
    ///   its route, engine and segment count; every receipt matches its
    ///   source digest.
    pub fn verify_audio_consistency(&self) -> Result<(), MetaError> {
        let sources: HashMap<String, AudioSource> = self
            .list_all_audio_sources()?
            .into_iter()
            .map(|r| (r.source.header.id.as_str().to_owned(), r.source))
            .collect();
        for s in sources.values() {
            self.audio_project_in_scope(&s.project_id, &s.header, "audio source")?;
        }
        let sessions = self.list_all_capture_sessions()?;
        let mut chunk_bytes: HashMap<String, (u32, u64)> = HashMap::new();
        for c in self.list_all_capture_chunks()? {
            let e = chunk_bytes
                .entry(c.session_id.as_str().to_owned())
                .or_default();
            e.0 += 1;
            if c.seq != e.0 {
                return Err(corrupt("capture chunks are not contiguous".to_owned()));
            }
            e.1 += (c.content_hex.len() / 2) as u64;
        }
        let mut capture_sources = HashSet::new();
        for s in &sessions {
            self.audio_project_in_scope(&s.project_id, &s.header, "capture session")?;
            let chunks = chunk_bytes.remove(s.header.id.as_str());
            if s.state.is_open() {
                let (count, bytes) = chunks.unwrap_or_default();
                if count != s.chunk_count || bytes != s.captured_bytes {
                    return Err(corrupt(
                        "capture chunks disagree with their session".to_owned(),
                    ));
                }
            } else if chunks.is_some() {
                return Err(corrupt("a closed capture still holds chunks".to_owned()));
            }
            if let Some(source_id) = &s.source_id {
                let source = sources
                    .get(source_id.as_str())
                    .ok_or_else(|| corrupt("capture session names a missing source".to_owned()))?;
                if source.capture_session_id.as_ref() != Some(&s.header.id)
                    || source.project_id != s.project_id
                {
                    return Err(corrupt(
                        "capture source does not name its session".to_owned(),
                    ));
                }
                capture_sources.insert(source_id.as_str().to_owned());
            }
        }
        if !chunk_bytes.is_empty() {
            return Err(corrupt("capture chunks name a missing session".to_owned()));
        }
        for s in sources.values() {
            if s.kind == AudioSourceKind::Capture && !capture_sources.contains(s.header.id.as_str())
            {
                return Err(corrupt(
                    "capture source without its stopped session".to_owned(),
                ));
            }
        }
        let revisions = self.list_all_transcript_revisions()?;
        let mut by_source: HashMap<String, Vec<&TranscriptRevision>> = HashMap::new();
        let ids: HashMap<String, &TranscriptRevision> = revisions
            .iter()
            .map(|t| (t.header.id.as_str().to_owned(), t))
            .collect();
        for t in &revisions {
            self.audio_project_in_scope(&t.project_id, &t.header, "transcript revision")?;
            let source = sources
                .get(t.source_id.as_str())
                .ok_or_else(|| corrupt("transcript names a missing source".to_owned()))?;
            if source.content_digest != t.source_digest || source.project_id != t.project_id {
                return Err(corrupt("transcript disagrees with its source".to_owned()));
            }
            t.validate(source.duration_ms)
                .map_err(|e| corrupt(format!("transcript revision: {e}")))?;
            if let TranscriptOrigin::HumanCorrection {
                parent_revision_id, ..
            } = &t.origin
            {
                let parent = ids
                    .get(parent_revision_id.as_str())
                    .ok_or_else(|| corrupt("correction names a missing parent".to_owned()))?;
                if parent.source_id != t.source_id || parent.revision_no >= t.revision_no {
                    return Err(corrupt(
                        "correction parent is not an earlier revision".to_owned(),
                    ));
                }
            }
            by_source
                .entry(t.source_id.as_str().to_owned())
                .or_default()
                .push(t);
        }
        for list in by_source.values_mut() {
            list.sort_by_key(|t| t.revision_no);
            for (i, t) in list.iter().enumerate() {
                if t.revision_no as usize != i + 1 {
                    return Err(corrupt(
                        "transcript revisions are not contiguous".to_owned(),
                    ));
                }
            }
        }
        let mut receipted = HashSet::new();
        for r in self.list_all_transcript_receipts()? {
            let source = sources
                .get(r.source_id.as_str())
                .ok_or_else(|| corrupt("receipt names a missing source".to_owned()))?;
            if source.content_digest != r.source_digest || source.project_id != r.project_id {
                return Err(corrupt("receipt disagrees with its source".to_owned()));
            }
            self.audio_project_in_scope(&r.project_id, &r.header, "transcript receipt")?;
            if let Some(id) = &r.transcript_revision_id {
                let t = ids
                    .get(id.as_str())
                    .ok_or_else(|| corrupt("receipt names a missing revision".to_owned()))?;
                let TranscriptOrigin::Engine { route, engine, .. } = &t.origin else {
                    return Err(corrupt("a receipt covers an engine revision".to_owned()));
                };
                if *route != r.route
                    || *engine != r.engine
                    || t.source_id != r.source_id
                    || t.segments.len() != r.segment_count as usize
                {
                    return Err(corrupt("receipt disagrees with its revision".to_owned()));
                }
                receipted.insert(id.as_str().to_owned());
            }
        }
        for t in &revisions {
            if matches!(t.origin, TranscriptOrigin::Engine { .. })
                && !receipted.contains(t.header.id.as_str())
            {
                return Err(corrupt("an engine revision has no receipt".to_owned()));
            }
        }
        Ok(())
    }

    pub fn restore_audio_source_row(&self, row: &AudioSourceRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(Self::audio_insert_source_on(
            self.conn(),
            &row.source,
            &bytes,
        ))
    }

    pub fn restore_capture_session_row(&self, s: &AudioSession) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::audio_insert_session_on(self.conn(), s))
    }

    pub fn restore_capture_chunk_row(&self, row: &AudioChunkRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO audio_capture_chunks(session_id, seq, content) VALUES (?1, ?2, ?3)",
                params![row.session_id.as_str(), row.seq, bytes],
            ),
            "capture chunk",
            row.session_id.as_str(),
        ))
    }

    /// Plain insert; cross-row checks run in `verify_audio_consistency`.
    pub fn restore_transcript_row(&self, t: &TranscriptRevision) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::audio_insert_transcript_on(self.conn(), t))
    }

    pub fn restore_transcript_receipt_row(&self, r: &TranscriptReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::audio_insert_receipt_on(self.conn(), r))
    }
}

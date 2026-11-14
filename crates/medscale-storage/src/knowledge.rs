//! Knowledge + Research Canvas durable rows (Spec 083, storage schema v12).
//!
//! Same pattern as Specs 079-082: each row keeps the validated contract
//! value as JSON (`body_json`) plus the columns queries need; reads re-check
//! columns against the body and fail closed on disagreement. An index
//! version's manifest and chunks are written in one transaction, and the
//! chunk list is re-digested against the manifest on every read. Canvas
//! revisions are contiguous per Canvas. Everything is insert-once; nothing
//! here references (or can cascade into) source rows of Specs 075-082.

use std::collections::HashMap;

use medscale_contracts::knowledge::{
    CanvasRevision, IndexChunk, IndexManifest, ReceiptHit, RetrievalReceipt, chunks_digest,
};
use medscale_contracts::objects::{ObjectHeader, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v12 DDL, executed inside `begin/finish_migration(12)`.
pub(crate) const V12_DDL: &str = r"
CREATE TABLE IF NOT EXISTS knowledge_manifests (
  manifest_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  body_json TEXT NOT NULL,
  UNIQUE (project_id, version)
);
CREATE TABLE IF NOT EXISTS knowledge_chunks (
  manifest_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  body_json TEXT NOT NULL,
  PRIMARY KEY (manifest_id, seq)
);
CREATE TABLE IF NOT EXISTS knowledge_receipts (
  receipt_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_knowledge_receipts_project
  ON knowledge_receipts(project_id);
CREATE TABLE IF NOT EXISTS knowledge_canvases (
  canvas_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  PRIMARY KEY (canvas_id, revision)
);
CREATE INDEX IF NOT EXISTS idx_knowledge_canvases_project
  ON knowledge_canvases(project_id);
";

/// One index version with its chunks (backup form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexVersionRow {
    pub manifest: IndexManifest,
    pub chunks: Vec<IndexChunk>,
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

/// Checks a chunk list against its manifest: count, contiguous 1-based
/// sequence, per-chunk invariants, every span from an indexed source, and
/// the recorded digest.
fn check_chunks(manifest: &IndexManifest, chunks: &[IndexChunk]) -> Result<(), String> {
    manifest.validate()?;
    if chunks.len() != manifest.chunk_count as usize {
        return Err("chunk count disagrees with its manifest".to_owned());
    }
    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.seq as usize != i + 1 {
            return Err("chunk sequence must be 1..n".to_owned());
        }
        chunk.validate()?;
        if manifest.sources.binary_search(&chunk.span.source).is_err() {
            return Err("a chunk names a source its manifest does not index".to_owned());
        }
    }
    if chunks_digest(chunks) != manifest.chunks_digest {
        return Err("chunks do not match their manifest digest".to_owned());
    }
    Ok(())
}

fn decode_manifest(row: &rusqlite::Row<'_>) -> Result<IndexManifest, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let version: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let value: IndexManifest = from_json(&body, "index manifest")?;
    check_column(&id, value.header.id.as_str(), "index manifest")?;
    check_column(&project_id, value.project_id.as_str(), "index manifest")?;
    if i64::from(value.version) != version {
        return Err(corrupt(
            "index manifest row columns disagree with its body".to_owned(),
        ));
    }
    value
        .validate()
        .map_err(|e| corrupt(format!("index manifest: {e}")))?;
    Ok(value)
}

fn decode_chunk(row: &rusqlite::Row<'_>) -> Result<IndexChunk, MetaError> {
    let seq: i64 = row.get(0)?;
    let body: String = row.get(1)?;
    let value: IndexChunk = from_json(&body, "index chunk")?;
    if i64::from(value.seq) != seq {
        return Err(corrupt(
            "index chunk row columns disagree with its body".to_owned(),
        ));
    }
    Ok(value)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<RetrievalReceipt, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let value: RetrievalReceipt = from_json(&body, "retrieval receipt")?;
    check_column(&id, value.header.id.as_str(), "retrieval receipt")?;
    check_column(&project_id, value.project_id.as_str(), "retrieval receipt")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("retrieval receipt: {e}")))?;
    Ok(value)
}

fn decode_canvas(row: &rusqlite::Row<'_>) -> Result<CanvasRevision, MetaError> {
    let id: String = row.get(0)?;
    let revision: i64 = row.get(1)?;
    let project_id: String = row.get(2)?;
    let body: String = row.get(3)?;
    let value: CanvasRevision = from_json(&body, "canvas")?;
    check_column(&id, value.header.id.as_str(), "canvas")?;
    check_column(&project_id, value.project_id.as_str(), "canvas")?;
    if i64::from(value.revision) != revision {
        return Err(corrupt(
            "canvas row columns disagree with its body".to_owned(),
        ));
    }
    value
        .validate()
        .map_err(|e| corrupt(format!("canvas: {e}")))?;
    Ok(value)
}

const MANIFEST_COLUMNS: &str = "manifest_id, project_id, version, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, project_id, body_json";
const CANVAS_COLUMNS: &str = "canvas_id, revision, project_id, body_json";

impl SqliteMetaStore {
    fn knowledge_rows<T>(
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

    /// Allocates one `prefix-N` Knowledge id from a durable sequence.
    pub fn alloc_knowledge_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("knowledge_seq_{prefix}");
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

    fn knowledge_insert_version_on(
        conn: &rusqlite::Connection,
        manifest: &IndexManifest,
        chunks: &[IndexChunk],
    ) -> Result<(), MetaError> {
        check_chunks(manifest, chunks)
            .map_err(|e| MetaError::UnsupportedSchema(format!("index version: {e}")))?;
        let previous: Option<i64> = conn
            .query_row(
                "SELECT MAX(version) FROM knowledge_manifests WHERE project_id = ?1",
                params![manifest.project_id.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        if i64::from(manifest.version) != previous.unwrap_or(0) + 1 {
            return Err(MetaError::Conflict(format!(
                "index version {} does not follow the project's latest",
                manifest.version
            )));
        }
        map_insert(
            conn.execute(
                "INSERT INTO knowledge_manifests(manifest_id, project_id, version, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    manifest.header.id.as_str(),
                    manifest.project_id.as_str(),
                    i64::from(manifest.version),
                    to_json(manifest)?
                ],
            ),
            "index manifest",
            manifest.header.id.as_str(),
        )?;
        let mut insert = conn.prepare(
            "INSERT INTO knowledge_chunks(manifest_id, seq, body_json) VALUES (?1, ?2, ?3)",
        )?;
        for chunk in chunks {
            map_insert(
                insert.execute(params![
                    manifest.header.id.as_str(),
                    i64::from(chunk.seq),
                    to_json(chunk)?
                ]),
                "index chunk",
                manifest.header.id.as_str(),
            )?;
        }
        Ok(())
    }

    /// Writes one index version (manifest and every chunk) atomically.
    pub fn commit_index_version(
        &self,
        manifest: &IndexManifest,
        chunks: &[IndexChunk],
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        Self::knowledge_insert_version_on(&tx, manifest, chunks)?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_index_manifest(&self, id: &OpaqueId) -> Result<IndexManifest, MetaError> {
        self.knowledge_rows(
            &format!("SELECT {MANIFEST_COLUMNS} FROM knowledge_manifests WHERE manifest_id = ?1"),
            &[&id.as_str()],
            decode_manifest,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    /// The newest index version of a Project, if any.
    pub fn latest_index_manifest(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Option<IndexManifest>, MetaError> {
        Ok(self
            .knowledge_rows(
                &format!(
                    "SELECT {MANIFEST_COLUMNS} FROM knowledge_manifests WHERE project_id = ?1 ORDER BY version DESC LIMIT 1"
                ),
                &[&project_id.as_str()],
                decode_manifest,
            )?
            .into_iter()
            .next())
    }

    /// An index version's chunks, checked against the manifest digest.
    pub fn get_index_chunks(&self, manifest: &IndexManifest) -> Result<Vec<IndexChunk>, MetaError> {
        let chunks = self.knowledge_rows(
            "SELECT seq, body_json FROM knowledge_chunks WHERE manifest_id = ?1 ORDER BY seq",
            &[&manifest.header.id.as_str()],
            decode_chunk,
        )?;
        check_chunks(manifest, &chunks).map_err(|e| corrupt(format!("index version: {e}")))?;
        Ok(chunks)
    }

    fn list_all_index_manifests(&self) -> Result<Vec<IndexManifest>, MetaError> {
        self.knowledge_rows(
            &format!("SELECT {MANIFEST_COLUMNS} FROM knowledge_manifests ORDER BY rowid"),
            &[],
            decode_manifest,
        )
    }

    pub fn list_all_index_versions(&self) -> Result<Vec<IndexVersionRow>, MetaError> {
        self.list_all_index_manifests()?
            .into_iter()
            .map(|manifest| {
                let chunks = self.get_index_chunks(&manifest)?;
                Ok(IndexVersionRow { manifest, chunks })
            })
            .collect()
    }

    fn knowledge_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &RetrievalReceipt,
    ) -> Result<(), MetaError> {
        r.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("retrieval receipt: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO knowledge_receipts(receipt_id, project_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.project_id.as_str(), to_json(r)?],
            ),
            "retrieval receipt",
            r.header.id.as_str(),
        )
    }

    pub fn insert_retrieval_receipt(&self, r: &RetrievalReceipt) -> Result<(), MetaError> {
        Self::knowledge_insert_receipt_on(self.conn(), r)
    }

    pub fn get_retrieval_receipt(&self, id: &OpaqueId) -> Result<RetrievalReceipt, MetaError> {
        self.knowledge_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM knowledge_receipts WHERE receipt_id = ?1"),
            &[&id.as_str()],
            decode_receipt,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_retrieval_receipts(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<RetrievalReceipt>, MetaError> {
        self.knowledge_rows(
            &format!(
                "SELECT {RECEIPT_COLUMNS} FROM knowledge_receipts WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_receipt,
        )
    }

    pub fn list_all_retrieval_receipts(&self) -> Result<Vec<RetrievalReceipt>, MetaError> {
        self.knowledge_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM knowledge_receipts ORDER BY rowid"),
            &[],
            decode_receipt,
        )
    }

    fn knowledge_insert_canvas_on(
        conn: &rusqlite::Connection,
        c: &CanvasRevision,
    ) -> Result<(), MetaError> {
        c.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("canvas: {e}")))?;
        let previous: Option<(i64, String)> = conn
            .query_row(
                "SELECT revision, project_id FROM knowledge_canvases WHERE canvas_id = ?1 ORDER BY revision DESC LIMIT 1",
                params![c.header.id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let expected = previous.as_ref().map_or(1, |(rev, _)| rev + 1);
        if i64::from(c.revision) != expected {
            return Err(MetaError::Conflict(format!(
                "canvas {} revision {} does not follow its latest",
                c.header.id.as_str(),
                c.revision
            )));
        }
        if previous.is_some_and(|(_, project)| project != c.project_id.as_str()) {
            return Err(MetaError::UnsupportedSchema(
                "a canvas cannot move between projects".to_owned(),
            ));
        }
        map_insert(
            conn.execute(
                "INSERT INTO knowledge_canvases(canvas_id, revision, project_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    c.header.id.as_str(),
                    i64::from(c.revision),
                    c.project_id.as_str(),
                    to_json(c)?
                ],
            ),
            "canvas revision",
            c.header.id.as_str(),
        )
    }

    /// Appends one Canvas revision; it must follow the latest revision.
    pub fn insert_canvas_revision(&self, c: &CanvasRevision) -> Result<(), MetaError> {
        Self::knowledge_insert_canvas_on(self.conn(), c)
    }

    /// The latest revision of a Canvas, or one exact revision.
    pub fn get_canvas(
        &self,
        id: &OpaqueId,
        revision: Option<u32>,
    ) -> Result<CanvasRevision, MetaError> {
        let rows = match revision {
            Some(rev) => self.knowledge_rows(
                &format!(
                    "SELECT {CANVAS_COLUMNS} FROM knowledge_canvases WHERE canvas_id = ?1 AND revision = ?2"
                ),
                &[&id.as_str(), &i64::from(rev)],
                decode_canvas,
            )?,
            None => self.knowledge_rows(
                &format!(
                    "SELECT {CANVAS_COLUMNS} FROM knowledge_canvases WHERE canvas_id = ?1 ORDER BY revision DESC LIMIT 1"
                ),
                &[&id.as_str()],
                decode_canvas,
            )?,
        };
        rows.into_iter().next().ok_or(MetaError::NotFound)
    }

    /// The latest revision of every Canvas of a Project.
    pub fn list_canvases(&self, project_id: &OpaqueId) -> Result<Vec<CanvasRevision>, MetaError> {
        self.knowledge_rows(
            &format!(
                "SELECT {CANVAS_COLUMNS} FROM knowledge_canvases c WHERE project_id = ?1 AND revision = (SELECT MAX(revision) FROM knowledge_canvases WHERE canvas_id = c.canvas_id) ORDER BY canvas_id LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_canvas,
        )
    }

    pub fn list_all_canvas_revisions(&self) -> Result<Vec<CanvasRevision>, MetaError> {
        self.knowledge_rows(
            &format!(
                "SELECT {CANVAS_COLUMNS} FROM knowledge_canvases ORDER BY canvas_id, revision"
            ),
            &[],
            decode_canvas,
        )
    }

    fn knowledge_project_in_scope(
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
    /// - every manifest, receipt and canvas names an existing Project in the
    ///   same realm and scope;
    /// - index versions are contiguous 1..n per Project, and every chunk set
    ///   matches its manifest (checked on read);
    /// - every receipt that ran names a manifest of its own Project with the
    ///   recorded chunk digest;
    /// - canvas revisions are contiguous 1..n and never change Project.
    pub fn verify_knowledge_consistency(&self) -> Result<(), MetaError> {
        let versions = self.list_all_index_versions()?;
        let mut by_project: HashMap<String, Vec<u32>> = HashMap::new();
        let mut manifests: HashMap<String, (&IndexManifest, &[IndexChunk])> = HashMap::new();
        for row in &versions {
            let m = &row.manifest;
            self.knowledge_project_in_scope(&m.project_id, &m.header, "index manifest")?;
            by_project
                .entry(m.project_id.as_str().to_owned())
                .or_default()
                .push(m.version);
            manifests.insert(m.header.id.as_str().to_owned(), (m, &row.chunks));
        }
        for list in by_project.values_mut() {
            list.sort_unstable();
            if list.iter().enumerate().any(|(i, v)| *v as usize != i + 1) {
                return Err(corrupt("index versions are not contiguous".to_owned()));
            }
        }
        for r in self.list_all_retrieval_receipts()? {
            self.knowledge_project_in_scope(&r.project_id, &r.header, "retrieval receipt")?;
            if let Some(id) = &r.manifest_id {
                let (m, chunks) = manifests
                    .get(id.as_str())
                    .ok_or_else(|| corrupt("a receipt names a missing index".to_owned()))?;
                if m.project_id != r.project_id
                    || Some(&m.chunks_digest) != r.manifest_chunks_digest.as_ref()
                {
                    return Err(corrupt("a receipt disagrees with its index".to_owned()));
                }
                // Every hit is one chunk of that index, span for span.
                let hit_in_index = |hit: &ReceiptHit| {
                    (hit.chunk_seq as usize)
                        .checked_sub(1)
                        .and_then(|i| chunks.get(i))
                        .is_some_and(|chunk| chunk.span == hit.span)
                };
                if !r.hits.iter().all(hit_in_index) {
                    return Err(corrupt(
                        "a receipt hit is not a chunk of its index".to_owned(),
                    ));
                }
            }
        }
        let mut canvases: HashMap<String, (u32, OpaqueId)> = HashMap::new();
        for c in self.list_all_canvas_revisions()? {
            self.knowledge_project_in_scope(&c.project_id, &c.header, "canvas")?;
            let entry = canvases.get(c.header.id.as_str());
            let expected = entry.map_or(1, |(rev, _)| rev + 1);
            if c.revision != expected {
                return Err(corrupt("canvas revisions are not contiguous".to_owned()));
            }
            if entry.is_some_and(|(_, project)| *project != c.project_id) {
                return Err(corrupt("a canvas moved between projects".to_owned()));
            }
            canvases.insert(
                c.header.id.as_str().to_owned(),
                (c.revision, c.project_id.clone()),
            );
        }
        Ok(())
    }

    pub fn restore_index_version_row(&self, row: &IndexVersionRow) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::knowledge_insert_version_on(
            self.conn(),
            &row.manifest,
            &row.chunks,
        ))
    }

    pub fn restore_retrieval_receipt_row(&self, r: &RetrievalReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::knowledge_insert_receipt_on(self.conn(), r))
    }

    pub fn restore_canvas_row(&self, c: &CanvasRevision) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::knowledge_insert_canvas_on(self.conn(), c))
    }
}

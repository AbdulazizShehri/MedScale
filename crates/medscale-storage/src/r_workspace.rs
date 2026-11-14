//! R Workspace durable rows (Spec 086, storage schema v15).
//!
//! Same pattern as Specs 079-085: each row keeps the validated contract
//! value as JSON (`body_json`) plus the columns queries need; reads
//! re-check columns against the body and fail closed on disagreement.
//!
//! A workspace row is immutable. Launch, run and publication receipts are
//! append-only and each must describe its workspace (Project, descriptor
//! digest, realm and scope). A publication's receipt and (when published)
//! its table and table bytes are written in one transaction, so a crash
//! never leaves a partial publication.

use std::collections::HashMap;

use medscale_contracts::analytics::ResultTableDoc;
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use medscale_contracts::r_workspace::{
    PublishState, RLaunchReceipt, RPublishReceipt, RPublishedTable, RRunReceipt, RWorkspace,
};
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v15 DDL, executed inside `begin/finish_migration(15)`.
pub(crate) const V15_DDL: &str = r"
CREATE TABLE IF NOT EXISTS rws_workspaces (
  workspace_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_rws_workspaces_project ON rws_workspaces(project_id);
CREATE TABLE IF NOT EXISTS rws_launch_receipts (
  receipt_id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS rws_run_receipts (
  receipt_id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS rws_publish_receipts (
  receipt_id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  state TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS rws_published_tables (
  table_id TEXT PRIMARY KEY,
  receipt_id TEXT NOT NULL UNIQUE,
  workspace_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  content BLOB NOT NULL
);
";

/// Names of the v15 tables (for rewind tests of earlier specs).
pub const R_WORKSPACE_TABLES: [&str; 5] = [
    "rws_workspaces",
    "rws_launch_receipts",
    "rws_run_receipts",
    "rws_publish_receipts",
    "rws_published_tables",
];

/// Published table plus its canonical bytes (backup form: bytes as hex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RPublishedTableRow {
    pub table: RPublishedTable,
    pub content_hex: String,
}

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

/// Strict hex decoding (see Spec 082: characters are checked first).
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

/// Decodes and checks a published table's canonical bytes: digest,
/// canonical encoding and recorded shape.
fn decode_table_doc(table: &RPublishedTable, bytes: &[u8]) -> Result<ResultTableDoc, MetaError> {
    if DigestSha256::of(bytes) != table.content_digest {
        return Err(corrupt(
            "published R table does not match its digest".to_owned(),
        ));
    }
    let doc: ResultTableDoc = from_json(
        std::str::from_utf8(bytes)
            .map_err(|_| corrupt("published R table is not UTF-8".to_owned()))?,
        "published R table content",
    )?;
    if doc.canonical_bytes() != bytes {
        return Err(corrupt("published R table is not canonical".to_owned()));
    }
    if doc.rows.len() as u64 != table.row_count
        || doc.columns.len() as u64 != u64::from(table.column_count)
    {
        return Err(corrupt(
            "published R table shape disagrees with its record".to_owned(),
        ));
    }
    Ok(doc)
}

fn same_scope(a: &ObjectHeader, b: &ObjectHeader) -> bool {
    a.realm_id == b.realm_id && a.authority_scope_id == b.authority_scope_id
}

/// A receipt must describe exactly its workspace.
fn describes(
    ws: &RWorkspace,
    header: &ObjectHeader,
    workspace_id: &OpaqueId,
    project_id: &OpaqueId,
    descriptor_digest: &DigestSha256,
) -> Result<(), String> {
    if workspace_id != ws.id()
        || project_id != &ws.manifest.project_id
        || descriptor_digest != &ws.descriptor_digest
        || !same_scope(header, &ws.manifest.header)
    {
        return Err("receipt does not describe its workspace".to_owned());
    }
    Ok(())
}

/// A published table must be exactly its receipt's table, derived from
/// its workspace's inputs with its workspace's class.
fn table_matches(ws: &RWorkspace, r: &RPublishReceipt, t: &RPublishedTable) -> Result<(), String> {
    let inputs: Vec<&OpaqueId> = ws.manifest.inputs.iter().map(|i| &i.snapshot_id).collect();
    if r.state != PublishState::Published
        || r.table_id.as_ref() != Some(&t.header.id)
        || r.table_digest.as_ref() != Some(&t.content_digest)
        || r.source_digest.as_ref() != Some(&t.source_digest)
        || t.receipt_id != r.header.id
        || t.workspace_id != r.workspace_id
        || t.project_id != r.project_id
        || t.output_name != r.output_name
        || t.row_count != r.row_count
        || t.column_count != r.column_count
        || t.derived_from.iter().collect::<Vec<_>>() != inputs
        || t.data_class != ws.manifest.data_class
        || !same_scope(&t.header, &r.header)
    {
        return Err("published table does not match its receipt".to_owned());
    }
    Ok(())
}

fn decode_workspace(row: &rusqlite::Row<'_>) -> Result<RWorkspace, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let value: RWorkspace = from_json(&body, "R workspace")?;
    check_column(&id, value.id().as_str(), "R workspace")?;
    check_column(
        &project_id,
        value.manifest.project_id.as_str(),
        "R workspace",
    )?;
    value
        .validate()
        .map_err(|e| corrupt(format!("R workspace: {e}")))?;
    Ok(value)
}

fn decode_launch(row: &rusqlite::Row<'_>) -> Result<RLaunchReceipt, MetaError> {
    let id: String = row.get(0)?;
    let workspace_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let body: String = row.get(3)?;
    let value: RLaunchReceipt = from_json(&body, "R launch receipt")?;
    check_column(&id, value.header.id.as_str(), "R launch receipt")?;
    check_column(
        &workspace_id,
        value.workspace_id.as_str(),
        "R launch receipt",
    )?;
    check_column(&project_id, value.project_id.as_str(), "R launch receipt")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("R launch receipt: {e}")))?;
    Ok(value)
}

fn decode_run(row: &rusqlite::Row<'_>) -> Result<RRunReceipt, MetaError> {
    let id: String = row.get(0)?;
    let workspace_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let body: String = row.get(3)?;
    let value: RRunReceipt = from_json(&body, "R run receipt")?;
    check_column(&id, value.header.id.as_str(), "R run receipt")?;
    check_column(&workspace_id, value.workspace_id.as_str(), "R run receipt")?;
    check_column(&project_id, value.project_id.as_str(), "R run receipt")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("R run receipt: {e}")))?;
    Ok(value)
}

fn decode_publish(row: &rusqlite::Row<'_>) -> Result<RPublishReceipt, MetaError> {
    let id: String = row.get(0)?;
    let workspace_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let state: String = row.get(3)?;
    let body: String = row.get(4)?;
    let value: RPublishReceipt = from_json(&body, "R publish receipt")?;
    check_column(&id, value.header.id.as_str(), "R publish receipt")?;
    check_column(
        &workspace_id,
        value.workspace_id.as_str(),
        "R publish receipt",
    )?;
    check_column(&project_id, value.project_id.as_str(), "R publish receipt")?;
    check_column(&state, value.state.as_str(), "R publish receipt")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("R publish receipt: {e}")))?;
    Ok(value)
}

fn decode_table(row: &rusqlite::Row<'_>) -> Result<(RPublishedTable, Vec<u8>), MetaError> {
    let id: String = row.get(0)?;
    let receipt_id: String = row.get(1)?;
    let workspace_id: String = row.get(2)?;
    let project_id: String = row.get(3)?;
    let body: String = row.get(4)?;
    let content: Vec<u8> = row.get(5)?;
    let value: RPublishedTable = from_json(&body, "published R table")?;
    check_column(&id, value.header.id.as_str(), "published R table")?;
    check_column(&receipt_id, value.receipt_id.as_str(), "published R table")?;
    check_column(
        &workspace_id,
        value.workspace_id.as_str(),
        "published R table",
    )?;
    check_column(&project_id, value.project_id.as_str(), "published R table")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("published R table: {e}")))?;
    decode_table_doc(&value, &content)?;
    Ok((value, content))
}

const WORKSPACE_COLUMNS: &str = "workspace_id, project_id, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, workspace_id, project_id, body_json";
const PUBLISH_COLUMNS: &str = "receipt_id, workspace_id, project_id, state, body_json";
const TABLE_COLUMNS: &str = "table_id, receipt_id, workspace_id, project_id, body_json, content";

impl SqliteMetaStore {
    fn rws_rows<T>(
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

    /// Allocates one `prefix-N` R Workspace id from a durable sequence.
    pub fn alloc_rws_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("rws_seq_{prefix}");
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

    fn rws_insert_workspace_on(
        conn: &rusqlite::Connection,
        ws: &RWorkspace,
    ) -> Result<(), MetaError> {
        ws.validate().map_err(|e| invalid("R workspace", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO rws_workspaces(workspace_id, project_id, body_json) VALUES (?1, ?2, ?3)",
                params![
                    ws.id().as_str(),
                    ws.manifest.project_id.as_str(),
                    to_json(ws)?
                ],
            ),
            "R workspace",
            ws.id().as_str(),
        )
    }

    fn rws_insert_launch_on(
        conn: &rusqlite::Connection,
        r: &RLaunchReceipt,
    ) -> Result<(), MetaError> {
        r.validate().map_err(|e| invalid("R launch receipt", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO rws_launch_receipts(receipt_id, workspace_id, project_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    r.header.id.as_str(),
                    r.workspace_id.as_str(),
                    r.project_id.as_str(),
                    to_json(r)?
                ],
            ),
            "R launch receipt",
            r.header.id.as_str(),
        )
    }

    fn rws_insert_run_on(conn: &rusqlite::Connection, r: &RRunReceipt) -> Result<(), MetaError> {
        r.validate().map_err(|e| invalid("R run receipt", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO rws_run_receipts(receipt_id, workspace_id, project_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    r.header.id.as_str(),
                    r.workspace_id.as_str(),
                    r.project_id.as_str(),
                    to_json(r)?
                ],
            ),
            "R run receipt",
            r.header.id.as_str(),
        )
    }

    fn rws_insert_publish_on(
        conn: &rusqlite::Connection,
        r: &RPublishReceipt,
    ) -> Result<(), MetaError> {
        r.validate().map_err(|e| invalid("R publish receipt", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO rws_publish_receipts(receipt_id, workspace_id, project_id, state, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    r.header.id.as_str(),
                    r.workspace_id.as_str(),
                    r.project_id.as_str(),
                    r.state.as_str(),
                    to_json(r)?
                ],
            ),
            "R publish receipt",
            r.header.id.as_str(),
        )
    }

    fn rws_insert_table_on(
        conn: &rusqlite::Connection,
        t: &RPublishedTable,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        t.validate().map_err(|e| invalid("published R table", e))?;
        decode_table_doc(t, bytes).map_err(|_| {
            MetaError::UnsupportedSchema("published R table content mismatch".to_owned())
        })?;
        map_insert(
            conn.execute(
                "INSERT INTO rws_published_tables(table_id, receipt_id, workspace_id, project_id, body_json, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    t.header.id.as_str(),
                    t.receipt_id.as_str(),
                    t.workspace_id.as_str(),
                    t.project_id.as_str(),
                    to_json(t)?,
                    bytes
                ],
            ),
            "published R table",
            t.header.id.as_str(),
        )
    }

    pub fn insert_r_workspace(&self, ws: &RWorkspace) -> Result<(), MetaError> {
        Self::rws_insert_workspace_on(self.conn(), ws)
    }

    pub fn insert_r_launch_receipt(&self, r: &RLaunchReceipt) -> Result<(), MetaError> {
        let ws = self.get_r_workspace(&r.workspace_id)?;
        describes(
            &ws,
            &r.header,
            &r.workspace_id,
            &r.project_id,
            &r.descriptor_digest,
        )
        .map_err(MetaError::UnsupportedSchema)?;
        Self::rws_insert_launch_on(self.conn(), r)
    }

    pub fn insert_r_run_receipt(&self, r: &RRunReceipt) -> Result<(), MetaError> {
        let ws = self.get_r_workspace(&r.workspace_id)?;
        describes(
            &ws,
            &r.header,
            &r.workspace_id,
            &r.project_id,
            &r.descriptor_digest,
        )
        .map_err(MetaError::UnsupportedSchema)?;
        Self::rws_insert_run_on(self.conn(), r)
    }

    /// Records a publication: a refused receipt alone, or a published
    /// receipt with its table and table bytes, in one transaction.
    pub fn insert_r_publication(
        &self,
        r: &RPublishReceipt,
        table: Option<(&RPublishedTable, &ResultTableDoc)>,
    ) -> Result<(), MetaError> {
        let ws = self.get_r_workspace(&r.workspace_id)?;
        describes(
            &ws,
            &r.header,
            &r.workspace_id,
            &r.project_id,
            &r.descriptor_digest,
        )
        .map_err(MetaError::UnsupportedSchema)?;
        let bytes = table.map(|(_, doc)| doc.canonical_bytes());
        match (table, &bytes) {
            (Some((t, _)), Some(_)) => {
                table_matches(&ws, r, t).map_err(MetaError::UnsupportedSchema)?;
            }
            _ => {
                if r.state == PublishState::Published {
                    return Err(MetaError::UnsupportedSchema(
                        "a published receipt needs its table".to_owned(),
                    ));
                }
            }
        }
        let tx = self.conn().unchecked_transaction()?;
        Self::rws_insert_publish_on(&tx, r)?;
        if let (Some((t, _)), Some(bytes)) = (table, &bytes) {
            Self::rws_insert_table_on(&tx, t, bytes)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_r_workspace(&self, id: &OpaqueId) -> Result<RWorkspace, MetaError> {
        self.rws_rows(
            &format!("SELECT {WORKSPACE_COLUMNS} FROM rws_workspaces WHERE workspace_id = ?1"),
            &[&id.as_str()],
            decode_workspace,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_r_workspaces(&self, project_id: &OpaqueId) -> Result<Vec<RWorkspace>, MetaError> {
        self.rws_rows(
            &format!(
                "SELECT {WORKSPACE_COLUMNS} FROM rws_workspaces WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_workspace,
        )
    }

    pub fn list_all_r_workspaces(&self) -> Result<Vec<RWorkspace>, MetaError> {
        self.rws_rows(
            &format!("SELECT {WORKSPACE_COLUMNS} FROM rws_workspaces ORDER BY rowid"),
            &[],
            decode_workspace,
        )
    }

    fn rws_where(workspace_id: Option<&OpaqueId>) -> (&'static str, Vec<String>) {
        match workspace_id {
            Some(id) => (" WHERE workspace_id = ?1", vec![id.as_str().to_owned()]),
            None => ("", Vec::new()),
        }
    }

    /// Launch receipts of one workspace, or of all when `None`.
    pub fn list_r_launch_receipts(
        &self,
        workspace_id: Option<&OpaqueId>,
    ) -> Result<Vec<RLaunchReceipt>, MetaError> {
        let (filter, args) = Self::rws_where(workspace_id);
        let args: Vec<&dyn rusqlite::ToSql> =
            args.iter().map(|a| a as &dyn rusqlite::ToSql).collect();
        self.rws_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM rws_launch_receipts{filter} ORDER BY rowid"),
            &args,
            decode_launch,
        )
    }

    pub fn list_r_run_receipts(
        &self,
        workspace_id: Option<&OpaqueId>,
    ) -> Result<Vec<RRunReceipt>, MetaError> {
        let (filter, args) = Self::rws_where(workspace_id);
        let args: Vec<&dyn rusqlite::ToSql> =
            args.iter().map(|a| a as &dyn rusqlite::ToSql).collect();
        self.rws_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM rws_run_receipts{filter} ORDER BY rowid"),
            &args,
            decode_run,
        )
    }

    pub fn list_r_publish_receipts(
        &self,
        workspace_id: Option<&OpaqueId>,
    ) -> Result<Vec<RPublishReceipt>, MetaError> {
        let (filter, args) = Self::rws_where(workspace_id);
        let args: Vec<&dyn rusqlite::ToSql> =
            args.iter().map(|a| a as &dyn rusqlite::ToSql).collect();
        self.rws_rows(
            &format!("SELECT {PUBLISH_COLUMNS} FROM rws_publish_receipts{filter} ORDER BY rowid"),
            &args,
            decode_publish,
        )
    }

    /// A published table with its digest-checked, canonical content.
    pub fn get_r_published_table(
        &self,
        table_id: &OpaqueId,
    ) -> Result<(RPublishedTable, ResultTableDoc), MetaError> {
        let (table, bytes) = self
            .rws_rows(
                &format!("SELECT {TABLE_COLUMNS} FROM rws_published_tables WHERE table_id = ?1"),
                &[&table_id.as_str()],
                decode_table,
            )?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)?;
        let doc = decode_table_doc(&table, &bytes)?;
        Ok((table, doc))
    }

    pub fn list_all_r_published_tables(&self) -> Result<Vec<RPublishedTableRow>, MetaError> {
        Ok(self
            .rws_rows(
                &format!("SELECT {TABLE_COLUMNS} FROM rws_published_tables ORDER BY rowid"),
                &[],
                decode_table,
            )?
            .into_iter()
            .map(|(table, bytes)| RPublishedTableRow {
                table,
                content_hex: to_hex(&bytes),
            })
            .collect())
    }

    /// Cross-row invariants, re-verified after restore:
    /// - every workspace names an existing Project in its realm and scope;
    /// - every receipt describes an existing workspace;
    /// - a publish receipt is `published` exactly when it has one table,
    ///   and the table matches it and its workspace (inputs, class);
    /// - no table exists without its receipt.
    pub fn verify_r_workspace_consistency(&self) -> Result<(), MetaError> {
        let workspaces: HashMap<String, RWorkspace> = self
            .list_all_r_workspaces()?
            .into_iter()
            .map(|w| (w.id().as_str().to_owned(), w))
            .collect();
        for ws in workspaces.values() {
            let project = match self.get_project(&ws.manifest.project_id) {
                Ok(project) => project,
                Err(MetaError::NotFound) => {
                    return Err(corrupt("R workspace names a missing project".to_owned()));
                }
                Err(other) => return Err(other),
            };
            if !same_scope(&project.header, &ws.manifest.header) {
                return Err(corrupt(
                    "R workspace is outside its project's scope".to_owned(),
                ));
            }
        }
        let find = |id: &OpaqueId| {
            workspaces
                .get(id.as_str())
                .ok_or_else(|| corrupt("R receipt without its workspace".to_owned()))
        };
        for r in self.list_r_launch_receipts(None)? {
            describes(
                find(&r.workspace_id)?,
                &r.header,
                &r.workspace_id,
                &r.project_id,
                &r.descriptor_digest,
            )
            .map_err(corrupt)?;
        }
        for r in self.list_r_run_receipts(None)? {
            describes(
                find(&r.workspace_id)?,
                &r.header,
                &r.workspace_id,
                &r.project_id,
                &r.descriptor_digest,
            )
            .map_err(corrupt)?;
        }
        let mut publications: HashMap<String, RPublishReceipt> = HashMap::new();
        for r in self.list_r_publish_receipts(None)? {
            describes(
                find(&r.workspace_id)?,
                &r.header,
                &r.workspace_id,
                &r.project_id,
                &r.descriptor_digest,
            )
            .map_err(corrupt)?;
            publications.insert(r.header.id.as_str().to_owned(), r);
        }
        let tables = self.list_all_r_published_tables()?;
        for row in &tables {
            let r = publications
                .get(row.table.receipt_id.as_str())
                .ok_or_else(|| corrupt("published R table without its receipt".to_owned()))?;
            table_matches(find(&r.workspace_id)?, r, &row.table).map_err(corrupt)?;
        }
        let published = publications
            .values()
            .filter(|r| r.state == PublishState::Published)
            .count();
        if published != tables.len() {
            return Err(corrupt("a published R receipt has no table".to_owned()));
        }
        Ok(())
    }

    /// Backup families (tables carry their bytes as hex).
    pub fn r_workspace_backup_families(
        &self,
    ) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        Ok(vec![
            (
                "rws_workspaces",
                v(serde_json::to_value(self.list_all_r_workspaces()?))?,
            ),
            (
                "rws_launch_receipts",
                v(serde_json::to_value(self.list_r_launch_receipts(None)?))?,
            ),
            (
                "rws_run_receipts",
                v(serde_json::to_value(self.list_r_run_receipts(None)?))?,
            ),
            (
                "rws_publish_receipts",
                v(serde_json::to_value(self.list_r_publish_receipts(None)?))?,
            ),
            (
                "rws_published_tables",
                v(serde_json::to_value(self.list_all_r_published_tables()?))?,
            ),
        ])
    }

    pub fn restore_r_workspace_row(&self, ws: &RWorkspace) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::rws_insert_workspace_on(self.conn(), ws))
    }

    pub fn restore_r_launch_row(&self, r: &RLaunchReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::rws_insert_launch_on(self.conn(), r))
    }

    pub fn restore_r_run_row(&self, r: &RRunReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::rws_insert_run_on(self.conn(), r))
    }

    pub fn restore_r_publish_row(&self, r: &RPublishReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::rws_insert_publish_on(self.conn(), r))
    }

    pub fn restore_r_table_row(&self, row: &RPublishedTableRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(Self::rws_insert_table_on(self.conn(), &row.table, &bytes))
    }
}

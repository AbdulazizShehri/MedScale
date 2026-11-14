//! Analytics Gate durable rows (Spec 082, storage schema v11).
//!
//! Same pattern as Specs 079-081: each row keeps the validated contract
//! value as JSON (`body_json`) plus the columns queries need; reads
//! re-check columns against the body and fail closed on disagreement.
//! A derived table's canonical bytes live in a `content` BLOB whose SHA-256
//! must equal the recorded digest on every read. A receipt and its derived
//! table are written in one transaction. Everything is insert-once.

use std::collections::{HashMap, HashSet};

use medscale_contracts::analytics::{
    CohortDefinition, DerivedTable, QueryOrigin, QueryReceipt, ResultTableDoc,
};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v11 DDL, executed inside `begin/finish_migration(11)`.
pub(crate) const V11_DDL: &str = r"
CREATE TABLE IF NOT EXISTS analytics_receipts (
  receipt_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_analytics_receipts_project
  ON analytics_receipts(project_id);
CREATE TABLE IF NOT EXISTS analytics_results (
  result_id TEXT PRIMARY KEY,
  receipt_id TEXT NOT NULL UNIQUE,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  content BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS analytics_cohorts (
  cohort_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_analytics_cohorts_project
  ON analytics_cohorts(project_id);
";

/// Derived table plus its canonical bytes (backup form: bytes as hex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedTableRow {
    pub table: DerivedTable,
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

/// Strict hex decoding. Checking the characters first keeps
/// a tampered backup from reaching a non-ASCII slice boundary (a panic) or
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

/// Decodes and checks a derived table's canonical bytes.
fn decode_table_doc(table: &DerivedTable, bytes: &[u8]) -> Result<ResultTableDoc, MetaError> {
    if DigestSha256::of(bytes) != table.content_digest {
        return Err(corrupt(
            "derived table content does not match its digest".to_owned(),
        ));
    }
    let doc: ResultTableDoc = from_json(
        std::str::from_utf8(bytes).map_err(|_| corrupt("derived table is not UTF-8".to_owned()))?,
        "derived table content",
    )?;
    if doc.rows.len() as u64 != table.row_count
        || doc.columns.len() as u64 != u64::from(table.column_count)
    {
        return Err(corrupt(
            "derived table shape disagrees with its record".to_owned(),
        ));
    }
    Ok(doc)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<QueryReceipt, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let value: QueryReceipt = from_json(&body, "query receipt")?;
    check_column(&id, value.header.id.as_str(), "query receipt")?;
    check_column(&project_id, value.project_id.as_str(), "query receipt")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("query receipt: {e}")))?;
    Ok(value)
}

fn decode_result(row: &rusqlite::Row<'_>) -> Result<(DerivedTable, Vec<u8>), MetaError> {
    let id: String = row.get(0)?;
    let receipt_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let body: String = row.get(3)?;
    let content: Vec<u8> = row.get(4)?;
    let value: DerivedTable = from_json(&body, "derived table")?;
    check_column(&id, value.header.id.as_str(), "derived table")?;
    check_column(&receipt_id, value.receipt_id.as_str(), "derived table")?;
    check_column(&project_id, value.project_id.as_str(), "derived table")?;
    decode_table_doc(&value, &content)?;
    Ok((value, content))
}

fn decode_cohort(row: &rusqlite::Row<'_>) -> Result<CohortDefinition, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let value: CohortDefinition = from_json(&body, "cohort")?;
    check_column(&id, value.header.id.as_str(), "cohort")?;
    check_column(&project_id, value.project_id.as_str(), "cohort")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("cohort: {e}")))?;
    Ok(value)
}

const RECEIPT_COLUMNS: &str = "receipt_id, project_id, body_json";
const RESULT_COLUMNS: &str = "result_id, receipt_id, project_id, body_json, content";
const COHORT_COLUMNS: &str = "cohort_id, project_id, body_json";

impl SqliteMetaStore {
    fn analytics_rows<T>(
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

    /// Allocates one `prefix-N` Analytics id from a durable sequence.
    pub fn alloc_analytics_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("analytics_seq_{prefix}");
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

    fn analytics_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &QueryReceipt,
    ) -> Result<(), MetaError> {
        r.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("query receipt: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO analytics_receipts(receipt_id, project_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.project_id.as_str(), to_json(r)?],
            ),
            "query receipt",
            r.header.id.as_str(),
        )
    }

    fn analytics_insert_result_on(
        conn: &rusqlite::Connection,
        t: &DerivedTable,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        decode_table_doc(t, bytes).map_err(|_| {
            MetaError::UnsupportedSchema("derived table content mismatch".to_owned())
        })?;
        map_insert(
            conn.execute(
                "INSERT INTO analytics_results(result_id, receipt_id, project_id, body_json, content) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    t.header.id.as_str(),
                    t.receipt_id.as_str(),
                    t.project_id.as_str(),
                    to_json(t)?,
                    bytes
                ],
            ),
            "derived table",
            t.header.id.as_str(),
        )
    }

    /// Writes one query outcome: the receipt and, when it completed, its
    /// derived table, in one transaction.
    pub fn commit_query(
        &self,
        receipt: &QueryReceipt,
        result: Option<(&DerivedTable, &ResultTableDoc)>,
    ) -> Result<(), MetaError> {
        let bytes = result.map(|(_, doc)| doc.canonical_bytes());
        match (result, &bytes) {
            (Some((t, _)), Some(bytes)) => {
                if receipt.result_id.as_ref() != Some(&t.header.id)
                    || receipt.result_digest.as_ref() != Some(&t.content_digest)
                    || t.receipt_id != receipt.header.id
                    || t.project_id != receipt.project_id
                    || t.row_count != receipt.row_count
                    || t.column_count != receipt.column_count
                    || DigestSha256::of(bytes) != t.content_digest
                {
                    return Err(MetaError::UnsupportedSchema(
                        "receipt and derived table disagree".to_owned(),
                    ));
                }
            }
            _ => {
                if receipt.result_id.is_some() {
                    return Err(MetaError::UnsupportedSchema(
                        "receipt names a result that is not written".to_owned(),
                    ));
                }
            }
        }
        let tx = self.conn().unchecked_transaction()?;
        Self::analytics_insert_receipt_on(&tx, receipt)?;
        if let (Some((t, _)), Some(bytes)) = (result, &bytes) {
            Self::analytics_insert_result_on(&tx, t, bytes)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_query_receipt(&self, id: &OpaqueId) -> Result<QueryReceipt, MetaError> {
        self.analytics_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM analytics_receipts WHERE receipt_id = ?1"),
            &[&id.as_str()],
            decode_receipt,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_query_receipts(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<QueryReceipt>, MetaError> {
        self.analytics_rows(
            &format!(
                "SELECT {RECEIPT_COLUMNS} FROM analytics_receipts WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_receipt,
        )
    }

    pub fn list_all_query_receipts(&self) -> Result<Vec<QueryReceipt>, MetaError> {
        self.analytics_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM analytics_receipts ORDER BY rowid"),
            &[],
            decode_receipt,
        )
    }

    /// A derived table with its digest-checked content.
    pub fn get_derived_table(
        &self,
        id: &OpaqueId,
    ) -> Result<(DerivedTable, ResultTableDoc), MetaError> {
        let (table, bytes) = self
            .analytics_rows(
                &format!("SELECT {RESULT_COLUMNS} FROM analytics_results WHERE result_id = ?1"),
                &[&id.as_str()],
                decode_result,
            )?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)?;
        let doc = decode_table_doc(&table, &bytes)?;
        Ok((table, doc))
    }

    pub fn list_all_derived_tables(&self) -> Result<Vec<DerivedTableRow>, MetaError> {
        Ok(self
            .analytics_rows(
                &format!("SELECT {RESULT_COLUMNS} FROM analytics_results ORDER BY rowid"),
                &[],
                decode_result,
            )?
            .into_iter()
            .map(|(table, bytes)| DerivedTableRow {
                table,
                content_hex: to_hex(&bytes),
            })
            .collect())
    }

    pub fn insert_cohort(&self, c: &CohortDefinition) -> Result<(), MetaError> {
        Self::analytics_insert_cohort_on(self.conn(), c)
    }

    fn analytics_insert_cohort_on(
        conn: &rusqlite::Connection,
        c: &CohortDefinition,
    ) -> Result<(), MetaError> {
        c.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("cohort: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO analytics_cohorts(cohort_id, project_id, body_json) VALUES (?1, ?2, ?3)",
                params![c.header.id.as_str(), c.project_id.as_str(), to_json(c)?],
            ),
            "cohort",
            c.header.id.as_str(),
        )
    }

    pub fn get_cohort(&self, id: &OpaqueId) -> Result<CohortDefinition, MetaError> {
        self.analytics_rows(
            &format!("SELECT {COHORT_COLUMNS} FROM analytics_cohorts WHERE cohort_id = ?1"),
            &[&id.as_str()],
            decode_cohort,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_cohorts(&self, project_id: &OpaqueId) -> Result<Vec<CohortDefinition>, MetaError> {
        self.analytics_rows(
            &format!(
                "SELECT {COHORT_COLUMNS} FROM analytics_cohorts WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_cohort,
        )
    }

    pub fn list_all_cohorts(&self) -> Result<Vec<CohortDefinition>, MetaError> {
        self.analytics_rows(
            &format!("SELECT {COHORT_COLUMNS} FROM analytics_cohorts ORDER BY rowid"),
            &[],
            decode_cohort,
        )
    }

    fn analytics_project_in_scope(
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
    /// - every receipt, derived table and cohort names an existing Project
    ///   in the same realm and scope;
    /// - every completed receipt has exactly one derived table, whose id,
    ///   digest, counts and project match the receipt; no table exists
    ///   without its receipt;
    /// - every cohort receipt names a stored cohort of the same project.
    pub fn verify_analytics_consistency(&self) -> Result<(), MetaError> {
        let receipts: HashMap<String, QueryReceipt> = self
            .list_all_query_receipts()?
            .into_iter()
            .map(|r| (r.header.id.as_str().to_owned(), r))
            .collect();
        let cohorts: HashMap<String, CohortDefinition> = self
            .list_all_cohorts()?
            .into_iter()
            .map(|c| (c.header.id.as_str().to_owned(), c))
            .collect();
        for c in cohorts.values() {
            self.analytics_project_in_scope(&c.project_id, &c.header, "cohort")?;
        }
        let mut with_table = HashSet::new();
        for row in self.list_all_derived_tables()? {
            let t = &row.table;
            self.analytics_project_in_scope(&t.project_id, &t.header, "derived table")?;
            let r = receipts
                .get(t.receipt_id.as_str())
                .ok_or_else(|| corrupt("derived table without its receipt".to_owned()))?;
            if r.result_id.as_ref() != Some(&t.header.id)
                || r.result_digest.as_ref() != Some(&t.content_digest)
                || r.row_count != t.row_count
                || r.column_count != t.column_count
                || r.project_id != t.project_id
            {
                return Err(corrupt(
                    "derived table disagrees with its receipt".to_owned(),
                ));
            }
            with_table.insert(r.header.id.as_str().to_owned());
        }
        for r in receipts.values() {
            self.analytics_project_in_scope(&r.project_id, &r.header, "query receipt")?;
            if r.outcome.has_result() && !with_table.contains(r.header.id.as_str()) {
                return Err(corrupt("a completed query has no derived table".to_owned()));
            }
            if r.origin == QueryOrigin::CohortBuilder {
                let cohort = r
                    .cohort_id
                    .as_ref()
                    .and_then(|id| cohorts.get(id.as_str()))
                    .ok_or_else(|| corrupt("a cohort query names a missing cohort".to_owned()))?;
                if cohort.project_id != r.project_id {
                    return Err(corrupt("a cohort query crosses projects".to_owned()));
                }
            }
        }
        Ok(())
    }

    pub fn restore_query_receipt_row(&self, r: &QueryReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::analytics_insert_receipt_on(self.conn(), r))
    }

    pub fn restore_derived_table_row(&self, row: &DerivedTableRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(Self::analytics_insert_result_on(
            self.conn(),
            &row.table,
            &bytes,
        ))
    }

    pub fn restore_cohort_row(&self, c: &CohortDefinition) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::analytics_insert_cohort_on(self.conn(), c))
    }
}

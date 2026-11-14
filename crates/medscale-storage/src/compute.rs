//! MedScale Compute durable rows (Spec 085, storage schema v14).
//!
//! Same pattern as Specs 079-084: each row keeps the validated contract
//! value as JSON (`body_json`) plus the columns queries need; reads
//! re-check columns against the body and fail closed on disagreement.
//!
//! A job's manifest never changes; only its state moves, and only
//! forward: `queued -> running -> terminal` or `queued -> terminal`. Every
//! state change is a conditional update on the expected current state, so
//! a job is claimed for execution at most once. A terminal state, its
//! receipt and (when completed) its output and output bytes are written in
//! one transaction, so a crash never leaves a partial output.

use std::collections::HashMap;

use medscale_contracts::analytics::ResultTableDoc;
use medscale_contracts::compute::{ComputeJob, ComputeOutput, ComputeReceipt, ComputeState};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v14 DDL, executed inside `begin/finish_migration(14)`.
pub(crate) const V14_DDL: &str = r"
CREATE TABLE IF NOT EXISTS compute_jobs (
  job_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  state TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_compute_jobs_project ON compute_jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_compute_jobs_state ON compute_jobs(state);
CREATE TABLE IF NOT EXISTS compute_receipts (
  receipt_id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL UNIQUE,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS compute_outputs (
  output_id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL UNIQUE,
  receipt_id TEXT NOT NULL UNIQUE,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  content BLOB NOT NULL
);
";

/// Names of the v14 tables (for rewind tests of earlier specs).
pub const COMPUTE_TABLES: [&str; 3] = ["compute_jobs", "compute_receipts", "compute_outputs"];

/// Output plus its canonical bytes (backup form: bytes as hex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeOutputRow {
    pub output: ComputeOutput,
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

/// Decodes and checks an output's canonical bytes: digest, canonical
/// encoding and recorded shape.
fn decode_output_doc(output: &ComputeOutput, bytes: &[u8]) -> Result<ResultTableDoc, MetaError> {
    if DigestSha256::of(bytes) != output.content_digest {
        return Err(corrupt(
            "compute output content does not match its digest".to_owned(),
        ));
    }
    let doc: ResultTableDoc = from_json(
        std::str::from_utf8(bytes)
            .map_err(|_| corrupt("compute output is not UTF-8".to_owned()))?,
        "compute output content",
    )?;
    if doc.canonical_bytes() != bytes {
        return Err(corrupt("compute output is not canonical".to_owned()));
    }
    if doc.rows.len() as u64 != output.row_count
        || doc.columns.len() as u64 != u64::from(output.column_count)
    {
        return Err(corrupt(
            "compute output shape disagrees with its record".to_owned(),
        ));
    }
    Ok(doc)
}

fn decode_job(row: &rusqlite::Row<'_>) -> Result<ComputeJob, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let state: String = row.get(2)?;
    let body: String = row.get(3)?;
    let value: ComputeJob = from_json(&body, "compute job")?;
    check_column(&id, value.header.id.as_str(), "compute job")?;
    check_column(
        &project_id,
        value.manifest.project_id.as_str(),
        "compute job",
    )?;
    check_column(&state, value.state.as_str(), "compute job")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("compute job: {e}")))?;
    Ok(value)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<ComputeReceipt, MetaError> {
    let id: String = row.get(0)?;
    let job_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let body: String = row.get(3)?;
    let value: ComputeReceipt = from_json(&body, "compute receipt")?;
    check_column(&id, value.header.id.as_str(), "compute receipt")?;
    check_column(&job_id, value.job_id.as_str(), "compute receipt")?;
    check_column(&project_id, value.project_id.as_str(), "compute receipt")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("compute receipt: {e}")))?;
    Ok(value)
}

fn decode_output(row: &rusqlite::Row<'_>) -> Result<(ComputeOutput, Vec<u8>), MetaError> {
    let id: String = row.get(0)?;
    let job_id: String = row.get(1)?;
    let receipt_id: String = row.get(2)?;
    let project_id: String = row.get(3)?;
    let body: String = row.get(4)?;
    let content: Vec<u8> = row.get(5)?;
    let value: ComputeOutput = from_json(&body, "compute output")?;
    check_column(&id, value.header.id.as_str(), "compute output")?;
    check_column(&job_id, value.job_id.as_str(), "compute output")?;
    check_column(&receipt_id, value.receipt_id.as_str(), "compute output")?;
    check_column(&project_id, value.project_id.as_str(), "compute output")?;
    decode_output_doc(&value, &content)?;
    Ok((value, content))
}

const JOB_COLUMNS: &str = "job_id, project_id, state, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, job_id, project_id, body_json";
const OUTPUT_COLUMNS: &str = "output_id, job_id, receipt_id, project_id, body_json, content";

/// The receipt must describe exactly this job.
fn receipt_matches_job(job: &ComputeJob, r: &ComputeReceipt) -> Result<(), String> {
    let m = &job.manifest;
    if r.job_id != job.header.id
        || r.project_id != m.project_id
        || r.kind != m.kind
        || r.manifest_digest != job.manifest_digest
        || r.input != m.input
        || r.runtime != m.runtime
        || r.header.realm_id != job.header.realm_id
        || r.header.authority_scope_id != job.header.authority_scope_id
    {
        return Err("receipt does not describe its job".to_owned());
    }
    Ok(())
}

/// The output must be exactly the receipt's output.
fn output_matches_receipt(o: &ComputeOutput, r: &ComputeReceipt) -> Result<(), String> {
    if r.output_id.as_ref() != Some(&o.header.id)
        || r.output_digest.as_ref() != Some(&o.content_digest)
        || o.receipt_id != r.header.id
        || o.job_id != r.job_id
        || o.project_id != r.project_id
        || o.kind != r.kind
        || o.row_count != r.row_count
        || o.column_count != r.column_count
        || r.input
            .as_ref()
            .map(|i| (&i.snapshot_id, &i.content_digest))
            != Some((&o.derived_from, &o.input_digest))
        || o.header.realm_id != r.header.realm_id
        || o.header.authority_scope_id != r.header.authority_scope_id
    {
        return Err("output does not match its receipt".to_owned());
    }
    Ok(())
}

impl SqliteMetaStore {
    fn compute_rows<T>(
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

    /// Allocates one `prefix-N` Compute id from a durable sequence.
    pub fn alloc_compute_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("compute_seq_{prefix}");
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

    fn compute_insert_job_on(
        conn: &rusqlite::Connection,
        job: &ComputeJob,
    ) -> Result<(), MetaError> {
        job.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("compute job: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO compute_jobs(job_id, project_id, state, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    job.header.id.as_str(),
                    job.manifest.project_id.as_str(),
                    job.state.as_str(),
                    to_json(job)?
                ],
            ),
            "compute job",
            job.header.id.as_str(),
        )
    }

    fn compute_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &ComputeReceipt,
    ) -> Result<(), MetaError> {
        r.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("compute receipt: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO compute_receipts(receipt_id, job_id, project_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    r.header.id.as_str(),
                    r.job_id.as_str(),
                    r.project_id.as_str(),
                    to_json(r)?
                ],
            ),
            "compute receipt",
            r.header.id.as_str(),
        )
    }

    fn compute_insert_output_on(
        conn: &rusqlite::Connection,
        o: &ComputeOutput,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        decode_output_doc(o, bytes).map_err(|_| {
            MetaError::UnsupportedSchema("compute output content mismatch".to_owned())
        })?;
        map_insert(
            conn.execute(
                "INSERT INTO compute_outputs(output_id, job_id, receipt_id, project_id, body_json, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    o.header.id.as_str(),
                    o.job_id.as_str(),
                    o.receipt_id.as_str(),
                    o.project_id.as_str(),
                    to_json(o)?,
                    bytes
                ],
            ),
            "compute output",
            o.header.id.as_str(),
        )
    }

    /// Moves a job from `from` to `to` state, or fails with `Conflict`
    /// when the job is not in `from` (it was already claimed or finished).
    fn compute_transition_on(
        conn: &rusqlite::Connection,
        job: &ComputeJob,
        from: ComputeState,
        to: ComputeState,
    ) -> Result<ComputeJob, MetaError> {
        let mut next = job.clone();
        next.state = to;
        next.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("compute job: {e}")))?;
        let changed = conn.execute(
            "UPDATE compute_jobs SET state = ?1, body_json = ?2 WHERE job_id = ?3 AND state = ?4",
            params![
                to.as_str(),
                to_json(&next)?,
                job.header.id.as_str(),
                from.as_str()
            ],
        )?;
        if changed != 1 {
            return Err(MetaError::Conflict(format!(
                "compute job {} is not {}",
                job.header.id.as_str(),
                from.as_str()
            )));
        }
        Ok(next)
    }

    /// Records a new job: queued with no receipt, or already terminal
    /// (refused at admission) together with its receipt.
    pub fn insert_compute_job(
        &self,
        job: &ComputeJob,
        receipt: Option<&ComputeReceipt>,
    ) -> Result<(), MetaError> {
        match (job.state, receipt) {
            (ComputeState::Queued, None) => {}
            (state, Some(r)) if state.is_terminal() && r.state == state => {
                receipt_matches_job(job, r).map_err(MetaError::UnsupportedSchema)?;
                if r.output_id.is_some() {
                    return Err(MetaError::UnsupportedSchema(
                        "a new job cannot carry an output".to_owned(),
                    ));
                }
            }
            _ => {
                return Err(MetaError::UnsupportedSchema(
                    "a new job is queued, or terminal with its receipt".to_owned(),
                ));
            }
        }
        let tx = self.conn().unchecked_transaction()?;
        Self::compute_insert_job_on(&tx, job)?;
        if let Some(r) = receipt {
            Self::compute_insert_receipt_on(&tx, r)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Claims a queued job for execution (`queued -> running`). A job is
    /// claimed at most once.
    pub fn claim_compute_job(&self, job_id: &OpaqueId) -> Result<ComputeJob, MetaError> {
        let job = self.get_compute_job(job_id)?;
        Self::compute_transition_on(
            self.conn(),
            &job,
            ComputeState::Queued,
            ComputeState::Running,
        )
    }

    /// Finishes a job from `from` (queued or running): its terminal state,
    /// receipt and (when completed) output, in one transaction.
    pub fn finish_compute_job(
        &self,
        job_id: &OpaqueId,
        from: ComputeState,
        receipt: &ComputeReceipt,
        output: Option<(&ComputeOutput, &ResultTableDoc)>,
    ) -> Result<ComputeJob, MetaError> {
        if from.is_terminal() {
            return Err(MetaError::UnsupportedSchema(
                "a terminal job cannot change state".to_owned(),
            ));
        }
        let job = self.get_compute_job(job_id)?;
        receipt_matches_job(&job, receipt).map_err(MetaError::UnsupportedSchema)?;
        let bytes = output.map(|(_, doc)| doc.canonical_bytes());
        match (output, &bytes) {
            (Some((o, _)), Some(bytes)) => {
                output_matches_receipt(o, receipt).map_err(MetaError::UnsupportedSchema)?;
                if DigestSha256::of(bytes) != o.content_digest {
                    return Err(MetaError::UnsupportedSchema(
                        "output bytes do not match their digest".to_owned(),
                    ));
                }
            }
            _ => {
                if receipt.output_id.is_some() {
                    return Err(MetaError::UnsupportedSchema(
                        "receipt names an output that is not written".to_owned(),
                    ));
                }
            }
        }
        let tx = self.conn().unchecked_transaction()?;
        let next = Self::compute_transition_on(&tx, &job, from, receipt.state)?;
        Self::compute_insert_receipt_on(&tx, receipt)?;
        if let (Some((o, _)), Some(bytes)) = (output, &bytes) {
            Self::compute_insert_output_on(&tx, o, bytes)?;
        }
        tx.commit()?;
        Ok(next)
    }

    pub fn get_compute_job(&self, id: &OpaqueId) -> Result<ComputeJob, MetaError> {
        self.compute_rows(
            &format!("SELECT {JOB_COLUMNS} FROM compute_jobs WHERE job_id = ?1"),
            &[&id.as_str()],
            decode_job,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_compute_jobs(&self, project_id: &OpaqueId) -> Result<Vec<ComputeJob>, MetaError> {
        self.compute_rows(
            &format!(
                "SELECT {JOB_COLUMNS} FROM compute_jobs WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_job,
        )
    }

    pub fn list_compute_jobs_in_state(
        &self,
        state: ComputeState,
    ) -> Result<Vec<ComputeJob>, MetaError> {
        self.compute_rows(
            &format!("SELECT {JOB_COLUMNS} FROM compute_jobs WHERE state = ?1 ORDER BY rowid"),
            &[&state.as_str()],
            decode_job,
        )
    }

    pub fn list_all_compute_jobs(&self) -> Result<Vec<ComputeJob>, MetaError> {
        self.compute_rows(
            &format!("SELECT {JOB_COLUMNS} FROM compute_jobs ORDER BY rowid"),
            &[],
            decode_job,
        )
    }

    pub fn get_compute_receipt_for_job(
        &self,
        job_id: &OpaqueId,
    ) -> Result<Option<ComputeReceipt>, MetaError> {
        Ok(self
            .compute_rows(
                &format!("SELECT {RECEIPT_COLUMNS} FROM compute_receipts WHERE job_id = ?1"),
                &[&job_id.as_str()],
                decode_receipt,
            )?
            .into_iter()
            .next())
    }

    pub fn list_all_compute_receipts(&self) -> Result<Vec<ComputeReceipt>, MetaError> {
        self.compute_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM compute_receipts ORDER BY rowid"),
            &[],
            decode_receipt,
        )
    }

    /// An output with its digest-checked, canonical content.
    pub fn get_compute_output(
        &self,
        id: &OpaqueId,
    ) -> Result<(ComputeOutput, ResultTableDoc), MetaError> {
        let (output, bytes) = self
            .compute_rows(
                &format!("SELECT {OUTPUT_COLUMNS} FROM compute_outputs WHERE output_id = ?1"),
                &[&id.as_str()],
                decode_output,
            )?
            .into_iter()
            .next()
            .ok_or(MetaError::NotFound)?;
        let doc = decode_output_doc(&output, &bytes)?;
        Ok((output, doc))
    }

    pub fn list_all_compute_outputs(&self) -> Result<Vec<ComputeOutputRow>, MetaError> {
        Ok(self
            .compute_rows(
                &format!("SELECT {OUTPUT_COLUMNS} FROM compute_outputs ORDER BY rowid"),
                &[],
                decode_output,
            )?
            .into_iter()
            .map(|(output, bytes)| ComputeOutputRow {
                output,
                content_hex: to_hex(&bytes),
            })
            .collect())
    }

    fn compute_project_in_scope(
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
    /// - every job names an existing Project in its realm and scope;
    /// - a job is terminal exactly when it has one receipt, and the receipt
    ///   describes that job (manifest digest, input, runtime, scope) and
    ///   carries the job's state;
    /// - a receipt is completed exactly when it has one output, and the
    ///   output matches it (id, digest, counts, input);
    /// - no receipt or output exists without its job or receipt.
    pub fn verify_compute_consistency(&self) -> Result<(), MetaError> {
        let jobs: HashMap<String, ComputeJob> = self
            .list_all_compute_jobs()?
            .into_iter()
            .map(|j| (j.header.id.as_str().to_owned(), j))
            .collect();
        let mut receipts: HashMap<String, ComputeReceipt> = HashMap::new();
        for r in self.list_all_compute_receipts()? {
            let job = jobs
                .get(r.job_id.as_str())
                .ok_or_else(|| corrupt("compute receipt without its job".to_owned()))?;
            receipt_matches_job(job, &r).map_err(corrupt)?;
            if r.state != job.state {
                return Err(corrupt(
                    "compute receipt state differs from its job".to_owned(),
                ));
            }
            receipts.insert(r.header.id.as_str().to_owned(), r);
        }
        let with_receipt: HashMap<&str, &ComputeReceipt> =
            receipts.values().map(|r| (r.job_id.as_str(), r)).collect();
        for job in jobs.values() {
            self.compute_project_in_scope(&job.manifest.project_id, &job.header, "compute job")?;
            if job.state.is_terminal() != with_receipt.contains_key(job.header.id.as_str()) {
                return Err(corrupt(
                    "a compute job is terminal exactly when it has a receipt".to_owned(),
                ));
            }
        }
        let outputs = self.list_all_compute_outputs()?;
        for row in &outputs {
            let r = receipts
                .get(row.output.receipt_id.as_str())
                .ok_or_else(|| corrupt("compute output without its receipt".to_owned()))?;
            output_matches_receipt(&row.output, r).map_err(corrupt)?;
        }
        let completed = receipts
            .values()
            .filter(|r| r.state == ComputeState::Completed)
            .count();
        if completed != outputs.len() {
            return Err(corrupt(
                "a completed compute receipt has no output".to_owned(),
            ));
        }
        Ok(())
    }

    /// Backup families (outputs carry their bytes as hex).
    pub fn compute_backup_families(
        &self,
    ) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        Ok(vec![
            (
                "compute_jobs",
                v(serde_json::to_value(self.list_all_compute_jobs()?))?,
            ),
            (
                "compute_receipts",
                v(serde_json::to_value(self.list_all_compute_receipts()?))?,
            ),
            (
                "compute_outputs",
                v(serde_json::to_value(self.list_all_compute_outputs()?))?,
            ),
        ])
    }

    pub fn restore_compute_job_row(&self, job: &ComputeJob) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::compute_insert_job_on(self.conn(), job))
    }

    pub fn restore_compute_receipt_row(&self, r: &ComputeReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::compute_insert_receipt_on(self.conn(), r))
    }

    pub fn restore_compute_output_row(&self, row: &ComputeOutputRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(Self::compute_insert_output_on(
            self.conn(),
            &row.output,
            &bytes,
        ))
    }
}

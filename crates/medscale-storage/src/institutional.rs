//! Institutional adapter rows (Spec 090, storage schema v19).
//!
//! Same pattern as Specs 079-089. Adapters and write intents change by
//! compare-and-set on their revision, each change committed with its
//! receipt. An intent is marked `sent` durably before the transport is
//! called, so a crash during a call leaves `sent`, which recovery turns
//! into `unknown` (never into a blind retry).

use std::collections::HashMap;

use medscale_contracts::institutional::{
    AdapterReceipt, ExternalWriteIntent, InstitutionalAdapter,
};
use medscale_contracts::objects::{EffectState, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v19 DDL, executed inside `begin/finish_migration(19)`.
pub(crate) const V19_DDL: &str = r"
CREATE TABLE IF NOT EXISTS ia_adapters (
  adapter_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  name TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL,
  UNIQUE(project_id, name)
);
CREATE TABLE IF NOT EXISTS ia_intents (
  intent_id TEXT PRIMARY KEY,
  adapter_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  state TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ia_intents_state ON ia_intents(state);
CREATE TABLE IF NOT EXISTS ia_receipts (
  receipt_id TEXT PRIMARY KEY,
  adapter_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
";

/// Names of the v19 tables (for rewind tests of earlier specs).
pub const INSTITUTIONAL_TABLES: [&str; 3] = ["ia_adapters", "ia_intents", "ia_receipts"];

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

fn rev(r: u64) -> Result<i64, MetaError> {
    i64::try_from(r).map_err(|_| invalid("institutional", "revision overflow".to_owned()))
}

/// Snake-case name of an effect state (the storage column).
#[must_use]
pub fn effect_state_name(s: EffectState) -> &'static str {
    match s {
        EffectState::Pending => "pending",
        EffectState::Sent => "sent",
        EffectState::Confirmed => "confirmed",
        EffectState::Failed => "failed",
        EffectState::Unknown => "unknown",
    }
}

fn decode_adapter(row: &rusqlite::Row<'_>) -> Result<InstitutionalAdapter, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let name: String = row.get(2)?;
    let revision: i64 = row.get(3)?;
    let body: String = row.get(4)?;
    let v: InstitutionalAdapter = from_json(&body, "institutional adapter")?;
    check(&id, v.header.id.as_str(), "institutional adapter")?;
    check(&project_id, v.project_id.as_str(), "institutional adapter")?;
    check(&name, &v.name, "institutional adapter")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("adapter revision disagrees".to_owned()));
    }
    v.validate()
        .map_err(|e| corrupt(format!("institutional adapter: {e}")))?;
    Ok(v)
}

fn decode_intent(row: &rusqlite::Row<'_>) -> Result<ExternalWriteIntent, MetaError> {
    let id: String = row.get(0)?;
    let adapter_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let state: String = row.get(3)?;
    let revision: i64 = row.get(4)?;
    let body: String = row.get(5)?;
    let v: ExternalWriteIntent = from_json(&body, "write intent")?;
    check(&id, v.header.id.as_str(), "write intent")?;
    check(&adapter_id, v.adapter_id.as_str(), "write intent")?;
    check(&project_id, v.project_id.as_str(), "write intent")?;
    check(&state, effect_state_name(v.state), "write intent")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("intent revision disagrees".to_owned()));
    }
    v.validate()
        .map_err(|e| corrupt(format!("write intent: {e}")))?;
    Ok(v)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<AdapterReceipt, MetaError> {
    let id: String = row.get(0)?;
    let adapter_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let v: AdapterReceipt = from_json(&body, "adapter receipt")?;
    check(&id, v.header.id.as_str(), "adapter receipt")?;
    check(&adapter_id, v.adapter_id.as_str(), "adapter receipt")?;
    Ok(v)
}

const ADAPTER_COLUMNS: &str = "adapter_id, project_id, name, revision, body_json";
const INTENT_COLUMNS: &str = "intent_id, adapter_id, project_id, state, revision, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, adapter_id, body_json";

/// One adapter-side change and its receipt, committed together.
#[derive(Debug, Default)]
pub struct AdapterChange<'a> {
    pub adapter: Option<(&'a InstitutionalAdapter, Option<u64>)>,
    pub intent: Option<(&'a ExternalWriteIntent, Option<u64>)>,
    pub receipt: Option<&'a AdapterReceipt>,
}

impl SqliteMetaStore {
    fn ia_rows<T>(
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

    /// Allocates one `prefix-N` institutional id from a durable sequence.
    pub fn alloc_institutional_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("ia_seq_{prefix}");
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

    fn ia_write_adapter_on(
        conn: &rusqlite::Connection,
        a: &InstitutionalAdapter,
        expected: Option<u64>,
    ) -> Result<(), MetaError> {
        a.validate()
            .map_err(|e| invalid("institutional adapter", e))?;
        match expected {
            None => map_insert(
                conn.execute(
                    "INSERT INTO ia_adapters(adapter_id, project_id, name, revision, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![a.header.id.as_str(), a.project_id.as_str(), a.name, rev(a.revision)?, to_json(a)?],
                ),
                "institutional adapter",
                a.header.id.as_str(),
            ),
            Some(e) => {
                if a.revision != e + 1 {
                    return Err(invalid("institutional adapter", "revision must advance by one".to_owned()));
                }
                let changed = conn.execute(
                    "UPDATE ia_adapters SET revision = ?1, body_json = ?2 WHERE adapter_id = ?3 AND revision = ?4",
                    params![rev(a.revision)?, to_json(a)?, a.header.id.as_str(), rev(e)?],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict("adapter changed concurrently".to_owned()))
                }
            }
        }
    }

    fn ia_write_intent_on(
        conn: &rusqlite::Connection,
        i: &ExternalWriteIntent,
        expected: Option<u64>,
    ) -> Result<(), MetaError> {
        i.validate().map_err(|e| invalid("write intent", e))?;
        match expected {
            None => map_insert(
                conn.execute(
                    "INSERT INTO ia_intents(intent_id, adapter_id, project_id, state, revision, body_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        i.header.id.as_str(),
                        i.adapter_id.as_str(),
                        i.project_id.as_str(),
                        effect_state_name(i.state),
                        rev(i.revision)?,
                        to_json(i)?
                    ],
                ),
                "write intent",
                i.header.id.as_str(),
            ),
            Some(e) => {
                if i.revision != e + 1 {
                    return Err(invalid("write intent", "revision must advance by one".to_owned()));
                }
                let changed = conn.execute(
                    "UPDATE ia_intents SET state = ?1, revision = ?2, body_json = ?3 WHERE intent_id = ?4 AND revision = ?5",
                    params![effect_state_name(i.state), rev(i.revision)?, to_json(i)?, i.header.id.as_str(), rev(e)?],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict("intent changed concurrently".to_owned()))
                }
            }
        }
    }

    fn ia_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &AdapterReceipt,
    ) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO ia_receipts(receipt_id, adapter_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.adapter_id.as_str(), to_json(r)?],
            ),
            "adapter receipt",
            r.header.id.as_str(),
        )
    }

    fn ia_apply(
        &self,
        change: &AdapterChange<'_>,
        receipt: &AdapterReceipt,
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        if let Some((a, e)) = change.adapter {
            Self::ia_write_adapter_on(&tx, a, e)?;
        }
        if let Some((i, e)) = change.intent {
            Self::ia_write_intent_on(&tx, i, e)?;
        }
        Self::ia_insert_receipt_on(&tx, receipt)?;
        tx.commit()?;
        Ok(())
    }

    /// Commits one change with its receipt in a single transaction.
    pub fn commit_adapter_change(&self, change: &AdapterChange<'_>) -> Result<(), MetaError> {
        let receipt = change
            .receipt
            .ok_or_else(|| invalid("adapter change", "a change carries its receipt".to_owned()))?;
        self.ia_apply(change, receipt)
    }

    pub fn get_institutional_adapter(
        &self,
        id: &OpaqueId,
    ) -> Result<InstitutionalAdapter, MetaError> {
        self.ia_rows(
            &format!("SELECT {ADAPTER_COLUMNS} FROM ia_adapters WHERE adapter_id = ?1"),
            &[&id.as_str()],
            decode_adapter,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_institutional_adapters(&self) -> Result<Vec<InstitutionalAdapter>, MetaError> {
        self.ia_rows(
            &format!("SELECT {ADAPTER_COLUMNS} FROM ia_adapters ORDER BY rowid"),
            &[],
            decode_adapter,
        )
    }

    pub fn get_write_intent(&self, id: &OpaqueId) -> Result<ExternalWriteIntent, MetaError> {
        self.ia_rows(
            &format!("SELECT {INTENT_COLUMNS} FROM ia_intents WHERE intent_id = ?1"),
            &[&id.as_str()],
            decode_intent,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_write_intents(
        &self,
        adapter_id: Option<&OpaqueId>,
    ) -> Result<Vec<ExternalWriteIntent>, MetaError> {
        match adapter_id {
            Some(a) => self.ia_rows(
                &format!(
                    "SELECT {INTENT_COLUMNS} FROM ia_intents WHERE adapter_id = ?1 ORDER BY rowid"
                ),
                &[&a.as_str()],
                decode_intent,
            ),
            None => self.ia_rows(
                &format!("SELECT {INTENT_COLUMNS} FROM ia_intents ORDER BY rowid"),
                &[],
                decode_intent,
            ),
        }
    }

    pub fn list_adapter_receipts(
        &self,
        adapter_id: Option<&OpaqueId>,
    ) -> Result<Vec<AdapterReceipt>, MetaError> {
        match adapter_id {
            Some(a) => self.ia_rows(
                &format!(
                    "SELECT {RECEIPT_COLUMNS} FROM ia_receipts WHERE adapter_id = ?1 ORDER BY rowid"
                ),
                &[&a.as_str()],
                decode_receipt,
            ),
            None => self.ia_rows(
                &format!("SELECT {RECEIPT_COLUMNS} FROM ia_receipts ORDER BY rowid"),
                &[],
                decode_receipt,
            ),
        }
    }

    /// Cross-row invariants, re-verified after restore: adapters name an
    /// existing Project in scope; intents name an existing adapter of the
    /// same Project; receipts name an existing adapter.
    pub fn verify_institutional_consistency(&self) -> Result<(), MetaError> {
        let adapters: HashMap<String, InstitutionalAdapter> = self
            .list_institutional_adapters()?
            .into_iter()
            .map(|a| (a.header.id.as_str().to_owned(), a))
            .collect();
        for a in adapters.values() {
            let project = match self.get_project(&a.project_id) {
                Ok(p) => p,
                Err(MetaError::NotFound) => {
                    return Err(corrupt("adapter names a missing project".to_owned()));
                }
                Err(e) => return Err(e),
            };
            if project.header.realm_id != a.header.realm_id
                || project.header.authority_scope_id != a.header.authority_scope_id
            {
                return Err(corrupt("adapter is outside its project's scope".to_owned()));
            }
        }
        for i in self.list_write_intents(None)? {
            let a = adapters
                .get(i.adapter_id.as_str())
                .ok_or_else(|| corrupt("intent of a missing adapter".to_owned()))?;
            if a.project_id != i.project_id {
                return Err(corrupt("intent crosses projects".to_owned()));
            }
        }
        for r in self.list_adapter_receipts(None)? {
            if !adapters.contains_key(r.adapter_id.as_str()) {
                return Err(corrupt("receipt of a missing adapter".to_owned()));
            }
        }
        Ok(())
    }

    /// Backup families (credential handles are names, never secrets).
    pub fn institutional_backup_families(
        &self,
    ) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        Ok(vec![
            (
                "ia_adapters",
                v(serde_json::to_value(self.list_institutional_adapters()?))?,
            ),
            (
                "ia_intents",
                v(serde_json::to_value(self.list_write_intents(None)?))?,
            ),
            (
                "ia_receipts",
                v(serde_json::to_value(self.list_adapter_receipts(None)?))?,
            ),
        ])
    }

    pub fn restore_institutional_adapter_row(
        &self,
        a: &InstitutionalAdapter,
    ) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ia_write_adapter_on(self.conn(), a, None))
    }

    pub fn restore_write_intent_row(&self, i: &ExternalWriteIntent) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ia_write_intent_on(self.conn(), i, None))
    }

    pub fn restore_adapter_receipt_row(&self, r: &AdapterReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ia_insert_receipt_on(self.conn(), r))
    }
}

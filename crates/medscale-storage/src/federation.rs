//! Federation rows (Spec 091, storage schema v20).
//!
//! Same pattern as Specs 079-090. The institution identity's secret lives
//! only in `fed_identity.secret_hex` and is never exported in a backup (a
//! restored vault has no identity secret and must create a new identity,
//! which peers must trust again). Peers change by compare-and-set; an
//! import commits the peer's sequence advance, every imported item and the
//! receipt in one transaction, so a replayed or half-applied bundle is
//! impossible.

use std::collections::{HashMap, HashSet};

use medscale_contracts::federation::{
    FederationIdentity, FederationPeer, FederationReceipt, ImportedItem, ImportedState,
};
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v20 DDL, executed inside `begin/finish_migration(20)`.
pub(crate) const V20_DDL: &str = r"
CREATE TABLE IF NOT EXISTS fed_identity (
  identity_id TEXT PRIMARY KEY,
  body_json TEXT NOT NULL,
  secret_hex TEXT
);
CREATE TABLE IF NOT EXISTS fed_peers (
  peer_id TEXT PRIMARY KEY,
  institution_id TEXT NOT NULL UNIQUE,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS fed_imported (
  item_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  from_institution TEXT NOT NULL,
  digest_hex TEXT NOT NULL,
  state TEXT NOT NULL,
  body_json TEXT NOT NULL,
  content BLOB NOT NULL,
  UNIQUE(project_id, from_institution, digest_hex)
);
CREATE TABLE IF NOT EXISTS fed_receipts (
  receipt_id TEXT PRIMARY KEY,
  body_json TEXT NOT NULL
);
";

/// Names of the v20 tables (for rewind tests of earlier specs).
pub const FEDERATION_TABLES: [&str; 4] =
    ["fed_identity", "fed_peers", "fed_imported", "fed_receipts"];

/// Imported item plus its bytes (backup form: bytes as hex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedItemRow {
    pub item: ImportedItem,
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

fn check(column: &str, body: &str, what: &str) -> Result<(), MetaError> {
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

fn from_hex(hex: &str) -> Result<Vec<u8>, MetaError> {
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) || !hex.len().is_multiple_of(2) {
        return Err(corrupt("bad hex".to_owned()));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| corrupt("bad hex".to_owned())))
        .collect()
}

/// Active items carry exactly their bytes; tombstoned items carry none.
fn check_item_bytes(item: &ImportedItem, bytes: &[u8]) -> Result<(), String> {
    match item.state {
        ImportedState::Active if DigestSha256::of(bytes) == item.digest => Ok(()),
        ImportedState::Tombstoned if bytes.is_empty() => Ok(()),
        _ => Err("imported item bytes do not match its state and digest".to_owned()),
    }
}

fn decode_identity(row: &rusqlite::Row<'_>) -> Result<FederationIdentity, MetaError> {
    let id: String = row.get(0)?;
    let body: String = row.get(1)?;
    let v: FederationIdentity = from_json(&body, "federation identity")?;
    check(&id, v.header.id.as_str(), "federation identity")?;
    Ok(v)
}

fn decode_peer(row: &rusqlite::Row<'_>) -> Result<FederationPeer, MetaError> {
    let id: String = row.get(0)?;
    let institution: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let v: FederationPeer = from_json(&body, "federation peer")?;
    check(&id, v.header.id.as_str(), "federation peer")?;
    check(&institution, &v.institution_id, "federation peer")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("peer revision disagrees".to_owned()));
    }
    v.validate()
        .map_err(|e| corrupt(format!("federation peer: {e}")))?;
    Ok(v)
}

fn decode_item(row: &rusqlite::Row<'_>) -> Result<(ImportedItem, Vec<u8>), MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let from: String = row.get(2)?;
    let digest: String = row.get(3)?;
    let state: String = row.get(4)?;
    let body: String = row.get(5)?;
    let content: Vec<u8> = row.get(6)?;
    let v: ImportedItem = from_json(&body, "imported item")?;
    check(&id, v.header.id.as_str(), "imported item")?;
    check(&project_id, v.project_id.as_str(), "imported item")?;
    check(&from, &v.from_institution, "imported item")?;
    check(&digest, &v.digest.to_hex(), "imported item")?;
    check(&state, v.state.as_str(), "imported item")?;
    check_item_bytes(&v, &content).map_err(corrupt)?;
    Ok((v, content))
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<FederationReceipt, MetaError> {
    let id: String = row.get(0)?;
    let body: String = row.get(1)?;
    let v: FederationReceipt = from_json(&body, "federation receipt")?;
    check(&id, v.header.id.as_str(), "federation receipt")?;
    Ok(v)
}

const PEER_COLUMNS: &str = "peer_id, institution_id, revision, body_json";
const ITEM_COLUMNS: &str =
    "item_id, project_id, from_institution, digest_hex, state, body_json, content";

/// One federation change and its receipt, committed together.
#[derive(Debug, Default)]
pub struct FederationChange<'a> {
    /// `(identity, secret_hex)`; inserted once.
    pub identity: Option<(&'a FederationIdentity, &'a str)>,
    pub peer: Option<(&'a FederationPeer, Option<u64>)>,
    pub new_items: Vec<(&'a ImportedItem, &'a [u8])>,
    /// Items (by id) to tombstone: bytes erased, state changed.
    pub tombstone_items: Vec<&'a ImportedItem>,
    pub receipt: Option<&'a FederationReceipt>,
}

impl SqliteMetaStore {
    fn fed_rows<T>(
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

    /// Allocates one `prefix-N` federation id from a durable sequence.
    pub fn alloc_federation_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("fed_seq_{prefix}");
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

    fn fed_write_peer_on(
        conn: &rusqlite::Connection,
        p: &FederationPeer,
        expected: Option<u64>,
    ) -> Result<(), MetaError> {
        p.validate().map_err(|e| invalid("federation peer", e))?;
        let rev = i64::try_from(p.revision)
            .map_err(|_| invalid("federation peer", "revision overflow".to_owned()))?;
        match expected {
            None => map_insert(
                conn.execute(
                    "INSERT INTO fed_peers(peer_id, institution_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                    params![p.header.id.as_str(), p.institution_id, rev, to_json(p)?],
                ),
                "federation peer",
                &p.institution_id,
            ),
            Some(e) => {
                if p.revision != e + 1 {
                    return Err(invalid("federation peer", "revision must advance by one".to_owned()));
                }
                let e = i64::try_from(e)
                    .map_err(|_| invalid("federation peer", "revision overflow".to_owned()))?;
                let changed = conn.execute(
                    "UPDATE fed_peers SET revision = ?1, body_json = ?2 WHERE peer_id = ?3 AND revision = ?4",
                    params![rev, to_json(p)?, p.header.id.as_str(), e],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict("peer changed concurrently".to_owned()))
                }
            }
        }
    }

    fn fed_insert_item_on(
        conn: &rusqlite::Connection,
        item: &ImportedItem,
        bytes: &[u8],
    ) -> Result<(), MetaError> {
        check_item_bytes(item, bytes).map_err(|e| invalid("imported item", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO fed_imported(item_id, project_id, from_institution, digest_hex, state, body_json, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    item.header.id.as_str(),
                    item.project_id.as_str(),
                    item.from_institution,
                    item.digest.to_hex(),
                    item.state.as_str(),
                    to_json(item)?,
                    bytes
                ],
            ),
            "imported item",
            item.header.id.as_str(),
        )
    }

    fn fed_insert_receipt_on(
        conn: &rusqlite::Connection,
        r: &FederationReceipt,
    ) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO fed_receipts(receipt_id, body_json) VALUES (?1, ?2)",
                params![r.header.id.as_str(), to_json(r)?],
            ),
            "federation receipt",
            r.header.id.as_str(),
        )
    }

    fn fed_apply(
        &self,
        change: &FederationChange<'_>,
        receipt: &FederationReceipt,
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        if let Some((identity, secret)) = change.identity {
            let existing: i64 =
                tx.query_row("SELECT COUNT(*) FROM fed_identity", [], |row| row.get(0))?;
            if existing != 0 {
                return Err(MetaError::Conflict("an identity already exists".to_owned()));
            }
            tx.execute(
                "INSERT INTO fed_identity(identity_id, body_json, secret_hex) VALUES (?1, ?2, ?3)",
                params![identity.header.id.as_str(), to_json(identity)?, secret],
            )?;
        }
        if let Some((p, e)) = change.peer {
            Self::fed_write_peer_on(&tx, p, e)?;
        }
        for (item, bytes) in &change.new_items {
            Self::fed_insert_item_on(&tx, item, bytes)?;
        }
        for item in &change.tombstone_items {
            let mut t = (*item).clone();
            t.state = ImportedState::Tombstoned;
            let changed = tx.execute(
                "UPDATE fed_imported SET state = 'tombstoned', body_json = ?1, content = X'' WHERE item_id = ?2 AND state = 'active'",
                params![to_json(&t)?, t.header.id.as_str()],
            )?;
            if changed != 1 {
                return Err(MetaError::Conflict(
                    "item was already tombstoned".to_owned(),
                ));
            }
        }
        Self::fed_insert_receipt_on(&tx, receipt)?;
        tx.commit()?;
        Ok(())
    }

    /// Commits one change with its receipt in a single transaction.
    pub fn commit_federation_change(&self, change: &FederationChange<'_>) -> Result<(), MetaError> {
        let receipt = change.receipt.ok_or_else(|| {
            invalid(
                "federation change",
                "a change carries its receipt".to_owned(),
            )
        })?;
        self.fed_apply(change, receipt)
    }

    /// The identity and (when present) its secret.
    pub fn get_federation_identity(
        &self,
    ) -> Result<Option<(FederationIdentity, Option<String>)>, MetaError> {
        let row: Option<(String, String, Option<String>)> = self
            .conn()
            .query_row(
                "SELECT identity_id, body_json, secret_hex FROM fed_identity",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some((id, body, secret)) => {
                let v: FederationIdentity = from_json(&body, "federation identity")?;
                check(&id, v.header.id.as_str(), "federation identity")?;
                Ok(Some((v, secret)))
            }
        }
    }

    pub fn get_federation_peer(&self, institution_id: &str) -> Result<FederationPeer, MetaError> {
        self.fed_rows(
            &format!("SELECT {PEER_COLUMNS} FROM fed_peers WHERE institution_id = ?1"),
            &[&institution_id],
            decode_peer,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_federation_peers(&self) -> Result<Vec<FederationPeer>, MetaError> {
        self.fed_rows(
            &format!("SELECT {PEER_COLUMNS} FROM fed_peers ORDER BY rowid"),
            &[],
            decode_peer,
        )
    }

    pub fn list_imported_items(&self) -> Result<Vec<ImportedItemRow>, MetaError> {
        Ok(self
            .fed_rows(
                &format!("SELECT {ITEM_COLUMNS} FROM fed_imported ORDER BY rowid"),
                &[],
                decode_item,
            )?
            .into_iter()
            .map(|(item, bytes)| ImportedItemRow {
                item,
                content_hex: to_hex(&bytes),
            })
            .collect())
    }

    pub fn list_federation_receipts(&self) -> Result<Vec<FederationReceipt>, MetaError> {
        self.fed_rows(
            "SELECT receipt_id, body_json FROM fed_receipts ORDER BY rowid",
            &[],
            decode_receipt,
        )
    }

    /// Cross-row invariants, re-verified after restore: imported items name
    /// an existing Project and a known peer; no duplicate digest per
    /// (Project, peer).
    pub fn verify_federation_consistency(&self) -> Result<(), MetaError> {
        let peers: HashMap<String, FederationPeer> = self
            .list_federation_peers()?
            .into_iter()
            .map(|p| (p.institution_id.clone(), p))
            .collect();
        let mut seen = HashSet::new();
        for row in self.list_imported_items()? {
            let item = &row.item;
            if !peers.contains_key(&item.from_institution) {
                return Err(corrupt("imported item from an unknown peer".to_owned()));
            }
            if self.get_project(&item.project_id).is_err() {
                return Err(corrupt("imported item names a missing project".to_owned()));
            }
            if !seen.insert((
                item.project_id.as_str().to_owned(),
                item.from_institution.clone(),
                item.digest.to_hex(),
            )) {
                return Err(corrupt("duplicate imported item".to_owned()));
            }
        }
        Ok(())
    }

    /// Backup families. The identity is exported without its secret.
    pub fn federation_backup_families(
        &self,
    ) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        let identity: Vec<FederationIdentity> = self.fed_rows(
            "SELECT identity_id, body_json FROM fed_identity",
            &[],
            decode_identity,
        )?;
        Ok(vec![
            ("fed_identity", v(serde_json::to_value(identity))?),
            (
                "fed_peers",
                v(serde_json::to_value(self.list_federation_peers()?))?,
            ),
            (
                "fed_imported",
                v(serde_json::to_value(self.list_imported_items()?))?,
            ),
            (
                "fed_receipts",
                v(serde_json::to_value(self.list_federation_receipts()?))?,
            ),
        ])
    }

    /// Restores the public identity without a secret.
    pub fn restore_federation_identity_row(&self, i: &FederationIdentity) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO fed_identity(identity_id, body_json, secret_hex) VALUES (?1, ?2, NULL)",
                params![i.header.id.as_str(), to_json(i)?],
            ),
            "federation identity",
            i.header.id.as_str(),
        ))
    }

    pub fn restore_federation_peer_row(&self, p: &FederationPeer) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::fed_write_peer_on(self.conn(), p, None))
    }

    pub fn restore_imported_item_row(&self, row: &ImportedItemRow) -> Result<(), MetaError> {
        let bytes = from_hex(&row.content_hex)?;
        restore_conflict_is_corrupt(Self::fed_insert_item_on(self.conn(), &row.item, &bytes))
    }

    pub fn restore_federation_receipt_row(&self, r: &FederationReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::fed_insert_receipt_on(self.conn(), r))
    }
}

//! Privacy Gate durable rows (Spec 079, storage schema v8).
//!
//! Dedicated tables inside the existing encrypted metadata DB. Each row keeps
//! the full contract value as validated JSON (`body_json`) plus the columns
//! that queries and uniqueness need. On every read the columns are checked
//! against the body, so a hand-edited row fails closed instead of being
//! trusted.
//!
//! No row stores a detected plaintext value. Pseudonym entries store only an
//! AEAD-sealed value (`WrappedBlob`) whose key lives in the `KeyStore`, never
//! in this database or in backups (`security.md` T4).
//!
//! Restore uses plain `INSERT` (never `INSERT OR REPLACE`) so a tampered
//! snapshot with duplicate ids fails closed, as in Specs 076-078.

use std::collections::{HashMap, HashSet};

use medscale_contracts::objects::OpaqueId;
use medscale_contracts::privacy_gate::{
    ArtifactClassification, DeidReceipt, DeidReceiptStatus, EgressDecision, PrivacyPolicyProfile,
    PrivacyProfileStatus, PseudonymMapRef, PseudonymMapStatus, ReidentificationAudit, is_pseudonym,
};
use medscale_keys::WrappedBlob;
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v8 DDL, executed inside `begin/finish_migration(8)`.
pub(crate) const V8_DDL: &str = r"
CREATE TABLE IF NOT EXISTS privacy_classifications (
  classification_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_privacy_classifications_artifact
  ON privacy_classifications(project_id, artifact_id);
CREATE TABLE IF NOT EXISTS privacy_profiles (
  profile_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_privacy_profiles_project
  ON privacy_profiles(project_id);
CREATE TABLE IF NOT EXISTS privacy_deid_receipts (
  receipt_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  output_artifact_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_privacy_deid_receipts_output
  ON privacy_deid_receipts(output_artifact_id);
CREATE INDEX IF NOT EXISTS idx_privacy_deid_receipts_project
  ON privacy_deid_receipts(project_id);
CREATE TABLE IF NOT EXISTS privacy_pseudonym_maps (
  map_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_privacy_pseudonym_maps_project
  ON privacy_pseudonym_maps(project_id);
CREATE TABLE IF NOT EXISTS privacy_pseudonym_entries (
  map_id TEXT NOT NULL,
  pseudonym TEXT NOT NULL,
  body_json TEXT NOT NULL,
  PRIMARY KEY (map_id, pseudonym)
);
CREATE TABLE IF NOT EXISTS privacy_reid_audit (
  audit_id TEXT PRIMARY KEY,
  map_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_privacy_reid_audit_map
  ON privacy_reid_audit(map_id);
CREATE TABLE IF NOT EXISTS privacy_egress_decisions (
  decision_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_privacy_egress_decisions_artifact
  ON privacy_egress_decisions(project_id, artifact_id);
";

/// One sealed reverse-map entry. The plaintext is recoverable only with the
/// map key held in the `KeyStore`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PseudonymEntryRow {
    pub map_id: OpaqueId,
    pub pseudonym: String,
    pub sealed: WrappedBlob,
}

/// Everything one privacy transform persists, written in one transaction.
#[derive(Debug, Clone)]
pub struct DeidTransformCommit {
    pub receipt: DeidReceipt,
    pub output_classification: ArtifactClassification,
    /// New sealed entries; entries already present for the same
    /// `(map_id, pseudonym)` are kept, not replaced.
    pub new_entries: Vec<PseudonymEntryRow>,
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

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

/// Fails closed when an indexed column disagrees with the JSON body.
fn check_column(column: &str, body: &str, what: &str) -> Result<(), MetaError> {
    if column == body {
        Ok(())
    } else {
        Err(corrupt(format!(
            "{what} row columns disagree with its body"
        )))
    }
}

fn check_revision_column(column: i64, body: u64, what: &str) -> Result<(), MetaError> {
    if column == revision_to_i64(body) {
        Ok(())
    } else {
        Err(corrupt(format!(
            "{what} row revision disagrees with its body"
        )))
    }
}

fn stale(expected: u64, current: u64) -> MetaError {
    MetaError::Conflict(format!(
        "stale revision: expected {expected}, current is {current}"
    ))
}

// ---------------------------------------------------------------------------
// row decoding
// ---------------------------------------------------------------------------

fn decode_classification(row: &rusqlite::Row<'_>) -> Result<ArtifactClassification, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let artifact_id: String = row.get(2)?;
    let revision: i64 = row.get(3)?;
    let body: String = row.get(4)?;
    let value: ArtifactClassification = from_json(&body, "classification")?;
    check_column(&id, value.header.id.as_str(), "classification")?;
    check_column(&project_id, value.project_id.as_str(), "classification")?;
    check_column(&artifact_id, value.artifact_id.as_str(), "classification")?;
    check_revision_column(revision, value.revision, "classification")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("classification: {e}")))?;
    Ok(value)
}

fn decode_profile(row: &rusqlite::Row<'_>) -> Result<PrivacyPolicyProfile, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let value: PrivacyPolicyProfile = from_json(&body, "privacy profile")?;
    check_column(&id, value.header.id.as_str(), "privacy profile")?;
    check_column(&project_id, value.project_id.as_str(), "privacy profile")?;
    check_revision_column(revision, value.revision, "privacy profile")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("privacy profile: {e}")))?;
    Ok(value)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<DeidReceipt, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let output_artifact_id: String = row.get(2)?;
    let revision: i64 = row.get(3)?;
    let body: String = row.get(4)?;
    let value: DeidReceipt = from_json(&body, "deid receipt")?;
    check_column(&id, value.header.id.as_str(), "deid receipt")?;
    check_column(&project_id, value.project_id.as_str(), "deid receipt")?;
    check_column(
        &output_artifact_id,
        value.output_artifact_id.as_str(),
        "deid receipt",
    )?;
    check_revision_column(revision, value.revision, "deid receipt")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("deid receipt: {e}")))?;
    Ok(value)
}

fn decode_map(row: &rusqlite::Row<'_>) -> Result<PseudonymMapRef, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let body: String = row.get(3)?;
    let value: PseudonymMapRef = from_json(&body, "pseudonym map")?;
    check_column(&id, value.header.id.as_str(), "pseudonym map")?;
    check_column(&project_id, value.project_id.as_str(), "pseudonym map")?;
    check_revision_column(revision, value.revision, "pseudonym map")?;
    Ok(value)
}

fn decode_entry(row: &rusqlite::Row<'_>) -> Result<PseudonymEntryRow, MetaError> {
    let map_id: String = row.get(0)?;
    let pseudonym: String = row.get(1)?;
    let body: String = row.get(2)?;
    let value: PseudonymEntryRow = from_json(&body, "pseudonym entry")?;
    check_column(&map_id, value.map_id.as_str(), "pseudonym entry")?;
    check_column(&pseudonym, &value.pseudonym, "pseudonym entry")?;
    if !is_pseudonym(&value.pseudonym) {
        return Err(corrupt("pseudonym entry has an invalid shape".to_owned()));
    }
    Ok(value)
}

fn decode_audit(row: &rusqlite::Row<'_>) -> Result<ReidentificationAudit, MetaError> {
    let id: String = row.get(0)?;
    let map_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let value: ReidentificationAudit = from_json(&body, "re-identification audit")?;
    check_column(&id, value.header.id.as_str(), "re-identification audit")?;
    check_column(&map_id, value.map_id.as_str(), "re-identification audit")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("re-identification audit: {e}")))?;
    Ok(value)
}

fn decode_decision(row: &rusqlite::Row<'_>) -> Result<EgressDecision, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let artifact_id: String = row.get(2)?;
    let body: String = row.get(3)?;
    let value: EgressDecision = from_json(&body, "egress decision")?;
    check_column(&id, value.header.id.as_str(), "egress decision")?;
    check_column(&project_id, value.project_id.as_str(), "egress decision")?;
    check_column(&artifact_id, value.artifact_id.as_str(), "egress decision")?;
    value
        .validate()
        .map_err(|e| corrupt(format!("egress decision: {e}")))?;
    Ok(value)
}

const CLASSIFICATION_COLUMNS: &str =
    "classification_id, project_id, artifact_id, revision, body_json";
const PROFILE_COLUMNS: &str = "profile_id, project_id, revision, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, project_id, output_artifact_id, revision, body_json";
const MAP_COLUMNS: &str = "map_id, project_id, revision, body_json";
const ENTRY_COLUMNS: &str = "map_id, pseudonym, body_json";
const AUDIT_COLUMNS: &str = "audit_id, map_id, body_json";
const DECISION_COLUMNS: &str = "decision_id, project_id, artifact_id, body_json";

impl SqliteMetaStore {
    fn query_rows<T>(
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

    fn query_one<T>(
        &self,
        sql: &str,
        args: &[&dyn rusqlite::ToSql],
        decode: fn(&rusqlite::Row<'_>) -> Result<T, MetaError>,
    ) -> Result<Option<T>, MetaError> {
        Ok(self.query_rows(sql, args, decode)?.into_iter().next())
    }

    /// Allocates one `prefix-N` Privacy Gate id from a durable sequence.
    pub fn alloc_privacy_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("privacy_gate_seq_{prefix}");
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

    // -- classifications ----------------------------------------------------

    fn insert_classification_on(
        conn: &rusqlite::Connection,
        value: &ArtifactClassification,
    ) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("classification: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO privacy_classifications(classification_id, project_id, artifact_id, revision, body_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    value.artifact_id.as_str(),
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "classification",
            value.artifact_id.as_str(),
        )
    }

    /// Inserts the first classification of an artifact in a Project.
    pub fn insert_classification(&self, value: &ArtifactClassification) -> Result<(), MetaError> {
        Self::insert_classification_on(self.conn(), value)
    }

    /// Replaces an artifact's classification (CAS on `expected_revision`).
    /// The caller supplies the full next value; its revision must be
    /// `expected_revision + 1`.
    pub fn update_classification(
        &self,
        next: &ArtifactClassification,
        expected_revision: u64,
    ) -> Result<(), MetaError> {
        next.validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("classification: {e}")))?;
        if next.revision != expected_revision + 1 {
            return Err(stale(expected_revision, next.revision.saturating_sub(1)));
        }
        let updated = self.conn().execute(
            "UPDATE privacy_classifications SET revision = ?1, body_json = ?2
             WHERE classification_id = ?3 AND project_id = ?4 AND artifact_id = ?5 AND revision = ?6",
            params![
                revision_to_i64(next.revision),
                to_json(next)?,
                next.header.id.as_str(),
                next.project_id.as_str(),
                next.artifact_id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict(
                "stale or missing classification".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn get_classification(
        &self,
        project_id: &OpaqueId,
        artifact_id: &OpaqueId,
    ) -> Result<Option<ArtifactClassification>, MetaError> {
        self.query_one(
            &format!(
                "SELECT {CLASSIFICATION_COLUMNS} FROM privacy_classifications WHERE project_id = ?1 AND artifact_id = ?2"
            ),
            &[&project_id.as_str(), &artifact_id.as_str()],
            decode_classification,
        )
    }

    pub fn list_classifications(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<ArtifactClassification>, MetaError> {
        self.query_rows(
            &format!(
                "SELECT {CLASSIFICATION_COLUMNS} FROM privacy_classifications WHERE project_id = ?1 ORDER BY classification_id LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_classification,
        )
    }

    // -- profiles -------------------------------------------------------------

    pub fn insert_privacy_profile(&self, value: &PrivacyPolicyProfile) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("privacy profile: {e}")))?;
        map_insert(
            self.conn().execute(
                "INSERT INTO privacy_profiles(profile_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "privacy profile",
            value.header.id.as_str(),
        )
    }

    pub fn get_privacy_profile(&self, id: &OpaqueId) -> Result<PrivacyPolicyProfile, MetaError> {
        self.query_one(
            &format!("SELECT {PROFILE_COLUMNS} FROM privacy_profiles WHERE profile_id = ?1"),
            &[&id.as_str()],
            decode_profile,
        )?
        .ok_or(MetaError::NotFound)
    }

    pub fn list_privacy_profiles(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<PrivacyPolicyProfile>, MetaError> {
        self.query_rows(
            &format!(
                "SELECT {PROFILE_COLUMNS} FROM privacy_profiles WHERE project_id = ?1 ORDER BY profile_id LIMIT 200"
            ),
            &[&project_id.as_str()],
            decode_profile,
        )
    }

    /// Revokes a profile (CAS on `expected_revision`). Revocation is terminal.
    pub fn revoke_privacy_profile(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<PrivacyPolicyProfile, MetaError> {
        let mut value = self.get_privacy_profile(id)?;
        if value.revision != expected_revision {
            return Err(stale(expected_revision, value.revision));
        }
        if value.status == PrivacyProfileStatus::Revoked {
            return Err(MetaError::Conflict("profile already revoked".to_owned()));
        }
        value.status = PrivacyProfileStatus::Revoked;
        value.revision = expected_revision + 1;
        let updated = self.conn().execute(
            "UPDATE privacy_profiles SET revision = ?1, body_json = ?2 WHERE profile_id = ?3 AND revision = ?4",
            params![
                revision_to_i64(value.revision),
                to_json(&value)?,
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent profile revoke".to_owned()));
        }
        Ok(value)
    }

    // -- receipts + transform commit -------------------------------------------

    fn insert_receipt_on(
        conn: &rusqlite::Connection,
        value: &DeidReceipt,
    ) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("deid receipt: {e}")))?;
        map_insert(
            conn.execute(
                "INSERT INTO privacy_deid_receipts(receipt_id, project_id, output_artifact_id, revision, body_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    value.output_artifact_id.as_str(),
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "deid receipt",
            value.header.id.as_str(),
        )
    }

    fn insert_entry_on(
        conn: &rusqlite::Connection,
        entry: &PseudonymEntryRow,
        keep_existing: bool,
    ) -> Result<bool, MetaError> {
        if !is_pseudonym(&entry.pseudonym) {
            return Err(MetaError::UnsupportedSchema(
                "pseudonym entry has an invalid shape".to_owned(),
            ));
        }
        let verb = if keep_existing {
            "INSERT OR IGNORE"
        } else {
            "INSERT"
        };
        let result = conn.execute(
            &format!(
                "{verb} INTO privacy_pseudonym_entries(map_id, pseudonym, body_json) VALUES (?1, ?2, ?3)"
            ),
            params![entry.map_id.as_str(), entry.pseudonym, to_json(entry)?],
        );
        match result {
            Ok(n) => Ok(n == 1),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate pseudonym entry in map {}",
                entry.map_id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Writes one transform's receipt, output classification and new sealed
    /// entries in a single transaction, and refreshes the map's entry count.
    /// Any failure rolls the whole transform back.
    pub fn commit_deid_transform(&self, commit: &DeidTransformCommit) -> Result<(), MetaError> {
        let receipt = &commit.receipt;
        if commit.output_classification.artifact_id != receipt.output_artifact_id
            || commit.output_classification.deid_receipt_id.as_ref() != Some(&receipt.header.id)
            || commit.output_classification.project_id != receipt.project_id
        {
            return Err(MetaError::UnsupportedSchema(
                "output classification does not match its receipt".to_owned(),
            ));
        }
        let tx = self.conn().unchecked_transaction()?;
        Self::insert_receipt_on(&tx, receipt)?;
        Self::insert_classification_on(&tx, &commit.output_classification)?;
        if let Some(map_id) = &receipt.pseudonym_map_id {
            let map_body: String = tx
                .query_row(
                    "SELECT body_json FROM privacy_pseudonym_maps WHERE map_id = ?1",
                    params![map_id.as_str()],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(MetaError::NotFound)?;
            let mut map: PseudonymMapRef = from_json(&map_body, "pseudonym map")?;
            if map.status != PseudonymMapStatus::Active {
                return Err(MetaError::Conflict("pseudonym map is revoked".to_owned()));
            }
            for entry in &commit.new_entries {
                if &entry.map_id != map_id {
                    return Err(MetaError::UnsupportedSchema(
                        "entry names a different map".to_owned(),
                    ));
                }
                Self::insert_entry_on(&tx, entry, true)?;
            }
            let count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM privacy_pseudonym_entries WHERE map_id = ?1",
                params![map_id.as_str()],
                |row| row.get(0),
            )?;
            let count = u32::try_from(count).unwrap_or(u32::MAX);
            if count != map.entry_count {
                let expected = map.revision;
                map.entry_count = count;
                map.revision = expected + 1;
                tx.execute(
                    "UPDATE privacy_pseudonym_maps SET revision = ?1, body_json = ?2 WHERE map_id = ?3 AND revision = ?4",
                    params![
                        revision_to_i64(map.revision),
                        to_json(&map)?,
                        map_id.as_str(),
                        revision_to_i64(expected),
                    ],
                )?;
            }
        } else if !commit.new_entries.is_empty() {
            return Err(MetaError::UnsupportedSchema(
                "pseudonym entries without a map".to_owned(),
            ));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_deid_receipt(&self, id: &OpaqueId) -> Result<DeidReceipt, MetaError> {
        self.query_one(
            &format!("SELECT {RECEIPT_COLUMNS} FROM privacy_deid_receipts WHERE receipt_id = ?1"),
            &[&id.as_str()],
            decode_receipt,
        )?
        .ok_or(MetaError::NotFound)
    }

    pub fn list_deid_receipts(&self, project_id: &OpaqueId) -> Result<Vec<DeidReceipt>, MetaError> {
        self.query_rows(
            &format!(
                "SELECT {RECEIPT_COLUMNS} FROM privacy_deid_receipts WHERE project_id = ?1 ORDER BY receipt_id LIMIT 500"
            ),
            &[&project_id.as_str()],
            decode_receipt,
        )
    }

    /// Revokes a receipt (CAS on `expected_revision`). Revocation is terminal.
    pub fn revoke_deid_receipt(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<DeidReceipt, MetaError> {
        let mut value = self.get_deid_receipt(id)?;
        if value.revision != expected_revision {
            return Err(stale(expected_revision, value.revision));
        }
        if value.status == DeidReceiptStatus::Revoked {
            return Err(MetaError::Conflict("receipt already revoked".to_owned()));
        }
        value.status = DeidReceiptStatus::Revoked;
        value.revision = expected_revision + 1;
        let updated = self.conn().execute(
            "UPDATE privacy_deid_receipts SET revision = ?1, body_json = ?2 WHERE receipt_id = ?3 AND revision = ?4",
            params![
                revision_to_i64(value.revision),
                to_json(&value)?,
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent receipt revoke".to_owned()));
        }
        Ok(value)
    }

    // -- pseudonym maps ---------------------------------------------------------

    pub fn insert_pseudonym_map(&self, value: &PseudonymMapRef) -> Result<(), MetaError> {
        if value.entry_count != 0 {
            return Err(MetaError::UnsupportedSchema(
                "a new pseudonym map has no entries".to_owned(),
            ));
        }
        map_insert(
            self.conn().execute(
                "INSERT INTO privacy_pseudonym_maps(map_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "pseudonym map",
            value.header.id.as_str(),
        )
    }

    pub fn get_pseudonym_map(&self, id: &OpaqueId) -> Result<PseudonymMapRef, MetaError> {
        self.query_one(
            &format!("SELECT {MAP_COLUMNS} FROM privacy_pseudonym_maps WHERE map_id = ?1"),
            &[&id.as_str()],
            decode_map,
        )?
        .ok_or(MetaError::NotFound)
    }

    pub fn list_pseudonym_maps(
        &self,
        project_id: &OpaqueId,
    ) -> Result<Vec<PseudonymMapRef>, MetaError> {
        self.query_rows(
            &format!(
                "SELECT {MAP_COLUMNS} FROM privacy_pseudonym_maps WHERE project_id = ?1 ORDER BY map_id LIMIT 200"
            ),
            &[&project_id.as_str()],
            decode_map,
        )
    }

    /// Marks a map revoked (CAS). The caller destroys the key separately.
    pub fn revoke_pseudonym_map(
        &self,
        id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<PseudonymMapRef, MetaError> {
        let mut value = self.get_pseudonym_map(id)?;
        if value.revision != expected_revision {
            return Err(stale(expected_revision, value.revision));
        }
        if value.status == PseudonymMapStatus::Revoked {
            return Err(MetaError::Conflict(
                "pseudonym map already revoked".to_owned(),
            ));
        }
        value.status = PseudonymMapStatus::Revoked;
        value.revision = expected_revision + 1;
        let updated = self.conn().execute(
            "UPDATE privacy_pseudonym_maps SET revision = ?1, body_json = ?2 WHERE map_id = ?3 AND revision = ?4",
            params![
                revision_to_i64(value.revision),
                to_json(&value)?,
                id.as_str(),
                revision_to_i64(expected_revision),
            ],
        )?;
        if updated == 0 {
            return Err(MetaError::Conflict("concurrent map revoke".to_owned()));
        }
        Ok(value)
    }

    pub fn get_pseudonym_entry(
        &self,
        map_id: &OpaqueId,
        pseudonym: &str,
    ) -> Result<Option<PseudonymEntryRow>, MetaError> {
        self.query_one(
            &format!(
                "SELECT {ENTRY_COLUMNS} FROM privacy_pseudonym_entries WHERE map_id = ?1 AND pseudonym = ?2"
            ),
            &[&map_id.as_str(), &pseudonym],
            decode_entry,
        )
    }

    // -- audit + decisions --------------------------------------------------------

    pub fn insert_reid_audit(&self, value: &ReidentificationAudit) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("re-identification audit: {e}")))?;
        map_insert(
            self.conn().execute(
                "INSERT INTO privacy_reid_audit(audit_id, map_id, body_json) VALUES (?1, ?2, ?3)",
                params![
                    value.header.id.as_str(),
                    value.map_id.as_str(),
                    to_json(value)?,
                ],
            ),
            "re-identification audit",
            value.header.id.as_str(),
        )
    }

    pub fn list_reid_audit(
        &self,
        map_id: &OpaqueId,
    ) -> Result<Vec<ReidentificationAudit>, MetaError> {
        self.query_rows(
            &format!(
                "SELECT {AUDIT_COLUMNS} FROM privacy_reid_audit WHERE map_id = ?1 ORDER BY rowid LIMIT 500"
            ),
            &[&map_id.as_str()],
            decode_audit,
        )
    }

    pub fn insert_egress_decision(&self, value: &EgressDecision) -> Result<(), MetaError> {
        value
            .validate()
            .map_err(|e| MetaError::UnsupportedSchema(format!("egress decision: {e}")))?;
        map_insert(
            self.conn().execute(
                "INSERT INTO privacy_egress_decisions(decision_id, project_id, artifact_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    value.artifact_id.as_str(),
                    to_json(value)?,
                ],
            ),
            "egress decision",
            value.header.id.as_str(),
        )
    }

    pub fn list_egress_decisions(
        &self,
        project_id: &OpaqueId,
        artifact_id: Option<&OpaqueId>,
    ) -> Result<Vec<EgressDecision>, MetaError> {
        match artifact_id {
            Some(artifact_id) => self.query_rows(
                &format!(
                    "SELECT {DECISION_COLUMNS} FROM privacy_egress_decisions WHERE project_id = ?1 AND artifact_id = ?2 ORDER BY rowid LIMIT 500"
                ),
                &[&project_id.as_str(), &artifact_id.as_str()],
                decode_decision,
            ),
            None => self.query_rows(
                &format!(
                    "SELECT {DECISION_COLUMNS} FROM privacy_egress_decisions WHERE project_id = ?1 ORDER BY rowid LIMIT 500"
                ),
                &[&project_id.as_str()],
                decode_decision,
            ),
        }
    }

    // -- full scans, consistency, restore ----------------------------------------

    pub fn list_all_classifications(&self) -> Result<Vec<ArtifactClassification>, MetaError> {
        self.query_rows(
            &format!("SELECT {CLASSIFICATION_COLUMNS} FROM privacy_classifications ORDER BY classification_id"),
            &[],
            decode_classification,
        )
    }

    pub fn list_all_privacy_profiles(&self) -> Result<Vec<PrivacyPolicyProfile>, MetaError> {
        self.query_rows(
            &format!("SELECT {PROFILE_COLUMNS} FROM privacy_profiles ORDER BY profile_id"),
            &[],
            decode_profile,
        )
    }

    pub fn list_all_deid_receipts(&self) -> Result<Vec<DeidReceipt>, MetaError> {
        self.query_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM privacy_deid_receipts ORDER BY receipt_id"),
            &[],
            decode_receipt,
        )
    }

    pub fn list_all_pseudonym_maps(&self) -> Result<Vec<PseudonymMapRef>, MetaError> {
        self.query_rows(
            &format!("SELECT {MAP_COLUMNS} FROM privacy_pseudonym_maps ORDER BY map_id"),
            &[],
            decode_map,
        )
    }

    pub fn list_all_pseudonym_entries(&self) -> Result<Vec<PseudonymEntryRow>, MetaError> {
        self.query_rows(
            &format!(
                "SELECT {ENTRY_COLUMNS} FROM privacy_pseudonym_entries ORDER BY map_id, pseudonym"
            ),
            &[],
            decode_entry,
        )
    }

    pub fn list_all_reid_audit(&self) -> Result<Vec<ReidentificationAudit>, MetaError> {
        self.query_rows(
            &format!("SELECT {AUDIT_COLUMNS} FROM privacy_reid_audit ORDER BY rowid"),
            &[],
            decode_audit,
        )
    }

    pub fn list_all_egress_decisions(&self) -> Result<Vec<EgressDecision>, MetaError> {
        self.query_rows(
            &format!("SELECT {DECISION_COLUMNS} FROM privacy_egress_decisions ORDER BY rowid"),
            &[],
            decode_decision,
        )
    }

    /// Cross-row invariants (`migration.md`), re-verified after restore:
    ///
    /// - a `DeidReceipt`-basis classification names an existing receipt in
    ///   the same Project whose output is that artifact and whose output
    ///   class equals the classification's class;
    /// - every receipt has exactly one matching output classification, and
    ///   names an existing profile (and map, if any) in its Project;
    /// - every map's `entry_count` equals its entry rows, and every entry
    ///   belongs to an existing map;
    /// - audit and decision rows name existing maps/receipts in their Project.
    pub fn verify_privacy_gate_consistency(&self) -> Result<(), MetaError> {
        let profiles: HashMap<String, PrivacyPolicyProfile> = self
            .list_all_privacy_profiles()?
            .into_iter()
            .map(|p| (p.header.id.as_str().to_owned(), p))
            .collect();
        let maps: HashMap<String, PseudonymMapRef> = self
            .list_all_pseudonym_maps()?
            .into_iter()
            .map(|m| (m.header.id.as_str().to_owned(), m))
            .collect();
        let receipts: HashMap<String, DeidReceipt> = self
            .list_all_deid_receipts()?
            .into_iter()
            .map(|r| (r.header.id.as_str().to_owned(), r))
            .collect();

        let mut classified_receipts = HashSet::new();
        for row in self.list_all_classifications()? {
            let Some(receipt_id) = &row.deid_receipt_id else {
                continue;
            };
            let receipt = receipts.get(receipt_id.as_str()).ok_or_else(|| {
                corrupt(format!(
                    "classification {} names a missing receipt",
                    row.header.id.as_str()
                ))
            })?;
            if receipt.output_artifact_id != row.artifact_id
                || receipt.project_id != row.project_id
                || receipt.output_class != row.data_class
            {
                return Err(corrupt(format!(
                    "classification {} disagrees with receipt {}",
                    row.header.id.as_str(),
                    receipt_id.as_str()
                )));
            }
            classified_receipts.insert(receipt_id.as_str().to_owned());
        }

        for (receipt_id, receipt) in &receipts {
            if !classified_receipts.contains(receipt_id) {
                return Err(corrupt(format!(
                    "receipt {receipt_id} has no output classification"
                )));
            }
            let profile = profiles
                .get(receipt.profile_id.as_str())
                .ok_or_else(|| corrupt(format!("receipt {receipt_id} names a missing profile")))?;
            if profile.project_id != receipt.project_id
                || receipt.profile_revision > profile.revision
            {
                return Err(corrupt(format!(
                    "receipt {receipt_id} disagrees with its profile"
                )));
            }
            if let Some(map_id) = &receipt.pseudonym_map_id {
                let map = maps
                    .get(map_id.as_str())
                    .ok_or_else(|| corrupt(format!("receipt {receipt_id} names a missing map")))?;
                if map.project_id != receipt.project_id {
                    return Err(corrupt(format!(
                        "receipt {receipt_id} crosses Project boundaries"
                    )));
                }
            }
        }

        let mut entry_counts: HashMap<String, u32> = HashMap::new();
        for entry in self.list_all_pseudonym_entries()? {
            if !maps.contains_key(entry.map_id.as_str()) {
                return Err(corrupt(
                    "pseudonym entry belongs to a missing map".to_owned(),
                ));
            }
            *entry_counts
                .entry(entry.map_id.as_str().to_owned())
                .or_default() += 1;
        }
        for (map_id, map) in &maps {
            let actual = entry_counts.get(map_id).copied().unwrap_or(0);
            if actual != map.entry_count {
                return Err(corrupt(format!(
                    "pseudonym map {map_id} records {} entries but has {actual}",
                    map.entry_count
                )));
            }
        }

        for audit in self.list_all_reid_audit()? {
            let map = maps
                .get(audit.map_id.as_str())
                .ok_or_else(|| corrupt("re-identification audit names a missing map".to_owned()))?;
            if map.project_id != audit.project_id {
                return Err(corrupt(
                    "re-identification audit crosses Project boundaries".to_owned(),
                ));
            }
        }
        for decision in self.list_all_egress_decisions()? {
            if let Some(receipt_id) = &decision.deid_receipt_id {
                let receipt = receipts
                    .get(receipt_id.as_str())
                    .ok_or_else(|| corrupt("egress decision names a missing receipt".to_owned()))?;
                if receipt.project_id != decision.project_id
                    || receipt.output_artifact_id != decision.artifact_id
                {
                    return Err(corrupt(
                        "egress decision disagrees with its receipt".to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn restore_classification_row(
        &self,
        value: &ArtifactClassification,
    ) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_classification(value))
    }

    pub fn restore_privacy_profile_row(
        &self,
        value: &PrivacyPolicyProfile,
    ) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_privacy_profile(value))
    }

    pub fn restore_deid_receipt_row(&self, value: &DeidReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::insert_receipt_on(self.conn(), value))
    }

    /// Restores a map row exactly, including its recorded `entry_count`
    /// (checked against the restored entries by the consistency pass).
    pub fn restore_pseudonym_map_row(&self, value: &PseudonymMapRef) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO privacy_pseudonym_maps(map_id, project_id, revision, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![
                    value.header.id.as_str(),
                    value.project_id.as_str(),
                    revision_to_i64(value.revision),
                    to_json(value)?,
                ],
            ),
            "pseudonym map",
            value.header.id.as_str(),
        ))
    }

    pub fn restore_pseudonym_entry_row(&self, value: &PseudonymEntryRow) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::insert_entry_on(self.conn(), value, false).map(|_| ()))
    }

    pub fn restore_reid_audit_row(&self, value: &ReidentificationAudit) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_reid_audit(value))
    }

    pub fn restore_egress_decision_row(&self, value: &EgressDecision) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(self.insert_egress_decision(value))
    }
}

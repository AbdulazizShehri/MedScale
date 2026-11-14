//! Research Pack rows (Spec 089, storage schema v18).
//!
//! Same pattern as Specs 079-088. An install is one row per (Project,
//! Pack); artifacts change by compare-and-set on their revision; every
//! change is committed with its receipt. A Pack upgrade rewrites the
//! install and migrates every artifact of that Pack in the Project in one
//! transaction, so a crash never leaves a half-migrated Project.

use std::collections::HashMap;

use medscale_contracts::objects::OpaqueId;
use medscale_contracts::research_packs::{
    PackReceipt, ResearchArtifact, ResearchPackInstall, clinical_research_pack,
};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v18 DDL, executed inside `begin/finish_migration(18)`.
pub(crate) const V18_DDL: &str = r"
CREATE TABLE IF NOT EXISTS rp_installs (
  install_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  pack_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL,
  UNIQUE(project_id, pack_id)
);
CREATE TABLE IF NOT EXISTS rp_artifacts (
  artifact_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  pack_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_rp_artifacts_project ON rp_artifacts(project_id, pack_id);
CREATE TABLE IF NOT EXISTS rp_receipts (
  receipt_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
";

/// Names of the v18 tables (for rewind tests of earlier specs).
pub const RESEARCH_PACK_TABLES: [&str; 3] = ["rp_installs", "rp_artifacts", "rp_receipts"];

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
    i64::try_from(r).map_err(|_| invalid("research pack", "revision overflow".to_owned()))
}

/// The install must name a shipped Pack version with its exact digest.
fn validate_install(i: &ResearchPackInstall) -> Result<(), String> {
    let manifest = clinical_research_pack(i.version)
        .filter(|m| m.pack_id == i.pack_id)
        .ok_or_else(|| "install names an unknown pack version".to_owned())?;
    if manifest.digest() != i.manifest_digest {
        return Err("install digest differs from the shipped pack".to_owned());
    }
    if i.revision == 0 {
        return Err("install revision starts at 1".to_owned());
    }
    Ok(())
}

/// The artifact must satisfy its Pack version's schema and workflow.
fn validate_artifact(a: &ResearchArtifact) -> Result<(), String> {
    let manifest = clinical_research_pack(a.pack_version)
        .filter(|m| m.pack_id == a.pack_id)
        .ok_or_else(|| "artifact names an unknown pack version".to_owned())?;
    manifest.check_fields(&a.type_id, &a.fields)?;
    let schema = manifest
        .schema(&a.type_id)
        .ok_or_else(|| "unknown artifact type".to_owned())?;
    let workflow = manifest
        .workflow(&schema.workflow_id)
        .ok_or_else(|| "unknown workflow".to_owned())?;
    if !workflow.states.contains(&a.workflow_state) {
        return Err("artifact is in an unknown workflow state".to_owned());
    }
    for assessment in &a.assessments {
        assessment.validate()?;
    }
    if a.revision == 0 {
        return Err("artifact revision starts at 1".to_owned());
    }
    Ok(())
}

fn decode_install(row: &rusqlite::Row<'_>) -> Result<ResearchPackInstall, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let pack_id: String = row.get(2)?;
    let revision: i64 = row.get(3)?;
    let body: String = row.get(4)?;
    let v: ResearchPackInstall = from_json(&body, "research pack install")?;
    check(&id, v.header.id.as_str(), "research pack install")?;
    check(&project_id, v.project_id.as_str(), "research pack install")?;
    check(&pack_id, &v.pack_id, "research pack install")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("install revision disagrees".to_owned()));
    }
    validate_install(&v).map_err(|e| corrupt(format!("research pack install: {e}")))?;
    Ok(v)
}

fn decode_artifact(row: &rusqlite::Row<'_>) -> Result<ResearchArtifact, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let pack_id: String = row.get(2)?;
    let revision: i64 = row.get(3)?;
    let body: String = row.get(4)?;
    let v: ResearchArtifact = from_json(&body, "research artifact")?;
    check(&id, v.header.id.as_str(), "research artifact")?;
    check(&project_id, v.project_id.as_str(), "research artifact")?;
    check(&pack_id, &v.pack_id, "research artifact")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("artifact revision disagrees".to_owned()));
    }
    validate_artifact(&v).map_err(|e| corrupt(format!("research artifact: {e}")))?;
    Ok(v)
}

fn decode_receipt(row: &rusqlite::Row<'_>) -> Result<PackReceipt, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let v: PackReceipt = from_json(&body, "pack receipt")?;
    check(&id, v.header.id.as_str(), "pack receipt")?;
    check(&project_id, v.project_id.as_str(), "pack receipt")?;
    Ok(v)
}

const INSTALL_COLUMNS: &str = "install_id, project_id, pack_id, revision, body_json";
const ARTIFACT_COLUMNS: &str = "artifact_id, project_id, pack_id, revision, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, project_id, body_json";

/// One Pack change: an install write and/or artifact writes, with the
/// receipt, committed together. `None` expected revision inserts.
#[derive(Debug, Default)]
pub struct PackChange<'a> {
    pub install: Option<(&'a ResearchPackInstall, Option<u64>)>,
    pub artifacts: Vec<(&'a ResearchArtifact, Option<u64>)>,
    pub receipt: Option<&'a PackReceipt>,
}

impl SqliteMetaStore {
    fn rp_rows<T>(
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

    /// Allocates one `prefix-N` Research Pack id from a durable sequence.
    pub fn alloc_pack_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("rp_seq_{prefix}");
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

    fn rp_write_install_on(
        conn: &rusqlite::Connection,
        i: &ResearchPackInstall,
        expected: Option<u64>,
    ) -> Result<(), MetaError> {
        validate_install(i).map_err(|e| invalid("research pack install", e))?;
        match expected {
            None => map_insert(
                conn.execute(
                    "INSERT INTO rp_installs(install_id, project_id, pack_id, revision, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![i.header.id.as_str(), i.project_id.as_str(), i.pack_id, rev(i.revision)?, to_json(i)?],
                ),
                "research pack install",
                i.header.id.as_str(),
            ),
            Some(e) => {
                if i.revision != e + 1 {
                    return Err(invalid("research pack install", "revision must advance by one".to_owned()));
                }
                let changed = conn.execute(
                    "UPDATE rp_installs SET revision = ?1, body_json = ?2 WHERE install_id = ?3 AND revision = ?4",
                    params![rev(i.revision)?, to_json(i)?, i.header.id.as_str(), rev(e)?],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict("install changed concurrently".to_owned()))
                }
            }
        }
    }

    fn rp_write_artifact_on(
        conn: &rusqlite::Connection,
        a: &ResearchArtifact,
        expected: Option<u64>,
    ) -> Result<(), MetaError> {
        validate_artifact(a).map_err(|e| invalid("research artifact", e))?;
        match expected {
            None => map_insert(
                conn.execute(
                    "INSERT INTO rp_artifacts(artifact_id, project_id, pack_id, revision, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![a.header.id.as_str(), a.project_id.as_str(), a.pack_id, rev(a.revision)?, to_json(a)?],
                ),
                "research artifact",
                a.header.id.as_str(),
            ),
            Some(e) => {
                if a.revision != e + 1 {
                    return Err(invalid("research artifact", "revision must advance by one".to_owned()));
                }
                let changed = conn.execute(
                    "UPDATE rp_artifacts SET revision = ?1, body_json = ?2 WHERE artifact_id = ?3 AND revision = ?4",
                    params![rev(a.revision)?, to_json(a)?, a.header.id.as_str(), rev(e)?],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict("artifact changed concurrently".to_owned()))
                }
            }
        }
    }

    fn rp_insert_receipt_on(conn: &rusqlite::Connection, r: &PackReceipt) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO rp_receipts(receipt_id, project_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.project_id.as_str(), to_json(r)?],
            ),
            "pack receipt",
            r.header.id.as_str(),
        )
    }

    fn rp_apply(&self, change: &PackChange<'_>, receipt: &PackReceipt) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        if let Some((i, expected)) = change.install {
            Self::rp_write_install_on(&tx, i, expected)?;
        }
        for (a, expected) in &change.artifacts {
            Self::rp_write_artifact_on(&tx, a, *expected)?;
        }
        Self::rp_insert_receipt_on(&tx, receipt)?;
        tx.commit()?;
        Ok(())
    }

    /// Commits one Pack change and its receipt in a single transaction.
    pub fn commit_pack_change(&self, change: &PackChange<'_>) -> Result<(), MetaError> {
        let receipt = change
            .receipt
            .ok_or_else(|| invalid("pack change", "a change carries its receipt".to_owned()))?;
        self.rp_apply(change, receipt)
    }

    pub fn get_pack_install(
        &self,
        project_id: &OpaqueId,
        pack_id: &str,
    ) -> Result<ResearchPackInstall, MetaError> {
        self.rp_rows(
            &format!(
                "SELECT {INSTALL_COLUMNS} FROM rp_installs WHERE project_id = ?1 AND pack_id = ?2"
            ),
            &[&project_id.as_str(), &pack_id],
            decode_install,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_pack_installs(&self) -> Result<Vec<ResearchPackInstall>, MetaError> {
        self.rp_rows(
            &format!("SELECT {INSTALL_COLUMNS} FROM rp_installs ORDER BY rowid"),
            &[],
            decode_install,
        )
    }

    pub fn get_research_artifact(&self, id: &OpaqueId) -> Result<ResearchArtifact, MetaError> {
        self.rp_rows(
            &format!("SELECT {ARTIFACT_COLUMNS} FROM rp_artifacts WHERE artifact_id = ?1"),
            &[&id.as_str()],
            decode_artifact,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_research_artifacts(
        &self,
        project_id: Option<(&OpaqueId, &str)>,
    ) -> Result<Vec<ResearchArtifact>, MetaError> {
        match project_id {
            Some((p, pack)) => self.rp_rows(
                &format!("SELECT {ARTIFACT_COLUMNS} FROM rp_artifacts WHERE project_id = ?1 AND pack_id = ?2 ORDER BY rowid"),
                &[&p.as_str(), &pack],
                decode_artifact,
            ),
            None => self.rp_rows(
                &format!("SELECT {ARTIFACT_COLUMNS} FROM rp_artifacts ORDER BY rowid"),
                &[],
                decode_artifact,
            ),
        }
    }

    pub fn list_pack_receipts(&self) -> Result<Vec<PackReceipt>, MetaError> {
        self.rp_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM rp_receipts ORDER BY rowid"),
            &[],
            decode_receipt,
        )
    }

    /// Cross-row invariants, re-verified after restore: installs name an
    /// existing Project in scope; every artifact belongs to an installed
    /// Pack of its Project and is at that install's version.
    pub fn verify_research_pack_consistency(&self) -> Result<(), MetaError> {
        let installs: HashMap<(String, String), ResearchPackInstall> = self
            .list_pack_installs()?
            .into_iter()
            .map(|i| ((i.project_id.as_str().to_owned(), i.pack_id.clone()), i))
            .collect();
        for i in installs.values() {
            let project = match self.get_project(&i.project_id) {
                Ok(p) => p,
                Err(MetaError::NotFound) => {
                    return Err(corrupt("pack install names a missing project".to_owned()));
                }
                Err(e) => return Err(e),
            };
            if project.header.realm_id != i.header.realm_id
                || project.header.authority_scope_id != i.header.authority_scope_id
            {
                return Err(corrupt(
                    "pack install is outside its project's scope".to_owned(),
                ));
            }
        }
        for a in self.list_research_artifacts(None)? {
            let i = installs
                .get(&(a.project_id.as_str().to_owned(), a.pack_id.clone()))
                .ok_or_else(|| corrupt("artifact of a pack that is not installed".to_owned()))?;
            if a.pack_version != i.version {
                return Err(corrupt(
                    "artifact is not at its install's version".to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// Backup families.
    pub fn research_pack_backup_families(
        &self,
    ) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        Ok(vec![
            (
                "rp_installs",
                v(serde_json::to_value(self.list_pack_installs()?))?,
            ),
            (
                "rp_artifacts",
                v(serde_json::to_value(self.list_research_artifacts(None)?))?,
            ),
            (
                "rp_receipts",
                v(serde_json::to_value(self.list_pack_receipts()?))?,
            ),
        ])
    }

    pub fn restore_pack_install_row(&self, i: &ResearchPackInstall) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::rp_write_install_on(self.conn(), i, None))
    }

    pub fn restore_research_artifact_row(&self, a: &ResearchArtifact) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::rp_write_artifact_on(self.conn(), a, None))
    }

    pub fn restore_pack_receipt_row(&self, r: &PackReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::rp_insert_receipt_on(self.conn(), r))
    }
}

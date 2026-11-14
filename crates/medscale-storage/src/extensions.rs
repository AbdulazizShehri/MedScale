//! Community Extensions durable rows (Spec 087, storage schema v16).
//!
//! Same pattern as Specs 079-086: each row keeps the validated contract
//! value as JSON (`body_json`) plus the columns queries need; reads
//! re-check columns against the body and fail closed on disagreement.
//!
//! Releases are immutable except for their revocation flag. An install is
//! one row per (Project, extension); every change is a compare-and-set on
//! its revision and is committed with its lifecycle receipt (and any new
//! grants) in one transaction. Revoking a publisher or a release
//! quarantines every affected install in the same transaction.

use std::collections::HashMap;

use medscale_contracts::extensions::{
    ExtensionGrant, ExtensionInstallRecord, ExtensionLifecycleReceipt, ExtensionPublisher,
    ExtensionRelease, ExtensionRuntimeReceipt, InstallState, PublisherState,
};
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v16 DDL, executed inside `begin/finish_migration(16)`.
pub(crate) const V16_DDL: &str = r"
CREATE TABLE IF NOT EXISTS ext_publishers (
  publisher_row_id TEXT PRIMARY KEY,
  publisher_id TEXT NOT NULL UNIQUE,
  state TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS ext_releases (
  digest_hex TEXT PRIMARY KEY,
  release_id TEXT NOT NULL UNIQUE,
  extension_id TEXT NOT NULL,
  publisher_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS ext_installs (
  install_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  extension_id TEXT NOT NULL,
  publisher_id TEXT NOT NULL,
  state TEXT NOT NULL,
  revision INTEGER NOT NULL,
  body_json TEXT NOT NULL,
  UNIQUE(project_id, extension_id)
);
CREATE TABLE IF NOT EXISTS ext_grants (
  grant_id TEXT PRIMARY KEY,
  install_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ext_grants_install ON ext_grants(install_id);
CREATE TABLE IF NOT EXISTS ext_lifecycle_receipts (
  receipt_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS ext_runtime_receipts (
  receipt_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  body_json TEXT NOT NULL
);
";

/// Names of the v16 tables (for rewind tests of earlier specs).
pub const EXTENSION_TABLES: [&str; 6] = [
    "ext_publishers",
    "ext_releases",
    "ext_installs",
    "ext_grants",
    "ext_lifecycle_receipts",
    "ext_runtime_receipts",
];

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

fn same_scope(a: &ObjectHeader, b: &ObjectHeader) -> bool {
    a.realm_id == b.realm_id && a.authority_scope_id == b.authority_scope_id
}

fn validate_publisher(p: &ExtensionPublisher) -> Result<(), String> {
    if p.key_hex.len() != 64 || !p.key_hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("publisher key is not 64 hex digits".to_owned());
    }
    if p.publisher_id.is_empty() {
        return Err("publisher id is empty".to_owned());
    }
    Ok(())
}

fn validate_install(i: &ExtensionInstallRecord) -> Result<(), String> {
    if i.revision == 0 {
        return Err("install revision starts at 1".to_owned());
    }
    if i.previous_release.as_ref() == Some(&i.active_release) {
        return Err("previous release equals the active one".to_owned());
    }
    Ok(())
}

fn decode_publisher(row: &rusqlite::Row<'_>) -> Result<ExtensionPublisher, MetaError> {
    let id: String = row.get(0)?;
    let publisher_id: String = row.get(1)?;
    let state: String = row.get(2)?;
    let body: String = row.get(3)?;
    let v: ExtensionPublisher = from_json(&body, "extension publisher")?;
    check(&id, v.header.id.as_str(), "extension publisher")?;
    check(&publisher_id, &v.publisher_id, "extension publisher")?;
    check(&state, v.state.as_str(), "extension publisher")?;
    validate_publisher(&v).map_err(|e| corrupt(format!("extension publisher: {e}")))?;
    Ok(v)
}

fn decode_release(row: &rusqlite::Row<'_>) -> Result<ExtensionRelease, MetaError> {
    let digest: String = row.get(0)?;
    let id: String = row.get(1)?;
    let extension_id: String = row.get(2)?;
    let publisher_id: String = row.get(3)?;
    let body: String = row.get(4)?;
    let v: ExtensionRelease = from_json(&body, "extension release")?;
    check(&digest, &v.digest.to_hex(), "extension release")?;
    check(&id, v.header.id.as_str(), "extension release")?;
    check(&extension_id, &v.manifest.extension_id, "extension release")?;
    check(&publisher_id, &v.manifest.publisher_id, "extension release")?;
    v.validate()
        .map_err(|e| corrupt(format!("extension release: {e}")))?;
    Ok(v)
}

fn decode_install(row: &rusqlite::Row<'_>) -> Result<ExtensionInstallRecord, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let extension_id: String = row.get(2)?;
    let publisher_id: String = row.get(3)?;
    let state: String = row.get(4)?;
    let revision: i64 = row.get(5)?;
    let body: String = row.get(6)?;
    let v: ExtensionInstallRecord = from_json(&body, "extension install")?;
    check(&id, v.header.id.as_str(), "extension install")?;
    check(&project_id, v.project_id.as_str(), "extension install")?;
    check(&extension_id, &v.extension_id, "extension install")?;
    check(&publisher_id, &v.publisher_id, "extension install")?;
    check(&state, v.state.as_str(), "extension install")?;
    if u64::try_from(revision).ok() != Some(v.revision) {
        return Err(corrupt("extension install revision disagrees".to_owned()));
    }
    validate_install(&v).map_err(|e| corrupt(format!("extension install: {e}")))?;
    Ok(v)
}

fn decode_grant(row: &rusqlite::Row<'_>) -> Result<ExtensionGrant, MetaError> {
    let id: String = row.get(0)?;
    let install_id: String = row.get(1)?;
    let project_id: String = row.get(2)?;
    let body: String = row.get(3)?;
    let v: ExtensionGrant = from_json(&body, "extension grant")?;
    check(&id, v.header.id.as_str(), "extension grant")?;
    check(&install_id, v.install_id.as_str(), "extension grant")?;
    check(&project_id, v.project_id.as_str(), "extension grant")?;
    Ok(v)
}

fn decode_lifecycle(row: &rusqlite::Row<'_>) -> Result<ExtensionLifecycleReceipt, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let v: ExtensionLifecycleReceipt = from_json(&body, "extension lifecycle receipt")?;
    check(&id, v.header.id.as_str(), "extension lifecycle receipt")?;
    check(
        &project_id,
        v.project_id.as_str(),
        "extension lifecycle receipt",
    )?;
    if v.refusal.is_some() == v.resulting_state.is_some() {
        return Err(corrupt(
            "a lifecycle receipt is refused or has a resulting state".to_owned(),
        ));
    }
    Ok(v)
}

fn decode_runtime(row: &rusqlite::Row<'_>) -> Result<ExtensionRuntimeReceipt, MetaError> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let body: String = row.get(2)?;
    let v: ExtensionRuntimeReceipt = from_json(&body, "extension runtime receipt")?;
    check(&id, v.header.id.as_str(), "extension runtime receipt")?;
    check(
        &project_id,
        v.project_id.as_str(),
        "extension runtime receipt",
    )?;
    v.validate()
        .map_err(|e| corrupt(format!("extension runtime receipt: {e}")))?;
    Ok(v)
}

const PUBLISHER_COLUMNS: &str = "publisher_row_id, publisher_id, state, body_json";
const RELEASE_COLUMNS: &str = "digest_hex, release_id, extension_id, publisher_id, body_json";
const INSTALL_COLUMNS: &str =
    "install_id, project_id, extension_id, publisher_id, state, revision, body_json";
const GRANT_COLUMNS: &str = "grant_id, install_id, project_id, body_json";
const RECEIPT_COLUMNS: &str = "receipt_id, project_id, body_json";

/// One install change with its receipt (and new grants), committed
/// together.
pub struct InstallChange<'a> {
    pub install: &'a ExtensionInstallRecord,
    /// `None` inserts a new install; `Some(r)` requires revision `r`.
    pub expected_revision: Option<u64>,
    pub new_grants: &'a [ExtensionGrant],
    /// Grant ids to mark revoked.
    pub revoke_grants: &'a [OpaqueId],
    pub receipt: &'a ExtensionLifecycleReceipt,
}

impl SqliteMetaStore {
    fn ext_rows<T>(
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

    /// Allocates one `prefix-N` extension id from a durable sequence.
    pub fn alloc_ext_id(&self, prefix: &str) -> Result<OpaqueId, MetaError> {
        let seq_key = format!("ext_seq_{prefix}");
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

    // ----- publishers -----

    fn ext_insert_publisher_on(
        conn: &rusqlite::Connection,
        p: &ExtensionPublisher,
    ) -> Result<(), MetaError> {
        validate_publisher(p).map_err(|e| invalid("extension publisher", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO ext_publishers(publisher_row_id, publisher_id, state, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![p.header.id.as_str(), p.publisher_id, p.state.as_str(), to_json(p)?],
            ),
            "extension publisher",
            &p.publisher_id,
        )
    }

    pub fn insert_ext_publisher(&self, p: &ExtensionPublisher) -> Result<(), MetaError> {
        if p.state != PublisherState::Trusted {
            return Err(invalid(
                "extension publisher",
                "a new publisher is trusted".to_owned(),
            ));
        }
        Self::ext_insert_publisher_on(self.conn(), p)
    }

    pub fn get_ext_publisher(&self, publisher_id: &str) -> Result<ExtensionPublisher, MetaError> {
        self.ext_rows(
            &format!("SELECT {PUBLISHER_COLUMNS} FROM ext_publishers WHERE publisher_id = ?1"),
            &[&publisher_id],
            decode_publisher,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_ext_publishers(&self) -> Result<Vec<ExtensionPublisher>, MetaError> {
        self.ext_rows(
            &format!("SELECT {PUBLISHER_COLUMNS} FROM ext_publishers ORDER BY rowid"),
            &[],
            decode_publisher,
        )
    }

    // ----- releases -----

    fn ext_insert_release_on(
        conn: &rusqlite::Connection,
        r: &ExtensionRelease,
    ) -> Result<(), MetaError> {
        r.validate().map_err(|e| invalid("extension release", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO ext_releases(digest_hex, release_id, extension_id, publisher_id, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    r.digest.to_hex(),
                    r.header.id.as_str(),
                    r.manifest.extension_id,
                    r.manifest.publisher_id,
                    to_json(r)?
                ],
            ),
            "extension release",
            &r.digest.to_hex(),
        )
    }

    pub fn insert_ext_release(&self, r: &ExtensionRelease) -> Result<(), MetaError> {
        Self::ext_insert_release_on(self.conn(), r)
    }

    pub fn get_ext_release(&self, digest: &DigestSha256) -> Result<ExtensionRelease, MetaError> {
        self.ext_rows(
            &format!("SELECT {RELEASE_COLUMNS} FROM ext_releases WHERE digest_hex = ?1"),
            &[&digest.to_hex()],
            decode_release,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_ext_releases(&self) -> Result<Vec<ExtensionRelease>, MetaError> {
        self.ext_rows(
            &format!("SELECT {RELEASE_COLUMNS} FROM ext_releases ORDER BY rowid"),
            &[],
            decode_release,
        )
    }

    // ----- installs, grants, receipts -----

    fn ext_write_install_on(
        conn: &rusqlite::Connection,
        i: &ExtensionInstallRecord,
        expected_revision: Option<u64>,
    ) -> Result<(), MetaError> {
        validate_install(i).map_err(|e| invalid("extension install", e))?;
        let revision = i64::try_from(i.revision)
            .map_err(|_| invalid("extension install", "revision overflow".to_owned()))?;
        match expected_revision {
            None => {
                if i.revision != 1 {
                    return Err(invalid(
                        "extension install",
                        "a new install has revision 1".to_owned(),
                    ));
                }
                map_insert(
                    conn.execute(
                        "INSERT INTO ext_installs(install_id, project_id, extension_id, publisher_id, state, revision, body_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            i.header.id.as_str(),
                            i.project_id.as_str(),
                            i.extension_id,
                            i.publisher_id,
                            i.state.as_str(),
                            revision,
                            to_json(i)?
                        ],
                    ),
                    "extension install",
                    i.header.id.as_str(),
                )
            }
            Some(expected) => {
                if i.revision != expected + 1 {
                    return Err(invalid(
                        "extension install",
                        "revision must advance by one".to_owned(),
                    ));
                }
                let expected = i64::try_from(expected)
                    .map_err(|_| invalid("extension install", "revision overflow".to_owned()))?;
                let changed = conn.execute(
                    "UPDATE ext_installs SET state = ?1, revision = ?2, body_json = ?3 WHERE install_id = ?4 AND revision = ?5",
                    params![i.state.as_str(), revision, to_json(i)?, i.header.id.as_str(), expected],
                )?;
                if changed == 1 {
                    Ok(())
                } else {
                    Err(MetaError::Conflict(format!(
                        "extension install {} changed concurrently",
                        i.header.id.as_str()
                    )))
                }
            }
        }
    }

    fn ext_insert_grant_on(
        conn: &rusqlite::Connection,
        g: &ExtensionGrant,
    ) -> Result<(), MetaError> {
        map_insert(
            conn.execute(
                "INSERT INTO ext_grants(grant_id, install_id, project_id, body_json) VALUES (?1, ?2, ?3, ?4)",
                params![g.header.id.as_str(), g.install_id.as_str(), g.project_id.as_str(), to_json(g)?],
            ),
            "extension grant",
            g.header.id.as_str(),
        )
    }

    fn ext_insert_lifecycle_on(
        conn: &rusqlite::Connection,
        r: &ExtensionLifecycleReceipt,
    ) -> Result<(), MetaError> {
        if r.refusal.is_some() == r.resulting_state.is_some() {
            return Err(invalid(
                "extension lifecycle receipt",
                "refused or with a resulting state".to_owned(),
            ));
        }
        map_insert(
            conn.execute(
                "INSERT INTO ext_lifecycle_receipts(receipt_id, project_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.project_id.as_str(), to_json(r)?],
            ),
            "extension lifecycle receipt",
            r.header.id.as_str(),
        )
    }

    fn ext_insert_runtime_on(
        conn: &rusqlite::Connection,
        r: &ExtensionRuntimeReceipt,
    ) -> Result<(), MetaError> {
        r.validate()
            .map_err(|e| invalid("extension runtime receipt", e))?;
        map_insert(
            conn.execute(
                "INSERT INTO ext_runtime_receipts(receipt_id, project_id, body_json) VALUES (?1, ?2, ?3)",
                params![r.header.id.as_str(), r.project_id.as_str(), to_json(r)?],
            ),
            "extension runtime receipt",
            r.header.id.as_str(),
        )
    }

    fn ext_revoke_grant_on(conn: &rusqlite::Connection, id: &OpaqueId) -> Result<(), MetaError> {
        let body: String = conn
            .query_row(
                "SELECT body_json FROM ext_grants WHERE grant_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(MetaError::NotFound)?;
        let mut g: ExtensionGrant = from_json(&body, "extension grant")?;
        g.revoked = true;
        conn.execute(
            "UPDATE ext_grants SET body_json = ?1 WHERE grant_id = ?2",
            params![to_json(&g)?, id.as_str()],
        )?;
        Ok(())
    }

    /// Records a refused lifecycle attempt.
    pub fn insert_ext_refusal(&self, r: &ExtensionLifecycleReceipt) -> Result<(), MetaError> {
        if r.refusal.is_none() {
            return Err(invalid(
                "extension lifecycle receipt",
                "not a refusal".to_owned(),
            ));
        }
        Self::ext_insert_lifecycle_on(self.conn(), r)
    }

    /// Applies one install change with its receipt and grants atomically.
    pub fn commit_ext_install_change(&self, change: &InstallChange<'_>) -> Result<(), MetaError> {
        let i = change.install;
        if change.receipt.resulting_state != Some(i.state)
            || change.receipt.install_id.as_ref() != Some(&i.header.id)
            || change.receipt.project_id != i.project_id
        {
            return Err(invalid(
                "extension lifecycle receipt",
                "receipt does not describe its install".to_owned(),
            ));
        }
        for g in change.new_grants {
            if g.install_id != i.header.id || g.project_id != i.project_id || g.revoked {
                return Err(invalid(
                    "extension grant",
                    "grant does not belong to its install".to_owned(),
                ));
            }
        }
        let release = self.get_ext_release(&i.active_release)?;
        if release.manifest.extension_id != i.extension_id
            || release.manifest.publisher_id != i.publisher_id
        {
            return Err(invalid(
                "extension install",
                "active release is another extension".to_owned(),
            ));
        }
        let tx = self.conn().unchecked_transaction()?;
        Self::ext_write_install_on(&tx, i, change.expected_revision)?;
        for g in change.new_grants {
            Self::ext_insert_grant_on(&tx, g)?;
        }
        for id in change.revoke_grants {
            Self::ext_revoke_grant_on(&tx, id)?;
        }
        Self::ext_insert_lifecycle_on(&tx, change.receipt)?;
        tx.commit()?;
        Ok(())
    }

    /// Revokes a publisher (terminal) or a release and quarantines every
    /// install it affects, with one receipt per quarantined install, in
    /// one transaction. `receipts` pairs each install id with its receipt.
    pub fn commit_ext_revocation(
        &self,
        publisher_id: Option<&str>,
        release: Option<&DigestSha256>,
        quarantined: &[(ExtensionInstallRecord, u64, ExtensionLifecycleReceipt)],
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        if let Some(pid) = publisher_id {
            let body: String = tx
                .query_row(
                    "SELECT body_json FROM ext_publishers WHERE publisher_id = ?1",
                    params![pid],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(MetaError::NotFound)?;
            let mut p: ExtensionPublisher = from_json(&body, "extension publisher")?;
            p.state = PublisherState::Revoked;
            tx.execute(
                "UPDATE ext_publishers SET state = ?1, body_json = ?2 WHERE publisher_id = ?3",
                params![p.state.as_str(), to_json(&p)?, pid],
            )?;
        }
        if let Some(digest) = release {
            let body: String = tx
                .query_row(
                    "SELECT body_json FROM ext_releases WHERE digest_hex = ?1",
                    params![digest.to_hex()],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(MetaError::NotFound)?;
            let mut r: ExtensionRelease = from_json(&body, "extension release")?;
            r.revoked = true;
            tx.execute(
                "UPDATE ext_releases SET body_json = ?1 WHERE digest_hex = ?2",
                params![to_json(&r)?, digest.to_hex()],
            )?;
        }
        for (install, expected, receipt) in quarantined {
            if install.state != InstallState::Quarantined
                || receipt.resulting_state != Some(InstallState::Quarantined)
            {
                return Err(invalid(
                    "extension install",
                    "revocation quarantines".to_owned(),
                ));
            }
            Self::ext_write_install_on(&tx, install, Some(*expected))?;
            Self::ext_insert_lifecycle_on(&tx, receipt)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_ext_runtime_receipt(&self, r: &ExtensionRuntimeReceipt) -> Result<(), MetaError> {
        Self::ext_insert_runtime_on(self.conn(), r)
    }

    pub fn get_ext_install(
        &self,
        project_id: &OpaqueId,
        extension_id: &str,
    ) -> Result<ExtensionInstallRecord, MetaError> {
        self.ext_rows(
            &format!(
                "SELECT {INSTALL_COLUMNS} FROM ext_installs WHERE project_id = ?1 AND extension_id = ?2"
            ),
            &[&project_id.as_str(), &extension_id],
            decode_install,
        )?
        .into_iter()
        .next()
        .ok_or(MetaError::NotFound)
    }

    pub fn list_ext_installs(&self) -> Result<Vec<ExtensionInstallRecord>, MetaError> {
        self.ext_rows(
            &format!("SELECT {INSTALL_COLUMNS} FROM ext_installs ORDER BY rowid"),
            &[],
            decode_install,
        )
    }

    pub fn list_ext_grants(
        &self,
        install_id: Option<&OpaqueId>,
    ) -> Result<Vec<ExtensionGrant>, MetaError> {
        match install_id {
            Some(id) => self.ext_rows(
                &format!(
                    "SELECT {GRANT_COLUMNS} FROM ext_grants WHERE install_id = ?1 ORDER BY rowid"
                ),
                &[&id.as_str()],
                decode_grant,
            ),
            None => self.ext_rows(
                &format!("SELECT {GRANT_COLUMNS} FROM ext_grants ORDER BY rowid"),
                &[],
                decode_grant,
            ),
        }
    }

    pub fn list_ext_lifecycle_receipts(&self) -> Result<Vec<ExtensionLifecycleReceipt>, MetaError> {
        self.ext_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM ext_lifecycle_receipts ORDER BY rowid"),
            &[],
            decode_lifecycle,
        )
    }

    pub fn list_ext_runtime_receipts(&self) -> Result<Vec<ExtensionRuntimeReceipt>, MetaError> {
        self.ext_rows(
            &format!("SELECT {RECEIPT_COLUMNS} FROM ext_runtime_receipts ORDER BY rowid"),
            &[],
            decode_runtime,
        )
    }

    /// Cross-row invariants, re-verified after restore:
    /// - every install names an existing Project in its scope and an
    ///   existing release of the same extension and publisher (active and
    ///   previous);
    /// - an install whose publisher is revoked, or whose active release is
    ///   revoked, is quarantined or uninstalled;
    /// - every grant belongs to an existing install of the same Project;
    /// - every release's publisher key matches a known publisher.
    pub fn verify_extension_consistency(&self) -> Result<(), MetaError> {
        let publishers: HashMap<String, ExtensionPublisher> = self
            .list_ext_publishers()?
            .into_iter()
            .map(|p| (p.publisher_id.clone(), p))
            .collect();
        let releases: HashMap<String, ExtensionRelease> = self
            .list_ext_releases()?
            .into_iter()
            .map(|r| (r.digest.to_hex(), r))
            .collect();
        for r in releases.values() {
            let p = publishers
                .get(&r.manifest.publisher_id)
                .ok_or_else(|| corrupt("release of an unknown publisher".to_owned()))?;
            if p.key_hex != r.manifest.publisher_key_hex {
                return Err(corrupt("release key differs from its publisher".to_owned()));
            }
        }
        let installs = self.list_ext_installs()?;
        let mut by_id = HashMap::new();
        for i in &installs {
            let project = match self.get_project(&i.project_id) {
                Ok(p) => p,
                Err(MetaError::NotFound) => {
                    return Err(corrupt("install names a missing project".to_owned()));
                }
                Err(e) => return Err(e),
            };
            if !same_scope(&project.header, &i.header) {
                return Err(corrupt("install is outside its project's scope".to_owned()));
            }
            for digest in std::iter::once(&i.active_release).chain(i.previous_release.iter()) {
                let r = releases
                    .get(&digest.to_hex())
                    .ok_or_else(|| corrupt("install names a missing release".to_owned()))?;
                if r.manifest.extension_id != i.extension_id
                    || r.manifest.publisher_id != i.publisher_id
                {
                    return Err(corrupt(
                        "install names another extension's release".to_owned(),
                    ));
                }
            }
            let revoked = publishers
                .get(&i.publisher_id)
                .is_some_and(|p| p.state == PublisherState::Revoked)
                || releases
                    .get(&i.active_release.to_hex())
                    .is_some_and(|r| r.revoked);
            if revoked
                && !matches!(
                    i.state,
                    InstallState::Quarantined | InstallState::Uninstalled
                )
            {
                return Err(corrupt("a revoked extension is not quarantined".to_owned()));
            }
            by_id.insert(i.header.id.as_str().to_owned(), i);
        }
        for g in self.list_ext_grants(None)? {
            let i = by_id
                .get(g.install_id.as_str())
                .ok_or_else(|| corrupt("grant without its install".to_owned()))?;
            if i.project_id != g.project_id {
                return Err(corrupt("grant crosses projects".to_owned()));
            }
        }
        Ok(())
    }

    /// Backup families.
    pub fn extension_backup_families(
        &self,
    ) -> Result<Vec<(&'static str, serde_json::Value)>, MetaError> {
        let v =
            |r: Result<serde_json::Value, serde_json::Error>| r.map_err(|e| corrupt(e.to_string()));
        Ok(vec![
            (
                "ext_publishers",
                v(serde_json::to_value(self.list_ext_publishers()?))?,
            ),
            (
                "ext_releases",
                v(serde_json::to_value(self.list_ext_releases()?))?,
            ),
            (
                "ext_installs",
                v(serde_json::to_value(self.list_ext_installs()?))?,
            ),
            (
                "ext_grants",
                v(serde_json::to_value(self.list_ext_grants(None)?))?,
            ),
            (
                "ext_lifecycle_receipts",
                v(serde_json::to_value(self.list_ext_lifecycle_receipts()?))?,
            ),
            (
                "ext_runtime_receipts",
                v(serde_json::to_value(self.list_ext_runtime_receipts()?))?,
            ),
        ])
    }

    pub fn restore_ext_publisher_row(&self, p: &ExtensionPublisher) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ext_insert_publisher_on(self.conn(), p))
    }

    pub fn restore_ext_release_row(&self, r: &ExtensionRelease) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ext_insert_release_on(self.conn(), r))
    }

    pub fn restore_ext_install_row(&self, i: &ExtensionInstallRecord) -> Result<(), MetaError> {
        // Restored installs keep their revision; insert directly.
        validate_install(i).map_err(|e| corrupt(format!("extension install: {e}")))?;
        let revision = i64::try_from(i.revision)
            .map_err(|_| corrupt("extension install revision overflow".to_owned()))?;
        restore_conflict_is_corrupt(map_insert(
            self.conn().execute(
                "INSERT INTO ext_installs(install_id, project_id, extension_id, publisher_id, state, revision, body_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    i.header.id.as_str(),
                    i.project_id.as_str(),
                    i.extension_id,
                    i.publisher_id,
                    i.state.as_str(),
                    revision,
                    to_json(i)?
                ],
            ),
            "extension install",
            i.header.id.as_str(),
        ))
    }

    pub fn restore_ext_grant_row(&self, g: &ExtensionGrant) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ext_insert_grant_on(self.conn(), g))
    }

    pub fn restore_ext_lifecycle_row(
        &self,
        r: &ExtensionLifecycleReceipt,
    ) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ext_insert_lifecycle_on(self.conn(), r))
    }

    pub fn restore_ext_runtime_row(&self, r: &ExtensionRuntimeReceipt) -> Result<(), MetaError> {
        restore_conflict_is_corrupt(Self::ext_insert_runtime_on(self.conn(), r))
    }
}

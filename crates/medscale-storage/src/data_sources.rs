//! Data Source Fabric durable rows (Spec 075, storage schema v4).
//!
//! Dedicated tables inside the existing encrypted metadata DB: no second
//! database, no canonical payload copies, no bulk bytes in metadata rows.
//! Snapshot bulk bytes live in the existing blob/sealed-blob stores under
//! content-digest identity; metadata rows carry digests and lineage only.
//! Every 075 contract field is a scalar column except nested enums/structs,
//! which travel as validated JSON. Reads re-validate and report `Corrupt`
//! or `UnsupportedSchema`.
//!
//! Concurrency: compare-and-swap on `revision` inside `unchecked_transaction`
//! for mutable rows (sources, views). Immutable rows (snapshots, parts,
//! receipts, transformations, releases) are insert-once; duplicates fail as
//! `Conflict`, refresh receipts upsert idempotently.
//!
//! Pagination: deterministic `*_id TEXT` order with `after` cursors carrying
//! the last seen id. Cursor length is bounded by the contracts vocabulary.

use medscale_contracts::data_sources::{
    DATA_SOURCE_SCHEMA_VERSION, DataSourceManifest, DataSourceSummary, DataViewKind,
    DatasetReleaseSummary, ReleaseManifest, SavedDataView, SavedViewStatus, SavedViewSummary,
    SnapshotStatus, SnapshotSummary, SourceStatus,
};
use medscale_contracts::objects::{AuthorityScopeId, DigestSha256, OpaqueId, RealmId};
use rusqlite::{OptionalExtension, params};

use crate::sqlite_meta::{MetaError, SqliteMetaStore};

/// Additive schema v4 DDL, executed inside `begin/finish_migration(4)`.
pub(crate) const V4_DDL: &str = r"
CREATE TABLE IF NOT EXISTS data_sources (
  source_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  display_name TEXT NOT NULL,
  format_or_engine TEXT NOT NULL,
  locator_json TEXT NOT NULL,
  credential_ref TEXT,
  capabilities_json TEXT NOT NULL,
  health TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_data_sources_project
  ON data_sources(project_id);
CREATE INDEX IF NOT EXISTS idx_data_sources_scope_status
  ON data_sources(authority_scope_id, status);
CREATE TABLE IF NOT EXISTS data_snapshots (
  snapshot_id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  parent_snapshot_id TEXT,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  source_revision_json TEXT NOT NULL,
  schema_json TEXT NOT NULL,
  schema_fingerprint_hex TEXT NOT NULL,
  row_count INTEGER NOT NULL,
  content_digest_hex TEXT NOT NULL,
  status TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_snapshots_source
  ON data_snapshots(source_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_snapshots_source_digest
  ON data_snapshots(source_id, content_digest_hex);
CREATE TABLE IF NOT EXISTS data_snapshot_parts (
  snapshot_id TEXT NOT NULL,
  part_index INTEGER NOT NULL,
  row_start INTEGER NOT NULL,
  row_end INTEGER NOT NULL,
  part_digest_hex TEXT NOT NULL,
  PRIMARY KEY (snapshot_id, part_index)
);
CREATE TABLE IF NOT EXISTS data_receipts (
  receipt_kind TEXT NOT NULL,
  snapshot_id TEXT NOT NULL,
  receipt_json TEXT NOT NULL,
  PRIMARY KEY (receipt_kind, snapshot_id)
);
CREATE TABLE IF NOT EXISTS data_saved_views (
  view_id TEXT PRIMARY KEY,
  snapshot_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  view_kind TEXT NOT NULL,
  state_json TEXT NOT NULL,
  status TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_saved_views_snapshot
  ON data_saved_views(snapshot_id);
CREATE TABLE IF NOT EXISTS data_transformations (
  output_snapshot_id TEXT PRIMARY KEY,
  first_input_snapshot_id TEXT NOT NULL,
  input_ids_json TEXT NOT NULL,
  ops_json TEXT NOT NULL,
  parameters_digest_hex TEXT NOT NULL,
  receipt_json TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_transformations_first_input
  ON data_transformations(first_input_snapshot_id);
CREATE TABLE IF NOT EXISTS dataset_releases (
  release_id TEXT PRIMARY KEY,
  snapshot_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  authority_scope_id TEXT NOT NULL,
  card_json TEXT NOT NULL,
  snapshot_digest_hex TEXT NOT NULL,
  schema_version INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_releases_project
  ON dataset_releases(project_id);
CREATE INDEX IF NOT EXISTS idx_releases_snapshot
  ON dataset_releases(snapshot_id);
";

// ---------- small helpers ----------

/// Bounded cursor guard (mirrors the contracts vocabulary).
const CURSOR_MAX_BYTES: usize = 256;

fn check_cursor(after: Option<&str>) -> Result<(), MetaError> {
    if let Some(value) = after
        && value.len() > CURSOR_MAX_BYTES
    {
        return Err(MetaError::UnsupportedSchema(
            "cursor exceeds bound".to_owned(),
        ));
    }
    Ok(())
}

fn is_conflict(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(f, _)
        if f.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

fn to_meta(err: rusqlite::Error) -> MetaError {
    MetaError::Sqlite(err)
}

fn digest_hex(digest: &DigestSha256) -> String {
    digest.to_hex()
}

fn parse_digest_hex(value: &str, what: &str) -> Result<DigestSha256, MetaError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(MetaError::CorruptObjectBody(format!(
            "invalid {what} digest"
        )));
    }
    let mut out = [0_u8; 32];
    for (i, chunk) in value.as_bytes().chunks(2).enumerate() {
        let text = std::str::from_utf8(chunk)
            .map_err(|_| MetaError::CorruptObjectBody(format!("invalid {what} digest")))?;
        out[i] = u8::from_str_radix(text, 16)
            .map_err(|_| MetaError::CorruptObjectBody(format!("invalid {what} digest")))?;
    }
    Ok(DigestSha256::from_bytes(out))
}

// ---------- value encoding ----------

fn kind_str(kind: medscale_contracts::data_sources::DataSourceKind) -> &'static str {
    kind.as_str()
}

fn parse_kind(value: &str) -> Result<medscale_contracts::data_sources::DataSourceKind, MetaError> {
    medscale_contracts::data_sources::DataSourceKind::parse(value)
        .map_err(MetaError::UnsupportedSchema)
}

fn health_str(health: medscale_contracts::data_sources::SourceHealth) -> &'static str {
    health.as_str()
}

fn parse_health(value: &str) -> Result<medscale_contracts::data_sources::SourceHealth, MetaError> {
    // SourceHealth has no parse helper; decode the closed vocabulary here so
    // unknown durable values fail closed instead of coercing.
    let parsed = match value {
        "healthy" => medscale_contracts::data_sources::SourceHealth::Healthy,
        "stale" => medscale_contracts::data_sources::SourceHealth::Stale,
        "unavailable" => medscale_contracts::data_sources::SourceHealth::Unavailable,
        "denied" => medscale_contracts::data_sources::SourceHealth::Denied,
        "schema_changed" => medscale_contracts::data_sources::SourceHealth::SchemaChanged,
        "credential_revoked" => medscale_contracts::data_sources::SourceHealth::CredentialRevoked,
        other => {
            return Err(MetaError::UnsupportedSchema(format!(
                "unknown source health {other}"
            )));
        }
    };
    Ok(parsed)
}

fn source_status_str(status: SourceStatus) -> &'static str {
    status.as_str()
}

fn parse_source_status(value: &str) -> Result<SourceStatus, MetaError> {
    match value {
        "active" => Ok(SourceStatus::Active),
        "archived" => Ok(SourceStatus::Archived),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown source status {other}"
        ))),
    }
}

fn view_kind_str(kind: DataViewKind) -> &'static str {
    kind.as_str()
}

fn parse_view_kind(value: &str) -> Result<DataViewKind, MetaError> {
    DataViewKind::parse(value).map_err(MetaError::UnsupportedSchema)
}

fn saved_view_status_str(status: SavedViewStatus) -> &'static str {
    status.as_str()
}

fn parse_saved_view_status(value: &str) -> Result<SavedViewStatus, MetaError> {
    match value {
        "active" => Ok(SavedViewStatus::Active),
        "archived" => Ok(SavedViewStatus::Archived),
        other => Err(MetaError::UnsupportedSchema(format!(
            "unknown view status {other}"
        ))),
    }
}

fn snapshot_status_str(status: &SnapshotStatus) -> String {
    match status {
        SnapshotStatus::Complete => "complete".to_owned(),
        SnapshotStatus::Partial { reason } => format!("partial:{reason}"),
    }
}

fn parse_snapshot_status(value: &str) -> Result<SnapshotStatus, MetaError> {
    if value == "complete" {
        return Ok(SnapshotStatus::Complete);
    }
    if let Some(reason) = value.strip_prefix("partial:") {
        let status = SnapshotStatus::Partial {
            reason: reason.to_owned(),
        };
        status.validate().map_err(MetaError::CorruptObjectBody)?;
        return Ok(status);
    }
    Err(MetaError::UnsupportedSchema(format!(
        "unknown snapshot status {value}"
    )))
}

// ---------- id allocation ----------

impl SqliteMetaStore {
    /// Allocates one `prefix-N` id from a durable sqlite-backed sequence.
    /// Transactional: crash-safe, no reuse after reopen.
    pub fn alloc_data_fabric_id(&self, seq_key: &str, prefix: &str) -> Result<OpaqueId, MetaError> {
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
                raw.parse::<u64>().map_err(|_| {
                    MetaError::CorruptObjectBody(format!("id sequence {seq_key} is not numeric"))
                })? + 1
            }
        };
        tx.execute(
            "INSERT OR REPLACE INTO store_state(key, value) VALUES (?1, ?2)",
            params![seq_key, next.to_string()],
        )?;
        tx.commit()?;
        Ok(OpaqueId::new(format!("{prefix}-{next}")))
    }
}

// ---------- data sources ----------

fn map_source_row(row: &rusqlite::Row<'_>) -> Result<DataSourceManifest, MetaError> {
    map_source_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
        row.get(6).map_err(MetaError::Sqlite)?,
        row.get(7).map_err(MetaError::Sqlite)?,
        row.get(8).map_err(MetaError::Sqlite)?,
        row.get(9).map_err(MetaError::Sqlite)?,
        row.get(10).map_err(MetaError::Sqlite)?,
        row.get(11).map_err(MetaError::Sqlite)?,
        row.get(12).map_err(MetaError::Sqlite)?,
        row.get(13).map_err(MetaError::Sqlite)?,
    )
}

fn map_source_row_result(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Result<DataSourceManifest, MetaError>> {
    Ok(map_source_row_inner(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, String>(7)?,
        row.get::<_, Option<String>>(8)?,
        row.get::<_, String>(9)?,
        row.get::<_, String>(10)?,
        row.get::<_, String>(11)?,
        row.get::<_, i64>(12)?,
        row.get::<_, i64>(13)?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn map_source_row_inner(
    id: String,
    project_id: String,
    realm: String,
    scope: String,
    kind: String,
    display_name: String,
    format_or_engine: String,
    locator_json: String,
    credential_ref: Option<String>,
    capabilities_json: String,
    health: String,
    status: String,
    revision: i64,
    schema_version: i64,
) -> Result<DataSourceManifest, MetaError> {
    use medscale_contracts::data_sources::{DataSourceCapability, SourceLocator};
    let locator: SourceLocator = serde_json::from_str(&locator_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad locator json: {e}")))?;
    locator.validate().map_err(MetaError::CorruptObjectBody)?;
    let capabilities: Vec<DataSourceCapability> = serde_json::from_str(&capabilities_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad capabilities json: {e}")))?;
    if capabilities.is_empty() {
        return Err(MetaError::CorruptObjectBody(
            "source capabilities empty".to_owned(),
        ));
    }
    let revision_u64 = u64::try_from(revision)
        .map_err(|_| MetaError::CorruptObjectBody("bad source revision".to_owned()))?;
    if revision_u64 < 1 {
        return Err(MetaError::CorruptObjectBody(
            "bad source revision".to_owned(),
        ));
    }
    Ok(DataSourceManifest {
        header: medscale_contracts::objects::ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: u32::try_from(schema_version).unwrap_or(DATA_SOURCE_SCHEMA_VERSION),
            realm_id: RealmId::new(realm),
            authority_scope_id: AuthorityScopeId::new(scope),
        },
        revision: revision_u64,
        project_id: OpaqueId::new(project_id),
        kind: parse_kind(&kind)?,
        display_name,
        format_or_engine,
        locator,
        credential_ref: credential_ref.map(OpaqueId::new),
        capabilities,
        health: parse_health(&health)?,
        status: parse_source_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new data-source row. Duplicate ids fail as `Conflict`.
    pub fn insert_data_source(&self, source: &DataSourceManifest) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO data_sources(source_id, project_id, realm_id, authority_scope_id, kind, display_name, format_or_engine, locator_json, credential_ref, capabilities_json, health, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                source.header.id.as_str(),
                source.project_id.as_str(),
                source.header.realm_id.as_opaque().as_str(),
                source.header.authority_scope_id.as_opaque().as_str(),
                kind_str(source.kind),
                source.display_name,
                source.format_or_engine,
                serde_json::to_string(&source.locator).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                source.credential_ref.as_ref().map(|c| c.as_str()),
                serde_json::to_string(&source.capabilities).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                health_str(source.health),
                source_status_str(source.status),
                source.revision as i64,
                source.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate data source {}",
                source.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one data-source row with integrity validation.
    pub fn get_data_source(&self, id: &OpaqueId) -> Result<DataSourceManifest, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT source_id, project_id, realm_id, authority_scope_id, kind, display_name, format_or_engine, locator_json, credential_ref, capabilities_json, health, status, revision, schema_version
             FROM data_sources WHERE source_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_source_row(row)
    }

    /// Lists data sources in one project/scope with deterministic id order.
    pub fn list_data_sources(
        &self,
        scope: &AuthorityScopeId,
        project_id: &OpaqueId,
        status: Option<SourceStatus>,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<DataSourceSummary>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let status_str = status.map(source_status_str);
        let mut stmt = self.conn().prepare(
            "SELECT source_id, project_id, kind, display_name, format_or_engine, health, status, revision
             FROM data_sources
             WHERE authority_scope_id = ?1 AND project_id = ?2 AND (?3 IS NULL OR status = ?3) AND source_id > ?4
             ORDER BY source_id LIMIT ?5",
        )?;
        let mapped = stmt.query_map(
            params![
                scope.as_opaque().as_str(),
                project_id.as_str(),
                status_str,
                after,
                limit
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )?;
        let mut out = Vec::new();
        for row in mapped {
            let (id, proj, kind, name, foe, health, status, rev) = row.map_err(to_meta)?;
            out.push(DataSourceSummary {
                source_id: OpaqueId::new(id),
                project_id: OpaqueId::new(proj),
                kind: parse_kind(&kind)?,
                display_name: name,
                format_or_engine: foe,
                health: parse_health(&health)?,
                status: parse_source_status(&status)?,
                revision: u64::try_from(rev).unwrap_or(0),
            });
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|s| s.source_id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Compare-and-swap source display name and credential reference.
    pub fn update_data_source_meta(
        &self,
        id: &OpaqueId,
        expected: u64,
        display_name: &str,
        credential_ref: Option<&OpaqueId>,
    ) -> Result<DataSourceManifest, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM data_sources WHERE source_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale data source revision: expected {expected}"
            )));
        }
        tx.execute(
            "UPDATE data_sources SET display_name = ?1, credential_ref = ?2, revision = revision + 1 WHERE source_id = ?3",
            params![
                display_name,
                credential_ref.map(|c| c.as_str()),
                id.as_str()
            ],
        )?;
        tx.commit()?;
        self.get_data_source(id)
    }

    /// Compare-and-swap source status (archive is terminal; enforced by Core).
    pub fn set_data_source_status(
        &self,
        id: &OpaqueId,
        expected: u64,
        status: SourceStatus,
    ) -> Result<DataSourceManifest, MetaError> {
        self.cas_source_scalar(id, expected, "status", source_status_str(status))
    }

    /// Compare-and-swap observed source health (revision bumps with it).
    pub fn set_data_source_health(
        &self,
        id: &OpaqueId,
        expected: u64,
        health: medscale_contracts::data_sources::SourceHealth,
    ) -> Result<DataSourceManifest, MetaError> {
        self.cas_source_scalar(id, expected, "health", health_str(health))
    }

    fn cas_source_scalar(
        &self,
        id: &OpaqueId,
        expected: u64,
        column: &str,
        value: &str,
    ) -> Result<DataSourceManifest, MetaError> {
        // Column is an internal constant, never caller input.
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM data_sources WHERE source_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale data source revision: expected {expected}"
            )));
        }
        tx.execute(
            &format!("UPDATE data_sources SET {column} = ?1, revision = revision + 1 WHERE source_id = ?2"),
            params![value, id.as_str()],
        )?;
        tx.commit()?;
        self.get_data_source(id)
    }

    /// Full source scan for snapshot export (bounded callers only).
    pub fn list_all_data_sources(&self) -> Result<Vec<DataSourceManifest>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT source_id, project_id, realm_id, authority_scope_id, kind, display_name, format_or_engine, locator_json, credential_ref, capabilities_json, health, status, revision, schema_version
             FROM data_sources ORDER BY source_id",
        )?;
        let mapped = stmt.query_map([], map_source_row_result)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        Ok(out)
    }

    /// Restores one source row exactly (backup restore path only).
    pub fn restore_data_source_row(&self, source: &DataSourceManifest) -> Result<(), MetaError> {
        let manifest = DataSourceManifest {
            revision: source.revision.max(1),
            ..source.clone()
        };
        self.conn().execute(
            "INSERT OR REPLACE INTO data_sources(source_id, project_id, realm_id, authority_scope_id, kind, display_name, format_or_engine, locator_json, credential_ref, capabilities_json, health, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                manifest.header.id.as_str(),
                manifest.project_id.as_str(),
                manifest.header.realm_id.as_opaque().as_str(),
                manifest.header.authority_scope_id.as_opaque().as_str(),
                kind_str(manifest.kind),
                manifest.display_name,
                manifest.format_or_engine,
                serde_json::to_string(&manifest.locator).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                manifest.credential_ref.as_ref().map(|c| c.as_str()),
                serde_json::to_string(&manifest.capabilities).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                health_str(manifest.health),
                source_status_str(manifest.status),
                manifest.revision as i64,
                manifest.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }
}

// ---------- snapshots ----------

fn map_snapshot_row(row: &rusqlite::Row<'_>) -> Result<SnapshotRecord, MetaError> {
    map_snapshot_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
        row.get(6).map_err(MetaError::Sqlite)?,
        row.get(7).map_err(MetaError::Sqlite)?,
        row.get(8).map_err(MetaError::Sqlite)?,
        row.get(9).map_err(MetaError::Sqlite)?,
        row.get(10).map_err(MetaError::Sqlite)?,
        row.get(11).map_err(MetaError::Sqlite)?,
        row.get(12).map_err(MetaError::Sqlite)?,
    )
}

fn map_snapshot_row_result(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Result<SnapshotRecord, MetaError>> {
    Ok(map_snapshot_row_inner(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, Option<String>>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, String>(7)?,
        row.get::<_, String>(8)?,
        row.get::<_, i64>(9)?,
        row.get::<_, String>(10)?,
        row.get::<_, String>(11)?,
        row.get::<_, i64>(12)?,
    ))
}

/// Storage-level snapshot record: the contract snapshot plus its schema and
/// project owner needed for scoped reads and exports.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotRecord {
    pub snapshot: medscale_contracts::data_sources::DataSnapshot,
    pub schema: medscale_contracts::data_sources::SourceSchema,
    pub project_id: OpaqueId,
}

#[allow(clippy::too_many_arguments)]
fn map_snapshot_row_inner(
    id: String,
    source_id: String,
    parent_snapshot_id: Option<String>,
    project_id: String,
    realm: String,
    scope: String,
    source_revision_json: String,
    schema_json: String,
    schema_fingerprint_hex: String,
    row_count: i64,
    content_digest_hex: String,
    status: String,
    schema_version: i64,
) -> Result<SnapshotRecord, MetaError> {
    use medscale_contracts::data_sources::{SourceRevisionBinding, SourceSchema};
    let source_revision: SourceRevisionBinding = serde_json::from_str(&source_revision_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad source revision json: {e}")))?;
    source_revision
        .validate()
        .map_err(MetaError::CorruptObjectBody)?;
    let schema: SourceSchema = serde_json::from_str(&schema_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad schema json: {e}")))?;
    schema.validate().map_err(MetaError::CorruptObjectBody)?;
    let schema_fingerprint = parse_digest_hex(&schema_fingerprint_hex, "schema")?;
    if schema_fingerprint != schema.schema_fingerprint {
        return Err(MetaError::CorruptObjectBody(
            "schema fingerprint mismatch".to_owned(),
        ));
    }
    let content_digest = parse_digest_hex(&content_digest_hex, "content")?;
    let row_count_u64 = u64::try_from(row_count)
        .map_err(|_| MetaError::CorruptObjectBody("bad row count".to_owned()))?;
    let snapshot = medscale_contracts::data_sources::DataSnapshot {
        header: medscale_contracts::objects::ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: u32::try_from(schema_version).unwrap_or(DATA_SOURCE_SCHEMA_VERSION),
            realm_id: RealmId::new(realm),
            authority_scope_id: AuthorityScopeId::new(scope),
        },
        source_id: OpaqueId::new(source_id),
        parent_snapshot_id: parent_snapshot_id.map(OpaqueId::new),
        source_revision,
        schema_fingerprint,
        row_count: row_count_u64,
        content_digest,
        status: parse_snapshot_status(&status)?,
    };
    snapshot.validate().map_err(MetaError::CorruptObjectBody)?;
    Ok(SnapshotRecord {
        snapshot,
        schema,
        project_id: OpaqueId::new(project_id),
    })
}

// Storage-level snapshot part rows travel as contract `SnapshotPart` values.

impl SqliteMetaStore {
    /// Inserts one snapshot with its parts atomically. Duplicate snapshot or
    /// digest-tuple ids fail as `Conflict`.
    pub fn insert_snapshot_full(
        &self,
        record: &SnapshotRecord,
        parts: &[medscale_contracts::data_sources::SnapshotPart],
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let snapshot = &record.snapshot;
        let schema_json = serde_json::to_string(&record.schema)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let revision_json = serde_json::to_string(&snapshot.source_revision)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let insert = tx.execute(
            "INSERT INTO data_snapshots(snapshot_id, source_id, parent_snapshot_id, project_id, realm_id, authority_scope_id, source_revision_json, schema_json, schema_fingerprint_hex, row_count, content_digest_hex, status, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                snapshot.header.id.as_str(),
                snapshot.source_id.as_str(),
                snapshot.parent_snapshot_id.as_ref().map(|p| p.as_str()),
                record.project_id.as_str(),
                snapshot.header.realm_id.as_opaque().as_str(),
                snapshot.header.authority_scope_id.as_opaque().as_str(),
                revision_json,
                schema_json,
                digest_hex(&snapshot.schema_fingerprint),
                snapshot.row_count as i64,
                digest_hex(&snapshot.content_digest),
                snapshot_status_str(&snapshot.status),
                snapshot.header.schema_version as i64,
            ],
        );
        match insert {
            Ok(_) => {}
            Err(e) if is_conflict(&e) => {
                return Err(MetaError::Conflict(format!(
                    "duplicate snapshot {}",
                    snapshot.header.id.as_str()
                )));
            }
            Err(e) => return Err(MetaError::Sqlite(e)),
        }
        for part in parts {
            if part.snapshot_id.as_str() != snapshot.header.id.as_str() {
                return Err(MetaError::CorruptObjectBody(
                    "part snapshot mismatch".to_owned(),
                ));
            }
            part.validate().map_err(MetaError::CorruptObjectBody)?;
            let insert_part = tx.execute(
                "INSERT INTO data_snapshot_parts(snapshot_id, part_index, row_start, row_end, part_digest_hex)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    part.snapshot_id.as_str(),
                    part.part_index as i64,
                    part.row_start as i64,
                    part.row_end as i64,
                    digest_hex(&part.part_digest),
                ],
            );
            match insert_part {
                Ok(_) => {}
                Err(e) if is_conflict(&e) => {
                    return Err(MetaError::Conflict(format!(
                        "duplicate snapshot part {}:{}",
                        part.snapshot_id.as_str(),
                        part.part_index
                    )));
                }
                Err(e) => return Err(MetaError::Sqlite(e)),
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Reads one snapshot record with integrity validation.
    pub fn get_snapshot(&self, id: &OpaqueId) -> Result<SnapshotRecord, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT snapshot_id, source_id, parent_snapshot_id, project_id, realm_id, authority_scope_id, source_revision_json, schema_json, schema_fingerprint_hex, row_count, content_digest_hex, status, schema_version
             FROM data_snapshots WHERE snapshot_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_snapshot_row(row)
    }

    /// Reads ordered parts for one snapshot.
    pub fn get_snapshot_parts(
        &self,
        id: &OpaqueId,
    ) -> Result<Vec<medscale_contracts::data_sources::SnapshotPart>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT snapshot_id, part_index, row_start, row_end, part_digest_hex
             FROM data_snapshot_parts WHERE snapshot_id = ?1 ORDER BY part_index",
        )?;
        let mapped = stmt.query_map(params![id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (snap, index, start, end, digest_hex) = row.map_err(to_meta)?;
            let part = medscale_contracts::data_sources::SnapshotPart {
                snapshot_id: OpaqueId::new(snap),
                part_index: u32::try_from(index).unwrap_or(u32::MAX),
                row_start: u64::try_from(start).unwrap_or(u64::MAX),
                row_end: u64::try_from(end).unwrap_or(0),
                part_digest: parse_digest_hex(&digest_hex, "part")?,
            };
            part.validate().map_err(MetaError::CorruptObjectBody)?;
            out.push(part);
        }
        Ok(out)
    }

    /// Lists snapshots for one source, oldest `snapshot_id` first (stable
    /// keyset pagination), with `after` cursor. Callers that need the most
    /// recent snapshot must take the last element of a page, not the first.
    pub fn list_snapshots(
        &self,
        source_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<SnapshotSummary>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT snapshot_id, source_id, row_count, content_digest_hex, status
             FROM data_snapshots
             WHERE source_id = ?1 AND snapshot_id > ?2
             ORDER BY snapshot_id LIMIT ?3",
        )?;
        let mapped = stmt.query_map(params![source_id.as_str(), after, limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (id, src, rows, digest_hex, status) = row.map_err(to_meta)?;
            out.push(SnapshotSummary {
                snapshot_id: OpaqueId::new(id),
                source_id: OpaqueId::new(src),
                row_count: u64::try_from(rows).unwrap_or(0),
                content_digest: parse_digest_hex(&digest_hex, "content")?,
                status: parse_snapshot_status(&status)?,
            });
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|s| s.snapshot_id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Returns the most recently inserted snapshot for one source, if any.
    pub fn latest_snapshot_for_source(
        &self,
        source_id: &OpaqueId,
    ) -> Result<Option<SnapshotRecord>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT snapshot_id, source_id, parent_snapshot_id, project_id, realm_id, authority_scope_id, source_revision_json, schema_json, schema_fingerprint_hex, row_count, content_digest_hex, status, schema_version
             FROM data_snapshots WHERE source_id = ?1 ORDER BY rowid DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![source_id.as_str()])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => Ok(Some(map_snapshot_row(row)?)),
        }
    }

    /// Finds a snapshot by exact content digest (idempotent re-import).
    pub fn find_snapshot_by_digest(
        &self,
        source_id: &OpaqueId,
        digest: &DigestSha256,
    ) -> Result<Option<SnapshotRecord>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT snapshot_id, source_id, parent_snapshot_id, project_id, realm_id, authority_scope_id, source_revision_json, schema_json, schema_fingerprint_hex, row_count, content_digest_hex, status, schema_version
             FROM data_snapshots WHERE source_id = ?1 AND content_digest_hex = ?2",
        )?;
        let mut rows = stmt.query(params![source_id.as_str(), digest_hex(digest)])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => Ok(Some(map_snapshot_row(row)?)),
        }
    }

    /// Full snapshot scan for snapshot export (bounded callers only).
    pub fn list_all_snapshots(&self) -> Result<Vec<SnapshotRecord>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT snapshot_id, source_id, parent_snapshot_id, project_id, realm_id, authority_scope_id, source_revision_json, schema_json, schema_fingerprint_hex, row_count, content_digest_hex, status, schema_version
             FROM data_snapshots ORDER BY snapshot_id",
        )?;
        let mapped = stmt.query_map([], map_snapshot_row_result)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        Ok(out)
    }

    /// Full part scan for snapshot export (bounded callers only).
    pub fn list_all_snapshot_parts(
        &self,
    ) -> Result<Vec<medscale_contracts::data_sources::SnapshotPart>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT snapshot_id, part_index, row_start, row_end, part_digest_hex
             FROM data_snapshot_parts ORDER BY snapshot_id, part_index",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (snap, index, start, end, digest_hex) = row.map_err(to_meta)?;
            let part = medscale_contracts::data_sources::SnapshotPart {
                snapshot_id: OpaqueId::new(snap),
                part_index: u32::try_from(index).unwrap_or(u32::MAX),
                row_start: u64::try_from(start).unwrap_or(u64::MAX),
                row_end: u64::try_from(end).unwrap_or(0),
                part_digest: parse_digest_hex(&digest_hex, "part")?,
            };
            part.validate().map_err(MetaError::CorruptObjectBody)?;
            out.push(part);
        }
        Ok(out)
    }

    /// Restores one snapshot plus parts exactly (backup restore path only).
    pub fn restore_snapshot_full(
        &self,
        record: &SnapshotRecord,
        parts: &[medscale_contracts::data_sources::SnapshotPart],
    ) -> Result<(), MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let snapshot = &record.snapshot;
        let schema_json = serde_json::to_string(&record.schema)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let revision_json = serde_json::to_string(&snapshot.source_revision)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        tx.execute(
            "INSERT OR REPLACE INTO data_snapshots(snapshot_id, source_id, parent_snapshot_id, project_id, realm_id, authority_scope_id, source_revision_json, schema_json, schema_fingerprint_hex, row_count, content_digest_hex, status, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                snapshot.header.id.as_str(),
                snapshot.source_id.as_str(),
                snapshot.parent_snapshot_id.as_ref().map(|p| p.as_str()),
                record.project_id.as_str(),
                snapshot.header.realm_id.as_opaque().as_str(),
                snapshot.header.authority_scope_id.as_opaque().as_str(),
                revision_json,
                schema_json,
                digest_hex(&snapshot.schema_fingerprint),
                snapshot.row_count as i64,
                digest_hex(&snapshot.content_digest),
                snapshot_status_str(&snapshot.status),
                snapshot.header.schema_version as i64,
            ],
        )?;
        tx.execute(
            "DELETE FROM data_snapshot_parts WHERE snapshot_id = ?1",
            params![snapshot.header.id.as_str()],
        )?;
        for part in parts {
            tx.execute(
                "INSERT OR REPLACE INTO data_snapshot_parts(snapshot_id, part_index, row_start, row_end, part_digest_hex)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    part.snapshot_id.as_str(),
                    part.part_index as i64,
                    part.row_start as i64,
                    part.row_end as i64,
                    digest_hex(&part.part_digest),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

// ---------- receipts ----------

/// Receipt kinds stored in `data_receipts`.
pub const RECEIPT_KIND_IMPORT: &str = "import";
/// Receipt kinds stored in `data_receipts`.
pub const RECEIPT_KIND_REFRESH: &str = "refresh";
/// Receipt kinds stored in `data_receipts`.
pub const RECEIPT_KIND_TRANSFORMATION: &str = "transformation";

impl SqliteMetaStore {
    /// Stores one receipt. Refresh receipts upsert idempotently; import and
    /// transformation receipts are insert-once and duplicates conflict.
    pub fn put_receipt(
        &self,
        kind: &str,
        snapshot_id: &OpaqueId,
        receipt_json: &serde_json::Value,
    ) -> Result<(), MetaError> {
        if kind != RECEIPT_KIND_IMPORT
            && kind != RECEIPT_KIND_REFRESH
            && kind != RECEIPT_KIND_TRANSFORMATION
        {
            return Err(MetaError::UnsupportedSchema(format!(
                "unknown receipt kind {kind}"
            )));
        }
        let text = serde_json::to_string(receipt_json)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        let result = if kind == RECEIPT_KIND_REFRESH {
            self.conn().execute(
                "INSERT OR REPLACE INTO data_receipts(receipt_kind, snapshot_id, receipt_json)
                 VALUES (?1, ?2, ?3)",
                params![kind, snapshot_id.as_str(), text],
            )
        } else {
            self.conn().execute(
                "INSERT INTO data_receipts(receipt_kind, snapshot_id, receipt_json)
                 VALUES (?1, ?2, ?3)",
                params![kind, snapshot_id.as_str(), text],
            )
        };
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate {kind} receipt for {}",
                snapshot_id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one receipt.
    pub fn get_receipt(
        &self,
        kind: &str,
        snapshot_id: &OpaqueId,
    ) -> Result<serde_json::Value, MetaError> {
        let text: Option<String> = self
            .conn()
            .query_row(
                "SELECT receipt_json FROM data_receipts WHERE receipt_kind = ?1 AND snapshot_id = ?2",
                params![kind, snapshot_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(text) = text else {
            return Err(MetaError::NotFound);
        };
        serde_json::from_str(&text)
            .map_err(|e| MetaError::CorruptObjectBody(format!("bad receipt json: {e}")))
    }

    /// Full receipt scan for snapshot export (bounded callers only).
    pub fn list_all_receipts(&self) -> Result<Vec<(String, String, serde_json::Value)>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT receipt_kind, snapshot_id, receipt_json FROM data_receipts ORDER BY receipt_kind, snapshot_id",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (kind, snap, text) = row.map_err(to_meta)?;
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| MetaError::CorruptObjectBody(format!("bad receipt json: {e}")))?;
            out.push((kind, snap, value));
        }
        Ok(out)
    }

    /// Restores one receipt exactly (backup restore path only).
    pub fn restore_receipt_row(
        &self,
        kind: &str,
        snapshot_id: &str,
        receipt: &serde_json::Value,
    ) -> Result<(), MetaError> {
        let text = serde_json::to_string(receipt)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO data_receipts(receipt_kind, snapshot_id, receipt_json)
             VALUES (?1, ?2, ?3)",
            params![kind, snapshot_id, text],
        )?;
        Ok(())
    }
}

// ---------- saved views ----------

fn map_saved_view_row(row: &rusqlite::Row<'_>) -> Result<SavedDataView, MetaError> {
    map_saved_view_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
        row.get(6).map_err(MetaError::Sqlite)?,
        row.get(7).map_err(MetaError::Sqlite)?,
        row.get(8).map_err(MetaError::Sqlite)?,
        row.get(9).map_err(MetaError::Sqlite)?,
    )
}

#[allow(clippy::too_many_arguments)]
fn map_saved_view_row_inner(
    id: String,
    snapshot_id: String,
    _project_id: String,
    realm: String,
    scope: String,
    view_kind: String,
    state_json: String,
    status: String,
    revision: i64,
    schema_version: i64,
) -> Result<SavedDataView, MetaError> {
    use medscale_contracts::data_sources::ViewState;
    let state: ViewState = serde_json::from_str(&state_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad view state json: {e}")))?;
    let revision_u64 = u64::try_from(revision)
        .map_err(|_| MetaError::CorruptObjectBody("bad view revision".to_owned()))?;
    if revision_u64 < 1 {
        return Err(MetaError::CorruptObjectBody("bad view revision".to_owned()));
    }
    Ok(SavedDataView {
        header: medscale_contracts::objects::ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: u32::try_from(schema_version).unwrap_or(DATA_SOURCE_SCHEMA_VERSION),
            realm_id: RealmId::new(realm),
            authority_scope_id: AuthorityScopeId::new(scope),
        },
        snapshot_id: OpaqueId::new(snapshot_id),
        revision: revision_u64,
        view_kind: parse_view_kind(&view_kind)?,
        state,
        status: parse_saved_view_status(&status)?,
    })
}

impl SqliteMetaStore {
    /// Inserts a new saved-view row. Duplicate ids fail as `Conflict`.
    pub fn insert_saved_view(
        &self,
        view: &SavedDataView,
        project_id: &OpaqueId,
    ) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO data_saved_views(view_id, snapshot_id, project_id, realm_id, authority_scope_id, view_kind, state_json, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                view.header.id.as_str(),
                view.snapshot_id.as_str(),
                project_id.as_str(),
                view.header.realm_id.as_opaque().as_str(),
                view.header.authority_scope_id.as_opaque().as_str(),
                view_kind_str(view.view_kind),
                serde_json::to_string(&view.state).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                saved_view_status_str(view.status),
                view.revision as i64,
                view.header.schema_version as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate saved view {}",
                view.header.id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one saved-view row with integrity validation.
    pub fn get_saved_view(&self, id: &OpaqueId) -> Result<SavedDataView, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT view_id, snapshot_id, project_id, realm_id, authority_scope_id, view_kind, state_json, status, revision, schema_version
             FROM data_saved_views WHERE view_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_saved_view_row(row)
    }

    /// Lists saved views for one snapshot with deterministic id order.
    pub fn list_saved_views(
        &self,
        snapshot_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<SavedViewSummary>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT view_id, snapshot_id, view_kind, status, revision
             FROM data_saved_views
             WHERE snapshot_id = ?1 AND view_id > ?2
             ORDER BY view_id LIMIT ?3",
        )?;
        let mapped = stmt.query_map(params![snapshot_id.as_str(), after, limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (id, snap, kind, status, rev) = row.map_err(to_meta)?;
            out.push(SavedViewSummary {
                view_id: OpaqueId::new(id),
                snapshot_id: OpaqueId::new(snap),
                view_kind: parse_view_kind(&kind)?,
                status: parse_saved_view_status(&status)?,
                revision: u64::try_from(rev).unwrap_or(0),
            });
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|v| v.view_id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Compare-and-swap saved-view state.
    pub fn update_saved_view_state(
        &self,
        id: &OpaqueId,
        expected: u64,
        state: &medscale_contracts::data_sources::ViewState,
    ) -> Result<SavedDataView, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM data_saved_views WHERE view_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale saved view revision: expected {expected}"
            )));
        }
        let state_json = serde_json::to_string(state)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        tx.execute(
            "UPDATE data_saved_views SET state_json = ?1, revision = revision + 1 WHERE view_id = ?2",
            params![state_json, id.as_str()],
        )?;
        tx.commit()?;
        self.get_saved_view(id)
    }

    /// Compare-and-swap saved-view status (archive terminal; enforced by Core).
    pub fn set_saved_view_status(
        &self,
        id: &OpaqueId,
        expected: u64,
        status: SavedViewStatus,
    ) -> Result<SavedDataView, MetaError> {
        let tx = self.conn().unchecked_transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM data_saved_views WHERE view_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current_rev) = current else {
            return Err(MetaError::NotFound);
        };
        if current_rev != expected as i64 {
            return Err(MetaError::Conflict(format!(
                "stale saved view revision: expected {expected}"
            )));
        }
        tx.execute(
            "UPDATE data_saved_views SET status = ?1, revision = revision + 1 WHERE view_id = ?2",
            params![saved_view_status_str(status), id.as_str()],
        )?;
        tx.commit()?;
        self.get_saved_view(id)
    }

    /// Full saved-view scan with project owners for snapshot export.
    pub fn list_all_saved_views(&self) -> Result<Vec<(SavedDataView, OpaqueId)>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT view_id, snapshot_id, project_id, realm_id, authority_scope_id, view_kind, state_json, status, revision, schema_version
             FROM data_saved_views ORDER BY view_id",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (id, snap, proj, realm, scope, kind, state, status, rev, ver) =
                row.map_err(to_meta)?;
            let view = map_saved_view_row_inner(
                id,
                snap,
                proj.clone(),
                realm,
                scope,
                kind,
                state,
                status,
                rev,
                ver,
            )?;
            out.push((view, OpaqueId::new(proj)));
        }
        Ok(out)
    }

    /// Restores one saved-view row exactly (backup restore path only).
    pub fn restore_saved_view_row(
        &self,
        view: &SavedDataView,
        project_id: &OpaqueId,
    ) -> Result<(), MetaError> {
        let state_json = serde_json::to_string(&view.state)
            .map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?;
        self.conn().execute(
            "INSERT OR REPLACE INTO data_saved_views(view_id, snapshot_id, project_id, realm_id, authority_scope_id, view_kind, state_json, status, revision, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                view.header.id.as_str(),
                view.snapshot_id.as_str(),
                project_id.as_str(),
                view.header.realm_id.as_opaque().as_str(),
                view.header.authority_scope_id.as_opaque().as_str(),
                view_kind_str(view.view_kind),
                state_json,
                saved_view_status_str(view.status),
                view.revision.max(1) as i64,
                view.header.schema_version as i64,
            ],
        )?;
        Ok(())
    }
}

// ---------- transformations ----------

/// Storage-level transformation record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransformationRecord {
    pub output_snapshot_id: OpaqueId,
    pub first_input_snapshot_id: OpaqueId,
    pub input_ids: Vec<OpaqueId>,
    pub ops: Vec<medscale_contracts::data_sources::TransformOp>,
    pub parameters_digest: DigestSha256,
    pub receipt: medscale_contracts::data_sources::TransformationReceipt,
}

impl SqliteMetaStore {
    /// Inserts one transformation record. Duplicate outputs fail as `Conflict`.
    pub fn insert_transformation(&self, record: &TransformationRecord) -> Result<(), MetaError> {
        let result = self.conn().execute(
            "INSERT INTO data_transformations(output_snapshot_id, first_input_snapshot_id, input_ids_json, ops_json, parameters_digest_hex, receipt_json, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.output_snapshot_id.as_str(),
                record.first_input_snapshot_id.as_str(),
                serde_json::to_string(&record.input_ids.iter().map(|i| i.as_str()).collect::<Vec<_>>()).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                serde_json::to_string(&record.ops).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                digest_hex(&record.parameters_digest),
                serde_json::to_string(&record.receipt).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                DATA_SOURCE_SCHEMA_VERSION as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate transformation output {}",
                record.output_snapshot_id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one transformation record by output snapshot.
    pub fn get_transformation_by_output(
        &self,
        output: &OpaqueId,
    ) -> Result<TransformationRecord, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT output_snapshot_id, first_input_snapshot_id, input_ids_json, ops_json, parameters_digest_hex, receipt_json
             FROM data_transformations WHERE output_snapshot_id = ?1",
        )?;
        let mut rows = stmt.query(params![output.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_transformation_row(row)
    }

    /// Lists transformation outputs whose first input matches (lineage walk).
    pub fn list_transformations_by_first_input(
        &self,
        input: &OpaqueId,
    ) -> Result<Vec<OpaqueId>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT output_snapshot_id FROM data_transformations
             WHERE first_input_snapshot_id = ?1 ORDER BY output_snapshot_id",
        )?;
        let mapped = stmt.query_map(params![input.as_str()], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(OpaqueId::new(row.map_err(to_meta)?));
        }
        Ok(out)
    }

    /// Full transformation scan for snapshot export (bounded callers only).
    pub fn list_all_transformations(&self) -> Result<Vec<TransformationRecord>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT output_snapshot_id, first_input_snapshot_id, input_ids_json, ops_json, parameters_digest_hex, receipt_json
             FROM data_transformations ORDER BY output_snapshot_id",
        )?;
        let mapped = stmt.query_map([], map_transformation_row_result)?;
        let mut out = Vec::new();
        for row in mapped {
            out.push(row.map_err(to_meta)??);
        }
        Ok(out)
    }

    /// Restores one transformation record exactly (backup restore path only).
    pub fn restore_transformation_row(
        &self,
        record: &TransformationRecord,
    ) -> Result<(), MetaError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO data_transformations(output_snapshot_id, first_input_snapshot_id, input_ids_json, ops_json, parameters_digest_hex, receipt_json, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.output_snapshot_id.as_str(),
                record.first_input_snapshot_id.as_str(),
                serde_json::to_string(&record.input_ids.iter().map(|i| i.as_str()).collect::<Vec<_>>()).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                serde_json::to_string(&record.ops).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                digest_hex(&record.parameters_digest),
                serde_json::to_string(&record.receipt).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                DATA_SOURCE_SCHEMA_VERSION as i64,
            ],
        )?;
        Ok(())
    }
}

fn map_transformation_row(row: &rusqlite::Row<'_>) -> Result<TransformationRecord, MetaError> {
    map_transformation_row_inner(
        row.get(0).map_err(MetaError::Sqlite)?,
        row.get(1).map_err(MetaError::Sqlite)?,
        row.get(2).map_err(MetaError::Sqlite)?,
        row.get(3).map_err(MetaError::Sqlite)?,
        row.get(4).map_err(MetaError::Sqlite)?,
        row.get(5).map_err(MetaError::Sqlite)?,
    )
}

fn map_transformation_row_result(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Result<TransformationRecord, MetaError>> {
    Ok(map_transformation_row_inner(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, String>(5)?,
    ))
}

fn map_transformation_row_inner(
    output: String,
    first_input: String,
    input_ids_json: String,
    ops_json: String,
    parameters_digest_hex: String,
    receipt_json: String,
) -> Result<TransformationRecord, MetaError> {
    use medscale_contracts::data_sources::{TransformOp, TransformationReceipt};
    let input_strs: Vec<String> = serde_json::from_str(&input_ids_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad input ids json: {e}")))?;
    if input_strs.is_empty() || input_strs.len() > 2 {
        return Err(MetaError::CorruptObjectBody(
            "transformation inputs out of bound".to_owned(),
        ));
    }
    let ops: Vec<TransformOp> = serde_json::from_str(&ops_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad ops json: {e}")))?;
    if ops.is_empty() {
        return Err(MetaError::CorruptObjectBody(
            "transformation ops empty".to_owned(),
        ));
    }
    let receipt: TransformationReceipt = serde_json::from_str(&receipt_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad receipt json: {e}")))?;
    receipt.validate().map_err(MetaError::CorruptObjectBody)?;
    Ok(TransformationRecord {
        output_snapshot_id: OpaqueId::new(output),
        first_input_snapshot_id: OpaqueId::new(first_input),
        input_ids: input_strs.into_iter().map(OpaqueId::new).collect(),
        ops,
        parameters_digest: parse_digest_hex(&parameters_digest_hex, "parameters")?,
        receipt,
    })
}

// ---------- dataset releases ----------

impl SqliteMetaStore {
    /// Inserts one dataset release. The release inherits realm/scope from its
    /// snapshot row; a missing snapshot fails as `NotFound`. Duplicate ids
    /// fail as `Conflict`.
    pub fn insert_dataset_release(&self, release: &ReleaseManifest) -> Result<(), MetaError> {
        let owner: Option<(String, String)> = self
            .conn()
            .query_row(
                "SELECT realm_id, authority_scope_id FROM data_snapshots WHERE snapshot_id = ?1",
                params![release.card.snapshot_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((realm, scope)) = owner else {
            return Err(MetaError::NotFound);
        };
        let result = self.conn().execute(
            "INSERT INTO dataset_releases(release_id, snapshot_id, project_id, realm_id, authority_scope_id, card_json, snapshot_digest_hex, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                release.release_id.as_str(),
                release.card.snapshot_id.as_str(),
                release.card.project_id.as_str(),
                realm,
                scope,
                serde_json::to_string(&release.card).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                digest_hex(&release.snapshot_digest),
                DATA_SOURCE_SCHEMA_VERSION as i64,
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) if is_conflict(&e) => Err(MetaError::Conflict(format!(
                "duplicate dataset release {}",
                release.release_id.as_str()
            ))),
            Err(e) => Err(MetaError::Sqlite(e)),
        }
    }

    /// Reads one dataset release with integrity validation.
    pub fn get_dataset_release(&self, id: &OpaqueId) -> Result<ReleaseManifest, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT release_id, snapshot_id, card_json, snapshot_digest_hex
             FROM dataset_releases WHERE release_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_str()])?;
        let row = rows.next()?.ok_or(MetaError::NotFound)?;
        map_release_row(row)
    }

    /// Lists releases for one project with deterministic id order.
    pub fn list_dataset_releases(
        &self,
        project_id: &OpaqueId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<(Vec<DatasetReleaseSummary>, Option<String>), MetaError> {
        check_cursor(after)?;
        let limit = limit.clamp(1, 100) as i64 + 1;
        let after = after.unwrap_or_default();
        let mut stmt = self.conn().prepare(
            "SELECT release_id, snapshot_id, card_json
             FROM dataset_releases
             WHERE project_id = ?1 AND release_id > ?2
             ORDER BY release_id LIMIT ?3",
        )?;
        let mapped = stmt.query_map(params![project_id.as_str(), after, limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (id, snap, card_json) = row.map_err(to_meta)?;
            let card: medscale_contracts::data_sources::DatasetCard =
                serde_json::from_str(&card_json)
                    .map_err(|e| MetaError::CorruptObjectBody(format!("bad card json: {e}")))?;
            out.push(DatasetReleaseSummary {
                release_id: OpaqueId::new(id),
                snapshot_id: OpaqueId::new(snap),
                version: card.version,
                rights_state: card.rights_state,
            });
        }
        let next = if out.len() == limit as usize {
            out.pop();
            out.last().map(|r| r.release_id.as_str().to_owned())
        } else {
            None
        };
        Ok((out, next))
    }

    /// Full release scan for snapshot export (bounded callers only).
    pub fn list_all_dataset_releases(&self) -> Result<Vec<ReleaseManifest>, MetaError> {
        let mut stmt = self.conn().prepare(
            "SELECT release_id, snapshot_id, card_json, snapshot_digest_hex
             FROM dataset_releases ORDER BY release_id",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in mapped {
            let (id, snap, card_json, digest_hex) = row.map_err(to_meta)?;
            out.push(map_release_row_parts(&id, &snap, &card_json, &digest_hex)?);
        }
        Ok(out)
    }

    /// Restores one release exactly (backup restore path only). Snapshots must
    /// restore before releases so realm/scope inheritance resolves.
    pub fn restore_dataset_release_row(&self, release: &ReleaseManifest) -> Result<(), MetaError> {
        let owner: Option<(String, String)> = self
            .conn()
            .query_row(
                "SELECT realm_id, authority_scope_id FROM data_snapshots WHERE snapshot_id = ?1",
                params![release.card.snapshot_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((realm, scope)) = owner else {
            return Err(MetaError::NotFound);
        };
        self.conn().execute(
            "INSERT OR REPLACE INTO dataset_releases(release_id, snapshot_id, project_id, realm_id, authority_scope_id, card_json, snapshot_digest_hex, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                release.release_id.as_str(),
                release.card.snapshot_id.as_str(),
                release.card.project_id.as_str(),
                realm,
                scope,
                serde_json::to_string(&release.card).map_err(|e| MetaError::CorruptObjectBody(e.to_string()))?,
                digest_hex(&release.snapshot_digest),
                DATA_SOURCE_SCHEMA_VERSION as i64,
            ],
        )?;
        Ok(())
    }
}

fn map_release_row(row: &rusqlite::Row<'_>) -> Result<ReleaseManifest, MetaError> {
    let id: String = row.get(0).map_err(MetaError::Sqlite)?;
    let snap: String = row.get(1).map_err(MetaError::Sqlite)?;
    let card_json: String = row.get(2).map_err(MetaError::Sqlite)?;
    let digest_hex: String = row.get(3).map_err(MetaError::Sqlite)?;
    map_release_row_parts(&id, &snap, &card_json, &digest_hex)
}

fn map_release_row_parts(
    id: &str,
    snap: &str,
    card_json: &str,
    digest_hex_str: &str,
) -> Result<ReleaseManifest, MetaError> {
    use medscale_contracts::data_sources::DatasetCard;
    let card: DatasetCard = serde_json::from_str(card_json)
        .map_err(|e| MetaError::CorruptObjectBody(format!("bad card json: {e}")))?;
    card.validate().map_err(MetaError::CorruptObjectBody)?;
    if card.snapshot_id.as_str() != snap {
        return Err(MetaError::CorruptObjectBody(
            "release card snapshot mismatch".to_owned(),
        ));
    }
    Ok(ReleaseManifest {
        release_id: OpaqueId::new(id.to_owned()),
        card,
        snapshot_digest: parse_digest_hex(digest_hex_str, "snapshot")?,
    })
}

// ---------- external SQLite read adapter ----------

/// Maximum tables returned by bounded external discovery.
pub const EXTERNAL_TABLES_MAX: usize = 256;

/// Maximum identifier length for external table names.
pub const EXTERNAL_IDENTIFIER_MAX_CHARS: usize = 128;

/// One cell read from an external SQLite table. BLOB values are refused at
/// read time (explicit `UnsupportedSchema`), never silently decoded.
#[derive(Debug, Clone, PartialEq)]
pub enum ExternalCell {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

/// One table read from an external SQLite file: column names in ordinal order
/// plus nullability from `PRAGMA table_info`, and bounded value rows.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalTable {
    pub columns: Vec<String>,
    pub not_null: Vec<bool>,
    pub rows: Vec<Vec<ExternalCell>>,
}

/// Validates an external table identifier against a closed charset. No free
/// SQL passes this boundary; the identifier is still quoted at use.
pub fn validate_external_identifier(table: &str) -> Result<(), MetaError> {
    if table.is_empty() || table.chars().count() > EXTERNAL_IDENTIFIER_MAX_CHARS {
        return Err(MetaError::UnsupportedSchema(
            "external table name out of bound".to_owned(),
        ));
    }
    let mut chars = table.chars();
    let first = chars.next().unwrap_or('0');
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(MetaError::UnsupportedSchema(
            "external table name must start with a letter or underscore".to_owned(),
        ));
    }
    if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(MetaError::UnsupportedSchema(
            "external table name uses unsupported characters".to_owned(),
        ));
    }
    if table.to_ascii_lowercase().starts_with("sqlite_") {
        return Err(MetaError::UnsupportedSchema(
            "sqlite internal tables are not readable sources".to_owned(),
        ));
    }
    Ok(())
}

fn open_external_readonly(db_path: &std::path::Path) -> Result<rusqlite::Connection, MetaError> {
    if !db_path.is_file() {
        return Err(MetaError::NotFound);
    }
    // RUSTSEC-2026-0459 (CVE-2026-119375): SQLite before 3.54 can return
    // uninitialised memory when a database is opened with a crafted `-wal`
    // sibling whose page size differs. The bundled SQLCipher has no fixed
    // release yet, and an external database is the one file MedScale opens
    // that someone else may have written, so one with a write-ahead log is
    // refused. Checkpointing the source or copying the database file alone
    // removes the log.
    let mut wal = db_path.as_os_str().to_owned();
    wal.push("-wal");
    if std::path::Path::new(&wal).exists() {
        return Err(MetaError::UnsupportedSchema(
            "an external database with a write-ahead log (-wal file) is not admitted;              checkpoint it or copy the database file on its own first"
                .to_owned(),
        ));
    }
    let conn =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    // Defense in depth: read-only open plus query-only pragma.
    conn.execute_batch("PRAGMA query_only = ON;")?;
    Ok(conn)
}

/// Bounded external schema discovery: user tables in name order, charset
/// filtered, capped. Internal `sqlite_%` tables are never listed.
pub fn list_external_sqlite_tables(db_path: &std::path::Path) -> Result<Vec<String>, MetaError> {
    let conn = open_external_readonly(db_path)?;
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name LIMIT ?1")?;
    let mapped = stmt.query_map(params![EXTERNAL_TABLES_MAX as i64 + 1], |row| {
        row.get::<_, String>(0)
    })?;
    let mut out = Vec::new();
    for row in mapped {
        let name = row.map_err(to_meta)?;
        if validate_external_identifier(&name).is_ok() {
            out.push(name);
        }
        if out.len() > EXTERNAL_TABLES_MAX {
            break;
        }
    }
    Ok(out)
}

/// Reads one external table read-only with an explicit row bound. BLOB cells
/// fail the whole read as `UnsupportedSchema`; no partial binary decoding.
pub fn read_external_sqlite_table(
    db_path: &std::path::Path,
    table: &str,
    max_rows: u64,
) -> Result<ExternalTable, MetaError> {
    validate_external_identifier(table)?;
    let conn = open_external_readonly(db_path)?;
    let known: Vec<String> = list_external_sqlite_tables(db_path)?;
    if !known.iter().any(|t| t == table) {
        return Err(MetaError::NotFound);
    }
    let quoted = format!("\"{}\"", table.replace('"', "\"\""));
    let info: Vec<(String, bool)> = {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({quoted})"))?;
        let mapped = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(3)?))
        })?;
        let mut cols = Vec::new();
        for row in mapped {
            let (name, notnull) = row.map_err(to_meta)?;
            cols.push((name, notnull != 0));
        }
        cols
    };
    if info.is_empty() || info.len() > medscale_contracts::data_sources::SCHEMA_FIELDS_MAX {
        return Err(MetaError::UnsupportedSchema(
            "external table has no usable columns".to_owned(),
        ));
    }
    let bound = max_rows
        .clamp(1, medscale_contracts::data_sources::SNAPSHOT_ROWS_MAX)
        .to_string();
    let mut stmt = conn.prepare(&format!("SELECT * FROM {quoted} LIMIT {bound}"))?;
    let width = stmt.column_count();
    let mut rows = stmt.query([])?;
    let mut out_rows = Vec::new();
    while let Some(row) = rows.next()? {
        let mut out_row = Vec::with_capacity(width);
        for index in 0..width {
            let value: rusqlite::types::Value = row.get(index)?;
            out_row.push(match value {
                rusqlite::types::Value::Null => ExternalCell::Null,
                rusqlite::types::Value::Integer(v) => ExternalCell::Integer(v),
                rusqlite::types::Value::Real(v) => ExternalCell::Real(v),
                rusqlite::types::Value::Text(v) => {
                    if v.len() > medscale_contracts::data_sources::CELL_TEXT_MAX_BYTES {
                        return Err(MetaError::UnsupportedSchema(
                            "external text cell exceeds bound".to_owned(),
                        ));
                    }
                    if v.contains('\0') {
                        return Err(MetaError::UnsupportedSchema(
                            "external text cell contains NUL".to_owned(),
                        ));
                    }
                    ExternalCell::Text(v)
                }
                rusqlite::types::Value::Blob(_) => {
                    return Err(MetaError::UnsupportedSchema(
                        "external BLOB columns are not readable sources".to_owned(),
                    ));
                }
            });
        }
        out_rows.push(out_row);
    }
    Ok(ExternalTable {
        columns: info.iter().map(|(name, _)| name.clone()).collect(),
        not_null: info.into_iter().map(|(_, nn)| nn).collect(),
        rows: out_rows,
    })
}

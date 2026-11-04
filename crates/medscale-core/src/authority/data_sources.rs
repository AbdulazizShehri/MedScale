//! Data Source Fabric Core authority paths (Spec 075).
//!
//! Every mutation flows: actor/session -> realm/scope -> authority ->
//! validation -> expected revision -> source validation -> quarantined
//! acquisition -> transaction -> audit/receipt -> typed result. Surfaces
//! never touch storage, drivers, or transports.
//!
//! Audit: source/view lifecycle mutations and release creation append an
//! `ActionAuditRecord` to the existing trail. Snapshot import/refresh and
//! transform execution return durable receipts instead: the revisioned sqlite
//! rows plus content-addressed blobs ARE the record, and a per-op memory
//! audit row would make every op O(full history) through the frozen
//! full-snapshot sync (same amendment as Spec 074). No second audit system
//! exists; no surface bypasses Core.
//!
//! Blob routing: synthetic vaults use the FS blob store, encrypted vaults
//! use the sealed blob store. Snapshot bytes are verified on read-back after
//! write and on every query; digest mismatch yields `Corrupt`, never
//! best-effort data.

use std::path::{Path, PathBuf};

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::data_sources::{
    AcquireOutcome, DATA_SOURCE_SCHEMA_VERSION, DataSourceCapability, DataSourceKind,
    DataSourceManifest, DataSourceSummary, DataTransformation, DataViewKind, DatabaseEngine,
    DatasetCard, DatasetReleaseSummary, FilterExpr, ImportReceipt, LocalFileFormat,
    RefreshChangeClass, RefreshReceipt, ReleaseManifest, RemoteDatasetProvider, RightsState,
    SNAPSHOT_BYTES_MAX, SavedDataView, SavedViewStatus, SavedViewSummary, SnapshotRowPage,
    SnapshotStatus, SnapshotSummary, SortKey, SourceHealth, SourceLocator, SourceRevisionBinding,
    SourceSchema, SourceStatus, TransformOp, TransformationReceipt, ViewState,
    effective_list_limit, effective_rows_limit, fingerprint_schema, parse_row_cursor,
    render_row_cursor, validate_cursor, validate_display_name,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::network::EgressAllowlistEntry;
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId, VaultId,
};
use medscale_network::BrokerTransport;
use medscale_storage::{
    EncryptedVault, MetaError, RECEIPT_KIND_IMPORT, RECEIPT_KIND_REFRESH,
    RECEIPT_KIND_TRANSFORMATION, SnapshotRecord, SqliteMetaStore, SyntheticVault,
    TransformationRecord,
};

use super::data_acquire::{
    AcquireFail, ParsedTable, RemoteFetchSpec, apply_view, brokered_dataset_fetch,
    decode_snapshot_bytes, execute_transform, map_external_table, materialize_parts,
    parse_delimited, parse_fetched_files, parse_json_bytes,
};
use super::store::{InMemoryAuthorityStore, StoredObject};
use crate::process::{LeaseRegistry, SessionRegistry};

// ---------- blob backend ----------

/// Snapshot byte backend bound to the open vault kind.
pub enum SnapshotBlobBackend<'a> {
    Synthetic(&'a SyntheticVault),
    Encrypted(&'a EncryptedVault),
}

impl SnapshotBlobBackend<'_> {
    fn meta(&self) -> &SqliteMetaStore {
        match self {
            Self::Synthetic(vault) => &vault.meta,
            Self::Encrypted(vault) => &vault.meta,
        }
    }

    fn vault_root(&self) -> &Path {
        match self {
            Self::Synthetic(vault) => &vault.root,
            Self::Encrypted(vault) => &vault.root,
        }
    }

    fn put_bytes(&self, bytes: &[u8]) -> Result<(DigestSha256, u64), AuthorityError> {
        let byte_length = bytes.len() as u64;
        match self {
            Self::Synthetic(vault) => {
                let placed = vault
                    .blobs
                    .put_blob(bytes)
                    .map_err(|e| AuthorityError::Internal {
                        message: e.to_string(),
                    })?;
                Ok((placed.digest, placed.byte_length))
            }
            Self::Encrypted(vault) => {
                let digest = vault
                    .put_blob(bytes)
                    .map_err(|e| AuthorityError::Internal {
                        message: e.to_string(),
                    })?;
                Ok((digest, byte_length))
            }
        }
    }

    fn get_bytes(&self, digest: &DigestSha256) -> Result<Vec<u8>, AuthorityError> {
        match self {
            Self::Synthetic(vault) => {
                vault
                    .blobs
                    .get_blob(digest)
                    .map_err(|_| AuthorityError::Corrupt {
                        message: "snapshot bytes missing or failing verification".to_owned(),
                    })
            }
            Self::Encrypted(vault) => vault.get_blob(digest).map_err(|_| AuthorityError::Corrupt {
                message: "snapshot bytes missing or failing verification".to_owned(),
            }),
        }
    }
}

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

fn acquire_err(fail: AcquireFail) -> AuthorityError {
    match fail {
        AcquireFail::Rejected(reason) => AuthorityError::InvalidArgument { message: reason },
        AcquireFail::Quarantined(reason) => AuthorityError::Corrupt { message: reason },
        AcquireFail::Unavailable(reason) => AuthorityError::Unavailable { message: reason },
        AcquireFail::Denied(_) => AuthorityError::Unauthorized,
        AcquireFail::Unsupported(reason) => AuthorityError::UnsupportedSchema { message: reason },
        AcquireFail::Missing(_) => AuthorityError::NotFound,
    }
}

/// Authenticated authority view for one request.
pub struct DataSources<'a> {
    pub store: &'a mut InMemoryAuthorityStore,
    pub backend: SnapshotBlobBackend<'a>,
    pub allowlist: &'a [EgressAllowlistEntry],
    pub transport: &'a dyn BrokerTransport,
    pub sessions: &'a SessionRegistry,
    pub leases: &'a LeaseRegistry,
    pub vault_id: &'a VaultId,
    pub realm: RealmId,
    pub scope: AuthorityScopeId,
    pub session_id: Option<OpaqueId>,
}

impl DataSources<'_> {
    /// Resolves the request actor: session holder, else lease holder.
    pub(super) fn actor(&self) -> Result<OpaqueId, AuthorityError> {
        if let Some(holder) = self
            .session_id
            .as_ref()
            .and_then(|session_id| self.sessions.holder_of(session_id))
        {
            return Ok(holder);
        }
        self.leases
            .holder(self.vault_id)
            .ok_or(AuthorityError::LeaseRequired)
    }

    /// Appends one audit row to the existing trail.
    pub(super) fn audit(
        &mut self,
        action: &str,
        targets: Vec<OpaqueId>,
    ) -> Result<(), AuthorityError> {
        let actor = self.actor()?;
        let id = self.store.alloc_id("audit");
        let record = medscale_contracts::objects::ActionAuditRecord {
            header: ObjectHeader {
                id,
                schema_version: AUTHORITY_SCHEMA_VERSION,
                realm_id: self.realm.clone(),
                authority_scope_id: self.scope.clone(),
            },
            kind: medscale_contracts::objects::ActionAuditKind::Audit,
            actor,
            action: action.to_owned(),
            target_refs: targets,
            effect_state: None,
            payload_digest: None,
            detail: None,
        };
        self.store.insert(StoredObject::Audit(record));
        Ok(())
    }

    pub(super) fn meta(&self) -> &SqliteMetaStore {
        self.backend.meta()
    }

    fn scoped_source(&self, id: &OpaqueId) -> Result<DataSourceManifest, AuthorityError> {
        let source = self.meta().get_data_source(id).map_err(meta_err)?;
        if source.header.realm_id != self.realm || source.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(source)
    }

    pub(super) fn scoped_snapshot(&self, id: &OpaqueId) -> Result<SnapshotRecord, AuthorityError> {
        let record = self.meta().get_snapshot(id).map_err(meta_err)?;
        if record.snapshot.header.realm_id != self.realm
            || record.snapshot.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        Ok(record)
    }

    pub(super) fn require_project(&self, project_id: &OpaqueId) -> Result<(), AuthorityError> {
        let project = self.meta().get_project(project_id).map_err(meta_err)?;
        if project.header.realm_id != self.realm || project.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        if project.status != medscale_contracts::project_graph::ProjectStatus::Active {
            return Err(AuthorityError::InvalidArgument {
                message: "project is not active".to_owned(),
            });
        }
        Ok(())
    }

    fn header_for(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: DATA_SOURCE_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    /// Resolves a vault-root-relative local path. Absolute paths, parent
    /// escapes, and symlinks leaving the vault fail as `PathOutsideClaim`.
    /// The vault's own internal storage (metadata store, blob stores, writer
    /// lock) is also refused: it lives inside the same vault root as
    /// user-placed source files, so without this check a `Database` or
    /// `LocalPath` source could name it and read authority state across
    /// every scope in the vault with no scoping applied.
    fn resolve_local_path(&self, locator_path: &str) -> Result<PathBuf, AuthorityError> {
        const RESERVED_VAULT_PATHS: &[&str] = &[
            "meta.sqlite3",
            "meta.sqlite3-wal",
            "meta.sqlite3-shm",
            "meta.sqlite3-journal",
            "meta.work.sqlite3",
            "meta.work.sqlite3-wal",
            "meta.work.sqlite3-shm",
            "meta.work.sqlite3-journal",
            "meta.work.sqlite3.tmp",
            "meta.work.sqlite3.tmp-journal",
            "meta.sealed",
            "meta.sealed.tmp",
            ".session.lock",
            "writer.lock.sqlite3",
            "blobs",
            "sealed_blobs",
        ];
        let relative = Path::new(locator_path);
        if relative.is_absolute()
            || relative.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(AuthorityError::PathOutsideClaim);
        }
        let first_component = match relative.components().next() {
            Some(std::path::Component::Normal(name)) => name.to_str().unwrap_or(""),
            _ => "",
        };
        if RESERVED_VAULT_PATHS
            .iter()
            .any(|reserved| first_component.eq_ignore_ascii_case(reserved))
        {
            return Err(AuthorityError::PathOutsideClaim);
        }
        let root =
            self.backend
                .vault_root()
                .canonicalize()
                .map_err(|_| AuthorityError::Unavailable {
                    message: "vault root is not readable".to_owned(),
                })?;
        let joined = root.join(relative);
        let canonical = joined.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AuthorityError::NotFound
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                AuthorityError::Unauthorized
            } else {
                AuthorityError::Unavailable {
                    message: e.to_string(),
                }
            }
        })?;
        if !canonical.starts_with(&root) {
            return Err(AuthorityError::PathOutsideClaim);
        }
        Ok(canonical)
    }

    // ---------- source lifecycle ----------

    /// Creates one source manifest after project/validation checks.
    pub fn create_source(
        &mut self,
        project_id: OpaqueId,
        display_name: String,
        locator: SourceLocator,
        credential_ref: Option<OpaqueId>,
    ) -> Result<DataSourceManifest, AuthorityError> {
        self.require_project(&project_id)?;
        validate_display_name(&display_name)
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        locator
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let kind = locator.kind();
        if matches!(locator, SourceLocator::Database { .. }) && credential_ref.is_some() {
            return Err(AuthorityError::InvalidArgument {
                message: "database credential references are not admitted in 075".to_owned(),
            });
        }
        let format_or_engine = match &locator {
            SourceLocator::LocalPath { format, .. } => format.as_str().to_owned(),
            SourceLocator::Database { engine, .. } => engine.as_str().to_owned(),
            SourceLocator::RemoteDataset { provider, .. } => provider.as_str().to_owned(),
        };
        let capabilities = match kind {
            DataSourceKind::LocalTabularFile | DataSourceKind::RemoteDataset => vec![
                DataSourceCapability::DiscoverSchema,
                DataSourceCapability::PreviewRows,
                DataSourceCapability::ImportSnapshot,
                DataSourceCapability::RefreshSnapshot,
            ],
            DataSourceKind::DatabaseRead => vec![
                DataSourceCapability::DiscoverSchema,
                DataSourceCapability::PreviewRows,
                DataSourceCapability::ImportSnapshot,
                DataSourceCapability::RefreshSnapshot,
                DataSourceCapability::BoundedQuery,
            ],
        };
        let id = self
            .meta()
            .alloc_data_fabric_id("data_source_id_seq", "dsrc")
            .map_err(meta_err)?;
        let manifest = DataSourceManifest::new(
            self.header_for(id),
            project_id,
            kind,
            display_name,
            format_or_engine,
            locator,
            credential_ref,
            capabilities,
        )
        .map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta()
            .insert_data_source(&manifest)
            .map_err(meta_err)?;
        self.audit(
            "data-source-create",
            vec![manifest.header.id.clone(), manifest.project_id.clone()],
        )?;
        Ok(manifest)
    }

    /// Reads one source manifest.
    pub fn get_source(&self, source_id: &OpaqueId) -> Result<DataSourceManifest, AuthorityError> {
        self.scoped_source(source_id)
    }

    /// Lists source summaries for one project in this scope.
    pub fn list_sources(
        &self,
        project_id: &OpaqueId,
        limit: Option<u32>,
        cursor: &Option<String>,
    ) -> Result<(Vec<DataSourceSummary>, Option<String>), AuthorityError> {
        self.require_project(project_id)?;
        validate_cursor(cursor).map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta()
            .list_data_sources(
                &self.scope,
                project_id,
                None,
                effective_list_limit(limit),
                cursor.as_deref(),
            )
            .map_err(meta_err)
    }

    /// Renames a source or rotates its credential reference (revision-guarded).
    pub fn update_source(
        &mut self,
        source_id: &OpaqueId,
        expected_revision: u64,
        display_name: Option<String>,
        credential_ref: Option<Option<OpaqueId>>,
    ) -> Result<DataSourceManifest, AuthorityError> {
        let source = self.scoped_source(source_id)?;
        if source.status != SourceStatus::Active {
            return Err(AuthorityError::IllegalTransition);
        }
        let name = match display_name {
            Some(name) => {
                validate_display_name(&name)
                    .map_err(|message| AuthorityError::InvalidArgument { message })?;
                name
            }
            None => source.display_name.clone(),
        };
        let credential = match credential_ref {
            Some(value) => value,
            None => source.credential_ref.clone(),
        };
        if matches!(source.locator, SourceLocator::Database { .. }) && credential.is_some() {
            return Err(AuthorityError::InvalidArgument {
                message: "database credential references are not admitted in 075".to_owned(),
            });
        }
        let updated = self
            .meta()
            .update_data_source_meta(source_id, expected_revision, &name, credential.as_ref())
            .map_err(meta_err)?;
        self.audit("data-source-update", vec![source_id.clone()])?;
        Ok(updated)
    }

    /// Archives one source (terminal; snapshots/views/lineage are retained).
    pub fn archive_source(
        &mut self,
        source_id: &OpaqueId,
        expected_revision: u64,
    ) -> Result<DataSourceManifest, AuthorityError> {
        let source = self.scoped_source(source_id)?;
        if source.status != SourceStatus::Active {
            return Err(AuthorityError::IllegalTransition);
        }
        let archived = self
            .meta()
            .set_data_source_status(source_id, expected_revision, SourceStatus::Archived)
            .map_err(meta_err)?;
        self.audit("data-source-archive", vec![source_id.clone()])?;
        Ok(archived)
    }

    fn set_health(
        &mut self,
        source: &DataSourceManifest,
        health: SourceHealth,
    ) -> Result<DataSourceManifest, AuthorityError> {
        self.meta()
            .set_data_source_health(&source.header.id, source.revision, health)
            .map_err(meta_err)
    }

    // ---------- acquisition ----------

    /// Acquires and parses current source bytes without persisting.
    fn acquire_parsed(
        &self,
        source: &DataSourceManifest,
    ) -> Result<(ParsedTable, SourceRevisionBinding), AcquireFail> {
        match &source.locator {
            SourceLocator::LocalPath { path, format } => {
                let resolved = self.resolve_local_path(path).map_err(|err| match err {
                    AuthorityError::NotFound => {
                        AcquireFail::Missing("local source file is gone".to_owned())
                    }
                    AuthorityError::Unauthorized => {
                        AcquireFail::Denied("local source file is denied".to_owned())
                    }
                    AuthorityError::PathOutsideClaim => {
                        AcquireFail::Denied("local source path escapes the vault".to_owned())
                    }
                    other => AcquireFail::Unavailable(format!("{other:?}")),
                })?;
                let declared_len =
                    std::fs::metadata(&resolved)
                        .map(|m| m.len())
                        .map_err(|e| match e.kind() {
                            std::io::ErrorKind::NotFound => {
                                AcquireFail::Missing("local source file is gone".to_owned())
                            }
                            std::io::ErrorKind::PermissionDenied => {
                                AcquireFail::Denied("local source file is denied".to_owned())
                            }
                            _ => AcquireFail::Unavailable(e.to_string()),
                        })?;
                if declared_len > SNAPSHOT_BYTES_MAX {
                    return Err(AcquireFail::Rejected(
                        "input exceeds snapshot byte bound".to_owned(),
                    ));
                }
                let bytes = std::fs::read(&resolved).map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        AcquireFail::Missing("local source file is gone".to_owned())
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        AcquireFail::Denied("local source file is denied".to_owned())
                    }
                    _ => AcquireFail::Unavailable(e.to_string()),
                })?;
                let digest = DigestSha256::of(&bytes);
                let binding = SourceRevisionBinding::LocalFile {
                    digest,
                    byte_length: bytes.len() as u64,
                };
                let table = match format {
                    LocalFileFormat::Csv => parse_delimited(&bytes, b',')?,
                    LocalFileFormat::Tsv => parse_delimited(&bytes, b'\t')?,
                    LocalFileFormat::JsonLines => parse_json_bytes(&bytes, true)?,
                    LocalFileFormat::Json => parse_json_bytes(&bytes, false)?,
                    LocalFileFormat::Parquet
                    | LocalFileFormat::ArrowIpc
                    | LocalFileFormat::Xlsx => {
                        return Err(AcquireFail::Unsupported(format!(
                            "{} is not a qualified format in 075",
                            format.as_str()
                        )));
                    }
                };
                Ok((table, binding))
            }
            SourceLocator::Database {
                engine,
                database,
                object,
            } => {
                if *engine != DatabaseEngine::ExternalSqlite {
                    return Err(AcquireFail::Unsupported(format!(
                        "{} is not a qualified engine in 075",
                        engine.as_str()
                    )));
                }
                let resolved = self.resolve_local_path(database).map_err(|err| match err {
                    AuthorityError::NotFound => {
                        AcquireFail::Missing("database file is gone".to_owned())
                    }
                    AuthorityError::Unauthorized => {
                        AcquireFail::Denied("database file is denied".to_owned())
                    }
                    AuthorityError::PathOutsideClaim => {
                        AcquireFail::Denied("database path escapes the vault".to_owned())
                    }
                    other => AcquireFail::Unavailable(format!("{other:?}")),
                })?;
                let external = medscale_storage::read_external_sqlite_table(
                    &resolved,
                    object,
                    medscale_contracts::data_sources::SNAPSHOT_ROWS_MAX,
                )
                .map_err(|err| match err {
                    MetaError::NotFound => {
                        AcquireFail::Unavailable("database object is gone".to_owned())
                    }
                    MetaError::UnsupportedSchema(reason) => AcquireFail::Unsupported(reason),
                    other => AcquireFail::Unavailable(other.to_string()),
                })?;
                let table = map_external_table(&external)?;
                let query_identity = format!("sqlite:{object}");
                let binding = SourceRevisionBinding::Database {
                    query_digest: DigestSha256::of(query_identity.as_bytes()),
                    schema_fingerprint: fingerprint_schema(&table.fields),
                    observed_revision: None,
                };
                Ok((table, binding))
            }
            SourceLocator::RemoteDataset {
                provider,
                repo,
                revision,
                files,
            } => {
                if *provider != RemoteDatasetProvider::HuggingFace
                    && *provider != RemoteDatasetProvider::Kaggle
                {
                    return Err(AcquireFail::Unsupported(
                        "unknown remote provider".to_owned(),
                    ));
                }
                let fetched = brokered_dataset_fetch(
                    self.allowlist,
                    self.transport,
                    &RemoteFetchSpec {
                        provider: *provider,
                        repo: repo.clone(),
                        revision: revision.clone(),
                        files: files.clone(),
                    },
                )?;
                let table = parse_fetched_files(&fetched)?;
                let binding = SourceRevisionBinding::Remote {
                    provider: *provider,
                    repo: repo.clone(),
                    revision: revision.clone(),
                    file_digests: fetched.iter().map(|f| f.digest.clone()).collect(),
                };
                Ok((table, binding))
            }
        }
    }

    /// Persists one immutable snapshot with parts and blob bytes. The caller
    /// writes the acquisition receipt after the snapshot id is allocated.
    fn persist_snapshot(
        &self,
        source: &DataSourceManifest,
        table: &ParsedTable,
        binding: &SourceRevisionBinding,
        parent_snapshot_id: Option<OpaqueId>,
    ) -> Result<SnapshotRecord, AuthorityError> {
        let schema = SourceSchema {
            source_id: source.header.id.clone(),
            schema_fingerprint: fingerprint_schema(&table.fields),
            fields: table.fields.clone(),
        };
        schema
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let (bytes, content_digest, parts) = materialize_parts(&table.fields, &table.rows);
        let (stored_digest, _) = self.backend.put_bytes(&bytes)?;
        if stored_digest != content_digest {
            return Err(AuthorityError::Corrupt {
                message: "snapshot blob digest mismatch after write".to_owned(),
            });
        }
        // Read-back verification before the metadata commit.
        let read_back = self.backend.get_bytes(&content_digest)?;
        if read_back != bytes {
            return Err(AuthorityError::Corrupt {
                message: "snapshot blob read-back mismatch".to_owned(),
            });
        }
        let snap_id = self
            .meta()
            .alloc_data_fabric_id("snapshot_id_seq", "snap")
            .map_err(meta_err)?;
        let snapshot = medscale_contracts::data_sources::DataSnapshot {
            header: self.header_for(snap_id),
            source_id: source.header.id.clone(),
            parent_snapshot_id,
            source_revision: binding.clone(),
            schema_fingerprint: schema.schema_fingerprint.clone(),
            row_count: table.rows.len() as u64,
            content_digest: content_digest.clone(),
            status: if table.rows_skipped > 0 && table.rows.is_empty() {
                SnapshotStatus::Partial {
                    reason: "all rows skipped".to_owned(),
                }
            } else {
                SnapshotStatus::Complete
            },
        };
        snapshot
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let record = SnapshotRecord {
            snapshot,
            schema,
            project_id: source.project_id.clone(),
        };
        let part_rows: Vec<medscale_contracts::data_sources::SnapshotPart> = parts
            .into_iter()
            .enumerate()
            .map(|(index, (start, end, part_digest))| {
                medscale_contracts::data_sources::SnapshotPart {
                    snapshot_id: record.snapshot.header.id.clone(),
                    part_index: index as u32,
                    row_start: start,
                    row_end: end,
                    part_digest,
                }
            })
            .collect();
        self.meta()
            .insert_snapshot_full(&record, &part_rows)
            .map_err(meta_err)?;
        Ok(record)
    }

    // ---------- preview / import / refresh ----------

    /// Previews schema plus leading rows without persisting anything.
    pub fn preview_source(
        &self,
        source_id: &OpaqueId,
        max_rows: Option<u32>,
    ) -> Result<
        (
            SourceSchema,
            Vec<Vec<medscale_contracts::data_sources::CellValue>>,
        ),
        AuthorityError,
    > {
        let source = self.scoped_source(source_id)?;
        if source.status != SourceStatus::Active {
            return Err(AuthorityError::IllegalTransition);
        }
        let (table, _) = self.acquire_parsed(&source).map_err(acquire_err)?;
        let schema = SourceSchema {
            source_id: source.header.id.clone(),
            schema_fingerprint: fingerprint_schema(&table.fields),
            fields: table.fields.clone(),
        };
        schema
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let limit = effective_rows_limit(max_rows) as usize;
        Ok((schema, table.rows.into_iter().take(limit).collect()))
    }

    /// Imports current source bytes into an immutable snapshot. Identical
    /// bytes re-resolve to the existing snapshot with its original receipt
    /// (idempotent import, no duplicate rows).
    pub fn import_source(
        &mut self,
        source_id: &OpaqueId,
    ) -> Result<(SnapshotRecord, ImportReceipt), AuthorityError> {
        let mut source = self.scoped_source(source_id)?;
        if source.status != SourceStatus::Active {
            return Err(AuthorityError::IllegalTransition);
        }
        let (table, binding) = self.acquire_parsed(&source).map_err(|fail| {
            self.import_health_hook(&source, &fail);
            acquire_err(fail)
        })?;
        let (_, content_digest, _) = materialize_parts(&table.fields, &table.rows);
        if let Some(existing) = self
            .meta()
            .find_snapshot_by_digest(&source.header.id, &content_digest)
            .map_err(meta_err)?
        {
            let receipt_value = self
                .meta()
                .get_receipt(RECEIPT_KIND_IMPORT, &existing.snapshot.header.id)
                .map_err(meta_err)?;
            let receipt: ImportReceipt =
                serde_json::from_value(receipt_value).map_err(|_| AuthorityError::Corrupt {
                    message: "stored import receipt is invalid".to_owned(),
                })?;
            return Ok((existing, receipt));
        }
        let warnings = table.warnings.clone();
        let record = self.persist_snapshot(&source, &table, &binding, None)?;
        // The stored receipt always names the snapshot it describes.
        let receipt = ImportReceipt {
            source_id: source.header.id.clone(),
            snapshot_id: record.snapshot.header.id.clone(),
            schema_fingerprint: record.snapshot.schema_fingerprint.clone(),
            rows_materialized: record.snapshot.row_count,
            rows_skipped: table.rows_skipped,
            bytes_hashed: table.bytes_hashed,
            outcome: if table.rows.is_empty() {
                AcquireOutcome::Partial
            } else {
                AcquireOutcome::Complete
            },
            warnings,
        };
        receipt
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let receipt_value =
            serde_json::to_value(&receipt).map_err(|_| AuthorityError::Internal {
                message: "receipt encoding failed".to_owned(),
            })?;
        self.meta()
            .put_receipt(
                RECEIPT_KIND_IMPORT,
                &record.snapshot.header.id,
                &receipt_value,
            )
            .map_err(meta_err)?;
        source = self.set_health(&source, SourceHealth::Healthy)?;
        let _ = source;
        Ok((record, receipt))
    }

    /// Best-effort health hook for failed acquisitions (revision-guarded;
    /// a concurrent mutation wins over the hook, never the reverse).
    fn import_health_hook(&self, source: &DataSourceManifest, fail: &AcquireFail) {
        // Mirrors refresh_source's classification: quarantined/rejected/
        // unsupported content leaves the source reachable but its last
        // known-good state suspect, so it is marked Stale rather than left
        // silently Healthy after a failed import.
        let health = match fail {
            AcquireFail::Unavailable(_) | AcquireFail::Missing(_) => {
                Some(SourceHealth::Unavailable)
            }
            AcquireFail::Denied(_) => Some(SourceHealth::Denied),
            AcquireFail::Quarantined(_)
            | AcquireFail::Rejected(_)
            | AcquireFail::Unsupported(_) => Some(SourceHealth::Stale),
        };
        if let Some(health) = health {
            let _ = self
                .meta()
                .set_data_source_health(&source.header.id, source.revision, health);
        }
    }

    /// Refreshes a source against current external state. Unchanged sources
    /// yield an idempotent receipt with no new snapshot. Schema changes
    /// require explicit acknowledgment (`allow_schema_change`).
    pub fn refresh_source(
        &mut self,
        source_id: &OpaqueId,
        allow_schema_change: bool,
    ) -> Result<(RefreshReceipt, Option<SnapshotRecord>), AuthorityError> {
        let mut source = self.scoped_source(source_id)?;
        if source.status != SourceStatus::Active {
            return Err(AuthorityError::IllegalTransition);
        }
        let latest = self
            .meta()
            .latest_snapshot_for_source(&source.header.id)
            .map_err(meta_err)?;
        let acquired = self.acquire_parsed(&source);
        let (table, binding) = match acquired {
            Ok(value) => value,
            Err(fail) => {
                let (class, health) = match &fail {
                    AcquireFail::Missing(_) => {
                        (RefreshChangeClass::SourceGone, SourceHealth::Unavailable)
                    }
                    AcquireFail::Unavailable(_) => (
                        RefreshChangeClass::SourceUnavailable,
                        SourceHealth::Unavailable,
                    ),
                    AcquireFail::Denied(_) => {
                        (RefreshChangeClass::SourceDenied, SourceHealth::Denied)
                    }
                    _ => {
                        let _ = self.set_health(&source, SourceHealth::Stale)?;
                        return Err(acquire_err(fail));
                    }
                };
                source = self.set_health(&source, health)?;
                let receipt = RefreshReceipt {
                    source_id: source.header.id.clone(),
                    previous_snapshot_id: latest
                        .as_ref()
                        .map(|r| r.snapshot.header.id.clone())
                        .unwrap_or_else(|| OpaqueId::new("none")),
                    new_snapshot_id: None,
                    change_class: class,
                    outcome: medscale_contracts::data_sources::AcquireOutcome::Rejected,
                    warnings: Vec::new(),
                };
                let value =
                    serde_json::to_value(&receipt).map_err(|_| AuthorityError::Internal {
                        message: "receipt encoding failed".to_owned(),
                    })?;
                let key = latest
                    .as_ref()
                    .map(|r| r.snapshot.header.id.clone())
                    .unwrap_or_else(|| source.header.id.clone());
                self.meta()
                    .put_receipt(RECEIPT_KIND_REFRESH, &key, &value)
                    .map_err(meta_err)?;
                return Ok((receipt, None));
            }
        };
        let new_fingerprint = fingerprint_schema(&table.fields);
        if let Some(previous) = &latest {
            if previous.snapshot.source_revision == binding {
                let receipt = RefreshReceipt {
                    source_id: source.header.id.clone(),
                    previous_snapshot_id: previous.snapshot.header.id.clone(),
                    new_snapshot_id: None,
                    change_class: RefreshChangeClass::Unchanged,
                    outcome: AcquireOutcome::Complete,
                    warnings: Vec::new(),
                };
                let value =
                    serde_json::to_value(&receipt).map_err(|_| AuthorityError::Internal {
                        message: "receipt encoding failed".to_owned(),
                    })?;
                self.meta()
                    .put_receipt(RECEIPT_KIND_REFRESH, &previous.snapshot.header.id, &value)
                    .map_err(meta_err)?;
                source = self.set_health(&source, SourceHealth::Healthy)?;
                let _ = source;
                return Ok((receipt, None));
            }
            if previous.snapshot.schema_fingerprint != new_fingerprint && !allow_schema_change {
                source = self.set_health(&source, SourceHealth::SchemaChanged)?;
                let _ = source;
                return Err(AuthorityError::StaleReference {
                    message: "source schema changed; refresh again with explicit acknowledgment"
                        .to_owned(),
                });
            }
        }
        let change_class = match &latest {
            None => RefreshChangeClass::ContentChanged,
            Some(previous) => {
                if previous.snapshot.schema_fingerprint != new_fingerprint {
                    RefreshChangeClass::SchemaChanged
                } else {
                    RefreshChangeClass::ContentChanged
                }
            }
        };
        let receipt = RefreshReceipt {
            source_id: source.header.id.clone(),
            previous_snapshot_id: latest
                .as_ref()
                .map(|r| r.snapshot.header.id.clone())
                .unwrap_or_else(|| OpaqueId::new("none")),
            new_snapshot_id: None,
            change_class,
            outcome: AcquireOutcome::Complete,
            warnings: table.warnings.clone(),
        };
        let record = self.persist_snapshot(&source, &table, &binding, None)?;
        let mut rebound = receipt;
        rebound.new_snapshot_id = Some(record.snapshot.header.id.clone());
        let value = serde_json::to_value(&rebound).map_err(|_| AuthorityError::Internal {
            message: "receipt encoding failed".to_owned(),
        })?;
        self.meta()
            .put_receipt(RECEIPT_KIND_REFRESH, &record.snapshot.header.id, &value)
            .map_err(meta_err)?;
        source = self.set_health(&source, SourceHealth::Healthy)?;
        let _ = source;
        Ok((rebound, Some(record)))
    }

    // ---------- snapshot reads ----------

    /// Reads one snapshot record.
    pub fn get_snapshot(&self, snapshot_id: &OpaqueId) -> Result<SnapshotRecord, AuthorityError> {
        self.scoped_snapshot(snapshot_id)
    }

    /// Lists snapshot summaries for one scope-checked source.
    pub fn list_snapshots(
        &self,
        source_id: &OpaqueId,
        limit: Option<u32>,
        cursor: &Option<String>,
    ) -> Result<(Vec<SnapshotSummary>, Option<String>), AuthorityError> {
        let source = self.scoped_source(source_id)?;
        validate_cursor(cursor).map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta()
            .list_snapshots(
                &source.header.id,
                effective_list_limit(limit),
                cursor.as_deref(),
            )
            .map_err(meta_err)
    }

    /// Loads, digest-verifies, and decodes snapshot bytes.
    pub(super) fn load_table(
        &self,
        record: &SnapshotRecord,
    ) -> Result<medscale_contracts::data_sources::SnapshotCanonicalDoc, AuthorityError> {
        let bytes = self.backend.get_bytes(&record.snapshot.content_digest)?;
        let doc =
            decode_snapshot_bytes(&bytes, &record.snapshot.content_digest).map_err(|fail| {
                match fail {
                    AcquireFail::Quarantined(reason) => AuthorityError::Corrupt { message: reason },
                    other => AuthorityError::Corrupt {
                        message: other.reason().to_owned(),
                    },
                }
            })?;
        if doc.schema_fingerprint_hex != record.snapshot.schema_fingerprint.to_hex() {
            return Err(AuthorityError::Corrupt {
                message: "snapshot schema fingerprint mismatch".to_owned(),
            });
        }
        if doc.rows.len() as u64 != record.snapshot.row_count {
            return Err(AuthorityError::Corrupt {
                message: "snapshot row count mismatch".to_owned(),
            });
        }
        Ok(doc)
    }

    /// The vault directory (Spec 086: staging must stay outside it).
    pub(super) fn vault_dir(&self) -> &Path {
        self.backend.vault_root()
    }

    /// The exact stored canonical bytes of a snapshot (Spec 085 staging).
    pub(super) fn snapshot_bytes(
        &self,
        record: &SnapshotRecord,
    ) -> Result<Vec<u8>, AuthorityError> {
        self.backend.get_bytes(&record.snapshot.content_digest)
    }

    /// Queries snapshot rows with optional filters/sort and stable paging.
    pub fn snapshot_rows(
        &self,
        snapshot_id: &OpaqueId,
        limit: Option<u32>,
        cursor: &Option<String>,
        filters: &[FilterExpr],
        sort: &[SortKey],
    ) -> Result<SnapshotRowPage, AuthorityError> {
        let record = self.scoped_snapshot(snapshot_id)?;
        for filter in filters {
            if filter.value.chars().count()
                > medscale_contracts::data_sources::FILTER_VALUE_MAX_CHARS
            {
                return Err(AuthorityError::InvalidArgument {
                    message: "filter value exceeds bound".to_owned(),
                });
            }
        }
        let doc = self.load_table(&record)?;
        let order = apply_view(&doc.rows, &record.schema.fields, filters, sort);
        let offset = parse_row_cursor(cursor)
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let limit = effective_rows_limit(limit) as usize;
        let page: Vec<Vec<medscale_contracts::data_sources::CellValue>> = order
            .iter()
            .skip(offset as usize)
            .take(limit)
            .map(|index| doc.rows[*index].clone())
            .collect();
        let next_cursor = if (offset as usize) + page.len() < order.len() {
            Some(render_row_cursor(offset + page.len() as u64))
        } else {
            None
        };
        Ok(SnapshotRowPage {
            rows: page,
            next_cursor,
        })
    }

    // ---------- saved views ----------

    /// Creates one saved view after schema validation (revision 1, active).
    pub fn create_view(
        &mut self,
        snapshot_id: OpaqueId,
        view_kind: DataViewKind,
        state: ViewState,
    ) -> Result<SavedDataView, AuthorityError> {
        let record = self.scoped_snapshot(&snapshot_id)?;
        state
            .validate(&record.schema.fields)
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let id = self
            .meta()
            .alloc_data_fabric_id("view_id_seq", "view")
            .map_err(meta_err)?;
        let view = SavedDataView {
            header: self.header_for(id),
            snapshot_id: record.snapshot.header.id.clone(),
            revision: medscale_contracts::project_graph::initial_revision(),
            view_kind,
            state,
            status: SavedViewStatus::Active,
        };
        self.meta()
            .insert_saved_view(&view, &record.project_id)
            .map_err(meta_err)?;
        self.audit(
            "saved-view-create",
            vec![view.header.id.clone(), record.snapshot.header.id.clone()],
        )?;
        Ok(view)
    }

    /// Reads one saved view (scope follows its snapshot).
    pub fn get_view(&self, view_id: &OpaqueId) -> Result<SavedDataView, AuthorityError> {
        let view = self.meta().get_saved_view(view_id).map_err(meta_err)?;
        self.scoped_snapshot(&view.snapshot_id)?;
        Ok(view)
    }

    /// Lists view summaries for one scope-checked snapshot.
    pub fn list_views(
        &self,
        snapshot_id: &OpaqueId,
        limit: Option<u32>,
        cursor: &Option<String>,
    ) -> Result<(Vec<SavedViewSummary>, Option<String>), AuthorityError> {
        let record = self.scoped_snapshot(snapshot_id)?;
        validate_cursor(cursor).map_err(|message| AuthorityError::InvalidArgument { message })?;
        self.meta()
            .list_saved_views(
                &record.snapshot.header.id,
                effective_list_limit(limit),
                cursor.as_deref(),
            )
            .map_err(meta_err)
    }

    /// Updates one saved view state (revision-guarded, schema-validated).
    pub fn update_view(
        &mut self,
        view_id: &OpaqueId,
        expected_revision: u64,
        state: ViewState,
    ) -> Result<SavedDataView, AuthorityError> {
        let view = self.get_view(view_id)?;
        if view.status != SavedViewStatus::Active {
            return Err(AuthorityError::IllegalTransition);
        }
        let record = self.scoped_snapshot(&view.snapshot_id)?;
        state
            .validate(&record.schema.fields)
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let updated = self
            .meta()
            .update_saved_view_state(view_id, expected_revision, &state)
            .map_err(meta_err)?;
        self.audit("saved-view-update", vec![view_id.clone()])?;
        Ok(updated)
    }

    // ---------- transformations ----------

    /// Executes a frozen deterministic transformation. Exactly one input is
    /// admitted (bounded join is not in the frozen op set).
    pub fn execute_transformation(
        &mut self,
        input_snapshot_ids: Vec<OpaqueId>,
        ops: Vec<TransformOp>,
    ) -> Result<(SnapshotRecord, TransformationReceipt), AuthorityError> {
        let transformation = DataTransformation {
            input_snapshot_ids: input_snapshot_ids.clone(),
            ops: ops.clone(),
            parameters_digest: DigestSha256::of(
                serde_json::to_vec(&ops).unwrap_or_default().as_slice(),
            ),
        };
        transformation
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        if input_snapshot_ids.len() != 1 {
            return Err(AuthorityError::InvalidArgument {
                message: "multi-input transforms require bounded join (not admitted)".to_owned(),
            });
        }
        let input = self.scoped_snapshot(&input_snapshot_ids[0])?;
        for op in &ops {
            op.validate(&input.schema.fields)
                .map_err(|message| AuthorityError::InvalidArgument { message })?;
        }
        let doc = self.load_table(&input)?;
        let (fields, rows, cast_failures) =
            execute_transform(&input.schema.fields, &doc.rows, &ops)
                .map_err(|message| AuthorityError::InvalidArgument { message })?;
        if fields.is_empty() {
            return Err(AuthorityError::InvalidArgument {
                message: "transform produced no columns".to_owned(),
            });
        }
        let table = ParsedTable {
            fields,
            rows,
            warnings: Vec::new(),
            rows_skipped: 0,
            bytes_hashed: 0,
        };
        let source = self.scoped_source(&input.snapshot.source_id)?;
        let rows_in = vec![input.snapshot.row_count];
        let record = self.persist_snapshot(
            &source,
            &table,
            &input.snapshot.source_revision,
            Some(input.snapshot.header.id.clone()),
        )?;
        let receipt = TransformationReceipt {
            input_snapshot_ids,
            output_snapshot_id: record.snapshot.header.id.clone(),
            ops_digest: transformation.parameters_digest.clone(),
            rows_in,
            rows_out: record.snapshot.row_count,
            cast_failures,
        };
        receipt
            .validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let value = serde_json::to_value(&receipt).map_err(|_| AuthorityError::Internal {
            message: "receipt encoding failed".to_owned(),
        })?;
        self.meta()
            .put_receipt(
                RECEIPT_KIND_TRANSFORMATION,
                &record.snapshot.header.id,
                &value,
            )
            .map_err(meta_err)?;
        let stored_record = TransformationRecord {
            output_snapshot_id: record.snapshot.header.id.clone(),
            first_input_snapshot_id: receipt.input_snapshot_ids[0].clone(),
            input_ids: receipt.input_snapshot_ids.clone(),
            ops,
            parameters_digest: receipt.ops_digest.clone(),
            receipt: receipt.clone(),
        };
        self.meta()
            .insert_transformation(&stored_record)
            .map_err(meta_err)?;
        Ok((record, receipt))
    }

    // ---------- dataset releases ----------

    /// Creates an immutable versioned dataset release for one snapshot.
    pub fn create_release(
        &mut self,
        snapshot_id: OpaqueId,
        version: String,
        split_group: Option<String>,
        annotation_schema_ref: Option<String>,
        rights_state: RightsState,
    ) -> Result<ReleaseManifest, AuthorityError> {
        let record = self.scoped_snapshot(&snapshot_id)?;
        // Verify bytes before release: digest must reproduce exactly.
        let bytes = self.backend.get_bytes(&record.snapshot.content_digest)?;
        if DigestSha256::of(&bytes) != record.snapshot.content_digest {
            return Err(AuthorityError::Corrupt {
                message: "snapshot bytes fail verification at release".to_owned(),
            });
        }
        let card = DatasetCard {
            snapshot_id: record.snapshot.header.id.clone(),
            version: version.clone(),
            split_group,
            annotation_schema_ref,
            rights_state,
            project_id: record.project_id.clone(),
        };
        card.validate()
            .map_err(|message| AuthorityError::InvalidArgument { message })?;
        let (existing, _) = self
            .meta()
            .list_dataset_releases(&record.project_id, 100, None)
            .map_err(meta_err)?;
        if existing
            .iter()
            .any(|r| r.snapshot_id == record.snapshot.header.id && r.version == version)
        {
            return Err(AuthorityError::Conflict {
                message: "dataset release version already exists for snapshot".to_owned(),
            });
        }
        let id = self
            .meta()
            .alloc_data_fabric_id("release_id_seq", "rel")
            .map_err(meta_err)?;
        let release = ReleaseManifest {
            release_id: id,
            card,
            snapshot_digest: record.snapshot.content_digest.clone(),
        };
        self.meta()
            .insert_dataset_release(&release)
            .map_err(meta_err)?;
        self.audit(
            "dataset-release-create",
            vec![
                release.release_id.clone(),
                record.snapshot.header.id.clone(),
            ],
        )?;
        Ok(release)
    }

    /// Reads one dataset release (scope follows its snapshot).
    pub fn get_release(&self, release_id: &OpaqueId) -> Result<ReleaseManifest, AuthorityError> {
        let release = self
            .meta()
            .get_dataset_release(release_id)
            .map_err(meta_err)?;
        self.scoped_snapshot(&release.card.snapshot_id)?;
        Ok(release)
    }

    /// Lists release summaries for one scope-checked project.
    pub fn list_releases(
        &self,
        project_id: &OpaqueId,
        limit: Option<u32>,
        cursor: &Option<String>,
    ) -> Result<(Vec<DatasetReleaseSummary>, Option<String>), AuthorityError> {
        let project = self.meta().get_project(project_id).map_err(meta_err)?;
        if project.header.realm_id != self.realm || project.header.authority_scope_id != self.scope
        {
            return Err(AuthorityError::WrongScope);
        }
        validate_cursor(cursor).map_err(|message| AuthorityError::InvalidArgument { message })?;
        let (all, next_cursor) = self
            .meta()
            .list_dataset_releases(project_id, effective_list_limit(limit), cursor.as_deref())
            .map_err(meta_err)?;
        // Scope follows each release snapshot; cross-scope rows are filtered,
        // never leaked. The underlying page cursor is still returned as-is
        // so a caller can keep paging instead of a filtered-short page being
        // mistaken for the end of the list.
        let mut out = Vec::new();
        for summary in all {
            if let Ok(release) = self.meta().get_dataset_release(&summary.release_id)
                && self.scoped_snapshot(&release.card.snapshot_id).is_ok()
            {
                out.push(summary);
            }
        }
        Ok((out, next_cursor))
    }
}

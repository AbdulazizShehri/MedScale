//! Data Source Fabric contracts (Spec 075).
//!
//! Governed data-source/snapshot/view/transformation vocabulary. A data
//! source REFERENCES external content through a validated locator; snapshots
//! carry immutable dataset bytes with exact lineage; views are projections
//! storing parameters, never copied rows; transformations are bounded
//! deterministic ops producing new immutable snapshots.
//!
//! Reused primitives (authoritative, defined elsewhere):
//! `OpaqueId`, `ObjectHeader`, `DigestSha256`, `RealmId`, `AuthorityScopeId`,
//! `ProjectRevision`/`check_revision` (Spec 074 revision pattern, single
//! definition), existing audit/evidence/provenance contracts.
//!
//! Transport error mapping (semantic -> `crate::envelopes::AuthorityError`):
//!
//! ```text
//! Invalid          -> InvalidArgument { message }
//! NotFound         -> NotFound
//! Denied           -> Unauthorized | SessionDenied | WrongScope (most precise applies)
//! Conflict         -> Conflict { message } (stale expected_revision, duplicate)
//! Stale            -> StaleReference { message } (source moved/changed under a pinned binding)
//! Partial          -> snapshot/view status, not an error (see SnapshotStatus)
//! Missing          -> NotFound (dangling locator: file/table gone)
//! Corrupt          -> Corrupt { message }
//! Unsupported      -> UnsupportedSchema { message }
//! Unavailable      -> Unavailable { message } (source down, egress disabled, timeout)
//! Cancelled        -> Cancelled { message } (caller-requested cancellation only)
//! Internal         -> Internal { message }
//! ```
//!
//! `Denied` keeps the existing variants on purpose: no second auth taxonomy.
//! `Cancelled` is the single additive `AuthorityError` variant in 075-A.
//!
//! Durable timestamps: none. 075 follows the existing audit convention where
//! ordering and history travel through the audit trail, not through per-object
//! clocks. `MedicalTime` keeps its clinical semantics and is not reused here.
//! Source-observed time inside dataset cells is data, not repository metadata.

use serde::{Deserialize, Serialize};

use crate::objects::{AuthorityScopeId, DigestSha256, ObjectHeader, OpaqueId, RealmId};
use crate::project_graph::{ProjectRevision, check_revision, initial_revision};

/// Durable schema version for Data Source Fabric rows (storage schema v4).
pub const DATA_SOURCE_SCHEMA_VERSION: u32 = 1;

/// Maximum display-name length in Unicode scalar values.
pub const SOURCE_NAME_MAX_CHARS: usize = 128;

/// Maximum locator string length in bytes.
pub const SOURCE_LOCATOR_MAX_BYTES: usize = 1_024;

/// Maximum schema-field name length in Unicode scalar values.
pub const SCHEMA_FIELD_NAME_MAX_CHARS: usize = 128;

/// Maximum fields admitted in one discovered schema.
pub const SCHEMA_FIELDS_MAX: usize = 1_024;

/// Maximum cell string length in bytes (larger cells fail closed at acquire).
pub const CELL_TEXT_MAX_BYTES: usize = 65_536;

/// Maximum rows admitted in one snapshot materialization.
pub const SNAPSHOT_ROWS_MAX: u64 = 1_000_000;

/// Maximum canonical snapshot bytes admitted in one materialization.
pub const SNAPSHOT_BYTES_MAX: u64 = 268_435_456;

/// Rows per snapshot part (paging/windowing granularity).
pub const SNAPSHOT_PART_ROWS: u64 = 500;

/// Default page size for bounded row queries.
pub const SNAPSHOT_ROWS_LIMIT_DEFAULT: u32 = 100;

/// Maximum page size for bounded row queries.
pub const SNAPSHOT_ROWS_LIMIT_MAX: u32 = 1_000;

/// Default page size for bounded source/snapshot/view/release listings.
pub const SOURCE_LIST_LIMIT_DEFAULT: u32 = 25;

/// Maximum page size for bounded source/snapshot/view/release listings.
pub const SOURCE_LIST_LIMIT_MAX: u32 = 100;

/// Maximum opaque cursor length in bytes.
pub const SOURCE_CURSOR_MAX_BYTES: usize = 256;

/// Maximum warnings recorded on one acquisition receipt.
pub const ACQUIRE_WARNINGS_MAX: usize = 32;

/// Maximum warning text length in Unicode scalar values.
pub const ACQUIRE_WARNING_MAX_CHARS: usize = 512;

/// Maximum engine-version strings recorded in one lineage.
pub const LINEAGE_ENGINE_VERSIONS_MAX: usize = 16;

/// Maximum engine-version string length in bytes.
pub const ENGINE_VERSION_MAX_BYTES: usize = 256;

/// Maximum filter clauses admitted in one view state or row query.
pub const VIEW_FILTERS_MAX: usize = 16;

/// Maximum filter value length in Unicode scalar values.
pub const FILTER_VALUE_MAX_CHARS: usize = 512;

/// Maximum sort keys admitted in one view state or row query.
pub const VIEW_SORT_KEYS_MAX: usize = 8;

/// Maximum visible-column entries admitted in one view state.
pub const VIEW_VISIBLE_COLUMNS_MAX: usize = 256;

/// Maximum transform ops admitted in one transformation.
pub const TRANSFORM_OPS_MAX: usize = 32;

/// Maximum dataset-release version string length in Unicode scalar values.
pub const RELEASE_VERSION_MAX_CHARS: usize = 64;

/// Acquisition mechanism. Bounded vocabulary; unknown kinds fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceKind {
    LocalTabularFile,
    DatabaseRead,
    RemoteDataset,
}

impl DataSourceKind {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalTabularFile => "local_tabular_file",
            Self::DatabaseRead => "database_read",
            Self::RemoteDataset => "remote_dataset",
        }
    }

    /// Parses a closed vocabulary value; unknown kinds fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local_tabular_file" => Ok(Self::LocalTabularFile),
            "database_read" => Ok(Self::DatabaseRead),
            "remote_dataset" => Ok(Self::RemoteDataset),
            other => Err(format!("unknown data source kind {other}")),
        }
    }
}

/// Local tabular file format. The vocabulary is wider than the qualified set:
/// Core reports `Unsupported` for formats not yet qualified in this unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalFileFormat {
    Csv,
    Tsv,
    JsonLines,
    Json,
    Parquet,
    ArrowIpc,
    Xlsx,
}

impl LocalFileFormat {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::JsonLines => "json_lines",
            Self::Json => "json",
            Self::Parquet => "parquet",
            Self::ArrowIpc => "arrow_ipc",
            Self::Xlsx => "xlsx",
        }
    }

    /// Parses a closed vocabulary value; unknown formats fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "csv" => Ok(Self::Csv),
            "tsv" => Ok(Self::Tsv),
            "json_lines" => Ok(Self::JsonLines),
            "json" => Ok(Self::Json),
            "parquet" => Ok(Self::Parquet),
            "arrow_ipc" => Ok(Self::ArrowIpc),
            "xlsx" => Ok(Self::Xlsx),
            other => Err(format!("unknown local file format {other}")),
        }
    }
}

/// Read-only database engine vocabulary. Only engines qualified by the frozen
/// contract may execute; others report explicit unavailability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseEngine {
    ExternalSqlite,
    Postgres,
    Mysql,
    SqlServer,
}

impl DatabaseEngine {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExternalSqlite => "external_sqlite",
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
            Self::SqlServer => "sql_server",
        }
    }

    /// Parses a closed vocabulary value; unknown engines fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "external_sqlite" => Ok(Self::ExternalSqlite),
            "postgres" => Ok(Self::Postgres),
            "mysql" => Ok(Self::Mysql),
            "sql_server" => Ok(Self::SqlServer),
            other => Err(format!("unknown database engine {other}")),
        }
    }
}

/// Remote dataset provider vocabulary. Acquisition is brokered, read-only,
/// exact-identity bound, and never executes remote code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteDatasetProvider {
    HuggingFace,
    Kaggle,
}

impl RemoteDatasetProvider {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HuggingFace => "hugging_face",
            Self::Kaggle => "kaggle",
        }
    }

    /// Parses a closed vocabulary value; unknown providers fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "hugging_face" => Ok(Self::HuggingFace),
            "kaggle" => Ok(Self::Kaggle),
            other => Err(format!("unknown remote dataset provider {other}")),
        }
    }
}

/// Operations a source instance may perform. Declared per instance from kind
/// and qualification state, never inferred from display strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceCapability {
    DiscoverSchema,
    PreviewRows,
    ImportSnapshot,
    RefreshSnapshot,
    BoundedQuery,
}

impl DataSourceCapability {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DiscoverSchema => "discover_schema",
            Self::PreviewRows => "preview_rows",
            Self::ImportSnapshot => "import_snapshot",
            Self::RefreshSnapshot => "refresh_snapshot",
            Self::BoundedQuery => "bounded_query",
        }
    }
}

/// Observed source health. Recomputed on access/refresh; transitions never
/// rewrite existing snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceHealth {
    Healthy,
    Stale,
    Unavailable,
    Denied,
    SchemaChanged,
    CredentialRevoked,
}

impl SourceHealth {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Stale => "stale",
            Self::Unavailable => "unavailable",
            Self::Denied => "denied",
            Self::SchemaChanged => "schema_changed",
            Self::CredentialRevoked => "credential_revoked",
        }
    }
}

/// Lifecycle of a source manifest. Archive is terminal in 075.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Active,
    Archived,
}

impl SourceStatus {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    /// Returns true for the transitions admitted in 075 (archive terminal).
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!((self, next), (Self::Active, Self::Archived))
    }
}

/// Typed source locator. A validated scoped reference, never an ambient path
/// handle: Core validates the locator against the admitted boundary before
/// any acquisition. Credential material never appears here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "locator", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceLocator {
    LocalPath {
        path: String,
        format: LocalFileFormat,
    },
    Database {
        engine: DatabaseEngine,
        database: String,
        object: String,
    },
    RemoteDataset {
        provider: RemoteDatasetProvider,
        repo: String,
        revision: String,
        files: Vec<String>,
    },
}

impl SourceLocator {
    /// Validates bounds and rejects NUL/empty components. Admitted-root
    /// enforcement happens in Core at acquisition time, not here.
    pub fn validate(&self) -> Result<(), String> {
        fn bounded(value: &str, what: &str) -> Result<(), String> {
            if value.is_empty() {
                return Err(format!("{what} must not be empty"));
            }
            if value.len() > SOURCE_LOCATOR_MAX_BYTES {
                return Err(format!("{what} exceeds locator bound"));
            }
            if value.contains('\0') {
                return Err(format!("{what} must not contain NUL"));
            }
            Ok(())
        }
        // Remote identifiers are never filesystem paths at this layer, but
        // fetched files are later staged to local paths derived from them;
        // reject traversal/absolute shapes here so that staging step never
        // has to trust an unvalidated segment.
        fn bounded_no_traversal(value: &str, what: &str) -> Result<(), String> {
            bounded(value, what)?;
            let path = std::path::Path::new(value);
            // has_root() (not is_absolute()) also catches a rootless
            // "/etc/passwd"-style path on Windows, which is_absolute()
            // alone would miss there.
            if path.has_root()
                || path.components().any(|c| {
                    matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::Prefix(_)
                    )
                })
            {
                return Err(format!("{what} must not escape its dataset scope"));
            }
            Ok(())
        }
        match self {
            Self::LocalPath { path, .. } => bounded(path, "local path"),
            Self::Database {
                database, object, ..
            } => {
                bounded(database, "database name")?;
                bounded(object, "database object")
            }
            Self::RemoteDataset {
                repo,
                revision,
                files,
                ..
            } => {
                bounded(repo, "dataset repo")?;
                bounded(revision, "dataset revision")?;
                if files.is_empty() || files.len() > SOURCE_LIST_LIMIT_MAX as usize {
                    return Err("dataset files must be non-empty and bounded".to_owned());
                }
                for file in files {
                    bounded_no_traversal(file, "dataset file")?;
                }
                Ok(())
            }
        }
    }

    /// Returns the kind this locator requires.
    #[must_use]
    pub const fn kind(&self) -> DataSourceKind {
        match self {
            Self::LocalPath { .. } => DataSourceKind::LocalTabularFile,
            Self::Database { .. } => DataSourceKind::DatabaseRead,
            Self::RemoteDataset { .. } => DataSourceKind::RemoteDataset,
        }
    }
}

/// Declared cell type. Unknown/ambiguous source types map per frozen policy,
/// never by silent coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Integer,
    Float,
    Boolean,
    Date,
    Time,
    DateTime,
    Binary,
}

impl FieldType {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Time => "time",
            Self::DateTime => "date_time",
            Self::Binary => "binary",
        }
    }

    /// Parses a closed vocabulary value; unknown types fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "integer" => Ok(Self::Integer),
            "float" => Ok(Self::Float),
            "boolean" => Ok(Self::Boolean),
            "date" => Ok(Self::Date),
            "time" => Ok(Self::Time),
            "date_time" => Ok(Self::DateTime),
            "binary" => Ok(Self::Binary),
            other => Err(format!("unknown field type {other}")),
        }
    }
}

/// One discovered schema field. Units are preserved verbatim, never assumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaField {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub declared_unit: Option<String>,
}

impl SchemaField {
    /// Validates name bounds and unit bounds.
    pub fn validate(&self) -> Result<(), String> {
        let chars = self.name.chars().count();
        if self.name.trim().is_empty() || chars > SCHEMA_FIELD_NAME_MAX_CHARS {
            return Err("schema field name is empty or exceeds bound".to_owned());
        }
        if self.name.contains('\0') {
            return Err("schema field name must not contain NUL".to_owned());
        }
        if let Some(unit) = &self.declared_unit
            && (unit.chars().count() > SCHEMA_FIELD_NAME_MAX_CHARS || unit.contains('\0'))
        {
            return Err("declared unit exceeds bound".to_owned());
        }
        Ok(())
    }

    /// Canonical descriptor line feeding the schema fingerprint.
    #[must_use]
    pub fn canonical_descriptor(&self) -> String {
        format!(
            "{}\0{}\0{}\0{}",
            self.name,
            self.field_type.as_str(),
            if self.nullable {
                "nullable"
            } else {
                "required"
            },
            self.declared_unit.as_deref().unwrap_or("")
        )
    }
}

/// Discovered source schema with a deterministic fingerprint over canonical
/// field descriptors. Same source revision yields the same fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSchema {
    pub source_id: OpaqueId,
    pub schema_fingerprint: DigestSha256,
    pub fields: Vec<SchemaField>,
}

impl SourceSchema {
    /// Validates field bounds, duplicate names, and fingerprint integrity.
    pub fn validate(&self) -> Result<(), String> {
        if self.fields.is_empty() || self.fields.len() > SCHEMA_FIELDS_MAX {
            return Err("schema fields must be non-empty and bounded".to_owned());
        }
        let mut seen = std::collections::BTreeSet::new();
        for field in &self.fields {
            field.validate()?;
            if !seen.insert(field.name.clone()) {
                return Err(format!("duplicate schema field {}", field.name));
            }
        }
        let expected = fingerprint_schema(&self.fields);
        if expected != self.schema_fingerprint {
            return Err("schema fingerprint does not match fields".to_owned());
        }
        Ok(())
    }
}

/// Computes the deterministic schema fingerprint over canonical descriptors.
#[must_use]
pub fn fingerprint_schema(fields: &[SchemaField]) -> DigestSha256 {
    let mut bytes = Vec::new();
    for field in fields {
        bytes.extend_from_slice(field.canonical_descriptor().as_bytes());
        bytes.push(0);
    }
    DigestSha256::of(&bytes)
}

/// Strongest exact-source identity the adapter actually supports. A binding
/// cannot claim immutability the source does not provide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "binding", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceRevisionBinding {
    LocalFile {
        digest: DigestSha256,
        byte_length: u64,
    },
    Database {
        query_digest: DigestSha256,
        schema_fingerprint: DigestSha256,
        observed_revision: Option<String>,
    },
    Remote {
        provider: RemoteDatasetProvider,
        repo: String,
        revision: String,
        file_digests: Vec<DigestSha256>,
    },
}

impl SourceRevisionBinding {
    /// Validates bounds on revision metadata.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::LocalFile { .. } => Ok(()),
            Self::Database {
                observed_revision, ..
            } => {
                if let Some(rev) = observed_revision
                    && (rev.is_empty()
                        || rev.len() > SOURCE_LOCATOR_MAX_BYTES
                        || rev.contains('\0'))
                {
                    return Err("observed revision exceeds bound".to_owned());
                }
                Ok(())
            }
            Self::Remote {
                repo,
                revision,
                file_digests,
                ..
            } => {
                if repo.is_empty() || revision.is_empty() || file_digests.is_empty() {
                    return Err("remote binding must carry repo, revision and digests".to_owned());
                }
                Ok(())
            }
        }
    }
}

/// Typed dataset cell. Missing values are `Null`, never zero/empty coercion.
/// `Eq` is deliberately absent (`Float` has no total equality); use explicit
/// comparison helpers in query paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum CellValue {
    Null,
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl CellValue {
    /// Validates cell bounds (text length).
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Text(text) = self {
            if text.len() > CELL_TEXT_MAX_BYTES {
                return Err("cell text exceeds bound".to_owned());
            }
            if text.contains('\0') {
                return Err("cell text must not contain NUL".to_owned());
            }
        }
        Ok(())
    }

    /// Returns true when the cell carries no value.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// Canonical snapshot document. Serialized with struct field order, which is
/// deterministic, so `content_digest` is stable across processes and reopen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotCanonicalDoc {
    pub schema_fingerprint_hex: String,
    pub fields: Vec<SchemaField>,
    pub rows: Vec<Vec<CellValue>>,
}

/// Encodes the canonical snapshot bytes verified by `content_digest`.
#[must_use]
pub fn canonical_snapshot_bytes(fields: &[SchemaField], rows: &[Vec<CellValue>]) -> Vec<u8> {
    let doc = SnapshotCanonicalDoc {
        schema_fingerprint_hex: fingerprint_schema(fields).to_hex(),
        fields: fields.to_vec(),
        rows: rows.to_vec(),
    };
    serde_json::to_vec(&doc).unwrap_or_default()
}

/// Canonical per-part encoding feeding `SnapshotPart.part_digest`.
#[must_use]
pub fn canonical_part_bytes(rows: &[Vec<CellValue>]) -> Vec<u8> {
    serde_json::to_vec(rows).unwrap_or_default()
}

/// Snapshot completeness. Partial snapshots record why the remainder is
/// absent; partial data is never presented as complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SnapshotStatus {
    Complete,
    Partial { reason: String },
}

impl SnapshotStatus {
    /// Validates the partial reason bound.
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Partial { reason } = self
            && (reason.trim().is_empty() || reason.chars().count() > ACQUIRE_WARNING_MAX_CHARS)
        {
            return Err("partial reason is empty or exceeds bound".to_owned());
        }
        Ok(())
    }
}

/// Immutable dataset snapshot. No update API exists: a changed source yields
/// a new snapshot, never a mutated old one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataSnapshot {
    pub header: ObjectHeader,
    pub source_id: OpaqueId,
    pub parent_snapshot_id: Option<OpaqueId>,
    pub source_revision: SourceRevisionBinding,
    pub schema_fingerprint: DigestSha256,
    pub row_count: u64,
    pub content_digest: DigestSha256,
    pub status: SnapshotStatus,
}

impl DataSnapshot {
    /// Validates bindings, row-count plausibility, and status.
    pub fn validate(&self) -> Result<(), String> {
        self.source_revision.validate()?;
        self.status.validate()?;
        if self.row_count > SNAPSHOT_ROWS_MAX {
            return Err("row count exceeds snapshot bound".to_owned());
        }
        Ok(())
    }
}

/// One paging/window partition of a snapshot. Parts are an internal
/// storage/windowing mechanism; queries always resolve through the owning
/// snapshot identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotPart {
    pub snapshot_id: OpaqueId,
    pub part_index: u32,
    pub row_start: u64,
    pub row_end: u64,
    pub part_digest: DigestSha256,
}

impl SnapshotPart {
    /// Validates a non-empty ordered row range.
    pub fn validate(&self) -> Result<(), String> {
        if self.row_start >= self.row_end || self.row_end > SNAPSHOT_ROWS_MAX {
            return Err("snapshot part row range is invalid".to_owned());
        }
        Ok(())
    }
}

/// Acquisition outcome. Quarantined/rejected acquisitions never materialize
/// snapshot rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquireOutcome {
    Complete,
    Partial,
    Quarantined,
    Rejected,
}

impl AcquireOutcome {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Quarantined => "quarantined",
            Self::Rejected => "rejected",
        }
    }
}

/// Refresh change classification. Unchanged refreshes create a receipt but no
/// new snapshot (idempotent refresh).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshChangeClass {
    Unchanged,
    ContentChanged,
    SchemaChanged,
    SourceUnavailable,
    SourceDenied,
    SourceGone,
}

impl RefreshChangeClass {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::ContentChanged => "content_changed",
            Self::SchemaChanged => "schema_changed",
            Self::SourceUnavailable => "source_unavailable",
            Self::SourceDenied => "source_denied",
            Self::SourceGone => "source_gone",
        }
    }
}

/// Immutable import receipt. Skipped/repaired rows are counted, never silent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReceipt {
    pub source_id: OpaqueId,
    pub snapshot_id: OpaqueId,
    pub schema_fingerprint: DigestSha256,
    pub rows_materialized: u64,
    pub rows_skipped: u64,
    pub bytes_hashed: u64,
    pub outcome: AcquireOutcome,
    pub warnings: Vec<String>,
}

impl ImportReceipt {
    /// Validates warning bounds and row plausibility.
    pub fn validate(&self) -> Result<(), String> {
        validate_warnings(&self.warnings)?;
        if self.rows_materialized > SNAPSHOT_ROWS_MAX {
            return Err("materialized rows exceed snapshot bound".to_owned());
        }
        Ok(())
    }
}

/// Immutable refresh receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshReceipt {
    pub source_id: OpaqueId,
    pub previous_snapshot_id: OpaqueId,
    pub new_snapshot_id: Option<OpaqueId>,
    pub change_class: RefreshChangeClass,
    pub outcome: AcquireOutcome,
    pub warnings: Vec<String>,
}

impl RefreshReceipt {
    /// Validates warning bounds.
    pub fn validate(&self) -> Result<(), String> {
        validate_warnings(&self.warnings)
    }
}

/// Validates bounded warning lists.
fn validate_warnings(warnings: &[String]) -> Result<(), String> {
    if warnings.len() > ACQUIRE_WARNINGS_MAX {
        return Err("acquisition warnings exceed bound".to_owned());
    }
    for warning in warnings {
        if warning.trim().is_empty() || warning.chars().count() > ACQUIRE_WARNING_MAX_CHARS {
            return Err("acquisition warning is empty or exceeds bound".to_owned());
        }
    }
    Ok(())
}

/// Bounded deterministic transformation operation. Widening this set requires
/// a contract amendment; Core rejects unknown ops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransformOp {
    SelectColumns {
        columns: Vec<String>,
    },
    DropColumns {
        columns: Vec<String>,
    },
    RenameColumn {
        from: String,
        to: String,
    },
    CastType {
        column: String,
        to: FieldType,
        strict: bool,
    },
    FilterRows {
        filters: Vec<FilterExpr>,
    },
    SortRows {
        keys: Vec<SortKey>,
    },
}

impl TransformOp {
    /// Validates op bounds against the snapshot schema field names.
    pub fn validate(&self, schema: &[SchemaField]) -> Result<(), String> {
        fn known(schema: &[SchemaField], column: &str) -> Result<(), String> {
            if schema.iter().any(|f| f.name == column) {
                Ok(())
            } else {
                Err(format!("transform references unknown column {column}"))
            }
        }
        fn bounded_name(value: &str, what: &str) -> Result<(), String> {
            if value.trim().is_empty()
                || value.chars().count() > SCHEMA_FIELD_NAME_MAX_CHARS
                || value.contains('\0')
            {
                return Err(format!("{what} is empty or exceeds bound"));
            }
            Ok(())
        }
        match self {
            Self::SelectColumns { columns } | Self::DropColumns { columns } => {
                if columns.is_empty() || columns.len() > VIEW_VISIBLE_COLUMNS_MAX {
                    return Err("transform column list is empty or exceeds bound".to_owned());
                }
                for column in columns {
                    bounded_name(column, "column")?;
                    known(schema, column)?;
                }
                Ok(())
            }
            Self::RenameColumn { from, to } => {
                bounded_name(from, "rename source")?;
                bounded_name(to, "rename target")?;
                known(schema, from)?;
                if schema.iter().any(|f| f.name == *to) {
                    return Err("rename target already exists".to_owned());
                }
                Ok(())
            }
            Self::CastType { column, .. } => {
                bounded_name(column, "cast column")?;
                known(schema, column)
            }
            Self::FilterRows { filters } => {
                validate_filters(filters, schema)?;
                Ok(())
            }
            Self::SortRows { keys } => {
                validate_sort_keys(keys, schema)?;
                Ok(())
            }
        }
    }
}

/// One deterministic row filter clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterExpr {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

/// Closed filter-operator vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
}

impl FilterOp {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equals => "equals",
            Self::NotEquals => "not_equals",
            Self::Contains => "contains",
            Self::GreaterThan => "greater_than",
            Self::LessThan => "less_than",
        }
    }

    /// Parses a closed vocabulary value; unknown ops fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "equals" => Ok(Self::Equals),
            "not_equals" => Ok(Self::NotEquals),
            "contains" => Ok(Self::Contains),
            "greater_than" => Ok(Self::GreaterThan),
            "less_than" => Ok(Self::LessThan),
            other => Err(format!("unknown filter op {other}")),
        }
    }
}

/// One deterministic sort key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SortKey {
    pub column: String,
    pub descending: bool,
}

/// Validates filter clauses against a schema.
fn validate_filters(filters: &[FilterExpr], schema: &[SchemaField]) -> Result<(), String> {
    if filters.is_empty() || filters.len() > VIEW_FILTERS_MAX {
        return Err("filters must be non-empty and bounded".to_owned());
    }
    for filter in filters {
        if !schema.iter().any(|f| f.name == filter.column) {
            return Err(format!(
                "filter references unknown column {}",
                filter.column
            ));
        }
        if filter.value.chars().count() > FILTER_VALUE_MAX_CHARS || filter.value.contains('\0') {
            return Err("filter value exceeds bound".to_owned());
        }
    }
    Ok(())
}

/// Validates sort keys against a schema.
fn validate_sort_keys(keys: &[SortKey], schema: &[SchemaField]) -> Result<(), String> {
    if keys.is_empty() || keys.len() > VIEW_SORT_KEYS_MAX {
        return Err("sort keys must be non-empty and bounded".to_owned());
    }
    for key in keys {
        if !schema.iter().any(|f| f.name == key.column) {
            return Err(format!("sort references unknown column {}", key.column));
        }
    }
    Ok(())
}

/// A frozen deterministic transformation definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataTransformation {
    pub input_snapshot_ids: Vec<OpaqueId>,
    pub ops: Vec<TransformOp>,
    pub parameters_digest: DigestSha256,
}

impl DataTransformation {
    /// Validates op bounds; per-op schema validation happens in Core against
    /// the resolved input snapshot schema.
    pub fn validate(&self) -> Result<(), String> {
        if self.input_snapshot_ids.is_empty() || self.input_snapshot_ids.len() > 2 {
            return Err("transformation inputs must be one or two snapshots".to_owned());
        }
        if self.ops.is_empty() || self.ops.len() > TRANSFORM_OPS_MAX {
            return Err("transformation ops must be non-empty and bounded".to_owned());
        }
        Ok(())
    }
}

/// Immutable transformation receipt with exact replay lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationReceipt {
    pub input_snapshot_ids: Vec<OpaqueId>,
    pub output_snapshot_id: OpaqueId,
    pub ops_digest: DigestSha256,
    pub rows_in: Vec<u64>,
    pub rows_out: u64,
    pub cast_failures: u64,
}

impl TransformationReceipt {
    /// Validates row-count plausibility.
    pub fn validate(&self) -> Result<(), String> {
        if self.rows_out > SNAPSHOT_ROWS_MAX {
            return Err("transformation output rows exceed bound".to_owned());
        }
        Ok(())
    }
}

/// Lineage binding every snapshot to its exact acquisition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "acquisition", rename_all = "snake_case", deny_unknown_fields)]
pub enum SnapshotLineage {
    Import { receipt: ImportReceipt },
    Refresh { receipt: RefreshReceipt },
    Transformation { receipt: TransformationReceipt },
}

/// Governed data-source manifest. Carries identity, locator, and health;
/// never credential plaintext, file bytes, or row payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataSourceManifest {
    pub header: ObjectHeader,
    pub revision: ProjectRevision,
    pub project_id: OpaqueId,
    pub kind: DataSourceKind,
    pub display_name: String,
    pub format_or_engine: String,
    pub locator: SourceLocator,
    pub credential_ref: Option<OpaqueId>,
    pub capabilities: Vec<DataSourceCapability>,
    pub health: SourceHealth,
    pub status: SourceStatus,
}

impl DataSourceManifest {
    /// Creates a revision-1 active manifest after validating metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        header: ObjectHeader,
        project_id: OpaqueId,
        kind: DataSourceKind,
        display_name: String,
        format_or_engine: String,
        locator: SourceLocator,
        credential_ref: Option<OpaqueId>,
        capabilities: Vec<DataSourceCapability>,
    ) -> Result<Self, String> {
        validate_display_name(&display_name)?;
        validate_format_or_engine(&format_or_engine)?;
        locator.validate()?;
        if locator.kind() != kind {
            return Err("locator kind does not match source kind".to_owned());
        }
        if capabilities.is_empty() || capabilities.len() > 8 {
            return Err("capabilities must be non-empty and bounded".to_owned());
        }
        Ok(Self {
            header,
            revision: initial_revision(),
            project_id,
            kind,
            display_name,
            format_or_engine,
            locator,
            credential_ref,
            capabilities,
            health: SourceHealth::Healthy,
            status: SourceStatus::Active,
        })
    }

    /// Applies a revision-guarded mutation. Archive is terminal.
    pub fn check_mutation(
        &self,
        expected_revision: ProjectRevision,
    ) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected_revision)
    }

    /// Returns true when the manifest scope matches the admitted scope.
    #[must_use]
    pub fn scope_matches(&self, scope: &AuthorityScopeId) -> bool {
        self.header.authority_scope_id == *scope
    }
}

/// Validates a source display name.
pub fn validate_display_name(value: &str) -> Result<(), String> {
    let chars = value.chars().count();
    if value.trim().is_empty() || chars > SOURCE_NAME_MAX_CHARS {
        return Err("display name is empty or exceeds bound".to_owned());
    }
    if value.contains('\0') {
        return Err("display name must not contain NUL".to_owned());
    }
    Ok(())
}

/// Validates the format/engine identity string.
fn validate_format_or_engine(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > SOURCE_LOCATOR_MAX_BYTES || value.contains('\0') {
        return Err("format/engine identity is empty or exceeds bound".to_owned());
    }
    Ok(())
}

/// Alternate view kind. All kinds are projections over the same snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataViewKind {
    Grid,
    Form,
    Gallery,
    Kanban,
    Calendar,
    Summary,
}

impl DataViewKind {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Grid => "grid",
            Self::Form => "form",
            Self::Gallery => "gallery",
            Self::Kanban => "kanban",
            Self::Calendar => "calendar",
            Self::Summary => "summary",
        }
    }

    /// Parses a closed vocabulary value; unknown kinds fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "grid" => Ok(Self::Grid),
            "form" => Ok(Self::Form),
            "gallery" => Ok(Self::Gallery),
            "kanban" => Ok(Self::Kanban),
            "calendar" => Ok(Self::Calendar),
            "summary" => Ok(Self::Summary),
            other => Err(format!("unknown view kind {other}")),
        }
    }
}

/// Frozen view presentation parameters. Stores parameters only, never rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewState {
    pub sort: Vec<SortKey>,
    pub filters: Vec<FilterExpr>,
    pub group_by: Option<String>,
    pub visible_columns: Option<Vec<String>>,
    pub page_size: u32,
}

impl ViewState {
    /// Validates view parameters against a snapshot schema.
    pub fn validate(&self, schema: &[SchemaField]) -> Result<(), String> {
        if !self.sort.is_empty() {
            validate_sort_keys(&self.sort, schema)?;
        }
        if !self.filters.is_empty() {
            validate_filters(&self.filters, schema)?;
        }
        if let Some(group) = &self.group_by
            && !schema.iter().any(|f| f.name == *group)
        {
            return Err("view groups by an unknown column".to_owned());
        }
        if let Some(columns) = &self.visible_columns {
            if columns.is_empty() || columns.len() > VIEW_VISIBLE_COLUMNS_MAX {
                return Err("visible columns are empty or exceed bound".to_owned());
            }
            for column in columns {
                if !schema.iter().any(|f| f.name == *column) {
                    return Err(format!("view shows unknown column {column}"));
                }
            }
        }
        if self.page_size == 0 || self.page_size > SNAPSHOT_ROWS_LIMIT_MAX {
            return Err("view page size is out of bound".to_owned());
        }
        Ok(())
    }
}

/// View lifecycle. Archive is terminal in 075.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedViewStatus {
    Active,
    Archived,
}

impl SavedViewStatus {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

/// Saved data view: presentation parameters bound to one snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedDataView {
    pub header: ObjectHeader,
    pub snapshot_id: OpaqueId,
    pub revision: ProjectRevision,
    pub view_kind: DataViewKind,
    pub state: ViewState,
    pub status: SavedViewStatus,
}

impl SavedDataView {
    /// Applies a revision-guarded mutation.
    pub fn check_mutation(
        &self,
        expected_revision: ProjectRevision,
    ) -> Result<ProjectRevision, String> {
        check_revision(self.revision, expected_revision)
    }
}

/// Rights state carried by a dataset release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightsState {
    SyntheticFixture,
    PermittedNonPhi,
    RedistributionLimited,
}

impl RightsState {
    /// Returns the canonical snake_case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SyntheticFixture => "synthetic_fixture",
            Self::PermittedNonPhi => "permitted_non_phi",
            Self::RedistributionLimited => "redistribution_limited",
        }
    }

    /// Parses a closed vocabulary value; unknown states fail closed.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "synthetic_fixture" => Ok(Self::SyntheticFixture),
            "permitted_non_phi" => Ok(Self::PermittedNonPhi),
            "redistribution_limited" => Ok(Self::RedistributionLimited),
            other => Err(format!("unknown rights state {other}")),
        }
    }
}

/// Versioned dataset card bound to one immutable snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetCard {
    pub snapshot_id: OpaqueId,
    pub version: String,
    pub split_group: Option<String>,
    pub annotation_schema_ref: Option<String>,
    pub rights_state: RightsState,
    pub project_id: OpaqueId,
}

impl DatasetCard {
    /// Validates version/split/schema-ref bounds.
    pub fn validate(&self) -> Result<(), String> {
        if self.version.trim().is_empty()
            || self.version.chars().count() > RELEASE_VERSION_MAX_CHARS
            || self.version.contains('\0')
        {
            return Err("release version is empty or exceeds bound".to_owned());
        }
        for value in [&self.split_group, &self.annotation_schema_ref]
            .into_iter()
            .flatten()
        {
            if value.trim().is_empty()
                || value.chars().count() > RELEASE_VERSION_MAX_CHARS
                || value.contains('\0')
            {
                return Err("release metadata field is empty or exceeds bound".to_owned());
            }
        }
        Ok(())
    }
}

/// Immutable release manifest. Releases are never edited; a change requires
/// a new version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub release_id: OpaqueId,
    pub card: DatasetCard,
    pub snapshot_digest: DigestSha256,
}

/// Lightweight source summary for listings (no locator/credential detail).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataSourceSummary {
    pub source_id: OpaqueId,
    pub project_id: OpaqueId,
    pub kind: DataSourceKind,
    pub display_name: String,
    pub format_or_engine: String,
    pub health: SourceHealth,
    pub status: SourceStatus,
    pub revision: ProjectRevision,
}

/// Lightweight snapshot summary for listings (no row bytes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotSummary {
    pub snapshot_id: OpaqueId,
    pub source_id: OpaqueId,
    pub row_count: u64,
    pub content_digest: DigestSha256,
    pub status: SnapshotStatus,
}

/// Lightweight view summary for listings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedViewSummary {
    pub view_id: OpaqueId,
    pub snapshot_id: OpaqueId,
    pub view_kind: DataViewKind,
    pub status: SavedViewStatus,
    pub revision: ProjectRevision,
}

/// Lightweight release summary for listings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetReleaseSummary {
    pub release_id: OpaqueId,
    pub snapshot_id: OpaqueId,
    pub version: String,
    pub rights_state: RightsState,
}

/// Bounded row page with an opaque cursor carrying the last row offset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRowPage {
    pub rows: Vec<Vec<CellValue>>,
    pub next_cursor: Option<String>,
}

/// Builds the effective row-page limit from an optional request limit.
#[must_use]
pub fn effective_rows_limit(limit: Option<u32>) -> u32 {
    let wanted = limit.unwrap_or(SNAPSHOT_ROWS_LIMIT_DEFAULT);
    wanted.clamp(1, SNAPSHOT_ROWS_LIMIT_MAX)
}

/// Builds the effective listing limit from an optional request limit.
#[must_use]
pub fn effective_list_limit(limit: Option<u32>) -> u32 {
    let wanted = limit.unwrap_or(SOURCE_LIST_LIMIT_DEFAULT);
    wanted.clamp(1, SOURCE_LIST_LIMIT_MAX)
}

/// Validates an opaque cursor bound.
pub fn validate_cursor(cursor: &Option<String>) -> Result<(), String> {
    if let Some(value) = cursor
        && value.len() > SOURCE_CURSOR_MAX_BYTES
    {
        return Err("cursor exceeds bound".to_owned());
    }
    Ok(())
}

/// Parses a row-offset cursor produced by Core (`after:<n>`).
pub fn parse_row_cursor(cursor: &Option<String>) -> Result<u64, String> {
    match cursor {
        None => Ok(0),
        Some(value) => {
            validate_cursor(&Some(value.clone()))?;
            value
                .strip_prefix("after:")
                .and_then(|n| n.parse::<u64>().ok())
                .ok_or_else(|| "cursor is not a valid row offset".to_owned())
        }
    }
}

/// Renders a row-offset cursor.
#[must_use]
pub fn render_row_cursor(offset: u64) -> String {
    format!("after:{offset}")
}

/// Realm/scope guard shared by Core and storage: the manifest scope must
/// match the admitted scope.
#[must_use]
pub fn manifest_scope_matches(manifest: &DataSourceManifest, scope: &AuthorityScopeId) -> bool {
    manifest.scope_matches(scope)
}

/// Realm marker re-export for authority modules.
#[must_use]
pub fn realm_of(header: &ObjectHeader) -> &RealmId {
    &header.realm_id
}

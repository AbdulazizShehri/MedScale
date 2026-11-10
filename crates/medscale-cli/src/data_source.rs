//! Spec 075 data source fabric commands (CLI vertical slice through Core).
//!
//! Every command opens the session scope, dispatches one typed Core request,
//! and renders the typed result as human lines or stable JSON. The CLI never
//! touches data-source storage, drivers, or transports directly. Authority
//! errors keep typed codes in both output modes.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use medscale_contracts::data_sources::{
    DataSourceManifest, DataViewKind, DatabaseEngine, FilterExpr, FilterOp, LocalFileFormat,
    RemoteDatasetProvider, RightsState, SortKey, SourceLocator, TransformOp, ViewState,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;

use super::{fail_json, print_json_or_debug};

/// Maps an authority error to a stable typed CLI failure.
fn data_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => ("denied", debug),
        AuthorityError::BrokerDenied { .. } | AuthorityError::PackDenied { .. } => {
            ("denied", debug)
        }
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::StaleReference { message } => ("stale_reference", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::LexicalReject { reason } => ("invalid", reason.clone()),
        AuthorityError::IllegalTransition => ("invalid", debug),
        AuthorityError::VersionReject { got } => (
            "invalid",
            format!(
                "unsupported version: {}",
                got.as_deref().unwrap_or("unknown")
            ),
        ),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::DigestMismatch => ("corrupt", debug),
        AuthorityError::UnsupportedSchema { message } => ("unsupported_schema", message.clone()),
        AuthorityError::Unavailable { message } => ("unavailable", message.clone()),
        AuthorityError::Cancelled { message } => ("cancelled", message.clone()),
        AuthorityError::LeaseRequired
        | AuthorityError::VaultRequired
        | AuthorityError::LeaseHeld { .. }
        | AuthorityError::AlreadyHeld { .. }
        | AuthorityError::NotHolder
        | AuthorityError::NotHeld
        | AuthorityError::MissingKeyMaterial
        | AuthorityError::ExternalGateRequired { .. }
        | AuthorityError::UnknownRequiresReconcile => ("unavailable", debug),
        AuthorityError::PathOutsideClaim | AuthorityError::Internal { .. } => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn open_data_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| data_fail(&err, json))
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

fn parse_format(value: &str) -> Result<LocalFileFormat, String> {
    LocalFileFormat::parse(value)
}

fn parse_engine(value: &str) -> Result<DatabaseEngine, String> {
    DatabaseEngine::parse(value)
}

fn parse_provider(value: &str) -> Result<RemoteDatasetProvider, String> {
    RemoteDatasetProvider::parse(value)
}

fn parse_view_kind(value: &str) -> Result<DataViewKind, String> {
    DataViewKind::parse(value)
}

fn parse_rights(value: &str) -> Result<RightsState, String> {
    RightsState::parse(value)
}

fn parse_filter_op(value: &str) -> Result<FilterOp, String> {
    FilterOp::parse(value)
}

/// Builds a typed locator from CLI flags (fail closed on missing parts).
#[allow(clippy::too_many_arguments)]
fn build_locator(
    kind: &str,
    format: Option<String>,
    path: Option<String>,
    engine: Option<String>,
    database: Option<String>,
    object: Option<String>,
    provider: Option<String>,
    repo: Option<String>,
    revision: Option<String>,
    files: Option<String>,
    json: bool,
) -> anyhow::Result<SourceLocator> {
    match kind {
        "local" => {
            let (Some(format), Some(path)) = (format, path) else {
                return Err(invalid(
                    "local sources require --format and --path".to_owned(),
                    json,
                ));
            };
            let format = parse_format(&format).map_err(|message| invalid(message, json))?;
            Ok(SourceLocator::LocalPath { path, format })
        }
        "database" => {
            let (Some(engine), Some(database), Some(object)) = (engine, database, object) else {
                return Err(invalid(
                    "database sources require --engine, --database and --object".to_owned(),
                    json,
                ));
            };
            let engine = parse_engine(&engine).map_err(|message| invalid(message, json))?;
            Ok(SourceLocator::Database {
                engine,
                database,
                object,
            })
        }
        "remote" => {
            let (Some(provider), Some(repo), Some(revision), Some(files)) =
                (provider, repo, revision, files)
            else {
                return Err(invalid(
                    "remote sources require --provider, --repo, --revision and --files".to_owned(),
                    json,
                ));
            };
            let provider = parse_provider(&provider).map_err(|message| invalid(message, json))?;
            let files: Vec<String> = files
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            if files.is_empty() {
                return Err(invalid("remote --files must not be empty".to_owned(), json));
            }
            Ok(SourceLocator::RemoteDataset {
                provider,
                repo,
                revision,
                files,
            })
        }
        other => Err(invalid(format!("unknown source kind: {other}"), json)),
    }
}

/// Parses `column:op:value` filter flags.
fn parse_filter_flag(flag: &str, json: bool) -> anyhow::Result<FilterExpr> {
    let mut parts = flag.splitn(3, ':');
    let (Some(column), Some(op), Some(value)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(invalid("filter must be column:op:value".to_owned(), json));
    };
    let op = parse_filter_op(op).map_err(|message| invalid(message, json))?;
    Ok(FilterExpr {
        column: column.to_owned(),
        op,
        value: value.to_owned(),
    })
}

/// Parses `column[:desc]` sort flags.
fn parse_sort_flag(flag: &str, _json: bool) -> SortKey {
    match flag.split_once(':') {
        Some((column, direction)) => SortKey {
            column: column.to_owned(),
            descending: direction.eq_ignore_ascii_case("desc"),
        },
        None => SortKey {
            column: flag.to_owned(),
            descending: false,
        },
    }
}

/// Parses one `--op` transform flag:
/// `select:a,b` `drop:c` `rename:a:b` `cast:col:type[:strict]`
/// `filter:col:op:value` `sort:col[:desc]`.
fn parse_transform_op(flag: &str, json: bool) -> anyhow::Result<TransformOp> {
    let (name, rest) = flag.split_once(':').unwrap_or((flag, ""));
    match name {
        "select" => {
            let columns: Vec<String> = rest
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            if columns.is_empty() {
                return Err(invalid("select requires columns".to_owned(), json));
            }
            Ok(TransformOp::SelectColumns { columns })
        }
        "drop" => {
            let columns: Vec<String> = rest
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            if columns.is_empty() {
                return Err(invalid("drop requires columns".to_owned(), json));
            }
            Ok(TransformOp::DropColumns { columns })
        }
        "rename" => {
            let (from, to) = rest
                .split_once(':')
                .ok_or_else(|| invalid("rename requires from:to".to_owned(), json))?;
            Ok(TransformOp::RenameColumn {
                from: from.to_owned(),
                to: to.to_owned(),
            })
        }
        "cast" => {
            let mut parts = rest.split(':');
            let (Some(column), Some(target)) = (parts.next(), parts.next()) else {
                return Err(invalid(
                    "cast requires column:type[:strict]".to_owned(),
                    json,
                ));
            };
            let strict = parts
                .next()
                .is_some_and(|s| s.eq_ignore_ascii_case("strict"));
            let to = medscale_contracts::data_sources::FieldType::parse(target)
                .map_err(|message| invalid(message, json))?;
            Ok(TransformOp::CastType {
                column: column.to_owned(),
                to,
                strict,
            })
        }
        "filter" => {
            let filter = parse_filter_flag(rest, json)?;
            Ok(TransformOp::FilterRows {
                filters: vec![filter],
            })
        }
        "sort" => {
            let keys: Vec<SortKey> = rest
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| parse_sort_flag(s, json))
                .collect();
            if keys.is_empty() {
                return Err(invalid("sort requires keys".to_owned(), json));
            }
            Ok(TransformOp::SortRows { keys })
        }
        other => Err(invalid(format!("unknown transform op: {other}"), json)),
    }
}

fn build_view_state(
    sort: &[String],
    filter: &[String],
    group_by: Option<String>,
    columns: Option<String>,
    page_size: Option<u32>,
    json: bool,
) -> anyhow::Result<ViewState> {
    let mut filters = Vec::new();
    for flag in filter {
        filters.push(parse_filter_flag(flag, json)?);
    }
    let sort_keys: Vec<SortKey> = sort.iter().map(|s| parse_sort_flag(s, json)).collect();
    let visible_columns = columns.map(|list| {
        list.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    Ok(ViewState {
        sort: sort_keys,
        filters,
        group_by,
        visible_columns,
        page_size: page_size.unwrap_or(100),
    })
}

#[derive(Debug, Args)]
pub struct DataSourceCreateArgs {
    #[arg(long)]
    vault_id: String,
    #[arg(long)]
    vault_root: PathBuf,
    #[arg(long)]
    project_id: String,
    #[arg(long)]
    name: String,
    /// local | database | remote.
    #[arg(long)]
    kind: String,
    #[arg(long)]
    format: Option<String>,
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    engine: Option<String>,
    #[arg(long)]
    database: Option<String>,
    #[arg(long)]
    object: Option<String>,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    revision: Option<String>,
    #[arg(long)]
    files: Option<String>,
    #[arg(long)]
    credential_ref: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
pub enum DataSourceCmd {
    /// Create a data source in a Project.
    Create(Box<DataSourceCreateArgs>),
    /// List data sources for a Project.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one data source.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Update data-source metadata (revision-guarded).
    Update {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Archive a data source (snapshots and lineage are retained).
    Archive {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCmd {
    /// Preview source schema plus leading rows without persisting.
    Preview {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        max_rows: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Import current source bytes into an immutable snapshot.
    Import {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List snapshots for a source.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one snapshot.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        snapshot_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Query snapshot rows with filters/sort/paging.
    Rows {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        snapshot_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        /// Repeatable column:op:value.
        #[arg(long)]
        filter: Vec<String>,
        /// Repeatable column[:desc].
        #[arg(long)]
        sort: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Refresh a source against current external state.
    Refresh {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        allow_schema_change: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
pub struct DataViewCreateArgs {
    #[arg(long)]
    vault_id: String,
    #[arg(long)]
    vault_root: PathBuf,
    #[arg(long)]
    snapshot_id: String,
    #[arg(long)]
    kind: String,
    /// Repeatable column[:desc].
    #[arg(long)]
    sort: Vec<String>,
    /// Repeatable column:op:value.
    #[arg(long)]
    filter: Vec<String>,
    #[arg(long)]
    group_by: Option<String>,
    #[arg(long)]
    columns: Option<String>,
    #[arg(long)]
    page_size: Option<u32>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
pub struct DataViewUpdateArgs {
    #[arg(long)]
    vault_id: String,
    #[arg(long)]
    vault_root: PathBuf,
    #[arg(long)]
    view_id: String,
    #[arg(long)]
    expected_revision: u64,
    /// Repeatable column[:desc].
    #[arg(long)]
    sort: Vec<String>,
    /// Repeatable column:op:value.
    #[arg(long)]
    filter: Vec<String>,
    #[arg(long)]
    group_by: Option<String>,
    #[arg(long)]
    columns: Option<String>,
    #[arg(long)]
    page_size: Option<u32>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
pub enum DataViewCmd {
    /// Create a saved view over a snapshot.
    Create(Box<DataViewCreateArgs>),
    /// List saved views for a snapshot.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        snapshot_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one saved view.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        view_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Update one saved view state (revision-guarded).
    Update(Box<DataViewUpdateArgs>),
}

#[derive(Debug, Subcommand)]
pub enum DataTransformCmd {
    /// Execute a deterministic transformation into a new snapshot.
    Execute {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        /// Repeatable input snapshot ids (exactly one in 075).
        #[arg(long)]
        input: Vec<String>,
        /// Repeatable transform op DSL.
        #[arg(long)]
        op: Vec<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DataReleaseCmd {
    /// Create an immutable dataset release for a snapshot.
    Create {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        snapshot_id: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        split_group: Option<String>,
        #[arg(long)]
        annotation_schema: Option<String>,
        #[arg(long)]
        rights: String,
        #[arg(long)]
        json: bool,
    },
    /// List dataset releases for a Project.
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one dataset release.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        release_id: String,
        #[arg(long)]
        json: bool,
    },
}

fn print_source_human(source: &DataSourceManifest) {
    println!("source_id: {}", source.header.id.as_str());
    println!("project_id: {}", source.project_id.as_str());
    println!("kind: {}", source.kind.as_str());
    println!("name: {}", source.display_name);
    println!("format_or_engine: {}", source.format_or_engine);
    println!("health: {}", source.health.as_str());
    println!("status: {}", source.status.as_str());
    println!("revision: {}", source.revision);
}

fn print_cell_human(cell: &medscale_contracts::data_sources::CellValue) -> String {
    use medscale_contracts::data_sources::CellValue;
    match cell {
        CellValue::Null => "null".to_owned(),
        CellValue::Text(s) => s.clone(),
        CellValue::Integer(v) => v.to_string(),
        CellValue::Float(v) => v.to_string(),
        CellValue::Boolean(b) => b.to_string(),
    }
}

/// Runs one `medscale datasource ...` invocation through Core.
pub fn run_data_source(action: DataSourceCmd) -> anyhow::Result<()> {
    match action {
        DataSourceCmd::Create(args) => {
            let DataSourceCreateArgs {
                vault_id,
                vault_root,
                project_id,
                name,
                kind,
                format,
                path,
                engine,
                database,
                object,
                provider,
                repo,
                revision,
                files,
                credential_ref,
                json,
            } = args.as_ref();
            let locator = build_locator(
                kind,
                format.clone(),
                path.clone(),
                engine.clone(),
                database.clone(),
                object.clone(),
                provider.clone(),
                repo.clone(),
                revision.clone(),
                files.clone(),
                *json,
            )?;
            let mut session = open_data_session(vault_id, vault_root, *json)?;
            let source = session
                .data_source_create(
                    OpaqueId::new(project_id),
                    name.clone(),
                    locator,
                    credential_ref.clone().map(OpaqueId::new),
                )
                .map_err(|err| data_fail(&err, *json))?;
            if *json {
                print_json_or_debug(&source, true)?;
            } else {
                print_source_human(&source);
            }
            Ok(())
        }
        DataSourceCmd::List {
            vault_id,
            vault_root,
            project_id,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let (sources, next_cursor) = session
                .data_source_list(OpaqueId::new(project_id), limit, cursor)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"sources": sources, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if sources.is_empty() {
                println!("sources: empty");
            } else {
                for summary in &sources {
                    println!(
                        "{} | {} | {} | {} | rev={}",
                        summary.source_id.as_str(),
                        summary.display_name,
                        summary.kind.as_str(),
                        summary.health.as_str(),
                        summary.revision
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        DataSourceCmd::Show {
            vault_id,
            vault_root,
            source_id,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let source = session
                .data_source_get(OpaqueId::new(source_id))
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&source, true)?;
            } else {
                print_source_human(&source);
            }
            Ok(())
        }
        DataSourceCmd::Update {
            vault_id,
            vault_root,
            source_id,
            expected_revision,
            name,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let source = session
                .data_source_update(OpaqueId::new(source_id), expected_revision, name, None)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&source, true)?;
            } else {
                print_source_human(&source);
            }
            Ok(())
        }
        DataSourceCmd::Archive {
            vault_id,
            vault_root,
            source_id,
            expected_revision,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let source = session
                .data_source_archive(OpaqueId::new(source_id), expected_revision)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&source, true)?;
            } else {
                print_source_human(&source);
            }
            Ok(())
        }
    }
}

fn print_schema_human(schema: &medscale_contracts::data_sources::SourceSchema) {
    println!("fingerprint: {}", schema.schema_fingerprint.to_hex());
    for field in &schema.fields {
        println!(
            "field: {} | {} | {}",
            field.name,
            field.field_type.as_str(),
            if field.nullable {
                "nullable"
            } else {
                "required"
            }
        );
    }
}

/// Runs one `medscale snapshot ...` invocation through Core.
pub fn run_snapshot(action: SnapshotCmd) -> anyhow::Result<()> {
    match action {
        SnapshotCmd::Preview {
            vault_id,
            vault_root,
            source_id,
            max_rows,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let (schema, rows) = session
                .snapshot_preview(OpaqueId::new(source_id), max_rows)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&serde_json::json!({"schema": schema, "rows": rows}), true)?;
            } else {
                print_schema_human(&schema);
                for row in &rows {
                    let cells: Vec<String> = row.iter().map(print_cell_human).collect();
                    println!("{}", cells.join(" | "));
                }
            }
            Ok(())
        }
        SnapshotCmd::Import {
            vault_id,
            vault_root,
            source_id,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let (snapshot, receipt) = session
                .snapshot_import(OpaqueId::new(source_id))
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"snapshot": snapshot, "receipt": receipt}),
                    true,
                )?;
            } else {
                println!("snapshot_id: {}", snapshot.header.id.as_str());
                println!("rows: {}", snapshot.row_count);
                println!("digest: {}", snapshot.content_digest.to_hex());
                println!("outcome: {}", receipt.outcome.as_str());
            }
            Ok(())
        }
        SnapshotCmd::List {
            vault_id,
            vault_root,
            source_id,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let (snapshots, next_cursor) = session
                .snapshot_list(OpaqueId::new(source_id), limit, cursor)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"snapshots": snapshots, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if snapshots.is_empty() {
                println!("snapshots: empty");
            } else {
                for summary in &snapshots {
                    println!(
                        "{} | rows={} | digest={}",
                        summary.snapshot_id.as_str(),
                        summary.row_count,
                        summary.content_digest.to_hex()
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        SnapshotCmd::Show {
            vault_id,
            vault_root,
            snapshot_id,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let snapshot = session
                .snapshot_get(OpaqueId::new(snapshot_id))
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&snapshot, true)?;
            } else {
                println!("snapshot_id: {}", snapshot.header.id.as_str());
                println!("source_id: {}", snapshot.source_id.as_str());
                println!("rows: {}", snapshot.row_count);
                println!("digest: {}", snapshot.content_digest.to_hex());
            }
            Ok(())
        }
        SnapshotCmd::Rows {
            vault_id,
            vault_root,
            snapshot_id,
            limit,
            cursor,
            filter,
            sort,
            json,
        } => {
            let mut filters = Vec::new();
            for flag in &filter {
                filters.push(parse_filter_flag(flag, json)?);
            }
            let sort_keys: Vec<SortKey> = sort.iter().map(|s| parse_sort_flag(s, json)).collect();
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let page = session
                .snapshot_rows(
                    OpaqueId::new(snapshot_id),
                    limit,
                    cursor,
                    filters,
                    sort_keys,
                )
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&page, true)?;
            } else if page.rows.is_empty() {
                println!("rows: empty");
            } else {
                for row in &page.rows {
                    let cells: Vec<String> = row.iter().map(print_cell_human).collect();
                    println!("{}", cells.join(" | "));
                }
                if let Some(cursor) = page.next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        SnapshotCmd::Refresh {
            vault_id,
            vault_root,
            source_id,
            allow_schema_change,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let (receipt, snapshot) = session
                .snapshot_refresh(OpaqueId::new(source_id), allow_schema_change)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"receipt": receipt, "snapshot": snapshot}),
                    true,
                )?;
            } else {
                println!("change: {}", receipt.change_class.as_str());
                println!("outcome: {}", receipt.outcome.as_str());
                match snapshot {
                    Some(snapshot) => {
                        println!("snapshot_id: {}", snapshot.header.id.as_str());
                    }
                    None => println!("snapshot: unchanged"),
                }
            }
            Ok(())
        }
    }
}

/// Runs one `medscale dataview ...` invocation through Core.
pub fn run_data_view(action: DataViewCmd) -> anyhow::Result<()> {
    match action {
        DataViewCmd::Create(args) => {
            let DataViewCreateArgs {
                vault_id,
                vault_root,
                snapshot_id,
                kind,
                sort,
                filter,
                group_by,
                columns,
                page_size,
                json,
            } = args.as_ref();
            let view_kind = parse_view_kind(kind).map_err(|message| invalid(message, *json))?;
            let state = build_view_state(
                sort,
                filter,
                group_by.clone(),
                columns.clone(),
                *page_size,
                *json,
            )?;
            let mut session = open_data_session(vault_id, vault_root, *json)?;
            let view = session
                .saved_view_create(OpaqueId::new(snapshot_id), view_kind, state)
                .map_err(|err| data_fail(&err, *json))?;
            if *json {
                print_json_or_debug(&view, true)?;
            } else {
                println!("view_id: {}", view.header.id.as_str());
                println!("kind: {}", view.view_kind.as_str());
                println!("revision: {}", view.revision);
            }
            Ok(())
        }
        DataViewCmd::List {
            vault_id,
            vault_root,
            snapshot_id,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let (views, next_cursor) = session
                .saved_view_list(OpaqueId::new(snapshot_id), limit, cursor)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"views": views, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if views.is_empty() {
                println!("views: empty");
            } else {
                for summary in &views {
                    println!(
                        "{} | {} | rev={}",
                        summary.view_id.as_str(),
                        summary.view_kind.as_str(),
                        summary.revision
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        DataViewCmd::Show {
            vault_id,
            vault_root,
            view_id,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let view = session
                .saved_view_get(OpaqueId::new(view_id))
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&view, true)?;
            } else {
                println!("view_id: {}", view.header.id.as_str());
                println!("kind: {}", view.view_kind.as_str());
                println!("revision: {}", view.revision);
            }
            Ok(())
        }
        DataViewCmd::Update(args) => {
            let DataViewUpdateArgs {
                vault_id,
                vault_root,
                view_id,
                expected_revision,
                sort,
                filter,
                group_by,
                columns,
                page_size,
                json,
            } = args.as_ref();
            let state = build_view_state(
                sort,
                filter,
                group_by.clone(),
                columns.clone(),
                *page_size,
                *json,
            )?;
            let mut session = open_data_session(vault_id, vault_root, *json)?;
            let view = session
                .saved_view_update(OpaqueId::new(view_id), *expected_revision, state)
                .map_err(|err| data_fail(&err, *json))?;
            if *json {
                print_json_or_debug(&view, true)?;
            } else {
                println!("view_id: {}", view.header.id.as_str());
                println!("revision: {}", view.revision);
            }
            Ok(())
        }
    }
}

/// Runs one `medscale datatransform ...` invocation through Core.
pub fn run_data_transform(action: DataTransformCmd) -> anyhow::Result<()> {
    let DataTransformCmd::Execute {
        vault_id,
        vault_root,
        input,
        op,
        json,
    } = action;
    if input.is_empty() || op.is_empty() {
        return Err(invalid(
            "transform requires --input and --op".to_owned(),
            json,
        ));
    }
    let mut ops = Vec::new();
    for flag in &op {
        ops.push(parse_transform_op(flag, json)?);
    }
    let inputs: Vec<OpaqueId> = input.into_iter().map(OpaqueId::new).collect();
    let mut session = open_data_session(&vault_id, &vault_root, json)?;
    let (snapshot, receipt) = session
        .transform_execute(inputs, ops)
        .map_err(|err| data_fail(&err, json))?;
    if json {
        print_json_or_debug(
            &serde_json::json!({"snapshot": snapshot, "receipt": receipt}),
            true,
        )?;
    } else {
        println!("snapshot_id: {}", snapshot.header.id.as_str());
        println!("rows: {}", snapshot.row_count);
        println!("cast_failures: {}", receipt.cast_failures);
    }
    Ok(())
}

/// Runs one `medscale datarelease ...` invocation through Core.
pub fn run_data_release(action: DataReleaseCmd) -> anyhow::Result<()> {
    match action {
        DataReleaseCmd::Create {
            vault_id,
            vault_root,
            snapshot_id,
            version,
            split_group,
            annotation_schema,
            rights,
            json,
        } => {
            let rights_state = parse_rights(&rights).map_err(|message| invalid(message, json))?;
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let release = session
                .dataset_release_create(
                    OpaqueId::new(snapshot_id),
                    version,
                    split_group,
                    annotation_schema,
                    rights_state,
                )
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&release, true)?;
            } else {
                println!("release_id: {}", release.release_id.as_str());
                println!("version: {}", release.card.version);
                println!("digest: {}", release.snapshot_digest.to_hex());
            }
            Ok(())
        }
        DataReleaseCmd::List {
            vault_id,
            vault_root,
            project_id,
            limit,
            cursor,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let (releases, next_cursor) = session
                .dataset_release_list(OpaqueId::new(project_id), limit, cursor)
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(
                    &serde_json::json!({"releases": releases, "next_cursor": next_cursor}),
                    true,
                )?;
            } else if releases.is_empty() {
                println!("releases: empty");
            } else {
                for summary in &releases {
                    println!(
                        "{} | {} | {}",
                        summary.release_id.as_str(),
                        summary.version,
                        summary.rights_state.as_str()
                    );
                }
                if let Some(cursor) = next_cursor {
                    println!("next_cursor: {cursor}");
                }
            }
            Ok(())
        }
        DataReleaseCmd::Show {
            vault_id,
            vault_root,
            release_id,
            json,
        } => {
            let mut session = open_data_session(&vault_id, &vault_root, json)?;
            let release = session
                .dataset_release_get(OpaqueId::new(release_id))
                .map_err(|err| data_fail(&err, json))?;
            if json {
                print_json_or_debug(&release, true)?;
            } else {
                println!("release_id: {}", release.release_id.as_str());
                println!("version: {}", release.card.version);
                println!("digest: {}", release.snapshot_digest.to_hex());
            }
            Ok(())
        }
    }
}

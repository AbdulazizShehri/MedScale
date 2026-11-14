//! Data Source Fabric + Data Workbench view-models (Spec 075).
//!
//! Desktop reads and mutates data sources only through the Core-owned
//! `CliSession` (facade authority; never storage, drivers, or transports).
//! Every function below maps one typed Core result to plain view-model rows
//! plus an explicit status string. No fake product data is ever synthesized.

use medscale_contracts::data_sources::{DataViewKind, FilterExpr, FilterOp, SortKey, ViewState};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

/// One data-source list row.
#[derive(Debug, Clone, Default)]
pub struct DataSourceRowVm {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub health: String,
    pub revision: u64,
}

/// One snapshot list row.
#[derive(Debug, Clone, Default)]
pub struct SnapshotRowVm {
    pub id: String,
    pub rows: u64,
    pub digest: String,
}

/// Bounded workbench page: schema header, rendered rows, detail line.
#[derive(Debug, Clone, Default)]
pub struct WorkbenchPageVm {
    pub snapshot_id: String,
    pub schema_line: String,
    pub rows: Vec<String>,
    pub next_cursor: Option<String>,
    pub detail: String,
}

/// Maps a typed Core error to an explicit workbench status (no payload leak).
#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope
        | AuthorityError::BrokerDenied { .. }
        | AuthorityError::PackDenied { .. } => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::Conflict { .. } => "Conflict: stale revision, nothing written",
        AuthorityError::StaleReference { .. } => "Stale: source changed since pinned",
        AuthorityError::InvalidArgument { .. }
        | AuthorityError::LexicalReject { .. }
        | AuthorityError::VersionReject { .. }
        | AuthorityError::IllegalTransition => "Invalid: rejected before any write",
        AuthorityError::Corrupt { .. } | AuthorityError::DigestMismatch => {
            "Corrupt: integrity check failed"
        }
        AuthorityError::UnsupportedSchema { .. } => "Unsupported: unknown schema value",
        AuthorityError::Cancelled { .. } => "Cancelled: no write was performed",
        AuthorityError::Unavailable { .. }
        | AuthorityError::LeaseRequired
        | AuthorityError::VaultRequired
        | AuthorityError::LeaseHeld { .. }
        | AuthorityError::AlreadyHeld { .. }
        | AuthorityError::NotHolder
        | AuthorityError::NotHeld
        | AuthorityError::MissingKeyMaterial
        | AuthorityError::ExternalGateRequired { .. }
        | AuthorityError::UnknownRequiresReconcile => "Unavailable: vault or source not ready",
        AuthorityError::PathOutsideClaim | AuthorityError::Internal { .. } => {
            "Internal: unexpected Core failure"
        }
    }
}

/// Lists data sources for one Project through Core.
pub fn refresh_sources(
    session: &mut CliSession,
    project_id: &str,
) -> Result<Vec<DataSourceRowVm>, AuthorityError> {
    let (sources, _) = session.data_source_list(OpaqueId::new(project_id), Some(100), None)?;
    Ok(sources
        .into_iter()
        .map(|summary| DataSourceRowVm {
            id: summary.source_id.as_str().to_owned(),
            name: summary.display_name.clone(),
            kind: summary.kind.as_str().to_owned(),
            health: summary.health.as_str().to_owned(),
            revision: summary.revision,
        })
        .collect())
}

/// Lists snapshots for one source through Core.
pub fn source_snapshots(
    session: &mut CliSession,
    source_id: &str,
) -> Result<Vec<SnapshotRowVm>, AuthorityError> {
    let (snapshots, _) = session.snapshot_list(OpaqueId::new(source_id), Some(100), None)?;
    Ok(snapshots
        .into_iter()
        .map(|summary| SnapshotRowVm {
            id: summary.snapshot_id.as_str().to_owned(),
            rows: summary.row_count,
            digest: summary.content_digest.to_hex(),
        })
        .collect())
}

/// Loads one workbench page: typed schema header plus rendered rows.
pub fn workbench_page(
    session: &mut CliSession,
    snapshot_id: &str,
) -> Result<WorkbenchPageVm, AuthorityError> {
    let snapshot = session.snapshot_get(OpaqueId::new(snapshot_id))?;
    let page = session.snapshot_rows(
        OpaqueId::new(snapshot_id),
        Some(100),
        None,
        Vec::new(),
        Vec::new(),
    )?;
    // Schema travels with the preview path (no persistence, same contract).
    let (schema, _) = session.snapshot_preview(snapshot.source_id.clone(), Some(1))?;
    let schema_line = schema
        .fields
        .iter()
        .map(|field| format!("{} [{}]", field.name, field.field_type.as_str()))
        .collect::<Vec<_>>()
        .join("  |  ");
    let rows = page
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    medscale_contracts::data_sources::CellValue::Null => "null".to_owned(),
                    medscale_contracts::data_sources::CellValue::Text(s) => s.clone(),
                    medscale_contracts::data_sources::CellValue::Integer(v) => v.to_string(),
                    medscale_contracts::data_sources::CellValue::Float(v) => v.to_string(),
                    medscale_contracts::data_sources::CellValue::Boolean(b) => b.to_string(),
                })
                .collect::<Vec<_>>()
                .join("  |  ")
        })
        .collect();
    Ok(WorkbenchPageVm {
        snapshot_id: snapshot.header.id.as_str().to_owned(),
        detail: format!(
            "{} rows · digest {} · status {}",
            snapshot.row_count,
            snapshot.content_digest.to_hex(),
            match snapshot.status {
                medscale_contracts::data_sources::SnapshotStatus::Complete => "complete",
                medscale_contracts::data_sources::SnapshotStatus::Partial { .. } => "partial",
            }
        ),
        schema_line,
        rows,
        next_cursor: page.next_cursor,
    })
}

/// Imports the active source through Core.
pub fn import_snapshot(
    session: &mut CliSession,
    source_id: &str,
) -> Result<SnapshotRowVm, AuthorityError> {
    let (snapshot, _) = session.snapshot_import(OpaqueId::new(source_id))?;
    Ok(SnapshotRowVm {
        id: snapshot.header.id.as_str().to_owned(),
        rows: snapshot.row_count,
        digest: snapshot.content_digest.to_hex(),
    })
}

/// Refreshes the active source through Core.
pub fn refresh_source(
    session: &mut CliSession,
    source_id: &str,
    allow_schema_change: bool,
) -> Result<String, AuthorityError> {
    let (receipt, snapshot) =
        session.snapshot_refresh(OpaqueId::new(source_id), allow_schema_change)?;
    Ok(match snapshot {
        Some(snapshot) => format!(
            "{}: new snapshot {} ({} rows)",
            receipt.change_class.as_str(),
            snapshot.header.id.as_str(),
            snapshot.row_count
        ),
        None => format!("{}: no new snapshot", receipt.change_class.as_str()),
    })
}

/// Creates a default Grid view over the active snapshot through Core.
pub fn create_grid_view(
    session: &mut CliSession,
    snapshot_id: &str,
) -> Result<String, AuthorityError> {
    let view = session.saved_view_create(
        OpaqueId::new(snapshot_id),
        DataViewKind::Grid,
        ViewState {
            sort: Vec::new(),
            filters: Vec::new(),
            group_by: None,
            visible_columns: None,
            page_size: 100,
        },
    )?;
    Ok(view.header.id.as_str().to_owned())
}

/// Applies a select + optional filter + optional sort transformation.
pub fn apply_workbench_transform(
    session: &mut CliSession,
    snapshot_id: &str,
    columns: Option<Vec<String>>,
    filter: Option<FilterExpr>,
    sort: Option<SortKey>,
) -> Result<String, AuthorityError> {
    use medscale_contracts::data_sources::TransformOp;
    let mut ops = Vec::new();
    if let Some(columns) = columns {
        ops.push(TransformOp::SelectColumns { columns });
    }
    if let Some(filter) = filter {
        ops.push(TransformOp::FilterRows {
            filters: vec![filter],
        });
    }
    if let Some(key) = sort {
        ops.push(TransformOp::SortRows { keys: vec![key] });
    }
    if ops.is_empty() {
        return Err(AuthorityError::InvalidArgument {
            message: "transform needs at least one operation".to_owned(),
        });
    }
    let (snapshot, receipt) = session.transform_execute(vec![OpaqueId::new(snapshot_id)], ops)?;
    Ok(format!(
        "new snapshot {} ({} rows, {} cast failures)",
        snapshot.header.id.as_str(),
        snapshot.row_count,
        receipt.cast_failures
    ))
}

/// Parses a `column:op:value` filter from workbench inputs.
pub fn parse_filter_input(
    column: &str,
    op: &str,
    value: &str,
) -> Result<Option<FilterExpr>, &'static str> {
    if column.trim().is_empty() {
        return Ok(None);
    }
    let op = FilterOp::parse(op).map_err(|_| "Unknown filter op")?;
    Ok(Some(FilterExpr {
        column: column.trim().to_owned(),
        op,
        value: value.to_owned(),
    }))
}

/// Parses a `column[:desc]` sort input.
pub fn parse_sort_input(raw: &str) -> Option<SortKey> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    match raw.split_once(':') {
        Some((column, direction)) => Some(SortKey {
            column: column.trim().to_owned(),
            descending: direction.eq_ignore_ascii_case("desc"),
        }),
        None => Some(SortKey {
            column: raw.to_owned(),
            descending: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session(name: &str) -> (CliSession, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("medscale-075d-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut session = CliSession::connect("desktop-data-test").expect("operator session");
        session
            .open_synthetic_vault(&dir.display().to_string())
            .expect("open vault");
        (session, dir)
    }

    #[test]
    fn workbench_flows_through_real_core_session() {
        let (mut session, dir) = test_session("flows");
        std::fs::write(dir.join("table.csv"), b"city,dose\na,5\nb,9\n").unwrap();
        let project = session
            .project_create("study".to_owned(), None)
            .expect("project");
        let source = session
            .data_source_create(
                project.header.id.clone(),
                "towns".to_owned(),
                medscale_contracts::data_sources::SourceLocator::LocalPath {
                    path: "table.csv".to_owned(),
                    format: medscale_contracts::data_sources::LocalFileFormat::Csv,
                },
                None,
            )
            .expect("source");
        let rows = refresh_sources(&mut session, source.project_id.as_str()).expect("refresh");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].health, "healthy");
        let imported = import_snapshot(&mut session, source.header.id.as_str()).expect("import");
        assert_eq!(imported.rows, 2);
        let snapshots =
            source_snapshots(&mut session, source.header.id.as_str()).expect("snapshots");
        assert_eq!(snapshots.len(), 1);
        let page = workbench_page(&mut session, snapshots[0].id.as_str()).expect("page");
        assert!(page.schema_line.contains("city [text]"));
        assert_eq!(page.rows.len(), 2);
        let view_id = create_grid_view(&mut session, snapshots[0].id.as_str()).expect("view");
        assert!(view_id.starts_with("view-"));
        let summary = apply_workbench_transform(
            &mut session,
            snapshots[0].id.as_str(),
            Some(vec!["city".to_owned()]),
            None,
            None,
        )
        .expect("transform");
        assert!(summary.contains("new snapshot snap-"));
        // Statuses stay explicit for every error class.
        assert_eq!(
            status_message(&AuthorityError::Cancelled {
                message: "x".to_owned()
            }),
            "Cancelled: no write was performed"
        );
        assert_eq!(
            status_message(&AuthorityError::NotFound),
            "Missing: not found in this vault"
        );
    }

    #[test]
    fn workbench_input_parsers_reject_gracefully() {
        assert!(
            parse_filter_input("", "equals", "1")
                .expect("empty")
                .is_none()
        );
        assert!(parse_filter_input("dose", "regex", "1").is_err());
        assert!(parse_sort_input("").is_none());
        let sort = parse_sort_input("city:desc").expect("sort");
        assert!(sort.descending);
    }
}

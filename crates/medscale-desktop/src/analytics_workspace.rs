//! Analytics Gate view-models (Spec 082).
//!
//! Desktop reaches analytics only through the Core-owned `CliSession`; it
//! holds no SQL engine or storage code. Every function maps one typed Core
//! result to plain rows plus an explicit status. Denied, truncated and
//! partial-input runs are shown as such, never as complete results.

use medscale_contracts::analytics::{QueryRequest, QueryView, ViewBinding};
use medscale_contracts::data_sources::CellValue;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReceiptRowVm {
    pub id: String,
    pub outcome: String,
    pub detail: String,
}

/// Result of one run, shaped for display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunResultVm {
    pub summary: String,
    pub preview: String,
}

#[must_use]
pub fn status_message(err: &AuthorityError) -> &'static str {
    match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => "Denied by Core authority",
        AuthorityError::NotFound => "Missing: not found in this vault",
        AuthorityError::InvalidArgument { .. } => "Invalid: rejected by Core",
        AuthorityError::Corrupt { .. } => "Corrupt: integrity check failed",
        _ => "Unavailable: vault or Core not ready",
    }
}

pub fn receipts(
    session: &mut CliSession,
    project_id: &str,
) -> Result<Vec<ReceiptRowVm>, AuthorityError> {
    Ok(session
        .analytics_receipt_list(OpaqueId::new(project_id))?
        .iter()
        .rev()
        .map(|r| ReceiptRowVm {
            id: r.header.id.as_str().to_owned(),
            outcome: r.outcome.as_str().to_owned(),
            detail: format!(
                "{} · {} · {} rows{}",
                r.origin.as_str(),
                r.reproducibility.as_str(),
                r.row_count,
                r.deny_reason
                    .map(|d| format!(" · {}", d.as_str()))
                    .unwrap_or_default()
            ),
        })
        .collect())
}

fn cell(c: &CellValue) -> String {
    match c {
        CellValue::Null => "NULL".to_owned(),
        CellValue::Text(t) => t.clone(),
        CellValue::Integer(i) => i.to_string(),
        CellValue::Float(f) => f.to_string(),
        CellValue::Boolean(b) => b.to_string(),
    }
}

fn run_vm(v: &QueryView) -> RunResultVm {
    let r = &v.receipt;
    let mut summary = format!("{} · {}", r.outcome.as_str(), r.reproducibility.as_str());
    if let Some(reason) = r.deny_reason {
        summary.push_str(&format!(" · {}", reason.as_str()));
    }
    if let Some(failure) = &r.failure {
        summary.push_str(&format!(" · {failure}"));
    }
    if r.outcome.has_result() {
        summary.push_str(&format!(" · {} rows", r.row_count));
    }
    let preview = v
        .table
        .as_ref()
        .map(|t| {
            let mut lines = vec![
                t.columns
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>()
                    .join(" | "),
            ];
            lines.extend(
                t.rows
                    .iter()
                    .take(20)
                    .map(|row| row.iter().map(cell).collect::<Vec<_>>().join(" | ")),
            );
            lines.join("\n")
        })
        .unwrap_or_default();
    RunResultVm { summary, preview }
}

/// Parses `alias=snapshot_id` pairs separated by commas or spaces.
fn parse_bindings(spec: &str) -> Option<Vec<ViewBinding>> {
    let parts: Vec<&str> = spec
        .split([',', ' '])
        .filter(|p| !p.trim().is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    parts
        .iter()
        .map(|p| {
            p.split_once('=').map(|(a, s)| ViewBinding {
                alias: a.trim().to_owned(),
                snapshot_id: OpaqueId::new(s.trim()),
            })
        })
        .collect()
}

/// Runs one read-only query through Core. Every run leaves a receipt.
pub fn run_query(
    session: &mut CliSession,
    project_id: &str,
    sql: &str,
    bindings: &str,
) -> Result<RunResultVm, AuthorityError> {
    let bindings = parse_bindings(bindings).ok_or_else(|| AuthorityError::InvalidArgument {
        message: "bindings are alias=snapshot_id pairs".to_owned(),
    })?;
    let view = session.analytics_query(QueryRequest {
        project_id: OpaqueId::new(project_id),
        sql: sql.to_owned(),
        bindings,
        max_rows: None,
    })?;
    Ok(run_vm(&view))
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};

    #[test]
    fn analytics_view_models_flow_through_a_real_core_session() {
        let root =
            std::env::temp_dir().join(format!("medscale-082-desktop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("t.csv"), "k,v\na,1\nb,2\nc,3\n").unwrap();
        let mut s = CliSession::connect("vault-082-desktop").unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let project = s.project_create("a".to_owned(), None).unwrap().header.id;
        let pid = project.as_str().to_owned();
        let source = s
            .data_source_create(
                project,
                "t".to_owned(),
                SourceLocator::LocalPath {
                    path: "t.csv".to_owned(),
                    format: LocalFileFormat::Csv,
                },
                None,
            )
            .unwrap()
            .header
            .id;
        let snap = s.snapshot_import(source).unwrap().0.header.id;
        let bind = format!("t={}", snap.as_str());

        assert!(receipts(&mut s, &pid).unwrap().is_empty());
        let ok = run_query(&mut s, &pid, "SELECT SUM(v) AS total FROM t", &bind).unwrap();
        assert!(
            ok.summary.starts_with("completed · exact"),
            "{}",
            ok.summary
        );
        assert_eq!(ok.preview, "total\n6");
        let denied = run_query(&mut s, &pid, "DELETE FROM t", &bind).unwrap();
        assert!(denied.summary.contains("denied") && denied.summary.contains("not_read_only"));
        assert!(denied.preview.is_empty());
        assert!(run_query(&mut s, &pid, "SELECT 1", "t").is_err());
        let rows = receipts(&mut s, &pid).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].outcome, "denied", "newest first");
    }
}

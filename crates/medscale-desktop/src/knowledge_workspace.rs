//! Knowledge + Research Canvas view-models (Spec 083).
//!
//! Desktop reaches the index and canvases only through the Core-owned
//! `CliSession`; it holds no index, tokenizer or storage code. Every
//! function maps one typed Core result to plain rows plus an explicit
//! status. Stale and tombstoned spans are labelled, never shown as current,
//! and insufficient evidence is stated as such.

use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::knowledge::{
    IndexStatus, RetrievalOutcome, RetrievalRequest, RetrievalResult, SpanLocator,
};
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CanvasRowVm {
    pub id: String,
    pub title: String,
    pub detail: String,
}

/// Result of one search, shaped for display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchResultVm {
    pub summary: String,
    pub hits: String,
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

fn status_line(status: Option<&IndexStatus>) -> String {
    match status {
        None => "No index yet · build one to search this project".to_owned(),
        Some(s) => format!(
            "Index version {} · {} chunks · {} · {} current, {} superseded, {} tombstoned, {} not yet indexed",
            s.version,
            s.chunk_count,
            if s.is_fresh() { "fresh" } else { "stale" },
            s.current_sources,
            s.superseded_sources,
            s.tombstoned_sources,
            s.unindexed_sources
        ),
    }
}

pub fn index_status(session: &mut CliSession, project_id: &str) -> Result<String, AuthorityError> {
    let status = session.knowledge_index_status(OpaqueId::new(project_id))?;
    Ok(status_line(status.as_ref()))
}

pub fn build_index(session: &mut CliSession, project_id: &str) -> Result<String, AuthorityError> {
    let (_, status) = session.knowledge_index_build(OpaqueId::new(project_id))?;
    Ok(status_line(Some(&status)))
}

fn search_vm(r: &RetrievalResult) -> SearchResultVm {
    let receipt = &r.receipt;
    let summary = match receipt.outcome {
        RetrievalOutcome::Results => format!(
            "{} match{} · receipt {}",
            r.hits.len(),
            if r.hits.len() == 1 { "" } else { "es" },
            receipt.header.id.as_str()
        ),
        RetrievalOutcome::InsufficientEvidence => format!(
            "Insufficient evidence: no current span matches · receipt {}",
            receipt.header.id.as_str()
        ),
        RetrievalOutcome::Denied => format!(
            "Refused: {} · receipt {}",
            receipt.deny_reason.map_or("unknown", |d| d.as_str()),
            receipt.header.id.as_str()
        ),
    };
    let hits = r
        .hits
        .iter()
        .map(|h| {
            let at = match &h.hit.span.locator {
                SpanLocator::Cell { row, column } => format!("row {row} · {column}"),
                SpanLocator::Segment {
                    start_ms, end_ms, ..
                } => format!("{start_ms}-{end_ms} ms"),
                SpanLocator::Excerpt => "excerpt".to_owned(),
            };
            format!(
                "{}. [{}] {} {} · {}\n   {}",
                h.hit.rank,
                h.hit.freshness.as_str(),
                h.hit.span.source.kind.as_str(),
                h.hit.span.source.object_id.as_str(),
                at,
                h.text
                    .as_deref()
                    .unwrap_or("(not shown: source no longer readable)")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    SearchResultVm { summary, hits }
}

pub fn search(
    session: &mut CliSession,
    project_id: &str,
    query: &str,
) -> Result<SearchResultVm, AuthorityError> {
    let result = session.knowledge_search(RetrievalRequest {
        project_id: OpaqueId::new(project_id),
        query: query.to_owned(),
        max_hits: None,
        include_stale: true,
    })?;
    Ok(search_vm(&result))
}

/// [`search`], also returning the receipt the search left, so a hit can be
/// cited on a Research Canvas.
pub fn search_with_receipt(
    session: &mut CliSession,
    project_id: &str,
    query: &str,
) -> Result<
    (
        SearchResultVm,
        medscale_contracts::knowledge::RetrievalReceipt,
    ),
    AuthorityError,
> {
    let result = session.knowledge_search(RetrievalRequest {
        project_id: OpaqueId::new(project_id),
        query: query.to_owned(),
        max_hits: None,
        include_stale: true,
    })?;
    Ok((search_vm(&result), result.receipt))
}

pub fn canvases(
    session: &mut CliSession,
    project_id: &str,
) -> Result<Vec<CanvasRowVm>, AuthorityError> {
    Ok(session
        .canvas_list(OpaqueId::new(project_id))?
        .into_iter()
        .map(|c| CanvasRowVm {
            id: c.canvas_id.as_str().to_owned(),
            title: c.title,
            detail: format!(
                "revision {} · {} nodes · {} links",
                c.revision, c.node_count, c.edge_count
            ),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};

    #[test]
    fn knowledge_view_models_flow_through_a_real_core_session() {
        let root =
            std::env::temp_dir().join(format!("medscale-083-desktop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("n.csv"), "id,note\n1,myalgia after statin\n").unwrap();
        let mut s = CliSession::connect("vault-083-desktop").unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let pid = s.project_create("p".to_owned(), None).unwrap().header.id;
        let source = s
            .data_source_create(
                pid.clone(),
                "n".to_owned(),
                SourceLocator::LocalPath {
                    path: "n.csv".to_owned(),
                    format: LocalFileFormat::Csv,
                },
                None,
            )
            .unwrap()
            .header
            .id;
        s.snapshot_import(source).unwrap();
        let p = pid.as_str();

        assert!(index_status(&mut s, p).unwrap().starts_with("No index yet"));
        let none = search(&mut s, p, "myalgia").unwrap();
        assert!(none.summary.starts_with("Refused: no_index"), "{none:?}");
        assert!(build_index(&mut s, p).unwrap().contains("fresh"));
        let hit = search(&mut s, p, "myalgia").unwrap();
        assert!(hit.summary.starts_with("1 match"), "{hit:?}");
        assert!(hit.hits.contains("[current]") && hit.hits.contains("myalgia after statin"));
        let miss = search(&mut s, p, "rhabdomyolysis").unwrap();
        assert!(miss.summary.starts_with("Insufficient evidence"));
        assert!(miss.hits.is_empty());
        assert!(canvases(&mut s, p).unwrap().is_empty());
        s.canvas_create(pid.clone(), "Statins".to_owned()).unwrap();
        let rows = canvases(&mut s, p).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Statins");
        assert!(matches!(
            index_status(&mut s, "proj-missing"),
            Err(AuthorityError::NotFound)
        ));
    }
}

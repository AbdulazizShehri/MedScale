//! Spec 083 Knowledge + Research Canvas commands (CLI vertical slice
//! through Core).
//!
//! Every command dispatches typed Core requests via `CliSession` and renders
//! typed results as human lines or stable JSON. Retrieval is lexical only,
//! every search leaves a receipt, and stale or tombstoned spans are labelled,
//! never shown as current.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::knowledge::{
    CanvasOp, CanvasView, EvidenceSpanRef, IndexStatus, RetrievalRequest, RetrievalResult,
    SpanLocator,
};
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;

use super::{fail_json, print_json_or_debug};

fn knowledge_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope => ("denied", debug),
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::LeaseRequired | AuthorityError::VaultRequired => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn open_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<CliSession> {
    let mut session = CliSession::connect(vault_id).map_err(|err| knowledge_fail(&err, json))?;
    session
        .open_synthetic_vault(&vault_root.display().to_string())
        .map_err(|err| knowledge_fail(&err, json))?;
    Ok(session)
}

/// Human form of a span: `kind object locator [start,end)`.
fn span_label(span: &EvidenceSpanRef) -> String {
    let at = match &span.locator {
        SpanLocator::Cell { row, column } => format!("row {row} column {column}"),
        SpanLocator::Segment {
            seq,
            start_ms,
            end_ms,
        } => format!("segment {seq} {start_ms}-{end_ms} ms"),
        SpanLocator::Excerpt => "excerpt".to_owned(),
    };
    format!(
        "{} {} {at} [{},{})",
        span.source.kind.as_str(),
        span.source.object_id.as_str(),
        span.char_start,
        span.char_end
    )
}

fn print_status(s: &IndexStatus) {
    println!(
        "index: {} version {} chunks={} {}",
        s.manifest_id.as_str(),
        s.version,
        s.chunk_count,
        if s.is_fresh() { "fresh" } else { "stale" }
    );
    println!(
        "sources: current={} superseded={} tombstoned={} unindexed={}",
        s.current_sources, s.superseded_sources, s.tombstoned_sources, s.unindexed_sources
    );
}

fn print_result(r: &RetrievalResult, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(r, true);
    }
    let receipt = &r.receipt;
    println!("receipt_id: {}", receipt.header.id.as_str());
    println!("outcome: {}", receipt.outcome.as_str());
    if let Some(reason) = receipt.deny_reason {
        println!("reason: {}", reason.as_str());
    }
    if let Some(status) = &receipt.index_status {
        print_status(status);
    }
    for hit in &r.hits {
        println!(
            "{}\t{}\tscore={}\t{}",
            hit.hit.rank,
            hit.hit.freshness.as_str(),
            hit.hit.score,
            span_label(&hit.hit.span)
        );
        match &hit.text {
            Some(text) => println!("\t{}", text.escape_debug()),
            None => println!("\t(not shown: source no longer readable)"),
        }
    }
    Ok(())
}

fn print_canvas(v: &CanvasView, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(v, true);
    }
    let c = &v.canvas;
    println!(
        "canvas: {} revision {} {}",
        c.header.id.as_str(),
        c.revision,
        c.title.escape_debug()
    );
    for node in &c.nodes {
        match &node.content {
            medscale_contracts::knowledge::CanvasNodeContent::Note { text } => {
                println!("note {}\t{}", node.key, text.escape_debug());
            }
            medscale_contracts::knowledge::CanvasNodeContent::Evidence { span } => {
                let r = v.resolutions.iter().find(|r| r.key == node.key);
                println!(
                    "evidence {}\t{}\t{}",
                    node.key,
                    r.map_or("unknown", |r| r.freshness.as_str()),
                    span_label(span)
                );
                if let Some(text) = r.and_then(|r| r.text.as_ref()) {
                    println!("\t{}", text.escape_debug());
                }
            }
        }
    }
    for e in &c.edges {
        println!("edge {} {} {}", e.from, e.relation.as_str(), e.to);
    }
    println!("unsupported notes: {}", v.unsupported_notes.join(", "));
    println!("contradictions: {}", v.contradictions.len());
    println!("superseded evidence: {}", v.superseded_evidence.join(", "));
    println!("missing evidence: {}", v.missing_evidence.join(", "));
    let overlaps: Vec<String> = v
        .overlapping_evidence
        .iter()
        .map(|o| format!("{} = {} ({:?})", o.first, o.second, o.kind).to_lowercase())
        .collect();
    println!("overlapping evidence: {}", overlaps.join(", "));
    for c in &v.evidence_coverage {
        println!(
            "coverage {}	{}	{:?}	{} source(s)",
            c.note, c.evidence_class, c.state, c.source_count
        );
    }
    println!("coverage hash: {}", v.coverage_hash);
    Ok(())
}

/// Spec 083 Knowledge + Research Canvas commands.
#[derive(Debug, Subcommand)]
pub enum KnowledgeCmd {
    /// Build the next index version from the Project's current sources.
    Build {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Freshness of the Project's latest index version.
    Status {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Lexical search over the latest index version (leaves a receipt).
    Search {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        query: String,
        #[arg(long)]
        max_hits: Option<u32>,
        /// Also list superseded and tombstoned matches (labelled).
        #[arg(long)]
        include_stale: bool,
        #[arg(long)]
        json: bool,
    },
    /// Retrieval receipts of a Project.
    Receipts {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Create an empty Research Canvas.
    CanvasCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        json: bool,
    },
    /// Apply edits (a JSON array of canvas ops) to an expected revision.
    CanvasEdit {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        canvas_id: String,
        #[arg(long)]
        expected_revision: u32,
        /// e.g. `[{"op":"add_note","key":"claim","text":"..."}]`.
        #[arg(long)]
        ops_json: String,
        #[arg(long)]
        json: bool,
    },
    /// Cite one hit of a stored retrieval receipt as a Canvas evidence node.
    CanvasCite {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        canvas_id: String,
        #[arg(long)]
        expected_revision: u32,
        #[arg(long)]
        receipt_id: String,
        #[arg(long)]
        rank: u32,
        #[arg(long)]
        key: String,
        #[arg(long)]
        json: bool,
    },
    /// Show a Canvas revision resolved live, with its inspection.
    CanvasShow {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        canvas_id: String,
        #[arg(long)]
        revision: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Canvases of a Project.
    CanvasList {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

fn print_revision(
    c: &medscale_contracts::knowledge::CanvasRevision,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(c, true);
    }
    println!(
        "{}\trevision {}\t{} nodes\t{} edges",
        c.header.id.as_str(),
        c.revision,
        c.nodes.len(),
        c.edges.len()
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub fn run_knowledge(cmd: KnowledgeCmd) -> anyhow::Result<()> {
    match cmd {
        KnowledgeCmd::Build {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let (manifest, status) = s
                .knowledge_index_build(OpaqueId::new(project_id))
                .map_err(|err| knowledge_fail(&err, json))?;
            if json {
                return print_json_or_debug(&(manifest, status), true);
            }
            println!(
                "tokenizer: {} sources={} chunks_digest=sha256:{}",
                manifest.tokenizer,
                manifest.sources.len(),
                manifest.chunks_digest.to_hex()
            );
            print_status(&status);
            Ok(())
        }
        KnowledgeCmd::Status {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let status = s
                .knowledge_index_status(OpaqueId::new(project_id))
                .map_err(|err| knowledge_fail(&err, json))?;
            if json {
                return print_json_or_debug(&status, true);
            }
            match &status {
                Some(status) => print_status(status),
                None => println!("index: none (run `medscale knowledge build`)"),
            }
            Ok(())
        }
        KnowledgeCmd::Search {
            vault_id,
            vault_root,
            project_id,
            query,
            max_hits,
            include_stale,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let result = s
                .knowledge_search(RetrievalRequest {
                    project_id: OpaqueId::new(project_id),
                    query,
                    max_hits,
                    include_stale,
                })
                .map_err(|err| knowledge_fail(&err, json))?;
            print_result(&result, json)
        }
        KnowledgeCmd::Receipts {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let receipts = s
                .knowledge_receipt_list(OpaqueId::new(project_id))
                .map_err(|err| knowledge_fail(&err, json))?;
            if json {
                return print_json_or_debug(&receipts, true);
            }
            for r in &receipts {
                println!(
                    "{}\t{}\t{}\thits={}\t{}",
                    r.header.id.as_str(),
                    r.outcome.as_str(),
                    r.deny_reason.map_or("-", |d| d.as_str()),
                    r.hits.len(),
                    r.query.escape_debug()
                );
            }
            Ok(())
        }
        KnowledgeCmd::CanvasCreate {
            vault_id,
            vault_root,
            project_id,
            title,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let c = s
                .canvas_create(OpaqueId::new(project_id), title)
                .map_err(|err| knowledge_fail(&err, json))?;
            print_revision(&c, json)
        }
        KnowledgeCmd::CanvasEdit {
            vault_id,
            vault_root,
            canvas_id,
            expected_revision,
            ops_json,
            json,
        } => {
            let ops: Vec<CanvasOp> = serde_json::from_str(&ops_json)
                .map_err(|e| invalid(format!("--ops-json: {e}"), json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let c = s
                .canvas_edit(OpaqueId::new(canvas_id), expected_revision, ops)
                .map_err(|err| knowledge_fail(&err, json))?;
            print_revision(&c, json)
        }
        KnowledgeCmd::CanvasCite {
            vault_id,
            vault_root,
            canvas_id,
            expected_revision,
            receipt_id,
            rank,
            key,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let receipt = s
                .knowledge_receipt_get(OpaqueId::new(receipt_id))
                .map_err(|err| knowledge_fail(&err, json))?;
            let span = receipt
                .hits
                .iter()
                .find(|h| h.rank == rank)
                .map(|h| h.span.clone())
                .ok_or_else(|| invalid(format!("the receipt has no hit {rank}"), json))?;
            let c = s
                .canvas_edit(
                    OpaqueId::new(canvas_id),
                    expected_revision,
                    vec![CanvasOp::AddEvidence { key, span }],
                )
                .map_err(|err| knowledge_fail(&err, json))?;
            print_revision(&c, json)
        }
        KnowledgeCmd::CanvasShow {
            vault_id,
            vault_root,
            canvas_id,
            revision,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .canvas_get(OpaqueId::new(canvas_id), revision)
                .map_err(|err| knowledge_fail(&err, json))?;
            print_canvas(&view, json)
        }
        KnowledgeCmd::CanvasList {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let list = s
                .canvas_list(OpaqueId::new(project_id))
                .map_err(|err| knowledge_fail(&err, json))?;
            if json {
                return print_json_or_debug(&list, true);
            }
            for c in &list {
                println!(
                    "{}\trevision {}\t{} nodes\t{} edges\t{}",
                    c.canvas_id.as_str(),
                    c.revision,
                    c.node_count,
                    c.edge_count,
                    c.title.escape_debug()
                );
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
    use medscale_contracts::knowledge::{Freshness, RetrievalOutcome};

    const VAULT: &str = "vault-083-cli";

    fn args(root: &std::path::Path) -> (String, PathBuf) {
        (VAULT.to_owned(), root.to_path_buf())
    }

    #[test]
    fn knowledge_commands_run_through_core_across_fresh_sessions() {
        let root = std::env::temp_dir().join(format!("medscale-083-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("notes.csv"),
            "id,note\n1,myalgia after statin start\n2,no complaints\n",
        )
        .unwrap();
        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let project = s
            .project_create("notes".to_owned(), None)
            .unwrap()
            .header
            .id;
        let source = s
            .data_source_create(
                project.clone(),
                "notes".to_owned(),
                SourceLocator::LocalPath {
                    path: "notes.csv".to_owned(),
                    format: LocalFileFormat::Csv,
                },
                None,
            )
            .unwrap()
            .header
            .id;
        s.snapshot_import(source).unwrap();
        drop(s);
        let pid = project.as_str().to_owned();
        let (vault_id, vault_root) = args(&root);

        for json in [true, false] {
            run_knowledge(KnowledgeCmd::Status {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                json,
            })
            .expect("status without an index");
            run_knowledge(KnowledgeCmd::Build {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                json,
            })
            .expect("build");
            run_knowledge(KnowledgeCmd::Search {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                query: "myalgia".to_owned(),
                max_hits: None,
                include_stale: false,
                json,
            })
            .expect("search");
            run_knowledge(KnowledgeCmd::Receipts {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                json,
            })
            .expect("receipts");
        }
        run_knowledge(KnowledgeCmd::CanvasCreate {
            vault_id: vault_id.clone(),
            vault_root: vault_root.clone(),
            project_id: pid.clone(),
            title: "Statins".to_owned(),
            json: true,
        })
        .expect("canvas create");

        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let receipts = s.knowledge_receipt_list(project.clone()).unwrap();
        assert_eq!(receipts.len(), 2);
        assert!(
            receipts
                .iter()
                .all(|r| r.outcome == RetrievalOutcome::Results
                    && r.hits[0].freshness == Freshness::Current)
        );
        let canvas = s.canvas_list(project.clone()).unwrap()[0].canvas_id.clone();
        drop(s);

        assert!(
            run_knowledge(KnowledgeCmd::CanvasEdit {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                canvas_id: canvas.as_str().to_owned(),
                expected_revision: 1,
                ops_json: "not json".to_owned(),
                json: true,
            })
            .is_err()
        );
        run_knowledge(KnowledgeCmd::CanvasEdit {
            vault_id: vault_id.clone(),
            vault_root: vault_root.clone(),
            canvas_id: canvas.as_str().to_owned(),
            expected_revision: 1,
            ops_json: r#"[{"op":"add_note","key":"claim","text":"Statin start preceded myalgia"}]"#
                .to_owned(),
            json: false,
        })
        .expect("edit");
        run_knowledge(KnowledgeCmd::CanvasCite {
            vault_id: vault_id.clone(),
            vault_root: vault_root.clone(),
            canvas_id: canvas.as_str().to_owned(),
            expected_revision: 2,
            receipt_id: receipts[0].header.id.as_str().to_owned(),
            rank: 1,
            key: "chart".to_owned(),
            json: false,
        })
        .expect("cite");
        assert!(
            run_knowledge(KnowledgeCmd::CanvasCite {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                canvas_id: canvas.as_str().to_owned(),
                expected_revision: 2,
                receipt_id: receipts[0].header.id.as_str().to_owned(),
                rank: 1,
                key: "again".to_owned(),
                json: true,
            })
            .is_err(),
            "stale expected revision"
        );
        for json in [true, false] {
            run_knowledge(KnowledgeCmd::CanvasShow {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                canvas_id: canvas.as_str().to_owned(),
                revision: None,
                json,
            })
            .expect("show");
            run_knowledge(KnowledgeCmd::CanvasList {
                vault_id: vault_id.clone(),
                vault_root: vault_root.clone(),
                project_id: pid.clone(),
                json,
            })
            .expect("list");
        }

        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let view = s.canvas_get(canvas, None).unwrap();
        assert_eq!(view.canvas.revision, 3);
        // Cited, but not yet linked as support: the note stays unsupported.
        assert_eq!(view.unsupported_notes, vec!["claim".to_owned()]);
        let chart = &view.resolutions[0];
        assert_eq!(chart.text.as_deref(), Some("myalgia after statin start"));
    }
}

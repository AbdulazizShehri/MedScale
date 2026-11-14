//! Spec 082 Analytics Gate commands (CLI vertical slice through Core).
//!
//! Every command dispatches typed Core requests via `CliSession` and renders
//! typed results as human lines or stable JSON. Queries are read-only SQL
//! over exact snapshots; every run leaves a receipt.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::analytics::{
    CohortCriterion, CohortOp, QueryReceipt, QueryRequest, QueryView, StatisticKind,
    StatisticValue, ViewBinding,
};
use medscale_contracts::data_sources::CellValue;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;

use super::{fail_json, print_json_or_debug};

fn analytics_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| analytics_fail(&err, json))
}

fn cell_text(c: &CellValue) -> String {
    match c {
        CellValue::Null => "NULL".to_owned(),
        CellValue::Text(t) => t.escape_debug().to_string(),
        CellValue::Integer(i) => i.to_string(),
        CellValue::Float(f) => f.to_string(),
        CellValue::Boolean(b) => b.to_string(),
    }
}

fn print_receipt(r: &QueryReceipt) {
    println!("receipt_id: {}", r.header.id.as_str());
    println!("outcome: {}", r.outcome.as_str());
    if let Some(reason) = r.deny_reason {
        println!("reason: {}", reason.as_str());
    }
    if let Some(failure) = &r.failure {
        println!("failure: {failure}");
    }
    println!(
        "engine: {} {} (max_rows {}, timeout {} ms)",
        r.engine.engine, r.engine.version, r.engine.max_rows, r.engine.timeout_ms
    );
    println!("inputs: {}", r.reproducibility.as_str());
    for i in &r.inputs {
        println!(
            "  {} = {} sha256:{} rows={}{}",
            i.alias,
            i.snapshot_id.as_str(),
            i.content_digest.to_hex(),
            i.row_count,
            if i.complete { "" } else { " (partial)" }
        );
    }
    if let Some(id) = &r.result_id {
        println!(
            "result: {} rows={} columns={}",
            id.as_str(),
            r.row_count,
            r.column_count
        );
    }
}

fn print_view(v: &QueryView, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(v, true);
    }
    print_receipt(&v.receipt);
    if let Some(t) = &v.table {
        println!(
            "{}",
            t.columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join("\t")
        );
        for row in t.rows.iter().take(50) {
            println!(
                "{}",
                row.iter().map(cell_text).collect::<Vec<_>>().join("\t")
            );
        }
        if t.rows.len() > 50 {
            println!("... {} more rows (use --json)", t.rows.len() - 50);
        }
    }
    Ok(())
}

fn parse_binding(spec: &str) -> Result<ViewBinding, String> {
    let (alias, snapshot) = spec
        .split_once('=')
        .ok_or_else(|| format!("binding {spec} must be alias=snapshot_id"))?;
    Ok(ViewBinding {
        alias: alias.trim().to_owned(),
        snapshot_id: OpaqueId::new(snapshot.trim()),
    })
}

/// `"..."` is always text; `true`/`false` are booleans; otherwise an
/// integer, a float, or text. Core refuses a value that does not fit its
/// field's type.
fn parse_value(raw: &str) -> CellValue {
    if let Some(text) = raw
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        CellValue::Text(text.to_owned())
    } else if raw == "true" || raw == "false" {
        CellValue::Boolean(raw == "true")
    } else if let Ok(i) = raw.parse::<i64>() {
        CellValue::Integer(i)
    } else if let Ok(f) = raw.parse::<f64>() {
        CellValue::Float(f)
    } else {
        CellValue::Text(raw.to_owned())
    }
}

/// `field:op` or `field:op:value` (see `parse_value` for value typing).
fn parse_criterion(spec: &str) -> Result<CohortCriterion, String> {
    let mut parts = spec.splitn(3, ':');
    let field = parts.next().unwrap_or("").to_owned();
    let op = CohortOp::parse(parts.next().unwrap_or(""))?;
    let value = parts.next().map(parse_value);
    Ok(CohortCriterion { field, op, value })
}

/// Spec 082 Analytics Gate commands.
#[derive(Debug, Subcommand)]
pub enum AnalyticsCmd {
    /// Run one read-only SQL query over bound snapshots.
    Query {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        sql: String,
        /// `alias=snapshot_id`; repeat for several tables.
        #[arg(long = "bind", required = true)]
        bindings: Vec<String>,
        #[arg(long)]
        max_rows: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// List the Project's query receipts.
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
    /// Re-run a receipt against its pinned snapshots and compare digests.
    Replay {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        receipt_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Descriptive statistics over one column of a stored result.
    Stats {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        result_id: String,
        #[arg(long)]
        column: String,
        /// Comma separated: count,missing,mean,std_dev,min,median,max
        #[arg(long, default_value = "count,missing,mean,std_dev,min,median,max")]
        kinds: String,
        #[arg(long)]
        json: bool,
    },
    /// Store a cohort: every criterion (`field:op[:value]`) must hold.
    CohortCreate {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        snapshot_id: String,
        /// `field:op[:value]`; repeat for AND. A `"..."` value is always
        /// text, and `true`/`false` are booleans.
        #[arg(long = "where", required = true)]
        criteria: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Run a stored cohort as a read-only query.
    CohortRun {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        cohort_id: String,
        #[arg(long)]
        max_rows: Option<u32>,
        #[arg(long)]
        json: bool,
    },
}

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

pub fn run_analytics(cmd: AnalyticsCmd) -> anyhow::Result<()> {
    match cmd {
        AnalyticsCmd::Query {
            vault_id,
            vault_root,
            project_id,
            sql,
            bindings,
            max_rows,
            json,
        } => {
            let bindings = bindings
                .iter()
                .map(|b| parse_binding(b))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .analytics_query(QueryRequest {
                    project_id: OpaqueId::new(project_id),
                    sql,
                    bindings,
                    max_rows,
                })
                .map_err(|err| analytics_fail(&err, json))?;
            print_view(&view, json)
        }
        AnalyticsCmd::Receipts {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let receipts = s
                .analytics_receipt_list(OpaqueId::new(project_id))
                .map_err(|err| analytics_fail(&err, json))?;
            if json {
                return print_json_or_debug(&receipts, true);
            }
            for r in &receipts {
                println!(
                    "{}\t{}\t{}\t{}\trows={}",
                    r.header.id.as_str(),
                    r.origin.as_str(),
                    r.outcome.as_str(),
                    r.deny_reason.map_or("-", |d| d.as_str()),
                    r.row_count
                );
            }
            Ok(())
        }
        AnalyticsCmd::Replay {
            vault_id,
            vault_root,
            receipt_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let report = s
                .analytics_replay(OpaqueId::new(receipt_id))
                .map_err(|err| analytics_fail(&err, json))?;
            if json {
                return print_json_or_debug(&report, true);
            }
            println!(
                "{}\t{}",
                report.receipt_id.as_str(),
                report.verdict.as_str()
            );
            Ok(())
        }
        AnalyticsCmd::Stats {
            vault_id,
            vault_root,
            result_id,
            column,
            kinds,
            json,
        } => {
            let kinds = kinds
                .split(',')
                .map(|k| StatisticKind::parse(k.trim()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let stats = s
                .analytics_statistics(OpaqueId::new(result_id), column, kinds)
                .map_err(|err| analytics_fail(&err, json))?;
            if json {
                return print_json_or_debug(&stats, true);
            }
            for st in &stats {
                let shown = match &st.value {
                    StatisticValue::Value { value } => value.to_string(),
                    StatisticValue::Insufficient { needed, available } => {
                        format!("insufficient (needs {needed}, has {available})")
                    }
                    StatisticValue::NotNumeric => "not numeric".to_owned(),
                };
                println!("{}\t{}\t{shown}", st.column, st.kind.as_str());
            }
            Ok(())
        }
        AnalyticsCmd::CohortCreate {
            vault_id,
            vault_root,
            project_id,
            label,
            snapshot_id,
            criteria,
            json,
        } => {
            let criteria = criteria
                .iter()
                .map(|c| parse_criterion(c))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| invalid(e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let cohort = s
                .analytics_cohort_create(
                    OpaqueId::new(project_id),
                    label,
                    OpaqueId::new(snapshot_id),
                    criteria,
                )
                .map_err(|err| analytics_fail(&err, json))?;
            if json {
                return print_json_or_debug(&cohort, true);
            }
            println!(
                "{}\t{}\t{} criteria\tsnapshot {}",
                cohort.header.id.as_str(),
                cohort.label.escape_debug(),
                cohort.criteria.len(),
                cohort.snapshot_id.as_str()
            );
            Ok(())
        }
        AnalyticsCmd::CohortRun {
            vault_id,
            vault_root,
            cohort_id,
            max_rows,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .analytics_cohort_run(OpaqueId::new(cohort_id), max_rows)
                .map_err(|err| analytics_fail(&err, json))?;
            print_view(&view, json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::analytics::{QueryDenyReason, QueryOutcome, ReplayVerdict};
    use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
    use medscale_core::CliSession;

    const VAULT: &str = "vault-082-cli";

    #[test]
    fn criteria_and_bindings_parse_strictly() {
        let c = parse_criterion("age:ge:40").unwrap();
        assert_eq!(c.value, Some(CellValue::Integer(40)));
        assert_eq!(
            parse_criterion("ldl:lt:4.5").unwrap().value,
            Some(CellValue::Float(4.5))
        );
        assert_eq!(parse_criterion("ldl:is_null").unwrap().value, None);
        assert_eq!(
            parse_criterion("zip:eq:\"007\"").unwrap().value,
            Some(CellValue::Text("007".to_owned()))
        );
        assert_eq!(
            parse_criterion("smoker:eq:true").unwrap().value,
            Some(CellValue::Boolean(true))
        );
        assert_eq!(
            parse_criterion("sex:eq:f").unwrap().value,
            Some(CellValue::Text("f".to_owned()))
        );
        assert!(parse_criterion("age:like:4").is_err());
        assert!(parse_binding("labs").is_err());
        assert_eq!(parse_binding("labs=snap-1").unwrap().alias, "labs");
    }

    #[test]
    fn analytics_commands_run_through_core_across_fresh_sessions() {
        let root = std::env::temp_dir().join(format!("medscale-082-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("labs.csv"),
            "id,age,sex\n1,34,f\n2,71,m\n3,58,f\n",
        )
        .unwrap();
        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let project = s.project_create("labs".to_owned(), None).unwrap().header.id;
        let source = s
            .data_source_create(
                project.clone(),
                "labs".to_owned(),
                SourceLocator::LocalPath {
                    path: "labs.csv".to_owned(),
                    format: LocalFileFormat::Csv,
                },
                None,
            )
            .unwrap()
            .header
            .id;
        let snapshot = s.snapshot_import(source).unwrap().0.header.id;
        drop(s);
        let pid = project.as_str().to_owned();
        let bind = format!("labs={}", snapshot.as_str());

        for json in [true, false] {
            run_analytics(AnalyticsCmd::Query {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                sql: "SELECT sex, COUNT(*) AS n FROM labs GROUP BY sex ORDER BY sex".to_owned(),
                bindings: vec![bind.clone()],
                max_rows: None,
                json,
            })
            .expect("query");
        }
        run_analytics(AnalyticsCmd::Query {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            project_id: pid.clone(),
            sql: "DELETE FROM labs".to_owned(),
            bindings: vec![bind.clone()],
            max_rows: None,
            json: true,
        })
        .expect("a denied query still returns its receipt");
        assert!(
            run_analytics(AnalyticsCmd::Query {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                sql: "SELECT 1".to_owned(),
                bindings: vec!["no-equals".to_owned()],
                max_rows: None,
                json: true,
            })
            .is_err()
        );
        for json in [true, false] {
            run_analytics(AnalyticsCmd::CohortCreate {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                label: format!("older {json}"),
                snapshot_id: snapshot.as_str().to_owned(),
                criteria: vec!["age:ge:50".to_owned()],
                json,
            })
            .expect("cohort");
            run_analytics(AnalyticsCmd::Receipts {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: pid.clone(),
                json,
            })
            .expect("receipts");
        }

        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let receipts = s.analytics_receipt_list(project.clone()).unwrap();
        assert_eq!(receipts.len(), 3);
        assert_eq!(receipts[0].outcome, QueryOutcome::Completed);
        assert_eq!(receipts[2].deny_reason, Some(QueryDenyReason::NotReadOnly));
        let cohorts = s.analytics_cohort_list(project.clone()).unwrap();
        assert_eq!(cohorts.len(), 2);
        let first = receipts[0].clone();
        drop(s);

        run_analytics(AnalyticsCmd::CohortRun {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            cohort_id: cohorts[0].header.id.as_str().to_owned(),
            max_rows: None,
            json: false,
        })
        .expect("cohort run");
        for json in [true, false] {
            run_analytics(AnalyticsCmd::Replay {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                receipt_id: first.header.id.as_str().to_owned(),
                json,
            })
            .expect("replay");
            run_analytics(AnalyticsCmd::Stats {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                result_id: first.result_id.clone().unwrap().as_str().to_owned(),
                column: "n".to_owned(),
                kinds: "count,mean,std_dev".to_owned(),
                json,
            })
            .expect("stats");
        }
        assert!(
            run_analytics(AnalyticsCmd::Stats {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                result_id: first.result_id.clone().unwrap().as_str().to_owned(),
                column: "n".to_owned(),
                kinds: "mode".to_owned(),
                json: true,
            })
            .is_err()
        );

        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        assert_eq!(
            s.analytics_replay(first.header.id).unwrap().verdict,
            ReplayVerdict::Reproduced
        );
        let all = s.analytics_receipt_list(project).unwrap();
        assert_eq!(all.len(), 4, "the cohort run left a receipt");
        assert_eq!(all[3].row_count, 2);
    }
}

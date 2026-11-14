//! Spec 085 MedScale Compute commands (CLI vertical slice through Core).
//!
//! Every command dispatches typed Core requests via `CliSession` and renders
//! typed results as human lines or stable JSON. Jobs run only the closed
//! set of MedScale job kinds in the MedScale worker; there is no way to
//! pass code, a script, a shell command or a worker path.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::compute::{
    ComputeJobKind, ComputeJobRequest, ComputeJobView, ComputeParams, SandboxRequirement,
};
use medscale_contracts::data_sources::CellValue;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;

use super::{fail_json, print_json_or_debug};

fn compute_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
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

fn invalid(message: String, json: bool) -> anyhow::Error {
    fail_json("invalid", message, json)
}

fn open_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    crate::vault_session::open(vault_id, vault_root).map_err(|err| compute_fail(&err, json))
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

fn split_names(raw: Option<&str>) -> Vec<String> {
    raw.map(|r| {
        r.split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

fn build_params(
    kind: &str,
    columns: Option<&str>,
    sort_by: Option<&str>,
    descending: bool,
) -> Result<ComputeParams, String> {
    match ComputeJobKind::parse(kind)? {
        ComputeJobKind::ColumnProfile => {
            if columns.is_some() || sort_by.is_some() || descending {
                return Err("column_profile takes no columns or sort keys".to_owned());
            }
            Ok(ComputeParams::ColumnProfile {})
        }
        ComputeJobKind::SortedProjection => Ok(ComputeParams::SortedProjection {
            columns: split_names(columns),
            sort_by: split_names(sort_by),
            descending,
        }),
    }
}

fn print_view(v: &ComputeJobView, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(v, true);
    }
    let job = &v.job;
    println!("job_id: {}", job.header.id.as_str());
    println!("kind: {}", job.manifest.kind.as_str());
    println!("state: {}", job.state.as_str());
    match &job.manifest.input {
        Some(i) => println!(
            "input: {} sha256:{} rows={} bytes={}",
            i.snapshot_id.as_str(),
            i.content_digest.to_hex(),
            i.row_count,
            i.byte_len
        ),
        None => println!(
            "input: {} (not pinned)",
            job.manifest.requested_snapshot_id.as_str()
        ),
    }
    println!(
        "runtime: {} {} protocol {} kind v{}",
        job.manifest.runtime.worker,
        job.manifest.runtime.worker_version,
        job.manifest.runtime.protocol_version,
        job.manifest.runtime.kind_version
    );
    println!("sandbox_required: {}", job.manifest.policy.sandbox.as_str());
    if let Some(r) = &v.receipt {
        println!("receipt_id: {}", r.header.id.as_str());
        if let Some(reason) = r.deny_reason {
            println!("reason: {}", reason.as_str());
        }
        if let Some(failure) = r.failure {
            println!("failure: {}", failure.as_str());
        }
        if let Some(s) = &r.sandbox {
            println!(
                "sandbox: {} ready_base_applied={} platform_qualified={}",
                s.mechanism.as_str(),
                s.ready_base_applied,
                s.platform_qualified
            );
        }
        println!(
            "stdout_bytes: {} stderr_bytes: {} (stderr text is never stored)",
            r.stdout_bytes, r.stderr_bytes
        );
    }
    if let (Some(o), Some(t)) = (&v.output, &v.table) {
        println!(
            "output: {} sha256:{} rows={} columns={} review=unreviewed",
            o.header.id.as_str(),
            o.content_digest.to_hex(),
            o.row_count,
            o.column_count
        );
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

/// Spec 085 MedScale Compute commands.
#[derive(Debug, Subcommand)]
pub enum ComputeCmd {
    /// Admit one job over one snapshot of the Project (does not run it).
    Submit {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        snapshot_id: String,
        /// `column_profile` or `sorted_projection`.
        #[arg(long)]
        kind: String,
        /// Comma separated (sorted_projection).
        #[arg(long)]
        columns: Option<String>,
        /// Comma separated sort keys (sorted_projection).
        #[arg(long)]
        sort_by: Option<String>,
        #[arg(long)]
        descending: bool,
        /// Accept process isolation without the OS READY_BASE mechanism.
        #[arg(long)]
        process_isolation_only: bool,
        #[arg(long)]
        json: bool,
    },
    /// Run one queued job in the MedScale worker (a job runs at most once).
    Run {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Cancel a queued job.
    Cancel {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one job with its receipt and output.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List the Project's jobs.
    Jobs {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Mark jobs left `running` by a crash as `interrupted`.
    Recover {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Worker availability and sandbox posture.
    Status {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_compute(cmd: ComputeCmd) -> anyhow::Result<()> {
    match cmd {
        ComputeCmd::Submit {
            vault_id,
            vault_root,
            project_id,
            snapshot_id,
            kind,
            columns,
            sort_by,
            descending,
            process_isolation_only,
            json,
        } => {
            let params = build_params(&kind, columns.as_deref(), sort_by.as_deref(), descending)
                .map_err(|e| invalid(e, json))?;
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .compute_submit(ComputeJobRequest {
                    project_id: OpaqueId::new(project_id),
                    snapshot_id: OpaqueId::new(snapshot_id),
                    params,
                    sandbox: process_isolation_only
                        .then_some(SandboxRequirement::ProcessIsolationOnly),
                    limits: None,
                })
                .map_err(|err| compute_fail(&err, json))?;
            print_view(&view, json)
        }
        ComputeCmd::Run {
            vault_id,
            vault_root,
            job_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .compute_run(OpaqueId::new(job_id))
                .map_err(|err| compute_fail(&err, json))?;
            print_view(&view, json)
        }
        ComputeCmd::Cancel {
            vault_id,
            vault_root,
            job_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .compute_cancel(OpaqueId::new(job_id))
                .map_err(|err| compute_fail(&err, json))?;
            print_view(&view, json)
        }
        ComputeCmd::Show {
            vault_id,
            vault_root,
            job_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let view = s
                .compute_job(OpaqueId::new(job_id))
                .map_err(|err| compute_fail(&err, json))?;
            print_view(&view, json)
        }
        ComputeCmd::Jobs {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let jobs = s
                .compute_jobs(OpaqueId::new(project_id))
                .map_err(|err| compute_fail(&err, json))?;
            if json {
                return print_json_or_debug(&jobs, true);
            }
            for j in &jobs {
                println!(
                    "{}\t{}\t{}\t{}",
                    j.header.id.as_str(),
                    j.manifest.kind.as_str(),
                    j.state.as_str(),
                    j.manifest.requested_snapshot_id.as_str()
                );
            }
            Ok(())
        }
        ComputeCmd::Recover {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let views = s
                .compute_recover()
                .map_err(|err| compute_fail(&err, json))?;
            if json {
                return print_json_or_debug(&views, true);
            }
            println!("recovered: {}", views.len());
            for v in &views {
                println!("{}\t{}", v.job.header.id.as_str(), v.job.state.as_str());
            }
            Ok(())
        }
        ComputeCmd::Status {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, json)?;
            let status = s.compute_status().map_err(|err| compute_fail(&err, json))?;
            if json {
                return print_json_or_debug(&status, true);
            }
            println!("worker_found: {}", status.worker_found);
            println!("protocol_version: {}", status.protocol_version);
            println!("expected_mechanism: {}", status.expected_mechanism.as_str());
            println!("platform_qualified: {}", status.platform_qualified);
            println!(
                "kinds: {}",
                status
                    .kinds
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            println!("queued: {} running: {}", status.queued, status.running);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::compute::{ComputeFailure, ComputeState};
    use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
    use medscale_core::CliSession;

    const VAULT: &str = "vault-085-cli";

    #[test]
    fn params_parse_strictly() {
        assert_eq!(
            build_params("column_profile", None, None, false).unwrap(),
            ComputeParams::ColumnProfile {}
        );
        assert!(build_params("column_profile", Some("a"), None, false).is_err());
        assert!(build_params("shell", None, None, false).is_err());
        assert!(build_params("python", None, None, false).is_err());
        assert_eq!(
            build_params("sorted_projection", Some("a, b"), Some("b"), true).unwrap(),
            ComputeParams::SortedProjection {
                columns: vec!["a".to_owned(), "b".to_owned()],
                sort_by: vec!["b".to_owned()],
                descending: true,
            }
        );
    }

    #[test]
    fn compute_commands_run_through_core_across_fresh_sessions() {
        let root = std::env::temp_dir().join(format!("medscale-085-cli-{}", std::process::id()));
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
        let status = s.compute_status().unwrap();
        assert!(!status.platform_qualified);
        assert!(
            status.worker_found,
            "medscale-compute-worker must be built next to the test binary"
        );
        let submitted = s
            .compute_submit(ComputeJobRequest {
                project_id: project.clone(),
                snapshot_id: snapshot.clone(),
                params: ComputeParams::SortedProjection {
                    columns: vec!["id".to_owned(), "age".to_owned()],
                    sort_by: vec!["age".to_owned()],
                    descending: true,
                },
                sandbox: None,
                limits: None,
            })
            .unwrap();
        assert_eq!(submitted.job.state, ComputeState::Queued);
        let job_id = submitted.job.header.id.clone();
        drop(s);

        // A fresh session runs it; a second run is refused.
        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let ran = s.compute_run(job_id.clone()).unwrap();
        assert_eq!(ran.job.state, ComputeState::Completed, "{:?}", ran.receipt);
        let table = ran.table.as_ref().unwrap();
        let ages: Vec<_> = table.rows.iter().map(|r| r[1].clone()).collect();
        assert_eq!(
            ages,
            vec![
                CellValue::Integer(71),
                CellValue::Integer(58),
                CellValue::Integer(34)
            ]
        );
        let receipt = ran.receipt.as_ref().unwrap();
        let sandbox = receipt.sandbox.as_ref().unwrap();
        assert!(sandbox.ready_base_applied);
        assert!(!sandbox.platform_qualified);
        assert_eq!(sandbox.env_var_count, 0);
        assert!(matches!(
            s.compute_run(job_id.clone()),
            Err(AuthorityError::Conflict { .. })
        ));
        // A queued job cancels; an unknown column is refused at admission.
        let queued = s
            .compute_submit(ComputeJobRequest {
                project_id: project.clone(),
                snapshot_id: snapshot.clone(),
                params: ComputeParams::ColumnProfile {},
                sandbox: None,
                limits: None,
            })
            .unwrap();
        let cancelled = s.compute_cancel(queued.job.header.id.clone()).unwrap();
        assert_eq!(cancelled.job.state, ComputeState::Cancelled);
        assert_eq!(
            cancelled.receipt.unwrap().failure,
            Some(ComputeFailure::CancelledByUser)
        );
        let refused = s
            .compute_submit(ComputeJobRequest {
                project_id: project.clone(),
                snapshot_id: snapshot,
                params: ComputeParams::SortedProjection {
                    columns: vec!["nope".to_owned()],
                    sort_by: vec![],
                    descending: false,
                },
                sandbox: None,
                limits: None,
            })
            .unwrap();
        assert_eq!(refused.job.state, ComputeState::Denied);
        let jobs = s.compute_jobs(project.clone()).unwrap();
        assert_eq!(jobs.len(), 3);
        assert!(s.compute_recover().unwrap().is_empty());
        let shown = s.compute_job(job_id).unwrap();
        assert_eq!(shown.table, ran.table);
        drop(s);

        // Command rendering paths (human and JSON).
        for json in [false, true] {
            run_compute(ComputeCmd::Jobs {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                project_id: project.as_str().to_owned(),
                json,
            })
            .unwrap();
            run_compute(ComputeCmd::Status {
                vault_id: VAULT.to_owned(),
                vault_root: root.clone(),
                json,
            })
            .unwrap();
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}

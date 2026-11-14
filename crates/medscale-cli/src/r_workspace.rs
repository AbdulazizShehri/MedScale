//! Spec 086 R Workspace commands (CLI vertical slice through Core).
//!
//! Every command dispatches typed Core requests via `CliSession`. The
//! staging directory and external programs come from this host's
//! environment (`MEDSCALE_R_STAGE_DIR`, `MEDSCALE_RSTUDIO`,
//! `MEDSCALE_POSITRON`, `MEDSCALE_OPEN_FOLDER`, `MEDSCALE_RSCRIPT`) or the
//! `--stage-dir` flag, never from a request. MedScale does not run R:
//! `run` records the request and reports its refusal.

use std::path::PathBuf;

use clap::Subcommand;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::r_workspace::{
    IdeKind, RLaunchRequest, RPublishRequest, RPublishView, RRunRequest, RStageRequest, RWorkspace,
};
use medscale_core::r_workspace_host::RWorkspaceHost;

use super::{fail_json, print_json_or_debug};

fn r_fail(err: &AuthorityError, json: bool) -> anyhow::Error {
    let debug = format!("{err:?}");
    let (code, message) = match err {
        AuthorityError::Unauthorized
        | AuthorityError::SessionRequired
        | AuthorityError::SessionExpired
        | AuthorityError::SessionRevoked
        | AuthorityError::SessionDenied
        | AuthorityError::WrongScope
        | AuthorityError::PathOutsideClaim => ("denied", debug),
        AuthorityError::NotFound => ("not_found", debug),
        AuthorityError::Conflict { message } => ("conflict", message.clone()),
        AuthorityError::InvalidArgument { message } => ("invalid", message.clone()),
        AuthorityError::Corrupt { message } => ("corrupt", message.clone()),
        AuthorityError::Unavailable { message } => ("unavailable", message.clone()),
        AuthorityError::LeaseRequired | AuthorityError::VaultRequired => ("unavailable", debug),
        _ => ("internal", debug),
    };
    fail_json(code, message, json)
}

fn open_session(
    vault_id: &str,
    vault_root: &std::path::Path,
    stage_dir: Option<PathBuf>,
    json: bool,
) -> anyhow::Result<crate::vault_session::VaultSession> {
    let mut session =
        crate::vault_session::open(vault_id, vault_root).map_err(|err| r_fail(&err, json))?;
    let mut host = RWorkspaceHost::from_env();
    if stage_dir.is_some() {
        host.stage_dir = stage_dir;
    }
    session.set_r_workspace_host(host);
    Ok(session)
}

/// Strict lowercase-or-uppercase 64-digit hex.
fn parse_sha256(hex: &str) -> Result<DigestSha256, String> {
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected 64 hex digits".to_owned());
    }
    let mut out = [0_u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(DigestSha256::from_bytes(out))
}

fn split_ids(raw: &str) -> Vec<OpaqueId> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(OpaqueId::new)
        .collect()
}

fn print_workspace(ws: &RWorkspace, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(ws, true);
    }
    let m = &ws.manifest;
    println!("workspace_id: {}", m.header.id.as_str());
    println!("label: {}", m.label);
    println!("path: {}", m.stage_root);
    println!("data_class: {}", m.data_class.as_str());
    println!("descriptor_sha256: {}", ws.descriptor_digest.to_hex());
    for i in &m.inputs {
        println!(
            "input: {} sha256:{} rows={} class={} -> {}",
            i.snapshot_id.as_str(),
            i.content_digest.to_hex(),
            i.row_count,
            i.data_class.as_str(),
            i.data_file
        );
    }
    Ok(())
}

fn print_publish(v: &RPublishView, json: bool) -> anyhow::Result<()> {
    if json {
        return print_json_or_debug(v, true);
    }
    let r = &v.receipt;
    println!("receipt_id: {}", r.header.id.as_str());
    println!("state: {}", r.state.as_str());
    if let Some(reason) = r.refusal {
        println!("refusal: {}", reason.as_str());
    }
    if let Some(d) = &r.source_digest {
        println!("source_sha256: {}", d.to_hex());
    }
    if let Some(t) = &v.table {
        println!("table_id: {}", t.header.id.as_str());
        println!("table_sha256: {}", t.content_digest.to_hex());
        println!("rows: {} columns: {}", t.row_count, t.column_count);
        println!(
            "data_class: {} reviewed: {}",
            t.data_class.as_str(),
            t.reviewed
        );
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum RCmd {
    /// Stage exact snapshots into a new workspace directory.
    Stage {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        label: String,
        /// Comma-separated snapshot ids.
        #[arg(long)]
        snapshot_ids: String,
        /// Overrides `MEDSCALE_R_STAGE_DIR` (must exist, outside the vault).
        #[arg(long)]
        stage_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Integrity, input currency, lockfile evidence and publishable outputs.
    Inspect {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Open the workspace in RStudio, Positron or the folder opener.
    Open {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        workspace_id: String,
        /// rstudio | positron | folder
        #[arg(long)]
        ide: String,
        #[arg(long)]
        json: bool,
    },
    /// Request a managed `Rscript` run (recorded and refused while the
    /// sandbox is not platform qualified).
    Run {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        workspace_id: String,
        /// A file name in `scripts/`.
        #[arg(long)]
        script: String,
        #[arg(long)]
        json: bool,
    },
    /// Publish one CSV file from `outputs/` as a derived, unreviewed table.
    Publish {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        workspace_id: String,
        /// A file name in `outputs/`.
        #[arg(long)]
        output: String,
        /// Refuse unless the file has exactly this SHA-256.
        #[arg(long)]
        expected_sha256: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// A workspace with its launch, run and publication receipts.
    Show {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        workspace_id: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        json: bool,
    },
    /// A published table with its receipt and content.
    Table {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        table_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Staging and program configuration, and the execution posture.
    Status {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        vault_root: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

pub fn run_r(cmd: RCmd) -> anyhow::Result<()> {
    match cmd {
        RCmd::Stage {
            vault_id,
            vault_root,
            project_id,
            label,
            snapshot_ids,
            stage_dir,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, stage_dir, json)?;
            let ws = s
                .r_stage(RStageRequest {
                    project_id: OpaqueId::new(project_id),
                    label,
                    snapshot_ids: split_ids(&snapshot_ids),
                })
                .map_err(|err| r_fail(&err, json))?;
            print_workspace(&ws, json)
        }
        RCmd::Inspect {
            vault_id,
            vault_root,
            workspace_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let i = s
                .r_inspect(OpaqueId::new(workspace_id))
                .map_err(|err| r_fail(&err, json))?;
            if json {
                return print_json_or_debug(&i, true);
            }
            println!("integrity: {}", i.integrity.as_str());
            println!("inputs_current: {}", i.inputs_current);
            match &i.lockfile {
                Some(l) => println!("renv_lock_sha256: {} bytes={}", l.digest.to_hex(), l.bytes),
                None => println!("renv_lock: absent"),
            }
            for c in &i.candidates {
                println!(
                    "output: {} bytes={} sha256:{}",
                    c.name,
                    c.bytes,
                    c.digest.to_hex()
                );
            }
            println!("skipped: {}", i.skipped);
            Ok(())
        }
        RCmd::Open {
            vault_id,
            vault_root,
            workspace_id,
            ide,
            json,
        } => {
            let ide = IdeKind::parse(&ide).map_err(|e| fail_json("invalid", e, json))?;
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let r = s
                .r_launch(RLaunchRequest {
                    workspace_id: OpaqueId::new(workspace_id),
                    ide,
                })
                .map_err(|err| r_fail(&err, json))?;
            if json {
                return print_json_or_debug(&r, true);
            }
            println!("receipt_id: {}", r.header.id.as_str());
            println!("ide: {}", r.ide.as_str());
            println!("state: {}", r.state.as_str());
            Ok(())
        }
        RCmd::Run {
            vault_id,
            vault_root,
            workspace_id,
            script,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let r = s
                .r_run(RRunRequest {
                    workspace_id: OpaqueId::new(workspace_id),
                    script,
                })
                .map_err(|err| r_fail(&err, json))?;
            if json {
                return print_json_or_debug(&r, true);
            }
            println!("receipt_id: {}", r.header.id.as_str());
            println!("executed: false");
            println!("refusal: {}", r.refusal.as_str());
            if let Some(d) = &r.script_digest {
                println!("script_sha256: {}", d.to_hex());
            }
            Ok(())
        }
        RCmd::Publish {
            vault_id,
            vault_root,
            workspace_id,
            output,
            expected_sha256,
            json,
        } => {
            let expected_digest = expected_sha256
                .as_deref()
                .map(parse_sha256)
                .transpose()
                .map_err(|e| fail_json("invalid", e, json))?;
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let v = s
                .r_publish(RPublishRequest {
                    workspace_id: OpaqueId::new(workspace_id),
                    output_name: output,
                    expected_digest,
                })
                .map_err(|err| r_fail(&err, json))?;
            print_publish(&v, json)
        }
        RCmd::Show {
            vault_id,
            vault_root,
            workspace_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let h = s
                .r_workspace(OpaqueId::new(workspace_id))
                .map_err(|err| r_fail(&err, json))?;
            if json {
                return print_json_or_debug(&h, true);
            }
            print_workspace(&h.workspace, false)?;
            for l in &h.launches {
                println!(
                    "launch: {} {} {}",
                    l.header.id.as_str(),
                    l.ide.as_str(),
                    l.state.as_str()
                );
            }
            for r in &h.runs {
                println!(
                    "run: {} {} refused={}",
                    r.header.id.as_str(),
                    r.script.escape_debug(),
                    r.refusal.as_str()
                );
            }
            for p in &h.publications {
                println!(
                    "publish: {} {} {}",
                    p.header.id.as_str(),
                    p.output_name.escape_debug(),
                    p.state.as_str()
                );
            }
            Ok(())
        }
        RCmd::List {
            vault_id,
            vault_root,
            project_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let list = s
                .r_workspaces(OpaqueId::new(project_id))
                .map_err(|err| r_fail(&err, json))?;
            if json {
                return print_json_or_debug(&list, true);
            }
            for ws in &list {
                println!(
                    "{} {} inputs={} class={}",
                    ws.id().as_str(),
                    ws.manifest.label,
                    ws.manifest.inputs.len(),
                    ws.manifest.data_class.as_str()
                );
            }
            Ok(())
        }
        RCmd::Table {
            vault_id,
            vault_root,
            table_id,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let v = s
                .r_published(OpaqueId::new(table_id))
                .map_err(|err| r_fail(&err, json))?;
            print_publish(&v, json)
        }
        RCmd::Status {
            vault_id,
            vault_root,
            json,
        } => {
            let mut s = open_session(&vault_id, &vault_root, None, json)?;
            let st = s.r_status().map_err(|err| r_fail(&err, json))?;
            if json {
                return print_json_or_debug(&st, true);
            }
            println!("stage_dir_configured: {}", st.stage_dir_configured);
            for (ide, found) in &st.ides {
                println!(
                    "ide {}: {}",
                    ide.as_str(),
                    if *found { "found" } else { "not found" }
                );
            }
            println!("rscript_found: {}", st.runtime.found);
            println!("managed_run_admitted: {}", st.managed_run_admitted);
            println!("platform_qualified: {}", st.platform_qualified);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::data_sources::{LocalFileFormat, SourceLocator};
    use medscale_contracts::r_workspace::{PublishState, RunRefusal, WorkspaceIntegrity};
    use medscale_core::CliSession;

    const VAULT: &str = "vault-086-cli";

    #[test]
    fn digests_and_ids_parse_strictly() {
        let d = DigestSha256::of(b"x");
        assert_eq!(parse_sha256(&d.to_hex()).unwrap(), d);
        assert_eq!(parse_sha256(&d.to_hex().to_uppercase()).unwrap(), d);
        assert!(parse_sha256("abc").is_err());
        assert!(parse_sha256(&"g".repeat(64)).is_err());
        assert!(parse_sha256(&format!("+{}", &d.to_hex()[1..])).is_err());
        assert_eq!(
            split_ids(" snapshot-1, ,snapshot-2 "),
            vec![OpaqueId::new("snapshot-1"), OpaqueId::new("snapshot-2")]
        );
    }

    #[test]
    fn r_commands_run_through_core_across_fresh_sessions() {
        let base = std::env::temp_dir().join(format!("medscale-086-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("vault");
        let stage = base.join("stage");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(root.join("labs.csv"), "id,age\n1,34\n2,71\n").unwrap();
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
        let mut host = RWorkspaceHost::platform_default();
        host.stage_dir = Some(stage.clone());
        s.set_r_workspace_host(host);
        let ws = s
            .r_stage(RStageRequest {
                project_id: project.clone(),
                label: "labs".to_owned(),
                snapshot_ids: vec![snapshot],
            })
            .unwrap();
        let dir = PathBuf::from(&ws.manifest.stage_root);
        assert!(
            std::fs::canonicalize(&dir)
                .unwrap()
                .starts_with(std::fs::canonicalize(&stage).unwrap())
        );
        let status = s.r_status().unwrap();
        assert!(!status.managed_run_admitted && !status.platform_qualified);
        drop(s);

        // A fresh session sees the same workspace through Core.
        let mut s = CliSession::connect(VAULT).unwrap();
        s.open_synthetic_vault(&root.display().to_string()).unwrap();
        let wid = ws.id().clone();
        assert_eq!(
            s.r_inspect(wid.clone()).unwrap().integrity,
            WorkspaceIntegrity::Intact
        );
        std::fs::write(dir.join("scripts").join("a.R"), "print(1)\n").unwrap();
        let run = s
            .r_run(RRunRequest {
                workspace_id: wid.clone(),
                script: "a.R".to_owned(),
            })
            .unwrap();
        assert_eq!(run.refusal, RunRefusal::ComputeDeniedPlatformUnqualified);
        std::fs::write(
            dir.join("outputs").join("means.csv"),
            "group,mean\nall,52.5\n",
        )
        .unwrap();
        let v = s
            .r_publish(RPublishRequest {
                workspace_id: wid.clone(),
                output_name: "means.csv".to_owned(),
                expected_digest: None,
            })
            .unwrap();
        assert_eq!(v.receipt.state, PublishState::Published);
        let table_id = v.table.unwrap().header.id;
        assert_eq!(s.r_workspaces(project).unwrap().len(), 1);
        assert_eq!(s.r_workspace(wid).unwrap().publications.len(), 1);
        drop(s);
        run_r(RCmd::Table {
            vault_id: VAULT.to_owned(),
            vault_root: root.clone(),
            table_id: table_id.as_str().to_owned(),
            json: true,
        })
        .unwrap();
        run_r(RCmd::Status {
            vault_id: VAULT.to_owned(),
            vault_root: root,
            json: true,
        })
        .unwrap();
    }
}

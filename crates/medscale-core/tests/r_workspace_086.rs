//! Spec 086 R Workspace Core authority integration tests.
//!
//! Every call goes through `CliSession` -> `CoreFacade::dispatch`. Inputs
//! are synthetic CSV files imported as Spec 075 snapshots. External
//! programs are played by the qualification harness `medscale-r-fake-ide`,
//! started by Core's launcher. No R is installed or needed: MedScale never
//! runs R in this slice.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use medscale_contracts::compute::resolve_sibling_exe;
use medscale_contracts::data_sources::{CellValue, LocalFileFormat, SourceLocator};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{DigestSha256, OpaqueId};
use medscale_contracts::privacy_gate::DataClass;
use medscale_contracts::r_workspace::{
    DESCRIPTOR_FILE, IdeKind, LaunchState, PUBLISH_BYTES_MAX, PublishRefusal, PublishState,
    RLaunchRequest, RPublishRequest, RPublishView, RRunRequest, RStageRequest, RVersionEvidence,
    RWorkspace, RunRefusal, WorkspaceIntegrity, launch_env_allowed,
};
use medscale_core::CliSession;
use medscale_core::r_workspace_host::RWorkspaceHost;

const LABS: &str = "id,age,ldl,sex\n1,34,3.1,f\n2,71,4.4,m\n3,58,,f\n";
const FAKE_IDE: &str = "medscale-r-fake-ide";

struct Lab {
    s: CliSession,
    base: PathBuf,
    vault_dir: PathBuf,
    stage: PathBuf,
    vault: String,
    project: OpaqueId,
    snapshot: OpaqueId,
}

fn fake_ide() -> PathBuf {
    resolve_sibling_exe(FAKE_IDE)
        .unwrap_or_else(|| panic!("{FAKE_IDE} must be built next to the test binary"))
}

fn host(stage: &Path) -> RWorkspaceHost {
    RWorkspaceHost {
        stage_dir: Some(stage.to_path_buf()),
        programs: [(IdeKind::Folder, fake_ide())].into_iter().collect(),
        rscript: None,
    }
}

fn setup(name: &str) -> Lab {
    let base = std::env::temp_dir().join(format!("medscale-086c-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let vault_dir = base.join("vault");
    let stage = base.join("stage");
    fs::create_dir_all(&vault_dir).unwrap();
    fs::create_dir_all(&stage).unwrap();
    fs::write(vault_dir.join("labs.csv"), LABS).unwrap();
    let vault = format!("vault-086-{name}");
    let mut s = CliSession::connect(&vault).unwrap();
    s.open_synthetic_vault(&vault_dir.display().to_string())
        .unwrap();
    s.set_r_workspace_host(host(&stage));
    let project = s
        .project_create("study".to_owned(), None)
        .unwrap()
        .header
        .id;
    let snapshot = import(&mut s, &project);
    Lab {
        s,
        base,
        vault_dir,
        stage,
        vault,
        project,
        snapshot,
    }
}

fn import(s: &mut CliSession, project: &OpaqueId) -> OpaqueId {
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
    s.snapshot_import(source).unwrap().0.header.id
}

fn reopen(lab: Lab) -> Lab {
    let Lab {
        s,
        base,
        vault_dir,
        stage,
        vault,
        project,
        snapshot,
    } = lab;
    drop(s);
    let mut s = CliSession::connect(&vault).unwrap();
    s.open_synthetic_vault(&vault_dir.display().to_string())
        .unwrap();
    s.set_r_workspace_host(host(&stage));
    Lab {
        s,
        base,
        vault_dir,
        stage,
        vault,
        project,
        snapshot,
    }
}

fn stage(lab: &mut Lab, label: &str) -> RWorkspace {
    lab.s
        .r_stage(RStageRequest {
            project_id: lab.project.clone(),
            label: label.to_owned(),
            snapshot_ids: vec![lab.snapshot.clone()],
        })
        .unwrap()
}

fn root(ws: &RWorkspace) -> PathBuf {
    PathBuf::from(&ws.manifest.stage_root)
}

fn publish(lab: &mut Lab, ws: &RWorkspace, name: &str) -> RPublishView {
    lab.s
        .r_publish(RPublishRequest {
            workspace_id: ws.id().clone(),
            output_name: name.to_owned(),
            expected_digest: None,
        })
        .unwrap()
}

fn refusal(view: &RPublishView) -> Option<PublishRefusal> {
    assert!(view.table.is_none() || view.receipt.state == PublishState::Published);
    view.receipt.refusal
}

fn wait_for(path: &Path) {
    let start = Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{} never appeared",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[allow(clippy::permissions_set_readonly_false)]
fn make_writable(path: &Path) {
    let mut p = fs::metadata(path).unwrap().permissions();
    p.set_readonly(false);
    fs::set_permissions(path, p).unwrap();
}

fn all_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            all_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn staging_writes_exact_read_only_copies_outside_the_vault() {
    let mut lab = setup("stage");
    let ws = stage(&mut lab, "labs");
    let dir = root(&ws);
    let canonical = fs::canonicalize(&dir).unwrap();
    assert!(canonical.starts_with(fs::canonicalize(&lab.stage).unwrap()));
    assert!(!canonical.starts_with(fs::canonicalize(&lab.vault_dir).unwrap()));
    assert!(!ws.manifest.stage_root.starts_with(r"\\?\"));
    for sub in ["data", "scripts", "outputs"] {
        assert!(dir.join(sub).is_dir(), "{sub}");
    }
    let csv_path = dir.join(&ws.manifest.inputs[0].data_file);
    assert_eq!(fs::read_to_string(&csv_path).unwrap(), LABS);
    assert!(fs::metadata(&csv_path).unwrap().permissions().readonly());
    assert_eq!(
        DigestSha256::of(&fs::read(dir.join(DESCRIPTOR_FILE)).unwrap()),
        ws.descriptor_digest
    );
    assert!(dir.join("labs.Rproj").is_file() && dir.join("README.md").is_file());
    // Unclassified snapshots count as local PHI, and so does the workspace.
    assert_eq!(ws.manifest.inputs[0].data_class, DataClass::LocalPhi);
    assert_eq!(ws.manifest.data_class, DataClass::LocalPhi);

    // No staged file names the vault, its database or its key material.
    let vault = fs::canonicalize(&lab.vault_dir).unwrap();
    let vault_text = vault.to_string_lossy().into_owned();
    let mut files = Vec::new();
    all_files(&dir, &mut files);
    assert_eq!(files.len(), ws.manifest.files.len() + 1);
    for file in files {
        let text = String::from_utf8_lossy(&fs::read(&file).unwrap()).into_owned();
        assert!(!text.contains(&vault_text), "{}", file.display());
        assert!(!text.contains("meta.sqlite3"), "{}", file.display());
    }

    let inspection = lab.s.r_inspect(ws.id().clone()).unwrap();
    assert_eq!(inspection.integrity, WorkspaceIntegrity::Intact);
    assert!(inspection.inputs_current);
    assert!(inspection.candidates.is_empty() && inspection.lockfile.is_none());
}

#[test]
fn staging_is_deterministic_across_workspaces() {
    let mut lab = setup("determinism");
    let a = stage(&mut lab, "labs");
    let b = stage(&mut lab, "labs");
    assert_ne!(a.id(), b.id());
    assert_ne!(root(&a), root(&b));
    assert_eq!(a.manifest.files, b.manifest.files);
    assert_eq!(a.manifest.inputs, b.manifest.inputs);
}

#[test]
fn staging_refusals_create_nothing() {
    let mut lab = setup("refusals");
    let request = |label: &str, ids: Vec<OpaqueId>| RStageRequest {
        project_id: lab.project.clone(),
        label: label.to_owned(),
        snapshot_ids: ids,
    };
    let snap = lab.snapshot.clone();
    for (label, ids) in [
        ("../escape", vec![snap.clone()]),
        ("", vec![snap.clone()]),
        ("labs", vec![]),
        ("labs", vec![snap.clone(), snap.clone()]),
    ] {
        assert!(
            matches!(
                lab.s.r_stage(request(label, ids)),
                Err(AuthorityError::InvalidArgument { .. })
            ),
            "{label}"
        );
    }
    assert!(matches!(
        lab.s
            .r_stage(request("labs", vec![OpaqueId::new("snapshot-999")])),
        Err(AuthorityError::NotFound)
    ));
    // Another Project's snapshot is not found in this one.
    let other = lab
        .s
        .project_create("other".to_owned(), None)
        .unwrap()
        .header
        .id;
    assert!(matches!(
        lab.s.r_stage(RStageRequest {
            project_id: other.clone(),
            label: "labs".to_owned(),
            snapshot_ids: vec![snap.clone()],
        }),
        Err(AuthorityError::NotFound)
    ));

    // Host configuration: missing, nonexistent, or inside the vault.
    let mut h = host(&lab.stage);
    h.stage_dir = None;
    lab.s.set_r_workspace_host(h.clone());
    assert!(matches!(
        lab.s.r_stage(request("labs", vec![snap.clone()])),
        Err(AuthorityError::Unavailable { .. })
    ));
    h.stage_dir = Some(lab.base.join("no-such-dir"));
    lab.s.set_r_workspace_host(h.clone());
    assert!(matches!(
        lab.s.r_stage(request("labs", vec![snap.clone()])),
        Err(AuthorityError::Unavailable { .. })
    ));
    let inside = lab.vault_dir.join("staging");
    fs::create_dir_all(&inside).unwrap();
    h.stage_dir = Some(inside.clone());
    lab.s.set_r_workspace_host(h.clone());
    assert!(matches!(
        lab.s.r_stage(request("labs", vec![snap.clone()])),
        Err(AuthorityError::PathOutsideClaim)
    ));
    h.stage_dir = Some(lab.vault_dir.clone());
    lab.s.set_r_workspace_host(h);
    assert!(matches!(
        lab.s.r_stage(request("labs", vec![snap])),
        Err(AuthorityError::PathOutsideClaim)
    ));
    assert_eq!(fs::read_dir(&inside).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&lab.stage).unwrap().count(), 0);
    assert!(lab.s.r_workspaces(lab.project.clone()).unwrap().is_empty());
    assert!(lab.s.r_workspaces(other).unwrap().is_empty());
}

#[test]
fn external_launch_gets_the_workspace_and_an_allowlisted_environment_only() {
    let mut lab = setup("launch");
    let ws = stage(&mut lab, "labs");
    let dir = root(&ws);
    let receipt = lab
        .s
        .r_launch(RLaunchRequest {
            workspace_id: ws.id().clone(),
            ide: IdeKind::Folder,
        })
        .unwrap();
    assert_eq!(receipt.state, LaunchState::Launched, "{receipt:?}");
    wait_for(&dir.join("outputs").join("done.marker"));
    let seen = fs::read_to_string(dir.join("outputs").join("ide-env.txt")).unwrap();
    let seen: Vec<&str> = seen.lines().filter(|l| !l.is_empty()).collect();
    for name in &seen {
        assert!(launch_env_allowed(name), "{name} leaked into the launch");
    }
    let mut passed = receipt.env_names.clone();
    passed.sort();
    let mut seen_sorted: Vec<String> = seen.iter().map(|s| (*s).to_owned()).collect();
    seen_sorted.sort();
    // Windows may add a few process variables of its own; everything the
    // program saw that MedScale passed is on the allowlist either way.
    assert!(passed.iter().all(|p| seen_sorted.contains(p)));
    let inherited = std::env::vars_os()
        .filter_map(|(k, _)| k.into_string().ok())
        .filter(|k| !launch_env_allowed(k))
        .count();
    assert!(inherited > 0, "the test process has variables to strip");

    // The external program's output returns only when published.
    let inspection = lab.s.r_inspect(ws.id().clone()).unwrap();
    let names: Vec<&str> = inspection
        .candidates
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(names.contains(&"result.csv"), "{names:?}");
    let view = publish(&mut lab, &ws, "result.csv");
    assert_eq!(view.receipt.state, PublishState::Published, "{view:?}");
    let table = view.table.as_ref().unwrap();
    assert_eq!(table.derived_from, vec![lab.snapshot.clone()]);
    assert_eq!(table.data_class, DataClass::LocalPhi);
    assert!(!table.reviewed);
    assert_eq!((table.row_count, table.column_count), (3, 4));
    let content = view.content.as_ref().unwrap();
    assert_eq!(content.rows[1][1], CellValue::Integer(71));
    assert_eq!(content.rows[2][2], CellValue::Null);
    assert_eq!(
        lab.s.r_published(table.header.id.clone()).unwrap().content,
        view.content
    );

    // A digest the user did not inspect is refused.
    let changed = lab
        .s
        .r_publish(RPublishRequest {
            workspace_id: ws.id().clone(),
            output_name: "result.csv".to_owned(),
            expected_digest: Some(DigestSha256::of(b"something else")),
        })
        .unwrap();
    assert_eq!(refusal(&changed), Some(PublishRefusal::OutputChanged));
    assert!(changed.table.is_none());

    // Unconfigured, missing and relative programs never start.
    let mut h = host(&lab.stage);
    h.programs.remove(&IdeKind::Folder);
    h.programs
        .insert(IdeKind::Rstudio, lab.base.join("no-such-rstudio"));
    h.programs
        .insert(IdeKind::Positron, PathBuf::from(FAKE_IDE));
    lab.s.set_r_workspace_host(h);
    for (ide, expected) in [
        (IdeKind::Folder, LaunchState::IdeNotConfigured),
        (IdeKind::Rstudio, LaunchState::IdeNotFound),
        (IdeKind::Positron, LaunchState::IdeNotFound),
    ] {
        let r = lab
            .s
            .r_launch(RLaunchRequest {
                workspace_id: ws.id().clone(),
                ide,
            })
            .unwrap();
        assert_eq!(r.state, expected);
        assert!(r.env_names.is_empty());
    }
    let history = lab.s.r_workspace(ws.id().clone()).unwrap();
    assert_eq!(history.launches.len(), 4);
    assert_eq!(history.publications.len(), 2);
}

#[test]
fn managed_runs_are_recorded_and_refused_never_executed() {
    let mut lab = setup("run");
    let ws = stage(&mut lab, "labs");
    let dir = root(&ws);
    // The configured "Rscript" is a program that would write outputs if it
    // ran; the refusal must leave no trace of it.
    let mut h = host(&lab.stage);
    h.rscript = Some(fake_ide());
    lab.s.set_r_workspace_host(h);
    let script = "writeLines('x', 'outputs/pwned.txt')\n";
    fs::write(dir.join("scripts").join("analysis.R"), script).unwrap();
    fs::write(dir.join("renv.lock"), "{\"R\":{\"Version\":\"4.4.1\"}}\n").unwrap();
    let r = lab
        .s
        .r_run(RRunRequest {
            workspace_id: ws.id().clone(),
            script: "analysis.R".to_owned(),
        })
        .unwrap();
    assert_eq!(r.refusal, RunRefusal::ComputeDeniedPlatformUnqualified);
    assert_eq!(r.script_digest, Some(DigestSha256::of(script.as_bytes())));
    assert!(r.runtime.found && r.runtime.program_digest.is_some());
    assert_eq!(r.runtime.version, RVersionEvidence::NotProbed);
    assert!(r.lockfile.is_some());
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(fs::read_dir(dir.join("outputs")).unwrap().count(), 0);

    for bad in ["../analysis.R", "analysis.py", "missing.R", ""] {
        let r = lab
            .s
            .r_run(RRunRequest {
                workspace_id: ws.id().clone(),
                script: bad.to_owned(),
            })
            .unwrap();
        assert_eq!(r.refusal, RunRefusal::ScriptInvalid, "{bad}");
        assert!(r.script_digest.is_none());
    }
    let long = "a".repeat(10_000);
    let r = lab
        .s
        .r_run(RRunRequest {
            workspace_id: ws.id().clone(),
            script: long,
        })
        .unwrap();
    assert_eq!(r.refusal, RunRefusal::ScriptInvalid);
    assert!(r.script.len() <= 128);
    // Six refusals everywhere, plus the linked script on Unix.
    let runs = 6 + usize::from(cfg!(unix));
    #[cfg(unix)]
    {
        let outside = lab.base.join("outside.R");
        fs::write(&outside, "1\n").unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("scripts").join("linked.R")).unwrap();
        let r = lab
            .s
            .r_run(RRunRequest {
                workspace_id: ws.id().clone(),
                script: "linked.R".to_owned(),
            })
            .unwrap();
        assert_eq!(r.refusal, RunRefusal::ScriptInvalid);
    }
    let status = lab.s.r_status().unwrap();
    assert!(!status.managed_run_admitted && !status.platform_qualified);
    assert_eq!(lab.s.r_workspace(ws.id().clone()).unwrap().runs.len(), runs);
}

#[test]
fn publication_refuses_unsafe_or_inexact_outputs() {
    let mut lab = setup("publish");
    let ws = stage(&mut lab, "labs");
    let out = root(&ws).join("outputs");
    fs::create_dir(out.join("dir.csv")).unwrap();
    fs::write(out.join("latin1.csv"), b"name\n\xe9t\xe9\n").unwrap();
    fs::write(out.join("ragged.csv"), "a,b\n1,2\n3\n").unwrap();
    fs::write(out.join("header.csv"), "a,b\n").unwrap();
    fs::write(out.join("dup.csv"), "a,a\n1,2\n").unwrap();
    let big = vec![b'x'; usize::try_from(PUBLISH_BYTES_MAX).unwrap() + 1];
    fs::write(out.join("big.csv"), big).unwrap();
    fs::write(out.join("ok.csv"), "k,v\na,1\n").unwrap();
    let cases = [
        ("../medscale-workspace.json", PublishRefusal::BadName),
        ("data/x.csv", PublishRefusal::BadName),
        (".hidden", PublishRefusal::BadName),
        ("C:\\x.csv", PublishRefusal::BadName),
        ("absent.csv", PublishRefusal::NotFound),
        ("dir.csv", PublishRefusal::NotRegularFile),
        ("latin1.csv", PublishRefusal::OutputInvalid),
        ("ragged.csv", PublishRefusal::OutputInvalid),
        ("header.csv", PublishRefusal::OutputInvalid),
        ("dup.csv", PublishRefusal::OutputInvalid),
        ("big.csv", PublishRefusal::TooLarge),
    ];
    for (name, expected) in cases {
        let view = publish(&mut lab, &ws, name);
        assert_eq!(refusal(&view), Some(expected), "{name}");
        assert!(view.table.is_none() && view.content.is_none());
        assert_eq!((view.receipt.row_count, view.receipt.column_count), (0, 0));
    }
    #[cfg(unix)]
    {
        let secret = lab.base.join("secret.csv");
        fs::write(&secret, "k,v\nsecret,1\n").unwrap();
        std::os::unix::fs::symlink(&secret, out.join("link.csv")).unwrap();
        let view = publish(&mut lab, &ws, "link.csv");
        assert_eq!(refusal(&view), Some(PublishRefusal::NotRegularFile));
        let inspection = lab.s.r_inspect(ws.id().clone()).unwrap();
        assert!(inspection.candidates.iter().all(|c| c.name != "link.csv"));
    }
    let view = publish(&mut lab, &ws, "ok.csv");
    assert_eq!(view.receipt.state, PublishState::Published);
    let history = lab.s.r_workspace(ws.id().clone()).unwrap();
    assert_eq!(
        history
            .publications
            .iter()
            .filter(|p| p.state == PublishState::Published)
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn a_linked_outputs_directory_is_not_followed() {
    let mut lab = setup("outputs-link");
    let ws = stage(&mut lab, "labs");
    let dir = root(&ws);
    let elsewhere = lab.base.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    fs::write(elsewhere.join("x.csv"), "k,v\na,1\n").unwrap();
    fs::remove_dir(dir.join("outputs")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, dir.join("outputs")).unwrap();
    let view = publish(&mut lab, &ws, "x.csv");
    assert_eq!(refusal(&view), Some(PublishRefusal::NotRegularFile));
    assert!(
        lab.s
            .r_inspect(ws.id().clone())
            .unwrap()
            .candidates
            .is_empty()
    );
}

#[test]
fn a_changed_or_missing_workspace_blocks_launch_run_and_publish() {
    let mut lab = setup("tamper");
    let ws = stage(&mut lab, "labs");
    let dir = root(&ws);
    fs::write(dir.join("outputs").join("ok.csv"), "k,v\na,1\n").unwrap();
    fs::write(dir.join("scripts").join("a.R"), "1\n").unwrap();
    let csv = dir.join(&ws.manifest.inputs[0].data_file);
    make_writable(&csv);
    fs::write(&csv, "id,age,ldl,sex\n1,99,3.1,f\n").unwrap();
    assert_eq!(
        lab.s.r_inspect(ws.id().clone()).unwrap().integrity,
        WorkspaceIntegrity::Changed
    );
    assert_eq!(
        refusal(&publish(&mut lab, &ws, "ok.csv")),
        Some(PublishRefusal::WorkspaceChanged)
    );
    let launch = lab
        .s
        .r_launch(RLaunchRequest {
            workspace_id: ws.id().clone(),
            ide: IdeKind::Folder,
        })
        .unwrap();
    assert_eq!(launch.state, LaunchState::WorkspaceChanged);
    let run = lab
        .s
        .r_run(RRunRequest {
            workspace_id: ws.id().clone(),
            script: "a.R".to_owned(),
        })
        .unwrap();
    assert_eq!(run.refusal, RunRefusal::WorkspaceChanged);

    // A second workspace whose descriptor is edited is also changed.
    let other = stage(&mut lab, "other");
    fs::write(root(&other).join(DESCRIPTOR_FILE), "{}").unwrap();
    assert_eq!(
        lab.s.r_inspect(other.id().clone()).unwrap().integrity,
        WorkspaceIntegrity::Changed
    );

    // A removed workspace is missing; its records remain.
    fs::rename(&dir, lab.base.join("moved")).unwrap();
    assert_eq!(
        lab.s.r_inspect(ws.id().clone()).unwrap().integrity,
        WorkspaceIntegrity::Missing
    );
    assert_eq!(
        refusal(&publish(&mut lab, &ws, "ok.csv")),
        Some(PublishRefusal::WorkspaceMissing)
    );
    assert_eq!(
        lab.s
            .r_workspace(ws.id().clone())
            .unwrap()
            .publications
            .len(),
        2
    );
}

#[test]
fn a_stale_input_blocks_publication() {
    let mut lab = setup("stale");
    let ws = stage(&mut lab, "labs");
    fs::write(root(&ws).join("outputs").join("ok.csv"), "k,v\na,1\n").unwrap();
    let conn = rusqlite::Connection::open(lab.vault_dir.join("meta.sqlite3")).unwrap();
    conn.execute(
        "UPDATE data_snapshots SET content_digest_hex = ?1 WHERE snapshot_id = ?2",
        rusqlite::params!["0".repeat(64), lab.snapshot.as_str()],
    )
    .unwrap();
    drop(conn);
    assert!(!lab.s.r_inspect(ws.id().clone()).unwrap().inputs_current);
    assert_eq!(
        refusal(&publish(&mut lab, &ws, "ok.csv")),
        Some(PublishRefusal::InputStale)
    );
}

#[test]
fn workspaces_and_publications_survive_restart() {
    let mut lab = setup("restart");
    let ws = stage(&mut lab, "labs");
    fs::write(root(&ws).join("outputs").join("ok.csv"), "k,v\na,1\n").unwrap();
    let view = publish(&mut lab, &ws, "ok.csv");
    let table_id = view.table.as_ref().unwrap().header.id.clone();
    let mut lab = reopen(lab);
    let history = lab.s.r_workspace(ws.id().clone()).unwrap();
    assert_eq!(history.workspace, ws);
    assert_eq!(history.publications, vec![view.receipt.clone()]);
    assert_eq!(
        lab.s.r_inspect(ws.id().clone()).unwrap().integrity,
        WorkspaceIntegrity::Intact
    );
    assert_eq!(lab.s.r_published(table_id).unwrap().content, view.content);
    assert_eq!(lab.s.r_workspaces(lab.project.clone()).unwrap().len(), 1);
    assert!(matches!(
        lab.s.r_inspect(OpaqueId::new("r-workspace-99")),
        Err(AuthorityError::NotFound)
    ));
}

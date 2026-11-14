//! Spec 085 MedScale Compute Core authority integration tests.
//!
//! Every call goes through `CliSession` -> `CoreFacade::dispatch`. Inputs are
//! synthetic CSV files imported as Spec 075 snapshots. Jobs run in the real
//! `medscale-compute-worker`; fault cases name the qualification harness
//! `medscale-compute-fault-worker` through an explicit runtime.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use medscale_contracts::compute::{
    COMPUTE_WORKER_NAME, ComputeDenyReason, ComputeFailure, ComputeJobRequest, ComputeJobView,
    ComputeParams, ComputeState, OutputReview, PROFILE_COLUMNS, ResourceCeilings, SandboxMechanism,
    SandboxRequirement, resolve_sibling_exe,
};
use medscale_contracts::data_sources::{CellValue, LocalFileFormat, SourceLocator};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::OpaqueId;
use medscale_core::CliSession;
use medscale_core::compute_supervisor::ComputeRuntime;

const LABS: &str = "id,age,ldl,sex\n1,34,3.1,f\n2,71,4.4,m\n3,58,,f\n4,45,5.0,m\n";
const FAULT_WORKER: &str = "medscale-compute-fault-worker";

struct Lab {
    s: CliSession,
    dir: PathBuf,
    vault: String,
    project: OpaqueId,
    snapshot: OpaqueId,
}

fn setup(name: &str) -> Lab {
    let dir = std::env::temp_dir().join(format!("medscale-085c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("labs.csv"), LABS).unwrap();
    let vault = format!("vault-085-{name}");
    let mut s = CliSession::connect(&vault).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    let project = s
        .project_create("study".to_owned(), None)
        .unwrap()
        .header
        .id;
    let snapshot = import(&mut s, &project, "labs");
    Lab {
        s,
        dir,
        vault,
        project,
        snapshot,
    }
}

fn import(s: &mut CliSession, project: &OpaqueId, name: &str) -> OpaqueId {
    let source = s
        .data_source_create(
            project.clone(),
            name.to_owned(),
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
        dir,
        vault,
        project,
        snapshot,
    } = lab;
    drop(s);
    let mut s = CliSession::connect(&vault).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    Lab {
        s,
        dir,
        vault,
        project,
        snapshot,
    }
}

fn projection(columns: &[&str], sort_by: &[&str], descending: bool) -> ComputeParams {
    ComputeParams::SortedProjection {
        columns: columns.iter().map(|c| (*c).to_owned()).collect(),
        sort_by: sort_by.iter().map(|c| (*c).to_owned()).collect(),
        descending,
    }
}

fn request(lab: &Lab, params: ComputeParams) -> ComputeJobRequest {
    ComputeJobRequest {
        project_id: lab.project.clone(),
        snapshot_id: lab.snapshot.clone(),
        params,
        sandbox: None,
        limits: None,
    }
}

fn submit_and_run(lab: &mut Lab, req: ComputeJobRequest) -> ComputeJobView {
    let submitted = lab.s.compute_submit(req).unwrap();
    assert_eq!(submitted.job.state, ComputeState::Queued, "{submitted:?}");
    lab.s.compute_run(submitted.job.header.id).unwrap()
}

fn fault(mode: &str) -> ComputeRuntime {
    let exe = resolve_sibling_exe(FAULT_WORKER)
        .unwrap_or_else(|| panic!("{FAULT_WORKER} must be built next to the test binary"));
    ComputeRuntime::with_worker(exe, vec![mode.to_owned()])
}

fn assert_no_output(view: &ComputeJobView) {
    assert!(view.output.is_none() && view.table.is_none(), "{view:?}");
    let r = view.receipt.as_ref().unwrap();
    assert!(r.output_id.is_none() && r.output_digest.is_none());
    assert_eq!((r.row_count, r.column_count), (0, 0));
}

fn int(i: i64) -> CellValue {
    CellValue::Integer(i)
}

#[test]
fn jobs_run_in_the_worker_and_match_hand_computed_fixtures() {
    assert!(
        resolve_sibling_exe(COMPUTE_WORKER_NAME).is_some(),
        "{COMPUTE_WORKER_NAME} must be built next to the test binary"
    );
    let mut lab = setup("fixtures");
    let before = lab.s.snapshot_get(lab.snapshot.clone()).unwrap();

    let req = request(&lab, ComputeParams::ColumnProfile {});

    let profile = submit_and_run(&mut lab, req);
    assert_eq!(profile.job.state, ComputeState::Completed, "{profile:?}");
    let table = profile.table.clone().unwrap();
    let names: Vec<_> = table.columns.iter().map(|c| c.name.as_str()).collect();
    let expected: Vec<_> = PROFILE_COLUMNS.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, expected);
    let text = |t: &str| CellValue::Text(t.to_owned());
    assert_eq!(
        table.rows,
        vec![
            vec![
                text("id"),
                text("integer"),
                int(4),
                int(0),
                int(4),
                int(1),
                int(4),
                CellValue::Float(2.5)
            ],
            vec![
                text("age"),
                text("integer"),
                int(4),
                int(0),
                int(4),
                int(34),
                int(71),
                CellValue::Float(52.0)
            ],
            vec![
                text("ldl"),
                text("float"),
                int(3),
                int(1),
                int(3),
                CellValue::Float(3.1),
                CellValue::Float(5.0),
                CellValue::Float((3.1 + 4.4 + 5.0) / 3.0)
            ],
            vec![
                text("sex"),
                text("text"),
                int(4),
                int(0),
                int(2),
                text("f"),
                text("m"),
                CellValue::Null
            ],
        ]
    );

    let req = request(&lab, projection(&["id", "ldl"], &["ldl"], true));
    let sorted = submit_and_run(&mut lab, req);
    assert_eq!(sorted.job.state, ComputeState::Completed, "{sorted:?}");
    let ids: Vec<_> = sorted
        .table
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .map(|r| r[0].clone())
        .collect();
    // Null is the lowest value, so it is last when descending.
    assert_eq!(ids, vec![int(4), int(2), int(1), int(3)]);

    // Receipt, output and pinned input agree; the output is a derived,
    // unreviewed artifact of exactly the pinned snapshot.
    let r = profile.receipt.as_ref().unwrap();
    let o = profile.output.as_ref().unwrap();
    let input = profile.job.manifest.input.as_ref().unwrap();
    assert_eq!(input.content_digest, before.content_digest);
    assert_eq!(input.row_count, 4);
    assert_eq!(r.output_digest.as_ref(), Some(&table.digest()));
    assert_eq!(o.content_digest, table.digest());
    assert_eq!(o.derived_from, lab.snapshot);
    assert_eq!(o.input_digest, before.content_digest);
    assert_eq!(o.review, OutputReview::Unreviewed);
    let sandbox = r.sandbox.as_ref().unwrap();
    assert!(sandbox.ready_base_applied, "{sandbox:?}");
    assert_eq!(sandbox.mechanism, SandboxMechanism::for_this_os());
    assert!(!sandbox.platform_qualified);
    assert_eq!(sandbox.env_var_count, 0);
    assert_eq!(r.stderr_bytes, 0);
    assert!(r.stderr_prefix_digest.is_none());
    assert_eq!(
        profile.job.manifest.policy.sandbox,
        SandboxRequirement::ReadyBaseRequired
    );

    // Deterministic: the same job again yields byte-identical output.
    let req = request(&lab, ComputeParams::ColumnProfile {});
    let again = submit_and_run(&mut lab, req);
    assert_eq!(
        again.output.as_ref().unwrap().content_digest,
        o.content_digest
    );

    // The source snapshot is unchanged, and the stored output reads back.
    assert_eq!(lab.s.snapshot_get(lab.snapshot.clone()).unwrap(), before);
    let mut lab = reopen(lab);
    let shown = lab.s.compute_job(profile.job.header.id.clone()).unwrap();
    assert_eq!(shown.table, profile.table);
    assert_eq!(shown.receipt, profile.receipt);
    assert_eq!(lab.s.compute_jobs(lab.project.clone()).unwrap().len(), 3);
    let status = lab.s.compute_status().unwrap();
    assert!(status.worker_found);
    assert!(!status.platform_qualified);
    assert_eq!((status.queued, status.running), (0, 0));
}

#[test]
fn admission_refuses_with_receipts_and_runs_nothing() {
    let mut lab = setup("admission");
    let other_project = lab
        .s
        .project_create("other".to_owned(), None)
        .unwrap()
        .header
        .id;
    let foreign = import(&mut lab.s, &other_project, "other-labs");
    let cases: Vec<(ComputeJobRequest, ComputeDenyReason)> = vec![
        (
            ComputeJobRequest {
                snapshot_id: OpaqueId::new("snapshot-999"),
                ..request(&lab, ComputeParams::ColumnProfile {})
            },
            ComputeDenyReason::InputUnavailable,
        ),
        (
            ComputeJobRequest {
                snapshot_id: foreign,
                ..request(&lab, ComputeParams::ColumnProfile {})
            },
            ComputeDenyReason::InputUnavailable,
        ),
        (
            request(&lab, projection(&["id", "nope"], &[], false)),
            ComputeDenyReason::UnknownColumn,
        ),
        (
            request(&lab, projection(&["id"], &["../etc/passwd"], false)),
            ComputeDenyReason::UnknownColumn,
        ),
        (
            request(&lab, projection(&["id", "id"], &[], false)),
            ComputeDenyReason::BadParams,
        ),
        (
            request(&lab, projection(&[], &[], false)),
            ComputeDenyReason::BadParams,
        ),
        (
            ComputeJobRequest {
                limits: Some(ResourceCeilings {
                    timeout_ms: 1,
                    ..ResourceCeilings::default()
                }),
                ..request(&lab, ComputeParams::ColumnProfile {})
            },
            ComputeDenyReason::BadLimits,
        ),
        (
            ComputeJobRequest {
                limits: Some(ResourceCeilings {
                    max_input_bytes: 16,
                    ..ResourceCeilings::default()
                }),
                ..request(&lab, ComputeParams::ColumnProfile {})
            },
            ComputeDenyReason::InputTooLarge,
        ),
    ];
    for (req, reason) in cases {
        let view = lab.s.compute_submit(req).unwrap();
        assert_eq!(view.job.state, ComputeState::Denied, "{view:?}");
        let r = view.receipt.as_ref().unwrap();
        assert_eq!(r.deny_reason, Some(reason));
        assert!(r.sandbox.is_none(), "nothing ran");
        assert_no_output(&view);
        // A refused job never runs.
        assert!(matches!(
            lab.s.compute_run(view.job.header.id.clone()),
            Err(AuthorityError::Conflict { .. })
        ));
    }
    // Another Project's input is reported exactly like a missing one.
    let jobs = lab.s.compute_jobs(lab.project.clone()).unwrap();
    assert!(jobs[0].manifest.input.is_none() && jobs[1].manifest.input.is_none());
    // An unknown Project is refused outright.
    assert!(
        lab.s
            .compute_submit(ComputeJobRequest {
                project_id: OpaqueId::new("project-999"),
                ..request(&lab, ComputeParams::ColumnProfile {})
            })
            .is_err()
    );
    assert!(lab.s.compute_jobs(other_project).unwrap().is_empty());
}

#[test]
fn a_job_runs_at_most_once_and_cancelled_jobs_never_run() {
    let mut lab = setup("once");
    let req = request(&lab, ComputeParams::ColumnProfile {});
    let done = submit_and_run(&mut lab, req);
    assert_eq!(done.job.state, ComputeState::Completed);
    let id = done.job.header.id.clone();
    for _ in 0..2 {
        assert!(matches!(
            lab.s.compute_run(id.clone()),
            Err(AuthorityError::Conflict { .. })
        ));
    }
    assert!(matches!(
        lab.s.compute_cancel(id.clone()),
        Err(AuthorityError::Conflict { .. })
    ));
    let shown = lab.s.compute_job(id).unwrap();
    assert_eq!(shown.receipt, done.receipt);

    let queued = lab
        .s
        .compute_submit(request(&lab, ComputeParams::ColumnProfile {}))
        .unwrap();
    let cancelled = lab.s.compute_cancel(queued.job.header.id.clone()).unwrap();
    assert_eq!(cancelled.job.state, ComputeState::Cancelled);
    assert_eq!(
        cancelled.receipt.as_ref().unwrap().failure,
        Some(ComputeFailure::CancelledByUser)
    );
    assert_no_output(&cancelled);
    assert!(matches!(
        lab.s.compute_run(queued.job.header.id),
        Err(AuthorityError::Conflict { .. })
    ));
    assert!(matches!(
        lab.s.compute_run(OpaqueId::new("compute-job-999")),
        Err(AuthorityError::NotFound)
    ));
}

fn blob_path(dir: &std::path::Path, digest_hex: &str) -> PathBuf {
    dir.join("blobs").join(digest_hex)
}

#[test]
fn inputs_are_re_verified_before_staging() {
    let mut lab = setup("reverify");
    // Input replacement after submission: the stored bytes change.
    let replaced = lab
        .s
        .compute_submit(request(&lab, ComputeParams::ColumnProfile {}))
        .unwrap();
    let digest = replaced
        .job
        .manifest
        .input
        .as_ref()
        .unwrap()
        .content_digest
        .to_hex();
    let path = blob_path(&lab.dir, &digest);
    let original = std::fs::read(&path).unwrap();
    let mut forged = original.clone();
    let last = forged.len() - 2;
    forged[last] ^= 0x01;
    std::fs::write(&path, &forged).unwrap();
    let view = lab.s.compute_run(replaced.job.header.id.clone()).unwrap();
    assert_eq!(view.job.state, ComputeState::Corrupt, "{view:?}");
    assert_eq!(
        view.receipt.as_ref().unwrap().failure,
        Some(ComputeFailure::InputDigestMismatch)
    );
    assert!(
        view.receipt.as_ref().unwrap().sandbox.is_none(),
        "no worker ran"
    );
    assert_no_output(&view);
    std::fs::write(&path, &original).unwrap();

    // Stale reference: the snapshot is no longer this Project's.
    let stale = lab
        .s
        .compute_submit(request(&lab, ComputeParams::ColumnProfile {}))
        .unwrap();
    let other = lab
        .s
        .project_create("elsewhere".to_owned(), None)
        .unwrap()
        .header
        .id;
    let mut lab = {
        let Lab {
            s,
            dir,
            vault,
            project,
            snapshot,
        } = lab;
        drop(s);
        let conn = rusqlite::Connection::open(dir.join("meta.sqlite3")).unwrap();
        conn.execute(
            "UPDATE data_snapshots SET project_id = ?1 WHERE snapshot_id = ?2",
            rusqlite::params![other.as_str(), snapshot.as_str()],
        )
        .unwrap();
        drop(conn);
        let mut s = CliSession::connect(&vault).unwrap();
        s.open_synthetic_vault(&dir.display().to_string()).unwrap();
        Lab {
            s,
            dir,
            vault,
            project,
            snapshot,
        }
    };
    let view = lab.s.compute_run(stale.job.header.id).unwrap();
    assert_eq!(view.job.state, ComputeState::Denied, "{view:?}");
    assert_eq!(
        view.receipt.as_ref().unwrap().deny_reason,
        Some(ComputeDenyReason::InputUnavailable)
    );
    assert_no_output(&view);
}

#[test]
fn faulty_workers_are_contained_and_commit_nothing() {
    let mut lab = setup("faults");
    let small = ResourceCeilings {
        max_output_bytes: 4 * 1024,
        timeout_ms: 3_000,
        ..ResourceCeilings::default()
    };
    let cases: Vec<(&str, ComputeParams, ComputeState, ComputeFailure)> = vec![
        (
            "hang",
            ComputeParams::ColumnProfile {},
            ComputeState::TimedOut,
            ComputeFailure::TimeLimit,
        ),
        (
            "flood-stdout",
            ComputeParams::ColumnProfile {},
            ComputeState::ResourceExhausted,
            ComputeFailure::OutputTooLarge,
        ),
        (
            "crash",
            ComputeParams::ColumnProfile {},
            ComputeState::Failed,
            ComputeFailure::WorkerCrashed,
        ),
        (
            "partial",
            ComputeParams::ColumnProfile {},
            ComputeState::Corrupt,
            ComputeFailure::MalformedProtocol,
        ),
        (
            "malformed",
            ComputeParams::ColumnProfile {},
            ComputeState::Corrupt,
            ComputeFailure::MalformedProtocol,
        ),
        (
            "trailing",
            ComputeParams::ColumnProfile {},
            ComputeState::Corrupt,
            ComputeFailure::MalformedProtocol,
        ),
        (
            "wrong-job",
            ComputeParams::ColumnProfile {},
            ComputeState::Corrupt,
            ComputeFailure::JobMismatch,
        ),
        (
            "wrong-digest",
            ComputeParams::ColumnProfile {},
            ComputeState::Corrupt,
            ComputeFailure::OutputDigestMismatch,
        ),
        (
            "noncanonical",
            ComputeParams::ColumnProfile {},
            ComputeState::Corrupt,
            ComputeFailure::OutputInvalid,
        ),
        (
            "bad-shape",
            projection(&["id"], &[], false),
            ComputeState::Corrupt,
            ComputeFailure::OutputInvalid,
        ),
        (
            "claims-qualified",
            ComputeParams::ColumnProfile {},
            ComputeState::Corrupt,
            ComputeFailure::MalformedProtocol,
        ),
        (
            "env-leak",
            ComputeParams::ColumnProfile {},
            ComputeState::Unavailable,
            ComputeFailure::EnvironmentNotCleared,
        ),
        (
            "no-sandbox",
            ComputeParams::ColumnProfile {},
            ComputeState::Unavailable,
            ComputeFailure::SandboxUnavailable,
        ),
        (
            "no-sandbox-completes",
            ComputeParams::ColumnProfile {},
            ComputeState::Unavailable,
            ComputeFailure::SandboxUnavailable,
        ),
    ];
    for (mode, params, state, failure) in cases {
        lab.s.set_compute_runtime(fault(mode));
        let req = ComputeJobRequest {
            limits: Some(small.clone()),
            ..request(&lab, params)
        };
        let view = submit_and_run(&mut lab, req);
        assert_eq!(view.job.state, state, "{mode}: {view:?}");
        assert_eq!(
            view.receipt.as_ref().unwrap().failure,
            Some(failure),
            "{mode}"
        );
        assert_no_output(&view);
    }

    // Stderr is bounded and never stored as text; the job still completes.
    lab.s.set_compute_runtime(fault("flood-stderr"));
    let req = request(&lab, ComputeParams::ColumnProfile {});
    let view = submit_and_run(&mut lab, req);
    assert_eq!(view.job.state, ComputeState::Completed, "{view:?}");
    let r = view.receipt.as_ref().unwrap();
    assert_eq!(r.stderr_bytes, 64 * 64 * 1024);
    assert!(r.stderr_prefix_digest.is_some());
    let stored = serde_json::to_string(&lab.s.compute_job(view.job.header.id).unwrap()).unwrap();
    assert!(!stored.contains("eeee"));

    // Process isolation only: a worker without an OS mechanism may run,
    // and the receipt says exactly that.
    lab.s.set_compute_runtime(fault("no-sandbox"));
    let req = ComputeJobRequest {
        sandbox: Some(SandboxRequirement::ProcessIsolationOnly),
        ..request(&lab, ComputeParams::ColumnProfile {})
    };
    let view = submit_and_run(&mut lab, req);
    assert_eq!(view.job.state, ComputeState::Completed, "{view:?}");
    let sandbox = view.receipt.as_ref().unwrap().sandbox.clone().unwrap();
    assert_eq!(sandbox.mechanism, SandboxMechanism::None);
    assert!(!sandbox.ready_base_applied);

    // A missing worker is unavailable, not a silent fallback.
    lab.s.set_compute_runtime(ComputeRuntime::with_worker(
        lab.dir.join("no-such-worker"),
        vec![],
    ));
    let req = request(&lab, ComputeParams::ColumnProfile {});
    let view = submit_and_run(&mut lab, req);
    assert_eq!(view.job.state, ComputeState::Unavailable);
    assert_eq!(
        view.receipt.as_ref().unwrap().failure,
        Some(ComputeFailure::WorkerMissing)
    );
    assert!(!lab.s.compute_status().unwrap().worker_found);
    assert_eq!(lab.s.compute_status().unwrap().running, 0);
}

#[test]
fn a_running_job_cancels_through_its_handle() {
    let mut lab = setup("cancel");
    lab.s.set_compute_runtime(fault("slow"));
    let job = lab
        .s
        .compute_submit(request(&lab, ComputeParams::ColumnProfile {}))
        .unwrap();
    let canceller = lab.s.compute_canceller();
    let finished = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&finished);
    let thread = std::thread::spawn(move || {
        // Keep cancelling whatever run is current until the run returns,
        // so the test does not depend on when the run installs its handle.
        while !flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
            canceller.cancel_current();
        }
    });
    let started = std::time::Instant::now();
    let view = lab.s.compute_run(job.job.header.id).unwrap();
    finished.store(true, Ordering::SeqCst);
    thread.join().unwrap();
    assert_eq!(view.job.state, ComputeState::Cancelled, "{view:?}");
    assert_eq!(
        view.receipt.as_ref().unwrap().failure,
        Some(ComputeFailure::CancelledByUser)
    );
    assert_no_output(&view);
    assert!(started.elapsed() < Duration::from_secs(15));
    // The next run starts with a fresh handle and is not cancelled.
    lab.s.set_compute_runtime(ComputeRuntime::resolve());
    let req = request(&lab, ComputeParams::ColumnProfile {});
    let view = submit_and_run(&mut lab, req);
    assert_eq!(view.job.state, ComputeState::Completed, "{view:?}");
}

fn force_running(dir: &std::path::Path, job_id: &OpaqueId) {
    let conn = rusqlite::Connection::open(dir.join("meta.sqlite3")).unwrap();
    let body: String = conn
        .query_row(
            "SELECT body_json FROM compute_jobs WHERE job_id = ?1",
            [job_id.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&body).unwrap();
    value["state"] = serde_json::json!("running");
    conn.execute(
        "UPDATE compute_jobs SET state = 'running', body_json = ?1 WHERE job_id = ?2",
        rusqlite::params![value.to_string(), job_id.as_str()],
    )
    .unwrap();
}

#[test]
fn jobs_orphaned_by_a_crash_are_interrupted_not_rerun() {
    let mut lab = setup("recover");
    let orphan = lab
        .s
        .compute_submit(request(&lab, ComputeParams::ColumnProfile {}))
        .unwrap()
        .job
        .header
        .id;
    let second = lab
        .s
        .compute_submit(request(&lab, ComputeParams::ColumnProfile {}))
        .unwrap()
        .job
        .header
        .id;
    let dir = lab.dir.clone();
    drop(std::mem::replace(
        &mut lab.s,
        CliSession::connect("vault-085-recover-placeholder").unwrap(),
    ));
    // A crash between claim and commit leaves the job `running`.
    force_running(&dir, &orphan);
    let mut s = CliSession::connect(&lab.vault).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    lab.s = s;
    assert_eq!(lab.s.compute_status().unwrap().running, 1);
    let recovered = lab.s.compute_recover().unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].job.header.id, orphan);
    assert_eq!(recovered[0].job.state, ComputeState::Interrupted);
    assert_eq!(
        recovered[0].receipt.as_ref().unwrap().failure,
        Some(ComputeFailure::InterruptedByRestart)
    );
    assert_no_output(&recovered[0]);
    assert!(lab.s.compute_recover().unwrap().is_empty());
    assert!(matches!(
        lab.s.compute_run(orphan.clone()),
        Err(AuthorityError::Conflict { .. })
    ));

    // A run also recovers orphans first, then runs only its own job.
    let third = lab
        .s
        .compute_submit(request(&lab, ComputeParams::ColumnProfile {}))
        .unwrap()
        .job
        .header
        .id;
    drop(std::mem::replace(
        &mut lab.s,
        CliSession::connect("vault-085-recover-placeholder-2").unwrap(),
    ));
    force_running(&dir, &second);
    let mut s = CliSession::connect(&lab.vault).unwrap();
    s.open_synthetic_vault(&dir.display().to_string()).unwrap();
    lab.s = s;
    let view = lab.s.compute_run(third).unwrap();
    assert_eq!(view.job.state, ComputeState::Completed);
    assert_eq!(
        lab.s.compute_job(second).unwrap().job.state,
        ComputeState::Interrupted
    );
    assert_eq!(
        lab.s.compute_job(orphan).unwrap().job.state,
        ComputeState::Interrupted
    );
}

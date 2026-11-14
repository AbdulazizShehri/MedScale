//! Spec 085 worker process supervision.
//!
//! Launches the Compute worker with a cleared environment, a new empty
//! working directory outside the vault and stdio pipes only; writes one
//! request to stdin; reads stdout and stderr on bounded readers; and kills
//! the worker on timeout, cancellation or stdout overflow. Nothing the
//! worker writes is trusted here: stdout is returned raw for Core to
//! validate, and stderr is reduced to a byte count and the digest of a
//! bounded prefix.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use medscale_contracts::compute::{COMPUTE_WORKER_NAME, resolve_sibling_exe};
use medscale_contracts::objects::DigestSha256;

/// Which worker executable Core launches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeRuntime {
    exe: Option<PathBuf>,
    args: Vec<String>,
}

impl ComputeRuntime {
    /// The MedScale worker next to the running executable, if present.
    #[must_use]
    pub fn resolve() -> Self {
        Self {
            exe: resolve_sibling_exe(COMPUTE_WORKER_NAME),
            args: Vec::new(),
        }
    }

    /// An explicit worker (qualification harness only; the facade always
    /// uses [`ComputeRuntime::resolve`]).
    #[must_use]
    pub fn with_worker(exe: PathBuf, args: Vec<String>) -> Self {
        Self {
            exe: Some(exe),
            args,
        }
    }

    #[must_use]
    pub fn worker_found(&self) -> bool {
        self.exe.as_ref().is_some_and(|p| p.is_file())
    }
}

/// Cancels a running job from another thread.
#[derive(Debug, Clone, Default)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Holds the cancel handle of the run in progress. Each run installs a
/// fresh handle, so a cancellation never carries over to a later run.
#[derive(Debug, Clone, Default)]
pub struct CancelSlot(Arc<Mutex<CancelHandle>>);

impl CancelSlot {
    fn lock(&self) -> std::sync::MutexGuard<'_, CancelHandle> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Installs and returns a fresh handle for a run that is starting.
    #[must_use]
    pub fn fresh(&self) -> CancelHandle {
        let handle = CancelHandle::new();
        *self.lock() = handle.clone();
        handle
    }

    /// Cancels the run in progress (a no-op when none is running).
    pub fn cancel_current(&self) {
        self.lock().cancel();
    }
}

/// Bounds for one supervised run.
#[derive(Debug, Clone, Copy)]
pub struct SupervisionLimits {
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub timeout: Duration,
}

/// How the worker process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessEnd {
    /// Exited on its own; `success` is a zero exit status.
    Exited {
        success: bool,
    },
    TimedOut,
    Cancelled,
    StdoutOverflow,
}

/// Everything observed about one worker run.
#[derive(Debug, Clone)]
pub struct Supervised {
    pub end: ProcessEnd,
    /// Raw stdout, at most `stdout_bytes` (empty after an overflow).
    pub stdout: Vec<u8>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stderr_prefix_digest: Option<DigestSha256>,
}

/// Why the worker could not be started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    WorkerMissing,
    SpawnFailed,
}

/// A new empty directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn create() -> Option<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir();
        for _ in 0..16 {
            let n = NEXT.fetch_add(1, Ordering::SeqCst);
            let dir = base.join(format!(
                "medscale-compute-{}-{}-{n}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos())
            ));
            // `create_dir` fails if anything already exists there, so the
            // worker never starts in a directory someone else prepared.
            if std::fs::create_dir(&dir).is_ok() {
                return Some(Self(dir));
            }
        }
        None
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn read_bounded(mut reader: impl Read, keep: u64, overflow: Option<&AtomicBool>) -> (Vec<u8>, u64) {
    let mut kept = Vec::new();
    let mut total: u64 = 0;
    let mut buf = [0_u8; 16 * 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                total = total.saturating_add(n as u64);
                let room = keep.saturating_sub(kept.len() as u64);
                let take = usize::try_from(room.min(n as u64)).unwrap_or(0);
                kept.extend_from_slice(&buf[..take]);
                if let Some(flag) = overflow.filter(|_| total > keep) {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        }
    }
    (kept, total)
}

/// Runs the worker once with `request` on stdin.
pub fn supervise(
    runtime: &ComputeRuntime,
    request: Vec<u8>,
    limits: SupervisionLimits,
    cancel: &CancelHandle,
) -> Result<Supervised, LaunchError> {
    let Some(exe) = runtime.exe.as_ref().filter(|p| p.is_file()) else {
        return Err(LaunchError::WorkerMissing);
    };
    let scratch = Scratch::create().ok_or(LaunchError::SpawnFailed)?;
    let mut child = Command::new(exe)
        .args(&runtime.args)
        .env_clear()
        .current_dir(scratch.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| LaunchError::SpawnFailed)?;
    let (Some(mut stdin), Some(stdout), Some(stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(LaunchError::SpawnFailed);
    };
    let writer = std::thread::spawn(move || {
        // A worker that exits early closes the pipe; the write error is
        // expected then and carries no information.
        let _ = stdin.write_all(&request);
    });
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_flag = Arc::clone(&overflow);
    let stdout_keep = limits.stdout_bytes;
    let out_reader =
        std::thread::spawn(move || read_bounded(stdout, stdout_keep, Some(stdout_flag.as_ref())));
    let stderr_keep = limits.stderr_bytes;
    let err_reader = std::thread::spawn(move || read_bounded(stderr, stderr_keep, None));

    let started = Instant::now();
    let end = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                break ProcessEnd::Exited {
                    success: status.success(),
                };
            }
            Ok(None) => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break ProcessEnd::Exited { success: false };
            }
        }
        let stop = if cancel.is_cancelled() {
            Some(ProcessEnd::Cancelled)
        } else if overflow.load(Ordering::SeqCst) {
            Some(ProcessEnd::StdoutOverflow)
        } else if started.elapsed() >= limits.timeout {
            Some(ProcessEnd::TimedOut)
        } else {
            None
        };
        if let Some(end) = stop {
            let _ = child.kill();
            let _ = child.wait();
            break end;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let _ = writer.join();
    let (stdout, stdout_bytes) = out_reader.join().unwrap_or_default();
    let (stderr_prefix, stderr_bytes) = err_reader.join().unwrap_or_default();
    drop(scratch);
    // An overflow detected only after exit is still an overflow.
    let end = if matches!(end, ProcessEnd::Exited { .. }) && stdout_bytes > limits.stdout_bytes {
        ProcessEnd::StdoutOverflow
    } else {
        end
    };
    Ok(Supervised {
        end,
        stdout: if end == ProcessEnd::StdoutOverflow {
            Vec::new()
        } else {
            stdout
        },
        stdout_bytes,
        stderr_bytes,
        stderr_prefix_digest: (stderr_bytes > 0).then(|| DigestSha256::of(&stderr_prefix)),
    })
}

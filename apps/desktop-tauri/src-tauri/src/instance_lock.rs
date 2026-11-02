//! One MedScale process per data directory (Spec 108 step 3).
//!
//! Exclusive OS lock (`LockFileEx` on Windows, `flock` on Unix, through
//! `std::fs::File::try_lock`) on `data_dir/medscale-instance.lock`, taken in
//! `setup` before any window, vault, manifest or staging access and held for
//! the life of the process. The OS releases it when the handle closes,
//! including on crash or kill, so a leftover file never blocks a restart:
//! the open locked handle is the lock, not the file.
//!
//! The file stays empty and nothing in MedScale ever deletes it. On Windows
//! it is opened without `FILE_SHARE_DELETE`, so it cannot be deleted or
//! renamed while held.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub const LOCK_FILE_NAME: &str = "medscale-instance.lock";

/// Windows may release a terminated process's locks a little late, and
/// security software can briefly open a new file; retry for this long.
const RETRY_FOR: Duration = Duration::from_secs(3);
const RETRY_STEP: Duration = Duration::from_millis(50);

/// Held for the process lifetime; never dropped or unlocked explicitly.
static INSTANCE_LOCK: OnceLock<File> = OnceLock::new();

#[derive(Debug, PartialEq, Eq)]
pub enum InstanceLockError {
    /// Another MedScale process holds the lock for this data directory.
    AlreadyRunning,
    /// Exclusivity could not be confirmed (permission, I/O, unsupported FS).
    Unavailable,
}

fn open_lock_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x0000_0001;
        const FILE_SHARE_WRITE: u32 = 0x0000_0002;
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    }
    options.open(path)
}

/// True when the locked handle still refers to the file at `path`. On Unix a
/// held file can be unlinked and replaced; a lock on the old inode would not
/// exclude a process that opens the new one.
#[cfg(unix)]
fn still_names_locked_file(file: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (file.metadata(), std::fs::metadata(path)) {
        (Ok(held), Ok(named)) => held.dev() == named.dev() && held.ino() == named.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn still_names_locked_file(_file: &File, _path: &Path) -> bool {
    true
}

/// Windows ERROR_SHARING_VIOLATION / ERROR_LOCK_VIOLATION while opening.
fn transient_open_error(error: &std::io::Error) -> bool {
    cfg!(windows) && matches!(error.raw_os_error(), Some(32 | 33))
}

/// Try to take the instance lock, retrying transient conditions briefly.
pub fn try_acquire(data_dir: &Path) -> Result<File, InstanceLockError> {
    let path = data_dir.join(LOCK_FILE_NAME);
    let deadline = Instant::now() + RETRY_FOR;
    loop {
        let outcome = match open_lock_file(&path) {
            Ok(file) => match file.try_lock() {
                Ok(()) if still_names_locked_file(&file, &path) => return Ok(file),
                Ok(()) => Err(InstanceLockError::AlreadyRunning),
                Err(TryLockError::WouldBlock) => Err(InstanceLockError::AlreadyRunning),
                Err(TryLockError::Error(_)) => return Err(InstanceLockError::Unavailable),
            },
            Err(error) if transient_open_error(&error) => Err(InstanceLockError::AlreadyRunning),
            Err(_) => return Err(InstanceLockError::Unavailable),
        };
        if Instant::now() >= deadline {
            return outcome;
        }
        std::thread::sleep(RETRY_STEP);
    }
}

/// Take the lock for this process and keep it until exit.
pub fn acquire_for_process(data_dir: &Path) -> Result<(), InstanceLockError> {
    let file = try_acquire(data_dir)?;
    // A second call in the same process keeps the first handle.
    let _ = INSTANCE_LOCK.set(file);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "medscale-instance-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn second_holder_is_refused_until_the_first_is_dropped() {
        let dir = scratch_dir("exclusive");
        let first = try_acquire(&dir).expect("first instance");
        // Locks are per handle (LockFileEx) / per open file description
        // (flock), so a second open in this process models a second process.
        let started = Instant::now();
        assert_eq!(
            try_acquire(&dir).err(),
            Some(InstanceLockError::AlreadyRunning)
        );
        assert!(
            started.elapsed() >= RETRY_FOR,
            "a held lock is retried before refusing"
        );
        drop(first);
        let again = try_acquire(&dir).expect("released lock can be taken again");
        drop(again);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn leftover_lock_file_with_content_does_not_block_and_is_not_truncated() {
        let dir = scratch_dir("leftover");
        std::fs::write(dir.join(LOCK_FILE_NAME), b"left by a crashed process").expect("stale");
        let held = try_acquire(&dir).expect("a stale file is not a lock");
        drop(held);
        assert_eq!(
            std::fs::read(dir.join(LOCK_FILE_NAME)).expect("read"),
            b"left by a crashed process",
            "opening never truncates the lock file"
        );
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn held_lock_file_cannot_be_deleted_on_windows() {
        let dir = scratch_dir("nodelete");
        let held = try_acquire(&dir).expect("lock");
        assert!(
            std::fs::remove_file(dir.join(LOCK_FILE_NAME)).is_err(),
            "no FILE_SHARE_DELETE while held"
        );
        drop(held);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn missing_data_directory_is_unavailable_not_already_running() {
        let dir = scratch_dir("missing").join("does-not-exist");
        assert_eq!(
            try_acquire(&dir).err(),
            Some(InstanceLockError::Unavailable)
        );
    }
}

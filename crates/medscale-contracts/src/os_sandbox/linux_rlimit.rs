//! Linux RLIMIT helpers for Spec 052 Landlock composition.
//!
//! Workspace `unsafe_code` is `deny`; this module allows confined libc FFI only.

#![allow(unsafe_code)]

use super::OsSandboxApplyError;

/// Lower `RLIMIT_NOFILE` soft/hard for composition measurement.
pub(super) fn apply_rlimit_nofile() -> Result<(), OsSandboxApplyError> {
    // SAFETY: getrlimit/setrlimit with stack-local `rlimit` are standard POSIX FFI.
    unsafe {
        let mut cur = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut cur) != 0 {
            return Err(OsSandboxApplyError::ApplyFailed(
                "getrlimit(RLIMIT_NOFILE) failed".to_owned(),
            ));
        }
        // Already tightly constrained — Landlock axes still applied by caller.
        if cur.rlim_cur <= 8 {
            return Ok(());
        }
        let target = if cur.rlim_cur > 64 {
            64
        } else {
            (cur.rlim_cur / 2).max(8)
        };
        let next = libc::rlimit {
            rlim_cur: target,
            rlim_max: target.min(cur.rlim_max),
        };
        if libc::setrlimit(libc::RLIMIT_NOFILE, &next) != 0 {
            return Err(OsSandboxApplyError::ApplyFailed(
                "setrlimit(RLIMIT_NOFILE) failed".to_owned(),
            ));
        }
        let mut after = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut after) != 0 {
            return Err(OsSandboxApplyError::ApplyFailed(
                "getrlimit after set failed".to_owned(),
            ));
        }
        if after.rlim_cur >= cur.rlim_cur {
            return Err(OsSandboxApplyError::ApplyFailed(
                "RLIMIT_NOFILE was not lowered".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Soft `RLIMIT_NOFILE` for probe/tests.
#[must_use]
pub fn soft_nofile() -> Option<u64> {
    // SAFETY: getrlimit with stack-local `rlimit` is standard POSIX FFI.
    unsafe {
        let mut cur = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut cur) != 0 {
            return None;
        }
        Some(cur.rlim_cur)
    }
}

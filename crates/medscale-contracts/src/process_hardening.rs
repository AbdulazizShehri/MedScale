//! Process-level crash-dump hardening.
//!
//! Prevents operating-system crash reporters from capturing process memory
//! (which may hold decrypted vault metadata or keys while a vault is open).
//!
//! Workspace `unsafe_code` is `deny`; this module confines platform FFI calls
//! to `kernel32` (Windows), `libc` (Linux), and `libSystem` (macOS) with no
//! additional crate dependencies.

#![allow(unsafe_code)]

/// Outcome of requesting OS crash-dump suppression for the current process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashDumpHardening {
    pub applied: bool,
    pub mechanism: &'static str,
}

/// Suppresses OS crash memory dumps for the current process.
///
/// Safe to call multiple times at process startup (desktop and CLI).
#[must_use]
pub fn disable_crash_dumps() -> CrashDumpHardening {
    platform::disable_crash_dumps()
}

#[cfg(windows)]
mod platform {
    use super::CrashDumpHardening;

    const SEM_FAILCRITICALERRORS: u32 = 0x0001;
    const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;
    const WER_FAULT_REPORTING_NO_UI: u32 = 0x0020;
    const WER_FAULT_REPORTING_DISABLE_SNAPSHOT_CRASH: u32 = 0x0080;
    const WER_FAULT_REPORTING_DISABLE_SNAPSHOT_HANG: u32 = 0x0100;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetErrorMode(u_mode: u32) -> u32;
        fn WerSetFlags(dw_flags: u32) -> i32;
    }

    pub(super) fn disable_crash_dumps() -> CrashDumpHardening {
        // SAFETY: Process-global error mode and WER flags take plain integer
        // bitmasks with no pointers or borrowed memory.
        unsafe {
            SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
            let hr = WerSetFlags(
                WER_FAULT_REPORTING_NO_UI
                    | WER_FAULT_REPORTING_DISABLE_SNAPSHOT_CRASH
                    | WER_FAULT_REPORTING_DISABLE_SNAPSHOT_HANG,
            );
            CrashDumpHardening {
                applied: hr == 0,
                mechanism: "windows_wer_no_snapshot_and_no_ui",
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::CrashDumpHardening;

    pub(super) fn disable_crash_dumps() -> CrashDumpHardening {
        // SAFETY: Standard POSIX setrlimit(RLIMIT_CORE, 0) and Linux
        // prctl(PR_SET_DUMPABLE, 0) using stack-local POD values.
        unsafe {
            let zero = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            let rlimit_ok = libc::setrlimit(libc::RLIMIT_CORE, &zero) == 0;
            let prctl_ok = libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) == 0;
            CrashDumpHardening {
                applied: rlimit_ok && prctl_ok,
                mechanism: "linux_rlimit_core_zero_and_non_dumpable",
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::os::raw::c_int;

    use super::CrashDumpHardening;

    const RLIMIT_CORE: c_int = 4;

    #[repr(C)]
    struct Rlimit {
        rlim_cur: u64,
        rlim_max: u64,
    }

    unsafe extern "C" {
        fn setrlimit(resource: c_int, rlp: *const Rlimit) -> c_int;
    }

    pub(super) fn disable_crash_dumps() -> CrashDumpHardening {
        let zero = Rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: Standard Darwin setrlimit(RLIMIT_CORE, {0, 0}) with a valid
        // stack-local rlimit struct.
        let rc = unsafe { setrlimit(RLIMIT_CORE, &zero) };
        CrashDumpHardening {
            applied: rc == 0,
            mechanism: "macos_rlimit_core_zero",
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod platform {
    use super::CrashDumpHardening;

    pub(super) fn disable_crash_dumps() -> CrashDumpHardening {
        CrashDumpHardening {
            applied: false,
            mechanism: "unsupported_platform",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disable_crash_dumps_applies_on_supported_platforms() {
        let result = disable_crash_dumps();
        #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
        assert!(
            result.applied,
            "crash-dump hardening should apply: {:?}",
            result
        );
        assert!(!result.mechanism.is_empty());
    }
}

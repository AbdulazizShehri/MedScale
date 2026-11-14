//! macOS Seatbelt READY_BASE apply (Spec 031).
//!
//! Uses Apple `sandbox_init(3)` with a custom SBPL profile string (no App Sandbox
//! entitlements / codesigning). Full App Sandbox remains scaffold documentation only.
//!
//! `sandbox_init` is deprecated in headers but still ships in libSystem and is the
//! practical in-process Seatbelt apply path for CLI/CI without entitlements.

#![allow(unsafe_code)]

use super::OsSandboxApplyError;

#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "macos")]
static SEATBELT_APPLIED: AtomicBool = AtomicBool::new(false);

/// SBPL profile: allow default ambient behavior, deny all network operations.
///
/// Measured ambient capability: socket/connect/bind must fail after apply.
pub(super) const NETWORK_DENY_PROFILE: &str = "\
(version 1)
(allow default)
(deny network*)
";

/// Apply Seatbelt network-deny profile to the current process (ReadyBaseMeasured).
///
/// Irreversible for process lifetime — callers should use a dedicated probe
/// subprocess for measurement so the cargo test runner is not poisoned.
#[cfg(target_os = "macos")]
pub(super) fn apply_seatbelt_macos() -> Result<(), OsSandboxApplyError> {
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int};
    use std::ptr;

    if SEATBELT_APPLIED.load(Ordering::SeqCst) {
        return Ok(());
    }

    // libSystem exports sandbox_init / sandbox_free_error; no extra crate admission.
    unsafe extern "C" {
        fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *mut c_char) -> c_int;
        fn sandbox_free_error(errorbuf: *mut c_char);
    }

    // flags=0: profile is a raw SBPL string (not SANDBOX_NAMED builtin name).
    let profile = CString::new(NETWORK_DENY_PROFILE).map_err(|_| {
        OsSandboxApplyError::ApplyFailed("seatbelt profile contained interior NUL".to_owned())
    })?;

    let mut errbuf: *mut c_char = ptr::null_mut();
    // SAFETY: sandbox_init writes an optional NUL-terminated error string into errbuf;
    // we free it with sandbox_free_error. Profile is valid CString for the call duration.
    let rc = unsafe { sandbox_init(profile.as_ptr(), 0, &mut errbuf) };
    if rc != 0 {
        let msg = if errbuf.is_null() {
            format!("sandbox_init failed rc={rc}")
        } else {
            // SAFETY: errbuf is a C string owned by the sandbox API until free.
            let owned = unsafe { CStr::from_ptr(errbuf) }
                .to_string_lossy()
                .into_owned();
            unsafe { sandbox_free_error(errbuf) };
            owned
        };
        return Err(OsSandboxApplyError::ApplyFailed(msg));
    }

    SEATBELT_APPLIED.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(super) fn apply_seatbelt_macos() -> Result<(), OsSandboxApplyError> {
    Err(OsSandboxApplyError::NotReadyOnThisHost)
}

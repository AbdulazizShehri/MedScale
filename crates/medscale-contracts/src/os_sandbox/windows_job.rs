//! Windows Job Object READY_BASE apply (Spec 030).
//!
//! AppContainer remains a scaffold; this module measures Job Object
//! `ACTIVE_PROCESS` confinement without requiring administrator rights.

#![allow(unsafe_code)]

use super::OsSandboxApplyError;

#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
static JOB_APPLIED: AtomicBool = AtomicBool::new(false);

/// Apply Job Object limits to the current process (ReadyBaseMeasured).
///
/// Sets `JOB_OBJECT_LIMIT_ACTIVE_PROCESS = 1` and `KILL_ON_JOB_CLOSE`.
/// The job handle is intentionally leaked so `KILL_ON_JOB_CLOSE` does not
/// terminate the process when this function returns.
#[cfg(windows)]
pub(super) fn apply_job_object_windows() -> Result<(), OsSandboxApplyError> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    if JOB_APPLIED.load(Ordering::SeqCst) {
        return Ok(());
    }

    // SAFETY: Win32 Job Object APIs; handles validated before use; job handle leaked by design.
    unsafe {
        let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err(OsSandboxApplyError::ApplyFailed(format!(
                "CreateJobObjectW failed: {}",
                GetLastError()
            )));
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_ACTIVE_PROCESS | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        info.BasicLimitInformation.ActiveProcessLimit = 1;

        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&info).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>()
                .try_into()
                .expect("job info size fits u32"),
        );
        if ok == 0 {
            let err = GetLastError();
            CloseHandle(job);
            return Err(OsSandboxApplyError::ApplyFailed(format!(
                "SetInformationJobObject failed: {err}"
            )));
        }

        let process = GetCurrentProcess();
        let assigned = AssignProcessToJobObject(job, process);
        if assigned == 0 {
            let err = GetLastError();
            CloseHandle(job);
            return Err(OsSandboxApplyError::ApplyFailed(format!(
                "AssignProcessToJobObject failed: {err} (nested-job/host job may block; AppContainer still scaffold)"
            )));
        }

        // Intentionally do not CloseHandle(job): KILL_ON_JOB_CLOSE must not fire.
        let _leaked_job: HANDLE = job;
    }

    JOB_APPLIED.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn apply_job_object_windows() -> Result<(), OsSandboxApplyError> {
    Err(OsSandboxApplyError::NotReadyOnThisHost)
}

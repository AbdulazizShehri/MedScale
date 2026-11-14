//! Spec 033 Windows AppContainer FS READY_BASE honesty + measured deny.

use medscale_contracts::os_sandbox::{
    OsSandboxApplyError, OsSandboxPlan, OsSandboxQualification, try_apply_os_sandbox,
};

#[test]
fn doctor_windows_appcontainer_fs_measured_honest() {
    let d = medscale_contracts::os_sandbox::OsSandboxDoctorStatus::ready_base();
    assert!(d.is_honest_ready_base());
    assert!(d.windows_measured);
    assert!(d.windows_appcontainer_fs_measured);
    assert!(d.linux_measured);
    assert!(d.macos_measured);
    assert!(!d.platform_qualified);
    assert!(!d.release_ready);
}

#[test]
fn scaffold_still_not_platform_qualified() {
    assert!(!OsSandboxPlan::windows_appcontainer_scaffold().claims_platform_qualified());
    assert_eq!(
        try_apply_os_sandbox(&OsSandboxPlan::windows_appcontainer_scaffold()),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
}

#[test]
fn platform_qualified_never_honored() {
    let mut plan = OsSandboxPlan::windows_appcontainer_fs_ready_base();
    plan.qualification = OsSandboxQualification::PlatformQualified;
    assert_eq!(
        try_apply_os_sandbox(&plan),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
}

#[cfg(not(windows))]
#[test]
fn appcontainer_fs_not_ready_off_windows() {
    assert_eq!(
        try_apply_os_sandbox(&OsSandboxPlan::windows_appcontainer_fs_ready_base()),
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    );
}

#[cfg(windows)]
#[test]
fn appcontainer_fs_measured_denies_host_marker() {
    use std::process::Command;

    let probe = env!("CARGO_BIN_EXE_medscale-os-sandbox-probe");
    let output = Command::new(probe)
        .arg("appcontainer-fs")
        .output()
        .expect("spawn medscale-os-sandbox-probe appcontainer-fs");
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "expected AppContainer FS deny (exit 0); got {code}; stderr={stderr}"
    );
    assert!(
        stderr.contains("AppContainer FS deny") || stderr.contains("OK:"),
        "stderr={stderr}"
    );
}

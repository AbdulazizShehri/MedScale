//! Spec 031 macOS Seatbelt READY_BASE honesty + measured deny.

use medscale_contracts::os_sandbox::{
    OsSandboxApplyError, OsSandboxPlan, OsSandboxQualification, try_apply_os_sandbox,
};

#[test]
fn doctor_macos_measured_honest() {
    let d = medscale_contracts::os_sandbox::OsSandboxDoctorStatus::ready_base();
    assert!(d.is_honest_ready_base());
    assert!(d.macos_measured);
    assert!(d.windows_measured);
    assert!(d.windows_appcontainer_fs_measured);
    assert!(d.linux_measured);
    assert!(!d.platform_qualified);
    assert!(!d.release_ready);
}

#[test]
fn macos_scaffold_still_not_platform_qualified() {
    assert!(!OsSandboxPlan::macos_seatbelt_scaffold().claims_platform_qualified());
    assert_eq!(
        try_apply_os_sandbox(&OsSandboxPlan::macos_seatbelt_scaffold()),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
}

#[test]
fn appcontainer_scaffold_unchanged() {
    assert!(!OsSandboxPlan::windows_appcontainer_scaffold().claims_platform_qualified());
    assert_eq!(
        try_apply_os_sandbox(&OsSandboxPlan::windows_appcontainer_scaffold()),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
}

#[test]
fn platform_qualified_never_honored() {
    let mut plan = OsSandboxPlan::macos_seatbelt_ready_base();
    plan.qualification = OsSandboxQualification::PlatformQualified;
    assert_eq!(
        try_apply_os_sandbox(&plan),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn macos_ready_base_not_ready_off_macos() {
    assert_eq!(
        try_apply_os_sandbox(&OsSandboxPlan::macos_seatbelt_ready_base()),
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn seatbelt_measured_denies_network() {
    use std::process::Command;

    let probe = env!("CARGO_BIN_EXE_medscale-os-sandbox-probe");
    let output = Command::new(probe)
        .output()
        .expect("spawn medscale-os-sandbox-probe");
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code, 0,
        "expected measured network-deny (exit 0); got {code}; stderr={stderr}"
    );
    assert!(
        stderr.contains("network denied") || stderr.contains("OK:"),
        "stderr={stderr}"
    );
}

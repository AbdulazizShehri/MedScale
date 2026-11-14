//! Spec 026 OS sandbox honesty + Linux measured apply.

use medscale_contracts::os_sandbox::{
    OsSandboxApplyError, OsSandboxPlan, OsSandboxQualification, try_apply_os_sandbox,
};
use medscale_core::{build_doctor_report, privacy_proof_artifact_present};

#[test]
fn doctor_os_sandbox_and_pack_signer_honest() {
    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(report.pack_signer.is_honest_ready_base());
    assert!(report.os_sandbox.is_honest_ready_base());
    assert!(report.os_sandbox.linux_measured);
    assert!(report.os_sandbox.linux_landlock_composition_measured);
    assert!(report.os_sandbox.windows_measured);
    assert!(report.os_sandbox.windows_appcontainer_fs_measured);
    assert!(report.os_sandbox.windows_appcontainer_network_measured);
    assert!(report.os_sandbox.windows_appcontainer_lpac_measured);
    assert!(report.os_sandbox.macos_measured);
    assert!(report.os_sandbox.macos_app_sandbox_entitlements_measured);
    assert!(!report.os_sandbox.macos_app_sandbox_enforcement_measured);
    assert!(!report.os_sandbox.platform_qualified);
    assert!(!report.os_sandbox.release_ready);
    assert!(!report.pack_signer.release_ready);
}

#[test]
fn windows_appcontainer_and_macos_scaffold_remain_not_platform_qualified() {
    assert!(!OsSandboxPlan::windows_appcontainer_scaffold().claims_platform_qualified());
    assert!(!OsSandboxPlan::macos_seatbelt_scaffold().claims_platform_qualified());
    assert_eq!(
        try_apply_os_sandbox(&OsSandboxPlan::windows_appcontainer_scaffold()),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
    assert_eq!(
        try_apply_os_sandbox(&OsSandboxPlan::macos_seatbelt_scaffold()),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
}

#[test]
fn platform_qualified_never_honored_while_gate_open() {
    let mut plan = OsSandboxPlan::linux_landlock_ready_base(vec!["/tmp".into()]);
    plan.qualification = OsSandboxQualification::PlatformQualified;
    assert_eq!(
        try_apply_os_sandbox(&plan),
        Err(OsSandboxApplyError::NotPlatformQualified)
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn ready_base_linux_plan_not_ready_on_this_host() {
    let plan = OsSandboxPlan::linux_landlock_ready_base(vec!["C:\\Windows\\Temp".into()]);
    assert_eq!(
        try_apply_os_sandbox(&plan),
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    );
    let doctor = build_doctor_report(None, false, privacy_proof_artifact_present()).os_sandbox;
    assert!(doctor.linux_measured);
    assert!(doctor.windows_measured);
    assert!(doctor.windows_appcontainer_fs_measured);
    assert!(doctor.windows_appcontainer_network_measured);
    assert!(doctor.windows_appcontainer_lpac_measured);
    assert!(doctor.macos_measured);
    assert!(!doctor.platform_qualified);
}

#[cfg(target_os = "linux")]
#[test]
fn landlock_measured_denies_path_outside_allowlist() {
    use std::fs;
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let base = std::env::temp_dir().join(format!(
            "medscale-landlock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let allow = base.join("allow");
        let deny_parent = base.join("deny");
        fs::create_dir_all(&allow).unwrap();
        fs::create_dir_all(&deny_parent).unwrap();
        let allow_file = allow.join("ok.txt");
        let deny_file = deny_parent.join("secret.txt");
        fs::write(&allow_file, b"ok").unwrap();
        fs::write(&deny_file, b"secret").unwrap();

        let plan =
            OsSandboxPlan::linux_landlock_ready_base(vec![allow.to_string_lossy().into_owned()]);
        try_apply_os_sandbox(&plan).expect("landlock apply");

        let allowed = fs::File::open(&allow_file).is_ok();
        let denied = fs::File::open(&deny_file).is_err();
        let _ = fs::remove_dir_all(&base);
        tx.send((allowed, denied)).unwrap();
    })
    .join()
    .expect("thread");

    let (allowed, denied) = rx.recv().unwrap();
    assert!(allowed, "allowlisted path must remain readable");
    assert!(denied, "path outside allowlist must be denied");
}

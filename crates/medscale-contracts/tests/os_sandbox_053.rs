//! Spec 053 - Linux seccomp-bpf strict allowlist composition honesty.

use medscale_contracts::os_sandbox::{
    OsSandboxCompositionInventory, OsSandboxDoctorStatus, OsSandboxPlan,
};

#[test]
fn seccomp_plan_never_claims_platform_qualified() {
    let plan = OsSandboxPlan::linux_seccomp_composition_ready_base();
    assert!(plan.claims_ready_base_measured());
    assert!(!plan.claims_platform_qualified());
    assert!(
        plan.mechanisms
            .iter()
            .any(|m| m.contains("seccomp_bpf_strict_allowlist"))
    );
    assert!(plan.mechanisms.iter().any(|m| m.contains("no_new_privs")));
}

#[test]
fn doctor_seccomp_axis_honest() {
    let d = OsSandboxDoctorStatus::ready_base();
    assert!(d.linux_seccomp_composition_measured);
    assert!(!d.platform_qualified);
    assert!(d.is_honest_ready_base());
    let inv = OsSandboxCompositionInventory::trusted_v1_ready_base();
    assert!(
        inv.axes_ready_base_measured
            .contains(&"linux_seccomp_composition".to_owned())
    );
    assert!(inv.is_honest());
}

#[cfg(not(target_os = "linux"))]
#[test]
fn seccomp_not_ready_on_non_linux() {
    use medscale_contracts::os_sandbox::{OsSandboxApplyError, try_apply_os_sandbox};

    let plan = OsSandboxPlan::linux_seccomp_composition_ready_base();
    assert_eq!(
        try_apply_os_sandbox(&plan),
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    );
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn seccomp_child_dies_by_sigsys_on_denied_socket() {
    use medscale_contracts::os_sandbox::measure_seccomp_composition;

    let probe = std::path::PathBuf::from(env!("CARGO_BIN_EXE_medscale-os-sandbox-probe"));
    measure_seccomp_composition(&probe).expect("seccomp SIGSYS deny measured");
}

#[cfg(all(
    target_os = "linux",
    not(any(target_arch = "x86_64", target_arch = "aarch64"))
))]
#[test]
fn seccomp_unsupported_arch_is_not_ready() {
    use medscale_contracts::os_sandbox::{OsSandboxApplyError, resolve_seccomp_probe_exe};

    let exe = resolve_seccomp_probe_exe().expect("probe path resolves");
    assert_eq!(
        medscale_contracts::os_sandbox::measure_seccomp_composition(&exe),
        Err(OsSandboxApplyError::NotReadyOnThisHost)
    );
}

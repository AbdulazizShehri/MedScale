//! Spec 044 multi-OS sandbox composition honesty.

use medscale_contracts::os_sandbox::{
    OsSandboxCompositionInventory, OsSandboxDoctorStatus, composition_residuals_open,
};
use medscale_core::build_doctor_report;

#[test]
fn composition_inventory_never_claims_platform_qualified() {
    let inv = OsSandboxCompositionInventory::trusted_v1_ready_base();
    assert!(inv.is_honest());
    assert!(!inv.platform_qualified);
    assert_eq!(inv.residuals_open, composition_residuals_open());
    assert!(
        inv.residuals_open
            .contains(&"macos_app_sandbox_signed_enforcement".to_owned())
    );
    assert!(
        inv.residuals_open
            .contains(&"multi_os_platform_qualified_composition".to_owned())
    );
}

#[test]
fn doctor_exposes_composition_inventory() {
    let report = build_doctor_report(None, false, false);
    let d = &report.os_sandbox;
    assert!(d.composition_inventory_present);
    assert!(!d.platform_qualified);
    assert_eq!(d.composition_residuals_open, composition_residuals_open());
    assert!(OsSandboxDoctorStatus::ready_base().is_honest_ready_base());
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("platform_qualified=false")
                && (n.contains("Spec 044") || n.contains("Spec 052")))
    );
}

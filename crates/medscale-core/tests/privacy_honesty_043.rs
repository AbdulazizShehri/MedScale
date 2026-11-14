//! Spec 043 swap/snapshot honesty — never claims protection measured.

use medscale_contracts::doctor::ProbeHonestyClass;
use medscale_core::build_doctor_report;
use medscale_storage::{ProbeHonestyClass as StorageHonesty, probe_os_privacy_surfaces};

#[test]
fn swap_snapshot_honesty_never_claims_protection_measured() {
    let r = probe_os_privacy_surfaces();
    assert!(r.swap_snapshot_honesty_present);
    for surface in [
        &r.pagefile,
        &r.hibernate,
        &r.swap,
        &r.snapshot,
        &r.core_dump_config,
    ] {
        assert_ne!(
            surface.protection_honesty,
            StorageHonesty::ProtectionMeasured
        );
        assert_eq!(
            surface.protection_honesty,
            StorageHonesty::OwnerOrOsPolicyRequired
        );
    }
}

#[test]
fn doctor_exposes_043_honesty_fields() {
    let report = build_doctor_report(None, false, false);
    let v = &report.vault_privacy;
    assert!(v.swap_snapshot_honesty_present);
    assert!(!v.private_data_ready);
    assert_eq!(
        v.pagefile_protection_honesty,
        ProbeHonestyClass::OwnerOrOsPolicyRequired
    );
    assert_eq!(
        v.snapshot_protection_honesty,
        ProbeHonestyClass::OwnerOrOsPolicyRequired
    );
    assert!(matches!(
        v.pagefile_existence_honesty,
        ProbeHonestyClass::ConfigurationDetected
            | ProbeHonestyClass::NotMeasurableOnHost
            | ProbeHonestyClass::OwnerOrOsPolicyRequired
    ));
}

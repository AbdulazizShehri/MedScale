//! Spec 032/043 privacy probes / NOTICE doctor honesty.

use medscale_contracts::doctor::{OsResidualFileProbe, ProbeHonestyClass};
use medscale_core::build_doctor_report;
use medscale_storage::{probe_os_privacy_surfaces, residual_risk_classes_open};

#[test]
fn vault_privacy_probes_present_and_residuals_open() {
    let report = build_doctor_report(None, false, false);
    let v = &report.vault_privacy;
    assert!(v.present);
    assert!(v.probes_present);
    assert!(v.swap_snapshot_honesty_present);
    assert!(!v.private_data_ready);
    assert!(v.vault_leftover_scan_available);
    assert!(v.crash_sidecar_detect_available);
    assert_eq!(
        v.residual_risk_classes_open,
        vec![
            "swap".to_owned(),
            "hibernate".to_owned(),
            "snapshot".to_owned(),
            "pagefile".to_owned(),
            "core_dump".to_owned()
        ]
    );
    assert!(matches!(
        v.pagefile_existence,
        OsResidualFileProbe::Detected
            | OsResidualFileProbe::NotFound
            | OsResidualFileProbe::NotReadable
            | OsResidualFileProbe::NotApplicable
    ));
    assert!(matches!(
        v.snapshot_existence,
        OsResidualFileProbe::Detected
            | OsResidualFileProbe::NotFound
            | OsResidualFileProbe::NotReadable
            | OsResidualFileProbe::NotApplicable
    ));
    assert_ne!(
        v.snapshot_protection_honesty,
        ProbeHonestyClass::ProtectionMeasured
    );
    assert_eq!(
        v.snapshot_protection_honesty,
        ProbeHonestyClass::OwnerOrOsPolicyRequired
    );
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("Spec 043") && n.contains("PRIVATE_DATA_READY=false"))
    );
}

#[test]
fn release_qualification_notice_inventory_with_license_decision() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.notice_inventory_present);
    assert!(rq.rights_license_decision);
    assert!(rq.is_honest_prep());
    assert!(!rq.release_ready);
    assert!(
        !rq.missing_evidence_classes
            .contains(&"public_source_license_choice".to_owned())
    );
}

#[test]
fn storage_probe_api_matches_doctor_residual_classes() {
    let r = probe_os_privacy_surfaces();
    assert!(r.probes_present);
    assert!(r.swap_snapshot_honesty_present);
    assert_eq!(r.residual_risk_classes_open, residual_risk_classes_open());
}

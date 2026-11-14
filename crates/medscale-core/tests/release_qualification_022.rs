//! Spec 022 release-qualification prep honesty.

use medscale_core::build_doctor_report;

#[test]
fn release_qualification_axis_is_honest_prep_not_release_ready() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.present);
    assert!(rq.prep_ready_base);
    assert!(!rq.release_ready);
    assert!(rq.locked_builds);
    assert!(rq.immutable_ci_action_pins);
    assert!(rq.cargo_lock_committed);
    assert!(rq.windows_linux_ci_baseline);
    assert!(rq.macos_ci_present);
    assert!(!rq.macos_qualified);
    assert!(!rq.mobile_release_qualified);
    assert!(
        !rq.missing_evidence_classes
            .contains(&"mobile_app_release_qualification".to_owned())
    );
    assert!(rq.branch_protection_configured);
    assert!(!rq.missing_evidence_classes.is_empty());
    assert!(rq.is_honest_prep());
    assert!(rq.perf_harness_present);
    assert!(rq.sbom_scaffold_present);
    assert!(rq.sbom_lock_bound);
    assert!(rq.release_dry_run_verifier_present);
    assert!(rq.migration_recovery_ready_base);
    assert!(rq.package_upgrade_rollback_scaffold_present);
    assert!(rq.portable_release_package_qualified);
    assert!(rq.package_lifecycle_qualified);
    assert!(rq.host_perf_measurement_path_present);
    assert!(rq.runtime_perf_measurement_coverage_present);
    assert!(rq.notice_inventory_present);
    assert!(rq.rights_license_decision);
    assert!(rq.material_findings_clearance);
    assert!(
        !rq.missing_evidence_classes
            .contains(&"release_package_upgrade_rollback_proof".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"reproducible_release_package_contents".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"release_bar_migration_recovery_proof".to_owned())
    );
    assert!(
        rq.missing_evidence_classes
            .contains(&"macos_platform_product_qualification".to_owned())
    );
    assert!(
        rq.missing_evidence_classes
            .contains(&"perf_budgets_attained_on_qualified_hardware".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"unresolved_material_findings_clearance".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"repo_branch_protection_required_checks".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"public_source_license_choice".to_owned())
    );
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("RELEASE_READY=false") && n.contains("Release qualification"))
    );
}

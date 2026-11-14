//! Spec 049 — package upgrade/rollback scaffold doctor honesty.

use medscale_core::build_doctor_report;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn package_upgrade_rollback_scaffold_doctor_honest() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.package_upgrade_rollback_scaffold_present);
    assert!(rq.migration_recovery_ready_base);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    // Spec 058 supersedes the scaffold-only residual with real portable-package lifecycle proof.
    assert!(rq.portable_release_package_qualified);
    assert!(rq.package_lifecycle_qualified);
    assert!(
        !rq.missing_evidence_classes
            .contains(&"release_package_upgrade_rollback_proof".to_owned())
    );
}

#[test]
fn package_upgrade_rollback_scripts_and_evidence_present() {
    let root = repo_root();
    assert!(
        root.join("scripts/verify-package-upgrade-rollback.ps1")
            .is_file()
    );
    assert!(
        root.join("evidence/049-package-upgrade-rollback/SUMMARY.md")
            .is_file()
    );
    assert!(
        root.join("evidence/049-package-upgrade-rollback/LIMITATIONS.md")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_049_PROMOTION.md")
            .is_file()
    );
}

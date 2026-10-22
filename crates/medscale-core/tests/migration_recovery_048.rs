//! Spec 048 doctor honesty for migration/recovery READY_BASE.

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
fn migration_recovery_doctor_honest() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.migration_recovery_ready_base);
    assert!(rq.release_dry_run_verifier_present);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    // Spec 058 later closes the package lifecycle residual; Spec 048 recovery remains qualified.
    assert!(
        !rq.missing_evidence_classes
            .contains(&"release_package_upgrade_rollback_proof".to_owned())
    );
    assert!(
        !rq.missing_evidence_classes
            .contains(&"release_bar_migration_recovery_proof".to_owned())
    );
}

#[test]
fn migration_recovery_evidence_present() {
    let root = repo_root();
    assert!(
        root.join("evidence/048-migration-recovery-release-bar/SUMMARY.md")
            .is_file()
    );
    assert!(
        root.join("evidence/048-migration-recovery-release-bar/LIMITATIONS.md")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_048_PROMOTION.md")
            .is_file()
    );
}

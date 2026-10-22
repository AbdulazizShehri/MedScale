//! Spec 047 — release dry-run verifier doctor honesty + script presence.

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
fn release_dry_run_verifier_doctor_honest() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.release_dry_run_verifier_present);
    assert!(rq.sbom_lock_bound);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    assert!(
        rq.missing_evidence_classes
            .contains(&"release_sbom_signing_provenance".to_owned())
    );
    assert!(
        rq.missing_evidence_classes
            .contains(&"checksums_provenance_signing_verification".to_owned())
    );
}

#[test]
fn release_dry_run_scripts_and_evidence_present() {
    let root = repo_root();
    assert!(root.join("scripts/release-dry-run.ps1").is_file());
    assert!(root.join("scripts/verify-release-manifest.ps1").is_file());
    assert!(
        root.join("evidence/047-release-dry-run-verifier/SUMMARY.md")
            .is_file()
    );
    assert!(
        root.join("evidence/047-release-dry-run-verifier/LIMITATIONS.md")
            .is_file()
    );
    assert!(
        root.join("evidence/047-release-dry-run-verifier/native-deps-inventory.json")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_047_PROMOTION.md")
            .is_file()
    );
}

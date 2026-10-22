//! Spec 050 — host perf measurement path doctor honesty.

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
fn host_perf_measurement_path_doctor_honest() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.host_perf_measurement_path_present);
    assert!(rq.perf_harness_present);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());
    assert!(
        rq.missing_evidence_classes
            .contains(&"perf_budgets_attained_on_qualified_hardware".to_owned())
    );
}

#[test]
fn host_perf_measurement_script_and_evidence_present() {
    let root = repo_root();
    assert!(root.join("scripts/run-host-perf-measurement.ps1").is_file());
    assert!(
        root.join("evidence/050-host-perf-measurement/SUMMARY.md")
            .is_file()
    );
    assert!(
        root.join("evidence/050-host-perf-measurement/LIMITATIONS.md")
            .is_file()
    );
    assert!(
        root.join("docs/qualification/promotions/SPEC_050_PROMOTION.md")
            .is_file()
    );
}

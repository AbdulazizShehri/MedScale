//! Spec 057 release-qualification residual integrity.
//! Verifies CI action immutability and honest runtime perf coverage.

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
fn every_github_action_use_is_sha_pinned() {
    let workflow = std::fs::read_to_string(repo_root().join(".github/workflows/ci.yml"))
        .expect("read CI workflow");
    let mut uses_count = 0_usize;

    for line in workflow.lines().filter(|line| line.contains("uses:")) {
        uses_count += 1;
        let reference = line
            .split("uses:")
            .nth(1)
            .expect("uses reference")
            .split_whitespace()
            .next()
            .expect("action token");
        let pin = reference
            .rsplit_once('@')
            .unwrap_or_else(|| panic!("unversioned action reference: {reference}"))
            .1;
        assert_eq!(
            pin.len(),
            40,
            "action is not pinned to a 40-char SHA: {reference}"
        );
        assert!(
            pin.chars().all(|c| c.is_ascii_hexdigit()),
            "action pin is not hexadecimal: {reference}"
        );
    }

    assert!(
        uses_count >= 8,
        "unexpectedly small action inventory: {uses_count}"
    );
    assert!(workflow.contains("actions/upload-artifact@b7c566a772e6b6bfb58ed0dc250532a479d7789f"));
    assert!(workflow.contains("actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803"));
}

#[test]
fn runtime_perf_coverage_is_present_without_attainment_claim() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;

    assert!(rq.runtime_perf_measurement_coverage_present);
    assert!(!rq.release_ready);
    assert!(
        rq.missing_evidence_classes
            .contains(&"perf_budgets_attained_on_qualified_hardware".to_owned())
    );
}
#[test]
fn final_ui_latency_is_present_but_external_measurement_gated() {
    let runtime_test = std::fs::read_to_string(
        repo_root().join("crates/medscale-desktop/tests/runtime_perf_057.rs"),
    )
    .expect("read runtime perf harness");
    let methodology = std::fs::read_to_string(
        repo_root().join("evidence/027-perf-sbom-release-evidence/PERF_METHODOLOGY.md"),
    )
    .expect("read perf methodology");

    assert!(runtime_test.contains("FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED"));
    assert!(runtime_test.contains("budgets_claimed_met\": false"));
    assert!(methodology.contains("Cold model-free desktop launch"));
    assert!(methodology.contains("Model-free desktop idle memory"));
    assert!(methodology.contains("FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED"));
}

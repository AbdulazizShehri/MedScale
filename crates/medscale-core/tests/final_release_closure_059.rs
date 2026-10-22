//! Spec 059 final release-closure audit honesty.

use medscale_core::build_doctor_report;
use std::{collections::BTreeSet, path::PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn release_residuals_are_exactly_external_after_material_clearance() {
    let report = build_doctor_report(None, false, false);
    let rq = &report.release_qualification;
    assert!(rq.material_findings_clearance);
    assert!(!rq.release_ready);
    assert!(rq.is_honest_prep());

    let expected = BTreeSet::from([
        "checksums_provenance_signing_verification".to_owned(),
        "macos_platform_product_qualification".to_owned(),
        "perf_budgets_attained_on_qualified_hardware".to_owned(),
        "release_sbom_signing_provenance".to_owned(),
        "wcag_final_v0_ui_accessibility_qualification".to_owned(),
    ]);
    let actual = rq
        .missing_evidence_classes
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn living_release_documents_encode_external_only_residual_truth() {
    let root = repo_root();
    let checklist = std::fs::read_to_string(
        root.join("evidence/022-release-qualification-prep/RELEASE_READY_CHECKLIST.md"),
    )
    .expect("read living release checklist");
    let signing = std::fs::read_to_string(
        root.join("evidence/022-release-qualification-prep/SIGNING_PROVENANCE_PREP.md"),
    )
    .expect("read signing packet");
    let gates = std::fs::read_to_string(root.join("docs/qualification/EXTERNAL_GATES.md"))
        .expect("read external gates");

    for stale in [
        "No release package pipeline",
        "package upgrade/rollback still missing",
        "SPDX still PENDING",
        "observed main unprotected and rulesets empty",
    ] {
        assert!(!checklist.contains(stale));
        assert!(!signing.contains(stale));
        assert!(!gates.contains(stale));
    }

    for gate in [
        "DESKTOP_RELEASE_SIGNING_PROVENANCE",
        "QUALIFIED_RELEASE_PERFORMANCE_HARDWARE",
        "MACOS_SIGNED_PRODUCT_QUALIFICATION",
        "FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION",
    ] {
        assert!(
            gates.contains(gate),
            "missing external gate mapping: {gate}"
        );
    }
}

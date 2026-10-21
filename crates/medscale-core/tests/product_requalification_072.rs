//! Spec 072 rebuilt-product requalification honesty regressions.

use medscale_core::{build_doctor_report, privacy_proof_artifact_present};
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
fn rebuilt_native_route_inventory_includes_models_and_evidence() {
    let ui = std::fs::read_to_string(repo_root().join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    for route in [
        "Home",
        "Patients",
        "Documents",
        "Insights",
        "Models",
        "Evidence",
        "Workflows",
        "Tasks",
        "Messages",
        "Audit Trail",
        "Exports",
        "Integrations",
        "Settings",
        "About",
    ] {
        assert!(
            ui.contains(&format!("label: \"{route}\"")),
            "missing nav route: {route}"
        );
        assert!(
            ui.contains(&format!("root.active-route == \"{route}\"")),
            "missing route surface: {route}"
        );
    }
    assert!(!ui.contains("This surface is scheduled in a later Desktop slice."));
    assert!(!ui.contains("label: \"MESC\""));
    assert!(!ui.contains("Optional / deferred"));
    let home = std::fs::read_to_string(
        repo_root().join("crates/medscale-desktop/ui/command-center.slint"),
    )
    .expect("Command Center");
    assert!(ui.contains("if root.active-route == \"Home\": CommandCenter"));
    assert!(ui.contains("patient-name: root.patient-name"));
    assert!(home.contains("Synthetic demo"));
}

#[test]
fn rebuilt_model_and_evidence_surfaces_preserve_truth_boundaries() {
    let root = repo_root();
    let ui = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    let product =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/product_intelligence.rs"))
            .expect("product intelligence");

    for required in [
        "Local Pack operator.",
        "Admit local Pack",
        "Refresh",
        "MedScale evidence ledger.",
        "Claim state ",
        "Limitations · ",
    ] {
        assert!(
            ui.contains(required),
            "missing rebuilt truth surface: {required}"
        );
    }
    for forbidden in [
        "what OpenMed still does better",
        "See where OpenMed is ahead",
        "verdict: \"PROVEN ADVANTAGE\"",
        "verdict: \"OPENMED AHEAD\"",
        "verdict: \"PARITY\"",
        "verdict: \"SURPASS\"",
    ] {
        assert!(
            !ui.contains(forbidden),
            "forbidden UI claim returned: {forbidden}"
        );
        assert!(
            !product.contains(forbidden),
            "forbidden product claim returned: {forbidden}"
        );
    }
    for required in [
        "Claim ledger {}/39 accounted",
        "0 BenchmarkManifest(s)",
        "No parity, surpass, privacy-superiority or runtime-winner claim is authorized.",
        "SESSION INVENTORY",
        "QUALIFICATION REFERENCE",
        "UNMEASURED",
        "STRUCTURAL ONLY",
        "ANTI-METRIC",
    ] {
        assert!(
            product.contains(required),
            "missing claim-discipline marker: {required}"
        );
    }
}

#[test]
fn rebuilt_accessibility_packet_covers_models_evidence_and_stays_external() {
    let root = repo_root();
    let packet = std::fs::read_to_string(
        root.join("evidence/072-product-requalification/ACCESSIBILITY_ACTION_PACKET.md"),
    )
    .expect("Spec 072 accessibility packet");
    for required in [
        "Models",
        "Evidence",
        "VoiceOver",
        "NVDA",
        "Orca",
        "Keyboard-only traversal",
        "200%",
        "39/39",
        "UNMEASURED",
        "STRUCTURAL ONLY",
        "ANTI-METRIC",
        "wcag_conformance_claimed=false",
        "release_ready=false",
    ] {
        assert!(
            packet.contains(required),
            "missing accessibility requirement: {required}"
        );
    }

    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(report.accessibility.final_v0_ui_present);
    assert!(!report.accessibility.wcag_conformance_claimed);
    assert!(!report.accessibility.release_ready);
}

#[test]
fn rebuilt_performance_packet_covers_models_evidence_without_attainment_claim() {
    let root = repo_root();
    let packet = std::fs::read_to_string(
        root.join("evidence/072-product-requalification/PERFORMANCE_ACTION_PACKET.md"),
    )
    .expect("Spec 072 performance packet");
    for required in [
        "3 warmups + 30 timed runs",
        "Models",
        "Evidence",
        "p50",
        "p95",
        "<= 2000 ms",
        "<= 100 ms",
        "<= 250 MiB",
        "FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED",
        "budgets_claimed_met=true",
    ] {
        assert!(
            packet.contains(required),
            "missing performance requirement: {required}"
        );
    }
    assert!(packet.contains("cannot set `budgets_claimed_met=true`"));
}

#[test]
fn release_residuals_remain_exactly_external_after_rebuild() {
    let root = repo_root();
    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert_eq!(report.mesc_artifact.disposition, "SEPARATE_PROJECT");
    assert_eq!(report.mesc_artifact.gate, "NONE");
    assert_eq!(report.mesc_artifact.integration_status, "OUT_OF_SCOPE");
    let rq = &report.release_qualification;
    assert!(rq.material_findings_clearance);
    assert!(!rq.release_ready);
    let expected = BTreeSet::from([
        "checksums_provenance_signing_verification".to_owned(),
        "macos_platform_product_qualification".to_owned(),
        "perf_budgets_attained_on_qualified_hardware".to_owned(),
        "release_sbom_signing_provenance".to_owned(),
        "wcag_final_v0_ui_accessibility_qualification".to_owned(),
    ]);
    assert_eq!(
        rq.missing_evidence_classes
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected
    );

    let audit = std::fs::read_to_string(
        root.join("evidence/072-product-requalification/RELEASE_RESIDUAL_AUDIT.md"),
    )
    .expect("Spec 072 residual audit");
    let gates = std::fs::read_to_string(root.join("docs/planning/EXTERNAL_GATES.md"))
        .expect("external gates");
    for class in &expected {
        assert!(audit.contains(class), "residual not mapped: {class}");
    }
    assert!(
        audit
            .contains("MESC is a separate project and is excluded from MedScale release residuals")
    );
    assert!(!gates.contains("| MESC_RELEASED_ARTIFACT |"));
    let completion =
        std::fs::read_to_string(root.join("docs/planning/PROJECT_COMPLETION_STATUS.md"))
            .expect("completion status");
    assert!(!completion.contains("MESC_RELEASE_BLOCKING"));
    let separation = std::fs::read_to_string(root.join("docs/planning/MESC_PROJECT_SEPARATION.md"))
        .expect("MESC project separation decision");
    assert!(separation.contains("separate project/repository"));
    let closure =
        std::fs::read_to_string(root.join("docs/planning/REPOSITORY_IMPLEMENTATION_CLOSURE.md"))
            .expect("repository implementation closure");
    assert!(closure.contains("STATUS = REPOSITORY_IMPLEMENTATION_COMPLETE_PENDING_EXTERNAL_GATES"));
    assert!(closure.contains("PROMOTED_REPOSITORY_OWNED_RESIDUALS = 0"));
    assert!(closure.contains("NEXT_PROMOTED_SPEC = NONE"));
    assert!(
        closure.contains("FINAL_IMPLEMENTATION_MERGE = f9587c3ed66609b6e2a7c23bf2c45df6a67c8f2e")
    );
    assert!(
        !completion
            .contains("No repository-owned Desktop+CLI implementation unit remains promoted")
    );
    assert!(gates.contains("Spec 072"));
    assert!(gates.contains("including Models/Evidence"));
}

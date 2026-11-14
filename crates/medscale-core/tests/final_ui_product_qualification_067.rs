//! Spec 067 final UI product qualification honesty regression.

use medscale_core::{build_doctor_report, privacy_proof_artifact_present};
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
fn final_native_surface_inventory_has_no_future_placeholder_or_risk_claim() {
    let root = repo_root();
    let ui = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    for route in [
        "Home",
        "Patients",
        "Insights",
        "Workflows",
        "Tasks",
        "Messages",
        "Documents",
        "Audit Trail",
        "Exports",
        "Integrations",
        "Settings",
        "About",
    ] {
        assert!(
            ui.contains(&format!("root.active-route == \"{route}\"")),
            "missing final route handling: {route}"
        );
    }
    assert!(!ui.contains("This surface is scheduled in a later Desktop slice."));
    assert!(!ui.contains("High risk"));
    let home =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/command-center.slint"))
            .expect("Command Center");
    assert!(ui.contains("if root.active-route == \"Home\": CommandCenter"));
    assert!(ui.contains("patient-name: root.patient-name"));
    assert!(home.contains("Synthetic demo"));
    assert!(home.contains("not a clinical risk ranking"));
    assert!(ui.contains("Documents — bounded intake"));
}

#[test]
fn final_accessibility_posture_is_engineering_qualified_but_not_wcag() {
    let root = repo_root();
    let components =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/components.slint"))
            .expect("components");
    let app =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint")).expect("app");
    for required in [
        "FocusScope",
        "accessible-role: button",
        "accessible-label:",
        "accessible-action-default",
        "interaction.has-focus",
    ] {
        assert!(
            components.contains(required) || app.contains(required),
            "missing final accessibility primitive: {required}"
        );
    }

    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(report.accessibility.final_v0_ui_present);
    assert!(report.accessibility.is_honest_ready_base());
    assert!(!report.accessibility.wcag_conformance_claimed);
    assert!(!report.accessibility.release_ready);
}

#[test]
fn final_ui_performance_language_no_longer_claims_ui_absence() {
    let root = repo_root();
    let runtime =
        std::fs::read_to_string(root.join("crates/medscale-desktop/tests/runtime_perf_057.rs"))
            .expect("runtime perf harness");
    let methodology = std::fs::read_to_string(
        root.join("evidence/027-perf-sbom-release-evidence/PERF_METHODOLOGY.md"),
    )
    .expect("performance methodology");
    let marker = "FINAL_UI_PRESENT_EXTERNAL_INTERACTION_MEASUREMENT_REQUIRED";
    assert!(runtime.contains(marker));
    assert!(methodology.contains(marker));
    assert!(!runtime.contains("BLOCKED_BY_FINAL_V0_UI"));
    assert!(!methodology.contains("BLOCKED_BY_FINAL_V0_UI"));
    assert!(runtime.contains("budgets_claimed_met\": false"));
}

#[test]
fn external_accessibility_and_performance_packets_are_exact_and_non_claiming() {
    let root = repo_root();
    let a11y = std::fs::read_to_string(
        root.join("evidence/067-final-ui-product-qualification/ACCESSIBILITY_ACTION_PACKET.md"),
    )
    .expect("accessibility packet");
    for required in [
        "VoiceOver",
        "NVDA",
        "Orca",
        "Keyboard-only",
        "200%",
        "wcag_conformance_claimed=false",
    ] {
        assert!(
            a11y.contains(required),
            "missing accessibility action: {required}"
        );
    }

    let perf = std::fs::read_to_string(
        root.join("evidence/067-final-ui-product-qualification/PERFORMANCE_ACTION_PACKET.md"),
    )
    .expect("performance packet");
    for required in [
        "3 warmups + 30 timed runs",
        "p50",
        "p95",
        "<= 2000 ms",
        "<= 100 ms",
        "<= 250 MiB",
        "budgets_claimed_met=true",
    ] {
        assert!(
            perf.contains(required),
            "missing performance action: {required}"
        );
    }
}

#[test]
fn final_qualification_keeps_release_private_data_and_mesc_separate() {
    let root = repo_root();
    let spec =
        std::fs::read_to_string(root.join("specs/067-final-ui-product-qualification/spec.md"))
            .expect("Spec 067");
    assert!(spec.contains("Spec 012/MESC remains optional/deferred and untouched"));
    assert!(spec.contains("Repository-owned implementation completion MUST remain separate"));

    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(!report.release_qualification.release_ready);
    assert!(!report.vault_privacy.private_data_ready);
    assert!(!report.host_authority.multi_client_release_ready);
    assert!(!report.real_phi_authorized);
}

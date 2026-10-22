//! Spec 064 native utility-surface honesty regression binding.

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
fn audit_exports_settings_integrations_are_real_native_surfaces() {
    let root = repo_root();
    let app =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint")).expect("app");
    for label in [
        "Synthetic disclosure history · Core Host remains canonical",
        "Loss-aware FHIR support · No full conformance claim",
        "Doctor truth and evidence-gated readiness",
        "Brokered integration posture · Default deny",
        "Audit authority boundary",
        "FHIR R4 support matrix",
        "Settings are not evidence",
        "Integration boundary",
        "Missing release evidence",
    ] {
        assert!(app.contains(label), "missing 064 UI label: {label}");
    }
    assert!(app.contains("for item in root.utility-audit-rows"));
    assert!(app.contains("for item in root.utility-export-rows"));
    assert!(app.contains("for item in root.utility-settings-rows"));
    assert!(app.contains("for item in root.utility-integration-rows"));
}

#[test]
fn utility_boundary_reuses_doctor_fhir_disclosure_and_broker_truth() {
    let root = repo_root();
    let model =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/utility_surfaces.rs"))
            .expect("model");
    for term in [
        "DoctorReport",
        "DisclosureRecord",
        "SupportLevel::Qualified",
        "SupportLevel::Partial",
        "SupportLevel::Unsupported",
        "Network Broker",
        "Default deny",
        "Not authorized",
        "WCAG not claimed",
    ] {
        assert!(model.contains(term), "missing 064 boundary term: {term}");
    }
    assert!(!model.contains("label: \"MESC\""));
    assert!(!model.contains("Optional / deferred"));
}

#[test]
fn desktop_still_has_no_direct_storage_or_network_client() {
    let root = repo_root();
    let manifest =
        std::fs::read_to_string(root.join("crates/medscale-desktop/Cargo.toml")).expect("manifest");
    let main =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/main.rs")).expect("main");
    assert!(!manifest.contains("medscale-storage"));
    assert!(!manifest.contains("medscale-network"));
    assert!(!main.contains("ureq::"));
    assert!(!main.contains("reqwest::"));
}

#[test]
fn spec_064_keeps_release_phi_and_mesc_claims_honest() {
    let root = repo_root();
    let spec =
        std::fs::read_to_string(root.join("specs/064-audit-exports-settings-integrations/spec.md"))
            .expect("spec");
    let public_scope =
        std::fs::read_to_string(root.join("docs/qualification/INTEROPERABILITY_SCOPE.md"))
            .expect("public cross-project authority scope");
    assert!(spec.contains("Real PHI, production partner integrations"));
    assert!(spec.contains("release readiness remain out of scope"));
    assert!(public_scope.contains("not MedScale runtime authority"));
}

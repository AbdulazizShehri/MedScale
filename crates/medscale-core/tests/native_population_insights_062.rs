//! Spec 062 native Population Insights + assistant honesty regression binding.

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
fn insights_route_is_real_native_and_accessible() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("desktop app UI");

    for label in [
        "Evidence-aware population view · Synthetic demo",
        "Coverage state distribution",
        "MedScale Assistant",
        "Cohort explorer",
        "Assistant evidence context",
        "Risk distribution boundary",
        "Trend boundary",
        "Evidence gaps",
        "Review recommendation",
    ] {
        assert!(app.contains(label), "missing Insights label: {label}");
    }
    assert!(app.contains("for item in root.insights-coverage"));
    assert!(app.contains("for item in root.insights-cohorts"));
    assert!(app.contains("for item in root.insights-evidence"));
    assert!(app.contains("accessible-name: \"Population evidence assistant query\""));
    let components =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/components.slint"))
            .expect("desktop components");
    assert!(components.contains("accessible-label: root.accessible-name"));
}

#[test]
fn population_boundary_uses_trusted_contracts_without_new_authority() {
    let root = repo_root();
    let model =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/population_insights.rs"))
            .expect("population insights model");
    let manifest = std::fs::read_to_string(root.join("crates/medscale-desktop/Cargo.toml"))
        .expect("desktop manifest");

    for trusted in [
        "SubjectBriefV1",
        "SubjectCoverageV1",
        "LexicalRetrieveResult",
        "relevance_is_not_authority",
        "CoverageStatus::Conflict",
        "CoverageStatus::IncomparableUnits",
        "CoverageStatus::UnhealthyEvidence",
        "CoverageStatus::UnsupportedResourceType",
    ] {
        assert!(
            model.contains(trusted),
            "missing trusted boundary: {trusted}"
        );
    }
    assert!(model.contains("no qualified risk contract"));
    assert!(model.contains("not clinical care-gap claims"));
    assert!(!manifest.contains("medscale-storage"));
    assert!(!manifest.contains("medscale-network"));
}

#[test]
fn assistant_is_deterministic_and_no_provider_authority_is_added() {
    let root = repo_root();
    let model =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/population_insights.rs"))
            .expect("population insights model");
    let main = std::fs::read_to_string(root.join("crates/medscale-desktop/src/main.rs"))
        .expect("desktop main");

    assert!(model.contains("Evidence-context only; relevance is not clinical authority"));
    assert!(model.contains("RETRACTED"));
    assert!(main.contains("insights-assistant:"));
    assert!(!main.contains("ureq::"));
    assert!(!main.contains("reqwest::"));
}

#[test]
fn spec_062_keeps_mesc_phi_and_release_claims_out_of_scope() {
    let root = repo_root();
    let spec =
        std::fs::read_to_string(root.join("specs/062-population-insights-assistant-ux/spec.md"))
            .expect("Spec 062");
    let public_scope =
        std::fs::read_to_string(root.join("docs/qualification/INTEROPERABILITY_SCOPE.md"))
            .expect("public cross-project authority scope");

    assert!(spec.contains("Real PHI, MESC"));
    assert!(spec.contains("release-readiness claims remain out of scope"));
    assert!(public_scope.contains("not MedScale runtime authority"));
}

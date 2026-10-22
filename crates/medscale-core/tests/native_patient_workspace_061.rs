//! Spec 061 native patient-workspace honesty and regression binding.

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
fn patients_route_is_real_and_keyboard_reachable() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("desktop app UI");
    let components =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/components.slint"))
            .expect("desktop components");

    for label in [
        "Trusted longitudinal workspace · Synthetic demo",
        "Overview",
        "Timeline",
        "Labs",
        "Medications",
        "Documents",
        "Care Plan",
        "Sources",
        "Evidence sources",
        "Authority boundary",
    ] {
        assert!(
            app.contains(label),
            "missing patient workspace label: {label}"
        );
    }
    assert!(app.contains("for item in root.patient-timeline"));
    assert!(app.contains("for item in root.patient-labs"));
    assert!(app.contains("for item in root.patient-sources"));
    assert!(app.contains("Inspect sources"));
    assert!(app.contains("root.patient-tab = \"Sources\""));
    assert!(components.contains("export component SegmentTab"));
    assert!(components.contains("export component HonestyPanel"));
    assert!(components.contains("forward-focus: interaction"));
    assert!(components.contains("accessible-action-default"));
}

#[test]
fn patient_boundary_uses_trusted_contracts_and_fails_closed() {
    let root = repo_root();
    let model =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/patient_workspace.rs"))
            .expect("patient workspace model");
    let manifest = std::fs::read_to_string(root.join("crates/medscale-desktop/Cargo.toml"))
        .expect("desktop manifest");

    for trusted in [
        "SubjectTimelineV1",
        "SubjectBriefV1",
        "SubjectCoverageV1",
        "CoverageStatus::Conflict",
        "CoverageStatus::UnsupportedResourceType",
        "PatientWorkspaceError::SubjectMismatch",
    ] {
        assert!(
            model.contains(trusted),
            "missing trusted boundary: {trusted}"
        );
    }
    assert!(manifest.contains("medscale-contracts = { workspace = true }"));
    assert!(!manifest.contains("medscale-storage"));
    assert!(!manifest.contains("medscale-network"));
    assert!(!manifest.contains("ureq"));
}

#[test]
fn unsupported_capabilities_are_explicit_non_claims() {
    let root = repo_root();
    let model =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/patient_workspace.rs"))
            .expect("patient workspace model");

    assert!(model.contains("Unsupported trusted projection"));
    assert!(model.contains("no medication regimen is inferred"));
    assert!(model.contains("does not imply OCR/ASR completeness"));
    assert!(model.contains("no authority-changing action is committed here"));
    assert!(model.contains("Synthetic fixture evidence"));
}

#[test]
fn spec_061_keeps_mesc_and_release_claims_out_of_scope() {
    let root = repo_root();
    let spec =
        std::fs::read_to_string(root.join("specs/061-patient-workspace-longitudinal-ux/spec.md"))
            .expect("Spec 061");
    let promotion =
        std::fs::read_to_string(root.join("docs/qualification/promotions/SPEC_061_PROMOTION.md"))
            .expect("Spec 061 promotion");

    assert!(spec.contains("MESC"));
    assert!(spec.contains("release-readiness claims remain out of scope"));
    assert!(promotion.contains("not a medical, security, clinical or release certification"));
    assert!(promotion.contains("Detailed operator decisions and plans are not published here"));
}

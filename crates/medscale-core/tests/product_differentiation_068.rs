//! Spec 068 product differentiation and honesty regression.

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
fn historical_spec_068_rejected_multicolor_palette_remains_retired() {
    let root = repo_root();
    let theme = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/theme.slint"))
        .expect("theme");
    let design = std::fs::read_to_string(root.join("DESIGN.md")).expect("design");

    for rejected_hex in ["#5B5CF6", "#FF785C", "#D8B4FE", "#2F4F46"] {
        assert!(
            !theme.contains(rejected_hex),
            "rejected palette returned: {rejected_hex}"
        );
    }
    assert!(theme.contains("brand-black: #000000"));
    assert!(theme.contains("brand-white: #FFFFFF"));
    assert!(theme.contains("signal: ink"));
    for retired_name in ["blurple", "coral", "lavender", "pine", "mint", "amber"] {
        assert!(
            !theme.contains(retired_name),
            "retired palette token returned: {retired_name}"
        );
    }
    assert!(design.contains("SUPERSEDED_BY_SPEC_068"));
    assert!(design.contains("SUPERSEDED_BY_SPEC_073"));
    assert!(design.contains("SUPERSEDED_BY_SPEC_095"));
    assert!(!design.contains("Cohere-inspired"));
}

#[test]
fn models_and_competitive_evidence_are_first_class_and_truthful() {
    let root = repo_root();
    let app =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint")).expect("app");
    let product =
        std::fs::read_to_string(root.join("crates/medscale-desktop/src/product_intelligence.rs"))
            .expect("product intelligence");

    assert!(app.contains("label: \"Models\""));
    assert!(app.contains("label: \"Evidence\""));
    assert!(app.contains("MedScale Evidence Center"));
    assert!(app.contains("MedScale evidence · measured claims only · limitations explicit"));
    assert!(app.contains("model-runtime-summary"));
    assert!(product.contains("real portable ONNX runtime"));
    assert!(product.contains("Model Center operator session"));
    assert!(product.contains("Try a model evaluates Packs in a separate workspace session"));
    assert!(product.contains("Admission in one session does not imply admission in the other"));
    assert!(product.contains("production clinical model not promoted"));
    assert!(product.contains("verdict: \"UNMEASURED\""));
    assert!(product.contains("verdict: \"STRUCTURAL ONLY\""));
    assert!(product.contains("verdict: \"ANTI-METRIC\""));
    assert!(!product.contains("verdict: \"PROVEN ADVANTAGE\""));
    assert!(!product.contains("verdict: \"OPENMED AHEAD\""));
    assert!(!product.contains("beats OpenMed"));
}

#[test]
fn home_is_a_clinical_workspace_not_an_admin_dashboard() {
    let root = repo_root();
    let app =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint")).expect("app");
    let review = std::fs::read_to_string(
        root.join("evidence/068-product-differentiation-rebuild/ABRIDGE_PRODUCT_REVIEW.md"),
    )
    .expect("Abridge product review");

    let home =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/command-center.slint"))
            .expect("Command Center");
    assert!(app.contains("if root.active-route == \"Home\": CommandCenter"));
    assert!(app.contains("coverage-summary: root.patient-coverage-summary"));
    assert!(app.contains("model-runtime-summary: root.model-runtime-summary"));
    assert!(home.contains("root.patient-name"));
    assert!(home.contains("Timeline, coverage and original sources"));
    assert!(home.contains("root.coverage-summary"));
    assert!(home.contains("root.navigate(\"Patients\")"));
    assert!(home.contains("root.navigate(\"Evidence\")"));
    assert!(home.contains("state: \"unknown\""));
    assert!(home.contains("state: \"unmeasured\""));
    assert!(home.contains("Synthetic demo"));
    assert!(!app.contains("Operational snapshot"));
    assert!(review.contains("the clinical work itself is the interface"));
    assert!(review.contains("Prepare → Understand → Act"));
}

#[test]
fn spec_068_closure_remains_historical_after_spec_073_closure() {
    let status = std::fs::read_to_string(
        repo_root().join("docs/qualification/PRODUCT_QUALIFICATION_STATUS.md"),
    )
    .expect("completion status");
    // A new program changes active status; it does not reopen historical closures.
    let queue =
        std::fs::read_to_string(repo_root().join("docs/qualification/HISTORICAL_CLOSURE_INDEX.md"))
            .expect("build queue");
    for spec in ["068", "073"] {
        assert!(queue.lines().any(|line| {
            line.contains(&format!("| {spec} |")) && line.contains("CLOSED_CANONICAL")
        }));
    }
    assert!(status.contains("MEDSCALE_RELEASE_READY = FALSE"));
    assert!(status.contains("## Spec 068 canonical closure"));
    assert!(status.contains("## Spec 073 canonical closure"));
}

#[test]
fn brand_identity_is_canonical_and_runtime_bound() {
    let root = repo_root();
    let app =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint")).expect("app");
    let mark =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/assets/medscale-mark.svg"))
            .expect("mark");
    let brand =
        std::fs::read_to_string(root.join("docs/brand/BRAND_IDENTITY_SYSTEM.md")).expect("brand");
    let typography =
        std::fs::read_to_string(root.join("docs/brand/TYPOGRAPHY_SYSTEM.md")).expect("typography");
    let logo = std::fs::read_to_string(root.join("docs/brand/LOGO_SPEC.md")).expect("logo");

    assert!(app.contains("default-font-family: Theme.font-ui"));
    assert!(brand.contains("Clinical Intelligence OS"));
    assert!(brand.contains("Evidence first. Action second."));
    assert!(typography.contains("Inter"));
    assert!(typography.contains("JetBrains Mono"));
    // Logo 12 (2026-10-19) supersedes the equilateral paired-M construction.
    assert!(logo.contains("FOUNDER_APPROVED_LOGO_12"));
    assert!(logo.to_lowercase().contains("three upright frames"));
    assert!(logo.contains("16 px"));
    assert!(logo.contains("clear space"));
    assert!(!mark.contains("<circle"));
    assert!(mark.contains("#000000"));
    assert!(mark.contains("id=\"medscale-frames\""));
    assert!(!mark.contains("#0A66FF"));
}

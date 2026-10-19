//! Spec 073 final UI polish, identity, and scope regressions.

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
fn approved_monochrome_mark_is_runtime_bound() {
    let root = repo_root();
    for asset in [
        "crates/medscale-desktop/ui/assets/medscale-mark.svg",
        "crates/medscale-desktop/ui/assets/medscale-app-icon.svg",
    ] {
        let svg = std::fs::read_to_string(root.join(asset)).expect("logo asset");
        // Founder-approved Logo 12 (2026-10-19) supersedes the Spec 095 paired mark and the historical circle.
        assert!(
            !svg.contains("<circle"),
            "superseded field returned: {asset}"
        );
        assert!(svg.contains("#000000"), "approved black missing: {asset}");
        assert!(
            svg.contains("id=\"medscale-frames\""),
            "MedScale Logo 12 frames missing: {asset}"
        );
        for forbidden in ["#0A66FF", "purple", "gradient", "heartbeat", "sparkle"] {
            assert!(
                !svg.to_lowercase().contains(&forbidden.to_lowercase()),
                "forbidden logo treatment returned in {asset}: {forbidden}"
            );
        }
    }
}

#[test]
fn adaptive_theme_and_admitted_typography_are_active() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    let theme = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/theme.slint"))
        .expect("theme");
    let typography =
        std::fs::read_to_string(root.join("docs/brand/TYPOGRAPHY_SYSTEM.md")).expect("typography");

    assert!(theme.contains("Palette.color-scheme == ColorScheme.dark"));
    assert!(theme.contains("signal: ink"));
    assert!(theme.contains("font-ui: \"Inter\""));
    assert!(theme.contains("font-mono: \"JetBrains Mono NL\""));
    assert!(app.contains("default-font-family: Theme.font-ui"));
    assert!(app.contains("InterVariable.ttf"));
    assert!(app.contains("JetBrainsMonoNL-Regular.ttf"));
    assert!(!app.contains("SourceSerif4-Regular.ttf"));
    assert!(!app.contains("default-font-family: \"Geist\""));
    assert!(typography.contains("CANONICAL_SPEC_095"));
    assert!(typography.contains("Inter"));
    assert!(typography.contains("JetBrains Mono"));
}

#[test]
fn dual_dock_and_custom_icon_family_cover_product_areas() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    let components =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/components.slint"))
            .expect("components");

    assert!(app.contains("rail := Rectangle"));
    assert!(app.contains("sidebar := Rectangle"));
    assert!(components.contains("export component RailAction"));
    for icon in [
        "home.svg",
        "patients.svg",
        "intelligence.svg",
        "workflows.svg",
        "governance.svg",
        "commands.svg",
        "settings.svg",
    ] {
        assert!(
            root.join("crates/medscale-desktop/ui/assets/icons")
                .join(icon)
                .is_file(),
            "missing custom icon: {icon}"
        );
        assert!(app.contains(icon), "icon not used by native shell: {icon}");
    }
    assert!(components.contains("accessible-role: button"));
    assert!(components.contains("interaction.has-focus"));
}

#[test]
fn web_reference_is_explicitly_documentation_only() {
    let root = repo_root();
    let readme = std::fs::read_to_string(root.join("docs/brand/web-reference/README.md"))
        .expect("web reference README");
    let html = std::fs::read_to_string(root.join("docs/brand/web-reference/reference.html"))
        .expect("web reference HTML");
    assert!(readme.contains("NON_PRODUCTION_REFERENCE"));
    assert!(readme.contains("not a production Web application"));
    assert!(html.contains("NON_PRODUCTION_REFERENCE"));
    assert!(html.contains("medscale-mark-white.svg"));
    assert!(html.contains("medscale-favicon.svg"));
    for forbidden in ["package.json", "vite.config", "next.config", "node_modules"] {
        assert!(
            !root
                .join("docs/brand/web-reference")
                .join(forbidden)
                .exists(),
            "web runtime artifact is not authorized: {forbidden}"
        );
    }
}

#[test]
fn spec_073_keeps_release_truth_and_mesc_separate() {
    let root = repo_root();
    let spec =
        std::fs::read_to_string(root.join("specs/073-final-ui-polish/spec.md")).expect("Spec 073");
    assert!(spec.contains("MESC project separation"));
    assert!(spec.contains("MESC work"));
    assert!(spec.contains("production Web runtime"));

    let report = build_doctor_report(None, false, privacy_proof_artifact_present());
    assert!(!report.release_qualification.release_ready);
    assert!(!report.real_phi_authorized);
    assert_eq!(report.mesc_artifact.disposition, "SEPARATE_PROJECT");
    assert_eq!(report.mesc_artifact.gate, "NONE");
    assert_eq!(report.mesc_artifact.integration_status, "OUT_OF_SCOPE");
}

#[test]
fn cli_identity_is_human_facing_without_json_brand_decorations() {
    let root = repo_root();
    let cli =
        std::fs::read_to_string(root.join("crates/medscale-cli/src/main.rs")).expect("CLI source");
    assert!(cli.contains("MedScale · evidence-first local clinical intelligence CLI"));
    assert!(cli.contains("human_heading(\"capability map\")"));
    assert!(cli.contains("serde_json::to_string_pretty(&rows)"));
    assert!(cli.contains("assert!(!json.contains(\"MedScale · capability map\"))"));
}

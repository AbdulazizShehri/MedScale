//! Spec 060 native Desktop design-system and final-v0 honesty regression.

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
fn native_desktop_shell_is_pinned_non_webview_and_branded() {
    let root = repo_root();
    let workspace = std::fs::read_to_string(root.join("Cargo.toml")).expect("workspace manifest");
    let desktop = std::fs::read_to_string(root.join("crates/medscale-desktop/Cargo.toml"))
        .expect("desktop manifest");
    let app = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("desktop app UI");
    let theme = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/theme.slint"))
        .expect("desktop theme");
    let mark =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/assets/medscale-mark.svg"))
            .expect("MedScale mark");
    let components =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/components.slint"))
            .expect("desktop components");

    assert!(workspace.contains("slint = { version = \"=1.16.1\""));
    assert!(workspace.contains("\"accessibility\""));
    assert!(workspace.contains("rust-version = \"1.88\""));
    assert!(workspace.contains("\"renderer-femtovg\""));
    assert!(desktop.contains("slint = { workspace = true }"));
    assert!(
        !desktop
            .lines()
            .any(|line| line.trim_start().starts_with("tauri ="))
    );
    assert!(
        !workspace
            .lines()
            .any(|line| line.trim_start().starts_with("tauri ="))
    );

    for required in [
        "Home",
        "Patients",
        "Insights",
        "Workflows",
        "Messages",
        "Tasks",
        "Documents",
        "Audit Trail",
        "Exports",
        "Settings",
        "About",
        "AboutSlint",
        "Command palette",
        "Synthetic demo",
    ] {
        assert!(
            app.contains(required),
            "missing Desktop shell label: {required}"
        );
    }
    assert!(app.contains("accessible-label"));
    assert!(app.contains("event.modifiers.control || event.modifiers.meta"));
    assert!(components.contains("FocusScope"));
    assert!(components.contains("accessible-action-default"));
    assert!(components.contains("interaction.has-focus ? Theme.focus-width"));
    assert!(theme.contains("focus-width: 2px"));
    assert!(components.contains("event.text == \" \" || event.text == \"\\n\""));
    // Historical Spec 060 requires a branded native shell, not a frozen legacy palette.
    // Spec 095 replaces visual assets forward while preserving native-shell semantics.
    assert!(theme.contains("brand-black: #000000"));
    assert!(theme.contains("brand-white: #FFFFFF"));
    assert!(theme.contains("font-ui: \"Inter\""));
    assert!(app.contains("default-font-family: Theme.font-ui"));
    assert!(mark.contains("id=\"medscale-frames\""));
    assert!(mark.contains("#000000"));
    assert!(!mark.contains("#0A66FF"));
    assert!(!mark.contains("<circle"));
    assert!(!mark.contains("<rect"));
    let deny = std::fs::read_to_string(root.join("deny.toml")).expect("deny config");
    assert!(deny.contains("LicenseRef-Slint-Royalty-free-2.0"));
    assert!(deny.contains("clipboard-win@5.4.1"));
    assert!(deny.contains("error-code@3.4.0"));
    assert!(deny.contains("libfuzzer-sys@0.4.13"));
    assert!(!deny.contains("dwrote@0.11.5"));
    assert!(app.contains("AboutSlint"));

    let lock = std::fs::read_to_string(root.join("Cargo.lock")).expect("Cargo lock");
    assert!(!lock.contains("name = \"lru\"\nversion = \"0.16.4\""));
    assert!(lock.contains("name = \"smol_str\""));
    assert!(lock.contains("version = \"0.3.2\""));
    assert!(lock.contains("name = \"typed-index-collections\""));
    assert!(lock.contains("version = \"3.3.0\""));

    let notice = std::fs::read_to_string(root.join("docs/legal/NOTICE_INVENTORY.md"))
        .expect("NOTICE inventory");
    assert!(notice.contains("| slint | 1.16.1 |"));
    assert!(notice.contains("| femtovg | 0.23.2 |"));
    assert!(notice.contains("| accesskit | 0.22.0 |"));
    assert!(!notice.contains("| slint | 1.13.1 |"));
    assert!(!notice.contains("| lru | 0.16.4 |"));
}

#[test]
fn doctor_reports_native_final_v0_without_inventing_wcag_or_release() {
    let report = build_doctor_report(None, false, false);
    assert_eq!(report.desktop_shell, "slint_native");
    assert!(!report.tauri_admitted);
    assert!(report.accessibility.final_v0_ui_present);
    assert!(report.accessibility.is_honest_ready_base());
    assert!(!report.accessibility.wcag_conformance_claimed);
    assert!(!report.accessibility.release_ready);
    assert!(!report.release_qualification.release_ready);
    assert!(
        report
            .release_qualification
            .missing_evidence_classes
            .contains(&"wcag_final_v0_ui_accessibility_qualification".to_owned())
    );
}

#[test]
fn product_phase_keeps_mobile_after_desktop_cli_launch() {
    let root = repo_root();
    let product = std::fs::read_to_string(root.join("PRODUCT.md")).expect("product truth");
    let design = std::fs::read_to_string(root.join("DESIGN.md")).expect("design truth");
    let queue =
        std::fs::read_to_string(root.join("docs/qualification/HISTORICAL_CLOSURE_INDEX.md"))
            .expect("build queue");
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("CI workflow");

    assert!(product.contains("Desktop and CLI are the launch surfaces"));
    assert!(product.contains("Mobile applications come only after Desktop + CLI launch"));
    // The active program preserves the quality rules rather than an old methodology label.
    for requirement in [
        "visible focus",
        "Measure contrast from actual tokens",
        "inspect actual native pixels",
        "source assertions do not qualify visuals",
        "Rust Core retains all authority",
    ] {
        assert!(
            design.contains(requirement),
            "missing design quality rule: {requirement}"
        );
    }
    assert!(queue.contains("065 | CLI Product Experience + Capability Parity"));
    assert!(queue.contains("Mobile remains deferred until Desktop+CLI launch"));
    assert!(ci.contains("Install Linux native Desktop build dependencies"));
    assert!(ci.contains("libfontconfig1-dev"));
}

//! Spec 066 Desktop + CLI hardening regression binding.

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
fn every_advertised_route_is_honest_and_documents_is_real() {
    let root = repo_root();
    let ui = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    for route in [
        "Patients",
        "Insights",
        "Workflows",
        "Tasks",
        "Messages",
        "Documents",
        "Audit Trail",
        "Exports",
        "Settings",
        "Integrations",
        "About",
        "Home",
    ] {
        let needle = format!("root.active-route == \"{route}\"");
        assert!(ui.contains(&needle), "route is not handled: {route}");
    }
    assert!(ui.contains("Documents — bounded intake"));
    assert!(ui.contains("Custody is not understanding"));
    assert!(!ui.contains("This surface is scheduled in a later Desktop slice."));
}

#[test]
fn home_demo_does_not_invent_clinical_risk_or_real_roster() {
    let root = repo_root();
    let ui = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    assert!(!ui.contains("High risk"));
    for real_like in [
        "Sarah Chen",
        "Michael Torres",
        "Emily Watson",
        "James Miller",
    ] {
        assert!(
            !ui.contains(real_like),
            "real-looking demo name remains: {real_like}"
        );
    }
    let home =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/command-center.slint"))
            .expect("Command Center");
    assert!(home.contains("Synthetic demo"));
    assert!(home.contains("not a clinical risk ranking"));
    assert!(home.contains("Review before anything consequential"));
    assert!(home.contains("root.patient-name"));
    assert!(home.contains("root.coverage-summary"));
}

#[test]
fn window_floor_and_command_copy_are_hardened() {
    let root = repo_root();
    let ui = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");
    assert!(ui.contains("min-width: 1100px"));
    assert!(ui.contains("min-height: 720px"));
    assert!(ui.contains("label: \"Commands\""));
    assert!(!ui.contains("Search patients, notes, commands, or ask MedScale"));
}

#[test]
fn ci_keeps_pr_and_main_gates_without_duplicate_spec_push() {
    let root = repo_root();
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("CI");
    assert!(ci.contains("branches: [main]"));
    assert!(ci.contains("pull_request:"));
    assert!(!ci.contains("spec/**"));
    for required in [
        "rust (${{ matrix.os }})",
        "perf delivery-plan scale (windows)",
        "cargo-deny",
        "supply-chain policy present",
    ] {
        assert!(
            ci.contains(required),
            "required job identity missing: {required}"
        );
    }
}

fn srgb_luminance(rgb: [u8; 3]) -> f64 {
    let channel = |value: u8| {
        let normalized = f64::from(value) / 255.0;
        if normalized <= 0.04045 {
            normalized / 12.92
        } else {
            ((normalized + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2])
}

fn contrast_ratio(foreground: [u8; 3], background: [u8; 3]) -> f64 {
    let a = srgb_luminance(foreground);
    let b = srgb_luminance(background);
    let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };
    (lighter + 0.05) / (darker + 0.05)
}

fn theme_color(theme: &str, token: &str, dark: bool) -> [u8; 3] {
    fn resolve(theme: &str, token: &str, dark: bool, depth: usize) -> [u8; 3] {
        assert!(depth < 12, "cyclic or excessive color aliases: {token}");
        let prefix = format!("out property <color> {token}:");
        let expression = theme
            .lines()
            .find_map(|line| line.trim().strip_prefix(&prefix))
            .unwrap_or_else(|| panic!("missing active color token: {token}"))
            .split(';')
            .next()
            .expect("color expression")
            .trim();
        let selected = if let Some(adaptive) = expression.strip_prefix("dark ?") {
            let (dark_value, light_value) = adaptive.split_once(':').expect("adaptive color pair");
            let value = if dark { dark_value } else { light_value };
            value.trim()
        } else {
            expression
        };
        if let Some(hex) = selected.strip_prefix('#') {
            assert_eq!(hex.len(), 6, "contrast requires opaque RGB: {token}");
            std::array::from_fn(|index| {
                u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).expect("RGB channel")
            })
        } else {
            resolve(theme, selected, dark, depth + 1)
        }
    }
    resolve(theme, token, dark, 0)
}

#[test]
fn primary_text_tokens_keep_engineering_contrast_floor_without_wcag_claim() {
    let root = repo_root();
    let theme = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/theme.slint"))
        .expect("theme");
    let components =
        std::fs::read_to_string(root.join("crates/medscale-desktop/ui/components.slint"))
            .expect("components");
    let ui = std::fs::read_to_string(root.join("crates/medscale-desktop/ui/app.slint"))
        .expect("Desktop UI");

    // Measure the actual adaptive values, including aliases, instead of old palette literals.
    for dark in [false, true] {
        for background in [
            "canvas",
            "surface",
            "surface-raised",
            "surface-recessed",
            "sidebar",
            "sidebar-selected",
            "signal-soft",
        ] {
            for foreground in ["ink", "ink-subtle", "ink-quiet", "signal-strong"] {
                let ratio = contrast_ratio(
                    theme_color(&theme, foreground, dark),
                    theme_color(&theme, background, dark),
                );
                assert!(
                    ratio >= 4.5,
                    "{foreground}/{background}, dark={dark}: contrast {ratio:.3} below engineering floor"
                );
            }
            let focus_ratio = contrast_ratio(
                theme_color(&theme, "focus", dark),
                theme_color(&theme, background, dark),
            );
            assert!(focus_ratio >= 3.0, "focus/{background}, dark={dark}");
        }
        for background in ["obsidian", "graphite", "graphite-raised", "nav-selected"] {
            for foreground in ["nav-text", "nav-subtle"] {
                assert!(
                    contrast_ratio(
                        theme_color(&theme, foreground, dark),
                        theme_color(&theme, background, dark),
                    ) >= 4.5,
                    "{foreground}/{background} below rail contrast floor"
                );
            }
        }
        assert!(
            contrast_ratio(
                theme_color(&theme, "selection-foreground", dark),
                theme_color(&theme, "selection-background", dark),
            ) >= 4.5,
            "selected text below contrast floor, dark={dark}"
        );
    }

    assert!(components.contains("color: Theme.ink;"));
    assert!(!ui.contains("color: Theme.warning;"));
    assert!(!ui.contains("color: Theme.success;"));
    assert!(!ui.contains("color: Theme.danger;"));
}

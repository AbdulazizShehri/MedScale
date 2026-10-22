//! Brand asset, redistribution, and non-color presentation contracts.
//! Logo 12 (founder-approved 2026-10-19) supersedes the Spec 095 paired mark.
//! The wordmark follows the founder amendment of 2026-10-22: filled outlines.
//! These deterministic checks do not establish rendered UI or WCAG qualification.

use medscale_contracts::objects::DigestSha256;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn attribute<'a>(element: &'a str, name: &str) -> &'a str {
    element
        .split_once(&format!(" {name}=\""))
        .unwrap_or_else(|| panic!("missing {name} attribute"))
        .1
        .split_once('"')
        .expect("closing attribute quote")
        .0
}

fn numbers(value: &str) -> Vec<f64> {
    value
        .split(|character: char| {
            character.is_ascii_alphabetic()
                || character.is_whitespace()
                || matches!(character, ',' | '(' | ')')
        })
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().expect("finite numeric geometry"))
        .inspect(|value: &f64| assert!(value.is_finite()))
        .collect()
}

/// The single group with `id`, plus the `d` attribute of every path inside it.
fn group<'a>(svg: &'a str, id: &str) -> (&'a str, Vec<&'a str>) {
    let marker = format!("<g id=\"{id}\"");
    assert_eq!(svg.matches(&marker).count(), 1, "exactly one {id} group");
    let start = svg.find(&marker).expect("group start");
    let open_end = start + svg[start..].find('>').expect("group open tag end") + 1;
    let close = open_end + svg[open_end..].find("</g>").expect("group close");
    let paths = svg[open_end..close]
        .split("<path ")
        .skip(1)
        .map(|path| {
            assert!(
                !path.contains("stroke-width") && !path.contains("fill="),
                "per-path weight or fill breaks the uniform stroke"
            );
            let d = path.split_once("d=\"").expect("path data").1;
            d.split_once('"').expect("path data end").0
        })
        .collect();
    (&svg[start..open_end], paths)
}

const FRAMES: &str = "medscale-frames";
const WORDMARK: &str = "medscale-wordmark";

fn assert_monochrome_stroke(open_tag: &str, color: &str) {
    assert_eq!(attribute(open_tag, "stroke"), color);
    assert_eq!(attribute(open_tag, "fill"), "none");
    assert_eq!(attribute(open_tag, "stroke-linejoin"), "round");
    let width: f64 = attribute(open_tag, "stroke-width")
        .parse()
        .expect("numeric stroke width");
    assert!(width > 0.0 && width.is_finite());
}

fn assert_monochrome_fill(open_tag: &str, color: &str) {
    assert_eq!(attribute(open_tag, "fill"), color);
    assert!(
        !open_tag.contains("stroke"),
        "the wordmark is filled artwork, never outlined"
    );
}

fn assert_no_decoration(svg: &str, asset: &str) {
    for forbidden in [
        "<circle",
        "<linearGradient",
        "<radialGradient",
        "<filter",
        "<image",
        "<text",
        "opacity",
        "medscale-paired-m",
        "M0 86.60254",
    ] {
        assert!(
            !svg.contains(forbidden),
            "unapproved treatment in {asset}: {forbidden}"
        );
    }
    for (index, _) in svg.match_indices('#') {
        let color = &svg[index..index + 7];
        assert!(
            matches!(color, "#000000" | "#FFFFFF"),
            "non-monochrome colour in {asset}: {color}"
        );
    }
}

#[test]
fn approved_logo_12_symbol_is_three_stroked_frames_with_the_interlocked_lower_stroke() {
    let black = read("assets/brand/mark-black.svg");
    let white = read("assets/brand/mark-white.svg");
    let (black_tag, black_paths) = group(&black, FRAMES);
    let (white_tag, white_paths) = group(&white, FRAMES);
    assert_monochrome_stroke(black_tag, "#000000");
    assert_monochrome_stroke(white_tag, "#FFFFFF");
    assert_eq!(black_paths, white_paths, "inverse geometry drift");
    assert_eq!(
        attribute(black_tag, "stroke-width"),
        attribute(white_tag, "stroke-width")
    );
    assert_no_decoration(&black, "mark-black.svg");
    assert_no_decoration(&white, "mark-white.svg");
    assert!(!black.contains("<rect"), "the symbol master is transparent");
    // Frame A is closed; frames B and C are one open stroke whose shared lower
    // stroke descends from C's lower-right corner to B's lower-left corner,
    // with C's left side continuing below it as the descender.
    assert_eq!(black_paths.len(), 2);
    assert!(black_paths[0].ends_with('Z'), "back frame is closed");
    assert!(!black_paths[1].contains('Z'), "interlocked pair stays open");
    for d in &black_paths {
        assert!(
            d.chars()
                .filter(char::is_ascii_alphabetic)
                .all(|c| matches!(c, 'M' | 'L' | 'Q' | 'Z'))
        );
        // Curve control points may sit outside the stroke bounds; the path
        // start (the descender foot or frame A's left side) may not.
        let view_box = numbers(attribute(&black, "viewBox"));
        let start = numbers(d);
        assert!(
            (0.0..=view_box[2]).contains(&start[0]) && (0.0..=view_box[3]).contains(&start[1]),
            "normalized to the symbol's own bounds"
        );
    }
    let corners = |d: &str| d.matches('Q').count();
    assert_eq!(
        corners(black_paths[0]),
        4,
        "four rounded corners on frame A"
    );
    assert_eq!(
        corners(black_paths[1]),
        6,
        "six rounded corners on frames B and C"
    );
    let coordinates = numbers(black_paths[1]);
    let descender_bottom = coordinates[1];
    assert!(
        coordinates
            .chunks_exact(2)
            .all(|point| point[1] <= descender_bottom),
        "the front frame's descender is the lowest stroke"
    );
}

#[test]
fn every_asset_reuses_the_identical_symbol_and_wordmark_geometry() {
    let mark = read("assets/brand/mark-black.svg");
    let (_, symbol) = group(&mark, FRAMES);
    let wordmark_svg = read("assets/brand/medscale-wordmark-black.svg");
    let (wordmark_tag, wordmark) = group(&wordmark_svg, WORDMARK);
    assert_monochrome_fill(wordmark_tag, "#000000");
    assert_eq!(
        wordmark.len(),
        8,
        "M e d S c a l e: one filled outline per letter"
    );
    let white_svg = read("assets/brand/medscale-wordmark-white.svg");
    let (white_tag, white) = group(&white_svg, WORDMARK);
    assert_monochrome_fill(white_tag, "#FFFFFF");
    assert_eq!(white, wordmark, "inverse wordmark drift");
    for (master, native) in [
        (
            "assets/brand/mark-black.svg",
            "crates/medscale-desktop/ui/assets/medscale-mark.svg",
        ),
        (
            "assets/brand/mark-white.svg",
            "crates/medscale-desktop/ui/assets/medscale-mark-white.svg",
        ),
        (
            "assets/brand/medscale-app-icon-dark.svg",
            "crates/medscale-desktop/ui/assets/medscale-app-icon.svg",
        ),
        (
            "assets/brand/medscale-app-icon-light.svg",
            "crates/medscale-desktop/ui/assets/medscale-app-icon-light.svg",
        ),
        (
            "assets/brand/medscale-favicon.svg",
            "apps/desktop-tauri/public/favicon.svg",
        ),
    ] {
        assert_eq!(read(master), read(native), "runtime asset drift: {native}");
    }
    for asset in [
        "medscale-wordmark-white.svg",
        "medscale-app-icon-dark.svg",
        "medscale-app-icon-light.svg",
        "medscale-icon-small-dark.svg",
        "medscale-icon-medium-dark.svg",
        "medscale-app-icon-macos.svg",
        "medscale-favicon.svg",
        "medscale-logo-canonical.svg",
        "medscale-logo-inverse.svg",
        "medscale-horizontal-black.svg",
        "medscale-horizontal-white.svg",
        "medscale-stacked-black.svg",
        "medscale-stacked-white.svg",
    ] {
        let svg = read(&format!("assets/brand/{asset}"));
        assert_no_decoration(&svg, asset);
        if svg.contains(FRAMES) {
            assert_eq!(group(&svg, FRAMES).1, symbol, "symbol drift: {asset}");
        }
        if svg.contains(WORDMARK) {
            let (tag, paths) = group(&svg, WORDMARK);
            assert_eq!(paths, wordmark, "wordmark drift: {asset}");
            assert!(!tag.contains("stroke"), "filled wordmark: {asset}");
            assert!(matches!(attribute(tag, "fill"), "#000000" | "#FFFFFF"));
        }
    }
    for color in ["black", "white"] {
        for layout in ["horizontal", "stacked"] {
            let svg = read(&format!("assets/brand/medscale-{layout}-{color}.svg"));
            assert!(svg.contains(FRAMES) && svg.contains(WORDMARK));
            assert!(!svg.contains("<rect"), "lockups are transparent");
        }
    }
    // The desktop frontend draws the same geometry, not a re-drawing of it.
    let ui = read("apps/desktop-tauri/src/components/brand.tsx");
    for d in symbol.iter().chain(&wordmark) {
        assert!(ui.contains(d), "desktop brand component geometry drift");
    }
}

#[test]
fn application_icons_center_the_symbol_with_clear_space() {
    let mark = numbers(attribute(&read("assets/brand/mark-black.svg"), "viewBox"));
    for (variant, background) in [("dark", "#000000"), ("light", "#FFFFFF")] {
        let svg = read(&format!("assets/brand/medscale-app-icon-{variant}.svg"));
        let view_box = numbers(attribute(&svg, "viewBox"));
        assert_eq!(view_box, vec![0.0, 0.0, 1024.0, 1024.0]);
        assert!(svg.contains(&format!(
            "<rect width=\"1024\" height=\"1024\" fill=\"{background}\"/>"
        )));
        let (tag, _) = group(&svg, FRAMES);
        let transform = numbers(attribute(tag, "transform"));
        assert_eq!(transform.len(), 3, "translation plus uniform scale");
        let (x, y, scale) = (transform[0], transform[1], transform[2]);
        let (width, height) = (mark[2] * scale, mark[3] * scale);
        assert!(
            (x - (1024.0 - width - x)).abs() < 0.01,
            "centred horizontally"
        );
        assert!(
            (y - (1024.0 - height - y)).abs() < 0.01,
            "centred vertically"
        );
        assert!(y >= 1024.0 * 0.15, "clear space above and below the symbol");
    }
    let wordmark = read("third_party/provenance/medscale-wordmark-2026-10-22.md");
    for required in [
        "2026-10-22",
        "InterVariable.ttf",
        "SIL Open Font License",
        "0.4542",
    ] {
        assert!(
            wordmark.contains(required),
            "wordmark provenance: {required}"
        );
    }
    // The ascender cut uses the symbol's own top slope.
    let geometry = read("assets/brand/source/geometry.json");
    assert!(geometry.contains("\"ts\": -0.4542") && geometry.contains("\"cut_slope\": 0.4542"));
    let provenance = read("third_party/provenance/logo-12-approved-identity.md");
    assert!(provenance.contains("2026-10-19"));
    assert!(provenance.to_lowercase().contains("no font"));
    assert!(
        provenance.contains("16"),
        "small-size qualification stays explicit"
    );
}

#[test]
fn complete_font_and_license_bytes_match_the_immutable_admission() {
    let provenance = read("third_party/provenance/095-native-typography.md");
    for (file, expected) in [
        (
            "InterVariable.ttf",
            "4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf",
        ),
        (
            "Inter-OFL.txt",
            "262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a",
        ),
        (
            "JetBrainsMonoNL-Regular.ttf",
            "fb3b2575d7b0657359707993288f12a7360344d39387bb26050e276d61f6bd2a",
        ),
        (
            "JetBrainsMono-OFL.txt",
            "30f0c136e3c88e422d0791acd97238870f9054a9729bc34cf2ff0d4ed8cac4ad",
        ),
    ] {
        let bytes = std::fs::read(repo_root().join("assets/brand/fonts").join(file))
            .expect("admitted font or license bytes");
        assert_eq!(
            DigestSha256::of(&bytes).to_hex(),
            expected,
            "asset drift: {file}"
        );
        assert!(provenance.contains(file));
        assert!(provenance.contains(expected));
    }
    for license in ["Inter-OFL.txt", "JetBrainsMono-OFL.txt"] {
        assert!(read(&format!("assets/brand/fonts/{license}")).contains("SIL OPEN FONT LICENSE"));
    }
    let app = read("crates/medscale-desktop/ui/app.slint");
    for font in ["InterVariable.ttf", "JetBrainsMonoNL-Regular.ttf"] {
        assert!(app.contains(&format!("import \"../../../assets/brand/fonts/{font}\";")));
    }
    let font_notice = read("assets/brand/fonts/FONT_NOTICE.md");
    assert!(font_notice.contains("Inter Project Authors"));
    assert!(font_notice.contains("JetBrains Mono Project Authors"));
    let builder = read("scripts/build-portable-release-package.ps1");
    let verifier = read("scripts/verify-portable-release-package.ps1");
    for file in ["Inter-OFL.txt", "JetBrainsMono-OFL.txt", "FONT_NOTICE.md"] {
        let packaged = format!("licenses/fonts/{file}");
        assert!(builder.contains(&packaged), "license not packaged: {file}");
        assert!(
            verifier.contains(&packaged),
            "license inventory not required: {file}"
        );
    }
    // Actual ZIP-mutation refusal proofs run in required CI's portable qualification step.
    let qualifier = read("scripts/qualify-portable-release-package.ps1");
    for failure in [
        "package file inventory mismatch",
        "payload hash mismatch: licenses/fonts/Inter-OFL.txt",
        "packaged font attribution missing from NOTICE.md",
    ] {
        assert!(qualifier.contains(failure));
    }
}

fn state_mapping(components: &str, property: &str) -> (BTreeMap<String, String>, String) {
    let prefix = format!("out property <string> {property}:");
    let expression = components
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .expect("state mapping expression")
        .trim()
        .trim_end_matches(';');
    let mut mapping = BTreeMap::new();
    for branch in expression.split("state == \"").skip(1) {
        let (state, value) = branch.split_once('"').expect("state key");
        let label = value
            .split_once("? \"")
            .expect("state value")
            .1
            .split_once('"')
            .expect("state value end")
            .0;
        assert!(mapping.insert(state.to_owned(), label.to_owned()).is_none());
    }
    let fallback = expression
        .rsplit_once(" : \"")
        .expect("fallback value")
        .1
        .trim_end_matches('"')
        .to_owned();
    (mapping, fallback)
}

#[test]
fn every_product_state_has_distinct_literal_text_and_a_non_color_marker() {
    let components = read("crates/medscale-desktop/ui/components.slint");
    let (labels, label_fallback) = state_mapping(&components, "state-label");
    let (markers, marker_fallback) = state_mapping(&components, "state-marker");
    let states: BTreeSet<_> = [
        "current",
        "reviewed",
        "unknown",
        "stale",
        "conflicting",
        "partial",
        "denied",
        "unavailable",
        "unsupported",
        "unmeasured",
        "corrupt",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(labels.keys().cloned().collect::<BTreeSet<_>>(), states);
    assert_eq!(markers.keys().cloned().collect::<BTreeSet<_>>(), states);
    assert_eq!(labels.values().collect::<BTreeSet<_>>().len(), states.len());
    assert_eq!(
        markers.values().collect::<BTreeSet<_>>().len(),
        states.len()
    );
    for (state, label) in &labels {
        assert_eq!(
            label.to_ascii_lowercase(),
            *state,
            "state collapsed: {state}"
        );
    }
    for marker in markers.values() {
        assert_eq!(marker.len(), 1, "single readable marker");
        assert!(marker.is_ascii() && !marker.chars().any(char::is_whitespace));
    }
    assert_eq!(label_fallback, "Unknown");
    assert_eq!(marker_fallback, markers["unknown"]);
    assert!(components.contains("state: \"unknown\""));
    assert!(components.contains("accessible-label: root.display-label"));
    assert!(components.contains("state-label + \" · \" + label"));
    assert!(components.contains("text: root.display-label"));
    assert!(components.contains("root.state-marker"));
}

//! Brand asset, redistribution, and non-color presentation contracts.
//! The founder-supplied mark of 2026-11-05 supersedes Logo 12 (2026-10-19) and
//! the Spec 095 paired mark. The wordmark follows the founder amendment of
//! 2026-10-22: filled outlines.
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
                "per-path weight or fill breaks the monochrome group"
            );
            let d = path.split_once("d=\"").expect("path data").1;
            d.split_once('"').expect("path data end").0
        })
        .collect();
    (&svg[start..open_end], paths)
}

const SYMBOL: &str = "medscale-symbol";
const WORDMARK: &str = "medscale-wordmark";
/// The near-black tile sampled from the founder's art. Only application icons may use it.
const TILE: &str = "#080808";
/// The mark's offset on the founder's 1024-unit tile.
const TILE_SYMBOL_OFFSET: (f64, f64) = (111.07, 247.1);
/// The one symbol path: a rounded trapezoid and three triangular counters.
const SYMBOL_PATH: &str = "M210.83 0L579.63 0C596.18 0 606.46 10.69 611.45 23.3L792.93 482.11C803.53 508.91 787.93 520.61 768.81 520.61L27.28 520.61C2.08 520.61 -6.18 506.94 4.68 479L176.62 36.63C185.41 14.02 195.49 0 210.83 0ZM296.48 69.3L504.78 69.3L400.68 371.8ZM236.83 138.95L351.08 446.05L113.08 446.05ZM559.38 138.8L678.33 446.05L445.33 446.05Z";

fn assert_filled_symbol(open_tag: &str, color: &str) {
    assert_eq!(attribute(open_tag, "fill"), color);
    assert_eq!(attribute(open_tag, "fill-rule"), "evenodd");
    assert!(
        !open_tag.contains("stroke"),
        "the symbol is filled artwork, never stroked"
    );
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
        // Logo 12 (2026-10-19), superseded on 2026-11-05: its group and frame paths.
        "medscale-frames",
        "M13.5 390.5",
        "M209.5 725",
        "stroke",
    ] {
        assert!(
            !svg.contains(forbidden),
            "unapproved treatment in {asset}: {forbidden}"
        );
    }
    let may_use_tile = asset.contains("icon") || asset.contains("favicon");
    for (index, _) in svg.match_indices('#') {
        let color = &svg[index..index + 7];
        assert!(
            matches!(color, "#000000" | "#FFFFFF") || (may_use_tile && color == TILE),
            "non-monochrome colour in {asset}: {color}"
        );
    }
}

#[test]
fn founder_symbol_is_one_filled_path_with_three_triangular_counters() {
    let black = read("assets/brand/mark-black.svg");
    let white = read("assets/brand/mark-white.svg");
    let (black_tag, black_paths) = group(&black, SYMBOL);
    let (white_tag, white_paths) = group(&white, SYMBOL);
    assert_filled_symbol(black_tag, "#000000");
    assert_filled_symbol(white_tag, "#FFFFFF");
    assert_eq!(black_paths, white_paths, "inverse geometry drift");
    assert_no_decoration(&black, "mark-black.svg");
    assert_no_decoration(&white, "mark-white.svg");
    assert!(!black.contains("<rect"), "the symbol master is transparent");
    assert_eq!(black_paths.len(), 1, "one filled path");
    let d = black_paths[0];
    assert_eq!(d, SYMBOL_PATH, "symbol geometry drift");
    assert!(
        d.chars()
            .filter(char::is_ascii_alphabetic)
            .all(|c| matches!(c, 'M' | 'L' | 'C' | 'Z'))
    );
    assert_eq!(d.matches('Z').count(), 4, "every subpath is closed");
    let subpaths: Vec<&str> = d.split('M').skip(1).collect();
    assert_eq!(subpaths.len(), 4, "the trapezoid and three counters");
    assert_eq!(
        subpaths[0].matches('C').count(),
        4,
        "four rounded corners on the trapezoid"
    );
    // Curve control points may sit outside the artwork bounds; the trapezoid's
    // start and every counter vertex may not.
    let view_box = numbers(attribute(&black, "viewBox"));
    assert!(
        view_box[2] > 1.4 * view_box[3],
        "the mark is wider than tall"
    );
    let inside =
        |x: f64, y: f64| (0.0..=view_box[2]).contains(&x) && (0.0..=view_box[3]).contains(&y);
    let outer = numbers(subpaths[0]);
    assert!(inside(outer[0], outer[1]), "normalized to its own bounds");
    let mut counters = Vec::new();
    for counter in &subpaths[1..] {
        assert_eq!(counter.matches('L').count(), 2, "a straight triangle");
        assert!(!counter.contains('C'), "counters keep sharp corners");
        let points = numbers(counter);
        assert_eq!(points.len(), 6, "three vertices");
        assert!(points.chunks_exact(2).all(|p| inside(p[0], p[1])));
        counters.push(points);
    }
    // The centre counter points down, with its flat edge on top; the outer two point up.
    assert!(counters[0][1] == counters[0][3] && counters[0][5] > counters[0][1]);
    for side in &counters[1..] {
        assert!(side[3] == side[5] && side[1] < side[3]);
    }
    assert!(
        counters[1][0] < counters[0][4] && counters[0][4] < counters[2][0],
        "left, centre and right counters"
    );
    assert!(
        read("assets/brand/source/geometry.json").contains(SYMBOL_PATH),
        "the generator's source holds the same path"
    );
}

#[test]
fn every_asset_reuses_the_identical_symbol_and_wordmark_geometry() {
    let mark = read("assets/brand/mark-black.svg");
    let (_, symbol) = group(&mark, SYMBOL);
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
    // The wordmark paths did not change with the 2026-11-05 symbol. These digests changed once, on
    // 2026-11-06, only to remove a stale mention of the retired symbol from the SVG metadata.
    for (file, expected) in [
        (
            "medscale-wordmark-black.svg",
            "6a103a28dca4dbee836f012aa08812020d12e642e858be4a3466410159c088bb",
        ),
        (
            "medscale-wordmark-white.svg",
            "688501906d890e6edfa2ea1cdc2b51d4bcc955050195e76215b7255ad488b327",
        ),
    ] {
        let bytes =
            std::fs::read(repo_root().join("assets/brand").join(file)).expect("wordmark master");
        assert_eq!(DigestSha256::of(&bytes).to_hex(), expected, "{file}");
    }
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
        (
            "assets/brand/mark-black.svg",
            "web/public/brand/mark-black.svg",
        ),
        (
            "assets/brand/mark-white.svg",
            "web/public/brand/mark-white.svg",
        ),
        (
            "assets/brand/medscale-app-icon-macos.svg",
            "web/public/brand/medscale-app-icon-macos.svg",
        ),
        (
            "assets/brand/medscale-favicon.svg",
            "web/public/brand/medscale-favicon.svg",
        ),
        (
            "assets/brand/medscale-horizontal-black.svg",
            "web/public/brand/medscale-horizontal-black.svg",
        ),
        (
            "assets/brand/medscale-horizontal-white.svg",
            "web/public/brand/medscale-horizontal-white.svg",
        ),
        (
            "assets/brand/medscale-wordmark-white.svg",
            "web/public/brand/medscale-wordmark-white.svg",
        ),
    ] {
        assert_eq!(read(master), read(native), "runtime asset drift: {native}");
    }
    for (master, copy) in [
        (
            "assets/brand/icons/icon.ico",
            "apps/desktop-tauri/src-tauri/icons/icon.ico",
        ),
        (
            "assets/brand/icons/icon.icns",
            "apps/desktop-tauri/src-tauri/icons/icon.icns",
        ),
        ("assets/brand/icons/icon.ico", "web/public/favicon.ico"),
        (
            "assets/brand/icons/favicon-32.png",
            "web/public/brand/favicon-32.png",
        ),
        (
            "assets/brand/icons/apple-touch-icon.png",
            "web/public/brand/apple-touch-icon.png",
        ),
    ] {
        let bytes = |path: &str| std::fs::read(repo_root().join(path)).expect("icon bytes");
        assert!(bytes(master) == bytes(copy), "icon drift: {copy}");
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
        if asset.contains("wordmark") {
            assert!(!svg.contains(SYMBOL), "the wordmark stands alone: {asset}");
        } else {
            let (tag, paths) = group(&svg, SYMBOL);
            assert_eq!(paths, symbol, "symbol drift: {asset}");
            assert_eq!(attribute(tag, "fill-rule"), "evenodd", "{asset}");
            assert!(!tag.contains("stroke"), "filled symbol: {asset}");
            assert!(matches!(attribute(tag, "fill"), "#000000" | "#FFFFFF"));
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
            assert!(svg.contains(SYMBOL) && svg.contains(WORDMARK));
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
fn application_icons_keep_the_founders_tile_and_clear_space() {
    let mark = numbers(attribute(&read("assets/brand/mark-black.svg"), "viewBox"));
    for (variant, background, ink) in [("dark", TILE, "#FFFFFF"), ("light", "#FFFFFF", "#000000")] {
        let svg = read(&format!("assets/brand/medscale-app-icon-{variant}.svg"));
        let view_box = numbers(attribute(&svg, "viewBox"));
        assert_eq!(view_box, vec![0.0, 0.0, 1024.0, 1024.0]);
        assert!(svg.contains(&format!(
            "<rect width=\"1024\" height=\"1024\" rx=\"215\" fill=\"{background}\"/>"
        )));
        let (tag, _) = group(&svg, SYMBOL);
        assert_eq!(attribute(tag, "fill"), ink);
        let transform = numbers(attribute(tag, "transform"));
        assert_eq!(transform.len(), 3, "translation plus uniform scale");
        let (x, y, scale) = (transform[0], transform[1], transform[2]);
        // The founder's art sits a few units off centre; keep exactly that placement.
        assert!((scale - 1.0).abs() < 0.0001, "the founder's proportion");
        assert!(
            (x - TILE_SYMBOL_OFFSET.0).abs() < 0.01,
            "founder's x offset"
        );
        assert!(
            (y - TILE_SYMBOL_OFFSET.1).abs() < 0.01,
            "founder's y offset"
        );
        let (width, height) = (mark[2] * scale, mark[3] * scale);
        assert!(
            (x - (1024.0 - width - x)).abs() < 1024.0 * 0.01,
            "within 1% of centre horizontally"
        );
        assert!(
            (y - (1024.0 - height - y)).abs() < 1024.0 * 0.01,
            "within 1% of centre vertically"
        );
        assert!(y >= 1024.0 * 0.15, "clear space above and below the symbol");
    }
    // Optical masters crop tighter around the same path, on the same tile.
    let founder_scale = 1.0;
    let mut previous = founder_scale;
    for asset in ["medium-dark", "small-dark"] {
        let svg = read(&format!("assets/brand/medscale-icon-{asset}.svg"));
        assert!(svg.contains(&format!(
            "<rect width=\"1024\" height=\"1024\" rx=\"215\" fill=\"{TILE}\"/>"
        )));
        let scale = numbers(attribute(group(&svg, SYMBOL).0, "transform"))[2];
        assert!(scale > previous, "tighter crop at smaller sizes: {asset}");
        assert!(
            mark[2] * scale < 1024.0,
            "the mark stays on the tile: {asset}"
        );
        previous = scale;
    }
    // macOS keeps Apple's 824/1024 rounded tile grid, with transparent margins.
    let macos = read("assets/brand/medscale-app-icon-macos.svg");
    assert!(macos.contains(&format!(
        "<rect x=\"100\" y=\"100\" width=\"824\" height=\"824\" rx=\"185\" fill=\"{TILE}\"/>"
    )));
    let wordmark = read("third_party/provenance/medscale-wordmark-2026-10-22.md");
    for required in [
        "2026-10-22",
        "PlusJakartaSans-Variable.ttf",
        "SIL Open Font License",
        "2.05",
    ] {
        assert!(
            wordmark.contains(required),
            "wordmark provenance: {required}"
        );
    }
    // The source font and its license are pinned; the logo never embeds the font.
    for (file, expected) in [
        (
            "PlusJakartaSans-Variable.ttf",
            "89b3fb38aa0d275d7a731d0d817a4f1622b316b4d7fbdedcf02ee9099ff68bc8",
        ),
        (
            "PlusJakartaSans-OFL.txt",
            "995c7199cab65954f545996326755daee7b63cc6b42b06c13da1f9502ab08a99",
        ),
    ] {
        let bytes = std::fs::read(repo_root().join("assets/brand/fonts").join(file))
            .expect("wordmark source font bytes");
        assert_eq!(DigestSha256::of(&bytes).to_hex(), expected, "{file}");
        assert!(wordmark.contains(expected), "provenance pins {file}");
    }
    // Lockup proportions for the wider, filled symbol: no stroke, no drop.
    let geometry = read("assets/brand/source/geometry.json");
    assert!(geometry.contains("\"symbol_to_cap\": 1.45"));
    assert!(geometry.contains("\"gap_to_cap\": 0.5"));
    assert!(geometry.contains("\"color\": \"#080808\""));
    assert!(!geometry.contains("symbol_stroke") && !geometry.contains("symbol_drop"));
    // Logo 12's record stays as history.
    assert!(read("third_party/provenance/logo-12-approved-identity.md").contains("2026-10-19"));
    let provenance = read("third_party/provenance/medscale-logo-2026-11-05.md");
    assert!(provenance.contains("supplied"));
    assert!(provenance.contains("2026-11-05"));
    for reference in [
        "7d8139bc35eb34d7ad03db37d3b49636c9898a68df55b854da98b7c2698d02d0",
        "f6854f4ee03e1ac21b85909679fe1047a27305ff7738e1293a80a650812eac4a",
    ] {
        assert!(
            provenance.contains(reference),
            "reference digest {reference}"
        );
    }
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

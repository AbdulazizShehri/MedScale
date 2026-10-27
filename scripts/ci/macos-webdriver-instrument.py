"""Instrument a disposable macOS Actions checkout with test-only WebDriver.

The repository branch and packaged public releases are never altered by this
script. The HTTP automation plugin is enabled ONLY with macos-webdriver-ci.
"""
import os
from pathlib import Path

if os.getenv("GITHUB_ACTIONS") != "true" or os.getenv("RUNNER_OS") != "macOS":
    raise SystemExit("Refusing non-ephemeral macOS runner instrumentation")

root = Path("apps/desktop-tauri/src-tauri")
cargo = root / "Cargo.toml"
source = cargo.read_text()
feature_anchor = 'e2e-webdriver = []\n'
dep_anchor = 'tauri-plugin-dialog = "=2.6.0"\n'
assert source.count(feature_anchor) == 1
assert source.count(dep_anchor) == 1
assert "macos-webdriver-ci" not in source
source = source.replace(feature_anchor,
                        feature_anchor + 'macos-webdriver-ci = ["dep:tauri-plugin-webdriver"]\n')
source = source.replace(dep_anchor,
                        dep_anchor + 'tauri-plugin-webdriver = { version = "=0.2.3", optional = true }\n')
cargo.write_text(source)

main = root / "src/main.rs"
src = main.read_text()
anchor = "    presentation_builder(tauri::Builder::default().plugin(tauri_plugin_dialog::init()))\n"
assert src.count(anchor) == 1
src = src.replace(anchor, """    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    // GitHub-hosted, disposable CI only. Never active in release builds.
    #[cfg(feature = "macos-webdriver-ci")]
    let builder = builder.plugin(tauri_plugin_webdriver::init());
    presentation_builder(builder)
""")
main.write_text(src)
print("MAC_WEBDRIVER_INSTRUMENTED: feature gated; no production permissions changed")

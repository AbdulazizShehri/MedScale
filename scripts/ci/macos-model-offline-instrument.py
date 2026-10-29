"""Fail-closed test-only network fault for disposable macOS GitHub Actions.

This script changes the ephemeral checkout ONLY. It adds an early error to
the actual Tauri Whisper HTTP streaming entry point, so GUI consent,
download-worker scheduling, error rendering and staging cleanup run through
production code without any real HTTP or model-weight transfer.
"""
import os
from pathlib import Path

if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("RUNNER_OS") != "macOS":
    raise SystemExit("Offline model fault instrumentation is allowed only on disposable macOS GitHub CI")

source = Path("apps/desktop-tauri/src-tauri/src/audio_model_manager.rs")
body = source.read_text()
anchor = "    let mut next = url.clone();\n"
if body.count(anchor) != 1 or "MEDSCALE_MAC_E2E_FORCE_OFFLINE" in body:
    raise SystemExit("Offline injection source marker missing, ambiguous or already modified")

fault = """    // CI-only substitution added to the throwaway checkout, never committed
    // to the production Rust crate or included in normal public packages.
    if std::env::var("MEDSCALE_MAC_E2E_FORCE_OFFLINE").as_deref() == Ok("1") {
        return Err("Network request failed; check connection and retry".into());
    }
"""
# Capture the *actual* app-owned data directory on this disposable runner.
# The native failure cleanup can then be inspected without guessing a
# platform-specific Tauri path or reading any customer files.
cache_anchor = '    let root = data_dir.join("audio-models");\n'
if body.count(cache_anchor) != 1:
    raise SystemExit("App-owned cache root marker missing or ambiguous")
probe = """    if std::env::var("MEDSCALE_MAC_E2E_FORCE_OFFLINE").as_deref() == Ok("1") {
        let temp = std::env::var("RUNNER_TEMP")
            .map_err(|_| "CI runner temporary directory unavailable".to_owned())?;
        std::fs::write(
            std::path::Path::new(&temp).join("medscale-offline-cache-root.txt"),
            root.to_string_lossy().as_bytes(),
        )
        .map_err(|_| "Cannot record CI-only app cache path".to_owned())?;
    }
"""
body = body.replace(anchor, fault + anchor, 1)
source.write_text(body.replace(cache_anchor, cache_anchor + probe, 1))
print("MAC_NATIVE_OFFLINE_FAULT_INSTRUMENTED: no HTTP, exact cache-root proof path in RUNNER_TEMP")

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
source.write_text(body.replace(anchor, fault + anchor, 1))
print("MAC_NATIVE_OFFLINE_FAULT_INSTRUMENTED: Rust HTTP entry point refuses all requests before network")

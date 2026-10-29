"""Inject a zero-byte model disk budget ONLY into disposable macOS GitHub CI.

The committed Rust implementation is never modified. A fail-closed trap
prevents any network/model acquisition if the native storage guard regresses.
"""
import os
from pathlib import Path

if os.getenv("GITHUB_ACTIONS") != "true" or os.getenv("RUNNER_OS") != "macOS":
    raise SystemExit("Only disposable macOS GitHub Actions may inject low storage")

source = Path("apps/desktop-tauri/src-tauri/src/audio_model_manager.rs")
body = source.read_text()
if "MEDSCALE_MAC_E2E_FORCE_LOW_SPACE" in body:
    raise SystemExit("Low-storage injection already present")
anchor = '''    let free_bytes = fs2::available_space(data_dir)
        .map_err(|_| CmdError::unavailable("Unavailable: cannot measure local free storage"))?;
'''
if body.count(anchor) != 1:
    raise SystemExit("Real preflight marker missing or ambiguous")
replacement = '''    // CI-only substitution in a disposable checkout.
    let free_bytes = if std::env::var("MEDSCALE_MAC_E2E_FORCE_LOW_SPACE").as_deref() == Ok("1") {
        let temp = std::env::var("RUNNER_TEMP")
            .map_err(|_| CmdError::unavailable("Unavailable: CI runner temporary directory"))?;
        std::fs::write(
            std::path::Path::new(&temp).join("medscale-low-space-cache-root.txt"),
            data_dir.join("audio-models").to_string_lossy().as_bytes(),
        )
        .map_err(|_| CmdError::unavailable("Unavailable: CI-only cache root probe"))?;
        0
    } else {
        fs2::available_space(data_dir)
            .map_err(|_| CmdError::unavailable("Unavailable: cannot measure local free storage"))?
    };
'''
body = body.replace(anchor, replacement, 1)
acquire = '''    let root = data_dir.join("audio-models");
    fs::create_dir_all(&root).map_err(|_| "Cannot create application model cache")?;
'''
if body.count(acquire) != 1:
    raise SystemExit("Native acquire marker missing or ambiguous")
trap = '''    if std::env::var("MEDSCALE_MAC_E2E_FORCE_LOW_SPACE").as_deref() == Ok("1") {
        let temp = std::env::var("RUNNER_TEMP")
            .map_err(|_| "CI runner temporary directory unavailable".to_owned())?;
        std::fs::write(
            std::path::Path::new(&temp).join("medscale-low-space-acquire-entered.txt"),
            b"unexpected acquisition",
        )
        .map_err(|_| "Cannot record unexpected CI-only acquisition".to_owned())?;
        return Err("CI-only: low-storage acquisition must not be entered".into());
    }
'''
source.write_text(body.replace(acquire, trap + acquire, 1))
print("MAC_NATIVE_LOW_SPACE_INSTRUMENTED: zero-byte budget and no-network acquisition trap")

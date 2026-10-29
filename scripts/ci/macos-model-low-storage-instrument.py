"""CI-only zero-byte storage simulation for true macOS WKWebView first-run GUI.

Modify ONLY the ephemeral, feature-instrumented checkout before native build.
Never fill real disk space or enable model downloads; normal production Rust
source and model-storage budgets are unchanged.
"""
import os
from pathlib import Path

if os.getenv("GITHUB_ACTIONS") != "true" or os.getenv("RUNNER_OS") != "macOS":
    raise SystemExit("Refusing low-storage injection outside disposable hosted macOS CI")

path = Path("apps/desktop-tauri/src-tauri/src/audio_model_manager.rs")
text = path.read_text()
original = """    Ok(preflight_with_available_bytes(entry, data_dir, free_bytes))
"""
if text.count(original) != 1 or "MEDSCALE_MAC_E2E_FORCE_LOW_STORAGE" in text:
    raise SystemExit("Native preflight injection point missing/ambiguous/already modified")
replacement = """    // Disposable CI-only diagnostic. The source tree in the actual repository
    // is untouched: only this temporary runner checkout is instrumented.
    let free_bytes = if std::env::var("MEDSCALE_MAC_E2E_FORCE_LOW_STORAGE").as_deref() == Ok("1") {
        let marker_dir = std::env::var("RUNNER_TEMP")
            .map_err(|_| CmdError::unavailable("Unavailable: no ephemeral runner temp"))?;
        std::fs::write(
            std::path::Path::new(&marker_dir).join("medscale-low-storage-data-dir.txt"),
            data_dir.to_string_lossy().as_bytes(),
        ).map_err(|_| CmdError::unavailable("Unavailable: cannot write temporary CI marker"))?;
        0
    } else {
        free_bytes
    };
    Ok(preflight_with_available_bytes(entry, data_dir, free_bytes))
"""
path.write_text(text.replace(original, replacement, 1))
print("MAC_NATIVE_LOW_STORAGE_INSTRUMENTED: simulated zero free bytes, no disk allocation")

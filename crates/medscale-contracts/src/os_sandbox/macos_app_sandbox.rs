//! macOS App Sandbox entitlements ReadyBaseMeasured (Spec 041).
//!
//! Separates:
//! - Seatbelt `sandbox_init` (Spec 031) — not App Sandbox
//! - Entitlements artifact + detection probe (this unit)
//! - Runtime entitlement enforcement — requires codesign; not claimed here
//!
//! Does **not** claim PLATFORM_QUALIFIED or clear EXTERNAL_GATES.

use super::OsSandboxApplyError;
use std::path::{Path, PathBuf};

/// Relative path of the worker entitlements fixture (repo evidence).
pub const ENTITLEMENTS_REL_PATH: &str =
    "evidence/041-macos-app-sandbox-entitlements/worker.entitlements.plist";

/// Locate entitlements fixture from CWD / repo-relative parents.
#[must_use]
pub fn resolve_entitlements_path() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(ENTITLEMENTS_REL_PATH),
        PathBuf::from("..").join(ENTITLEMENTS_REL_PATH),
        PathBuf::from("../..").join(ENTITLEMENTS_REL_PATH),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// True if process appears to be running inside an App Sandbox container.
///
/// Detection uses `APP_SANDBOX_CONTAINER_ID` (set by the App Sandbox runtime).
/// Unsigned CI/CLI processes normally report **false**. This is intentional honesty —
/// Seatbelt Spec 031 is not App Sandbox.
#[must_use]
pub fn app_sandbox_container_active() -> bool {
    std::env::var_os("APP_SANDBOX_CONTAINER_ID").is_some_and(|v| !v.is_empty())
}

/// Validate entitlements plist contains App Sandbox enablement key.
pub fn validate_entitlements_artifact(path: &Path) -> Result<(), OsSandboxApplyError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        OsSandboxApplyError::ApplyFailed(format!(
            "failed to read entitlements {}: {e}",
            path.display()
        ))
    })?;
    if !text.contains("com.apple.security.app-sandbox") {
        return Err(OsSandboxApplyError::ApplyFailed(
            "entitlements missing com.apple.security.app-sandbox key".to_owned(),
        ));
    }
    // Require an explicit true enablement (plist boolean or string forms).
    let enabled = text.contains("<key>com.apple.security.app-sandbox</key>")
        && (text.contains("<true/>") || text.contains("<true />"));
    if !enabled {
        return Err(OsSandboxApplyError::ApplyFailed(
            "entitlements must enable com.apple.security.app-sandbox = true".to_owned(),
        ));
    }
    Ok(())
}

/// ReadyBaseMeasured apply: validate entitlements artifact + run detection probe.
///
/// On macOS CI (unsigned), `app_sandbox_container_active()` is expected **false**.
/// Enforcement under signed entitlements remains EXTERNAL (SIGNING_ACTION).
pub(super) fn apply_macos_app_sandbox_entitlements() -> Result<(), OsSandboxApplyError> {
    let path = resolve_entitlements_path().ok_or_else(|| {
        OsSandboxApplyError::ApplyFailed(format!(
            "App Sandbox entitlements fixture missing ({ENTITLEMENTS_REL_PATH})"
        ))
    })?;
    validate_entitlements_artifact(&path)?;

    // Probe always runs; unsigned hosts report inactive — fail-closed only if
    // we cannot evaluate the environment variable API (always available).
    let _active = app_sandbox_container_active();
    let _ = _active;
    Ok(())
}

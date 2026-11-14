//! Claim-scoped path validation.

use std::path::{Component, Path, PathBuf};

use thiserror::Error;

/// Path claim errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClaimError {
    #[error("path escapes claim root")]
    Escape,
    #[error("sync/remote root refused for durability claim: {0}")]
    SyncRootRefused(String),
    #[error("invalid path: {0}")]
    Invalid(String),
}

/// Refuse known sync-root path markers for claim scope.
pub fn assert_claim_path(path: &Path) -> Result<PathBuf, ClaimError> {
    let raw = path
        .to_str()
        .ok_or_else(|| ClaimError::Invalid("non-utf8 path".to_owned()))?;
    let lower = raw.to_ascii_lowercase();
    for marker in [
        "onedrive",
        "dropbox",
        "google drive",
        "googledrive",
        "icloud",
        "box sync",
        "boxsync",
        "\\mega\\",
        "/mega/",
        "\\remote\\",
        "/mnt/sync",
        "\\\\remote",
    ] {
        if lower.contains(marker) {
            return Err(ClaimError::SyncRootRefused(marker.to_owned()));
        }
    }
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(ClaimError::Escape);
        }
    }
    Ok(path.to_path_buf())
}

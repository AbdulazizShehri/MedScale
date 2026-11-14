//! Host configuration for the Spec 086 R Workspace.
//!
//! Where workspaces are staged and which external programs may open them
//! is the host's decision (CLI flags or environment, Desktop settings),
//! never a request's. Programs are absolute paths, started directly with
//! the workspace directory as their only argument: no shell, no `PATH`
//! lookup, stdio closed, and an environment reduced to
//! `LAUNCH_ENV_ALLOWLIST`. MedScale never starts R itself.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use medscale_contracts::objects::DigestSha256;
use medscale_contracts::r_workspace::{
    IdeKind, PROGRAM_DIGEST_BYTES_MAX, RRuntimeIdentity, RVersionEvidence, launch_env_allowed,
};

/// Environment variable naming the staging directory.
pub const ENV_STAGE_DIR: &str = "MEDSCALE_R_STAGE_DIR";
pub const ENV_RSTUDIO: &str = "MEDSCALE_RSTUDIO";
pub const ENV_POSITRON: &str = "MEDSCALE_POSITRON";
pub const ENV_OPEN_FOLDER: &str = "MEDSCALE_OPEN_FOLDER";
pub const ENV_RSCRIPT: &str = "MEDSCALE_RSCRIPT";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RWorkspaceHost {
    /// Existing directory, outside the vault, under which each workspace
    /// gets a new subdirectory.
    pub stage_dir: Option<PathBuf>,
    pub programs: BTreeMap<IdeKind, PathBuf>,
    /// `Rscript`, identified for receipts; never executed in this slice.
    pub rscript: Option<PathBuf>,
}

/// The platform's folder opener, when present at its standard path.
#[must_use]
pub fn default_folder_opener() -> Option<PathBuf> {
    let candidate = if cfg!(windows) {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("explorer.exe")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/usr/bin/open")
    } else {
        PathBuf::from("/usr/bin/xdg-open")
    };
    candidate.is_file().then_some(candidate)
}

impl RWorkspaceHost {
    /// Nothing configured except the platform folder opener.
    #[must_use]
    pub fn platform_default() -> Self {
        let mut programs = BTreeMap::new();
        if let Some(opener) = default_folder_opener() {
            programs.insert(IdeKind::Folder, opener);
        }
        Self {
            stage_dir: None,
            programs,
            rscript: None,
        }
    }

    /// The platform default overlaid with `MEDSCALE_R_STAGE_DIR`,
    /// `MEDSCALE_RSTUDIO`, `MEDSCALE_POSITRON`, `MEDSCALE_OPEN_FOLDER` and
    /// `MEDSCALE_RSCRIPT`.
    #[must_use]
    pub fn from_env() -> Self {
        let mut host = Self::platform_default();
        let path = |name: &str| {
            std::env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        host.stage_dir = path(ENV_STAGE_DIR);
        for (name, kind) in [
            (ENV_RSTUDIO, IdeKind::Rstudio),
            (ENV_POSITRON, IdeKind::Positron),
            (ENV_OPEN_FOLDER, IdeKind::Folder),
        ] {
            if let Some(p) = path(name) {
                host.programs.insert(kind, p);
            }
        }
        host.rscript = path(ENV_RSCRIPT);
        host
    }

    /// Identifies `Rscript` by path and file digest, without running it.
    #[must_use]
    pub fn runtime_identity(&self) -> RRuntimeIdentity {
        let program = self.rscript.as_ref();
        let found = program.is_some_and(|p| p.is_absolute() && p.is_file());
        let program_digest = if found {
            program.and_then(|p| digest_file(p, PROGRAM_DIGEST_BYTES_MAX))
        } else {
            None
        };
        RRuntimeIdentity {
            program: program.map(|p| p.to_string_lossy().into_owned()),
            found,
            program_digest,
            version: RVersionEvidence::NotProbed,
        }
    }
}

fn digest_file(path: &Path, max: u64) -> Option<DigestSha256> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(max + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= max).then(|| DigestSha256::of(&bytes))
}

/// The allowlisted environment of this process, sorted by name.
#[must_use]
pub fn launch_environment() -> Vec<(String, std::ffi::OsString)> {
    let mut env: Vec<(String, std::ffi::OsString)> = std::env::vars_os()
        .filter_map(|(k, v)| k.into_string().ok().map(|k| (k, v)))
        .filter(|(k, _)| launch_env_allowed(k))
        .collect();
    env.sort_by(|a, b| a.0.cmp(&b.0));
    env
}

/// Starts `program` with `workspace` as its only argument and working
/// directory. Returns the names of the variables passed on. The child is
/// reaped on a background thread; MedScale does not wait for it.
pub fn launch(program: &Path, workspace: &Path) -> std::io::Result<Vec<String>> {
    let env = launch_environment();
    let mut child = Command::new(program)
        .arg(workspace)
        .current_dir(workspace)
        .env_clear()
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_os_str())))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(env.into_iter().map(|(k, _)| k).collect())
}

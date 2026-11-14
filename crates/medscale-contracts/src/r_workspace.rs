//! R Workspace contracts (Spec 086, foundation slice).
//!
//! Core stages exact Spec 075 snapshots into a bounded workspace directory
//! outside the vault (CSV copies plus schema files, a MedScale descriptor, a
//! README and an `.Rproj`), can open that directory in an external program
//! the host configured, and admits outputs only through an explicit,
//! validated publication. R itself never runs inside MedScale: managed
//! `Rscript` execution is arbitrary code and is not admitted while the OS
//! sandbox is not platform qualified, so a run request is recorded and
//! refused.
//!
//! The R process, the IDE and anything the user does in them are outside
//! MedScale's authority. The only way back in is `publish`, which reads one
//! named regular file from `outputs/`, pins its exact bytes and commits a
//! derived, unreviewed table that inherits the workspace's data class.
//!
//! ```text
//! source snapshot != staged copy != script != (external) execution
//!   != output file != published table (derived, unreviewed)
//!   != reviewed result (not here) != external effect (none)
//! ```

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::analytics::{ResultColumn, ResultTableDoc};
use crate::data_sources::{CellValue, SchemaField};
use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};
use crate::privacy_gate::DataClass;

/// Durable schema version for every R Workspace object (Spec 086 v1).
pub const R_WORKSPACE_SCHEMA_VERSION: u32 = 1;
/// Version of the staged directory layout.
pub const R_WORKSPACE_LAYOUT_VERSION: u32 = 1;

pub const WORKSPACE_INPUTS_MAX: usize = 16;
pub const LABEL_MAX_CHARS: usize = 64;
pub const STAGE_ROOT_MAX_CHARS: usize = 1024;
pub const OUTPUT_NAME_MAX_CHARS: usize = 128;
/// Largest output file `publish` reads.
pub const PUBLISH_BYTES_MAX: u64 = 16 * 1024 * 1024;
/// Largest `renv.lock` or script digested as evidence.
pub const EVIDENCE_FILE_BYTES_MAX: u64 = 4 * 1024 * 1024;
/// Largest program file digested for runtime identity.
pub const PROGRAM_DIGEST_BYTES_MAX: u64 = 256 * 1024 * 1024;
/// Total CSV bytes one workspace may stage.
pub const STAGE_BYTES_MAX: u64 = 512 * 1024 * 1024;
/// `outputs/` entries examined by one inspection.
pub const OUTPUT_ENTRIES_MAX: usize = 64;
pub const IDE_PATH_MAX_CHARS: usize = 1024;

/// Name of the MedScale descriptor inside a staged workspace.
pub const DESCRIPTOR_FILE: &str = "medscale-workspace.json";
pub const README_FILE: &str = "README.md";
pub const DATA_DIR: &str = "data";
pub const SCRIPTS_DIR: &str = "scripts";
pub const OUTPUTS_DIR: &str = "outputs";
pub const RENV_LOCK_FILE: &str = "renv.lock";

/// Environment variables an external program may inherit (names compared
/// case-insensitively). Everything else, including every `MEDSCALE_*`
/// variable, is removed before launch.
pub const LAUNCH_ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "TZ",
    "TMPDIR",
    "TEMP",
    "TMP",
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XDG_RUNTIME_DIR",
    "XDG_DATA_DIRS",
    "DBUS_SESSION_BUS_ADDRESS",
    "SYSTEMROOT",
    "WINDIR",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "HOMEDRIVE",
    "HOMEPATH",
];

/// Whether an environment variable may pass to a launched program.
#[must_use]
pub fn launch_env_allowed(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    !upper.starts_with("MEDSCALE") && LAUNCH_ENV_ALLOWLIST.contains(&upper.as_str())
}

macro_rules! closed_vocabulary {
    ($name:ident, $what:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("unknown ", $what, " {}"), other)),
                }
            }
        }
    };
}

/// How inputs are staged. Parquet/Arrow need a dependency admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingMode {
    CsvCopy,
}

/// Outputs return only through an explicit publication; nothing watches
/// the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputPolicy {
    ExplicitPublishOnly,
}

/// External programs. Launching one proves nothing about an analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeKind {
    Rstudio,
    Positron,
    /// The OS file manager or any folder-opening program the host names.
    Folder,
}

closed_vocabulary!(IdeKind, "IDE kind", {
    Rstudio => "rstudio",
    Positron => "positron",
    Folder => "folder",
});

/// Why a workspace's class is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassBasis {
    /// The most restrictive Spec 079 effective class of the inputs; an
    /// input without a classification row counts as `local_phi`.
    InheritedFromInputs,
}

// ---------------------------------------------------------------------
// Requests.
// ---------------------------------------------------------------------

/// Stage exact snapshots into a new workspace. The directory comes from
/// host configuration, never from a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RStageRequest {
    pub project_id: OpaqueId,
    pub label: String,
    pub snapshot_ids: Vec<OpaqueId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RLaunchRequest {
    pub workspace_id: OpaqueId,
    pub ide: IdeKind,
}

/// A managed `Rscript` run of `scripts/<script>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RRunRequest {
    pub workspace_id: OpaqueId,
    pub script: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RPublishRequest {
    pub workspace_id: OpaqueId,
    /// A plain file name in `outputs/`.
    pub output_name: String,
    /// When given, the bytes read must have this digest (the one the user
    /// inspected).
    pub expected_digest: Option<DigestSha256>,
}

// ---------------------------------------------------------------------
// Workspace.
// ---------------------------------------------------------------------

/// One staged input: the exact snapshot and the files written for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RStagedInput {
    pub snapshot_id: OpaqueId,
    pub content_digest: DigestSha256,
    pub schema_fingerprint: DigestSha256,
    pub row_count: u64,
    pub data_class: DataClass,
    /// `data/<snapshot_id>.csv`
    pub data_file: String,
    /// `data/<snapshot_id>.schema.json`
    pub schema_file: String,
}

/// One file MedScale generated, with the digest of its exact bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedFile {
    /// Relative path with `/` separators.
    pub path: String,
    pub digest: DigestSha256,
    pub bytes: u64,
}

/// A staged workspace. Immutable once written; the staged descriptor is
/// the canonical JSON of this value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RWorkspaceManifest {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub label: String,
    pub layout_version: u32,
    /// Absolute path of the staged directory on the staging machine.
    pub stage_root: String,
    pub staging_mode: StagingMode,
    pub output_policy: OutputPolicy,
    pub data_class: DataClass,
    pub class_basis: ClassBasis,
    pub inputs: Vec<RStagedInput>,
    /// Every generated file except the descriptor itself, sorted by path.
    pub files: Vec<StagedFile>,
}

/// Validates a workspace label (also used as the `.Rproj` file stem).
pub fn validate_label(label: &str) -> Result<(), String> {
    let ok = !label.is_empty()
        && label.chars().count() <= LABEL_MAX_CHARS
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !label.starts_with('-');
    if ok {
        Ok(())
    } else {
        Err("label must be 1-64 ASCII letters, digits, '-' or '_'".to_owned())
    }
}

/// A plain file name: no separators, no parent or hidden names, no
/// control characters. Paths never come from a request.
pub fn validate_output_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.chars().count() <= OUTPUT_NAME_MAX_CHARS
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if ok {
        Ok(())
    } else {
        Err("name must be a plain file name".to_owned())
    }
}

/// A script name: a plain file name ending in `.R` or `.r`.
pub fn validate_script_name(name: &str) -> Result<(), String> {
    validate_output_name(name)?;
    if name.len() > 2 && (name.ends_with(".R") || name.ends_with(".r")) {
        Ok(())
    } else {
        Err("script must be a plain `.R` file name".to_owned())
    }
}

/// The most restrictive of the given classes (`local_phi` when empty).
#[must_use]
pub fn most_restrictive(classes: impl IntoIterator<Item = DataClass>) -> DataClass {
    classes
        .into_iter()
        .max_by_key(|c| c.restrictiveness())
        .unwrap_or(DataClass::LocalPhi)
}

impl RWorkspaceManifest {
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap_or_default()
    }

    #[must_use]
    pub fn digest(&self) -> DigestSha256 {
        DigestSha256::of(&self.canonical_bytes())
    }

    #[must_use]
    pub fn rproj_file(&self) -> String {
        format!("{}.Rproj", self.label)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.header.schema_version != R_WORKSPACE_SCHEMA_VERSION
            || self.layout_version != R_WORKSPACE_LAYOUT_VERSION
        {
            return Err("unsupported R workspace version".to_owned());
        }
        validate_label(&self.label)?;
        if self.stage_root.is_empty() || self.stage_root.chars().count() > STAGE_ROOT_MAX_CHARS {
            return Err("stage root is empty or too long".to_owned());
        }
        if self.inputs.is_empty() || self.inputs.len() > WORKSPACE_INPUTS_MAX {
            return Err("input count out of bounds".to_owned());
        }
        let mut snapshots = BTreeSet::new();
        for input in &self.inputs {
            if !snapshots.insert(input.snapshot_id.as_str()) {
                return Err("duplicate staged snapshot".to_owned());
            }
            if input.data_file != data_file_name(&input.snapshot_id)
                || input.schema_file != schema_file_name(&input.snapshot_id)
            {
                return Err("staged input file names are not derived from the snapshot".to_owned());
            }
        }
        if self.data_class != most_restrictive(self.inputs.iter().map(|i| i.data_class)) {
            return Err("workspace class is not the most restrictive input class".to_owned());
        }
        let mut paths = BTreeSet::new();
        for file in &self.files {
            if !is_generated_path(&file.path) || file.path == DESCRIPTOR_FILE {
                return Err("generated file path is not admitted".to_owned());
            }
            if !paths.insert(file.path.as_str()) {
                return Err("duplicate generated file".to_owned());
            }
        }
        if !self.files.windows(2).all(|w| w[0].path < w[1].path) {
            return Err("generated files must be sorted by path".to_owned());
        }
        let mut expected: BTreeSet<String> = BTreeSet::new();
        for input in &self.inputs {
            expected.insert(input.data_file.clone());
            expected.insert(input.schema_file.clone());
        }
        expected.insert(README_FILE.to_owned());
        expected.insert(self.rproj_file());
        if paths
            .iter()
            .map(|p| (*p).to_owned())
            .collect::<BTreeSet<_>>()
            != expected
        {
            return Err("generated files are not exactly the layout's files".to_owned());
        }
        Ok(())
    }
}

/// Snapshot ids are Core-allocated (`snapshot-N`); names derive from them.
#[must_use]
pub fn data_file_name(snapshot_id: &OpaqueId) -> String {
    format!("{DATA_DIR}/{}.csv", sanitize(snapshot_id.as_str()))
}

#[must_use]
pub fn schema_file_name(snapshot_id: &OpaqueId) -> String {
    format!("{DATA_DIR}/{}.schema.json", sanitize(snapshot_id.as_str()))
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn is_generated_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let plain = |p: &str| validate_output_name(p).is_ok();
    match parts.as_slice() {
        [name] => plain(name),
        [dir, name] => *dir == DATA_DIR && plain(name),
        _ => false,
    }
}

/// A stored workspace: its manifest plus the digest of the staged
/// descriptor bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RWorkspace {
    pub manifest: RWorkspaceManifest,
    pub descriptor_digest: DigestSha256,
}

impl RWorkspace {
    pub fn validate(&self) -> Result<(), String> {
        self.manifest.validate()?;
        if self.manifest.digest() != self.descriptor_digest {
            return Err("descriptor digest does not match the manifest".to_owned());
        }
        Ok(())
    }

    #[must_use]
    pub fn id(&self) -> &OpaqueId {
        &self.manifest.header.id
    }
}

/// What the workspace directory holds now, as Core sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceIntegrity {
    /// The descriptor and every generated file have their recorded bytes.
    Intact,
    /// A generated file is missing, changed, a link, or not a regular file.
    Changed,
    /// The workspace directory is gone (or is not a real directory).
    Missing,
}

closed_vocabulary!(WorkspaceIntegrity, "workspace integrity", {
    Intact => "intact",
    Changed => "changed",
    Missing => "missing",
});

/// An `renv.lock` seen in the workspace (evidence only; MedScale never
/// restores packages).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RLockfileRef {
    pub digest: DigestSha256,
    pub bytes: u64,
}

/// How much MedScale knows about the R version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RVersionEvidence {
    /// Asking R for its version means running R, which is not admitted.
    NotProbed,
}

/// The host's configured `Rscript`, identified without running it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RRuntimeIdentity {
    pub program: Option<String>,
    pub found: bool,
    /// Digest of the program file, when found and within bounds.
    pub program_digest: Option<DigestSha256>,
    pub version: RVersionEvidence,
}

/// One file in `outputs/` a user could publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputCandidate {
    pub name: String,
    pub bytes: u64,
    pub digest: DigestSha256,
}

/// Read-only view of a staged workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceInspection {
    pub workspace_id: OpaqueId,
    pub integrity: WorkspaceIntegrity,
    /// Whether every input is still the Project's snapshot with the pinned
    /// digest.
    pub inputs_current: bool,
    pub lockfile: Option<RLockfileRef>,
    /// Regular files directly in `outputs/`, sorted by name; links and
    /// directories are never listed.
    pub candidates: Vec<OutputCandidate>,
    /// Entries in `outputs/` that are not publishable (links, directories,
    /// unsafe names, oversized files, entries past the scan bound).
    pub skipped: u64,
}

/// Host configuration as Core sees it (no paths leave the host).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RWorkspaceStatus {
    pub stage_dir_configured: bool,
    pub ides: Vec<(IdeKind, bool)>,
    pub runtime: RRuntimeIdentity,
    pub layout_version: u32,
    pub staging_mode: StagingMode,
    /// Always false in this slice.
    pub managed_run_admitted: bool,
    pub platform_qualified: bool,
}

// ---------------------------------------------------------------------
// Launch, run and publication receipts.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchState {
    /// The program was started with the workspace path as its only
    /// argument. This is not proof that any analysis ran.
    Launched,
    IdeNotConfigured,
    IdeNotFound,
    WorkspaceMissing,
    WorkspaceChanged,
    SpawnFailed,
}

closed_vocabulary!(LaunchState, "launch state", {
    Launched => "launched",
    IdeNotConfigured => "ide_not_configured",
    IdeNotFound => "ide_not_found",
    WorkspaceMissing => "workspace_missing",
    WorkspaceChanged => "workspace_changed",
    SpawnFailed => "spawn_failed",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RLaunchReceipt {
    pub header: ObjectHeader,
    pub workspace_id: OpaqueId,
    pub project_id: OpaqueId,
    pub ide: IdeKind,
    /// The program path exactly as configured (never run through a shell).
    pub program: Option<String>,
    pub descriptor_digest: DigestSha256,
    /// Names (never values) of the environment variables passed on.
    pub env_names: Vec<String>,
    pub state: LaunchState,
}

impl RLaunchReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.state == LaunchState::IdeNotConfigured && self.program.is_some() {
            return Err("an unconfigured launch names no program".to_owned());
        }
        if self.state != LaunchState::IdeNotConfigured && self.program.is_none() {
            return Err("a configured launch names its program".to_owned());
        }
        if self.state != LaunchState::Launched && !self.env_names.is_empty() {
            return Err("only a launch passes an environment".to_owned());
        }
        if !self.env_names.iter().all(|n| launch_env_allowed(n)) {
            return Err("environment name outside the allowlist".to_owned());
        }
        Ok(())
    }
}

/// Why a managed R run did not happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunRefusal {
    /// User R code is arbitrary code; Compute does not admit it while the
    /// OS sandbox is not platform qualified.
    ComputeDeniedPlatformUnqualified,
    WorkspaceMissing,
    WorkspaceChanged,
    InputStale,
    ScriptInvalid,
}

closed_vocabulary!(RunRefusal, "run refusal", {
    ComputeDeniedPlatformUnqualified => "compute_denied_platform_unqualified",
    WorkspaceMissing => "workspace_missing",
    WorkspaceChanged => "workspace_changed",
    InputStale => "input_stale",
    ScriptInvalid => "script_invalid",
});

/// A managed run request, recorded and refused in this slice. Nothing
/// executes; the receipt holds what a run would have been bound to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RRunReceipt {
    pub header: ObjectHeader,
    pub workspace_id: OpaqueId,
    pub project_id: OpaqueId,
    pub descriptor_digest: DigestSha256,
    /// The name as requested (bounded; `scripts/<name>` when valid).
    pub script: String,
    pub script_digest: Option<DigestSha256>,
    pub runtime: RRuntimeIdentity,
    pub lockfile: Option<RLockfileRef>,
    pub refusal: RunRefusal,
}

impl RRunReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.script.chars().count() > OUTPUT_NAME_MAX_CHARS {
            return Err("script name too long".to_owned());
        }
        if self.refusal == RunRefusal::ComputeDeniedPlatformUnqualified
            && self.script_digest.is_none()
        {
            return Err("a platform refusal names the script it refused".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishState {
    Published,
    Refused,
}

closed_vocabulary!(PublishState, "publish state", {
    Published => "published",
    Refused => "refused",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishRefusal {
    BadName,
    NotFound,
    /// A link, directory or other non-regular file.
    NotRegularFile,
    TooLarge,
    /// The bytes read differ from the expected digest, or the file changed
    /// while it was read.
    OutputChanged,
    /// Not a UTF-8 CSV table MedScale can type exactly.
    OutputInvalid,
    WorkspaceMissing,
    WorkspaceChanged,
    /// An input is no longer the Project's snapshot with its pinned digest.
    InputStale,
}

closed_vocabulary!(PublishRefusal, "publish refusal", {
    BadName => "bad_name",
    NotFound => "not_found",
    NotRegularFile => "not_regular_file",
    TooLarge => "too_large",
    OutputChanged => "output_changed",
    OutputInvalid => "output_invalid",
    WorkspaceMissing => "workspace_missing",
    WorkspaceChanged => "workspace_changed",
    InputStale => "input_stale",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RPublishReceipt {
    pub header: ObjectHeader,
    pub workspace_id: OpaqueId,
    pub project_id: OpaqueId,
    pub descriptor_digest: DigestSha256,
    /// The name as requested (bounded).
    pub output_name: String,
    /// Digest of the exact file bytes read, when read.
    pub source_digest: Option<DigestSha256>,
    pub lockfile: Option<RLockfileRef>,
    pub state: PublishState,
    pub refusal: Option<PublishRefusal>,
    pub table_id: Option<OpaqueId>,
    pub table_digest: Option<DigestSha256>,
    pub row_count: u64,
    pub column_count: u32,
}

impl RPublishReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.output_name.chars().count() > OUTPUT_NAME_MAX_CHARS {
            return Err("output name too long".to_owned());
        }
        let published = self.state == PublishState::Published;
        if published == self.refusal.is_some() {
            return Err("exactly refused receipts carry a reason".to_owned());
        }
        if published != self.table_id.is_some()
            || published != self.table_digest.is_some()
            || (published && self.source_digest.is_none())
        {
            return Err("exactly published receipts name a table and its source".to_owned());
        }
        if !published && (self.row_count != 0 || self.column_count != 0) {
            return Err("refused receipt has rows".to_owned());
        }
        Ok(())
    }
}

/// A published output: a derived, unreviewed table in the Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RPublishedTable {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub workspace_id: OpaqueId,
    pub receipt_id: OpaqueId,
    pub output_name: String,
    pub source_digest: DigestSha256,
    pub content_digest: DigestSha256,
    pub row_count: u64,
    pub column_count: u32,
    /// The workspace's input snapshots, in staging order.
    pub derived_from: Vec<OpaqueId>,
    pub data_class: DataClass,
    pub reviewed: bool,
}

impl RPublishedTable {
    pub fn validate(&self) -> Result<(), String> {
        if self.reviewed {
            return Err("a published R output is unreviewed".to_owned());
        }
        if self.derived_from.is_empty() {
            return Err("a published R output names its inputs".to_owned());
        }
        validate_output_name(&self.output_name)
    }
}

/// One publication with what it produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RPublishView {
    pub receipt: RPublishReceipt,
    pub table: Option<RPublishedTable>,
    pub content: Option<ResultTableDoc>,
}

/// Everything recorded about one workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RWorkspaceHistory {
    pub workspace: RWorkspace,
    pub launches: Vec<RLaunchReceipt>,
    pub runs: Vec<RRunReceipt>,
    pub publications: Vec<RPublishReceipt>,
}

/// Bounds a requested name for a receipt without trusting it.
#[must_use]
pub fn bounded_name(name: &str) -> String {
    name.chars().take(OUTPUT_NAME_MAX_CHARS).collect()
}

// ---------------------------------------------------------------------
// Deterministic staging content.
// ---------------------------------------------------------------------

fn csv_field(out: &mut String, text: &str) {
    if text.is_empty()
        || text.contains([',', '"', '\n', '\r'])
        || text.starts_with(' ')
        || text.ends_with(' ')
    {
        out.push('"');
        out.push_str(&text.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(text);
    }
}

/// Canonical CSV of a snapshot: header row, one line per row, `\n` line
/// endings, RFC 4180 quoting, an empty unquoted field for null and `""`
/// for empty text, `true`/`false`, and floats in round-trip form.
#[must_use]
pub fn render_csv(fields: &[SchemaField], rows: &[Vec<CellValue>]) -> Vec<u8> {
    let mut out = String::new();
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        csv_field(&mut out, &f.name);
    }
    out.push('\n');
    for row in rows {
        for i in 0..fields.len() {
            if i > 0 {
                out.push(',');
            }
            match row.get(i).unwrap_or(&CellValue::Null) {
                CellValue::Null => {}
                CellValue::Text(t) => csv_field(&mut out, t),
                CellValue::Integer(v) => {
                    let _ = write!(out, "{v}");
                }
                CellValue::Float(v) => {
                    let _ = write!(out, "{v:?}");
                }
                CellValue::Boolean(b) => out.push_str(if *b { "true" } else { "false" }),
            }
        }
        out.push('\n');
    }
    out.into_bytes()
}

/// Schema file: the snapshot's typed fields, for `readr` column types.
#[must_use]
pub fn render_schema(input: &RStagedInput, fields: &[SchemaField]) -> Vec<u8> {
    let columns: Vec<serde_json::Value> = fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "name": f.name,
                "type": f.field_type.as_str(),
                "nullable": f.nullable,
            })
        })
        .collect();
    serde_json::to_vec_pretty(&serde_json::json!({
        "snapshot_id": input.snapshot_id.as_str(),
        "content_sha256": input.content_digest.to_hex(),
        "schema_sha256": input.schema_fingerprint.to_hex(),
        "row_count": input.row_count,
        "data_class": input.data_class.as_str(),
        "columns": columns,
    }))
    .unwrap_or_default()
}

#[must_use]
pub fn render_rproj() -> Vec<u8> {
    b"Version: 1.0\n\nRestoreWorkspace: No\nSaveWorkspace: No\nAlwaysSaveHistory: No\n\nEnableCodeIndexing: Yes\nEncoding: UTF-8\n"
        .to_vec()
}

#[must_use]
pub fn render_readme(label: &str, data_class: DataClass, inputs: &[RStagedInput]) -> Vec<u8> {
    let mut out = format!(
        "# MedScale R workspace `{label}`\n\n\
         Staged by MedScale. Data class: `{}` (the most restrictive class of\n\
         the inputs). Treat everything here, including your outputs, at\n\
         least that restrictively.\n\n\
         - `data/` holds exact copies of MedScale snapshots (read-only).\n\
         - `scripts/` is yours.\n\
         - `outputs/` is where results go. Nothing returns to MedScale\n\
           unless you publish one file explicitly.\n\n\
         MedScale does not run R, does not install packages and does not\n\
         watch this folder. What you do in R or an IDE here, including any\n\
         network access, is outside MedScale's authority. Record packages\n\
         with `renv::snapshot()`; MedScale keeps the digest of `renv.lock`\n\
         as evidence only.\n\n\
         ## Inputs\n\n",
        data_class.as_str()
    );
    for input in inputs {
        let _ = writeln!(
            out,
            "- `{}`: snapshot `{}`, {} rows, class `{}`, sha256 `{}`",
            input.data_file,
            input.snapshot_id.as_str(),
            input.row_count,
            input.data_class.as_str(),
            input.content_digest.to_hex()
        );
    }
    out.into_bytes()
}

/// Types a published CSV table into the canonical result form.
#[must_use]
pub fn table_doc(fields: &[SchemaField], rows: Vec<Vec<CellValue>>) -> ResultTableDoc {
    ResultTableDoc {
        columns: fields
            .iter()
            .map(|f| ResultColumn {
                name: f.name.clone(),
                observed_type: f.field_type.as_str().to_owned(),
            })
            .collect(),
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_sources::FieldType;
    use crate::objects::{AuthorityScopeId, RealmId};

    fn field(name: &str, t: FieldType) -> SchemaField {
        SchemaField {
            name: name.to_owned(),
            field_type: t,
            nullable: true,
            declared_unit: None,
        }
    }

    fn header(id: &str) -> ObjectHeader {
        ObjectHeader {
            id: OpaqueId::new(id),
            schema_version: R_WORKSPACE_SCHEMA_VERSION,
            realm_id: RealmId::new("realm"),
            authority_scope_id: AuthorityScopeId::new("scope"),
        }
    }

    fn digest(s: &str) -> DigestSha256 {
        DigestSha256::of(s.as_bytes())
    }

    fn manifest() -> RWorkspaceManifest {
        let snapshot = OpaqueId::new("snapshot-1");
        let input = RStagedInput {
            data_file: data_file_name(&snapshot),
            schema_file: schema_file_name(&snapshot),
            snapshot_id: snapshot,
            content_digest: digest("c"),
            schema_fingerprint: digest("s"),
            row_count: 2,
            data_class: DataClass::TeamProtected,
        };
        let file = |path: &str| StagedFile {
            path: path.to_owned(),
            digest: digest(path),
            bytes: 1,
        };
        RWorkspaceManifest {
            header: header("r-workspace-1"),
            project_id: OpaqueId::new("project-1"),
            label: "labs".to_owned(),
            layout_version: R_WORKSPACE_LAYOUT_VERSION,
            stage_root: "/tmp/labs-r-workspace-1".to_owned(),
            staging_mode: StagingMode::CsvCopy,
            output_policy: OutputPolicy::ExplicitPublishOnly,
            data_class: DataClass::TeamProtected,
            class_basis: ClassBasis::InheritedFromInputs,
            files: vec![
                file(README_FILE),
                file("data/snapshot-1.csv"),
                file("data/snapshot-1.schema.json"),
                file("labs.Rproj"),
            ],
            inputs: vec![input],
        }
    }

    #[test]
    fn csv_is_canonical_and_quoted() {
        let fields = vec![
            field("id", FieldType::Integer),
            field("note, text", FieldType::Text),
            field("ldl", FieldType::Float),
            field("ok", FieldType::Boolean),
        ];
        let rows = vec![
            vec![
                CellValue::Integer(1),
                CellValue::Text("say \"hi\"".into()),
                CellValue::Float(3.0),
                CellValue::Boolean(true),
            ],
            vec![
                CellValue::Integer(2),
                CellValue::Text(" padded".into()),
                CellValue::Null,
                CellValue::Boolean(false),
            ],
            vec![
                CellValue::Integer(3),
                CellValue::Text(String::new()),
                CellValue::Float(-0.5),
                CellValue::Null,
            ],
        ];
        let csv = String::from_utf8(render_csv(&fields, &rows)).unwrap();
        assert_eq!(
            csv,
            "id,\"note, text\",ldl,ok\n1,\"say \"\"hi\"\"\",3.0,true\n2,\" padded\",,false\n3,\"\",-0.5,\n"
        );
        assert_eq!(render_csv(&fields, &rows), csv.into_bytes());
    }

    #[test]
    fn names_are_plain() {
        for bad in [
            "", ".", "..", "../x.csv", "a/b.csv", "a\\b.csv", ".hidden", "x\0.csv", "C:x.csv",
        ] {
            assert!(validate_output_name(bad).is_err(), "{bad:?}");
        }
        assert!(validate_output_name("result-1.csv").is_ok());
        assert!(validate_script_name("analysis.R").is_ok());
        for bad in ["analysis.py", ".R", "../a.R", "a/b.R", "R"] {
            assert!(validate_script_name(bad).is_err(), "{bad:?}");
        }
        for bad in ["", "-x", "a b", "a/b", "é", "..", "a.b"] {
            assert!(validate_label(bad).is_err(), "{bad:?}");
        }
        assert!(validate_label("labs_2026").is_ok());
        assert_eq!(
            data_file_name(&OpaqueId::new("snapshot-1")),
            "data/snapshot-1.csv"
        );
        assert_eq!(
            data_file_name(&OpaqueId::new("../../x")),
            "data/______x.csv"
        );
        assert!(!is_generated_path("../x"));
        assert!(!is_generated_path("data/../x"));
        assert!(!is_generated_path("scripts/a.R"));
        assert!(!is_generated_path("/etc/passwd"));
        assert!(is_generated_path("data/snapshot-1.csv"));
    }

    #[test]
    fn manifest_holds_its_layout() {
        let m = manifest();
        m.validate().unwrap();
        let ws = RWorkspace {
            descriptor_digest: m.digest(),
            manifest: m.clone(),
        };
        ws.validate().unwrap();
        let mut tampered = ws.clone();
        tampered.manifest.label = "other".to_owned();
        assert!(tampered.validate().is_err());

        let mut extra = m.clone();
        extra.files.push(StagedFile {
            path: "zz.txt".to_owned(),
            digest: digest("z"),
            bytes: 1,
        });
        assert!(extra.validate().is_err(), "unlisted file admitted");
        let mut escape = m.clone();
        escape.files[0].path = "../README.md".to_owned();
        assert!(escape.validate().is_err());
        let mut unsorted = m.clone();
        unsorted.files.swap(0, 1);
        assert!(unsorted.validate().is_err());
        let mut weaker = m.clone();
        weaker.data_class = DataClass::Public;
        assert!(weaker.validate().is_err(), "class must be inherited");
        let mut dup = m;
        dup.inputs.push(dup.inputs[0].clone());
        assert!(dup.validate().is_err());
    }

    #[test]
    fn class_is_the_most_restrictive() {
        assert_eq!(most_restrictive([]), DataClass::LocalPhi);
        assert_eq!(
            most_restrictive([DataClass::Public, DataClass::TeamProtected]),
            DataClass::TeamProtected
        );
        assert_eq!(
            most_restrictive([DataClass::LocalPhi, DataClass::Public]),
            DataClass::LocalPhi
        );
    }

    #[test]
    fn launch_environment_is_allowlisted() {
        assert!(launch_env_allowed("PATH"));
        assert!(launch_env_allowed("Path"));
        assert!(launch_env_allowed("SystemRoot"));
        for denied in [
            "MEDSCALE_PASSPHRASE",
            "medscale_vault",
            "AWS_SECRET_ACCESS_KEY",
            "GITHUB_TOKEN",
            "R_LIBS",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "HTTPS_PROXY",
        ] {
            assert!(!launch_env_allowed(denied), "{denied}");
        }
    }

    #[test]
    fn vocabularies_are_closed() {
        assert!(IdeKind::parse("vscode").is_err());
        assert!(serde_json::from_str::<StagingMode>(r#""parquet""#).is_err());
        assert!(serde_json::from_str::<OutputPolicy>(r#""auto_import""#).is_err());
        assert!(serde_json::from_str::<RVersionEvidence>(r#""probed""#).is_err());
        for r in RunRefusal::ALL {
            assert_eq!(RunRefusal::parse(r.as_str()).unwrap(), *r);
        }
        for r in PublishRefusal::ALL {
            assert_eq!(PublishRefusal::parse(r.as_str()).unwrap(), *r);
        }
        assert!(
            serde_json::from_str::<RPublishRequest>(
                r#"{"workspace_id":"r-workspace-1","output_name":"x.csv","expected_digest":null,"path":"/etc"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn receipts_hold_their_invariants() {
        let refused = RPublishReceipt {
            header: header("r-publish-1"),
            workspace_id: OpaqueId::new("r-workspace-1"),
            project_id: OpaqueId::new("project-1"),
            descriptor_digest: digest("d"),
            output_name: "x.csv".into(),
            source_digest: None,
            lockfile: None,
            state: PublishState::Refused,
            refusal: Some(PublishRefusal::NotFound),
            table_id: None,
            table_digest: None,
            row_count: 0,
            column_count: 0,
        };
        refused.validate().unwrap();
        let mut bad = refused.clone();
        bad.refusal = None;
        assert!(bad.validate().is_err());
        let mut rows = refused.clone();
        rows.row_count = 1;
        assert!(rows.validate().is_err());
        let mut published = refused;
        published.state = PublishState::Published;
        published.refusal = None;
        assert!(published.validate().is_err(), "published without a table");
        published.table_id = Some(OpaqueId::new("r-table-1"));
        published.table_digest = Some(digest("t"));
        published.source_digest = Some(digest("s"));
        published.validate().unwrap();

        let launch = RLaunchReceipt {
            header: header("r-launch-1"),
            workspace_id: OpaqueId::new("r-workspace-1"),
            project_id: OpaqueId::new("project-1"),
            ide: IdeKind::Folder,
            program: Some("/usr/bin/xdg-open".into()),
            descriptor_digest: digest("d"),
            env_names: vec!["PATH".into()],
            state: LaunchState::Launched,
        };
        launch.validate().unwrap();
        let mut leaky = launch.clone();
        leaky.env_names.push("MEDSCALE_PASSPHRASE".into());
        assert!(leaky.validate().is_err());
        let mut unconfigured = launch;
        unconfigured.state = LaunchState::IdeNotConfigured;
        unconfigured.env_names.clear();
        assert!(unconfigured.validate().is_err());
        unconfigured.program = None;
        unconfigured.validate().unwrap();

        let run = RRunReceipt {
            header: header("r-run-1"),
            workspace_id: OpaqueId::new("r-workspace-1"),
            project_id: OpaqueId::new("project-1"),
            descriptor_digest: digest("d"),
            script: "a.R".into(),
            script_digest: None,
            runtime: RRuntimeIdentity {
                program: None,
                found: false,
                program_digest: None,
                version: RVersionEvidence::NotProbed,
            },
            lockfile: None,
            refusal: RunRefusal::ComputeDeniedPlatformUnqualified,
        };
        assert!(run.validate().is_err());
    }
}

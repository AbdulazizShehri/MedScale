//! MedScale Compute contracts (Spec 085).
//!
//! Core admits a typed job over one exact Spec 075 snapshot, pins the
//! snapshot's content digest, and runs the job in a separate MedScale-owned
//! worker process (`medscale-compute-worker`). The worker executes only the
//! closed set of deterministic job kinds below, implemented here; there is
//! no shell, Python, R, script, plugin or network path. The worker's answer
//! is a candidate until Core validates its binding, digest, canonical
//! encoding and shape, and commits it as a derived output.
//!
//! The worker applies its OS's `ReadyBaseMeasured` mechanism to itself and
//! reports it. That is defense in depth for MedScale's own code, never a
//! claim of `platform_qualified` isolation for untrusted code.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::analytics::{ResultColumn, ResultTableDoc};
use crate::data_sources::{CellValue, SchemaField, SnapshotCanonicalDoc};
use crate::objects::{DigestSha256, ObjectHeader, OpaqueId};

/// Durable schema version for every Compute object (Spec 085 v1).
pub const COMPUTE_SCHEMA_VERSION: u32 = 1;
/// Core <-> worker protocol version.
pub const COMPUTE_PROTOCOL_VERSION: u32 = 1;
/// The only worker executable Core launches.
pub const COMPUTE_WORKER_NAME: &str = "medscale-compute-worker";

pub const INPUT_BYTES_MAX: u64 = 64 * 1024 * 1024;
pub const INPUT_BYTES_DEFAULT: u64 = 16 * 1024 * 1024;
pub const OUTPUT_BYTES_MAX: u64 = 64 * 1024 * 1024;
pub const OUTPUT_BYTES_DEFAULT: u64 = 16 * 1024 * 1024;
pub const OUTPUT_ROWS_MAX: u64 = 1_000_000;
pub const OUTPUT_ROWS_DEFAULT: u64 = 100_000;
pub const STDERR_BYTES_MAX: u64 = 64 * 1024;
pub const STDERR_BYTES_DEFAULT: u64 = 16 * 1024;
pub const TIMEOUT_MS_MIN: u64 = 100;
pub const TIMEOUT_MS_MAX: u64 = 600_000;
pub const TIMEOUT_MS_DEFAULT: u64 = 60_000;
pub const PROJECTION_COLUMNS_MAX: usize = 64;
pub const SORT_KEYS_MAX: usize = 8;
pub const COLUMN_NAME_MAX_CHARS: usize = 128;
/// Protocol bytes allowed on stdout beyond the output itself (the JSON
/// envelope and string escaping of the embedded output).
pub const STDOUT_OVERHEAD_BYTES: u64 = 64 * 1024;
/// Largest request the worker reads from stdin.
pub const WORKER_REQUEST_BYTES_MAX: u64 = INPUT_BYTES_MAX * 2 + 64 * 1024;

/// Output columns of `column_profile` v1, in order, with observed types.
pub const PROFILE_COLUMNS: [(&str, &str); 8] = [
    ("column", "text"),
    ("field_type", "text"),
    ("non_null", "integer"),
    ("missing", "integer"),
    ("distinct", "integer"),
    ("min", "mixed"),
    ("max", "mixed"),
    ("mean", "float"),
];

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

/// Closed set of admitted job kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeJobKind {
    ColumnProfile,
    SortedProjection,
}

closed_vocabulary!(ComputeJobKind, "compute job kind", {
    ColumnProfile => "column_profile",
    SortedProjection => "sorted_projection",
});

impl ComputeJobKind {
    /// Version of the kind's algorithm and output shape.
    #[must_use]
    pub const fn kind_version(self) -> u32 {
        1
    }
}

/// Typed parameters; the tag is the job kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComputeParams {
    /// An empty struct variant, not a unit variant: serde ignores unknown
    /// fields on unit variants of internally tagged enums.
    ColumnProfile {},
    SortedProjection {
        columns: Vec<String>,
        sort_by: Vec<String>,
        descending: bool,
    },
}

fn check_names(names: &[String], what: &str, min: usize, max: usize) -> Result<(), String> {
    if names.len() < min || names.len() > max {
        return Err(format!("{what} count out of bounds"));
    }
    let mut seen = BTreeSet::new();
    for name in names {
        if name.trim().is_empty()
            || name.chars().count() > COLUMN_NAME_MAX_CHARS
            || name.contains('\0')
        {
            return Err(format!("{what} name is empty, too long or has NUL"));
        }
        if !seen.insert(name.as_str()) {
            return Err(format!("duplicate {what} name"));
        }
    }
    Ok(())
}

impl ComputeParams {
    #[must_use]
    pub const fn kind(&self) -> ComputeJobKind {
        match self {
            Self::ColumnProfile {} => ComputeJobKind::ColumnProfile,
            Self::SortedProjection { .. } => ComputeJobKind::SortedProjection,
        }
    }

    /// Checks bounds that do not depend on the input.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::ColumnProfile {} => Ok(()),
            Self::SortedProjection {
                columns, sort_by, ..
            } => {
                check_names(columns, "projection column", 1, PROJECTION_COLUMNS_MAX)?;
                check_names(sort_by, "sort key", 0, SORT_KEYS_MAX)
            }
        }
    }

    /// Every named column must exist in the input schema.
    #[must_use]
    pub fn columns_exist(&self, fields: &[SchemaField]) -> bool {
        match self {
            Self::ColumnProfile {} => true,
            Self::SortedProjection {
                columns, sort_by, ..
            } => columns
                .iter()
                .chain(sort_by)
                .all(|name| fields.iter().any(|f| &f.name == name)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPosture {
    Denied,
}

/// Only the closed job kinds run; no shell, interpreter or user code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodePosture {
    ClosedKindsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentPosture {
    Cleared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlePosture {
    StdioPipesOnly,
}

/// The worker's working directory is a new empty directory outside the
/// vault; input arrives on stdin, never as a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemPosture {
    EmptyScratchOnly,
}

/// What isolation a job requires before the worker computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxRequirement {
    /// The worker must apply its OS's `ReadyBaseMeasured` mechanism, or the
    /// job ends `unavailable` without computing (the default).
    ReadyBaseRequired,
    /// A separate process with the fixed posture above is enough.
    ProcessIsolationOnly,
}

closed_vocabulary!(SandboxRequirement, "sandbox requirement", {
    ReadyBaseRequired => "ready_base_required",
    ProcessIsolationOnly => "process_isolation_only",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPolicy {
    pub network: NetworkPosture,
    pub code: CodePosture,
    pub environment: EnvironmentPosture,
    pub handles: HandlePosture,
    pub filesystem: FilesystemPosture,
    pub sandbox: SandboxRequirement,
}

impl ExecutionPolicy {
    #[must_use]
    pub const fn fixed(sandbox: SandboxRequirement) -> Self {
        Self {
            network: NetworkPosture::Denied,
            code: CodePosture::ClosedKindsOnly,
            environment: EnvironmentPosture::Cleared,
            handles: HandlePosture::StdioPipesOnly,
            filesystem: FilesystemPosture::EmptyScratchOnly,
            sandbox,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCeilings {
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_output_rows: u64,
    pub max_stderr_bytes: u64,
    pub timeout_ms: u64,
}

impl Default for ResourceCeilings {
    fn default() -> Self {
        Self {
            max_input_bytes: INPUT_BYTES_DEFAULT,
            max_output_bytes: OUTPUT_BYTES_DEFAULT,
            max_output_rows: OUTPUT_ROWS_DEFAULT,
            max_stderr_bytes: STDERR_BYTES_DEFAULT,
            timeout_ms: TIMEOUT_MS_DEFAULT,
        }
    }
}

impl ResourceCeilings {
    pub fn validate(&self) -> Result<(), String> {
        let ok = (1..=INPUT_BYTES_MAX).contains(&self.max_input_bytes)
            && (1..=OUTPUT_BYTES_MAX).contains(&self.max_output_bytes)
            && (1..=OUTPUT_ROWS_MAX).contains(&self.max_output_rows)
            && (1..=STDERR_BYTES_MAX).contains(&self.max_stderr_bytes)
            && (TIMEOUT_MS_MIN..=TIMEOUT_MS_MAX).contains(&self.timeout_ms);
        if ok {
            Ok(())
        } else {
            Err("resource ceiling out of bounds".to_owned())
        }
    }

    /// Largest stdout Core accepts before killing the worker.
    #[must_use]
    pub const fn stdout_ceiling(&self) -> u64 {
        self.max_output_bytes
            .saturating_mul(2)
            .saturating_add(STDOUT_OVERHEAD_BYTES)
    }
}

/// A submission. Core allocates the job id and pins the input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeJobRequest {
    pub project_id: OpaqueId,
    pub snapshot_id: OpaqueId,
    pub params: ComputeParams,
    #[serde(default)]
    pub sandbox: Option<SandboxRequirement>,
    #[serde(default)]
    pub limits: Option<ResourceCeilings>,
}

/// The exact staged input: the snapshot's canonical bytes, pinned by
/// the snapshot's own content digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedInput {
    pub snapshot_id: OpaqueId,
    pub content_digest: DigestSha256,
    pub schema_fingerprint: DigestSha256,
    pub row_count: u64,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIdentity {
    pub worker: String,
    pub worker_version: String,
    pub protocol_version: u32,
    pub kind_version: u32,
}

impl RuntimeIdentity {
    #[must_use]
    pub fn current(kind: ComputeJobKind) -> Self {
        Self {
            worker: COMPUTE_WORKER_NAME.to_owned(),
            worker_version: crate::MEDSCALE_VERSION.to_owned(),
            protocol_version: COMPUTE_PROTOCOL_VERSION,
            kind_version: kind.kind_version(),
        }
    }
}

/// Everything a job binds. Immutable after submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeManifest {
    pub job_id: OpaqueId,
    pub project_id: OpaqueId,
    pub kind: ComputeJobKind,
    pub params: ComputeParams,
    /// The snapshot the submission named.
    pub requested_snapshot_id: OpaqueId,
    /// The pinned input; `None` only when it could not be read at
    /// admission (the job is then `denied` or `corrupt`, never run).
    pub input: Option<StagedInput>,
    pub runtime: RuntimeIdentity,
    pub policy: ExecutionPolicy,
    pub limits: ResourceCeilings,
}

impl ComputeManifest {
    #[must_use]
    pub fn digest(&self) -> DigestSha256 {
        DigestSha256::of(&serde_json::to_vec(self).unwrap_or_default())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.kind != self.params.kind() {
            return Err("manifest kind does not match its params".to_owned());
        }
        if self.runtime.worker != COMPUTE_WORKER_NAME
            || self.runtime.protocol_version != COMPUTE_PROTOCOL_VERSION
            || self.runtime.kind_version != self.kind.kind_version()
            || self.runtime.worker_version.is_empty()
        {
            return Err("manifest runtime identity is not admitted".to_owned());
        }
        if self
            .input
            .as_ref()
            .is_some_and(|i| i.snapshot_id != self.requested_snapshot_id)
        {
            return Err("manifest input is not the requested snapshot".to_owned());
        }
        Ok(())
    }
}

/// Job states. Every state except `queued` and `running` is terminal and
/// has exactly one receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeState {
    Queued,
    Running,
    Completed,
    Cancelled,
    TimedOut,
    Denied,
    ResourceExhausted,
    Failed,
    Unavailable,
    Corrupt,
    /// Left `running` by a crash or restart; never re-executed automatically.
    Interrupted,
}

closed_vocabulary!(ComputeState, "compute state", {
    Queued => "queued",
    Running => "running",
    Completed => "completed",
    Cancelled => "cancelled",
    TimedOut => "timed_out",
    Denied => "denied",
    ResourceExhausted => "resource_exhausted",
    Failed => "failed",
    Unavailable => "unavailable",
    Corrupt => "corrupt",
    Interrupted => "interrupted",
});

impl ComputeState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }
}

/// Why a submission was refused at admission (or its input vanished).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeDenyReason {
    /// Missing, another Project's or out-of-scope snapshot.
    InputUnavailable,
    UnknownColumn,
    BadParams,
    BadLimits,
    InputTooLarge,
}

closed_vocabulary!(ComputeDenyReason, "compute deny reason", {
    InputUnavailable => "input_unavailable",
    UnknownColumn => "unknown_column",
    BadParams => "bad_params",
    BadLimits => "bad_limits",
    InputTooLarge => "input_too_large",
});

/// Fixed failure codes; never worker text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeFailure {
    WorkerMissing,
    SandboxUnavailable,
    /// The worker saw environment variables: the cleared-environment
    /// posture did not hold, so isolation is treated as unavailable.
    EnvironmentNotCleared,
    SpawnFailed,
    WorkerCrashed,
    WorkerRefused,
    MalformedProtocol,
    JobMismatch,
    OutputDigestMismatch,
    OutputInvalid,
    InputDigestMismatch,
    OutputTooLarge,
    RowLimitExceeded,
    TimeLimit,
    CancelledByUser,
    InterruptedByRestart,
}

closed_vocabulary!(ComputeFailure, "compute failure", {
    WorkerMissing => "worker_missing",
    SandboxUnavailable => "sandbox_unavailable",
    EnvironmentNotCleared => "environment_not_cleared",
    SpawnFailed => "spawn_failed",
    WorkerCrashed => "worker_crashed",
    WorkerRefused => "worker_refused",
    MalformedProtocol => "malformed_protocol",
    JobMismatch => "job_mismatch",
    OutputDigestMismatch => "output_digest_mismatch",
    OutputInvalid => "output_invalid",
    InputDigestMismatch => "input_digest_mismatch",
    OutputTooLarge => "output_too_large",
    RowLimitExceeded => "row_limit_exceeded",
    TimeLimit => "time_limit",
    CancelledByUser => "cancelled_by_user",
    InterruptedByRestart => "interrupted_by_restart",
});

impl ComputeFailure {
    /// The one terminal state each failure belongs to.
    #[must_use]
    pub const fn state(self) -> ComputeState {
        match self {
            Self::WorkerMissing | Self::SandboxUnavailable | Self::EnvironmentNotCleared => {
                ComputeState::Unavailable
            }
            Self::SpawnFailed | Self::WorkerCrashed | Self::WorkerRefused => ComputeState::Failed,
            Self::MalformedProtocol
            | Self::JobMismatch
            | Self::OutputDigestMismatch
            | Self::OutputInvalid
            | Self::InputDigestMismatch => ComputeState::Corrupt,
            Self::OutputTooLarge | Self::RowLimitExceeded => ComputeState::ResourceExhausted,
            Self::TimeLimit => ComputeState::TimedOut,
            Self::CancelledByUser => ComputeState::Cancelled,
            Self::InterruptedByRestart => ComputeState::Interrupted,
        }
    }
}

/// The OS mechanism the worker applied to itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMechanism {
    None,
    /// Spec 052: Landlock FS allowlist (scratch dir) + TCP deny + `RLIMIT_NOFILE`.
    LinuxLandlockComposition,
    /// Spec 030: Job Object active-process limit 1 + kill on job close.
    WindowsJobObject,
    /// Spec 031: Seatbelt network deny.
    MacosSeatbelt,
}

closed_vocabulary!(SandboxMechanism, "sandbox mechanism", {
    None => "none",
    LinuxLandlockComposition => "linux_landlock_composition",
    WindowsJobObject => "windows_job_object",
    MacosSeatbelt => "macos_seatbelt",
});

impl SandboxMechanism {
    /// The `ReadyBaseMeasured` mechanism this build's OS has.
    #[must_use]
    pub const fn for_this_os() -> Self {
        if cfg!(target_os = "linux") {
            Self::LinuxLandlockComposition
        } else if cfg!(windows) {
            Self::WindowsJobObject
        } else if cfg!(target_os = "macos") {
            Self::MacosSeatbelt
        } else {
            Self::None
        }
    }
}

/// What the worker reports about its own confinement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxReport {
    pub mechanism: SandboxMechanism,
    pub ready_base_applied: bool,
    /// Always false; a report claiming otherwise is corrupt.
    pub platform_qualified: bool,
    /// Environment variables visible to the worker (0 when cleared).
    pub env_var_count: u32,
}

impl SandboxReport {
    pub fn validate(&self) -> Result<(), String> {
        if self.platform_qualified {
            return Err("worker claims platform qualification".to_owned());
        }
        if self.ready_base_applied == (self.mechanism == SandboxMechanism::None) {
            return Err("sandbox mechanism and applied flag disagree".to_owned());
        }
        Ok(())
    }
}

/// The only review state this foundation writes: outputs are derived,
/// not reviewed results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputReview {
    Unreviewed,
}

/// A job: its immutable manifest plus its current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeJob {
    pub header: ObjectHeader,
    pub manifest: ComputeManifest,
    pub manifest_digest: DigestSha256,
    pub state: ComputeState,
}

impl ComputeJob {
    pub fn validate(&self) -> Result<(), String> {
        if self.header.schema_version != COMPUTE_SCHEMA_VERSION {
            return Err("unsupported compute job schema version".to_owned());
        }
        if self.header.id != self.manifest.job_id {
            return Err("job id does not match its manifest".to_owned());
        }
        if self.manifest.digest() != self.manifest_digest {
            return Err("manifest digest mismatch".to_owned());
        }
        let runnable = self
            .manifest
            .input
            .as_ref()
            .is_some_and(|i| i.byte_len <= self.manifest.limits.max_input_bytes);
        if !runnable && !matches!(self.state, ComputeState::Denied | ComputeState::Corrupt) {
            return Err("a job without a runnable input must be denied or corrupt".to_owned());
        }
        // A job refused at admission keeps what was requested, even when
        // its params or limits were out of bounds; any other job must be
        // within them.
        if self.state != ComputeState::Denied {
            self.manifest.params.validate()?;
            self.manifest.limits.validate()?;
        }
        self.manifest.validate()
    }
}

/// One terminal outcome of one job. Immutable once written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeReceipt {
    pub header: ObjectHeader,
    pub job_id: OpaqueId,
    pub project_id: OpaqueId,
    pub kind: ComputeJobKind,
    pub manifest_digest: DigestSha256,
    pub input: Option<StagedInput>,
    pub runtime: RuntimeIdentity,
    pub sandbox: Option<SandboxReport>,
    pub state: ComputeState,
    pub deny_reason: Option<ComputeDenyReason>,
    pub failure: Option<ComputeFailure>,
    pub output_id: Option<OpaqueId>,
    pub output_digest: Option<DigestSha256>,
    pub row_count: u64,
    pub column_count: u32,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    /// Digest of the retained stderr prefix; the text is never stored.
    pub stderr_prefix_digest: Option<DigestSha256>,
}

impl ComputeReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.header.schema_version != COMPUTE_SCHEMA_VERSION {
            return Err("unsupported compute receipt schema version".to_owned());
        }
        if !self.state.is_terminal() {
            return Err("receipt state must be terminal".to_owned());
        }
        match self.state {
            ComputeState::Denied => {
                if self.deny_reason.is_none() || self.failure.is_some() {
                    return Err("denied receipt needs exactly a deny reason".to_owned());
                }
            }
            ComputeState::Completed => {
                if self.deny_reason.is_some() || self.failure.is_some() {
                    return Err("completed receipt carries a refusal".to_owned());
                }
            }
            state => {
                if self.deny_reason.is_some()
                    || self.failure.map(ComputeFailure::state) != Some(state)
                {
                    return Err("receipt failure does not match its state".to_owned());
                }
            }
        }
        let completed = self.state == ComputeState::Completed;
        if completed != self.output_id.is_some() || completed != self.output_digest.is_some() {
            return Err("exactly completed receipts name an output".to_owned());
        }
        if !completed && (self.row_count != 0 || self.column_count != 0) {
            return Err("receipt without output has rows".to_owned());
        }
        if let Some(report) = &self.sandbox {
            report.validate()?;
        }
        if completed && (self.sandbox.is_none() || self.input.is_none()) {
            return Err("completed receipt lacks its input or sandbox report".to_owned());
        }
        Ok(())
    }
}

/// A committed derived output (Core-validated candidate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeOutput {
    pub header: ObjectHeader,
    pub project_id: OpaqueId,
    pub job_id: OpaqueId,
    pub receipt_id: OpaqueId,
    pub kind: ComputeJobKind,
    pub content_digest: DigestSha256,
    pub row_count: u64,
    pub column_count: u32,
    pub derived_from: OpaqueId,
    pub input_digest: DigestSha256,
    pub review: OutputReview,
}

/// A job with its receipt and (when completed) output table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeJobView {
    pub job: ComputeJob,
    pub receipt: Option<ComputeReceipt>,
    pub output: Option<ComputeOutput>,
    pub table: Option<ResultTableDoc>,
}

/// Compute availability as this build sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeStatus {
    pub worker_found: bool,
    pub protocol_version: u32,
    pub expected_mechanism: SandboxMechanism,
    pub platform_qualified: bool,
    pub kinds: Vec<ComputeJobKind>,
    pub queued: u64,
    pub running: u64,
}

// ---------------------------------------------------------------------
// Worker protocol (stdin: one request; stdout: one response).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRequest {
    pub protocol_version: u32,
    pub job_id: OpaqueId,
    pub manifest_digest: DigestSha256,
    pub params: ComputeParams,
    pub sandbox: SandboxRequirement,
    pub max_output_rows: u64,
    pub input_digest: DigestSha256,
    /// The exact staged snapshot bytes (UTF-8 JSON).
    pub input_json: String,
}

/// Why the worker refused to compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRefusal {
    ProtocolMismatch,
    SandboxUnavailable,
    InputDigestMismatch,
    MalformedInput,
    BadParams,
    UnknownColumn,
    RowLimitExceeded,
}

impl WorkerRefusal {
    /// Core's reading of a worker refusal.
    #[must_use]
    pub const fn failure(self) -> ComputeFailure {
        match self {
            Self::ProtocolMismatch | Self::BadParams | Self::UnknownColumn => {
                ComputeFailure::WorkerRefused
            }
            Self::SandboxUnavailable => ComputeFailure::SandboxUnavailable,
            Self::InputDigestMismatch | Self::MalformedInput => ComputeFailure::InputDigestMismatch,
            Self::RowLimitExceeded => ComputeFailure::RowLimitExceeded,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkerOutcome {
    Completed {
        output_digest: DigestSha256,
        /// Canonical `ResultTableDoc` bytes (UTF-8 JSON).
        output_json: String,
    },
    Refused {
        refusal: WorkerRefusal,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerResponse {
    pub protocol_version: u32,
    pub job_id: OpaqueId,
    pub manifest_digest: DigestSha256,
    pub sandbox: SandboxReport,
    pub outcome: WorkerOutcome,
}

// ---------------------------------------------------------------------
// Deterministic job kinds.
// ---------------------------------------------------------------------

/// Stands in for a cell a row does not have.
static NULL_CELL: CellValue = CellValue::Null;

const fn variant_rank(cell: &CellValue) -> u8 {
    match cell {
        CellValue::Null => 0,
        CellValue::Boolean(_) => 1,
        CellValue::Integer(_) | CellValue::Float(_) => 2,
        CellValue::Text(_) => 3,
    }
}

/// Total, deterministic order over cells: null < boolean < number < text;
/// integers compare exactly, other numbers by IEEE total order; text by
/// bytes.
#[must_use]
pub fn compare_cells(a: &CellValue, b: &CellValue) -> Ordering {
    match (a, b) {
        (CellValue::Boolean(x), CellValue::Boolean(y)) => x.cmp(y),
        (CellValue::Integer(x), CellValue::Integer(y)) => x.cmp(y),
        (CellValue::Integer(x), CellValue::Float(y)) => (*x as f64).total_cmp(y),
        (CellValue::Float(x), CellValue::Integer(y)) => x.total_cmp(&(*y as f64)),
        (CellValue::Float(x), CellValue::Float(y)) => x.total_cmp(y),
        (CellValue::Text(x), CellValue::Text(y)) => x.as_bytes().cmp(y.as_bytes()),
        _ => variant_rank(a).cmp(&variant_rank(b)),
    }
}

fn distinct_key(cell: &CellValue) -> String {
    serde_json::to_string(cell).unwrap_or_default()
}

fn column_profile(doc: &SnapshotCanonicalDoc) -> ResultTableDoc {
    let columns = PROFILE_COLUMNS
        .iter()
        .map(|(name, ty)| ResultColumn {
            name: (*name).to_owned(),
            observed_type: (*ty).to_owned(),
        })
        .collect();
    let mut rows = Vec::with_capacity(doc.fields.len());
    for (index, field) in doc.fields.iter().enumerate() {
        let mut non_null: i64 = 0;
        let mut missing: i64 = 0;
        let mut distinct = BTreeSet::new();
        let mut min: Option<&CellValue> = None;
        let mut max: Option<&CellValue> = None;
        let mut sum = 0.0_f64;
        let mut numeric: i64 = 0;
        for row in &doc.rows {
            let cell = row.get(index).unwrap_or(&NULL_CELL);
            if cell.is_null() {
                missing += 1;
                continue;
            }
            non_null += 1;
            distinct.insert(distinct_key(cell));
            if min.is_none_or(|m| compare_cells(cell, m) == Ordering::Less) {
                min = Some(cell);
            }
            if max.is_none_or(|m| compare_cells(cell, m) == Ordering::Greater) {
                max = Some(cell);
            }
            match cell {
                CellValue::Integer(i) => {
                    sum += *i as f64;
                    numeric += 1;
                }
                CellValue::Float(f) => {
                    sum += *f;
                    numeric += 1;
                }
                _ => {}
            }
        }
        let mean = if numeric > 0 && numeric == non_null {
            let value = sum / numeric as f64;
            if value.is_finite() {
                CellValue::Float(value)
            } else {
                CellValue::Null
            }
        } else {
            CellValue::Null
        };
        rows.push(vec![
            CellValue::Text(field.name.clone()),
            CellValue::Text(field.field_type.as_str().to_owned()),
            CellValue::Integer(non_null),
            CellValue::Integer(missing),
            CellValue::Integer(i64::try_from(distinct.len()).unwrap_or(i64::MAX)),
            min.cloned().unwrap_or(CellValue::Null),
            max.cloned().unwrap_or(CellValue::Null),
            mean,
        ]);
    }
    ResultTableDoc { columns, rows }
}

fn column_index(fields: &[SchemaField], name: &str) -> Result<usize, WorkerRefusal> {
    fields
        .iter()
        .position(|f| f.name == name)
        .ok_or(WorkerRefusal::UnknownColumn)
}

fn sorted_projection(
    doc: &SnapshotCanonicalDoc,
    columns: &[String],
    sort_by: &[String],
    descending: bool,
) -> Result<ResultTableDoc, WorkerRefusal> {
    let picks = columns
        .iter()
        .map(|c| column_index(&doc.fields, c))
        .collect::<Result<Vec<_>, _>>()?;
    let keys = sort_by
        .iter()
        .map(|c| column_index(&doc.fields, c))
        .collect::<Result<Vec<_>, _>>()?;
    let mut order: Vec<usize> = (0..doc.rows.len()).collect();
    // `sort_by` is stable: equal keys keep input order in both directions.
    order.sort_by(|&a, &b| {
        for &k in &keys {
            let x = doc.rows[a].get(k).unwrap_or(&NULL_CELL);
            let y = doc.rows[b].get(k).unwrap_or(&NULL_CELL);
            let ord = compare_cells(x, y);
            let ord = if descending { ord.reverse() } else { ord };
            if ord != Ordering::Equal {
                return ord;
            }
        }
        Ordering::Equal
    });
    let out_columns = picks
        .iter()
        .map(|&i| ResultColumn {
            name: doc.fields[i].name.clone(),
            observed_type: doc.fields[i].field_type.as_str().to_owned(),
        })
        .collect();
    let rows = order
        .into_iter()
        .map(|r| {
            picks
                .iter()
                .map(|&i| doc.rows[r].get(i).cloned().unwrap_or(CellValue::Null))
                .collect()
        })
        .collect();
    Ok(ResultTableDoc {
        columns: out_columns,
        rows,
    })
}

/// Runs one admitted job kind over a decoded input.
pub fn execute(
    params: &ComputeParams,
    doc: &SnapshotCanonicalDoc,
    max_output_rows: u64,
) -> Result<ResultTableDoc, WorkerRefusal> {
    params.validate().map_err(|_| WorkerRefusal::BadParams)?;
    let rows_out = match params {
        ComputeParams::ColumnProfile {} => doc.fields.len() as u64,
        ComputeParams::SortedProjection { .. } => doc.rows.len() as u64,
    };
    if rows_out > max_output_rows {
        return Err(WorkerRefusal::RowLimitExceeded);
    }
    match params {
        ComputeParams::ColumnProfile {} => Ok(column_profile(doc)),
        ComputeParams::SortedProjection {
            columns,
            sort_by,
            descending,
        } => sorted_projection(doc, columns, sort_by, *descending),
    }
}

/// The worker's whole job after reading its request and applying its
/// sandbox: check, decode, compute, encode.
#[must_use]
pub fn respond(request: &WorkerRequest, sandbox: SandboxReport) -> WorkerResponse {
    let outcome = respond_outcome(request, &sandbox);
    WorkerResponse {
        protocol_version: COMPUTE_PROTOCOL_VERSION,
        job_id: request.job_id.clone(),
        manifest_digest: request.manifest_digest.clone(),
        sandbox,
        outcome,
    }
}

fn respond_outcome(request: &WorkerRequest, sandbox: &SandboxReport) -> WorkerOutcome {
    let refuse = |refusal| WorkerOutcome::Refused { refusal };
    if request.protocol_version != COMPUTE_PROTOCOL_VERSION {
        return refuse(WorkerRefusal::ProtocolMismatch);
    }
    if request.sandbox == SandboxRequirement::ReadyBaseRequired && !sandbox.ready_base_applied {
        return refuse(WorkerRefusal::SandboxUnavailable);
    }
    if DigestSha256::of(request.input_json.as_bytes()) != request.input_digest {
        return refuse(WorkerRefusal::InputDigestMismatch);
    }
    let Ok(doc) = serde_json::from_str::<SnapshotCanonicalDoc>(&request.input_json) else {
        return refuse(WorkerRefusal::MalformedInput);
    };
    match execute(&request.params, &doc, request.max_output_rows) {
        Ok(table) => {
            let bytes = table.canonical_bytes();
            WorkerOutcome::Completed {
                output_digest: DigestSha256::of(&bytes),
                output_json: String::from_utf8(bytes).unwrap_or_default(),
            }
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Core-side shape check of a candidate output against its job.
pub fn validate_output(
    params: &ComputeParams,
    fields: &[SchemaField],
    input_rows: u64,
    table: &ResultTableDoc,
) -> Result<(), String> {
    let width = table.columns.len();
    for row in &table.rows {
        if row.len() != width {
            return Err("output row width differs from its columns".to_owned());
        }
        for cell in row {
            cell.validate()?;
        }
    }
    match params {
        ComputeParams::ColumnProfile {} => {
            let expected: Vec<(&str, &str)> = table
                .columns
                .iter()
                .map(|c| (c.name.as_str(), c.observed_type.as_str()))
                .collect();
            if expected != PROFILE_COLUMNS {
                return Err("profile columns are not the admitted shape".to_owned());
            }
            if table.rows.len() != fields.len() {
                return Err("profile must have one row per input column".to_owned());
            }
            for (row, field) in table.rows.iter().zip(fields) {
                let named = row[0] == CellValue::Text(field.name.clone())
                    && row[1] == CellValue::Text(field.field_type.as_str().to_owned());
                let counts = match (&row[2], &row[3], &row[4]) {
                    (CellValue::Integer(n), CellValue::Integer(m), CellValue::Integer(d)) => {
                        *n >= 0
                            && *m >= 0
                            && *d >= 0
                            && *d <= *n
                            && u64::try_from(n + m).ok() == Some(input_rows)
                    }
                    _ => false,
                };
                if !named || !counts {
                    return Err("profile row does not describe its input column".to_owned());
                }
            }
        }
        ComputeParams::SortedProjection { columns, .. } => {
            if width != columns.len() {
                return Err("projection width differs from its params".to_owned());
            }
            for (col, name) in table.columns.iter().zip(columns) {
                let Some(field) = fields.iter().find(|f| &f.name == name) else {
                    return Err("projection names an unknown column".to_owned());
                };
                if &col.name != name || col.observed_type != field.field_type.as_str() {
                    return Err("projection column does not match its params".to_owned());
                }
            }
            if table.rows.len() as u64 != input_rows {
                return Err("projection must keep every input row".to_owned());
            }
        }
    }
    Ok(())
}

/// Applies this OS's `ReadyBaseMeasured` mechanism to the current process
/// (the worker calls this on itself) and reports what took effect. On
/// Linux the Landlock allowlist is the working directory, which Core makes
/// a new empty scratch directory.
#[must_use]
pub fn apply_worker_sandbox(env_var_count: u32) -> SandboxReport {
    use crate::os_sandbox::{OsSandboxPlan, try_apply_os_sandbox};
    let mechanism = SandboxMechanism::for_this_os();
    let plan = match mechanism {
        SandboxMechanism::LinuxLandlockComposition => std::env::current_dir()
            .ok()
            .and_then(|dir| dir.to_str().map(str::to_owned))
            .map(|dir| OsSandboxPlan::linux_landlock_composition_ready_base(vec![dir])),
        SandboxMechanism::WindowsJobObject => Some(OsSandboxPlan::windows_job_object_ready_base()),
        SandboxMechanism::MacosSeatbelt => Some(OsSandboxPlan::macos_seatbelt_ready_base()),
        SandboxMechanism::None => None,
    };
    let applied = plan.is_some_and(|plan| try_apply_os_sandbox(&plan).is_ok());
    SandboxReport {
        mechanism: if applied {
            mechanism
        } else {
            SandboxMechanism::None
        },
        ready_base_applied: applied,
        platform_qualified: false,
        env_var_count,
    }
}

/// Locates a MedScale executable next to the current one (or, for test
/// binaries under `target/<profile>/deps`, in `target/<profile>`). There
/// is no environment or configuration override.
#[must_use]
pub fn resolve_sibling_exe(name: &str) -> Option<PathBuf> {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    let current = std::env::current_exe().ok()?;
    let dir = current.parent()?;
    let sibling = dir.join(&file);
    if sibling.is_file() {
        return Some(sibling);
    }
    if dir.file_name().and_then(|n| n.to_str()) == Some("deps") {
        let up = dir.parent()?.join(&file);
        if up.is_file() {
            return Some(up);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_sources::{FieldType, canonical_snapshot_bytes};

    fn field(name: &str, field_type: FieldType) -> SchemaField {
        SchemaField {
            name: name.to_owned(),
            field_type,
            nullable: true,
            declared_unit: None,
        }
    }

    fn fixture() -> (Vec<SchemaField>, Vec<Vec<CellValue>>) {
        let fields = vec![
            field("id", FieldType::Integer),
            field("sex", FieldType::Text),
            field("ldl", FieldType::Float),
        ];
        let rows = vec![
            vec![
                CellValue::Integer(1),
                CellValue::Text("f".into()),
                CellValue::Float(3.5),
            ],
            vec![
                CellValue::Integer(2),
                CellValue::Text("m".into()),
                CellValue::Null,
            ],
            vec![
                CellValue::Integer(3),
                CellValue::Text("f".into()),
                CellValue::Float(2.5),
            ],
        ];
        (fields, rows)
    }

    fn doc() -> SnapshotCanonicalDoc {
        let (fields, rows) = fixture();
        serde_json::from_slice(&canonical_snapshot_bytes(&fields, &rows)).unwrap()
    }

    fn request(params: ComputeParams) -> WorkerRequest {
        let (fields, rows) = fixture();
        let bytes = canonical_snapshot_bytes(&fields, &rows);
        WorkerRequest {
            protocol_version: COMPUTE_PROTOCOL_VERSION,
            job_id: OpaqueId::new("compute-job-1"),
            manifest_digest: DigestSha256::of(b"m"),
            params,
            sandbox: SandboxRequirement::ReadyBaseRequired,
            max_output_rows: 100,
            input_digest: DigestSha256::of(&bytes),
            input_json: String::from_utf8(bytes).unwrap(),
        }
    }

    fn applied() -> SandboxReport {
        SandboxReport {
            mechanism: SandboxMechanism::WindowsJobObject,
            ready_base_applied: true,
            platform_qualified: false,
            env_var_count: 0,
        }
    }

    #[test]
    fn vocabularies_round_trip_and_are_closed() {
        for s in ComputeState::ALL {
            assert_eq!(ComputeState::parse(s.as_str()).unwrap(), *s);
        }
        assert_eq!(ComputeState::ALL.len(), 11);
        for f in ComputeFailure::ALL {
            assert!(f.state().is_terminal());
            assert_ne!(f.state(), ComputeState::Completed);
            assert_ne!(f.state(), ComputeState::Denied);
        }
        assert!(ComputeJobKind::parse("shell").is_err());
        assert!(serde_json::from_str::<ComputeParams>(r#"{"kind":"python"}"#).is_err());
        assert!(
            serde_json::from_str::<ComputeParams>(r#"{"kind":"column_profile","code":"x"}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<NetworkPosture>(r#""allowed""#).is_err());
        assert_eq!(
            serde_json::from_str::<ComputeParams>(r#"{"kind":"column_profile"}"#).unwrap(),
            ComputeParams::ColumnProfile {}
        );
        assert_eq!(
            serde_json::to_string(&ComputeParams::ColumnProfile {}).unwrap(),
            r#"{"kind":"column_profile"}"#
        );
    }

    #[test]
    fn column_profile_matches_hand_computed_values() {
        let table = execute(&ComputeParams::ColumnProfile {}, &doc(), 100).unwrap();
        assert_eq!(table.rows.len(), 3);
        assert_eq!(
            table.rows[0],
            vec![
                CellValue::Text("id".into()),
                CellValue::Text("integer".into()),
                CellValue::Integer(3),
                CellValue::Integer(0),
                CellValue::Integer(3),
                CellValue::Integer(1),
                CellValue::Integer(3),
                CellValue::Float(2.0),
            ]
        );
        assert_eq!(table.rows[1][4], CellValue::Integer(2));
        assert_eq!(table.rows[1][7], CellValue::Null);
        assert_eq!(table.rows[2][2], CellValue::Integer(2));
        assert_eq!(table.rows[2][3], CellValue::Integer(1));
        assert_eq!(table.rows[2][5], CellValue::Float(2.5));
        assert_eq!(table.rows[2][7], CellValue::Float(3.0));
        let (fields, _) = fixture();
        validate_output(&ComputeParams::ColumnProfile {}, &fields, 3, &table).unwrap();
    }

    #[test]
    fn sorted_projection_is_stable_and_bounded() {
        let params = ComputeParams::SortedProjection {
            columns: vec!["id".into(), "sex".into()],
            sort_by: vec!["sex".into()],
            descending: false,
        };
        let table = execute(&params, &doc(), 100).unwrap();
        let ids: Vec<_> = table.rows.iter().map(|r| r[0].clone()).collect();
        assert_eq!(
            ids,
            vec![
                CellValue::Integer(1),
                CellValue::Integer(3),
                CellValue::Integer(2)
            ]
        );
        let desc = ComputeParams::SortedProjection {
            columns: vec!["id".into()],
            sort_by: vec!["ldl".into()],
            descending: true,
        };
        let table = execute(&desc, &doc(), 100).unwrap();
        let ids: Vec<_> = table.rows.iter().map(|r| r[0].clone()).collect();
        // Null sorts lowest, so it is last when descending.
        assert_eq!(
            ids,
            vec![
                CellValue::Integer(1),
                CellValue::Integer(3),
                CellValue::Integer(2)
            ]
        );
        assert_eq!(
            execute(&params, &doc(), 2),
            Err(WorkerRefusal::RowLimitExceeded)
        );
        let unknown = ComputeParams::SortedProjection {
            columns: vec!["nope".into()],
            sort_by: vec![],
            descending: false,
        };
        assert_eq!(
            execute(&unknown, &doc(), 100),
            Err(WorkerRefusal::UnknownColumn)
        );
        let dup = ComputeParams::SortedProjection {
            columns: vec!["id".into(), "id".into()],
            sort_by: vec![],
            descending: false,
        };
        assert_eq!(execute(&dup, &doc(), 100), Err(WorkerRefusal::BadParams));
    }

    #[test]
    fn worker_responses_are_deterministic_and_checked() {
        let req = request(ComputeParams::ColumnProfile {});
        let a = respond(&req, applied());
        let b = respond(&req, applied());
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&b).unwrap()
        );
        let WorkerOutcome::Completed {
            output_digest,
            output_json,
        } = &a.outcome
        else {
            panic!("expected completion");
        };
        assert_eq!(&DigestSha256::of(output_json.as_bytes()), output_digest);

        let mut bad = req.clone();
        bad.input_json.push(' ');
        assert_eq!(
            respond(&bad, applied()).outcome,
            WorkerOutcome::Refused {
                refusal: WorkerRefusal::InputDigestMismatch
            }
        );
        let none = SandboxReport {
            mechanism: SandboxMechanism::None,
            ready_base_applied: false,
            platform_qualified: false,
            env_var_count: 0,
        };
        assert_eq!(
            respond(&req, none.clone()).outcome,
            WorkerOutcome::Refused {
                refusal: WorkerRefusal::SandboxUnavailable
            }
        );
        let mut relaxed = req.clone();
        relaxed.sandbox = SandboxRequirement::ProcessIsolationOnly;
        assert!(matches!(
            respond(&relaxed, none).outcome,
            WorkerOutcome::Completed { .. }
        ));
        let mut old = req;
        old.protocol_version = 0;
        assert_eq!(
            respond(&old, applied()).outcome,
            WorkerOutcome::Refused {
                refusal: WorkerRefusal::ProtocolMismatch
            }
        );
    }

    #[test]
    fn output_validation_rejects_wrong_shapes() {
        let (fields, _) = fixture();
        let params = ComputeParams::SortedProjection {
            columns: vec!["id".into()],
            sort_by: vec![],
            descending: false,
        };
        let good = execute(&params, &doc(), 100).unwrap();
        validate_output(&params, &fields, 3, &good).unwrap();
        let mut dropped = good.clone();
        dropped.rows.pop();
        assert!(validate_output(&params, &fields, 3, &dropped).is_err());
        let mut renamed = good.clone();
        renamed.columns[0].name = "sex".into();
        assert!(validate_output(&params, &fields, 3, &renamed).is_err());
        let mut ragged = good;
        ragged.rows[0].push(CellValue::Null);
        assert!(validate_output(&params, &fields, 3, &ragged).is_err());
        let mut profile = execute(&ComputeParams::ColumnProfile {}, &doc(), 100).unwrap();
        profile.rows[0][3] = CellValue::Integer(5);
        assert!(validate_output(&ComputeParams::ColumnProfile {}, &fields, 3, &profile).is_err());
    }

    #[test]
    fn receipts_and_reports_hold_their_invariants() {
        let report = SandboxReport {
            mechanism: SandboxMechanism::None,
            ready_base_applied: true,
            platform_qualified: false,
            env_var_count: 0,
        };
        assert!(report.validate().is_err());
        let mut qualified = applied();
        qualified.platform_qualified = true;
        assert!(qualified.validate().is_err());
        let limits = ResourceCeilings {
            timeout_ms: 1,
            ..ResourceCeilings::default()
        };
        assert!(limits.validate().is_err());
        assert!(ResourceCeilings::default().validate().is_ok());
    }
}

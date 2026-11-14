//! MedScale Compute Core authority paths (Spec 085).
//!
//! Core owns authorization and job creation: a submission is checked
//! against the Project and the Spec 075 snapshot authority, and its exact
//! input is pinned by the snapshot's content digest. A run claims the job
//! once, stages the exact stored snapshot bytes (re-verified), and hands
//! them to the MedScale-owned worker over a pipe. The worker's answer is a
//! candidate: Core checks its binding, digest, canonical encoding and shape
//! before committing it, with the receipt and terminal state, in one
//! transaction. The worker never touches the vault. Nothing here writes to
//! a source snapshot.

use std::time::Duration;

use medscale_contracts::analytics::ResultTableDoc;
use medscale_contracts::compute::{
    COMPUTE_PROTOCOL_VERSION, COMPUTE_SCHEMA_VERSION, ComputeDenyReason, ComputeFailure,
    ComputeJob, ComputeJobKind, ComputeJobRequest, ComputeJobView, ComputeManifest, ComputeOutput,
    ComputeReceipt, ComputeState, ComputeStatus, ExecutionPolicy, OutputReview, RuntimeIdentity,
    SandboxMechanism, SandboxReport, SandboxRequirement, StagedInput, WorkerOutcome, WorkerRequest,
    WorkerResponse, validate_output,
};
use medscale_contracts::data_sources::SchemaField;
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{DigestSha256, ObjectHeader, OpaqueId};
use medscale_storage::{MetaError, SnapshotRecord};

use super::data_sources::DataSources;
use crate::compute_supervisor::{
    CancelHandle, ComputeRuntime, LaunchError, ProcessEnd, SupervisionLimits, supervise,
};

fn meta_err(err: MetaError) -> AuthorityError {
    match err {
        MetaError::NotFound => AuthorityError::NotFound,
        MetaError::Conflict(message) => AuthorityError::Conflict { message },
        MetaError::UnsupportedSchema(message) => AuthorityError::UnsupportedSchema { message },
        MetaError::CorruptObjectBody(message) => AuthorityError::Corrupt { message },
        other => AuthorityError::Internal {
            message: other.to_string(),
        },
    }
}

/// How a run ended, before it is written down.
struct Ending {
    state: ComputeState,
    deny_reason: Option<ComputeDenyReason>,
    failure: Option<ComputeFailure>,
    sandbox: Option<SandboxReport>,
    stdout_bytes: u64,
    stderr_bytes: u64,
    stderr_prefix_digest: Option<DigestSha256>,
    table: Option<ResultTableDoc>,
}

impl Ending {
    const fn failed(failure: ComputeFailure) -> Self {
        Self {
            state: failure.state(),
            deny_reason: None,
            failure: Some(failure),
            sandbox: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            stderr_prefix_digest: None,
            table: None,
        }
    }

    const fn denied(reason: ComputeDenyReason) -> Self {
        Self {
            state: ComputeState::Denied,
            deny_reason: Some(reason),
            failure: None,
            sandbox: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            stderr_prefix_digest: None,
            table: None,
        }
    }
}

/// Core's reading of a worker's stdout after a clean exit.
fn judge_response(
    job: &ComputeJob,
    fields: &[SchemaField],
    stdout: &[u8],
) -> (
    Result<ResultTableDoc, ComputeFailure>,
    Option<SandboxReport>,
) {
    let Ok(response) = serde_json::from_slice::<WorkerResponse>(stdout) else {
        return (Err(ComputeFailure::MalformedProtocol), None);
    };
    if response.protocol_version != COMPUTE_PROTOCOL_VERSION
        || response.job_id != job.header.id
        || response.manifest_digest != job.manifest_digest
    {
        return (Err(ComputeFailure::JobMismatch), None);
    }
    if response.sandbox.validate().is_err() {
        return (Err(ComputeFailure::MalformedProtocol), None);
    }
    let sandbox = Some(response.sandbox.clone());
    if response.sandbox.env_var_count != 0 {
        return (Err(ComputeFailure::EnvironmentNotCleared), sandbox);
    }
    let manifest = &job.manifest;
    if manifest.policy.sandbox == SandboxRequirement::ReadyBaseRequired
        && !response.sandbox.ready_base_applied
    {
        return (Err(ComputeFailure::SandboxUnavailable), sandbox);
    }
    let (output_digest, output_json) = match response.outcome {
        WorkerOutcome::Refused { refusal } => return (Err(refusal.failure()), sandbox),
        WorkerOutcome::Completed {
            output_digest,
            output_json,
        } => (output_digest, output_json),
    };
    if output_json.len() as u64 > manifest.limits.max_output_bytes {
        return (Err(ComputeFailure::OutputTooLarge), sandbox);
    }
    if DigestSha256::of(output_json.as_bytes()) != output_digest {
        return (Err(ComputeFailure::OutputDigestMismatch), sandbox);
    }
    let Ok(table) = serde_json::from_str::<ResultTableDoc>(&output_json) else {
        return (Err(ComputeFailure::OutputInvalid), sandbox);
    };
    if table.canonical_bytes() != output_json.as_bytes() {
        return (Err(ComputeFailure::OutputInvalid), sandbox);
    }
    if table.rows.len() as u64 > manifest.limits.max_output_rows {
        return (Err(ComputeFailure::RowLimitExceeded), sandbox);
    }
    let input_rows = manifest.input.as_ref().map_or(0, |i| i.row_count);
    if validate_output(&manifest.params, fields, input_rows, &table).is_err() {
        return (Err(ComputeFailure::OutputInvalid), sandbox);
    }
    (Ok(table), sandbox)
}

impl DataSources<'_> {
    fn compute_header(&self, id: OpaqueId) -> ObjectHeader {
        ObjectHeader {
            id,
            schema_version: COMPUTE_SCHEMA_VERSION,
            realm_id: self.realm.clone(),
            authority_scope_id: self.scope.clone(),
        }
    }

    /// A job of this realm and scope.
    fn scoped_compute_job(&self, job_id: &OpaqueId) -> Result<ComputeJob, AuthorityError> {
        let job = self.meta().get_compute_job(job_id).map_err(meta_err)?;
        if job.header.realm_id != self.realm || job.header.authority_scope_id != self.scope {
            return Err(AuthorityError::WrongScope);
        }
        Ok(job)
    }

    /// The Project's snapshot, or `None` when it is missing, out of scope
    /// or another Project's (all reported alike).
    fn compute_snapshot(
        &self,
        project_id: &OpaqueId,
        snapshot_id: &OpaqueId,
    ) -> Option<SnapshotRecord> {
        self.scoped_snapshot(snapshot_id)
            .ok()
            .filter(|r| &r.project_id == project_id)
    }

    fn compute_receipt_for(
        &self,
        job: &ComputeJob,
        ending: &Ending,
        output_id: Option<&OpaqueId>,
    ) -> Result<ComputeReceipt, AuthorityError> {
        let receipt_id = self
            .meta()
            .alloc_compute_id("compute-receipt")
            .map_err(meta_err)?;
        let (row_count, column_count) = ending.table.as_ref().map_or((0, 0), |t| {
            (
                t.rows.len() as u64,
                u32::try_from(t.columns.len()).unwrap_or(u32::MAX),
            )
        });
        let receipt = ComputeReceipt {
            header: self.compute_header(receipt_id),
            job_id: job.header.id.clone(),
            project_id: job.manifest.project_id.clone(),
            kind: job.manifest.kind,
            manifest_digest: job.manifest_digest.clone(),
            input: job.manifest.input.clone(),
            runtime: job.manifest.runtime.clone(),
            sandbox: ending.sandbox.clone(),
            state: ending.state,
            deny_reason: ending.deny_reason,
            failure: ending.failure,
            output_id: output_id.cloned(),
            output_digest: ending.table.as_ref().map(ResultTableDoc::digest),
            row_count,
            column_count,
            stdout_bytes: ending.stdout_bytes,
            stderr_bytes: ending.stderr_bytes,
            stderr_prefix_digest: ending.stderr_prefix_digest.clone(),
        };
        receipt.validate().map_err(|e| AuthorityError::Internal {
            message: format!("compute receipt invariant: {e}"),
        })?;
        Ok(receipt)
    }

    /// Writes a job's terminal state from `from`, with its receipt and
    /// (when completed) output, and audits it.
    fn compute_finish(
        &mut self,
        job: &ComputeJob,
        from: ComputeState,
        ending: Ending,
        action: &str,
    ) -> Result<ComputeJobView, AuthorityError> {
        let output_id = match &ending.table {
            Some(_) => Some(
                self.meta()
                    .alloc_compute_id("compute-output")
                    .map_err(meta_err)?,
            ),
            None => None,
        };
        let receipt = self.compute_receipt_for(job, &ending, output_id.as_ref())?;
        let output = match (&output_id, &ending.table, &job.manifest.input) {
            (Some(id), Some(table), Some(input)) => Some(ComputeOutput {
                header: self.compute_header(id.clone()),
                project_id: job.manifest.project_id.clone(),
                job_id: job.header.id.clone(),
                receipt_id: receipt.header.id.clone(),
                kind: job.manifest.kind,
                content_digest: table.digest(),
                row_count: receipt.row_count,
                column_count: receipt.column_count,
                derived_from: input.snapshot_id.clone(),
                input_digest: input.content_digest.clone(),
                review: OutputReview::Unreviewed,
            }),
            _ => None,
        };
        let next = self
            .meta()
            .finish_compute_job(
                &job.header.id,
                from,
                &receipt,
                output.as_ref().zip(ending.table.as_ref()),
            )
            .map_err(meta_err)?;
        let mut targets = vec![job.header.id.clone(), receipt.header.id.clone()];
        if let Some(o) = &output {
            targets.push(o.header.id.clone());
        }
        self.audit(action, targets)?;
        Ok(ComputeJobView {
            job: next,
            receipt: Some(receipt),
            output,
            table: ending.table,
        })
    }

    /// Admits one job. Every submission that reaches a Project leaves a
    /// job; a refused one is terminal at once, with its receipt.
    pub fn compute_submit(
        &mut self,
        request: ComputeJobRequest,
    ) -> Result<ComputeJobView, AuthorityError> {
        self.require_project(&request.project_id)?;
        let job_id = self
            .meta()
            .alloc_compute_id("compute-job")
            .map_err(meta_err)?;
        let kind = request.params.kind();
        let limits = request.limits.clone().unwrap_or_default();
        let sandbox = request
            .sandbox
            .unwrap_or(SandboxRequirement::ReadyBaseRequired);
        let mut ending: Option<Ending> = None;
        let mut input: Option<StagedInput> = None;
        if request.params.validate().is_err() {
            ending = Some(Ending::denied(ComputeDenyReason::BadParams));
        } else if limits.validate().is_err() {
            ending = Some(Ending::denied(ComputeDenyReason::BadLimits));
        } else {
            match self.compute_snapshot(&request.project_id, &request.snapshot_id) {
                None => ending = Some(Ending::denied(ComputeDenyReason::InputUnavailable)),
                Some(record) => match self.snapshot_bytes(&record) {
                    Err(_) => ending = Some(Ending::failed(ComputeFailure::InputDigestMismatch)),
                    Ok(bytes) => {
                        let staged = StagedInput {
                            snapshot_id: record.snapshot.header.id.clone(),
                            content_digest: record.snapshot.content_digest.clone(),
                            schema_fingerprint: record.snapshot.schema_fingerprint.clone(),
                            row_count: record.snapshot.row_count,
                            byte_len: bytes.len() as u64,
                        };
                        if DigestSha256::of(&bytes) != staged.content_digest {
                            ending = Some(Ending::failed(ComputeFailure::InputDigestMismatch));
                        } else if !request.params.columns_exist(&record.schema.fields) {
                            ending = Some(Ending::denied(ComputeDenyReason::UnknownColumn));
                        } else if staged.byte_len > limits.max_input_bytes {
                            ending = Some(Ending::denied(ComputeDenyReason::InputTooLarge));
                        }
                        input = Some(staged);
                    }
                },
            }
        }
        // A corrupt input at admission is recorded without pinning it.
        if ending
            .as_ref()
            .is_some_and(|e| e.state == ComputeState::Corrupt)
        {
            input = None;
        }
        let manifest = ComputeManifest {
            job_id: job_id.clone(),
            project_id: request.project_id.clone(),
            kind,
            params: request.params,
            requested_snapshot_id: request.snapshot_id,
            input,
            runtime: RuntimeIdentity::current(kind),
            policy: ExecutionPolicy::fixed(sandbox),
            limits,
        };
        let manifest_digest = manifest.digest();
        let mut job = ComputeJob {
            header: self.compute_header(job_id.clone()),
            manifest,
            manifest_digest,
            state: ComputeState::Queued,
        };
        let receipt = match &ending {
            Some(e) => {
                job.state = e.state;
                Some(self.compute_receipt_for(&job, e, None)?)
            }
            None => None,
        };
        self.meta()
            .insert_compute_job(&job, receipt.as_ref())
            .map_err(meta_err)?;
        let mut targets = vec![job_id];
        if let Some(r) = &receipt {
            targets.push(r.header.id.clone());
        }
        self.audit("compute.submit", targets)?;
        Ok(ComputeJobView {
            job,
            receipt,
            output: None,
            table: None,
        })
    }

    /// Runs one queued job to a terminal state. A job runs at most once:
    /// any other state is refused before anything executes. Jobs left
    /// `running` by an earlier crash are recovered first.
    pub fn compute_run(
        &mut self,
        job_id: &OpaqueId,
        runtime: &ComputeRuntime,
        cancel: &CancelHandle,
    ) -> Result<ComputeJobView, AuthorityError> {
        let job = self.scoped_compute_job(job_id)?;
        if job.state != ComputeState::Queued {
            return Err(AuthorityError::Conflict {
                message: format!(
                    "compute job is {}; a job runs at most once",
                    job.state.as_str()
                ),
            });
        }
        self.compute_recover()?;
        let job = self.meta().claim_compute_job(job_id).map_err(meta_err)?;
        let ending = self.compute_execute(&job, runtime, cancel);
        self.compute_finish(&job, ComputeState::Running, ending, "compute.run")
    }

    fn compute_execute(
        &self,
        job: &ComputeJob,
        runtime: &ComputeRuntime,
        cancel: &CancelHandle,
    ) -> Ending {
        let manifest = &job.manifest;
        let Some(pinned) = &manifest.input else {
            return Ending::denied(ComputeDenyReason::InputUnavailable);
        };
        // Stale reference: the snapshot must still be this Project's.
        let Some(record) = self.compute_snapshot(&manifest.project_id, &pinned.snapshot_id) else {
            return Ending::denied(ComputeDenyReason::InputUnavailable);
        };
        // Input replacement: the stored bytes must still be the pinned ones.
        let bytes = match self.snapshot_bytes(&record) {
            Ok(bytes) => bytes,
            Err(_) => return Ending::failed(ComputeFailure::InputDigestMismatch),
        };
        if record.snapshot.content_digest != pinned.content_digest
            || record.snapshot.schema_fingerprint != pinned.schema_fingerprint
            || DigestSha256::of(&bytes) != pinned.content_digest
            || bytes.len() as u64 != pinned.byte_len
        {
            return Ending::failed(ComputeFailure::InputDigestMismatch);
        }
        let Ok(input_json) = String::from_utf8(bytes) else {
            return Ending::failed(ComputeFailure::InputDigestMismatch);
        };
        if cancel.is_cancelled() {
            return Ending::failed(ComputeFailure::CancelledByUser);
        }
        let request = WorkerRequest {
            protocol_version: COMPUTE_PROTOCOL_VERSION,
            job_id: job.header.id.clone(),
            manifest_digest: job.manifest_digest.clone(),
            params: manifest.params.clone(),
            sandbox: manifest.policy.sandbox,
            max_output_rows: manifest.limits.max_output_rows,
            input_digest: pinned.content_digest.clone(),
            input_json,
        };
        let Ok(request_bytes) = serde_json::to_vec(&request) else {
            return Ending::failed(ComputeFailure::SpawnFailed);
        };
        drop(request);
        let limits = SupervisionLimits {
            stdout_bytes: manifest.limits.stdout_ceiling(),
            stderr_bytes: manifest.limits.max_stderr_bytes,
            timeout: Duration::from_millis(manifest.limits.timeout_ms),
        };
        let observed = match supervise(runtime, request_bytes, limits, cancel) {
            Err(LaunchError::WorkerMissing) => {
                return Ending::failed(ComputeFailure::WorkerMissing);
            }
            Err(LaunchError::SpawnFailed) => return Ending::failed(ComputeFailure::SpawnFailed),
            Ok(observed) => observed,
        };
        let mut ending = match observed.end {
            ProcessEnd::Cancelled => Ending::failed(ComputeFailure::CancelledByUser),
            ProcessEnd::TimedOut => Ending::failed(ComputeFailure::TimeLimit),
            ProcessEnd::StdoutOverflow => Ending::failed(ComputeFailure::OutputTooLarge),
            ProcessEnd::Exited { success: false } => Ending::failed(ComputeFailure::WorkerCrashed),
            ProcessEnd::Exited { success: true } => {
                match judge_response(job, &record.schema.fields, &observed.stdout) {
                    (Ok(table), sandbox) => Ending {
                        state: ComputeState::Completed,
                        deny_reason: None,
                        failure: None,
                        sandbox,
                        stdout_bytes: 0,
                        stderr_bytes: 0,
                        stderr_prefix_digest: None,
                        table: Some(table),
                    },
                    (Err(failure), sandbox) => Ending {
                        sandbox,
                        ..Ending::failed(failure)
                    },
                }
            }
        };
        ending.stdout_bytes = observed.stdout_bytes;
        ending.stderr_bytes = observed.stderr_bytes;
        ending.stderr_prefix_digest = observed.stderr_prefix_digest;
        ending
    }

    /// Cancels a queued job. A running job is cancelled through the
    /// `CancelHandle` of the call that runs it.
    pub fn compute_cancel(&mut self, job_id: &OpaqueId) -> Result<ComputeJobView, AuthorityError> {
        let job = self.scoped_compute_job(job_id)?;
        if job.state != ComputeState::Queued {
            return Err(AuthorityError::Conflict {
                message: format!(
                    "compute job is {}; only a queued job cancels directly",
                    job.state.as_str()
                ),
            });
        }
        self.compute_finish(
            &job,
            ComputeState::Queued,
            Ending::failed(ComputeFailure::CancelledByUser),
            "compute.cancel",
        )
    }

    /// Marks every job of this realm and scope left `running` as
    /// `interrupted`. Runs are synchronous under the single-writer lease,
    /// so a `running` job seen here was orphaned by a crash or restart. It
    /// is never re-executed automatically.
    pub fn compute_recover(&mut self) -> Result<Vec<ComputeJobView>, AuthorityError> {
        let running = self
            .meta()
            .list_compute_jobs_in_state(ComputeState::Running)
            .map_err(meta_err)?;
        let mut out = Vec::new();
        for job in running {
            if job.header.realm_id != self.realm || job.header.authority_scope_id != self.scope {
                continue;
            }
            out.push(self.compute_finish(
                &job,
                ComputeState::Running,
                Ending::failed(ComputeFailure::InterruptedByRestart),
                "compute.recover",
            )?);
        }
        Ok(out)
    }

    /// A job with its receipt and, when completed, its output table.
    pub fn compute_job(&self, job_id: &OpaqueId) -> Result<ComputeJobView, AuthorityError> {
        let job = self.scoped_compute_job(job_id)?;
        let receipt = self
            .meta()
            .get_compute_receipt_for_job(job_id)
            .map_err(meta_err)?;
        let (output, table) = match receipt.as_ref().and_then(|r| r.output_id.as_ref()) {
            Some(output_id) => {
                let (output, table) = self
                    .meta()
                    .get_compute_output(output_id)
                    .map_err(meta_err)?;
                (Some(output), Some(table))
            }
            None => (None, None),
        };
        Ok(ComputeJobView {
            job,
            receipt,
            output,
            table,
        })
    }

    pub fn compute_jobs(&self, project_id: &OpaqueId) -> Result<Vec<ComputeJob>, AuthorityError> {
        self.require_project(project_id)?;
        let jobs = self
            .meta()
            .list_compute_jobs(project_id)
            .map_err(meta_err)?;
        Ok(jobs
            .into_iter()
            .filter(|j| {
                j.header.realm_id == self.realm && j.header.authority_scope_id == self.scope
            })
            .collect())
    }

    pub fn compute_status(
        &self,
        runtime: &ComputeRuntime,
    ) -> Result<ComputeStatus, AuthorityError> {
        let count = |state| {
            self.meta()
                .list_compute_jobs_in_state(state)
                .map(|jobs| {
                    jobs.iter()
                        .filter(|j| {
                            j.header.realm_id == self.realm
                                && j.header.authority_scope_id == self.scope
                        })
                        .count() as u64
                })
                .map_err(meta_err)
        };
        Ok(ComputeStatus {
            worker_found: runtime.worker_found(),
            protocol_version: COMPUTE_PROTOCOL_VERSION,
            expected_mechanism: SandboxMechanism::for_this_os(),
            platform_qualified: false,
            kinds: ComputeJobKind::ALL.to_vec(),
            queued: count(ComputeState::Queued)?,
            running: count(ComputeState::Running)?,
        })
    }
}

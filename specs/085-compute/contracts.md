# Contracts — Spec 085 MedScale Compute

`crates/medscale-contracts/src/compute.rs` (schema version 1, protocol 1):

| Contract | Meaning |
|---|---|
| `ComputeJobKind` | closed: `column_profile`, `sorted_projection` (each `kind_version` 1) |
| `ComputeParams` | tagged by kind; `sorted_projection` names 1-64 columns and 0-8 sort keys (no duplicates) and a direction |
| `ComputeJobRequest` | Project, snapshot, params, optional `SandboxRequirement`, optional `ResourceCeilings` |
| `StagedInput` | snapshot id, content digest, schema fingerprint, row count, byte length of the exact stored bytes |
| `RuntimeIdentity` | worker `medscale-compute-worker`, worker version, protocol version, kind version |
| `ExecutionPolicy` | fixed postures `network=denied`, `code=closed_kinds_only`, `environment=cleared`, `handles=stdio_pipes_only`, `filesystem=empty_scratch_only`; `sandbox` = `ready_base_required` (default) or `process_isolation_only` |
| `ResourceCeilings` | input bytes, output bytes, output rows, stderr bytes, timeout (all bounded by hard maxima) |
| `ComputeManifest`, `ComputeJob` | the immutable manifest (including the requested snapshot and the pinned input, absent only when it could not be read) and its SHA-256 digest, plus the job state |
| `ComputeState` | `queued`, `running`, `completed`, `cancelled`, `timed_out`, `denied`, `resource_exhausted`, `failed`, `unavailable`, `corrupt`, `interrupted` |
| `ComputeDenyReason` | `input_unavailable`, `unknown_column`, `bad_params`, `bad_limits`, `input_too_large` |
| `ComputeFailure` | fixed codes, each owned by exactly one state (below) |
| `SandboxReport` | mechanism applied, `ready_base_applied`, `platform_qualified` (always false), environment variable count |
| `ComputeReceipt` | one per terminal job: manifest digest, input, runtime, sandbox report, state, reason or failure, output id and digest, stdout/stderr byte counts, digest of the bounded stderr prefix |
| `ComputeOutput` | committed derived output: digest, counts, `derived_from` snapshot, input digest, `review=unreviewed` |
| `WorkerRequest`, `WorkerResponse`, `WorkerOutcome`, `WorkerRefusal` | the stdin/stdout protocol |

Failure to state (never collapsed):

| State | Failures |
|---|---|
| `unavailable` | `worker_missing`, `sandbox_unavailable`, `environment_not_cleared` |
| `failed` | `spawn_failed`, `worker_crashed`, `worker_refused` |
| `corrupt` | `malformed_protocol`, `job_mismatch`, `output_digest_mismatch`, `output_invalid`, `input_digest_mismatch` |
| `resource_exhausted` | `output_too_large`, `row_limit_exceeded` |
| `timed_out` | `time_limit` |
| `cancelled` | `cancelled_by_user` |
| `interrupted` | `interrupted_by_restart` |

`denied` carries a deny reason and no failure; `completed` carries
neither and always names its output, input and sandbox report.

Worker protocol: stdin carries one `WorkerRequest` with the exact staged
bytes as `input_json` and their digest; stdout carries one
`WorkerResponse` echoing job id, manifest digest and protocol version,
with the sandbox report and either the canonical output bytes plus their
digest, or a refusal. Stderr is not part of the protocol.

Capabilities (`envelopes/mod.rs`): `ComputeSubmit` (submit, cancel),
`ComputeRun` (run, recover), `ComputeRead` (job, jobs, status).

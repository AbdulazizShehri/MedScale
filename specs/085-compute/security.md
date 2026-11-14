# Security — Spec 085 MedScale Compute

The worker runs only MedScale's own code for two closed job kinds. Its OS
mechanism is `ReadyBaseMeasured` defense in depth, not a sandbox for
untrusted code, and `platform_qualified` stays false.

| # | Threat | Control | Proof |
|---|---|---|---|
| C1 | Arbitrary code | requests carry only typed params of closed kinds; no field names code, a script, a shell, an interpreter, a plugin or a worker path; unknown kinds and fields fail to parse | contract `vocabularies_round_trip_and_are_closed`; CLI `params_parse_strictly` |
| C2 | Path traversal or symlinks in inputs | inputs are named by snapshot id, never by path; Core reads the stored bytes itself and pipes them; the worker opens no file | Core `admission_refuses_with_receipts_and_runs_nothing` (path-shaped column refused as unknown) |
| C3 | Cross-Project or stale input | admission and every run re-check that the snapshot is this Project's and in scope; other Projects' snapshots are reported like missing ones | Core admission test; `inputs_are_re_verified_before_staging` (stale reference) |
| C4 | Input replacement or digest mismatch | stored bytes are re-read and re-digested against the pinned digest before staging; the worker re-checks the digest before decoding | Core `inputs_are_re_verified_before_staging`; contract `worker_responses_are_deterministic_and_checked` |
| C5 | Oversized input | input byte ceiling at admission (`input_too_large`); request read bound in the worker | Core admission test |
| C6 | Output explosion | stdout is read on a bounded reader; overflow kills the worker (`output_too_large`); embedded output and row ceilings are re-checked | Core `faulty_workers_are_contained_and_commit_nothing` (`flood-stdout`) |
| C7 | Untrusted stderr | stderr is drained on a bounded reader; only its byte count and the digest of its prefix are recorded, never its text | Core fault test (`flood-stderr`) |
| C8 | Timeout escape | Core polls the child and kills it at the deadline (`timed_out`) | Core fault test (`hang`) |
| C9 | Cancellation race | a run installs a fresh handle; cancel kills the worker and ends `cancelled`; a cancel before a run cannot affect a later run; queued jobs cancel directly | Core `a_running_job_cancels_through_its_handle`, `a_job_runs_at_most_once_and_cancelled_jobs_never_run` |
| C10 | Duplicate or replayed execution | a conditional `queued -> running` claim; any other state is refused before anything executes | Core at-most-once test; storage `compute_rows_hold_their_invariants` |
| C11 | Malformed, partial, mismatched or forged output | strict protocol parse (unknown fields and trailing bytes refused); job id, manifest digest and protocol echoed; output digest over the exact bytes; bytes must be canonical; shape checked per kind | Core fault test (`partial`, `malformed`, `trailing`, `wrong-job`, `wrong-digest`, `noncanonical`, `bad-shape`) |
| C12 | Receipt/output mismatch, partial output after a crash | state, receipt and output are one transaction; restore re-verifies every cross-row invariant | storage invariants and tamper tests |
| C13 | Worker crash, restart | a non-zero exit is `failed`; a job left `running` is recovered as `interrupted` and never re-run | Core fault test (`crash`); `jobs_orphaned_by_a_crash_are_interrupted_not_rerun` |
| C14 | Environment leakage | the worker starts with a cleared environment and reports the count it sees; a non-zero count ends `unavailable` | Core fixtures test (count 0); fault test (`env-leak`) |
| C15 | Inherited handles, temp-file escape | stdio pipes only (Rust opens other handles close-on-exec / non-inheritable); the working directory is a new empty directory created by `create_dir` and removed after the run; on Linux it is the only Landlock-allowed path | supervisor design; Linux mechanism below |
| C16 | Network access | the worker has no network code; Linux Landlock denies TCP bind/connect; macOS Seatbelt denies network; Windows has no network axis in this mechanism (recorded residual) | Spec 052/031 measurements; this spec reports the mechanism in every receipt |
| C17 | Fork/process explosion | the worker spawns nothing; Windows Job Object limits it to one active process; not OS-enforced on Linux or macOS (recorded residual) | Spec 030 measurement |
| C18 | Sandbox unavailable | with `ready_base_required` (the default) the worker refuses to compute, and Core independently refuses a completion without an applied mechanism (`unavailable`) | Core fault test (`no-sandbox`, `no-sandbox-completes`) |
| C19 | False qualification claims | a report with `platform_qualified=true` is corrupt | Core fault test (`claims-qualified`); contract invariants |
| C20 | Worker substitution | the worker is found only next to the running executable; no request, environment variable or configuration names it | `resolve_sibling_exe`; a missing worker is `unavailable` |

Recorded residuals: memory is bounded by the input ceiling and the job
design, not an OS quota; the process-count limit is OS-enforced on
Windows only; Windows has no network-deny axis in the applied mechanism;
running jobs cancel only through the in-process handle; the worker is not
yet in the portable release package. Real PHI remains unauthorized; all
fixtures are synthetic.

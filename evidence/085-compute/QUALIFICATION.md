# Qualification — Spec 085 MedScale Compute

Exact-head CI for the final code head is recorded in
`EXACT_HEAD_QUALIFICATION.md`; the counts below are test inventories.

All data is synthetic. Jobs run in a separate worker process on every CI
OS (Linux, Windows, macOS).

## Contracts (`crates/medscale-contracts/src/compute.rs`, 6 tests)

`vocabularies_round_trip_and_are_closed` (eleven states; every failure
owns one terminal state that is neither `completed` nor `denied`; unknown
kinds such as `shell`/`python`, unknown param fields and a non-`denied`
network posture fail to parse),
`column_profile_matches_hand_computed_values`,
`sorted_projection_is_stable_and_bounded` (stable ties, nulls lowest, row
limit, unknown and duplicate columns refused),
`worker_responses_are_deterministic_and_checked` (byte-identical
responses; input digest mismatch, missing sandbox and protocol mismatch
refused), `output_validation_rejects_wrong_shapes`,
`receipts_and_reports_hold_their_invariants` (a `platform_qualified`
claim is invalid; ceilings bounded).

## Storage v14 (`crates/medscale-storage/tests/compute_085.rs`, 5 tests)

`migration_v13_to_v14_is_additive` (idempotent reopen),
`compute_rows_hold_their_invariants` (unique ids; a new job is queued or
terminal with its receipt; single claim; a transition from the wrong
state writes nothing; a finished job cannot finish again; a completed
receipt needs its output with matching bytes; a receipt for another job
is refused; edited state columns and output bytes fail closed on read),
`backup_restore_round_trips_compute_rows` (a `running` job restores as
`running`), `tampered_compute_backups_are_refused` (13 cases: missing or
non-array families, state edits in both directions, manifest edit,
receipt removed or moved, qualification claim, output removed, edited,
non-hex or re-pointed, duplicate job),
`pre_085_v13_backup_restores_with_empty_compute_tables`.

The Spec 078-084 storage tests rewind past v14 as well.

## Core (`crates/medscale-core/tests/compute_085.rs`, 7 tests)

| Requirement | Test |
|---|---|
| Both kinds complete in the worker and equal hand-computed fixtures; output digest, pinned input, `derived_from`, `review=unreviewed`; sandbox mechanism of this OS applied, `platform_qualified=false`, zero environment variables; byte-identical rerun; snapshot unchanged; state survives reopen | `jobs_run_in_the_worker_and_match_hand_computed_fixtures` |
| Missing, other-Project, unknown-column, path-shaped key, duplicate or empty columns, out-of-bounds limits and oversized input are `denied` with receipts and never run; other Projects' inputs are indistinguishable from missing ones | `admission_refuses_with_receipts_and_runs_nothing` |
| A completed or cancelled job cannot run again; a queued job cancels | `a_job_runs_at_most_once_and_cancelled_jobs_never_run` |
| Replaced stored bytes end `corrupt` without a worker; a snapshot moved to another Project ends `denied` (stale reference) | `inputs_are_re_verified_before_staging` |
| Hang, stdout flood, crash, partial/malformed/trailing output, wrong job, wrong digest, non-canonical output, wrong shape, qualification claim, environment leak, missing sandbox (refused and lying) each end in their own state and commit nothing; stderr flood is counted and never stored; process-isolation-only runs report `none`; a missing worker is `unavailable` | `faulty_workers_are_contained_and_commit_nothing` |
| A running job cancels through its handle; the next run is unaffected | `a_running_job_cancels_through_its_handle` |
| A job left `running` is recovered as `interrupted`, never re-run; a run recovers orphans first | `jobs_orphaned_by_a_crash_are_interrupted_not_rerun` |

## CLI (`crates/medscale-cli/src/compute.rs`, 2 tests)

`params_parse_strictly`; `compute_commands_run_through_core_across_fresh_sessions`
(submit, run in a fresh session, second run refused, cancel, admission
refusal, list, recover, show, status; human and JSON rendering).

## Not demonstrated (recorded, non-blocking)

- No OS memory quota; memory is bounded by the input ceiling and job
  design.
- Process-count limit OS-enforced on Windows only; no network-deny axis
  in the Windows mechanism (the worker has no network code).
- Cancelling a running job needs the in-process handle; the CLI runs jobs
  synchronously.
- The worker is not in the portable release package yet; there Compute
  reports `unavailable`.
- No Desktop surface.

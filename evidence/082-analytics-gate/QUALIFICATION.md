# Qualification — Spec 082 Analytics Gate

Exact-head CI for the final code head is recorded in
`EXACT_HEAD_QUALIFICATION.md`; the counts below are test inventories.

All inputs are synthetic: CSV text written by the tests and imported as
Spec 075 snapshots, or rows built in code. Engine choice (founder,
2026-09-23): the already-admitted SQLite, read-only, behind the
engine-neutral `EngineIdentity` contract. DataFusion is not qualified and no
cross-engine equivalence is claimed.

## Contracts (`crates/medscale-contracts/src/analytics.rs`, 6 tests)

`vocabularies_round_trip_and_are_closed`, `aliases_are_plain_identifiers`,
`requests_validate_shape` (SQL length, 1-8 unique bindings, row bound),
`receipts_hold_their_invariants` (SQL digest, exactly denied receipts carry
a reason, exactly failed/timed-out receipts carry a failure, exactly
completed/truncated receipts name a result, cohort origin names a cohort),
`cohorts_bind_values_and_validate` (null values, operator/value agreement),
`statistic_values_never_hide_missing_data`.

## Engine (`crates/medscale-storage/src/analytics_engine.rs`, 9 unit tests)

| Behavior | Test |
|---|---|
| `SELECT`/`WITH` run over the bound table with observed column types | `select_queries_run_and_are_typed` |
| Writes, DDL, `REPLACE`, `ATTACH`, `pragma_*`, extensions, `sqlcipher_export`, keywords in comments, stacked statements, unknown tables, syntax errors, writable CTEs refused with the right reason | `writes_ddl_and_escapes_are_refused` |
| Semicolon and comment stacking, `PRAGMA`/`BEGIN`/`SAVEPOINT`/`EXPLAIN`, `VACUUM INTO`, `DETACH`, `readfile`, `fts3_tokenizer`, writable CTEs; a trailing `;` or comment is still one statement; `sqlite_schema` lists only the bound table | `statement_tricks_are_refused` |
| Row cap gives `truncated`; exact fit is not truncated; BLOB and non-finite results refused; mixed types reported | `results_are_bounded_and_honest` |
| Injection-shaped parameter is data | `parameters_are_bound_not_spliced` |
| Giant values refused before they are built (`zeroblob`, `hex`, `replace` growth); cell bound, NUL, invalid UTF-8; 129 columns refused, 128 accepted | `giant_values_and_wide_results_are_bounded` |
| Without the text screen, the restricted connection still refuses `ATTACH`, `DELETE`, `CREATE TEMP TABLE` and schema writes, and data is unchanged | `engine_connection_refuses_escapes_without_the_text_screen` |
| Recursive CTE and 24-way Cartesian join end at the time limit | `runaway_queries_time_out`, `cartesian_explosions_time_out` |

## Storage v11 (`crates/medscale-storage/tests/analytics_082.rs`, 8 tests)

`migration_v10_to_v11_is_additive` (pre-082 state unchanged; idempotent
reopen), `crash_mid_v11_migration_fails_closed_and_backup_recovers`,
`receipts_and_results_commit_atomically_and_are_digest_checked`,
`cohorts_are_validated_and_listed_per_project`,
`consistency_check_detects_invariant_breaks`,
`backup_restore_roundtrips_every_082_row_exactly`,
`restore_rejects_hand_edited_082_snapshots` (9 cases: result bytes, result
hex not ASCII, result hex with signs, receipt SQL edited, result without
receipt, receipt names another digest, cohort removed, receipt in another
scope, duplicate cohort), `pre_082_v10_backup_restores_with_empty_analytics_tables`.
The Spec 078, 079, 080 and 081 storage suites rewind to their own schema by
dropping the v11 tables and pass with `CURRENT_META_SCHEMA_VERSION = 11`.

## Core (`crates/medscale-core/tests/analytics_082.rs`, 8 tests; 3 unit tests)

| Behavior | Test |
|---|---|
| Query over a bound snapshot pins id, digest, schema fingerprint and row count; result digest matches the receipt; replay `reproduced` | `queries_run_read_only_over_pinned_snapshots_and_replay` |
| Writes, escapes, stacking, unknown tables, syntax errors, bad bindings, missing and other-Project snapshots all leave `denied` receipts with reasons; the snapshot is unchanged; denied receipts are `not_replayable` | `writes_escapes_and_foreign_inputs_are_refused_with_receipts` |
| Row cap gives a `truncated` receipt and table flag, never a full-result claim; the truncated result replays | `truncation_is_explicit_and_never_the_full_result` |
| A later snapshot of the same source does not change a completed run, which still reproduces against its pinned snapshot | `a_refresh_never_mutates_a_completed_run` |
| Cohorts compile to bound parameters and reproduce; an injection-shaped value is data; missing fields, type mismatches, malformed operators and null comparisons refused | `cohorts_bind_values_and_reproduce` |
| Statistics match hand-computed values; `not_numeric`; unknown column refused | `statistics_are_hand_checked_and_honest` |
| Receipts and results identical after reopen; replay still `reproduced` | `analytics_state_survives_reopen` |
| A receipt whose pinned input digest no longer matches reports `input_unavailable` without re-running; a changed recorded result reports `diverged` with the true replay digest | `replay_reports_moved_inputs_and_changed_results` |
| Unit: hand-computed statistics (mean, sample SD, median odd/even, min, max, count, missing); `insufficient` and `not_numeric` never a zero; cohort SQL uses quoted identifiers and positional parameters | `authority::analytics::tests::*` |

## CLI (`crates/medscale-cli/src/analytics.rs`, 2 tests)

`criteria_and_bindings_parse_strictly` (integer, float, boolean, forced
text, text; unknown operator and malformed binding refused) and
`analytics_commands_run_through_core_across_fresh_sessions` (query in
human and JSON form, a denied query, cohort create, receipts, cohort run,
replay and statistics in human and JSON form, across fresh Core sessions
in one test process). The CLI manifest has no
storage or SQLite dependency (`cli_does_not_depend_on_storage_or_rusqlite`).

## Desktop (`crates/medscale-desktop/src/analytics_workspace.rs`, 1 test)

`analytics_view_models_flow_through_a_real_core_session`: run, denied run
and receipt list through `CliSession`; denied runs show their reason and no
preview. Desktop has no storage or SQLite dependency.

## Not demonstrated (recorded, non-blocking)

- No rendered Desktop screenshot (no CI rendering step).
- Performance at scale beyond the snapshot row bound is unmeasured.
- No user-initiated query cancel; the time limit is the only stop.
- No DataFusion qualification; no NL-to-SQL; no charts; no inferential
  statistics (all out of scope by promotion).

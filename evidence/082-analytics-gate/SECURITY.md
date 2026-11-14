# Security Challenge — Spec 082 Analytics Gate

Deterministic challenge of the SQL surface and its storage, run on this
branch under `FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`. No external
or LLM reviewer ran. Each row names the control and the test that proves it,
or states the residual honestly. Test names refer to:
- engine: `crates/medscale-storage/src/analytics_engine.rs` unit tests;
- storage: `crates/medscale-storage/tests/analytics_082.rs`;
- Core: `crates/medscale-core/tests/analytics_082.rs` and the unit tests in
  `crates/medscale-core/src/authority/analytics.rs`;
- CLI: `crates/medscale-cli/src/analytics.rs` tests.

## Findings fixed on this branch (commit `624989b`, `52ef6c4`)

| # | Finding | Fix | Proof |
|---|---|---|---|
| F1 | A query could make SQLite build a giant value (for example `hex(zeroblob(N))` or `replace` growth) in memory before the post-hoc result checks ran. On a memory-constrained host this is a denial of service inside the 10 s window. | The private connection sets `SQLITE_LIMIT_LENGTH` (16 MiB floor, raised to twice the widest bound input row so every loaded row can still be read, sorted and grouped), `SQLITE_LIMIT_ATTACHED = 0` and SQLite defensive mode. `SQLITE_TOOBIG` maps to the fixed failure `a value exceeds the engine bound`. Enables the dependency-free rusqlite `limits` feature (admission 005 amended). | engine `giant_values_and_wide_results_are_bounded`, `engine_connection_refuses_escapes_without_the_text_screen` |
| F2 | `from_hex` sliced backup text by byte index, so a non-ASCII `content_hex` in a tampered backup panicked restore instead of being refused. `from_str_radix` also accepted a signed `+f` pair. | Strict ASCII hex-digit check before decoding. | storage `restore_rejects_hand_edited_082_snapshots` (cases `result hex not ASCII`, `result hex with signs`) |
| F3 | A cohort value whose type did not fit its field (text against an integer column, a number against a text column) was accepted and silently matched nothing. | Core refuses the cohort at creation, naming the field and type. The CLI gained `true`/`false` booleans and a `"..."` forced-text form so every fitting value stays expressible. | Core `cohorts_bind_values_and_reproduce`; CLI `criteria_and_bindings_parse_strictly` |
| F4 | Statistics indexed a stored row by column position without a bounds check. The row is digest-verified on read, so this needed a digest-consistent corrupt write, but it would have panicked. | Refused as `Corrupt`. | code path; no reachable fixture (digest check precedes it) |

The same `from_hex` pattern exists in the closed Spec 080 (`browse.rs`) and
Spec 081 (`audio.rs`) storage modules. It is outside the Spec 082 scope, so
it is recorded here and in the closure as a follow-up fix, not changed in
this PR.

## Challenge matrix

| Challenge | Result | Control / proof |
|---|---|---|
| Multiple statements; semicolon bypass | refused `multiple_statements` | rusqlite prepares the tail and refuses a second statement; engine `writes_ddl_and_escapes_are_refused`, `statement_tricks_are_refused` (including a comment before the semicolon and no whitespace after it). A trailing `;` or comment is one statement and runs. |
| Comments hiding a keyword | refused | leading comments are skipped before the first-word check (`/* SELECT */ DROP` is `not_read_only`); forbidden words are matched anywhere, including in comments and literals (conservative) |
| CTE-based writes / writable CTEs | refused `not_read_only` | `WITH ... DELETE/UPDATE/INSERT` pass the first-word screen but fail `sqlite3_stmt_readonly`; `query_only` is the second barrier; engine tests |
| `PRAGMA` variants, `pragma_*` functions | refused | screen (any case); `query_only` and defensive mode underneath; engine tests |
| `ATTACH` / `DETACH`, URI databases | refused | screen; `SQLITE_LIMIT_ATTACHED = 0` refuses attachment even without the screen; the engine opens only `:memory:`; engine `engine_connection_refuses_escapes_without_the_text_screen` |
| Temporary schema abuse | refused | `CREATE TEMP TABLE` is not read-only and fails under `query_only`; engine connection test |
| `VACUUM` / `VACUUM INTO` | refused | screen; engine tests |
| Extension loading | refused | screen; rusqlite `load_extension` feature is not enabled, so the C API is off |
| Virtual tables, `fts3_tokenizer`, file functions (`readfile`, `writefile`) | refused or absent | screen for the named functions; virtual table creation is DDL (not read-only); built-in table-valued functions such as `json_each` read only their arguments |
| `sqlite_schema` exposure | contained | the private database holds only the bound tables; engine `statement_tricks_are_refused` shows `sqlite_schema` lists only the bound alias |
| Recursive CTE exhaustion | `timed_out` | 10 s interrupt; engine `runaway_queries_time_out` |
| Cartesian explosion | `timed_out` | engine `cartesian_explosions_time_out` (24-way self join) |
| Giant strings / BLOBs | `failed` with fixed text | F1 |
| Excessive columns | `failed` | 128-column bound; engine `giant_values_and_wide_results_are_bounded` (129 refused, 128 accepted) |
| Excessive rows | `truncated`, never a full-result claim | engine `results_are_bounded_and_honest`; Core `truncation_is_explicit_and_never_the_full_result` |
| Oversized text cell, NUL, invalid UTF-8 | `failed` with fixed text | engine `giant_values_and_wide_results_are_bounded` |
| Binary and non-finite results | `failed` | engine `results_are_bounded_and_honest` |
| Timeout enforcement | `timed_out` receipt | engine timeout tests; Core records `timed_out` receipts |
| Cancellation | residual | there is no user-initiated cancel in this foundation; the time limit is the only stop. A cancel control is a later-spec item. |
| Cohort injection (quote, comment, boolean expression) | data, not SQL | values are positional parameters; field names are quoted identifiers; Core unit `cohorts_compile_to_bound_parameters`; Core `cohorts_bind_values_and_reproduce`; engine `parameters_are_bound_not_spliced` |
| Malformed operators, null semantics, missing columns, type mismatch | refused at creation | contracts `cohorts_bind_values_and_validate`; Core `cohorts_bind_values_and_reproduce` (F3) |
| Cross-Project and cross-scope snapshots | refused `snapshot_unavailable` with a receipt | Core `writes_escapes_and_foreign_inputs_are_refused_with_receipts` |
| Stale or missing snapshot references | refused with a receipt; replay reports `input_unavailable` | Core deny and replay tests |
| Replay after snapshot refresh | historical result unchanged; replay still `reproduced` against the pinned snapshot | Core `a_refresh_never_mutates_a_completed_run` |
| Receipt tampering | refused on restore | storage `restore_rejects_hand_edited_082_snapshots` (SQL edited, digest swapped, scope moved) |
| Result tampering | refused on read and restore | digest re-checked on every read; storage `receipts_and_results_commit_atomically_and_are_digest_checked`, restore tamper cases |
| Backup/restore invariant loss | refused | cross-row consistency check after restore; storage `consistency_check_detects_invariant_breaks`, `backup_restore_roundtrips_every_082_row_exactly` |
| Source snapshot deletion cascade | none possible | analytics tables have no foreign keys into Spec 075 tables and no delete path; analytics never writes a snapshot |

## Honest residuals

- Row-level integrity of receipts in the metadata store is structural
  (column/body agreement, invariants, cross-row checks), not cryptographic:
  a consistent hand edit of an unencrypted `SyntheticVault` row is not
  detected. This matches Specs 078-081. Encrypted vaults add SQLCipher
  page encryption.
- Sort and group work can spill to SQLite temporary storage within the
  time limit; memory and temporary disk are bounded by the time limit and
  value bounds, not by a per-query byte quota. SQLite's hard heap limit is
  process-wide and would affect vault connections, so it is not used.
- Replay failures (engine error or timeout) are reported as `diverged`
  with no replay digest; the contract has no separate verdict for them.
- No cross-engine equivalence is claimed; DataFusion is not qualified.

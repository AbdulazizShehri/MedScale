# CLOSURE — Spec 082 Analytics Gate

## Terminal truth

```text
SPEC_082_CLOSED_CANONICAL=true
MERGE_SHA=3ea66109617e5fce70fd8a659f5e8657ec39bcb6 (PR #142)
FINAL_HEAD=7cc58ec6e0fc437cc1f4df67c6c1423bc16fb0ee
EXACT_HEAD_CI=35928650470 (6/6)
CODE_HEAD_CI=35921559272 (6/6 on 11150b5; ubuntu 807 passed / 0 failed,
  windows 804 passed / 0 failed / 1 ignored)
POST_MERGE_MAIN_CI=35969739867 (6/6 on 3ea6610)
REVIEW_POLICY=FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external reviewer;
  deterministic scope record in EXACT_RANGE_REVIEW.md; security challenge in
  SECURITY.md)
ENGINE=sqlite (already admitted; rusqlite `limits` feature enabled, no new
  crate); DATAFUSION=DEFERRED_CANDIDATE; ENGINE_CONTRACT=ENGINE_NEUTRAL
CONTRACTS=PASS (6)  ENGINE=PASS (9)  STORAGE_MIGRATION_RECOVERY=PASS (8,
  v10->v11 additive; 9 restore tamper cases)  CORE_AUTHORITY=PASS (8 + 3 unit)
CLI=PASS (2)  DESKTOP_VIEW_MODEL=PASS (1)
CROSS_ENGINE_EQUIVALENCE=NOT_CLAIMED
REAL_PHI_AUTHORIZED=false
RELEASE_READY=false
PRIVATE_DATA_READY=false
SPEC_083_IMPLEMENTATION_AUTHORIZED=false until its own promotion
```

## What this closure establishes

MedScale has one Core-owned, read-only analytics path over exact snapshots:
- Every query binds snapshots of its own Project by id, content digest,
  schema fingerprint and row count, and leaves a receipt, whether it
  completed, was truncated, denied, timed out or failed.
- The engine is a private in-memory SQLite database holding only the bound
  rows. Writes, DDL, attachment, pragmas, extensions, file functions and
  stacked statements are refused. Values larger than the engine bound are
  never built, and runaway queries stop at the time limit.
- Completed results are immutable derived tables whose digest is re-checked
  on every read. Replay re-runs a receipt against its pinned inputs and
  reports `reproduced`, `diverged`, `input_unavailable` or `not_replayable`.
  A later snapshot never changes an earlier run.
- Cohorts compile to quoted identifiers and bound parameters; values that
  do not fit their field type are refused.
- Descriptive statistics match hand-computed fixtures and report
  `insufficient` or `not_numeric`, never a silent zero.

## Defects found and fixed during qualification

The deterministic security challenge found four defects, each fixed forward
(`SECURITY.md` F1-F4): engine-side giant values, a restore panic on
non-ASCII backup hex, silent cohort type mismatches, and unchecked row
indexing in statistics. The same restore-hex defect in the closed Spec 080
and 081 modules is fixed separately in PR #144.

## Honest residuals (non-blocking, recorded)

- DataFusion is not qualified; no cross-engine equivalence is claimed.
- No user-initiated cancel; the time limit is the only stop.
- Per-query memory and temporary disk are bounded by the time limit and
  value bounds, not by a byte quota.
- Replay engine errors report `diverged` with no replay digest.
- Receipt-row integrity in unencrypted vaults is structural, not
  cryptographic.
- No NL-to-SQL, charts or inferential statistics (out of scope).
- No rendered Desktop screenshot (no CI rendering step).
- Performance at scale beyond the snapshot row bound is unmeasured.
- No local compile or test run completed on this workstation; GitHub
  Actions is the compiler of record.

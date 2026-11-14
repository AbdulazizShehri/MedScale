# Spec 082 — Analytics Gate

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-23
**Base SHA:** `__BASE__`
**Target branch:** `spec/082-analytics-gate`
**Dependency:** 074 + 075 + 079 (all closed)
**Promotion authority:** `docs/planning/SPEC_082_PROMOTION.md`

## 1. Problem

Researchers need to ask questions of their datasets: counts, groups,
cohorts, summaries. Doing it outside MedScale loses the link to the exact
data. Doing it inside with an unrestricted engine would let a query change
data, reach files, or report a partial answer as complete.

## 2. Goal

One Core-owned, read-only analysis path over exact immutable snapshots:
- every run pins its inputs and engine, and leaves a receipt;
- results are immutable derived tables that can be replayed;
- cohorts are built from typed criteria with bound values;
- statistics say when they could not be computed.

## 3. Scenarios

1. A researcher groups a lab snapshot by sex and averages LDL. The result
   and a receipt pinning the snapshot digest are stored.
2. `DELETE`, `DROP`, `ATTACH`, a pragma, or a second statement is refused
   with a reason and a receipt; the snapshot is unchanged.
3. A query returning more than the row limit is stored as `truncated`.
4. The source is refreshed into a new snapshot. The old receipt still
   replays as `reproduced`.
5. A cohort "age >= 40 and sex = f" runs; an injection-shaped value matches
   nothing.
6. Statistics over a column with a missing value report the count and the
   missing count, and a one-value standard deviation is `insufficient`.

## 4. Requirements

- FR-01 Governed bindings: alias to a snapshot of the same Project, pinned
  by digest.
- FR-02 Read-only engine with text screen, single-statement and read-only
  checks, `query_only`, and bounds.
- FR-03 Receipts for every outcome; derived tables for completed runs.
- FR-04 Replay against pinned inputs.
- FR-05 Cohort builder with bound parameters.
- FR-06 Descriptive statistics with honest non-computation states.
- FR-07 Storage v11, backup/restore and consistency checks.
- FR-08 CLI and Desktop over Core.

## 5. Success criteria

The frozen acceptance requirements in `SPEC_082_PROMOTION.md`.

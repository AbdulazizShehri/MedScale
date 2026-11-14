# Spec 085 — MedScale Compute (local bounded worker foundation)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-26
**Base SHA:** see `docs/planning/SPEC_085_PROMOTION.md`
**Target branch:** `spec/085-compute`
**Dependency:** 077 + 079 (both closed); the local worker does not require Hub.
**Promotion authority:** `docs/planning/SPEC_085_PROMOTION.md`

## 1. Problem

Researchers need derived tables computed from their exact data: profiles,
sorted extracts, and later heavier analysis. Running that work inside
Core mixes it with the process that holds the vault. Running it outside
MedScale loses the link to the exact input and lets partial, oversized or
forged results pass as real.

## 2. Goal

A Core-owned job path with a separate MedScale-owned worker process:
- Core admits a typed job from a closed set of kinds, pins one exact Spec
  075 snapshot by its content digest, and records the job;
- a run claims the job once, stages the exact stored bytes, and hands them
  to `medscale-compute-worker` over a pipe, with a cleared environment, an
  empty private working directory and no path into the vault;
- the worker applies its OS's `ReadyBaseMeasured` mechanism to itself,
  computes, and answers on stdout;
- Core treats the answer as a candidate and commits it as a derived
  output only after checking binding, digest, canonical encoding and
  shape; every outcome leaves a receipt with a distinct state.

## 3. Scenarios

1. A researcher profiles a lab snapshot. The worker returns one row per
   column; Core stores it with a receipt pinning the snapshot digest and
   the worker's sandbox report.
2. A sorted extract of two columns is produced; running the same job again
   produces byte-identical output.
3. A job naming an unknown column, another Project's snapshot or an
   oversized input is refused at admission with a receipt.
4. The snapshot's stored bytes change after submission: the job is not
   executed and ends `corrupt`.
5. A worker hangs, floods its output, crashes, or lies about its job or
   digest: the job ends `timed_out`, `resource_exhausted`, `failed` or
   `corrupt`, and nothing is committed.
6. The machine crashes mid-run: the job is recovered as `interrupted` and
   never re-run automatically.

## 4. Requirements

- FR-01 Closed job kinds `column_profile` v1 and `sorted_projection` v1;
  no code, script, shell, interpreter, plugin or worker path in a request.
- FR-02 Admission with pinned input, execution policy, resource ceilings,
  runtime identity and manifest digest; receipts for refusals.
- FR-03 Single claim; staging of exact bytes re-verified at run time.
- FR-04 Supervised worker process: cleared environment, empty scratch
  directory, stdio pipes only, timeout, cancellation, bounded stdout and
  stderr.
- FR-05 Worker self-confinement with the measured OS mechanism, reported
  honestly; fail closed when a required mechanism is unavailable.
- FR-06 Candidate validation before an atomic commit of state, receipt and
  output.
- FR-07 Restart recovery to `interrupted`; storage v14, backup/restore and
  consistency checks.
- FR-08 CLI over Core.

## 5. Success criteria

The frozen acceptance requirements in `SPEC_085_PROMOTION.md`.

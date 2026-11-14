# Spec 086 — R Workspace (foundation slice)

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-26
**Base SHA:** see `docs/planning/SPEC_086_PROMOTION.md`
**Target branch:** `spec/086-r-workspace`
**Dependency:** 075 + 079 + 082 + 085 (all closed).
**Promotion authority:** `docs/planning/SPEC_086_PROMOTION.md`

## 1. Problem

Researchers analyse in R. Exporting data by hand loses the link to the
exact snapshot; results pasted back lose their provenance; and running a
user's R inside MedScale would hand arbitrary code the process that holds
the vault.

## 2. Goal

A reproducible hand-off in both directions without giving R any MedScale
authority:
- Core stages exact Spec 075 snapshots as digest-pinned, read-only CSV
  copies into a workspace directory outside the vault, with an `.Rproj`,
  a README and a MedScale descriptor;
- the user opens it in RStudio, Positron or a folder, launched by Core
  with no shell and an allowlisted environment;
- results return only when the user publishes one named file, which Core
  validates, pins and commits as a derived, unreviewed table;
- a managed `Rscript` run is recorded and refused: arbitrary code is not
  admitted while the OS sandbox is not platform qualified.

## 3. Scenarios

1. A researcher stages two lab snapshots, opens the workspace in RStudio,
   writes `outputs/means.csv`, inspects it, and publishes it with the
   digest they saw. The table names both snapshots and is `local_phi`.
2. The same snapshots staged again produce byte-identical data files.
3. A script, a symlink or a 20 MB log in `outputs/` is never imported.
4. The user edits a staged data copy: the workspace is `changed`, and
   launch, run and publication are refused until they stage afresh.
5. The researcher asks MedScale to run `analysis.R`: the request is
   recorded with the script and `renv.lock` digests and refused.

## 4. Requirements

- FR-01 Staging of exact snapshots to a host-configured directory outside
  the vault; deterministic content; read-only data copies; atomic from
  the database's view.
- FR-02 Integrity inspection against recorded digests; input currency;
  lockfile evidence; publishable candidates (regular files only).
- FR-03 External launch (configured absolute program, workspace as only
  argument, no shell, stdio closed, allowlisted environment) with
  receipts for every attempt.
- FR-04 Managed run requests recorded and refused; nothing executes.
- FR-05 Explicit publication with link, bound, digest and exact-CSV
  checks; atomic commit of receipt and table; class inheritance.
- FR-06 Storage v15, backup/restore and consistency checks.
- FR-07 CLI over Core.

## 5. Success criteria

The frozen acceptance requirements in `SPEC_086_PROMOTION.md`.

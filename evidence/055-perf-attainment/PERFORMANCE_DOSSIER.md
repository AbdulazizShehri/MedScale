# Performance Attainment Dossier - Spec 055 (READY_BASE)

- verdict: PERFORMANCE_NON_ATTAINMENT
- budgets_claimed_met: false
- missing_bar: perf_budgets_attained_on_qualified_hardware
- release_ready: false (performance axis alone does not clear release)

## What was measured
- CI-scale harnesses from specs 027 (64 events / 128KiB), 042 (1000 timeline
  events + 1MiB FHIR), 045 (procedural 10k lexical corpus wiring), 050
  (operator host-bound path + binding sidecar). CI runs green at these scales;
  they are feasibility/scale probes, not budget attainment runs.

## Why NON_ATTAINMENT
- No locked release budget set has been claimed in-repo.
- No locked multi-OS qualified-hardware measurement matrix exists
  (OS/version/CPU/RAM/storage/rustc/target/profile/warmups/runs/p50/p95/peak).
- One-host or CI-runner figures cannot establish release performance.

## What would flip the verdict
1. Canonical budget set locked (operations, scales, p50/p95/peak thresholds).
2. Locked harness revisions run on declared qualified hardware (each OS:
   Linux/Windows/macOS) with full binding sidecars.
3. All budgets attained with margin, reproduced, and committed as evidence.
4. Doctor `budgets_claimed_met` flipped only by that evidence.

## Honesty
- This dossier asserts no budget attainment and no release readiness.
- No trust/security/correctness trade was made for numbers.
- MESC not involved.


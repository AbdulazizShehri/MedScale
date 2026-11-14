# Feature Specification: Performance Attainment / Non-Attainment Dossier (Q05 residual)

**Branch**: `spec/055-perf-attainment-dossier`
**Status**: READY_BASE dossier (this package)
**Promotion**: EXISTING_Q05_RESIDUAL (closes `budgets_claimed_met` ambiguity with an honest bound dossier)
**Does not**: claim budget attainment, RELEASE_READY, qualified hardware, multi-OS performance, or clinical quality.

## User Stories

### US1 Bound performance dossier (P1)
A committed dossier binds source SHA, tree SHA, Cargo.lock SHA256, toolchain
(rustc/cargo/target), build profile, OS/CPU/RAM/storage, harness scales
(027/042/045/050 lineage), warmups/run counts, and observed p50/p95/peak-memory
figures with exact provenance. Every figure carries its measurement context;
no figure is presented as a release budget claim.

### US2 Honest non-attainment verdict (P1)
The dossier records verdict `PERFORMANCE_NON_ATTAINMENT` (budgets not claimed
on qualified hardware) with the precise missing bar:
`perf_budgets_attained_on_qualified_hardware`. It states what would be required
to flip the verdict (qualified multi-OS hardware + locked harness runs +
attained budgets) without claiming any of it.

### US3 Machine-checkable honesty (P1)
An integration test asserts the dossier and binding files exist, contain the
bound SHAs keys, contain `PERFORMANCE_NON_ATTAINMENT`, and never contain a
`budgets_claimed_met=true` or `RELEASE_READY=true` claim.

## Anti-scope
Optimizing for benchmark numbers; weakening trust/security/correctness;
claiming attainment; multi-OS qualification; signing/installers/SPDX.


# Feature Specification: Release Qualification Residual Integrity (Q05)

**Branch**: `spec/057-trusted-v1-perf-coverage`
**Status**: CLOSED_CANONICAL
**Promotion**: FRESH_EXECUTABILITY_AUDIT_2026-09-14
**Does not**: claim performance-budget attainment, final-v0 UI qualification, signing, installer qualification, or `RELEASE_READY`.

## User Stories

### US1 Runtime performance measurement coverage (P1)
Trusted V1 release qualification has an executable measurement path for model-free desktop process cold launch and scaffold idle RSS, with explicit warmups/runs, p50/p95, source/tree/lock/toolchain/OS binding, and no budget-pass claim.

### US2 Final-UI honesty (P1)
The performance methodology lists all six Trusted V1 performance targets. UI interaction response and final-shell launch/memory remain explicitly blocked by `FINAL_V0_UI_ARTIFACT`; scaffold measurements must never be presented as final product qualification.

### US3 Immutable CI action integrity (P1)
Every GitHub Actions `uses:` reference in the canonical CI workflow is pinned to an immutable commit SHA. The mutable `actions/upload-artifact@v4` reference is replaced by the exact `v4.6.2` commit SHA.

### US4 Machine-checkable regression protection (P1)
Tests fail if the runtime measurement path disappears, performance honesty is weakened, or a mutable action tag is reintroduced into `.github/workflows/ci.yml`.

## Anti-scope
Performance optimization; changing security/privacy semantics; redesigning UI; MESC; signing credentials; real PHI; claiming qualified multi-OS budget attainment.

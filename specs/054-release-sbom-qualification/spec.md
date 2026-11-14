# Feature Specification: Native / Full Release SBOM Qualification (Q05 residual)

**Branch**: `spec/054-release-sbom-qualification`
**Status**: READY_BASE implementation (this package)
**Promotion**: EXISTING_Q05_RESIDUAL (closes `release_sbom_native_model_assets` scaffold gap at READY_BASE)
**Does not**: claim RELEASE_READY, REPRODUCIBLE_BUILD, SPDX license decision, signed release, or real installer.

## User Stories

### US1 Deterministic CycloneDX release SBOM (P1)
A deterministic generator produces a CycloneDX 1.5 SBOM document from the actual
source state, binding `source_sha`, `tree_sha`, `cargo_lock_sha256`, build
environment (os, target triple, rustc, cargo), workspace package inventory,
Rust dependency inventory (cargo metadata), native/system dependency inventory
(explicitly classified), artifact inventory, and Pack/model asset inventory with
honest license metadata. No timestamps break determinism (fixed `ZERO_TIME`
unless an explicit override is supplied for display only and excluded from the
document hash).

### US2 SBOM verification (P1)
A verifier proves: document parses; required properties present; source/tree/
lock digests match the live checkout; every workspace package is represented;
native deps are represented or explicitly classified; artifacts represented;
license fields honestly sourced (missing license = `NOASSERTION`, never
fabricated); document SHA recorded. Verification fails closed on tamper, stale
digests, missing required components, and missing required properties.

### US3 Doctor honesty (P1)
Doctor reports `release_sbom_qualified=true` (READY_BASE), keeps
`release_ready=false`, `reproducible_build=unproven`, and drops
`release_sbom_native_model_assets` from the missing-evidence list while
retaining remaining external residuals (signing, installers, SPDX decision,
branch protection).

## Anti-scope
Public SPDX license choice; signing/notarization; real installers; bit-for-bit
reproducibility proof; vulnerability (VEX) analysis; MESC artifacts.

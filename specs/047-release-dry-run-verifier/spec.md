# Feature Specification: Release Dry-Run + Cross-Verifier (Q05 residual)

**Feature Branch**: `spec/047-release-dry-run-verifier`  
**Promotion**: `docs/planning/SPEC_047_PROMOTION.md`  
**Status**: READY (Trusted V1 Q05 residual)

## Clarifications

- Dry-run fills unsigned release-manifest digests from live repository identity only.
- Verifier never flips `release_ready` to true.
- Native-deps inventory lists admitted FFI/native components from MedScale admissions; model/Pack assets remain explicitly missing.

## User Scenarios

### US1 — Operator dry-run binds identity

**Given** a clean MedScale checkout with `Cargo.lock`  
**When** the operator runs `pwsh ./scripts/release-dry-run.ps1`  
**Then** the release-manifest records source SHA, tree SHA, Cargo.lock SHA-256, build-environment identity, and native-deps honesty inventory, with `release_ready=false`.

### US2 — Verifier fails closed on mismatch

**Given** a dry-run manifest  
**When** digests disagree with live git/lock or SBOM/checksum scaffolds  
**Then** `verify-release-manifest.ps1` exits non-zero and does not claim RELEASE_READY.

### US3 — Doctor honesty

**Given** Spec 047 READY_BASE  
**When** doctor release-qualification is queried  
**Then** `release_dry_run_verifier_present=true`, `sbom_lock_bound=true`, `release_ready=false`, and missing classes still include native/model assets and signing verification.

## Requirements

- **FR-001**: Dry-run script binds `source.git_sha`, `source.tree_sha`, `source.cargo_lock_sha256` from live repository.
- **FR-002**: Dry-run records `build_environment` (os, arch, rustc) without claiming reproducible release packages.
- **FR-003**: Dry-run writes admitted native-deps inventory with `model_assets_included=false`.
- **FR-004**: Verifier cross-checks manifest ↔ live identity ↔ SBOM `medscale:cargo_lock_sha256` ↔ checksums lockfile digest.
- **FR-005**: Verifier refuses manifests with `release_ready=true` or `signed=true` under Spec 047 (unsigned READY_BASE).
- **FR-006**: Doctor exposes `release_dry_run_verifier_present`; keeps RELEASE_READY false.

## Success Criteria

- Dry-run + verify round-trip succeeds on clean mainline checkout.
- Doctor honesty tests pass; no RELEASE_READY claim.
- Evidence under `evidence/047-release-dry-run-verifier/` with LIMITATIONS.

## Out of scope

Signing credentials, notarization, binary installers, model weight SBOMs, branch protection settings, public SPDX choice, WCAG/final v0, MESC release assets.

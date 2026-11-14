# Feature Specification: Package Upgrade/Rollback Dry-Run Scaffold (Q05 residual)

**Feature Branch**: `spec/049-package-upgrade-rollback`  
**Promotion**: `docs/planning/SPEC_049_PROMOTION.md`

## Requirements

- **FR-001**: Script verifies baseline release-manifest + checksums exist and are unsigned (`release_ready=false`).
- **FR-002**: Script can compare a candidate dry-run snapshot against baseline lock digest continuity (same Cargo.lock identity for scaffold; candidate may rebind source/tree).
- **FR-003**: Rollback check: restoring baseline paths/digests must be expressible as a documented fail-closed procedure without mutating production installers (none exist).
- **FR-004**: Doctor `package_upgrade_rollback_scaffold_present=true`; `release_ready=false`.
- **FR-005**: Keep honest missing residual for real package artifacts (`release_package_upgrade_rollback_proof` remains until installers exist).

## Out of scope

Real installers, signed upgrade channels, app-store submission, RELEASE_READY.

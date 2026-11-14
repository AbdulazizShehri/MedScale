# Feature Specification: Portable Release Package Qualification (Q05)

**Branch**: `spec/058-portable-release-package`
**Status**: CLOSED_CANONICAL
**Promotion**: FRESH_EXECUTABILITY_AUDIT_2026-09-14
**Does not**: claim reproducible binary builds, signing/notarization, native installer formats, final-v0 qualification, or `RELEASE_READY`.

## User Stories

### US1 Deterministic portable package contents (P1)
A cross-platform package builder produces an unsigned portable ZIP containing the real release-profile CLI/Desktop binaries, Apache-2.0 license, NOTICE inventory, release SBOM, README, and a source/tree/lock-bound payload manifest. Two packages built from the same binaries, source identity, platform and revision are bit-for-bit identical.

### US2 Fail-closed package verification (P1)
A verifier extracts the portable ZIP into a temporary directory, rejects missing/unexpected/tampered payloads, verifies every manifest SHA-256, confirms mandatory legal/SBOM artifacts, and smoke-runs the packaged binaries.

### US3 Install / upgrade / rollback proof (P1)
A portable installer manages versioned package slots plus an explicit current/previous state. Qualification installs a baseline package, upgrades to a distinct candidate package revision, rolls back to baseline, and smoke-validates the active slot after every transition.

### US4 Honest release qualification (P1)
Doctor reports portable package contents and package lifecycle proof present, removes only `reproducible_release_package_contents` and `release_package_upgrade_rollback_proof`, and keeps signing/provenance, qualified-hardware performance, macOS final-product, final-v0/WCAG, and material-findings gates open.

## Anti-scope
Production signing keys; notarization; MSI/MSIX/DMG/pkg/deb/rpm/AppImage; app-store publication; final v0 UI; MESC; real PHI; bit-for-bit reproducible compiler output.

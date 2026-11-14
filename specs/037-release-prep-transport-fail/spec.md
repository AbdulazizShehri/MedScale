# Feature Specification: Release Prep + Transport Fail Fixtures

**Feature Branch**: `spec/037-release-prep-transport-fail`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 013 network broker; Spec 022/027 release prep; Spec 032 NOTICE  
**Does not**: configure GitHub branch protection; choose SPDX; claim RELEASE_READY; enable live partners.

## Requirements

- **FR-001**: Owner-facing required-check inventory with exact CI job names.
- **FR-002**: FixtureTransport synthetic `fail:timeout` / `fail:transport` map to stable broker reasons.
- **FR-003**: Checksum verify script for existing package-checksums.json scaffold.
- **FR-004**: License counsel packet reducing founder decision to a smallest SPDX choice (no choice made here).
- **FR-005**: BUILD_QUEUE honesty: Spec 037 closed; advanced deferred **038+**.

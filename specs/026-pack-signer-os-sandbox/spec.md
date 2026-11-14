# Feature Specification: Pack Signer / Trust / Anti-Rollback + OS Sandbox READY_BASE (Q09)

**Feature Branch**: `spec/026-pack-signer-os-sandbox`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 008 CLOSED; Trusted V1 Q09 P1  
**Does not**: claim `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, full multi-OS `PLATFORM_QUALIFIED`, clear `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`, real model admission, gated model terms.

## User Stories

### US1 — Signed synthetic Pack admission (P1)
Operator/tests admit a local Pack whose manifest carries digests, rights, `sbom_ref`, pack epoch, trust-root id, and an ed25519 signature verified against the MedScale synthetic trust root.

### US2 — Anti-rollback (P1)
Re-admission of the same `pack_id` with a lower `pack_epoch` (or equal epoch with lower version) is rejected.

### US3 — Linux Landlock measured apply (P1)
On `cfg(target_os = "linux")`, `try_apply_os_sandbox` for a `ReadyBaseMeasured` Linux plan enforces an allowlist and denies ambient open of a path outside it. Windows/macOS remain NotPlatformQualified scaffolds.

### US4 — Doctor honesty (P1)
Doctor reports pack_signer and os_sandbox READY_BASE honesty: `linux_measured=true`, `platform_qualified=false`, `release_ready=false`.

## Requirements

- **FR-001**: Pack admission requires signature verification against synthetic trust root (fixture/synthetic Packs only).
- **FR-002**: Digests, rights_uri, and sbom_ref remain mandatory.
- **FR-003**: Anti-rollback rejects lower epoch/version than last admitted for the same pack_id.
- **FR-004**: `OsSandboxQualification::ReadyBaseMeasured` for Linux Landlock only; never set multi-OS `PlatformQualified` while EXTERNAL_GATES remains OPEN.
- **FR-005**: Measured Linux apply + test proving denied path outside allowlist; Windows/macOS stubs compile.
- **FR-006**: Doctor axes pack_signer / os_sandbox honesty flags as above.
- **FR-007**: Dependency admissions for new crates; update deny.toml if needed.
- **FR-008**: Spec Kit package + evidence; BUILD_QUEUE/roadmap/START_HERE → Spec 026 CLOSED; deferred **027+**.

## Out of scope

Real model Packs, gated HF terms, Windows AppContainer / macOS Seatbelt measured apply, clearing `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`, RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY, REAL_PHI, MESC mutation.

## Success Criteria

- Workspace `--locked` green on Windows (stubs); Linux CI exercises Landlock measured test
- Doctor honesty flags as above
- EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN
- Evidence LIMITATIONS honest

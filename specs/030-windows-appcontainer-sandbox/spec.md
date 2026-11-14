# Feature Specification: Windows AppContainer / Job Object Worker Sandbox READY_BASE (Q09 residual)

**Feature Branch**: `spec/030-windows-appcontainer-sandbox`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 026 CLOSED (Linux Landlock ReadyBaseMeasured); Spec 008 OS sandbox scaffold  
**Does not**: claim `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, clear `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`, macOS Seatbelt measured apply, full AppContainer FS/network isolation PLATFORM_QUALIFIED.

## User Stories

### US1 — Windows Job Object ReadyBaseMeasured (P1)
On `cfg(windows)`, `try_apply_os_sandbox` for a `ReadyBaseMeasured` Windows plan applies Job Object limits and a measured test proves a denied capability (child process creation under `ACTIVE_PROCESS=1`) without requiring administrator rights in CI.

### US2 — AppContainer honesty (P1)
Full AppContainer profile/FS/network isolation remains scaffold (`NotPlatformQualified`) when too heavy or privilege-sensitive for CI. Limitations are documented honestly.

### US3 — Doctor honesty (P1)
Doctor `os_sandbox`: `windows_measured=true`, `linux_measured=true`, `platform_qualified=false`, `release_ready=false`. EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` stays OPEN (macOS still NotPlatformQualified).

### US4 — Cross-OS non-regression (P1)
Linux Landlock path remains green; macOS CI remains green (Windows plan returns `NotReadyOnThisHost` off-Windows; scaffolds unchanged).

## Requirements

- **FR-001**: Extend `OsSandbox` so Windows can report ReadyBaseMeasured with measured confinement proof on `cfg(windows)`.
- **FR-002**: Prefer Job Object limits runnable without admin; keep AppContainer as scaffold if not CI-feasible.
- **FR-003**: Doctor axes include `windows_measured`; never set `platform_qualified` or `release_ready`.
- **FR-004**: Leave EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` OPEN.
- **FR-005**: Admit `windows-sys` (Windows-only) with provenance; update deny if needed.
- **FR-006**: Spec Kit + evidence; BUILD_QUEUE 030 CLOSED; deferred **031+**.

## Out of scope

Clearing multi-OS PLATFORM_QUALIFIED gate; macOS Seatbelt measured apply; RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT; REAL_PHI; MESC mutation; claiming Job Object equals AppContainer FS isolation.

## Success Criteria

- Workspace `--locked` green (fmt, clippy `-D warnings`, test) with `CARGO_TARGET_DIR=D:\medscale-target`
- Windows measured child-deny evidence; Linux/macOS non-regression
- Doctor honesty flags as above; gate remains OPEN

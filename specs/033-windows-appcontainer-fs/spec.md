# Feature Specification: Windows AppContainer FS Worker Sandbox READY_BASE (Q09 residual)

**Feature Branch**: `spec/033-windows-appcontainer-fs`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 030 CLOSED (Windows Job Object ReadyBaseMeasured); Spec 026/031 CLOSED  
**Does not**: claim `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, clear `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`, measure AppContainer network/LPAC, or claim multi-OS PlatformQualified.

## User Stories

### US1 — AppContainer FS ReadyBaseMeasured (P1)
On `cfg(windows)`, `try_apply_os_sandbox` for `windows_appcontainer_fs_ready_base` creates/derives a per-user AppContainer profile, launches `medscale-os-sandbox-probe` as a child with `SECURITY_CAPABILITIES`, and proves the child cannot read a host-temp marker file outside the profile.

### US2 — Job Object non-regression (P1)
Spec 030 Job Object ReadyBaseMeasured remains green; AppContainer network/LPAC remains scaffold (`NotPlatformQualified`).

### US3 — Doctor honesty (P1)
Doctor `os_sandbox`: `windows_appcontainer_fs_measured=true`, `windows_measured=true`, `linux_measured=true`, `macos_measured=true`, `platform_qualified=false`, `release_ready=false`. EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` stays OPEN.

### US4 — Cross-OS non-regression (P1)
Off-Windows hosts return `NotReadyOnThisHost` for the AppContainer FS plan; Linux/macOS measured paths unchanged.

## Requirements

- **FR-001**: Extend `OsSandbox` with `WindowsAppContainerFs` ReadyBaseMeasured plan and measured host-marker FS deny.
- **FR-002**: Prefer `CreateAppContainerProfile` / `DeriveAppContainerSidFromAppContainerName` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` without elevation.
- **FR-003**: Doctor axis `windows_appcontainer_fs_measured`; never set `platform_qualified` or `release_ready`.
- **FR-004**: Leave EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` OPEN.
- **FR-005**: Reuse `windows-sys` 0.61.2; Isolation + Authorization features admitted.
- **FR-006**: Spec Kit + evidence; BUILD_QUEUE 033 CLOSED; deferred **034+**.

## Out of scope

Clearing multi-OS PLATFORM_QUALIFIED; AppContainer network/LPAC matrix; macOS App Sandbox entitlements; RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT; REAL_PHI; MESC mutation.

## Success Criteria

- Workspace `--locked` green; Windows CI measures AppContainer FS deny
- Doctor honesty flags as above; gate remains OPEN

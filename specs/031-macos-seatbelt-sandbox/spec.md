# Feature Specification: macOS Seatbelt / sandbox Worker Sandbox READY_BASE (Q09 residual)

**Feature Branch**: `spec/031-macos-seatbelt-sandbox`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 026 CLOSED (Linux Landlock ReadyBaseMeasured); Spec 030 CLOSED (Windows Job Object ReadyBaseMeasured); Spec 008 OS sandbox scaffold  
**Does not**: claim `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, clear `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`, App Sandbox entitlements PLATFORM_QUALIFIED, Windows AppContainer FS/network measured isolation.

## User Stories

### US1 — macOS Seatbelt ReadyBaseMeasured (P1)
On `cfg(target_os = "macos")`, `try_apply_os_sandbox` for a `ReadyBaseMeasured` macOS plan applies an in-process Seatbelt/`sandbox_init` profile and a measured test proves a denied ambient capability (network) without App Sandbox entitlements in CI (`macos-latest`).

### US2 — Entitlements honesty (P1)
Full App Sandbox entitlements / container FS isolation remain scaffold (`NotPlatformQualified`) when impossible or not CI-feasible. Limitations are documented honestly; strongest measured deny available (`sandbox_init` SBPL) is used.

### US3 — Doctor honesty (P1)
Doctor `os_sandbox`: `macos_measured=true`, `windows_measured=true`, `linux_measured=true`, `platform_qualified=false`, `release_ready=false`. EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` stays OPEN (Windows AppContainer FS still scaffold; multi-OS PLATFORM_QUALIFIED not claimed).

### US4 — Cross-OS non-regression (P1)
Linux Landlock and Windows Job Object paths remain green; off-macOS hosts return `NotReadyOnThisHost` for the macOS ReadyBaseMeasured plan; scaffolds unchanged.

## Requirements

- **FR-001**: Extend `OsSandbox` so macOS can report ReadyBaseMeasured with measured confinement proof on `cfg(target_os = "macos")`.
- **FR-002**: Prefer Apple `sandbox_init` / Seatbelt profile string applied in-process; if full App Sandbox entitlements are impossible in CI, use the strongest measured deny available and document limitations.
- **FR-003**: Doctor axes include `macos_measured`; never set `platform_qualified` or `release_ready`.
- **FR-004**: Leave EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` OPEN.
- **FR-005**: Prefer zero new crates (libSystem FFI); admit FFI provenance if required.
- **FR-006**: Spec Kit + evidence; BUILD_QUEUE 031 CLOSED; deferred **032+**.

## Out of scope

Clearing multi-OS PLATFORM_QUALIFIED gate; Windows AppContainer FS/network measured apply; RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT; REAL_PHI; MESC mutation; claiming Seatbelt network-deny equals App Sandbox container isolation.

## Success Criteria

- Workspace `--locked` green (fmt, clippy `-D warnings`, test); local Windows uses `CARGO_TARGET_DIR=D:\medscale-target` with non-macOS stubs
- macOS measured network-deny evidence on CI `macos-latest`; Linux/Windows non-regression
- Doctor honesty flags as above; gate remains OPEN

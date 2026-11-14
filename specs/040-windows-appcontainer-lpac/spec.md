# Feature Specification: Windows AppContainer LPAC READY_BASE (Q09 residual)

**Feature Branch**: `spec/040-windows-appcontainer-lpac`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 038 AppContainer network CLOSED  
**Promotion**: `EXISTING_Q09_RESIDUAL_ELIGIBLE_FOR_PROMOTION` — see `docs/planning/SPEC_040_PROMOTION.md`  
**Does not**: claim PLATFORM_QUALIFIED; clear WORKER_OS_SANDBOX_PLATFORM_QUALIFIED; RELEASE_READY; expand into plugins/CUDA/generic sandbox framework.

## Requirements

- **FR-001**: `WindowsAppContainerLpac` ReadyBaseMeasured plan launches a zero-capability AppContainer child with `PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY = PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT` (LPAC).
- **FR-002**: Child proves (a) AppContainer token, (b) absence of ALL APPLICATION PACKAGES group (LPAC identity), (c) host-temp FS deny, (d) TCP connect deny (TEST-NET-3).
- **FR-003**: Doctor `windows_appcontainer_lpac_measured=true`; `platform_qualified=false`.
- **FR-004**: Fail-closed when LPAC attribute setup or CreateProcess cannot be established.
- **FR-005**: EXTERNAL_GATES sandbox gate stays OPEN (macOS App Sandbox entitlements + multi-OS composition remain).
- **FR-006**: Spec Kit + Windows-host evidence bound to SOURCE_SHA/TREE_SHA/OS/RUNNER/probe/results; BUILD_QUEUE 040 CLOSED after merge; deferred advanced **041+**.

## Clarifications

| Question | Resolution |
|---|---|
| Is 040+ forever forbidden? | No — only advanced product scope; LPAC is existing Q09 residual. |
| ReadyBaseMeasured from mocks? | No — require Windows CI / host probe exit 0. |
| Clear PLATFORM_QUALIFIED? | No. |

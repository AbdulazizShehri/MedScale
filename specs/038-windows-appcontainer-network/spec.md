# Feature Specification: Windows AppContainer Network READY_BASE (Q09 residual)

**Feature Branch**: `spec/038-windows-appcontainer-network`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 033 AppContainer FS CLOSED  
**Does not**: claim PLATFORM_QUALIFIED; measure LPAC matrix; clear WORKER_OS_SANDBOX_PLATFORM_QUALIFIED; RELEASE_READY.

## Requirements

- **FR-001**: `WindowsAppContainerNetwork` ReadyBaseMeasured plan launches zero-capability AppContainer child and proves TCP connect deny.
- **FR-002**: Doctor `windows_appcontainer_network_measured=true`; `platform_qualified=false`.
- **FR-003**: LPAC remains scaffold (`windows_appcontainer_scaffold`).
- **FR-004**: EXTERNAL_GATES sandbox gate stays OPEN.
- **FR-005**: Spec Kit + evidence; BUILD_QUEUE 038 CLOSED; deferred **039+**.

# ADR 033-001 — AppContainer FS ReadyBaseMeasured

## Status

Accepted for Spec 033 READY_BASE.

## Decision

Measure Windows AppContainer filesystem isolation by launching a helper child under a per-user AppContainer profile and proving host-temp marker read is denied. Do not clear `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`.

## Consequences

- Doctor gains `windows_appcontainer_fs_measured`.
- Network/LPAC AppContainer and macOS App Sandbox entitlements remain scaffold.
- Probe binary required for CreateProcess child measurement.

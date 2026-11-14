# Limitations — Spec 030 Windows sandbox READY_BASE

- Windows Job Object READY_BASE measures **process-creation confinement** (`ACTIVE_PROCESS=1`), not filesystem or network allowlists.
- Job Objects are **not** equivalent to Linux Landlock path-beneath rules or AppContainer LPAC isolation.
- **AppContainer** profile creation, capability SIDs, FS/network isolation remain **scaffold** (`NotPlatformQualified`).
- **macOS Seatbelt** ReadyBaseMeasured network-deny landed in Spec 031; App Sandbox entitlements remain scaffold.
- `platform_qualified=false`; EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains **OPEN**.
- `release_ready=false`; no PRIVATE_DATA_READY / MULTI_CLIENT claims.
- AssignProcessToJobObject may fail if a host job forbids nesting; documented in apply error text — probe/CI expect a normal nested-capable environment.
- `unsafe` Win32 FFI is confined to `windows_job` (`#![allow(unsafe_code)]`); workspace lint is `deny` (not `forbid`) to permit that module allow.

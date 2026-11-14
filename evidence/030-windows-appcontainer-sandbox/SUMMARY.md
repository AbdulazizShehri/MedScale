# Spec 030 evidence summary

**Status:** CLOSED_CANONICAL READY_BASE  
**Gate:** `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains **OPEN**

## Delivered

1. **Windows Job Object ReadyBaseMeasured** via `OsSandboxPlan::windows_job_object_ready_base()` and `try_apply_os_sandbox` on `cfg(windows)`.
2. **Measured deny:** `medscale-os-sandbox-probe` applies the plan then proves `CreateProcess`/child spawn is denied under `ACTIVE_PROCESS=1`.
3. **AppContainer** remains `windows_appcontainer_scaffold()` / NotPlatformQualified.
4. **Doctor:** `windows_measured=true`, `linux_measured=true`, `platform_qualified=false`, `release_ready=false`.
5. **Admission:** `docs/engineering/admissions/030-windows-sys.md`.

## Non-claims

- Not multi-OS PLATFORM_QUALIFIED; App Sandbox entitlements / Windows AppContainer FS still scaffold.
- Not AppContainer filesystem/network isolation.
- Not RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY.

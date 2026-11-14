# Spec 031 evidence summary

**Status:** CLOSED_CANONICAL READY_BASE  
**Gate:** `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains **OPEN**

## Delivered

1. **macOS Seatbelt ReadyBaseMeasured** via `OsSandboxPlan::macos_seatbelt_ready_base()` and `try_apply_os_sandbox` on `cfg(target_os = "macos")`.
2. **Measured deny:** `medscale-os-sandbox-probe` applies `sandbox_init` SBPL `(allow default)(deny network*)` then proves TCP connect fails as PermissionDenied/EPERM (not ConnectionRefused).
3. **App Sandbox entitlements** remain `macos_seatbelt_scaffold()` / NotPlatformQualified.
4. **Doctor:** `macos_measured=true`, `windows_measured=true`, `linux_measured=true`, `platform_qualified=false`, `release_ready=false`.
5. **Admission:** `docs/engineering/admissions/031-macos-sandbox-init-ffi.md` (libSystem FFI; no new crate).

## Non-claims

- Not multi-OS PLATFORM_QUALIFIED; Windows AppContainer FS/network still scaffold.
- Not App Sandbox container isolation / entitlements.
- Not RELEASE_READY / PRIVATE_DATA_READY / MULTI_CLIENT_RELEASE_READY.

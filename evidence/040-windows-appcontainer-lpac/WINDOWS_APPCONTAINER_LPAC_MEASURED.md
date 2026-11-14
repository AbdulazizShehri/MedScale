# Windows AppContainer LPAC ReadyBaseMeasured (Spec 040)

## Measure
- Profile create/derive + `SECURITY_CAPABILITIES` with `CapabilityCount=0`
- `PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY` =
  `PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT` (LPAC)
- Child `appcontainer-lpac-child`:
  - `TokenIsAppContainer` must be true
  - ALL APPLICATION PACKAGES (`WinBuiltinAnyPackageSid`) must **not** be an enabled group
  - Host-temp marker FS deny (PermissionDenied / ERROR_ACCESS_DENIED)
  - TCP connect to `203.0.113.1:9` (TEST-NET-3) denied (WSAEACCES / access denied)

## Binding (local Windows host measure 2026-09-10)

| Field | Value |
|---|---|
| SOURCE_SHA | `4ce0eaddb0a852bbdff3ac3ef1ea4fc0751a8675` |
| TREE_SHA | `914f771cc3a6f122c071c82d5abcc61665937d51` |
| OS | Windows 10/11 (win32 10.0.26200 local) + windows-latest CI |
| RUNNER | local host + `rust (windows-latest)` |
| TEST/PROBE | `medscale-os-sandbox-probe appcontainer-lpac` / `os_sandbox_040::appcontainer_lpac_measured_identity_fs_network` |
| EXPECTED_RESULT | exit 0; stderr contains `OK: AppContainer LPAC deny measured` |
| ACTUAL_RESULT | local: exit 0; 4/4 `os_sandbox_040` passed (033/038 regression also green) |
| EXIT_STATUS | 0 |

## Non-claims
- Not multi-OS PLATFORM_QUALIFIED
- Not full LPAC capability allowlist matrix
- Parent process is not AppContainer-confined
- EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN
- ReadyBaseMeasured ≠ PLATFORM_QUALIFIED

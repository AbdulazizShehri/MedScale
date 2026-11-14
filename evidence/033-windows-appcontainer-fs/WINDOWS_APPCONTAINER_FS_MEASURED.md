# Windows AppContainer FS measured — Spec 033

## Mechanism

- Profile: `CreateAppContainerProfile` / `DeriveAppContainerSidFromAppContainerName` on ALREADY_EXISTS
- Launch: `InitializeProcThreadAttributeList` + `UpdateProcThreadAttribute(PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES)` + `CreateProcessW` with `EXTENDED_STARTUPINFO_PRESENT`
- Child: `medscale-os-sandbox-probe appcontainer-fs-child <marker>`
- Proof: child cannot `read` host-temp marker → exit 0; readable → exit 2 (FAIL)

## Evidence hooks

- Module: `medscale_contracts::os_sandbox::windows_appcontainer`
- Probe: `medscale-os-sandbox-probe appcontainer-fs`
- Test: `os_sandbox_033::appcontainer_fs_measured_denies_host_marker` (`#[cfg(windows)]`)

## Honesty

ReadyBaseMeasured only. EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN.

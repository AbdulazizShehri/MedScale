# OS sandbox scaffold (Spec 008 follow-on)

**Date:** 2026-08-26 (updated Specs 026/030/031)  
**Main context:** post Spec 008 `CLOSED_CANONICAL`  
**Gate:** `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` = `OPEN`

## Claim scope

This artifact documents **plans** for:

- Linux Landlock (+ candidate seccomp/rlimit)
- Windows AppContainer + Job Object (+ brokered handles)
- macOS Seatbelt / App Sandbox / XPC pattern

**Claim:** Linux may use `OsSandboxQualification::ReadyBaseMeasured` (Spec 026).  
Windows may use Job Object `ReadyBaseMeasured` (Spec 030); AppContainer remains scaffold.  
macOS may use Seatbelt `sandbox_init` `ReadyBaseMeasured` (Spec 031); App Sandbox entitlements remain scaffold.  
**Not claimed:** multi-OS `PLATFORM_QUALIFIED`. Gate `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN.

See also: `evidence/026-pack-signer-os-sandbox/LINUX_LANDLOCK_MEASURED.md`,  
`evidence/030-windows-appcontainer-sandbox/WINDOWS_JOB_OBJECT_MEASURED.md`,  
`evidence/031-macos-seatbelt-sandbox/MACOS_SEATBELT_MEASURED.md`.

## Code

- `medscale_contracts::os_sandbox::{OsSandboxPlan, try_apply_os_sandbox}`
- Spec 026: Linux ReadyBaseMeasured Landlock apply.
- Spec 030: Windows Job Object ReadyBaseMeasured apply; AppContainer remains NotPlatformQualified scaffold.
- Spec 031: macOS Seatbelt ReadyBaseMeasured apply; App Sandbox entitlements remain NotPlatformQualified scaffold.
- `PlatformQualified` remains refused while EXTERNAL_GATES is OPEN.

## Next evidence required to close gate

Per-OS measured apply tests with exact SHA/toolchain/OS, corpus of denied ambient capabilities (including Windows AppContainer FS/network), and limitations recorded before flipping any plan to `PlatformQualified`.

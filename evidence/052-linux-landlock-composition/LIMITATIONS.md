# Spec 052 limitations

- Linux-only composition; Windows/macOS hosts return `NotReadyOnThisHost`.
- Requires Landlock TCP ABI V4+; apply fail-closes if network axis unavailable.
- No seccomp-bpf / capability bounding set composition in this unit.
- Does not clear `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` or set `platform_qualified=true`.
- Does not measure macOS App Sandbox signed enforcement or multi-OS brokered-handle composition.
- `unsafe` libc `getrlimit`/`setrlimit` confined to `linux_rlimit` (`#![allow(unsafe_code)]`).

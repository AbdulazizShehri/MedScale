# Spec 052 — Linux Landlock composition

## Delivered

- `OsSandboxTarget::LinuxLandlockComposition` + `linux_landlock_composition_ready_base`
- Apply path: Landlock FS allowlist + Landlock TCP bind/connect deny + `RLIMIT_NOFILE`
- Doctor: `linux_landlock_composition_measured=true`
- Inventory axis: `linux_landlock_composition`
- Probe mode: `medscale-os-sandbox-probe landlock-composition` (Linux)

## Honesty

| Claim | Value |
|---|---|
| PLATFORM_QUALIFIED | false |
| seccomp composition | not in this unit |
| WORKER_OS_SANDBOX_PLATFORM_QUALIFIED | remains OPEN |

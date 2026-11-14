# Linux Landlock measured evidence (Spec 026)

**Qualification:** `OsSandboxQualification::ReadyBaseMeasured`  
**Target:** `OsSandboxTarget::LinuxLandlock`  
**Gate:** `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains **OPEN** (multi-OS still required)

## Apply path

- Code: `medscale_contracts::os_sandbox::try_apply_os_sandbox`
- Crate: `landlock` **0.4.4** (Linux-only dependency admission `026-landlock`)
- ABI: Landlock ABI V1 via `AccessFs::from_all` + `path_beneath_rules` allowlist + `restrict_self`

## Measured proof

Integration test `os_sandbox_026::landlock_measured_denies_path_outside_allowlist` (`#[cfg(target_os = "linux")]`):

1. Create allow dir + deny dir with files.
2. Apply ReadyBaseMeasured plan with allow dir only.
3. Assert allow file remains openable.
4. Assert deny file open fails (ambient path outside allowlist denied).

Runs in a dedicated thread so Landlock domain does not poison other tests.

## Host matrix honesty

| Host | Apply | Claim |
|---|---|---|
| Linux | measured | ReadyBaseMeasured |
| Windows | NotReadyOnThisHost | NotPlatformQualified scaffold |
| macOS | NotReadyOnThisHost | NotPlatformQualified scaffold |

## Not claimed

`PlatformQualified`, release-ready worker confinement, Windows/macOS measured backends.

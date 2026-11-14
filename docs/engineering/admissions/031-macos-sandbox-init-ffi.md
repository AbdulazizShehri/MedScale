# Dependency / FFI admission: macOS sandbox_init (Spec 031)

| API | Source | Linkage |
|---|---|---|
| `sandbox_init` / `sandbox_free_error` | Apple libSystem (macOS) | Direct `extern "C"`; **no crate** |

| Field | Value |
|---|---|
| Owning Spec | 031 |
| Placement | `medscale-contracts` **macOS-only** (`cfg(target_os = "macos")`) Seatbelt apply |
| Purpose | Measured Seatbelt SBPL network-deny for ReadyBaseMeasured workers/probes |
| License / rights | Apple OS/SDK interfaces; MedScale does not redistribute Apple OS binaries |
| Security | Fail-closed on API failure; App Sandbox entitlements not claimed; PlatformQualified refused while EXTERNAL_GATES OPEN; sandbox irreversible per process |
| Tests required | macOS: probe denies network connect; non-macOS: NotReadyOnThisHost; Linux/Windows tests remain green |
| Update strategy | Re-run os_sandbox_031 on macos-latest after OS/toolchain changes; revisit if Apple removes sandbox_init |
| Exit strategy | Replace with entitlement-bearing App Sandbox worker launch or successor OS API behind `try_apply_os_sandbox` |

Not admitted: claiming App Sandbox container isolation; clearing EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`; PlatformQualified; new third-party sandbox crates.

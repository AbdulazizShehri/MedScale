# Dependency admission: windows-sys (Spec 030)

| Crate | Version | Features |
|---|---|---|
| windows-sys | **0.61.2** | `Win32_Foundation`, `Win32_System_JobObjects`, `Win32_System_Threading` |

| Field | Value |
|---|---|
| Owning Spec | 030 |
| Placement | `medscale-contracts` **Windows-only** (`cfg(windows)`) Job Object apply |
| Purpose | Measured Job Object ACTIVE_PROCESS confine for ReadyBaseMeasured workers |
| License | MIT OR Apache-2.0 |
| Security | Fail-closed on API failure; AppContainer not claimed; PlatformQualified refused while EXTERNAL_GATES OPEN; job handle leaked intentionally with KILL_ON_JOB_CLOSE |
| Tests required | Windows: probe denies child spawn; non-Windows: NotReadyOnThisHost; Linux Landlock tests remain green |
| Update strategy | Pin patch/minor; re-deny; re-run os_sandbox_030 on Windows |
| Exit strategy | Replace with raw `windows` crate safe wrappers or hand-written FFI behind `try_apply_os_sandbox` |

Not admitted: claiming AppContainer FS/network isolation; clearing EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`; PlatformQualified.

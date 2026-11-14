# Dependency admission: landlock (Spec 026)

| Crate | Version | Features |
|---|---|---|
| landlock | **0.4.4** | default |

| Field | Value |
|---|---|
| Owning Spec | 026 |
| Placement | `medscale-contracts` **Linux-only** (`cfg(target_os = "linux")`) OS sandbox apply |
| Purpose | Measured Landlock filesystem allowlist for ReadyBaseMeasured workers |
| License | MIT OR Apache-2.0 |
| Security | Fail-closed when kernel lacks Landlock; empty allowlist refused; Windows/macOS never link this crate |
| Tests required | Linux: deny open outside allowlist; compile stubs on Windows |
| Update strategy | Pin minor; re-deny; re-run os_sandbox_026 on Linux |
| Exit strategy | Replace with raw `landlock_*` syscalls behind `try_apply_os_sandbox` |

Not admitted: claiming multi-OS PLATFORM_QUALIFIED; clearing EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`.

# ADR-030-001 — Windows Job Object ReadyBaseMeasured; AppContainer scaffold

## Status

Accepted (Spec 030 READY_BASE)

## Context

Spec 026 delivered Linux Landlock ReadyBaseMeasured. Q09 residual asked for Windows AppContainer/Job Object complementary READY_BASE without clearing multi-OS `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` (macOS still open).

## Decision

1. Implement `OsSandboxPlan::windows_job_object_ready_base()` as `ReadyBaseMeasured` using Job Object `ACTIVE_PROCESS=1` (+ kill-on-job-close).
2. Keep `windows_appcontainer_scaffold()` as `NotPlatformQualified`.
3. Doctor: `windows_measured=true`, `platform_qualified=false`, `release_ready=false`.
4. Leave EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` OPEN.
5. Measure via dedicated probe subprocess so the cargo test process is not poisoned.

## Consequences

- Windows CI can prove a real denied capability without admin.
- Operators must not confuse Job Object process limits with AppContainer FS/network isolation.
- macOS Seatbelt remains the remaining OS scaffold for the multi-OS gate.

# ADR-031-001 — macOS Seatbelt ReadyBaseMeasured via sandbox_init

## Status

Accepted (Spec 031 READY_BASE)

## Context

Specs 026 and 030 delivered Linux Landlock and Windows Job Object ReadyBaseMeasured. Q09 residual asked for macOS Seatbelt complementary READY_BASE without clearing multi-OS `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` (Windows AppContainer FS still scaffold).

## Decision

1. Implement `OsSandboxPlan::macos_seatbelt_ready_base()` as `ReadyBaseMeasured` using `sandbox_init` + SBPL `(allow default)(deny network*)`.
2. Keep `macos_seatbelt_scaffold()` as `NotPlatformQualified` for App Sandbox entitlements / XPC candidates.
3. Doctor: `macos_measured=true`, `platform_qualified=false`, `release_ready=false`.
4. Leave EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` OPEN.
5. Measure via dedicated probe subprocess so the cargo test process is not poisoned.

## Consequences

- macOS CI can prove a real denied ambient capability without App Sandbox entitlements.
- Operators must not confuse Seatbelt network-deny with App Sandbox container isolation or Landlock path allowlists.
- Multi-OS PLATFORM_QUALIFIED still requires stronger cross-OS evidence (including Windows AppContainer FS) before gate close.

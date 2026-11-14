# Feature Specification: Linux Seccomp Composition (Q09 residual)

**Branch**: `spec/053-linux-seccomp-composition`
**Status**: READY_BASE implementation (this package)
**Input**: Close the Spec 052 seccomp residual with a child-measured strict
seccomp-bpf allowlist. Never claim PLATFORM_QUALIFIED, RELEASE_READY, or
PRIVATE_DATA_READY.

## User Stories

### US1 Strict allowlist plan (P1)
`OsSandboxPlan::linux_seccomp_composition_ready_base()` reports
ReadyBaseMeasured with mechanisms `seccomp_bpf_strict_allowlist`,
`sigsys_deny_child_measured`, `no_new_privs`, and never claims
PlatformQualified.

### US2 Child-measured SIGSYS deny (P1)
On Linux x86_64/aarch64, the probe child installs the filter, proves an
allowed write, then dies by SIGSYS on denied `socket()`. The parent
observes marker + signal. Other hosts return `NotReadyOnThisHost`.

### US3 Doctor honesty (P1)
Doctor reports `linux_seccomp_composition_measured=true`,
`platform_qualified=false`, inventory of 9 axes, residuals still open.

## Anti-scope
In-process confinement; parent confinement; signed enforcement; Windows/macOS
composition; any release/privacy readiness claim.

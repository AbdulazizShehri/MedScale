# Spec 052 — Linux Landlock composition (Q09 residual)

## Overview

Deepen Linux OS sandbox ReadyBaseMeasured beyond Spec 026 Landlock FS-only by
composing Landlock path allowlist + Landlock TCP network deny + RLIMIT_NOFILE,
without claiming multi-OS PLATFORM_QUALIFIED.

## Requirements

### Requirement: Composition plan

MedScale MUST expose an `OsSandboxPlan` target for Linux Landlock composition
with mechanisms naming FS allowlist, TCP network deny, and rlimit.

### Requirement: Fail-closed PlatformQualified

`PlatformQualified` plans MUST still be refused while EXTERNAL_GATES
`WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains OPEN.

### Requirement: Measured deny on Linux

On Linux hosts with Landlock available, applying the composition plan MUST:

1. Deny filesystem access outside `allow_paths`.
2. Deny TCP connect (and bind) via Landlock net access handling when the kernel
   ABI supports it; otherwise fail the apply with an explicit error (no silent
   skip of the network axis for this ReadyBaseMeasured unit).
3. Lower `RLIMIT_NOFILE` soft/hard limits below the pre-apply soft limit.

### Requirement: Honesty

Doctor MUST set `linux_landlock_composition_measured=true` and keep
`platform_qualified=false`. Seccomp composition remains out of scope / limitation.

## Success criteria

- Linux CI exercises composition measure (tests and/or probe).
- Non-Linux hosts return `NotReadyOnThisHost` for the composition plan.
- Evidence under `evidence/052-linux-landlock-composition/`.

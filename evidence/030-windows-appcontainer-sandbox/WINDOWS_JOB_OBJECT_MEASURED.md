# Windows Job Object measured evidence (Spec 030)

**Qualification:** `OsSandboxQualification::ReadyBaseMeasured`  
**Target:** `OsSandboxTarget::WindowsAppContainerJobObject`  
**Mechanism measured:** Job Object `JOB_OBJECT_LIMIT_ACTIVE_PROCESS = 1` (+ `KILL_ON_JOB_CLOSE`)

## Code

- Apply: `medscale_contracts::os_sandbox::windows_job::apply_job_object_windows`
- Plan: `OsSandboxPlan::windows_job_object_ready_base`
- Probe: `medscale-os-sandbox-probe` (`crates/medscale-contracts/src/bin/os_sandbox_probe.rs`)
- Test: `os_sandbox_030::job_object_measured_denies_child_process` (`#[cfg(windows)]`)

## Measurement procedure

1. Spawn probe as a dedicated child process (avoids poisoning the cargo test runner).
2. Probe calls `try_apply_os_sandbox` with ReadyBaseMeasured Windows plan.
3. Probe attempts `cmd.exe /C exit 0`.
4. Spawn must fail (child creation denied). Probe exits 0 on deny, non-zero otherwise.

Runs without administrator rights. Does **not** measure AppContainer FS/network isolation.

## Platform matrix

| OS | Status | Qualification |
|---|---|---|
| Windows | measured (Job Object process-limit) | ReadyBaseMeasured |
| Linux | N/A for this plan (`NotReadyOnThisHost`); Landlock separate (026) | — |
| macOS | Seatbelt scaffold only | NotPlatformQualified |

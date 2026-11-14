# Spec 053 summary - Linux seccomp composition READY_BASE

- `LinuxSeccompComposition` ReadyBaseMeasured plan (`seccomp_bpf_strict_allowlist`,
  `sigsys_deny_child_measured`, `no_new_privs`); never PlatformQualified.
- Child-measured: probe child installs strict allowlist (read/write/exit/
  exit_group/rt_sigreturn), proves allowed write, dies by SIGSYS on denied
  `socket()`; parent asserts marker + signal 31.
- Doctor `linux_seccomp_composition_measured=true`; inventory 9 axes;
  `platform_qualified=false`; WORKER_OS_SANDBOX_PLATFORM_QUALIFIED stays OPEN.
- Non-Linux / non-x86_64/aarch64: honest NotReadyOnThisHost.
- Tests: contracts `os_sandbox_053` (incl. live SIGSYS measure on Linux),
  core `os_sandbox_053` (doctor honesty + evidence presence).
- Files: `crates/medscale-contracts/src/os_sandbox/linux_seccomp.rs`,
  probe `seccomp-child`/`seccomp-composition` modes,
  `docs/planning/SPEC_053_PROMOTION.md`.

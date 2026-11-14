# Contract: Linux Landlock composition

- `OsSandboxPlan::linux_landlock_composition_ready_base(allow_paths)` exists.
- `try_apply_os_sandbox` on that plan applies FS+net+rlimit on Linux.
- Doctor `linux_landlock_composition_measured == true`.
- `platform_qualified == false`.

# LINUX_LANDLOCK_COMPOSITION_MEASURED

Spec 052 ReadyBaseMeasured composition on Linux:

1. **FS** — Landlock path-beneath allowlist (outside paths denied).
2. **Network** — Landlock `AccessNet::{BindTcp,ConnectTcp}` handled with no `NetPort` allow rules (TCP denied when ABI V4+ available).
3. **rlimit** — `RLIMIT_NOFILE` soft/hard lowered via `setrlimit`.

Evidence is exercised by:

- `crates/medscale-contracts/tests/os_sandbox_052.rs` (`cfg(target_os = "linux")`)
- `medscale-os-sandbox-probe landlock-composition`

Does **not** claim multi-OS `platform_qualified`.

# macOS Seatbelt measured READY_BASE (Spec 031)

## Mechanism

- API: Apple `sandbox_init(3)` / `sandbox_free_error` (libSystem)
- Apply: `medscale_contracts::os_sandbox::macos_seatbelt::apply_seatbelt_macos` (via `try_apply_os_sandbox`)
- Profile (SBPL, flags=0):

```text
(version 1)
(allow default)
(deny network*)
```

- Probe: `medscale-os-sandbox-probe` (`crates/medscale-contracts/src/bin/os_sandbox_probe.rs`)
- Test: `os_sandbox_031::seatbelt_measured_denies_network` (`#[cfg(target_os = "macos")]`)

## Measured sequence

1. Probe process starts unsandboxed.
2. Probe calls `try_apply_os_sandbox` with ReadyBaseMeasured macOS plan.
3. Probe attempts `TcpStream::connect_timeout` to `127.0.0.1:9`.
4. **PASS** only if the error is PermissionDenied / EPERM / "Operation not permitted".
5. **FAIL** if connect succeeds or returns ConnectionRefused (network stack still ambient).

## CI

Proven on GitHub Actions `macos-latest` (workspace matrix). Local Windows/Linux builds compile stubs and assert `NotReadyOnThisHost` for the macOS ReadyBaseMeasured plan.

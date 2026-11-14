# Limitations — Spec 033 Windows AppContainer FS

- Measures **child** AppContainer filesystem deny for a planted host-temp marker — not parent-process Landlock-style allowlists.
- Does **not** measure AppContainer **network** isolation or LPAC capability matrices.
- Does **not** clear EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED`.
- `platform_qualified=false`; `release_ready=false`; no PRIVATE_DATA_READY / MULTI_CLIENT claims.
- Requires `medscale-os-sandbox-probe` helper for CreateProcess child.
- Profile create/delete is best-effort cleanup; leftover profiles on crash are non-PHI synthetic names only.
- `unsafe` Win32 FFI confined to `windows_appcontainer` (`#![allow(unsafe_code)]`).

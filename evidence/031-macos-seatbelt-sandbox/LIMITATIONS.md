# Limitations — Spec 031 macOS Seatbelt sandbox READY_BASE

- macOS Seatbelt READY_BASE measures **network ambient deny** via `sandbox_init` SBPL `(deny network*)`, not filesystem allowlists or App Sandbox container isolation.
- **`sandbox_init` is deprecated** in Apple headers but still ships in libSystem; it is the strongest in-process Seatbelt apply available without entitlement-bearing app bundles.
- **App Sandbox entitlements** / container FS / XPC patterns remain **scaffold** (`NotPlatformQualified`).
- Seatbelt network-deny is **not** equivalent to Linux Landlock path-beneath or Windows AppContainer LPAC.
- Windows **AppContainer FS/network** isolation remains scaffold after Spec 030.
- `platform_qualified=false`; EXTERNAL_GATES `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` remains **OPEN**.
- `release_ready=false`; no PRIVATE_DATA_READY / MULTI_CLIENT claims.
- Seatbelt apply is irreversible for the process lifetime — measurement uses a dedicated probe subprocess.
- `unsafe` libSystem FFI is confined to `macos_seatbelt` (`#![allow(unsafe_code)]`); workspace lint is `deny` (not `forbid`).

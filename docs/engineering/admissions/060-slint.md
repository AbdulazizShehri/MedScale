# Admission 060 — Slint Native UI Toolkit

**Decision:** ADMITTED for `medscale-desktop` product UI.
**Version line:** `1.16.1`, pinned exactly in Cargo.
**Workspace MSRV:** Rust `1.88`.
**Reason:** native, non-WebView, Windows/macOS/Linux, declarative UI, OS accessibility integration, and a GPU renderer graph that passes the repository's fail-closed security policy.

## Security-driven version selection
The first candidate, Slint 1.13.1 + femtovg 0.17, failed exact-head `cargo-deny` because it selected `lru 0.16.4` (`RUSTSEC-2026-0253`, unsound use-after-free). MedScale did not ignore the advisory. Slint 1.16.1 raises the workspace MSRV from 1.85 to 1.88 and selects femtovg 0.23.2 with Slint's `swash` path; the active all-features/all-target graph contains no `lru` package. This preserves the lower-RSS GPU path without weakening security policy. The lockfile additionally pins `smol_str 0.3.2` and `typed-index-collections 3.3.0`; newer semver-compatible releases exceed the declared Rust 1.88 MSRV, while the pinned versions pass a real Rust 1.88 all-target workspace check.

## License
Slint is offered under multiple licenses. MedScale uses the royalty-free Desktop/Mobile/Web application license path and retains the required Slint attribution in the native About surface. MedScale remains Apache-2.0; this admission does not relicense MedScale source.

## Security / authority boundary
- Slint is presentation only.
- UI code does not open MedScale canonical storage, keys, network clients, or partner endpoints directly.
- All authority-changing operations remain behind typed Core Host/facade boundaries.
- Tauri/WebView remains absent.

## Cargo-deny scoped transitive exceptions
The Slint cross-platform/build graph requires narrowly scoped licenses that are not added to MedScale's global license allowlist:
- `clipboard-win@5.4.1` — `BSL-1.0`;
- `error-code@3.4.0` — `BSL-1.0`;
- `libfuzzer-sys@0.4.13` — `NCSA`, build-time path through `i-slint-compiler -> image -> ravif`.

`deny.toml` permits these licenses only for the exact crates above. It does not globally allow BSL-1.0 or NCSA.

## Linux build prerequisite
Slint 1.16.1's native Linux font stack uses `fontconfig` through `yeslogic-fontconfig-sys`. The required CI matrix therefore installs `libfontconfig1-dev` on Ubuntu before Rust compilation. This is a build prerequisite for the native Desktop target, not a WebView/browser dependency.

## Update policy
Stay pinned to 1.16.1 until a later dependency/release qualification unit admits another Slint line and proves its MSRV, licenses, advisories, accessibility path, and renderer behavior.

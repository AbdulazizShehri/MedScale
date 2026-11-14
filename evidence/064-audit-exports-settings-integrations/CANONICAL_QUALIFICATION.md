# Canonical Qualification — Spec 064

Date: 2026-09-15

Spec 064 is `CLOSED_CANONICAL` based on repository and GitHub evidence, not on a release-readiness inference.

- Final exact head: `086891bf3dd2cc997d40ed61a06ea6e0e90f08fb`.
- Exact-head pull-request CI: run `34924373669`, all six required jobs passed.
- Duplicate push CI on the same exact head: run `34924370978`, all six required jobs passed.
- PR: `#108`, merged normally without bypass.
- Merge commit: `eb8eec1e6643224430f60a607a519413dbcb8b57`.
- Merged at: `2026-09-15T03:27:37Z`.
- Post-merge main CI: run `34925062638`, all six required jobs passed on the merge commit.
- Required job identities proven: `rust (ubuntu-latest)`, `rust (windows-latest)`, `rust (macos-latest)`, `perf delivery-plan scale (windows)`, `cargo-deny`, and `supply-chain policy present`.

Canonical local qualification passed formatting, Spec 064 regression 4/4, Desktop 14/14 unit tests, runtime-performance coverage 3/3, workspace Clippy with `-D warnings`, Rust 1.88 workspace/all-target checking, Desktop smoke/perf probes, and diff checking.

## Selected artifact bindings
Exact-head run `34924373669`:
- `portable-release-windows-latest`: `sha256:b09fcf6436a09ef2e00c086c079ce05d885c7df5b8ec04de635d7947d29da325`.
- `portable-release-ubuntu-latest`: `sha256:4ae00aa1c582fa348c279939051e121aab25c2b5207c4cfcd37e33c2d0baaaf3`.
- `portable-release-macos-latest`: `sha256:434288bb5c95eb9c2d32a9e4f939dffa5b443ff1dd438b6adaec7074d1d6cf83`.
- `perf-delivery-plan-scale`: `sha256:940150a22afcdc96af12bb6adab5cacf3dd290742ef3d33843fc919b07ba7f7e`.

Post-merge run `34925062638`:
- `portable-release-windows-latest`: `sha256:5fecbb7d4a549097a3b63895fc0b9ce202c9b5fcd7bed5a2f1f606c08cb57ad2`.
- `portable-release-ubuntu-latest`: `sha256:470d65057bcd806cbaf7be8f68b4a3a5836f11ec123672bd17673478b5a70431`.
- `portable-release-macos-latest`: `sha256:2190427da0a095a9c7e213e49f680d656360a8934ec3a11be80054d05ebf38b6`.
- `perf-delivery-plan-scale`: `sha256:ec737ae7dce72bfe333f193a6b03c130457e25f28d1911d1e7803c7f5f7d5df3`.

This evidence closes only Spec 064. It does not establish `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, production signing/notarization, qualified-hardware performance attainment, WCAG conformance, real-PHI authorization, or MESC admission.

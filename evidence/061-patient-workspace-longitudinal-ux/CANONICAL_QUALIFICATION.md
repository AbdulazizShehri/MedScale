# Canonical Qualification — Spec 061

State: `CLOSED_CANONICAL`
Date: 2026-09-15

## Exact-head qualification
- PR: #105, `Desktop: add longitudinal patient workspace`.
- Final implementation head: `ee9e076fd39d4832c2037b077af18979e428b420`.
- Pull-request CI run: `34916956439` — all six required jobs `SUCCESS`.
- Duplicate push CI run: `34916952740` — all six required jobs `SUCCESS`.
- Merge: normal protected-branch merge, no bypass, merge commit `1438460e1b965518a9cd09d4289e842c016d83df`.

## Post-merge main verification
- Main CI run: `34917999834` on `1438460e1b965518a9cd09d4289e842c016d83df` — all six required jobs `SUCCESS`.
- Required jobs: Ubuntu/macOS/Windows Rust, Windows delivery-plan performance harness, `cargo-deny`, and supply-chain policy presence.
- Portable release package and runtime-performance evidence uploads succeeded on Windows, macOS, and Linux.

## Selected artifact bindings
Exact-head run `34916956439`:
- `portable-release-windows-latest`: `sha256:a54c543ff05c574d54ddf2712e8e984917c4a71adf2cf0f4c48e4c53de3fd66a`.
- `portable-release-ubuntu-latest`: `sha256:d94fc4589a81e690019664d0fc9b1a57933205c71cbce6c20131f22c42b55ad7`.
- `portable-release-macos-latest`: `sha256:260f0aa358b26c5345f51335b90db25a4de500dc8ce8fc97f54f989a99776dc2`.
- `perf-delivery-plan-scale`: `sha256:446e6a47d85cadc5d3cf4cdc522859bf1f10e25d5e0ea73709dc0b56e1c5eeb4`.

Post-merge run `34917999834`:
- `portable-release-windows-latest`: `sha256:8eb231c3f610c209086d20578a45d0db14baae984d3f2b47a0360dfc17afdc52`.
- `portable-release-ubuntu-latest`: `sha256:309610ec839447da006c2414ceb3be0869c5114d1bbd37c59e55e4bd927d2ce8`.
- `portable-release-macos-latest`: `sha256:c4ed548aed3eb26e58617f12313d5eedf518cc6b37961215e6c0d08decd135b0`.
- `perf-delivery-plan-scale`: `sha256:8b8b09b97690e2d4ac74a378e2be797d5b43a84f2b35e0f6cc48555684537486`.

## Non-claims retained
This qualification closes Spec 061 only. It does not establish `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, real-PHI authorization, qualified-hardware performance attainment, production signing/notarization, or WCAG conformance. Spec 012/MESC remains optional/deferred and untouched.

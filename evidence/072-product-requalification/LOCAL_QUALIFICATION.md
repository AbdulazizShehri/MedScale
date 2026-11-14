# Spec 072 Local Qualification

## Candidate binding

- Branch: `spec/072-product-requalification`
- Canonical base: `e2031bdef918980cc184aed811f381e41f5dfb8b`
- Qualified source commit: `96beb0fd86dc363f551f008f8898005330198d4b`
- Qualified source tree: `70253b907ad1b0e7fc424f6906b24d86488c3b18`
- Default toolchain: `rustc 1.97.1` / `cargo 1.97.1`
- MSRV qualification toolchain: `rustc 1.88.0` / `cargo 1.88.0`

## Final results

- `cargo fmt --all -- --check` — PASS.
- `git diff --check` — PASS.
- `cargo test -p medscale-core --test product_requalification_072 --locked` — PASS (`5/5`).
- Spec 059 release-closure regression — PASS (`2/2`).
- Spec 064 utility-surface regression — PASS (`4/4`) after removing the stale optional-MESC product assertion.
- Spec 067 final-UI regression — PASS (`5/5`).
- Spec 068 product-differentiation regression — PASS (`5/5`).
- Spec 069 local-runtime regression — PASS (`6/6`).
- Historical MESC interoperability compatibility remains fail-closed: Spec 012 admit (`3/3`), boundary (`1/1`), decoupling/backward decode (`5/5`), and Spec 036 verifier (`5/5`) all pass while the current MedScale doctor state is `SEPARATE_PROJECT / OUT_OF_SCOPE / gate=NONE`.
- `cargo test -p medscale-desktop --locked` — PASS (`23/23` unit tests; `3/3` runtime-performance tests).
- Native Desktop smoke — PASS (`medscale-desktop smoke ok`, `tauri_admitted=false`, `desktop_shell=slint_native`).
- Native Desktop bounded perf-idle probe — PASS (`medscale-desktop perf-idle-ready`).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS on the final corrected source tree.
- `cargo +1.88.0 check --workspace --all-targets --locked` — PASS on the final corrected source tree.
- `cargo test --workspace --locked` — PASS / exit `0` on the final corrected source tree.

## Findings resolved during qualification

1. Rebuilt-product review found two stale comparative-superiority phrases on the native UI: `what OpenMed still does better` and `See where OpenMed is ahead`. They were replaced with pinned-evidence / claim-state wording, and Spec 072 regresses against their return and against unmanifested `PROVEN ADVANTAGE`, `OPENMED AHEAD`, `PARITY`, and `SURPASS` verdict assignments.
2. The historical Spec 068 living progression guard still required `NEXT_PROMOTED_SPEC = 071`; it was advanced to require Spec 072 and reject Spec 073 while 072 is active.
3. The release external-gate pointers for qualified-hardware performance and final accessibility were advanced from the Spec 067 packets to the rebuilt-product Spec 072 packets, including Models/Evidence coverage.
4. Founder correction established that `TheHalfMoon/MESC` is a separate project/repository, not a MedScale optional lane. Living MedScale governance, external-gate accounting, completion status, Doctor truth, and Desktop integrations were corrected accordingly. Historical Specs 012/036 and fail-closed compatibility contracts remain provenance/backward compatibility only. A stale Spec 064 regression that required `Optional / deferred` was updated to require that MESC is absent from the MedScale integration surface.

## Environmental events

The authorized host had very low free disk space during qualification. Initial smoke/full-workspace/MSRV attempts encountered `errno=28` / `No space left on device` during compilation or linking. These events were not treated as product failures or passes. Only rebuildable build/cache artifacts were removed. Partial target artifacts malformed by ENOSPC were rebuilt before final qualification. The final corrected tree subsequently passed workspace tests, Clippy, Rust 1.88 all-target checking, and native smoke/perf probes.

## Honesty boundary

This qualification does not claim WCAG conformance, qualified-hardware performance attainment, production signing/notarization, signed macOS product/App Sandbox qualification, private-data readiness, multi-client release readiness, real-PHI authorization, OpenMed parity/surpass, or release readiness. The release doctor continues to expose exactly five external MedScale release evidence classes. MESC is a separate project/repository and is excluded from MedScale execution, completion, release, and residual calculations.

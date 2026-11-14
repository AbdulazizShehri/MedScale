# Local Qualification — Spec 064

Date: 2026-09-15
State: `IN_REVIEW` on canonical branch `spec/064-audit-exports-settings-integrations`, based on Spec 063 merge `26bc3cfea025ebb6b7413b2017b5b4681275fa2a`.

## Results
- `cargo fmt --all -- --check`: PASS.
- `cargo test -p medscale-core --test native_utility_surfaces_064 --locked`: PASS — 4/4.
- `cargo test -p medscale-desktop --locked`: PASS — 14 Desktop unit tests plus 3 runtime-performance coverage tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS.
- `cargo +1.88.0 check --workspace --all-targets --locked`: PASS.
- `cargo +1.88.0 run -p medscale-desktop --locked -- --smoke`: PASS (`medscale-desktop smoke ok`, `tauri_admitted=false`, `desktop_shell=slint_native`).
- `cargo +1.88.0 run -p medscale-desktop --locked -- --perf-idle-ms 100`: PASS (`medscale-desktop perf-idle-ready`).
- `git -c core.whitespace=cr-at-eol diff --check`: PASS.

The host build used `CARGO_INCREMENTAL=0` and debug-info-disabled development/test profiles to remain within local disk capacity. The MSRV run used a fresh `target/` cache rather than mixing Rust toolchains. These are build-hygiene controls only and do not alter product behavior or establish performance-budget attainment.

Full workspace tests and three-OS portable-package qualification remain mandatory in exact-head CI before merge.

## Boundaries retained
- Desktop has no direct `medscale-storage` or `medscale-network` dependency.
- Audit is a presentation of admitted disclosure history, not a replacement authority store.
- FHIR export UI exposes support/loss honesty and does not claim full conformance.
- Settings do not turn toggles into evidence: `REAL_PHI`, `PRIVATE_DATA_READY`, `RELEASE_READY`, and WCAG conformance remain false/unqualified.
- Integrations retain Network Broker default-deny and NPHIES external-gate posture.
- Spec 012/MESC remains optional/deferred and untouched.

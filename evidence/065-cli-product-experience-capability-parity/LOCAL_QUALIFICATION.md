# Local Qualification — Spec 065

Date: 2026-09-15
State: `IN_REVIEW` on canonical branch `spec/065-cli-product-experience-capability-parity`, based on Spec 064 merge `eb8eec1e6643224430f60a607a519413dbcb8b57`.

## Results
- `cargo fmt --all -- --check`: PASS.
- `cargo test -p medscale-core --test cli_product_experience_065 --locked`: PASS — 3/3.
- `cargo test -p medscale-cli --locked`: PASS — 9/9.
- `cargo clippy -p medscale-cli --all-targets --locked -- -D warnings`: PASS.
- `cargo +1.88.0 check --workspace --all-targets --locked`: PASS.
- `git -c core.whitespace=cr-at-eol diff --check`: PASS.

The canonical rerun used `CARGO_INCREMENTAL=0` and debug-info-disabled development/test profiles, with the target cache removed between native-toolchain tests and the Rust 1.88 workspace check. These are development-host build-hygiene controls only and do not alter product behavior or establish performance attainment.

An earlier candidate regression assertion was too sensitive to Markdown backticks around `REAL_PHI` and `UNKNOWN`; it was corrected to validate semantic phrases independently from Markdown formatting. The canonical branch rerun passed without weakening authority behavior.

## Boundaries retained
- CLI remains a `CliSession` / `CoreFacade` client with no direct storage, key, SQL, or network dependency.
- New outbox/disclosure/FHIR paths are read-only and reuse existing typed authority requests.
- Population Insights and Workflow Studio remain presentation-specific rather than gaining duplicate CLI authority.
- Transient CLI host behavior is documented; Host IPC remains the explicit persistent-host operator path.
- Real PHI stdout, production credentials, full FHIR conformance, release readiness, private-data readiness, and WCAG conformance remain unclaimed.
- Spec 012/MESC remains optional/deferred and untouched.

Full workspace tests and three-OS portable-package qualification remain mandatory in exact-head CI before merge.

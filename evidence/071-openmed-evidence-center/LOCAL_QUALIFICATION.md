# Spec 071 Local Qualification

## Binding

- Branch: `spec/071-openmed-evidence-center`
- Canonical merge base: `129b63d0d93e8fc14fa0dcd786d95b1ac4697e68`
- Spec-promotion commit before implementation: `5897809e563b0386ff1a1d9bc2259d433e701af0`
- OpenMed baseline tag: `v2.2.0`
- OpenMed baseline commit: `59d9cb0a2e0ccbba8fa3d891a66d83ffaf45e837`
- OpenMed baseline tree: `1c949e35b2b8f2ea69da4284b370074fc4bf84ab`

## Comparative claim discipline

The committed claim ledger accounts for all 39 canonical comparison rows: 22 `UNMEASURED`, 10 `WAIVED`, 6 `STRUCTURAL_DIFFERENTIATION_ONLY`, and 1 `PATTERN_ABSORPTION_ONLY`. The ledger records zero matched `BenchmarkManifest` artifacts. Product presentation therefore refuses parity, surpass, privacy-superiority, clinical-quality superiority, and runtime-winner claims.

Runtime validation fails closed unless the ledger has exactly 39 unique capability IDs, the exact pinned OpenMed baseline commit and tree, row-level baseline binding, zero unauthorized parity/surpass claims, and the expected claim-state vocabulary. A duplicate-capability regression proves that 39 repeated rows cannot satisfy completeness.

## Final local gates

- `cargo fmt --all -- --check` — PASS.
- `git diff --check` — PASS.
- `cargo test --workspace --locked` — PASS on the final implementation tree. The pinned external Hugging Face execution test remains intentionally ignored because model weights are not vendored; the already-qualified Spec 069 external evidence is not reinterpreted by Spec 071.
- Desktop unit tests — 23 passed, 0 failed.
- Desktop runtime perf tests — 3 passed, 0 failed.
- `medscale-desktop --smoke` — PASS with `medscale-desktop smoke ok`, `tauri_admitted=false`, and `desktop_shell=slint_native`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS on the final implementation tree.
- `cargo +1.88.0 check --workspace --all-targets --locked` — PASS on the final implementation tree.

## Findings resolved during qualification

1. Initial Desktop test compilation exposed missing test-module imports for the embedded ledger constants/helper. Imports were corrected; the affected tests then passed.
2. Full-workspace qualification exposed a Spec 066 contrast guard because limitation text used `Theme.warning` as primary text. The text was moved to `Theme.ink-subtle`; the Spec 066 regression returned to green.
3. Full-workspace qualification exposed a stale Spec 068 canonical-progression assertion expecting Spec 070. The guard now requires active Spec 071 and explicitly rejects premature Spec 072 promotion.
4. Manual review found that row-count validation alone could accept duplicate capability IDs. The validator now requires unique non-empty capability IDs, exact baseline tree binding, and exact row-level baseline commit binding; a duplicate-ledger regression fails closed.
5. Two local build attempts encountered host disk exhaustion while linking. Only temporary Cargo target directories were removed; source/evidence history was not rewritten. Final full-workspace, Clippy, and Rust 1.88 gates subsequently passed from clean temporary targets.

## Non-claims

This qualification does not authorize real PHI, production clinical model promotion, online Hugging Face acquisition, an accelerated-runtime winner, OpenMed parity/surpass claims, WCAG conformance, release readiness, or MESC mutation. Spec 012/MESC remains optional, deferred, and non-blocking.

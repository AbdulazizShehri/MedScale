# Spec 073 Local Qualification

## Candidate basis

- Branch: `spec/073-final-ui-polish`
- Canonical base verified during qualification: `2b973083cd72de61ee35a5219b9a57daa8297f16`
- Initial Spec 073 promotion head: `90cb055b0397e28a7dc94f6c2952817024c78d66`
- Qualification host: authorized macOS development host.
- MSRV toolchain: Rust/Cargo `1.88.0`.
- Final commit identity is intentionally not predeclared here; GitHub exact-head CI and the PR record bind the immutable candidate after commit/push.

## Final local gates

- `cargo fmt --all -- --check` — PASS.
- `git diff --check` — PASS.
- `cargo test -p medscale-core --test final_ui_polish_073 --locked` — PASS (`6/6`).
- Spec 068 compatibility regression — PASS (`5/5`) after rebinding historical guards to the current semantic identity instead of removed Spec 068 kickers.
- Spec 066 hardening compatibility regression — PASS (`5/5`) with adaptive Light/Dark engineering contrast floors.
- Spec 060 native-shell compatibility regression — PASS (`3/3`) with the current focus token, Instrument Sans identity, and signature mark.
- Spec 062 Insights accessibility compatibility regression — PASS (`4/4`) with `AdaptiveLineEdit` accessibility-name propagation.
- `cargo test -p medscale-desktop --locked` — PASS (`23/23` unit tests; `3/3` runtime-performance tests).
- `cargo test -p medscale-cli --locked` — PASS (`10/10`).
- Native Desktop smoke — PASS (`medscale-desktop smoke ok`, `tauri_admitted=false`, `desktop_shell=slint_native`).
- Native bounded perf-idle probe — PASS (`medscale-desktop perf-idle-ready`).
- Impeccable Web-reference detector — PASS (`0 findings`).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS.
- `cargo +1.88.0 check --workspace --all-targets --locked` — PASS.
- `cargo test --workspace --locked` — PASS / exit `0` after all stale historical compatibility guards were corrected.
- Final native rendered review — PASS; see `RENDERED_EVIDENCE.md`.

## Qualification-discovered repairs

1. Historical Spec 068 tests pinned removed UI kickers and the superseded mark-status string. They now protect Models/Evidence visibility, clinical-workspace hierarchy, the current founder-approved signature mark, and the live Spec 073 progression truth.
2. Historical Spec 066 pinned the previous palette. The guard now evaluates current adaptive tokens. This exposed a real Light-mode `ink-quiet` contrast deficit, which was corrected from `#737A76` to `#646B67`.
3. Historical Spec 060 pinned a literal focus-width expression, Geist, and the Spec 068 blue mark. It now protects the reusable focus token, native shell, current monochrome signature mark, and documented Impeccable disciplines.
4. Historical Spec 062 pinned a direct TextInput accessibility-label syntax. It now verifies `AdaptiveLineEdit`'s public accessibility name and the component's propagation to the native input label.
5. A performance-evidence file was mutated by local workspace-test execution. That generated change was explicitly reverted and is not part of Spec 073.
6. A one-time non-check `cargo fmt` converted the CLI file's canonical CRLF line endings. The intended 22-line CLI identity change was preserved while the canonical CRLF convention was restored, eliminating thousands of lines of false diff noise.

## Independent re-verification (execution handoff, same candidate tree)

Every gate above was re-run against the unchanged candidate tree before freeze: `fmt` PASS, `git diff --check` PASS, Spec 073 `6/6`, Spec 068 `5/5`, Spec 066 `5/5`, Spec 060 `3/3`, Spec 062 `4/4`, desktop unit `23/23`, CLI `10/10` (with `TMPDIR=/tmp`), desktop build/smoke/perf-idle PASS, Impeccable `0 findings`, Clippy PASS, Rust 1.88 PASS. Exact-range substantive review found no material findings: historical guards track live semantics, CLI diff holds only human-output identity with CRLF preserved, Web reference stays `NON_PRODUCTION_REFERENCE`, MESC stays separate, and no release/PHI/WCAG claim is introduced. Two environment-only notes, both proven unrelated to the tree: the `ps`-based RSS probe and the Seatbelt `sandbox_init` probe are denied by the executing tool sandbox (`Operation not permitted`; escalated execution unavailable), and both passed in the exact-tree logs on this same tree; the CLI wedge needs a writable temp dir. `perf_harness_latest.json` generated churn was reverted and is not staged.

## Honesty boundary

This qualification establishes repository-owned Spec 073 implementation quality only. It does not claim `RELEASE_READY`, `PRIVATE_DATA_READY`, `MULTI_CLIENT_RELEASE_READY`, `REAL_PHI_AUTHORIZED`, WCAG conformance, signed/notarized product qualification, final App Sandbox enforcement, qualified-hardware performance attainment, or any MESC status. MESC is a separate project/repository and is excluded from MedScale completion and release calculations.

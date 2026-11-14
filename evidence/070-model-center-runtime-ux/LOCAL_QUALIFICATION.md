# Spec 070 Local Qualification

## Scope

Candidate base: `99017b9b22400e5b6d35c217bea02c0602c5649e` (Spec 069 protected merge on `main`).

This qualification covers the Model Center runtime/operator UX implementation before exact-range review and GitHub exact-head CI. It does not claim canonical closure, release readiness, real-PHI authorization, Pack restart persistence, accelerated-runtime qualification, or production clinical-model promotion.

## Proven local gates

- `cargo fmt --all -- --check` — PASS.
- `git diff --check` — PASS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS on the candidate implementation before the final startup-honesty refinement.
- `cargo test --workspace --locked` — PASS / exit 0 on the candidate implementation; the pinned external Hugging Face test remains intentionally ignored because model weights are not vendored.
- Final affected-scope `cargo clippy -p medscale-desktop --all-targets --locked -- -D warnings` — PASS after the startup-honesty refinement.
- Final affected-scope `cargo test -p medscale-desktop --locked` — PASS after final review fixes: 20 unit tests and 3 runtime-performance tests.
- `cargo run -p medscale-desktop --locked -- --smoke` — PASS. Smoke mode exits before opening a Model Center Core session.
- `cargo +1.88.0 check --workspace --all-targets --locked` — PASS.
- Final affected-scope `cargo +1.88.0 check -p medscale-desktop --all-targets --locked` — PASS on review head `49e8fc122910ceaca3bc7c8917a5426aaa406397`.

## Authority and honesty regressions

- Model Center uses `CliSession::connect_pack_operator`, limited to `PacksList` and `PacksInstallLocal`.
- A regression proves the Model Center session is denied `OpenSyntheticVault` with `SessionDenied`.
- Desktop has no direct `medscale-pack` dependency.
- Local Pack input requires an absolute path before Core admission. Pack canonicalization, signature, digest, artifact-kind, path, symlink and byte bounds remain Core/Pack responsibilities.
- Session inventory and pinned qualification references are rendered as different scopes. A qualification reference is never labeled installed/admitted.
- Empty session inventory is explicit, and restart persistence is not claimed.
- Startup and refresh failures are surfaced in operator status rather than silently represented as successful refresh.
- A regression proves the local Pack admission helper releases its `RefCell` mutable borrow before inventory refresh, preventing nested-borrow panic on the success path.

## Exact-head CI portability correction

The first PR exact-head Windows run exposed a Unix-only test fixture assumption: `/tmp/signed-pack` is not an absolute Windows path. Production validation remained correct. The regression now uses `std::env::temp_dir()` to construct a platform-native absolute path before applying the same trim/absolute-path assertions. The portability correction code head is `e8cf4f2d0c91d7717bb83869a65ee765990be071`. A new exact-head CI run is required; the failed head is not reusable.

## Environment note

One intermediate local link attempt failed with `No space left on device` / mixed native build-cache warnings. Only temporary MedScale build targets were removed. A clean rebuild with reduced debug/incremental artifacts then passed Desktop tests and the full workspace suite. The environment failure is not counted as a source-code failure.

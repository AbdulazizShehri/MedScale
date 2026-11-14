# Spec 070 OpenCodeReview Record

**Tool**: Alibaba OpenCodeReview `v1.12.3` (`71ed3a288d`)
**Mode**: exact-range preview plus manual semantic/security/authority review
**Base**: `99017b9b22400e5b6d35c217bea02c0602c5649e`
**Final review head**: `49e8fc122910ceaca3bc7c8917a5426aaa406397`

## Exact-range coverage

The exact-range preview reported 14 changed files, including 4 reviewable Rust files and 10 files excluded by extension/default filtering. The reviewable Rust set was `crates/medscale-core/src/cli_session.rs`, `crates/medscale-core/tests/product_differentiation_068.rs`, `crates/medscale-desktop/src/main.rs`, and `crates/medscale-desktop/src/product_intelligence.rs`.

Unsupported Slint, Markdown, planning, task and evidence files were reviewed manually for UI truth, canonical-state consistency, authority boundaries, and claim honesty.

## Material findings resolved

1. **Startup inventory failure could be represented too optimistically.** The Desktop startup path now surfaces `Core inventory unavailable at startup` when `PacksList` fails instead of leaving only a successful-session status while falling back to the truthful empty projection.
2. **Pack v0 signature scope could be overstated in UI copy.** Model Center now states that Pack v0 signing binds identity/version/epoch, content digest, rights URI and SBOM reference, while runtime requirements, benchmark links and promotion state are declarations/Core state rather than signature authority. Generic admitted rows label runtime data as `Declared`.
3. **Successful local Pack admission could retain a `RefCell` mutable borrow across refresh.** Admission now executes through a helper whose mutable borrow ends before the success arm refreshes inventory. A regression proves a second mutable borrow succeeds immediately after admission.
4. **The absolute-path unit test encoded a Unix-only `/tmp/...` assumption.** Exact-head Windows CI correctly rejected that path as non-absolute. The regression now derives an absolute path from `std::env::temp_dir()`, preserving the production `Path::is_absolute()` contract across supported platforms.

## Final regression evidence

```text
cargo fmt --all -- --check: PASS
git diff <base>..HEAD --check: PASS
cargo clippy --workspace --all-targets --locked -- -D warnings: PASS on candidate
cargo test --workspace --locked: PASS / exit 0 on candidate
cargo clippy -p medscale-desktop --all-targets --locked -- -D warnings: PASS after review fixes
cargo test -p medscale-desktop --locked: 20 passed; runtime_perf_057: 3 passed
cargo +1.88.0 check --workspace --all-targets --locked: PASS on candidate
cargo +1.88.0 check -p medscale-desktop --all-targets --locked: PASS on final review head
```

The pinned external Hugging Face runtime test remains intentionally ignored in ordinary workspace runs because external model weights are not vendored; Spec 069 already qualified that exact external model path separately.

## Security and authority sweep

No new `unsafe` block, shell/process execution path, network acquisition path, Hugging Face SDK dependency, direct Desktop `medscale-pack` dependency, secret handling, or real-PHI authority was introduced. Model Center uses a least-privilege Core session limited to `PacksList` and `PacksInstallLocal`; a regression proves `OpenSyntheticVault` is denied with `SessionDenied`. Absolute-path prevalidation is presentation ergonomics only; canonicalization, signature/digest verification, artifact-path/symlink containment, byte bounds and admission remain Core/Pack responsibilities.

**Final review verdict**: `NO_MATERIAL_FINDINGS_REMAIN` for the implementation through portability-correction code head `e8cf4f2d0c91d7717bb83869a65ee765990be071`. The following evidence-only commit does not change reviewed runtime behavior.

Protected exact-head GitHub CI, protected merge, post-main verification and canonical queue promotion remain required before Spec 070 closure.

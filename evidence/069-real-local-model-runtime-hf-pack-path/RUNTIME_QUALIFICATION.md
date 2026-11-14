# Spec 069 Runtime Qualification

## Deterministic in-repository fixture

The repository contains a tiny signed ONNX token-classification Pack used by ordinary CI. It proves that model bytes execute through tract, output remains evidence-only, unsafe artifact paths fail closed, and a post-admission artifact swap is rejected. Final hardening also proves Pack admission rejects parent traversal before artifact reads, rejects symlink escapes that resolve outside the canonical Pack root even when the external bytes match an admitted digest, and rejects an oversized sparse ONNX artifact before model loading. The direct runtime independently repeats path containment and byte-bound enforcement.

Focused result after the final provenance/runtime schema changes:

```text
onnx_runtime_069: 9 passed
real_local_model_runtime_069: 6 passed
```

The Core integration also proves Strict session enforcement, explicit denial of real-PHI model execution, and that Desktop/CLI do not own `tract-onnx`, `tokenizers`, or `medscale-pack` runtime dependencies.

## Prepared-model behavior

Core re-admits current Pack bytes and verifies signed content identity on every evaluation request. Prepared execution plans remain indexed by `content_digest` only, but reuse additionally requires an exact runtime-contract binding over `runtime_requirements` plus the artifact path/kind/digest structure. User input is not cached. A same-digest Pack with a different runtime contract is re-prepared instead of receiving a cache hit, and alias Pack identity is supplied per invocation rather than retained in the prepared plan.

Artifact ceilings are explicit and enforced before execution: 1 MiB for `pack.manifest.json`, 1 GiB for ONNX model bytes, 64 MiB for tokenizer/fixture bytes, and 1 MiB for model metadata. Admission bounds the manifest before JSON parsing. Runtime preparation consumes the exact verified model/tokenizer bytes rather than reopening those files after verification. The optimized tract model must also expose exactly one static `f32` output with shape `[1, fixed_sequence_length, labels]` before a runnable plan is created, so an out-of-contract output shape is refused before inference execution.
## Final local candidate qualification — 2026-09-16

The final pre-PR tree was qualified from base `75b5a173a63ed2057c6ecf96ef12faf18ed5e479`. Exact commit identity is intentionally bound only after the candidate is committed; protected exact-head GitHub CI remains required before canonical closure.

```text
cargo fmt --all -- --check                                      PASS
git diff --check                                               PASS
cargo test -p medscale-pack --test onnx_runtime_069 --locked  9 passed
cargo test -p medscale-core --test real_local_model_runtime_069 --locked  6 passed
cargo test -p medscale-core --test product_differentiation_068 --locked   5 passed
Desktop product-intelligence truth unit                        1 passed
cargo clippy --workspace --all-targets --locked -- -D warnings PASS
cargo test --workspace --locked                                PASS
cargo +1.88.0 check --workspace --all-targets --locked         PASS
cargo check --release -p medscale-pack --example sign_hf_pack_069 --locked PASS
external pinned HF release execution                         PASS
fresh source -> transform -> sign -> semantic execution      PASS
```

The Rust 1.88 pass depends on the intentional `Cargo.lock` selection `kstring = 2.0.2`; an initial `2.0.4` resolution declared Rust 1.96 and was rejected rather than raising MedScale's MSRV.

For disk-bounded local builds, `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0` were used on the large workspace Clippy/test/check runs. These settings only reduce local build-artifact size; they do not change features, test selection, source, or runtime authority.

`cargo-deny` is not installed on this local host. No local cargo-deny PASS is claimed. The protected GitHub `cargo-deny` context remains authoritative for the exact candidate head.

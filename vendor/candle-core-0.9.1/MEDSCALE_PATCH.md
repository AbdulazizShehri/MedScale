# candle-core 0.9.1, MedScale security backport

This directory is the `candle-core` 0.9.1 crate as published on crates.io,
with one targeted security change and documented f32 performance changes
(items 5 and 6). It replaces the registry crate through
`[patch.crates-io]` in the workspace `Cargo.toml`.

## Provenance

- Source: `candle-core-0.9.1.crate` from crates.io, sha256
  `a9f51e2ecf6efe9737af8f993433c839f956d2b6ed4fd2dd4a7c6d8b0fa667ff`, equal to
  the checksum recorded in the workspace `Cargo.lock` before this change.
- Upstream: <https://github.com/huggingface/candle>, license
  `MIT OR Apache-2.0` (the crate ships the Apache-2.0 `LICENSE` text, kept
  unchanged here). Copyright remains with the candle authors.
- The first vendoring commit is the unmodified extraction, so the backport is
  reviewable as a plain diff against it.

## Why

RUSTSEC-2026-0367, RUSTSEC-2026-0368 and RUSTSEC-2026-0369 affect `zip`
before 9.0.0. `candle-core` 0.9.1 requires `zip ^1`, and no candle release
uses `zip >= 9` yet (0.11.0 requires `zip ^8.6`). The required `cargo-deny`
check therefore failed for every MedScale change. RustSec ignores are not
used; the vulnerable versions are removed from the dependency graph instead.

## Changes against the published crate

1. `Cargo.toml`: `zip` `1.1.1` → `9.0.3` (default features stay off).
2. `src/npy.rs` (two places) and `src/pickle.rs` (one place): ZIP 9 returns
   entry names as `Result<Cow<str>>` (`name()?`, fallible `file_names()`),
   adapted exactly as upstream huggingface/candle#4041 (head
   `70f72e9cfdbfc570846ca363ead2135394af3e9a`) does.
3. `src/lib.rs`: `#![allow(mismatched_lifetime_syntaxes)]`. As a path
   dependency the crate is not lint-capped like a registry crate, and this
   newer rustc lint is the only warning on unmodified upstream signatures under
   the workspace toolchain (`RUSTFLAGS=-Dwarnings` in CI). No behaviour change.
4. `tests/serialization_tests.rs`: the four ZIP 9 regression tests from #4041
   (UTF-8 and CP437 entry names, malformed NPZ and PTH archives) plus one
   MedScale test that traversal-style entry names are lookup keys only.

5. Performance (2026-10-19, MedScale Spec 103 latency work): `src/cpu/erf.rs`
   adds `erf_f32` and `tanh_f32`, branch-free single-precision rational
   approximations (the forms Eigen and XLA use for `float`). In `src/op.rs`
   only the **f32** CPU paths of `GeluErf` and `Gelu` use them. Before, they
   called the scalar f64 statrs `erf` and libm `tanhf`. The f64, f16 and bf16
   paths, other ops and the GPU kernels are unchanged.
   - **Accuracy:** against the f64 path over [-10, 10] and [-12, 12], GELU
     stays within 1.0e-6 absolute, or 4e-7 relative, for erf; and within
     1.5e-6 absolute, or 3e-7 relative, for tanh. Today's f32 `tanhf` path is
     within 4.7e-7. Enforced by
     `crates/medscale-pack/tests/candle_gelu_erf_accuracy.rs`.
   - **Speed** (Windows, 12 threads, 16×3072 f32): `gelu_erf` 26.3 → 1.2 ns
     per element; `gelu` (tanh) 10.7 → 1.5 ns per element.
   - **Warm encoder forward** (`crates/medscale-pack/examples/candle_latency_profile.rs`,
     random weights with real configs, 16 tokens): BERT-base 91.6 → 68.7 ms;
     DistilBERT 44.8 → 37.1 ms.
   - **Upstream:** this is an upstream-able optimisation. Re-apply it or drop it
     when this directory is removed.

6. Performance (2026-10-20): `src/cpu_backend/skinny.rs` and one dispatch in
   `src/cpu_backend/mod.rs` (`MatMul::f`, gemm build only).
   - **When it applies:** f32 matmuls with at most 64 lhs rows (after batch
     folding) whose rhs is the transposed view of a contiguous weight. This is
     the layout `Linear` produces, and a weight of at least 16 Ki elements is
     required. All other matmuls still go through gemm.
   - **Why it helps:** at that size the multiply is bound by streaming the
     weight. The kernel reads each block of four weight rows once and reuses
     it for every lhs row, instead of packing weight panels per tile.
     - It uses an AVX2+FMA 2x4 register-blocked dot kernel and only runs on
       x86_64 CPUs that have both features, checked at run time. Rayon tasks
       split the weight rows.
     - Elsewhere every matmul stays on gemm. On Apple Silicon, gemm's NEON
       kernels measured 2.4-7.7x faster than a portable version of this loop
       on the same runner (perf-ab run 39357885288).
   - **Accuracy:** within 2e-6·sqrt(k) of an f64 reference for `Linear`
     (`crates/medscale-pack/tests/candle_skinny_matmul_accuracy.rs`), plus a
     unit test in `skinny.rs`.
   - **Local Windows, 12 threads, 16 tokens:** one 768x3072 `Linear` went from
     1.06 to 0.58 ms. Warm BERT-base forward went from 68.7 to 30.5 ms, and
     DistilBERT from 37.1 to 21.0 ms.

No other file differs from the published crate. candle never calls
`zip::ZipArchive::extract`, where the three advisories apply; the backport
removes the vulnerable code from the build regardless.

## Verification (2026-10-18)

- `cargo test --tests` on this crate with `RUSTFLAGS=-Dwarnings`: every test
  binary passes, including `pth_tests` (real `.pt` archives) and
  `serialization_tests` (8 tests).
- `cargo tree -i zip`: only `zip 9.0.5`, through this crate.
- `cargo deny check` (unchanged policy): advisories pass.
- MedScale-side regression: `crates/medscale-pack/tests/zip9_backport_103.rs`
  runs in the workspace test suite on every CI platform.

## Removal

When MedScale adopts a candle release that depends on `zip >= 9`, delete this
directory and the `[patch.crates-io]` entry.

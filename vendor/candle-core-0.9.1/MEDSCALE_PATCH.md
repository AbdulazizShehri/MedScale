# candle-core 0.9.1, MedScale security backport

This directory is the `candle-core` 0.9.1 crate as published on crates.io,
with one targeted security change. It replaces the registry crate through
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

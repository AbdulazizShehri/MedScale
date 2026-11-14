# Spec 001 bootstrap evidence

**Date**: 2026-08-25  
**Branch**: `spec/001-rust-repository-speckit-bootstrap`  
**Base main**: `94ac72d411e7a7ffb62fc207ac999ac2df398ecd`  
**Platform**: Windows 10.0.26200 (`x86_64-pc-windows-msvc`)

## Toolchain

```text
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1
specify-cli 1.0.1 from git+https://github.com/github/spec-kit.git@v1.0.1
  resolved: 9118ed15a0ba65053469a94c560ea5d233f75884
cargo-deny 0.20.2
```

## Cargo.lock identity

```text
SHA256: 569D1673A80D9FAD757F26769B99CE5FFEB4BCEA8A0E7FC09F9D8400B4082B8C
```

## Commands and results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (3 unit tests) |
| `cargo run -p medscale-cli -- --version` | PASS (`0.1.0`) |
| `cargo run -p medscale-cli -- doctor` | PASS (local_only=true, medical_functionality=false) |
| `pwsh ./scripts/check-dependency-direction.ps1` | PASS |
| `cargo deny check` | PASS (advisories/bans/licenses/sources ok) |

## Limitations

- Linux CI PASS is proven only after GitHub Actions runs on the PR head.
- `cargo vet` CLI not installed locally; `supply-chain/` policy scaffold is present for future crate admissions.
- Public SPDX license for MedScale crates remains `EXTERNAL_GATES.md` → `PUBLIC_SOURCE_LICENSE_CHOICE`.

# Spec 002 evidence

**Date**: 2026-08-25  
**Branch**: `spec/002-trusted-object-source-authority-foundation`  
**Base main**: `db2b4cd790a5ebd625a6f501eb7b56799bbe3fe6`

## Toolchain

```text
rustc 1.97.1
cargo-deny 0.20.2
```

## Cargo.lock identity

```text
SHA256: 3F4D5250A30349A01150EE82360466ADF8E3FAC18012994A5E68CDC998C2281F
```

## Commands

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `cargo deny check` | PASS |
| `pwsh ./scripts/check-dependency-direction.ps1` | PASS |

## Limitations

- In-process Core Host lease only; OS IPC deferred to Spec 006.
- In-memory store only; durable ingest/storage is Spec 003/005.
- No FHIR/medical product surface in this unit.

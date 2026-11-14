# Spec 003 H0-A evidence

**Date**: 2026-08-25  
**Branch**: `spec/003-h0a-trusted-ingest-durability`

## Commands

| Command | Result |
|---|---|
| `cargo test --workspace` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo deny check` | PASS |
| `pwsh ./scripts/check-dependency-direction.ps1` | PASS |

## Claim scope

- Synthetic unencrypted vaults under local temp paths
- Sync/remote roots refused for durability claim
- SQLCipher not in scope (Spec 005)

## Limitations

- OS IPC still Spec 006
- Projection stub only (H0-B presentation deferred)
- Fault-injection / GC race suites are partial relative to full task list; core mark/tombstone/sweep + backup/restore + lexical gates proven

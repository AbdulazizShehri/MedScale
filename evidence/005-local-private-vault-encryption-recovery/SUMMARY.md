# Spec 005 evidence summary

## Delivered

- `medscale-keys`: Argon2id passphrase wraps, recovery codes, MemoryKeyStore
- `EncryptedVault`: AES-GCM sealed metadata + sealed blobs (research D4 primary ship path)
- Sync-root refusal extensions; single-writer lease file; backup/restore; key destruction; SyntheticVault→EncryptedVault migration
- Facade: CreateEncryptedVault / OpenEncryptedVault / CloseEncryptedVault
- SQLCipher admitted as preferred re-entry (`docs/engineering/admissions/005-sqlcipher-rusqlite.md`)

## Commands

```text
cargo test --workspace
cargo test -p medscale-storage --test encrypted_vault_005
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```

## Exit gates

| Gate | Status |
|---|---|
| Encryption round-trip / wrong key | PASS |
| Sync-root refuse | PASS |
| Lease conflict | PASS |
| Recovery + key-loss | PASS |
| Backup/restore/retention | PASS |
| Migration from SyntheticVault | PASS |
| REAL_PHI | still unauthorized |

## Notes

SQLCipher page crypto not enabled in this PR due to rusqlite `bundled` vs `bundled-sqlcipher` mutual exclusion with Spec 003 SyntheticVault; ciphertext-at-rest is proven via AES-GCM sealed store.

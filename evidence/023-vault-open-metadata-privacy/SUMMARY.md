# Evidence SUMMARY — Spec 023 Vault Open-Metadata Privacy

**Status:** CLOSED_CANONICAL READY_BASE  
**Branch:** `spec/023-vault-open-metadata-privacy`  
**Design:** Workspace-wide rusqlite `bundled-sqlcipher-vendored-openssl`; EncryptedVault keyed from VaultDek; SyntheticVault unkeyed.

## Delivered

- EncryptedVault `meta.work.sqlite3` page-encrypted while unlocked (SQLCipher)
- Wrong passphrase / wrong SQLCipher key fail-closed
- Round-trip durable source metadata through SQLCipher EncryptedVault
- Open-work disk negative scan for planted plaintext marker
- Doctor: `sqlcipher_enabled=true`, `open_work_page_encrypted=true`, `private_data_ready=false`
- Spec 017 wipe-on-close retained; AES-GCM `meta.sealed` retained
- Admission `005-sqlcipher-rusqlite.md` updated for Spec 023 enablement

## Measured SQLCipher version

```text
cipher_version=4.6.1 community
```

Source: `PRAGMA cipher_version` via `EncryptedVault::sqlcipher_cipher_version()` in
`crates/medscale-storage/tests/vault_open_metadata_023.rs` (rusqlite 0.37.0 /
libsqlite3-sys 0.35.0 `bundled-sqlcipher-vendored-openssl`).

## Honesty

- PRIVATE_DATA_READY = FALSE
- RELEASE_READY = FALSE
- MULTI_CLIENT_RELEASE_READY = FALSE
- REAL_PHI unauthorized

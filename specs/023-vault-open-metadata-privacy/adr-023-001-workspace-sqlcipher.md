# ADR-023-001: Workspace-wide SQLCipher for EncryptedVault open metadata

**Status**: Accepted for Spec 023  
**Date**: 2026-09-09

## Context

Spec 017 closed lifecycle wipe but left open-work plaintext. Admission preferred SQLCipher after solving rusqlite `bundled` vs `bundled-sqlcipher-*` mutual exclusion with SyntheticVault.

## Decision

1. Switch the **workspace** `rusqlite` dependency from `bundled` to `bundled-sqlcipher-vendored-openssl` (pin 0.37.0 unless CI forces ≤0.40.2 bump).
2. **EncryptedVault** applies a VaultDek-derived SQLCipher key before any metadata read/write; retain AES-GCM `meta.sealed` + wipe-on-close.
3. **SyntheticVault** and writer-lock DBs remain **unkeyed** (SQLCipher plaintext-compatible mode); no privacy claim.
4. Do **not** invent a second parallel vault API.
5. `PRIVATE_DATA_READY` remains **FALSE**.

## Consequences

- Open-work disk scans no longer reveal planted plaintext markers in EncryptedVault work DB.
- Build links vendored OpenSSL + SQLCipher; admission must record measured `cipher_version`.
- Residual OS/keyring risks remain documented; Q03 P0 incomplete for PRIVATE_DATA_READY until those gates close.

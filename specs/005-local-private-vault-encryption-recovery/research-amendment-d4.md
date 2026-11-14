# Spec 005 research amendment — implement disposition

**Date**: 2026-08-25

## D4 activation (AES-GCM sealed store ships first)

EncryptedVault lands on **AesGcmSealedStore** (sealed `meta.work.sqlite3` → `meta.sealed` + AES-GCM blob files) because workspace rusqlite **0.37.0** `bundled` (Spec 003 SyntheticVault) cannot coexist with `bundled-sqlcipher-vendored-openssl` in one dependency graph without splitting crates/processes.

SQLCipher **v4.17.0** remains the preferred metadata backend and is admitted for re-entry in `docs/engineering/admissions/005-sqlcipher-rusqlite.md`. Feature flag `sqlcipher` is reserved empty until SyntheticVault is isolated or retired.

Encryption-at-rest, wrong-key fail-closed, recovery/key-loss, backup/restore, and migration proofs are satisfied by the sealed AES-GCM path.

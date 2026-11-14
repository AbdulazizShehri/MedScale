# Dependency admission: SQLCipher + rusqlite (Specs 005 / 023)

## SQLCipher (embedded via rusqlite) / rusqlite binding

| Field | Value |
|---|---|
| Component | SQLCipher page encryption (EncryptedVault open-work metadata) |
| Acquisition pin | https://github.com/sqlcipher/sqlcipher tag **v4.17.0** (target) |
| Binding | rusqlite **0.37.0** features `bundled-sqlcipher-vendored-openssl` |
| Owning Spec | 005 (admit) / **023 (enable)** |
| Purpose | Page-encrypted SQLite metadata while EncryptedVault unlocked |
| License | SQLCipher BSD-style; rusqlite MIT/Apache-2.0; OpenSSL (vendored) Apache-2.0 |
| Placement | Workspace `rusqlite` + `medscale-storage` default feature `sqlcipher` |
| Security | One SQLCipher copy per process; keys never in IPC; VaultDek-derived `PRAGMA key`; never logged |
| Tests required | Wrong-key fail-closed; round-trip; open-work negative scan; version string evidence; doctor honesty |
| Update strategy | Pin minor; re-deny; re-verify embedded `PRAGMA cipher_version` |
| Exit strategy | `AesGcmSealedStore`-only EncryptedVault backend (research D4) if SQLCipher must be removed |

## Spec 005.1 implement disposition (2026-08-25) — historical

Workspace Spec 003 pinned rusqlite **0.37.0** `bundled` for unencrypted `SyntheticVault`. rusqlite cannot enable both `bundled` and `bundled-sqlcipher-*` in one dependency graph without splitting crates/processes.

Spec 005 shipped EncryptedVault on the D4 AES-GCM sealed metadata + AES-GCM blob backend so encryption-at-rest proofs landed without SQLCipher.

## Spec 023 enablement (2026-09-09)

**Design choice:** workspace-wide switch to `bundled-sqlcipher-vendored-openssl` (ADR-023-001). SyntheticVault / writer-lock remain **unkeyed** (SQLCipher plaintext-compatible mode). EncryptedVault applies VaultDek-derived SQLCipher key before any meta read/write; retains AES-GCM `meta.sealed` + Spec 017 wipe-on-close.

### Embedded version evidence

Measured via `PRAGMA cipher_version` in `crates/medscale-storage/tests/vault_open_metadata_023.rs`:

| Field | Value |
|---|---|
| Embedded `cipher_version` | **`4.6.1 community`** (from rusqlite 0.37.0 / libsqlite3-sys 0.35.0 `bundled-sqlcipher-vendored-openssl`) |
| Acquisition target note | Original Spec 005 pin referenced sqlcipher tag v4.17.0; **embedded measured string is authoritative** for this enablement |
| Feature flag | `medscale-storage` default `sqlcipher` = on |
| Doctor | `sqlcipher_enabled=true`, `open_work_page_encrypted=true`, `private_data_ready=false` |
| PRIVATE_DATA_READY | **FALSE** (OsKeyStore READY_BASE in Spec 028; Spec 032 probes only; swap/hibernate/snapshots/pagefile unqualified) |
| Windows build note | Vendored OpenSSL requires Perl on PATH (CI: Chocolatey StrawberryPerl) |
| macOS build note (Spec 029) | CI installs Homebrew `openssl@3` + `perl` and sets `OPENSSL_DIR` for vendored/SQLCipher builds; macOS CI ≠ product PLATFORM_QUALIFIED |

## Spec 082 feature amendment (2026-09-23)

Spec 082 (Analytics Gate) enables the rusqlite `limits` feature. It adds
no crate and no C code (`limits = []` in rusqlite 0.37.0); it only exposes
`sqlite3_limit` through `Connection::set_limit`. The analytics engine uses it
on its private in-memory connection to set `SQLITE_LIMIT_ATTACHED = 0` and
bound `SQLITE_LIMIT_LENGTH`, so a query cannot build a giant value in memory.
Vault connections are unchanged. `Cargo.lock` is unchanged by a feature flag.

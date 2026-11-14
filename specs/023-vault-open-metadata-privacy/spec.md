# Feature Specification: Vault Open-Metadata Privacy (Trusted V1 Q03 residual)

**Feature Branch**: `spec/023-vault-open-metadata-privacy`  
**Created**: 2026-09-09  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 017 `CLOSED_CANONICAL` (lifecycle wipe); Spec 005 EncryptedVault; admission `005-sqlcipher-rusqlite.md`  
**Does not**: claim `PRIVATE_DATA_READY`, `RELEASE_READY`, or `MULTI_CLIENT_RELEASE_READY`; authorize real PHI; clear OS keyring/swap/snapshot gates; mutate MESC.

## User Stories

### US1 — Page-encrypted open work metadata (P1)
An operator using EncryptedVault (synthetic data) opens a vault and writes authority/source metadata. While the vault is unlocked, a disk scan of `meta.work.sqlite3` (and sidecars) does not reveal a planted plaintext clinical/synthetic marker without the key.

### US2 — Fail-closed wrong key (P1)
Wrong passphrase/recovery/key material fails closed; no silent empty vault and no readable metadata under a wrong SQLCipher key.

### US3 — Durable round-trip through SQLCipher EncryptedVault (P1)
Authority/source data written through EncryptedVault survives close/reopen with the correct unlock method.

### US4 — Honest doctor posture (P1)
Doctor reports `sqlcipher_enabled=true`, `open_work_page_encrypted=true`, `private_data_ready=false`, and residual OS/keyring risk honestly (`open_work_plaintext_risk` may remain true for swap/hibernate/snapshots).

## Requirements

- **FR-001**: EncryptedVault open-work metadata uses SQLCipher page encryption (admitted rusqlite `bundled-sqlcipher-vendored-openssl`) with key material derived from VaultDek; apply key before any meta read/write.
- **FR-002**: Resolve `bundled` vs `bundled-sqlcipher` collision via workspace-wide SQLCipher (or crate/process isolation) such that SyntheticVault still works and `cargo test --workspace --locked` is green on Windows+Linux.
- **FR-003**: Wrong passphrase/key fail-closed; never log or persist raw key material; zeroize VaultDek on drop (existing).
- **FR-004**: Negative disk scan (or equivalent) proves open work bytes lack planted plaintext markers.
- **FR-005**: Record embedded SQLCipher version string in tests/evidence; update admission with exact evidence.
- **FR-006**: Doctor honesty: `sqlcipher_enabled`, `open_work_page_encrypted`, `private_data_ready=false`; residual risk documented.
- **FR-007**: Prefer migrating EncryptedVault path (no second parallel vault API). Keep Spec 017 close wipe + leftover detect.
- **FR-008**: Update BUILD_QUEUE / roadmap / START_HERE: Spec 023 CLOSED_CANONICAL READY_BASE; deferred advanced **024+**; clear premature “Trusted V1 exhausted / Q03 done” language. EXTERNAL_GATES: do not clear REAL_PHI; note OS key/snapshot still block PRIVATE_DATA_READY.

## Out of scope

OS keyring qualification; swap/hibernate/snapshot artifact proof; PRIVATE_DATA_READY=true; real PHI; MESC; multi-client release; inventing a second vault product API.

## Success Criteria

- Workspace fmt / clippy `-D warnings` / `cargo test --workspace --locked` PASS (`CARGO_TARGET_DIR=D:\medscale-target` on Windows)
- SQLCipher version recorded; admission updated
- Doctor honesty flags as above
- No PRIVATE_DATA_READY / RELEASE_READY / MULTI_CLIENT_RELEASE_READY / REAL_PHI claim

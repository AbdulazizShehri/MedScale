# Feature Specification: OS Keyring Custody (Trusted V1 Q03 residual)

**Feature Branch**: `spec/028-os-keyring-custody`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 023 CLOSED_CANONICAL READY_BASE; admission `005-keyring.md`  
**Does not**: claim `PRIVATE_DATA_READY`, `RELEASE_READY`, `MULTI_CLIENT_RELEASE_READY`, or authorize `REAL_PHI`. Does not qualify swap/hibernate/volume snapshots.

## User Stories

### US1 — OsKeyStore behind KeyStore (P1)
MedScale stores wrapped vault DEK material in the platform credential store via `OsKeyStore` implementing `KeyStore`, using admitted crates.io `keyring` 3.6.3 with platform features. Namespace: service `medscale`, account `medscale.vault.<vault_id>`.

### US2 — Runtime selection + Memory fallback (P1)
EncryptedVault / KeyProvider can use `OsKeyStore` when the OS store is available. CI or operators may force `MemoryKeyStore` via `MEDSCALE_FORCE_MEMORY_KEYSTORE=1`. Missing OS daemons/services fail soft.

### US3 — Doctor honesty (P1)
Doctor reports `os_keyring_available` and `os_keyring_used` from a live put/get/delete probe. `private_data_ready` remains **false**. `key_store` reports `OsStoreAvailable` when OS path is selected.

### US4 — Tests without over-claiming (P1)
MemoryKeyStore and FakeOsKeyStore unit tests always run. Real `OsKeyStore` round-trip runs when the platform store works (expect Windows Credential Manager); otherwise fail-soft with documentation.

## Requirements

- **FR-001**: Implement `OsKeyStore` using `keyring` 3.6.3 (`windows-native`, `apple-native`, `linux-native`).
- **FR-002**: Namespace wrapped DEK only under `medscale.vault.<vault_id>`.
- **FR-003**: `select_keystore` / doctor posture prefer OS when available unless Memory forced.
- **FR-004**: Doctor fields `os_keyring_available`, `os_keyring_used`; `private_data_ready=false`.
- **FR-005**: FakeOsKeyStore proves API boundary; OsKeyStore fail-soft when unavailable.
- **FR-006**: Update admission with exact versions; Spec Kit + evidence; BUILD_QUEUE → 028 CLOSED; deferred **029+**.

## Out of scope

PRIVATE_DATA_READY=true; swap/hibernate/snapshot qualification; REAL_PHI; keyring 4.x (rustc gate); ambient secret APIs; mobile app Keychain productization beyond existing Spec 009 policy types.

## Success Criteria

- Workspace fmt / clippy `-D warnings` / `cargo test --workspace --locked` PASS
- Doctor honesty: `private_data_ready=false`; os_keyring fields present
- Queue/roadmap: Spec 028 CLOSED_CANONICAL READY_BASE; deferred **029+**
- EXTERNAL_GATES `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA` remains OPEN (swap/snapshot residual)

# Feature Specification: EncryptedVault Authority Sync (Q02/Q03 residual)

**Feature Branch**: `spec/035-encrypted-vault-authority-sync`  
**Created**: 2026-09-10  
**Status**: CLOSED_CANONICAL READY_BASE  
**Depends on**: Spec 016 SyntheticVault durable sync; Spec 023 EncryptedVault SQLCipher  
**Does not**: claim PRIVATE_DATA_READY / RELEASE_READY; clear swap/snapshot gates; live partners.

## Requirements

- **FR-001**: Persist/reload authority graph through EncryptedVault meta + sealed blobs.
- **FR-002**: Mutual exclusion: synthetic vs encrypted vault open.
- **FR-003**: Doctor `encrypted_authority_sync_qualified=true`; `private_data_ready=false`.
- **FR-004**: Spec Kit + evidence; BUILD_QUEUE 035 CLOSED; deferred **036+**.

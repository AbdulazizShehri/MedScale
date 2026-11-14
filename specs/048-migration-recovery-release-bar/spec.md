# Feature Specification: Migration/Recovery Release-Bar READY_BASE (Q05 residual)

**Feature Branch**: `spec/048-migration-recovery-release-bar`  
**Promotion**: `docs/planning/SPEC_048_PROMOTION.md`

## Clarifications

- Interrupted schema migration remains fail-closed (`MigrationIncomplete`); recovery is verified encrypted backup/restore, not silent dual-schema continue.
- Spec 048 binds vault-level migration/recovery evidence; installable package upgrade/rollback remains a separate residual.

## Requirements

- **FR-001**: Prove interrupted migration journal is detected (`MigrationJournal::interrupted` / `MetaError::MigrationIncomplete`).
- **FR-002**: Prove encrypted vault backup → restore closure recovers blob digests with correct passphrase and fails closed on wrong key.
- **FR-003**: Prove SyntheticVault → EncryptedVault one-way migration preserves blob bytes.
- **FR-004**: Doctor `migration_recovery_ready_base=true`; `release_ready=false`.
- **FR-005**: Evidence under `evidence/048-migration-recovery-release-bar/` with LIMITATIONS.

## Out of scope

Package installers, signed upgrade channels, RELEASE_READY, real PHI, MESC.

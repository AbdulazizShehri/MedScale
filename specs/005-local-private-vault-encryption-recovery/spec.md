# Feature Specification: Local Private Vault + Encryption + Recovery

**Feature Branch**: `spec/005-local-private-vault-encryption-recovery`

**Created**: 2026-08-25

**Status**: Package complete; `QUALIFIED` for implementation (depends on Spec 004 `CLOSED_CANONICAL`)

**Input**: Deliver pre-PHI production technical qualification for encryption at rest, KeyProvider / key-class separation, recovery and key-loss proofs, encrypted backup/restore, retention/destruction, default local app-data vault placement, sync/remote filesystem refusal (extending Spec 003 claim scope), single-writer + key lifecycle, snapshot/retention proofs, and log/crash privacy hygiene—using **synthetic data only**. REAL_PHI remains separately unauthorized via `EXTERNAL_GATES`. No product runtime network, no MESC mutation, no Desktop/CLI product shell (006), no models/packs.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Encrypted Vault at Local App-Data Default (Priority: P1)

An engineer (or later Core Host) creates/opens a vault whose durable metadata and blobs are encrypted at rest under a vault-scoped key hierarchy. Default vault root is the OS local application-data path class—not Documents, Desktop, or a user-chosen sync folder.

**Why this priority**: Roadmap Spec 005 exit; GLM F-03/F-10; pre-PHI encryption qualification before any real PHI gate.

**Independent Test**: Create encrypted vault under resolved app-data path; write synthetic marker; close; reopen with keys; marker readable. Without keys, on-disk bytes fail closed (unreadable).

**Acceptance Scenarios**:

1. **Given** no vault yet, **When** `CreateEncryptedVault` runs with default location policy, **Then** vault root resolves under the documented local app-data path class and is claim-scoped.
2. **Given** an encrypted vault with synthetic objects/blobs, **When** opened with correct KeyProvider material, **Then** DurableStore/BlobStore reads succeed through the Core Host only.
3. **Given** the same on-disk files and absent/wrong keys, **When** open or raw file inspection is attempted, **Then** plaintext durable content is not obtainable (key-loss / wrong-key fail closed).

---

### User Story 2 - Sync / Remote Filesystem Refusal (Priority: P1)

Vault open/create refuses known sync/remote roots (OneDrive, Dropbox, iCloud, Google Drive, and Spec 003 markers), extending claim scope so production durability/encryption claims never include those roots.

**Why this priority**: Roadmap row; GLM F-03; Spec 003 claim scope must not be weakened.

**Independent Test**: Attempt open/create under paths containing refused markers → typed `SyncRootRefused` (or equivalent); successful opens only on local claim paths.

**Acceptance Scenarios**:

1. **Given** a path under OneDrive/Dropbox/iCloud/Google Drive (or Spec 003 markers), **When** create/open encrypted vault is requested, **Then** the operation fails closed with a typed sync/remote refusal—no silent proceed.
2. **Given** a local app-data path outside refused markers, **When** create/open runs, **Then** path validation succeeds (subject to OS/FS claim notes).
3. **Given** Spec 005 closeout evidence, **When** claim scope is reviewed, **Then** encryption/durability PASS claims bind local non-sync roots only.

---

### User Story 3 - Single-Writer + Key Lifecycle (Priority: P1)

Exactly one Core Host writer owns the writable encrypted store and active vault key material for an open vault. Key classes are separated (master/DEK, wrapping/KEK, recovery wrap, blob keys). Keys are never transferred over IPC; workers never receive ambient vault keys.

**Why this priority**: Constitution process topology; SPECKIT roadmap Core Host rules; SOURCE keyring placement.

**Independent Test**: Second concurrent writer lease fails; IPC messages contain no key/DB handles; KeyProvider APIs distinguish key classes.

**Acceptance Scenarios**:

1. **Given** vault V held by Core Host with lease, **When** a second independent open/writer is attempted, **Then** it is refused (`LeaseHeld` / equivalent).
2. **Given** KeyProvider operations, **When** keys are loaded, **Then** VaultMaster/DEK, KEK/wrap, RecoveryWrap, and BlobEncryption classes remain distinct types/roles—no ambient “god key” API.
3. **Given** any Client/IPC path, **When** messages are inspected in tests, **Then** no raw key bytes, DB connection handles, or SQLCipher PRAGMA secrets appear in envelopes.

---

### User Story 4 - Recovery: Passphrase + Recovery Codes (Priority: P1)

User can unlock via passphrase and/or recovery codes that wrap the vault DEK. Destroying OS keychain material alone must not make ciphertext permanently unrecoverable if recovery codes/passphrase remain; destroying all wrapping material must make ciphertext unreadable (key-loss proof).

**Why this priority**: Master plan pre-PHI recovery/key-loss; F-10.

**Independent Test**: Unlock with passphrase; unlock with recovery code after keychain wipe; after wiping all wraps + passphrase, assert open fails and raw ciphertext yields no plaintext markers.

**Acceptance Scenarios**:

1. **Given** a vault provisioned with passphrase + ≥1 recovery code, **When** OS keyring entry is deleted, **Then** unlock via passphrase or recovery code still succeeds.
2. **Given** correct recovery code after passphrase rotation policy allowing re-wrap, **When** unlock runs, **Then** DEK unwraps and vault opens.
3. **Given** all recovery codes destroyed, passphrase unknown, and keyring entry removed, **When** open is attempted, **Then** fail closed; synthetic plaintext markers are not recoverable from on-disk ciphertext.

---

### User Story 5 - Snapshot, Retention, Encrypted Backup/Restore (Priority: P1)

Encrypted backup/restore and retention/destruction behave correctly: restore closure under encryption, snapshots do not leak plaintext keys, retention destroys key material and/or ciphertext per policy, and restore requires authorized key unwrap.

**Why this priority**: Roadmap snapshot/retention/restore proof; extends Spec 003 backup interface into encrypted product posture (still synthetic-only).

**Independent Test**: Backup encrypted vault → restore to new claim path with keys → content matches; restore without keys fails; retention/destroy makes prior keys or ciphertext unusable per documented policy.

**Acceptance Scenarios**:

1. **Given** an encrypted vault with synthetic content, **When** `BackupEncryptedVault` then `RestoreEncryptedVault` with valid keys runs, **Then** restored vault opens and content digests match manifest closure.
2. **Given** a backup artifact, **When** restore is attempted without unwrap material, **Then** fail closed (no plaintext vault).
3. **Given** retention/destroy of vault key material (and/or secure delete policy for files), **When** prior paths are reopened, **Then** content is unreadable; audit records the destruction event without logging secrets.

---

### User Story 6 - One-Way Migration from Spec 003 Unencrypted Synthetic Vault (Priority: P1)

Existing Spec 003 `SyntheticVault` (unencrypted) can be upgraded once to an EncryptedVault. Migration is one-way: after success, the production open path does not silently reopen the unencrypted store as a durable authority.

**Why this priority**: Continuity from H0-A; explicit upgrade path required by founder instruction.

**Independent Test**: Fixture unencrypted vault → migrate → encrypted open succeeds; attempting to treat pre-migration DB as EncryptedVault without migration fails closed; re-running migration on already-encrypted vault is idempotent refuse or no-op success with typed status.

**Acceptance Scenarios**:

1. **Given** a Spec 003 synthetic unencrypted vault with known objects/blobs, **When** `MigrateToEncryptedVault` completes, **Then** all durable content is readable only via EncryptedVault + keys, and digests/ids are preserved.
2. **Given** migration success, **When** the old unencrypted open path is used against the upgraded root, **Then** it fails closed or is rejected as superseded (no dual-authority).
3. **Given** interruption mid-migration, **When** restart/recovery runs, **Then** journaled migration either completes or rolls back to a documented safe state (no half-encrypted silent PASS).

---

### User Story 7 - Log / Crash / Privacy Technical Hygiene (Priority: P1)

Logs, error strings, and crash-adjacent artifacts from vault/key paths must not emit raw keys, passphrases, recovery codes, or SQLCipher secrets. Spec 005 lays technical foundation for later `PRIVACY_PROOF` (006) without claiming system-wide zero packets.

**Why this priority**: Master plan pre-PHI log/crash privacy; F-17 claim scoping.

**Independent Test**: Negative tests: force key errors; scan log/output buffers for known key/passphrase/recovery fixtures → zero hits.

**Acceptance Scenarios**:

1. **Given** wrong passphrase / corrupt wrap, **When** errors are emitted, **Then** messages are typed and contain no secret material.
2. **Given** a synthetic known passphrase/recovery code used in tests, **When** vault operations run under log capture, **Then** those strings do not appear in captured logs.
3. **Given** Spec 005 evidence archive, **When** limitations are recorded, **Then** they state synthetic-only, no REAL_PHI, no product network egress, and no claim that Spec 005 alone authorizes PHI.

### Edge Cases

- Empty vault create/open: succeeds; no panic; keys still provisioned.
- Corrupted ciphertext page/blob: quarantine / fail closed; do not return partial plaintext silently.
- Clock / OS keyring unavailable: fail closed or use documented recovery-path-only unlock—never store DEK in plaintext beside the DB “for convenience.”
- Path with mixed-case OneDrive markers on Windows: still refused.
- Concurrent backup during writes: single-writer lease serializes; no torn encrypted backup PASS without lease.
- Extremely large blob: encryption streaming/chunking documented; tests use bounded fixtures.
- Real PHI-looking strings in fixtures: labeled synthetic; REAL_PHI authority remains NO.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide EncryptedVault create/open with encryption at rest for durable metadata and blobs under vault-scoped keys.
- **FR-002**: Default vault location MUST resolve to local OS application-data path class; MUST NOT default to Documents/Desktop/sync folders.
- **FR-003**: System MUST detect and refuse known sync/remote filesystem roots for vault claim paths, extending Spec 003 markers (OneDrive, Dropbox, iCloud, Google Drive, and documented equivalents).
- **FR-004**: System MUST enforce single-writer Core Host lease ownership of writable encrypted store and active key material.
- **FR-005**: System MUST implement KeyProvider with explicit key-class separation (at least Vault DEK, wrapping KEK, RecoveryWrap, BlobEncryption).
- **FR-006**: System MUST support unlock via passphrase and via recovery codes (key-wrapping); MUST prove key-loss leaves ciphertext unreadable without remaining wraps.
- **FR-007**: System MUST provide encrypted backup/restore with restore closure and fail-closed restore without keys.
- **FR-008**: System MUST provide retention/destruction behavior that renders prior vault content unreadable under the documented policy, with non-secret audit.
- **FR-009**: System MUST provide one-way migration from Spec 003 unencrypted SyntheticVault to EncryptedVault with crash-safe journal semantics.
- **FR-010**: Keys, passphrases, recovery codes, and DB encryption secrets MUST NOT appear in logs, IPC envelopes, or worker ambient authority.
- **FR-011**: Spec 005 MUST NOT authorize real PHI; all fixtures/evidence MUST be synthetic-only; REAL_PHI remains `EXTERNAL_GATES` / NOT_AUTHORIZED.
- **FR-012**: Spec 005 MUST NOT introduce product runtime network egress, MESC mutation/runtime, OpenMed runtime, models/packs, OCR/ASR, or Spec 006 Desktop/CLI product shell as authority.
- **FR-013**: Encryption backend MUST be behind an `EncryptedVault` (or equivalent) trait; preferred backend is SQLCipher per SOURCE_ACQUISITION; alternative backend only via documented fail-closed exit strategy—never plaintext production claim.
- **FR-014**: Implementation SHOULD introduce `medscale-keys` for KeyProvider/lifecycle and extend `medscale-storage` / `medscale-core` / `medscale-contracts`; platform keyring via `keyring-core` + selected stores only.
- **FR-015**: Counsel/flow-specific PDPL/SFDA legal mapping is out of product runtime scope; Spec 005 records technical privacy qualification only and MUST NOT treat legal counsel sign-off as an implement blocker (track in EXTERNAL_GATES / planning if needed).

### Key Entities

- **EncryptedVault**: Claim-scoped encrypted durable root + metadata + blobs.
- **VaultLocationPolicy**: Default app-data resolution + sync-root refusal.
- **KeyProvider**: Abstracts OS keyring + passphrase/recovery unwrap; no ambient secret API.
- **KeyClass**: VaultDEK | WrapKEK | RecoveryWrap | BlobEncryption | (optional) LeaseToken material.
- **RecoveryMaterial**: Passphrase verifier + recovery code wraps.
- **EncryptedBackupArtifact**: Manifest + encrypted snapshots/blobs + key-wrap metadata (no plaintext DEK).
- **MigrationJournal**: One-way SyntheticVault → EncryptedVault state machine.
- **RetentionDestructionRecord**: Audit of key/ciphertext destruction without secrets.
- **VaultLease**: Single-writer ownership token (extends Spec 002).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Encrypted vault golden suite: write → close → reopen with keys → 100% of synthetic markers recoverable; wrong/missing keys → 0% plaintext recovery.
- **SC-002**: Sync-root suite: 100% of documented refused path markers fail closed on create/open.
- **SC-003**: Lease suite: second writer refused while first holds lease (deterministic typed error).
- **SC-004**: Recovery suite: unlock succeeds with passphrase and with recovery code after keyring wipe; full key-loss → open fails and ciphertext yields no fixture plaintext.
- **SC-005**: Encrypted backup/restore suite: restore with keys matches digests; restore without keys fails closed.
- **SC-006**: Migration suite: Spec 003 unencrypted fixture upgrades once; post-migration unencrypted authority path rejected; crash-restart migration does not PASS half-state.
- **SC-007**: Log privacy suite: known passphrase/recovery/key fixtures never appear in captured logs/errors.
- **SC-008**: Deliverable introduces no real PHI, product network clients, MESC mutation, OpenMed runtime, models, or Spec 006 product UI shell as authority; REAL_PHI gate unchanged.

## Assumptions

- Spec 003 is `CLOSED_CANONICAL` (SyntheticVault, claim paths, backup interface).
- Spec 004 is `CLOSED_CANONICAL` (presentation continues to consume vault via Core Host; 005 does not reopen presentation).
- Spec 002 authority facade / lease / no key-handle-in-IPC rules remain binding.
- Ordinary engineering pins follow `IMPLEMENTATION_DECISION_DEFAULTS.md` and are recorded in `research.md` / `clarifications.md`.
- SQLCipher remains candidate (not constitutional); durability/key tests are mandatory for admission.
- Product runtime network remains DEFAULT_DENY by absence.
- Mobile Keychain/Keystore depth beyond desktop/CI stores is Spec 009; Spec 005 pins desktop/CI keyring stores and trait shape.

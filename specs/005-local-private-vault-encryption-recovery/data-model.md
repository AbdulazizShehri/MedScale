# Data Model: Spec 005 Local Private Vault + Encryption + Recovery

**Date**: 2026-08-25  
**Status**: Planning-canonical for Spec 005 implement  
**Persistence**: Encrypted durable objects over Spec 002/003 classes. Does not weaken Proposal≠Assertion or Projection≠truth. Synthetic-only.

## Principles

1. All Spec 002/003/004 class invariants remain in force.
2. Production durable path after 005 is **EncryptedVault** only (post-migration).
3. Key material is not a durable clinical object; wraps/salts/KDF params may persist beside ciphertext.
4. No plaintext DEK at rest beside the vault DB.
5. Sync/remote roots are outside claim scope.
6. REAL_PHI unauthorized; fixtures synthetic.

## Extensions to Spec 002/003 objects

### DurableStore / BlobStore

Same logical objects (SourceRecord, ClinicalAssertion, Projection, Audit, …) persist under encryption. Blob bytes at rest are AES-GCM sealed; metadata rows live in SQLCipher DB.

### BackupArtifact → EncryptedBackupArtifact

Adds wrap metadata, encryption profile id, and ciphertext digests. `claim_scope_note` becomes `encrypted-local-appdata` (or similar)—not `synthetic-unencrypted-h0a` for production encrypted backups.

### VaultLease (Spec 002)

Still single-writer; additionally gates access to active DEK in Core Host memory.

## New Spec 005 entities

### EncryptedVaultDescriptor

| Field | Required | Description |
|---|---|---|
| `vault_id` | yes | Opaque vault id |
| `root_path` | yes | Claim-scoped local app-data path |
| `encryption_profile` | yes | e.g. `sqlcipher4+aesgcm-blob.v1` |
| `schema_version` | yes | Vault layout version |
| `kdf_params` | yes | Argon2id params + salt id |
| `created_at` | yes | MedicalTime / UTC instant as documented |
| `migrated_from_synthetic` | optional | bool / source digest |

### KeyClass

```text
VaultDEK | WrapKEK | RecoveryWrap | BlobEncryption
```

### WrappedKeyRecord

| Field | Required | Description |
|---|---|---|
| `wrap_id` | yes | Opaque id |
| `key_class` | yes | What is wrapped |
| `wrap_method` | yes | `PassphraseArgon2id` \| `RecoveryCode` \| `OsKeyring` |
| `nonce` / `ciphertext` | yes | AES-GCM wrap of target key |
| `kdf_salt` | conditional | For passphrase method |
| `label` | optional | Non-secret label (e.g. recovery slot index) |
| `revoked` | yes | bool |

### RecoveryCodeMaterial

| Field | Required | Description |
|---|---|---|
| `code_id` | yes | Opaque |
| `verifier` | optional | Optional hash for UX validation without unwrap |
| `wrapped_dek` | yes | WrappedKeyRecord (RecoveryWrap) |
| `created_at` | yes | |

Passphrase itself is **never** stored—only salt + wrapped DEK.

### VaultLocationPolicy

| Field | Required | Description |
|---|---|---|
| `default_root_class` | yes | `LocalAppData` |
| `refused_markers` | yes | Extended sync/remote marker list |
| `allow_explicit_override` | yes | false for production claim paths in 005 tests (dev override only if explicitly non-claim) |

### MigrationJournal

| Field | Required | Description |
|---|---|---|
| `vault_id` | yes | |
| `state` | yes | See state machine |
| `source_plaintext_root` | yes | Spec 003 path |
| `dest_encrypted_root` | yes | May be same root after switch |
| `last_checkpoint` | yes | |
| `error` | optional | Typed non-secret |

### EncryptedBackupManifest

| Field | Required | Description |
|---|---|---|
| `schema_version` | yes | |
| `vault_id` | yes | |
| `encryption_profile` | yes | |
| `metadata_ciphertext_digest` | yes | |
| `blob_entries` | yes | sealed digest/size list |
| `wrap_records_public` | yes | salts/params/wrapped DEKs needed to restore |
| `claim_scope_note` | yes | local non-sync |

### RetentionDestructionRecord

| Field | Required | Description |
|---|---|---|
| `vault_id` | yes | |
| `destroyed_at` | yes | |
| `scope` | yes | `KeysOnly` \| `KeysAndCiphertext` \| `FullVaultRoot` |
| `audit_id` | yes | ActionAuditRecord ref |
| `secrets_logged` | yes | MUST be false |

### EncryptionProfile

| Field | Required | Description |
|---|---|---|
| `profile_id` | yes | `sqlcipher4+aesgcm-blob.v1` |
| `sqlcipher_version_claim` | yes | e.g. `4.17.0` or amended tested pin |
| `blob_aead` | yes | `AES-256-GCM` |
| `kdf` | yes | `Argon2id` |

## Relationships (summary)

```text
KeyProvider
  ├── WrapKEK(passphrase) ──wraps──> VaultDEK
  ├── RecoveryWrap(code)  ──wraps──> VaultDEK
  └── OsKeyring           ──stores─> WrappedKeyRecord(VaultDEK)

VaultDEK
  ├──> SQLCipher key material (metadata DB)
  └──> BlobEncryption ──AES-GCM──> sealed blobs

SyntheticVault ──MigrateToEncryptedVault──> EncryptedVault
EncryptedVault ──Backup──> EncryptedBackupArtifact ──Restore──> EncryptedVault
```

## State machines

### MigrationJournal.state

```text
NotStarted
  → Started
  → KeysProvisioned
  → SealingInProgress
  → SealedVerified
  → SwitchedAuthority
  → PlaintextTombstoned
  → Done

Any non-Done → (crash) → resume from last checkpoint
Unrecoverable → Failed (vault not claimed encrypted)
```

### Unlock

```text
Try OsKeyring wrapped DEK
  else Passphrase → Argon2id → unwrap
  else RecoveryCode → unwrap
  else FailClosed(MissingKeyMaterial)

Success → DEK in Core Host memory → open SQLCipher + blob keys
Close/Lock → zeroize best-effort
```

### Key-loss proof (test)

```text
Destroy keyring + passphrase knowledge + recovery wraps
  → Unlock fails
  → Raw DB/blob bytes contain no synthetic plaintext markers
```

# Contract: Encrypted Vault API (Spec 005)

**Spec**: 005-local-private-vault-encryption-recovery  
**Status**: Message/API sketch for implement (extends Spec 002 authority facade + Spec 003 durability)  
**Scope**: Synthetic-only; EncryptedVault; no product UI shell; no REAL_PHI; no network

## Roles

Uses Spec 002 roles (`CoreHost`, `Client`, `TransientHostOwner`). Only Core Host holds SQLCipher connection + active DEK. Workers never receive vault keys.

## New / extended capabilities

```text
Capability::CreateEncryptedVault
Capability::OpenEncryptedVault
Capability::CloseEncryptedVault
Capability::MigrateToEncryptedVault
Capability::BackupEncryptedVault
Capability::RestoreEncryptedVault
Capability::ProvisionRecoveryMaterial
Capability::UnlockWithPassphrase
Capability::UnlockWithRecoveryCode
Capability::DestroyVaultKeys        // retention/destruction
Capability::ResolveDefaultVaultPath
```

Unknown capability → `AuthorityError::Unauthorized`.  
Sync/remote path → `PathOutsideClaim` / `SyncRootRefused`.

## Envelope

Same `AuthorityRequest` / `AuthorityResponse` as Spec 002. **Forbidden in any body**: raw DEK, passphrase, recovery code plaintext (except in unlock request bodies that MUST be zeroized after use and never logged), SQLCipher key PRAGMA secrets, DB handles.

## Request / response sketches

### ResolveDefaultVaultPath

```text
ResolveDefaultVaultPath { vault_id: OpaqueId }

→ Ok { root: PathBuf, root_class: LocalAppData, claim_ok: true }
→ Err SyncRootRefused | PathOutsideClaim
```

### CreateEncryptedVault

```text
CreateEncryptedVault {
  vault_id: OpaqueId,
  root?: PathBuf,                 // default = app-data policy
  passphrase: SecretString,       // zeroize after
  recovery_code_count: u8,        // e.g. 10
}

→ Ok {
    vault_id,
    root,
    encryption_profile,
    recovery_codes: [SecretString],  // show-once to caller; never log
    lease_id,
  }

→ Err SyncRootRefused | PathOutsideClaim | LeaseHeld | KeyProviderError | DurableStoreError
```

**Behavioral rules**:

1. Refuse sync/remote roots.
2. Provision VaultDEK; wrap with passphrase + recovery codes; store OS keyring wrap when available.
3. Initialize SQLCipher DB + sealed blob store.
4. Return recovery codes once; caller responsible for offline storage UX (006+).

### OpenEncryptedVault

```text
OpenEncryptedVault {
  vault_id,
  root: PathBuf,
  unlock: OsKeyringFirst | Passphrase(SecretString) | RecoveryCode(SecretString),
}

→ Ok { lease_id, encryption_profile, unlocked_via }
→ Err SyncRootRefused | MissingKeyMaterial | WrongKeyMaterial | LeaseHeld
    | NotEncryptedVault | DurableStoreError
```

### MigrateToEncryptedVault

```text
MigrateToEncryptedVault {
  source_root: PathBuf,           // Spec 003 SyntheticVault
  dest_root?: PathBuf,            // default in-place or app-data
  passphrase: SecretString,
  recovery_code_count: u8,
}

→ Ok { vault_id, encryption_profile, recovery_codes, journal_final_state: Done }
→ Err SyncRootRefused | MigrationInProgress | MigrationFailed | ...
```

**Behavioral rules**:

1. One-way; journal crash-safe.
2. After `Done`, SyntheticVault open on that root fails closed / superseded.
3. Preserve object ids and content digests.

### BackupEncryptedVault / RestoreEncryptedVault

```text
BackupEncryptedVault { destination: PathBuf }
→ Ok EncryptedBackupManifest
→ Err SyncRootRefused | LeaseRequired | DurableStoreError

RestoreEncryptedVault {
  src: PathBuf,
  dest_root: PathBuf,
  unlock: Passphrase | RecoveryCode | WrappedKeyImport,
}
→ Ok RestoreReport { vault_id, objects_restored, blobs_restored }
→ Err MissingKeyMaterial | RestoreClosureFailed | SyncRootRefused
```

### DestroyVaultKeys

```text
DestroyVaultKeys {
  vault_id,
  scope: KeysOnly | KeysAndCiphertext | FullVaultRoot,
  confirm: ExplicitConfirmToken,
}
→ Ok RetentionDestructionRecord
→ Err Unauthorized | LeaseRequired
```

## Traits (logical)

```text
trait EncryptedVault {
  fn create(policy, keys) -> Result<Self>;
  fn open(root, unlock) -> Result<Self>;
  fn close(self);
  fn durable(&self) -> &dyn DurableStore;
  fn blobs(&self) -> &dyn SealedBlobStore;
}

trait SealedBlobStore {
  fn put_sealed(plain: &[u8]) -> Result<BlobRef>;
  fn get_unsealed(digest) -> Result<Bytes>; // verify + decrypt
  // quarantine/gc retain Spec 003 semantics on ciphertext objects
}
```

## Errors (non-exhaustive)

```text
SyncRootRefused
PathOutsideClaim
LeaseHeld
MissingKeyMaterial
WrongKeyMaterial
NotEncryptedVault
AlreadyEncrypted
MigrationFailed
RestoreClosureFailed
KeyProviderUnavailable
SecretRedactionViolation  // test-only detector signal
```

## Anti-scope

- No REAL_PHI
- No product network
- No MESC
- No Spec 006 Desktop/CLI product UX beyond optional debug harnesses
- No plaintext production vault claim

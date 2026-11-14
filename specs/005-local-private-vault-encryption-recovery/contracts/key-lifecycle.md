# Contract: Key Lifecycle (Spec 005)

**Spec**: 005-local-private-vault-encryption-recovery  
**Status**: KeyProvider / wrap lifecycle sketch for implement  
**Scope**: Synthetic-only; desktop/CI keyring stores; mobile depth deferred to 009

## KeyProvider trait (logical)

```text
trait KeyProvider {
  fn provision_vault_keys(passphrase, recovery_count) -> Result<ProvisionedKeys>;
  fn unlock_os_keyring(vault_id) -> Result<VaultDEK>;
  fn unlock_passphrase(vault_id, passphrase) -> Result<VaultDEK>;
  fn unlock_recovery_code(vault_id, code) -> Result<VaultDEK>;
  fn store_os_keyring_wrap(vault_id, wrapped: WrappedKeyRecord) -> Result<()>;
  fn delete_os_keyring_wrap(vault_id) -> Result<()>;
  fn rotate_passphrase(vault_id, old, new) -> Result<()>;  // re-wrap DEK
  fn revoke_recovery_code(vault_id, code_id) -> Result<()>;
  fn zeroize_active(&mut self);
}
```

## Key classes

| Class | At rest | In memory (Core Host only) | Notes |
|---|---|---|---|
| VaultDEK | Only wrapped | Yes while vault open | Never IPC |
| WrapKEK | Not stored (derived) | Ephemeral during unlock | Argon2id |
| RecoveryWrap | Wrapped DEK ciphertext + params | Ephemeral | Codes shown once at provision |
| BlobEncryption | Derived/wrapped from DEK | Yes while open | Vault-scoped |

## Passphrase KDF

- Algorithm: **Argon2id**
- Crate pin: **argon2 0.5.3** (admission)
- Params: freeze memory/time/parallelism at implement in `EncryptionProfile` (choose moderate desktop defaults; document exact numbers in admission/evidence)
- Salt: per-vault random ≥128-bit, stored beside wraps

## Recovery codes

- Generate high-entropy codes (e.g. 128-bit encoded Crockford/base32)—exact encoding pinned at implement
- Each code wraps VaultDEK via AES-256-GCM
- MVP: multi-use until revoked; optional single-use later
- Count default: **10** at create/migrate

## OS keyring integration

```text
set_default_store(platform_store) at Core Host start
service = "medscale.vault"
account = vault_id
secret  = WrappedKeyRecord bytes (not raw DEK)
```

Pins:

- keyring-core **1.0.0**
- windows-native-keyring-store **1.1.0**
- linux-keyutils-keyring-store **1.0.0**
- apple-native-keyring-store **1.0.2** (optional macOS)

Tests: in-memory mock store; do not require Secret Service daemon for unit tests.

## SQLCipher key material

- Unlock yields VaultDEK → derive/export SQLCipher key per profile (hex key PRAGMA or equivalent)—**only inside storage crate**
- Never log the key; never place in AuthorityResponse
- Close vault: close DB connection; zeroize DEK

## Rotation

1. Unlock with old material
2. Re-wrap DEK under new passphrase (and optionally regenerate recovery set)
3. Update keyring wrap
4. If SQLCipher rekey supported and tested, rekey DB; else document export/re-encrypt migration path

## Key-loss proof requirements

Test MUST demonstrate:

1. With wraps present → plaintext synthetic marker readable after open
2. After deleting keyring + withholding passphrase/codes → open fails
3. Scanning DB file + blob files for marker UTF-8/bytes → **not found** (or only found inside AEAD ciphertext with cryptographically expected randomness—assert marker absent as contiguous plaintext)

## Forbidden behaviors

- Storing raw DEK next to `metadata.db`
- Sharing DEK with workers
- Logging passphrase/recovery/key hex
- Using Documents/Desktop/OneDrive as key backup “folder” for production claim
- Ambient `KeyProvider::get_any_secret`

## Exit / backend note

KeyProvider is independent of SQLCipher vs AES-GCM EncryptedVault backend. If SQLCipher exit strategy activates, KeyProvider + wraps remain unchanged.

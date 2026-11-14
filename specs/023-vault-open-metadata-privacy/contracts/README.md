# Contracts: Spec 023

## Doctor JSON (additive field)

`VaultPrivacyDoctorStatus` gains:

```json
{
  "present": true,
  "private_data_ready": false,
  "sealed_at_close": true,
  "open_work_plaintext_risk": true,
  "open_work_page_encrypted": true,
  "work_wipe_on_close": true,
  "sqlcipher_enabled": true
}
```

Serde `deny_unknown_fields` remains; callers must use `spec_023_honest()` / updated constructors.

## Storage API (internal)

- `SqliteMetaStore::open_at_sqlcipher(path, key32)` — apply key before migrate
- `SqliteMetaStore::sqlcipher_cipher_version(conn)` — evidence helper
- EncryptedVault create/open paths call keyed open only

No new IPC Capability required; EncryptedVault create/open/close envelopes unchanged.

# Data model: Spec 023

## VaultPrivacyDoctorStatus (amended)

| Field | Type | Spec 023 value / meaning |
|---|---|---|
| `present` | bool | `true` |
| `private_data_ready` | bool | **always `false`** |
| `sealed_at_close` | bool | `true` (017 wipe retained) |
| `open_work_plaintext_risk` | bool | `true` (OS swap/hibernate/snapshots residual) |
| `open_work_page_encrypted` | bool | **`true`** when EncryptedVault SQLCipher backend active |
| `work_wipe_on_close` | bool | `true` |
| `sqlcipher_enabled` | bool | **`true`** |

## EncryptedVault work metadata

| Artifact | While unlocked | At rest (closed) |
|---|---|---|
| `meta.work.sqlite3` | SQLCipher page-encrypted under VaultDek-derived key | Removed (wiped) |
| `meta.sealed` | Present (prior seal) | AES-GCM sealed work bytes |
| Sealed blobs | Unchanged AES-GCM | Unchanged |

No new durable object classes. `Proposal != ClinicalAssertion` unchanged.

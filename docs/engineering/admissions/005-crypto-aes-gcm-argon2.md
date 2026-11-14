# Dependency admission: AES-GCM + Argon2 + zeroize + rand (Spec 005)

| Crate | Version pin | Purpose |
|---|---|---|
| aes-gcm | 0.10.3 | VaultDEK wrap + sealed metadata/blobs (AEAD) |
| argon2 | 0.5.3 | Passphrase → WrapKEK (Argon2id) |
| zeroize | 1.8.1 | Best-effort secret zeroization |
| rand | 0.8.5 | CSPRNG for salts/nonces/DEK/recovery codes |

| Field | Value |
|---|---|
| Owning Spec | 005 |
| Placement | `medscale-keys` + `medscale-storage` sealed backends |
| License | MIT/Apache-2.0 (RustCrypto / rand / zeroize) |
| Security | No custom cryptography; 12-byte nonces; AAD binds vault_id where applicable |
| Tests | Passphrase/recovery wrap round-trip; wrong key fails; sealed blob verify |
| Update strategy | Pin minor; cargo-deny |
| Exit strategy | Replace behind KeyProvider / SealedBlobStore traits |

Note: aes-gcm **0.10.3** chosen for ecosystem compatibility with `aead` 0.5; amend if 0.11.x is required later.

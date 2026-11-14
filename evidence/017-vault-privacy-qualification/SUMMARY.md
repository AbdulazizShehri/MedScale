# Spec 017 evidence summary

**Status**: lifecycle wipe + doctor honesty shipped; `PRIVATE_DATA_READY = FALSE`  
**ADR**: ADR-017-001 retain AES-GCM sealed-at-close; SQLCipher deferred

## Delivered

- EncryptedVault close wipes work + WAL/SHM/journal sidecars
- Open clears crash leftovers before unseal from `meta.sealed`
- Doctor `vault_privacy` axis reports honest non-ready status
- Tests `vault_privacy_017`

## Not claimed

See LIMITATIONS.md.

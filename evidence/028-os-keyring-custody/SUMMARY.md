# Evidence SUMMARY — Spec 028 OS Keyring Custody

**Status:** CLOSED_CANONICAL READY_BASE  
**Branch:** `spec/028-os-keyring-custody`  
**Design:** `OsKeyStore` via crates.io `keyring` **3.6.3** (`windows-native` / `apple-native` / `linux-native`); Memory fallback; doctor probe honesty.

## Delivered

- `OsKeyStore` implements `KeyStore` with binary secrets (`set_secret` / `get_secret`)
- Namespace: service `medscale`, account `medscale.vault.<vault_id>`
- `FakeOsKeyStore` for API-boundary unit tests without OS daemons
- `select_keystore` / `KeyStoreDoctorPosture` prefer OS when probe succeeds
- Force Memory: `MEDSCALE_FORCE_MEMORY_KEYSTORE=1`
- Doctor: `os_keyring_available`, `os_keyring_used`; `key_store` may be `OsStoreAvailable`
- `private_data_ready=false` retained
- Admission `docs/engineering/admissions/005-keyring.md` updated with exact pin

## Honesty

- PRIVATE_DATA_READY = FALSE
- RELEASE_READY = FALSE
- MULTI_CLIENT_RELEASE_READY = FALSE
- REAL_PHI unauthorized
- EXTERNAL_GATES `OS_KEYRING_SWAP_SNAPSHOT_PRIVATE_DATA` remains OPEN (swap/hibernate/snapshot)

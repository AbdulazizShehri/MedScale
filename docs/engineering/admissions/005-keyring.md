# Dependency admission: keyring (Specs 005 / 028)

## Exact pin (Spec 028)

| Crate | Version | Features | Role |
|---|---|---|---|
| **keyring** | **3.6.3** | `windows-native`, `apple-native`, `linux-native` (no defaults) | Platform credential Entry API |

**Not used:** keyring **4.x** (requires rustc ≥ 1.88; workspace `rust-version = 1.85`).  
**Not used as direct deps:** `keyring-core`, `windows-native-keyring-store`, `linux-keyutils-keyring-store`, `apple-native-keyring-store` (those are the 4.x architecture; 3.6.3 embeds platform backends via features above).

Historical planning candidates (superseded by this pin):

| Crate | Former candidate | Notes |
|---|---|---|
| keyring-core | 1.0.0 | Spec 028 uses umbrella `keyring` 3.6.3 instead |
| windows-native-keyring-store | 1.1.0 | Covered by `windows-native` feature |
| linux-keyutils-keyring-store | 1.0.0 | Covered by `linux-native` (keyutils) |
| apple-native-keyring-store | 1.0.2 | Covered by `apple-native` |

| Field | Value |
|---|---|
| Owning Spec | 005 / 009 / **028** |
| Placement | `medscale-keys` (`OsKeyStore`); doctor probe via `medscale-core` |
| Purpose | Store **wrapped** VaultDEK / unlock token only; no ambient secret API |
| Namespace | service=`medscale`, account=`medscale.vault.<vault_id>` |
| License | MIT OR Apache-2.0 (crates.io) |
| Spec 005.1 disposition | **In-process `MemoryKeyStore`** remains CI-safe default when OS store unavailable or `MEDSCALE_FORCE_MEMORY_KEYSTORE=1` |
| Spec 028 disposition | **`OsKeyStore`** wraps `keyring::Entry` with binary `set_secret`/`get_secret`; runtime detection + doctor `os_keyring_available` / `os_keyring_used`; **PRIVATE_DATA_READY stays false** |
| Exit strategy | Swap KeyStore implementation without changing wrap formats |
| Tests | MemoryKeyStore + FakeOsKeyStore unit; OsKeyStore fail-soft if platform store missing; Windows Credential Manager preferred for real OS CI |
| Update strategy | Pin patch/minor; re-run deny + keystore tests; do not jump to 4.x without rust-version bump |
| Security | Wrapped DEK only; probe account `__doctor_probe__` cleaned after doctor check |

## Linux CI note

`linux-native` uses kernel **keyutils** (no D-Bus Secret Service daemon). If the host lacks keyutils support, `OsKeyStore::probe_roundtrip` fails soft; doctor reports `os_keyring_available=false` and CI continues with Memory/Fake stores. Documented in `evidence/028-os-keyring-custody/`.

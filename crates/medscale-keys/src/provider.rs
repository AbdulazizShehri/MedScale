//! KeyProvider: passphrase Argon2id wraps + recovery + keystore.

use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::aead_wrap::{WrappedBlob, seal, unseal};
use crate::keystore::{KeyStore, KeyStoreError};
use crate::recovery::{
    DEFAULT_RECOVERY_COUNT, RecoveryCodeSet, generate_recovery_codes, unwrap_with_code,
    wrap_with_code,
};

/// Encryption profile id for vault header.
pub const ENCRYPTION_PROFILE_V1: &str = "medscale.vault.aesgcm.sealed.v1";

/// Argon2id memory (KiB) for new and rewrapped vaults: 64 MiB.
pub const ARGON2_M_KIB: u32 = 65_536;
pub const ARGON2_T: u32 = 3;
pub const ARGON2_P: u32 = 1;

/// Minimum accepted Argon2id parameters when opening an existing header (legacy 19 MiB v1 vaults).
pub const ARGON2_MIN_M_KIB: u32 = 19_456;
pub const ARGON2_MAX_M_KIB: u32 = 262_144;
pub const ARGON2_MIN_T: u32 = 2;
pub const ARGON2_MAX_T: u32 = 16;
pub const ARGON2_MIN_P: u32 = 1;
pub const ARGON2_MAX_P: u32 = 8;
const MIN_SALT_BYTES: usize = 16;
const MAX_SALT_BYTES: usize = 64;

pub const RECOVERY_CODE_COUNT: usize = DEFAULT_RECOVERY_COUNT;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionProfile {
    pub id: String,
    pub argon2_m_kib: u32,
    pub argon2_t: u32,
    pub argon2_p: u32,
}

impl EncryptionProfile {
    #[must_use]
    pub fn v1() -> Self {
        Self {
            id: ENCRYPTION_PROFILE_V1.to_owned(),
            argon2_m_kib: ARGON2_M_KIB,
            argon2_t: ARGON2_T,
            argon2_p: ARGON2_P,
        }
    }

    /// True when this profile uses weaker Argon2id parameters than the current
    /// default (for example a legacy 19 MiB header) and should be rewrapped on unlock.
    #[must_use]
    pub fn needs_upgrade(&self) -> bool {
        self.argon2_m_kib < ARGON2_M_KIB || self.argon2_t < ARGON2_T || self.argon2_p < ARGON2_P
    }

    /// Validates that header KDF parameters fall within safe bounds: refuses both
    /// silent weakening below legacy minimums and DoS/OOM parameters above caps.
    pub fn validate_bounds(&self) -> Result<(), KeyError> {
        if self.id != ENCRYPTION_PROFILE_V1 {
            return Err(KeyError::Invalid(
                "unsupported encryption profile".to_owned(),
            ));
        }
        if !(ARGON2_MIN_M_KIB..=ARGON2_MAX_M_KIB).contains(&self.argon2_m_kib)
            || !(ARGON2_MIN_T..=ARGON2_MAX_T).contains(&self.argon2_t)
            || !(ARGON2_MIN_P..=ARGON2_MAX_P).contains(&self.argon2_p)
        {
            return Err(KeyError::Invalid(
                "argon2 parameters out of bounds".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Vault data encryption key (32 bytes).
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultDek {
    bytes: [u8; 32],
}

impl VaultDek {
    #[must_use]
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        if bytes.len() != 32 {
            return Err(KeyError::Crypto);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self { bytes: arr })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl std::fmt::Debug for VaultDek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VaultDek([REDACTED])")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeyHeader {
    pub vault_id: String,
    pub profile: EncryptionProfile,
    pub salt: Vec<u8>,
    pub passphrase_wrap: WrappedBlob,
    pub recovery_wraps: Vec<WrappedBlob>,
    /// Revoked recovery wrap indexes (optional).
    pub revoked_recovery: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockMethod {
    Passphrase,
    RecoveryCode,
    KeyStore,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum KeyError {
    #[error("missing key material")]
    MissingKeyMaterial,
    #[error("crypto failure")]
    Crypto,
    #[error("keystore: {0}")]
    KeyStore(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
}

impl From<KeyStoreError> for KeyError {
    fn from(value: KeyStoreError) -> Self {
        Self::KeyStore(value.to_string())
    }
}

/// Creates and unlocks vault key material.
pub struct KeyProvider;

impl KeyProvider {
    fn argon2(profile: &EncryptionProfile) -> Result<Argon2<'static>, KeyError> {
        profile.validate_bounds()?;
        let params = Params::new(
            profile.argon2_m_kib,
            profile.argon2_t,
            profile.argon2_p,
            None,
        )
        .map_err(|_| KeyError::Crypto)?;
        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }

    fn derive_kek(
        passphrase: &str,
        salt: &[u8],
        profile: &EncryptionProfile,
    ) -> Result<Zeroizing<[u8; 32]>, KeyError> {
        if !(MIN_SALT_BYTES..=MAX_SALT_BYTES).contains(&salt.len()) {
            return Err(KeyError::Invalid(
                "vault salt length out of bounds".to_owned(),
            ));
        }
        let argon = Self::argon2(profile)?;
        let mut out = Zeroizing::new([0u8; 32]);
        argon
            .hash_password_into(passphrase.as_bytes(), salt, out.as_mut())
            .map_err(|_| KeyError::Crypto)?;
        Ok(out)
    }

    fn aad(vault_id: &str) -> Vec<u8> {
        format!("medscale.vault.aad.v1:{vault_id}").into_bytes()
    }

    /// Create new DEK + header + recovery codes (codes returned once).
    pub fn create_vault_keys(
        vault_id: &str,
        passphrase: &str,
    ) -> Result<(VaultDek, VaultKeyHeader, RecoveryCodeSet), KeyError> {
        if passphrase.is_empty() {
            return Err(KeyError::Invalid("empty passphrase".to_owned()));
        }
        let profile = EncryptionProfile::v1();
        let dek = VaultDek::generate();
        let mut salt = vec![0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        let aad = Self::aad(vault_id);
        let kek = Self::derive_kek(passphrase, &salt, &profile)?;
        let passphrase_wrap =
            seal(kek.as_slice(), dek.as_bytes(), &aad).map_err(|_| KeyError::Crypto)?;

        let codes = generate_recovery_codes(RECOVERY_CODE_COUNT);
        let mut recovery_wraps = Vec::new();
        for code in &codes.codes {
            recovery_wraps.push(wrap_with_code(&dek, code, &aad)?);
        }

        let header = VaultKeyHeader {
            vault_id: vault_id.to_owned(),
            profile,
            salt,
            passphrase_wrap,
            recovery_wraps,
            revoked_recovery: vec![],
        };
        Ok((dek, header, codes))
    }

    /// A new header whose passphrase wrap uses `new_passphrase`, a fresh salt
    /// and the current profile. `current` must already unlock `header`, and it
    /// must be the same data key as `dek`, so an open session alone can't change
    /// the passphrase. Recovery wraps are kept.
    pub fn change_passphrase(
        header: &VaultKeyHeader,
        dek: &VaultDek,
        current: &str,
        new_passphrase: &str,
    ) -> Result<VaultKeyHeader, KeyError> {
        if new_passphrase.is_empty() {
            return Err(KeyError::Invalid("empty passphrase".to_owned()));
        }
        let checked = Self::unlock_passphrase(header, current)?;
        if checked.as_bytes() != dek.as_bytes() {
            return Err(KeyError::MissingKeyMaterial);
        }
        let profile = EncryptionProfile::v1();
        let mut salt = vec![0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        let aad = Self::aad(&header.vault_id);
        let kek = Self::derive_kek(new_passphrase, &salt, &profile)?;
        let passphrase_wrap =
            seal(kek.as_slice(), dek.as_bytes(), &aad).map_err(|_| KeyError::Crypto)?;
        Ok(VaultKeyHeader {
            profile,
            salt,
            passphrase_wrap,
            ..header.clone()
        })
    }

    /// Upgrades a legacy header (e.g. 19 MiB Argon2id) to the current default
    /// profile (64 MiB) with a fresh salt using the already-verified passphrase
    /// and DEK. Returns `Ok(None)` when the header already meets current strength.
    pub fn rewrap_passphrase_if_needed(
        header: &VaultKeyHeader,
        dek: &VaultDek,
        passphrase: &str,
    ) -> Result<Option<VaultKeyHeader>, KeyError> {
        if !header.profile.needs_upgrade() {
            return Ok(None);
        }
        if passphrase.is_empty() {
            return Err(KeyError::Invalid("empty passphrase".to_owned()));
        }
        let profile = EncryptionProfile::v1();
        let mut salt = vec![0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        let aad = Self::aad(&header.vault_id);
        let kek = Self::derive_kek(passphrase, &salt, &profile)?;
        let passphrase_wrap =
            seal(kek.as_slice(), dek.as_bytes(), &aad).map_err(|_| KeyError::Crypto)?;
        Ok(Some(VaultKeyHeader {
            profile,
            salt,
            passphrase_wrap,
            ..header.clone()
        }))
    }

    /// Rewraps a header using the legacy 19 MiB (t=2, p=1) profile so
    /// downstream storage tests can exercise the 19 MiB -> 64 MiB migration path.
    #[doc(hidden)]
    pub fn wrap_legacy_19mib_for_upgrade_test(
        header: &VaultKeyHeader,
        dek: &VaultDek,
        passphrase: &str,
    ) -> Result<VaultKeyHeader, KeyError> {
        let legacy_profile = EncryptionProfile {
            id: ENCRYPTION_PROFILE_V1.to_owned(),
            argon2_m_kib: ARGON2_MIN_M_KIB,
            argon2_t: ARGON2_MIN_T,
            argon2_p: ARGON2_MIN_P,
        };
        let mut salt = vec![0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        let aad = Self::aad(&header.vault_id);
        let kek = Self::derive_kek(passphrase, &salt, &legacy_profile)?;
        let passphrase_wrap =
            seal(kek.as_slice(), dek.as_bytes(), &aad).map_err(|_| KeyError::Crypto)?;
        Ok(VaultKeyHeader {
            profile: legacy_profile,
            salt,
            passphrase_wrap,
            ..header.clone()
        })
    }

    /// A new header with a fresh set of recovery codes, returned once. Every
    /// earlier code stops working. Needs the current passphrase, like
    /// [`Self::change_passphrase`].
    pub fn replace_recovery_codes(
        header: &VaultKeyHeader,
        dek: &VaultDek,
        current: &str,
    ) -> Result<(VaultKeyHeader, RecoveryCodeSet), KeyError> {
        let checked = Self::unlock_passphrase(header, current)?;
        if checked.as_bytes() != dek.as_bytes() {
            return Err(KeyError::MissingKeyMaterial);
        }
        let aad = Self::aad(&header.vault_id);
        let codes = generate_recovery_codes(RECOVERY_CODE_COUNT);
        let mut recovery_wraps = Vec::with_capacity(codes.codes.len());
        for code in &codes.codes {
            recovery_wraps.push(wrap_with_code(dek, code, &aad)?);
        }
        Ok((
            VaultKeyHeader {
                recovery_wraps,
                revoked_recovery: vec![],
                ..header.clone()
            },
            codes,
        ))
    }

    pub fn unlock_passphrase(
        header: &VaultKeyHeader,
        passphrase: &str,
    ) -> Result<VaultDek, KeyError> {
        let aad = Self::aad(&header.vault_id);
        let kek = Self::derive_kek(passphrase, &header.salt, &header.profile)?;
        let bytes = Zeroizing::new(
            unseal(kek.as_slice(), &header.passphrase_wrap, &aad)
                .map_err(|_| KeyError::MissingKeyMaterial)?,
        );
        VaultDek::from_bytes(&bytes)
    }

    pub fn unlock_recovery(header: &VaultKeyHeader, code: &str) -> Result<VaultDek, KeyError> {
        let aad = Self::aad(&header.vault_id);
        for (idx, wrap) in header.recovery_wraps.iter().enumerate() {
            if header.revoked_recovery.contains(&idx) {
                continue;
            }
            if let Ok(dek) = unwrap_with_code(wrap, code, &aad) {
                return Ok(dek);
            }
        }
        Err(KeyError::MissingKeyMaterial)
    }

    pub fn keystore_account(vault_id: &str) -> String {
        format!("medscale.vault.{vault_id}")
    }

    /// Store wrapped DEK bytes in keystore (wrapped under a random OS wrap key stored alongside — MVP stores passphrase_wrap ciphertext only as unlock token is the sealed DEK under a keystore-local random key).
    pub fn store_wrapped_dek(
        store: &dyn KeyStore,
        vault_id: &str,
        dek: &VaultDek,
    ) -> Result<(), KeyError> {
        let mut wrap_key = Zeroizing::new([0u8; 32]);
        rand::thread_rng().fill_bytes(wrap_key.as_mut());
        let aad = Self::aad(vault_id);
        let sealed =
            seal(wrap_key.as_slice(), dek.as_bytes(), &aad).map_err(|_| KeyError::Crypto)?;
        let wrap_vec = Zeroizing::new(wrap_key.to_vec());
        let payload = Zeroizing::new(
            serde_json::to_vec(&(&*wrap_vec, sealed)).map_err(|_| KeyError::Crypto)?,
        );
        store.put(&Self::keystore_account(vault_id), &payload)?;
        Ok(())
    }

    pub fn unlock_keystore(
        store: &dyn KeyStore,
        header: &VaultKeyHeader,
    ) -> Result<VaultDek, KeyError> {
        let payload = Zeroizing::new(store.get(&Self::keystore_account(&header.vault_id))?);
        let (wrap_key_raw, sealed): (Vec<u8>, WrappedBlob) =
            serde_json::from_slice(&payload).map_err(|_| KeyError::MissingKeyMaterial)?;
        let wrap_key = Zeroizing::new(wrap_key_raw);
        let aad = Self::aad(&header.vault_id);
        let bytes = Zeroizing::new(
            unseal(&wrap_key, &sealed, &aad).map_err(|_| KeyError::MissingKeyMaterial)?,
        );
        VaultDek::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_passphrase_needs_the_current_one_and_keeps_the_data_key() {
        let (dek, header, codes) =
            KeyProvider::create_vault_keys("v-rekey", "old passphrase").unwrap();
        assert!(KeyProvider::change_passphrase(&header, &dek, "wrong", "new passphrase").is_err());
        assert!(KeyProvider::change_passphrase(&header, &dek, "old passphrase", "").is_err());
        let changed =
            KeyProvider::change_passphrase(&header, &dek, "old passphrase", "new passphrase")
                .unwrap();
        assert_ne!(changed.salt, header.salt);
        assert!(KeyProvider::unlock_passphrase(&changed, "old passphrase").is_err());
        let reopened = KeyProvider::unlock_passphrase(&changed, "new passphrase").unwrap();
        assert_eq!(reopened.as_bytes(), dek.as_bytes());
        // Recovery codes are untouched by a passphrase change.
        assert_eq!(
            KeyProvider::unlock_recovery(&changed, &codes.codes[0])
                .unwrap()
                .as_bytes(),
            dek.as_bytes()
        );
        // A different vault's key can't be used to rewrap this one.
        let other = VaultDek::generate();
        assert!(KeyProvider::change_passphrase(&header, &other, "old passphrase", "x").is_err());
    }

    #[test]
    fn replacing_recovery_codes_revokes_every_old_code() {
        let (dek, header, old) = KeyProvider::create_vault_keys("v-codes", "passphrase").unwrap();
        assert!(KeyProvider::replace_recovery_codes(&header, &dek, "wrong").is_err());
        let (replaced, new) =
            KeyProvider::replace_recovery_codes(&header, &dek, "passphrase").unwrap();
        assert_eq!(new.codes.len(), RECOVERY_CODE_COUNT);
        for code in &old.codes {
            assert!(
                KeyProvider::unlock_recovery(&replaced, code).is_err(),
                "old code still opens"
            );
        }
        assert_eq!(
            KeyProvider::unlock_recovery(&replaced, &new.codes[0])
                .unwrap()
                .as_bytes(),
            dek.as_bytes()
        );
        assert_eq!(
            KeyProvider::unlock_passphrase(&replaced, "passphrase")
                .unwrap()
                .as_bytes(),
            dek.as_bytes()
        );
    }
    use crate::keystore::MemoryKeyStore;

    #[test]
    fn passphrase_and_recovery_roundtrip() {
        let (dek, header, codes) =
            KeyProvider::create_vault_keys("v1", "correct horse battery").unwrap();
        let u1 = KeyProvider::unlock_passphrase(&header, "correct horse battery").unwrap();
        assert_eq!(u1.as_bytes(), dek.as_bytes());
        let u2 = KeyProvider::unlock_recovery(&header, &codes.codes[0]).unwrap();
        assert_eq!(u2.as_bytes(), dek.as_bytes());
        assert!(KeyProvider::unlock_passphrase(&header, "wrong").is_err());
    }

    #[test]
    fn keystore_roundtrip_and_wipe() {
        let store = MemoryKeyStore::new();
        let (dek, header, _) = KeyProvider::create_vault_keys("v2", "pw").unwrap();
        KeyProvider::store_wrapped_dek(&store, "v2", &dek).unwrap();
        let u = KeyProvider::unlock_keystore(&store, &header).unwrap();
        assert_eq!(u.as_bytes(), dek.as_bytes());
        store.wipe();
        assert!(KeyProvider::unlock_keystore(&store, &header).is_err());
    }

    #[test]
    fn new_vaults_use_64_mib_argon2id() {
        let (_dek, header, _codes) =
            KeyProvider::create_vault_keys("v-64m", "strong passphrase").unwrap();
        assert_eq!(header.profile.argon2_m_kib, 65_536);
        assert_eq!(header.profile.argon2_t, 3);
        assert_eq!(header.profile.argon2_p, 1);
        assert!(!header.profile.needs_upgrade());
    }

    #[test]
    fn kdf_header_bounds_reject_weak_and_oversized_parameters() {
        let (_dek, base, _codes) = KeyProvider::create_vault_keys("v-bounds", "pw").unwrap();

        let mut too_weak_mem = base.clone();
        too_weak_mem.profile.argon2_m_kib = ARGON2_MIN_M_KIB - 1;
        assert!(KeyProvider::unlock_passphrase(&too_weak_mem, "pw").is_err());

        let mut too_large_mem = base.clone();
        too_large_mem.profile.argon2_m_kib = ARGON2_MAX_M_KIB + 1;
        assert!(KeyProvider::unlock_passphrase(&too_large_mem, "pw").is_err());

        let mut too_weak_t = base.clone();
        too_weak_t.profile.argon2_t = ARGON2_MIN_T - 1;
        assert!(KeyProvider::unlock_passphrase(&too_weak_t, "pw").is_err());

        let mut too_large_t = base.clone();
        too_large_t.profile.argon2_t = ARGON2_MAX_T + 1;
        assert!(KeyProvider::unlock_passphrase(&too_large_t, "pw").is_err());

        let mut bad_p = base.clone();
        bad_p.profile.argon2_p = ARGON2_MAX_P + 1;
        assert!(KeyProvider::unlock_passphrase(&bad_p, "pw").is_err());

        let mut bad_profile = base.clone();
        bad_profile.profile.id = "medscale.vault.tampered.v0".to_owned();
        assert!(KeyProvider::unlock_passphrase(&bad_profile, "pw").is_err());

        let mut short_salt = base;
        short_salt.salt = vec![1, 2, 3];
        assert!(KeyProvider::unlock_passphrase(&short_salt, "pw").is_err());
    }

    #[test]
    fn legacy_19_mib_header_unlocks_and_rewraps_to_64_mib() {
        let dek = VaultDek::generate();
        let legacy_profile = EncryptionProfile {
            id: ENCRYPTION_PROFILE_V1.to_owned(),
            argon2_m_kib: 19_456,
            argon2_t: 2,
            argon2_p: 1,
        };
        assert!(legacy_profile.needs_upgrade());
        let salt = vec![0x42u8; 16];
        let aad = KeyProvider::aad("v-legacy");
        let kek = KeyProvider::derive_kek("legacy pass", &salt, &legacy_profile).unwrap();
        let passphrase_wrap = seal(kek.as_slice(), dek.as_bytes(), &aad).unwrap();
        let legacy_header = VaultKeyHeader {
            vault_id: "v-legacy".to_owned(),
            profile: legacy_profile,
            salt: salt.clone(),
            passphrase_wrap,
            recovery_wraps: vec![],
            revoked_recovery: vec![],
        };

        let unlocked = KeyProvider::unlock_passphrase(&legacy_header, "legacy pass").unwrap();
        assert_eq!(unlocked.as_bytes(), dek.as_bytes());

        let upgraded =
            KeyProvider::rewrap_passphrase_if_needed(&legacy_header, &unlocked, "legacy pass")
                .unwrap()
                .expect("legacy header should require upgrade");
        assert_eq!(upgraded.profile.argon2_m_kib, ARGON2_M_KIB);
        assert_eq!(upgraded.profile.argon2_t, ARGON2_T);
        assert_ne!(upgraded.salt, salt);
        assert!(!upgraded.profile.needs_upgrade());

        let unlocked_after = KeyProvider::unlock_passphrase(&upgraded, "legacy pass").unwrap();
        assert_eq!(unlocked_after.as_bytes(), dek.as_bytes());

        // Second check is a no-op.
        assert!(
            KeyProvider::rewrap_passphrase_if_needed(&upgraded, &unlocked_after, "legacy pass")
                .unwrap()
                .is_none()
        );
    }
}

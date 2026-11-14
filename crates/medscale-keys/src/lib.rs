//! KeyProvider and vault key classes (Spec 005). Synthetic-only proofs; REAL_PHI unauthorized.

mod aead_wrap;
mod device_key;
mod keystore;
mod pack_trust;
mod provider;
mod recovery;

pub use aead_wrap::{WrappedBlob, generate_key32, seal, unseal, zeroize_key};
pub use device_key::{
    DeviceKeyError, device_public_key, generate_device_key, sign_device_payload,
    verify_device_signature,
};
pub use keystore::{
    FORCE_MEMORY_KEYSTORE_ENV, FakeOsKeyStore, KeyStore, KeyStoreDoctorPosture, KeyStoreError,
    MemoryKeyStore, MobileKeyStorePolicy, OS_KEYRING_PROBE_ACCOUNT, OS_KEYRING_SERVICE, OsKeyStore,
    prefer_os_keystore, select_keystore,
};
pub use pack_trust::{
    PackTrustError, SYNTHETIC_PACK_TRUST_ROOT_ID, pack_signing_payload, sign_pack_payload,
    synthetic_verifying_key_hex, verify_pack_signature,
};
pub use provider::{
    ARGON2_M_KIB, ARGON2_P, ARGON2_T, ENCRYPTION_PROFILE_V1, EncryptionProfile, KeyError,
    KeyProvider, RECOVERY_CODE_COUNT, UnlockMethod, VaultDek, VaultKeyHeader,
};
pub use recovery::{RecoveryCodeSet, generate_recovery_codes};

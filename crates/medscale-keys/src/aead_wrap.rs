//! AES-256-GCM seal/unseal helpers.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

/// Sealed ciphertext with random 96-bit nonce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedBlob {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum SealError {
    #[error("aead encrypt/decrypt failed")]
    Aead,
    #[error("invalid key length")]
    BadKey,
}

/// Seal plaintext under a 32-byte key with optional AAD.
pub fn seal(key: &[u8], plaintext: &[u8], aad: &[u8]) -> Result<WrappedBlob, SealError> {
    if key.len() != 32 {
        return Err(SealError::BadKey);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| SealError::BadKey)?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| SealError::Aead)?;
    Ok(WrappedBlob {
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    })
}

/// Unseal ciphertext; fails closed on auth failure.
pub fn unseal(key: &[u8], blob: &WrappedBlob, aad: &[u8]) -> Result<Vec<u8>, SealError> {
    if key.len() != 32 || blob.nonce.len() != 12 {
        return Err(SealError::BadKey);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| SealError::BadKey)?;
    let nonce = Nonce::from_slice(&blob.nonce);
    cipher
        .decrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &blob.ciphertext,
                aad,
            },
        )
        .map_err(|_| SealError::Aead)
}

/// Generates a fresh random 32-byte key (for example a Spec 079 pseudonym
/// map key). The caller stores it only in a `KeyStore`.
#[must_use]
pub fn generate_key32() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Zeroize helper for owned key bytes.
pub fn zeroize_key(key: &mut [u8]) {
    key.zeroize();
}

//! Hub device keys (Spec 084): one ed25519 key pair per enrolled device.
//!
//! The secret never leaves the client vault; the Hub keeps only the public
//! key. Every encoding is strict lowercase hex of an exact length, and
//! verification uses `verify_strict` (no malleable signatures, no weak
//! public keys).

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use zeroize::Zeroize;

use crate::aead_wrap::generate_key32;

/// Why a device key operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeviceKeyError {
    #[error("device key encoding is not strict lowercase hex of the right length")]
    Encoding,
    #[error("device public key is not a valid ed25519 point")]
    PublicKey,
    #[error("device signature does not verify")]
    Verify,
}

/// A fresh device key pair as `(secret_hex, public_hex)`.
#[must_use]
pub fn generate_device_key() -> (String, String) {
    let mut seed = generate_key32();
    let signing = SigningKey::from_bytes(&seed);
    seed.zeroize();
    let secret = hex_encode(signing.as_bytes());
    let public = hex_encode(signing.verifying_key().as_bytes());
    (secret, public)
}

/// The public key hex of a secret key hex.
pub fn device_public_key(secret_hex: &str) -> Result<String, DeviceKeyError> {
    let signing = signing_key(secret_hex)?;
    Ok(hex_encode(signing.verifying_key().as_bytes()))
}

/// Signs `payload` with the device secret; returns the signature hex.
pub fn sign_device_payload(secret_hex: &str, payload: &[u8]) -> Result<String, DeviceKeyError> {
    let signing = signing_key(secret_hex)?;
    Ok(hex_encode(&signing.sign(payload).to_bytes()))
}

/// Verifies a device signature strictly.
pub fn verify_device_signature(
    public_hex: &str,
    payload: &[u8],
    signature_hex: &str,
) -> Result<(), DeviceKeyError> {
    let key_bytes: [u8; 32] = decode_exact(public_hex)?;
    let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| DeviceKeyError::PublicKey)?;
    if key.is_weak() {
        return Err(DeviceKeyError::PublicKey);
    }
    let sig_bytes: [u8; 64] = decode_exact(signature_hex)?;
    let sig = Signature::from_bytes(&sig_bytes);
    key.verify_strict(payload, &sig)
        .map_err(|_| DeviceKeyError::Verify)
}

fn signing_key(secret_hex: &str) -> Result<SigningKey, DeviceKeyError> {
    let mut seed: [u8; 32] = decode_exact(secret_hex)?;
    let key = SigningKey::from_bytes(&seed);
    seed.zeroize();
    Ok(key)
}

fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

fn nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Decodes exactly `N` bytes of strict lowercase hex. Anything else (odd or
/// wrong length, uppercase, signs, whitespace, non-ASCII) is refused.
fn decode_exact<const N: usize>(hex: &str) -> Result<[u8; N], DeviceKeyError> {
    let bytes = hex.as_bytes();
    if bytes.len() != N * 2 {
        return Err(DeviceKeyError::Encoding);
    }
    let mut out = [0_u8; N];
    for (slot, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let hi = nibble(pair[0]).ok_or(DeviceKeyError::Encoding)?;
        let lo = nibble(pair[1]).ok_or(DeviceKeyError::Encoding)?;
        *slot = (hi << 4) | lo;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_sign_and_verify() {
        let (secret, public) = generate_device_key();
        assert_eq!(secret.len(), 64);
        assert_eq!(public.len(), 64);
        assert_eq!(device_public_key(&secret).unwrap(), public);
        let sig = sign_device_payload(&secret, b"payload").unwrap();
        assert_eq!(sig.len(), 128);
        verify_device_signature(&public, b"payload", &sig).unwrap();
        assert_eq!(
            verify_device_signature(&public, b"other", &sig),
            Err(DeviceKeyError::Verify)
        );
        let (_, other_public) = generate_device_key();
        assert_eq!(
            verify_device_signature(&other_public, b"payload", &sig),
            Err(DeviceKeyError::Verify)
        );
    }

    #[test]
    fn encodings_are_strict() {
        let (secret, public) = generate_device_key();
        let sig = sign_device_payload(&secret, b"p").unwrap();
        for bad in [
            public.to_uppercase(),
            format!("{} ", &public[..63]),
            format!("+{}", &public[1..]),
            public[..62].to_owned(),
            format!("{}\u{e9}", &public[..62]),
        ] {
            assert_eq!(
                verify_device_signature(&bad, b"p", &sig),
                Err(DeviceKeyError::Encoding),
                "{bad:?}"
            );
        }
        let mut sig_upper = sig.clone();
        sig_upper.make_ascii_uppercase();
        assert!(verify_device_signature(&public, b"p", &sig_upper).is_err());
        assert_eq!(
            sign_device_payload("zz", b"p"),
            Err(DeviceKeyError::Encoding)
        );
        // The identity point is a weak key.
        let mut identity = "01".to_owned();
        identity.push_str(&"0".repeat(62));
        assert!(verify_device_signature(&identity, b"p", &sig).is_err());
    }
}

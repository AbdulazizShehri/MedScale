//! Synthetic Pack trust (ed25519) — Spec 026.
//!
//! Fixture/synthetic Packs only. Signing seed is deterministic for reproducible fixtures;
//! it is not a production code-signing root.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use thiserror::Error;

/// Trust root id embedded in Pack manifests.
pub const SYNTHETIC_PACK_TRUST_ROOT_ID: &str = "synthetic-pack-trust-v1";

/// Deterministic synthetic signing seed (NOT a production secret).
const SYNTHETIC_SIGNING_SEED: [u8; 32] = [
    0x4d, 0x45, 0x44, 0x53, 0x43, 0x41, 0x4c, 0x45, // MEDSCALE
    0x2d, 0x50, 0x41, 0x43, 0x4b, 0x2d, 0x54, 0x52, // -PACK-TR
    0x55, 0x53, 0x54, 0x2d, 0x56, 0x31, 0x2d, 0x53, // UST-V1-S
    0x59, 0x4e, 0x54, 0x48, 0x45, 0x54, 0x49, 0x43, // YNTHETIC
];

/// Errors verifying or signing Pack trust payloads.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PackTrustError {
    #[error("unknown trust root id")]
    UnknownTrustRoot,
    #[error("invalid signature encoding")]
    InvalidEncoding,
    #[error("signature verification failed")]
    VerifyFailed,
}

/// Canonical bytes signed for Pack admission (Spec 026).
#[must_use]
pub fn pack_signing_payload(
    pack_id: &str,
    version: &str,
    pack_epoch: u64,
    content_digest_hex: &str,
    rights_uri: &str,
    sbom_ref: &str,
) -> Vec<u8> {
    format!(
        "MEDSCALE_PACK_SIGN_V1\n{pack_id}\n{version}\n{pack_epoch}\n{content_digest_hex}\n{rights_uri}\n{sbom_ref}\n"
    )
    .into_bytes()
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&SYNTHETIC_SIGNING_SEED)
}

fn verifying_key() -> VerifyingKey {
    signing_key().verifying_key()
}

/// Hex-encoded verifying key for the synthetic trust root (diagnostics / evidence).
#[must_use]
pub fn synthetic_verifying_key_hex() -> String {
    hex_encode(verifying_key().as_bytes())
}

/// Sign a Pack payload with the synthetic trust root (fixtures/tests only).
#[must_use]
pub fn sign_pack_payload(payload: &[u8]) -> String {
    let sig = signing_key().sign(payload);
    hex_encode(&sig.to_bytes())
}

/// Verify a Pack signature against the synthetic trust root.
pub fn verify_pack_signature(
    trust_root_id: &str,
    payload: &[u8],
    signature_hex: &str,
) -> Result<(), PackTrustError> {
    if trust_root_id != SYNTHETIC_PACK_TRUST_ROOT_ID {
        return Err(PackTrustError::UnknownTrustRoot);
    }
    let bytes = hex_decode(signature_hex).map_err(|_| PackTrustError::InvalidEncoding)?;
    if bytes.len() != Signature::BYTE_SIZE {
        return Err(PackTrustError::InvalidEncoding);
    }
    let mut arr = [0_u8; Signature::BYTE_SIZE];
    arr.copy_from_slice(&bytes);
    let sig = Signature::from_bytes(&arr);
    verifying_key()
        .verify(payload, &sig)
        .map_err(|_| PackTrustError::VerifyFailed)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, ()> {
    let hex = hex.trim();
    if !hex.len().is_multiple_of(2) {
        return Err(());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    for chunk in hex.as_bytes().chunks(2) {
        let s = std::str::from_utf8(chunk).map_err(|_| ())?;
        out.push(u8::from_str_radix(s, 16).map_err(|_| ())?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_sign_verify() {
        let payload = pack_signing_payload(
            "pack-a",
            "1.0.0",
            1,
            "aabb",
            "https://medscale.local/rights/synthetic-fixture",
            "sbom:x",
        );
        let sig = sign_pack_payload(&payload);
        verify_pack_signature(SYNTHETIC_PACK_TRUST_ROOT_ID, &payload, &sig).unwrap();
    }

    #[test]
    fn wrong_trust_root_rejected() {
        let payload = b"x";
        let sig = sign_pack_payload(payload);
        assert_eq!(
            verify_pack_signature("other", payload, &sig),
            Err(PackTrustError::UnknownTrustRoot)
        );
    }
}

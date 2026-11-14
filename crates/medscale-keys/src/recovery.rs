//! Recovery code generation and wrap helpers.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::aead_wrap::{WrappedBlob, seal, unseal};
use crate::provider::{KeyError, VaultDek};

/// Number of recovery codes generated at vault create.
pub const DEFAULT_RECOVERY_COUNT: usize = 10;

/// Generated recovery codes (show-once to operator; wraps stored in header).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryCodeSet {
    pub codes: Vec<String>,
}

/// Generate high-entropy recovery codes (hex of 16 random bytes each).
#[must_use]
pub fn generate_recovery_codes(count: usize) -> RecoveryCodeSet {
    let mut codes = Vec::with_capacity(count);
    let mut rng = rand::thread_rng();
    for _ in 0..count {
        let mut buf = [0u8; 16];
        rng.fill_bytes(&mut buf);
        codes.push(hex::encode(&buf));
    }
    RecoveryCodeSet { codes }
}

fn code_kek(code: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let digest = Sha256::digest(format!("medscale.recovery.v1:{code}").as_bytes());
    out.copy_from_slice(&digest);
    out
}

/// Wrap DEK under a recovery code.
pub fn wrap_with_code(dek: &VaultDek, code: &str, aad: &[u8]) -> Result<WrappedBlob, KeyError> {
    let kek = code_kek(code);
    seal(&kek, dek.as_bytes(), aad).map_err(|_| KeyError::Crypto)
}

/// Unwrap DEK with a recovery code.
pub fn unwrap_with_code(blob: &WrappedBlob, code: &str, aad: &[u8]) -> Result<VaultDek, KeyError> {
    let kek = code_kek(code);
    let bytes = unseal(&kek, blob, aad).map_err(|_| KeyError::MissingKeyMaterial)?;
    VaultDek::from_bytes(&bytes)
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

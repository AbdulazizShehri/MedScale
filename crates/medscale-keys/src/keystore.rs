//! KeyStore trait + Memory / OS / Fake OS stores (Specs 005 / 028).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use thiserror::Error;

/// Product service name for platform credential entries.
pub const OS_KEYRING_SERVICE: &str = "medscale";

/// Probe account used only for doctor/availability round-trips (never a real vault).
pub const OS_KEYRING_PROBE_ACCOUNT: &str = "medscale.vault.__doctor_probe__";

const OS_KEYRING_PROBE_SECRET: &[u8] = b"medscale-os-keyring-probe-v1";

/// Env var: when `1`/`true`, prefer MemoryKeyStore even if OS keyring probes OK.
pub const FORCE_MEMORY_KEYSTORE_ENV: &str = "MEDSCALE_FORCE_MEMORY_KEYSTORE";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KeyStoreError {
    #[error("not found")]
    NotFound,
    #[error("store error: {0}")]
    Other(String),
}

/// Stores wrapped unlock material only — never ambient secrets.
pub trait KeyStore: Send + Sync {
    fn put(&self, account: &str, secret: &[u8]) -> Result<(), KeyStoreError>;
    fn get(&self, account: &str) -> Result<Vec<u8>, KeyStoreError>;
    fn delete(&self, account: &str) -> Result<(), KeyStoreError>;
}

/// Policy constraints for OS/mobile key stores (Spec 009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileKeyStorePolicy {
    pub apple_synchronizable_allowed: bool,
    pub android_hardware_backed_preferred: bool,
}

impl MobileKeyStorePolicy {
    /// Canonical READY_BASE policy.
    #[must_use]
    pub const fn ready_base() -> Self {
        Self {
            apple_synchronizable_allowed: false,
            android_hardware_backed_preferred: true,
        }
    }

    #[must_use]
    pub const fn forbids_apple_icloud_key_sync(self) -> bool {
        !self.apple_synchronizable_allowed
    }
}

/// In-process mock / CI store.
#[derive(Debug, Default)]
pub struct MemoryKeyStore {
    inner: Mutex<HashMap<String, Vec<u8>>>,
}

impl MemoryKeyStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Wipe all entries (keyring loss simulation).
    pub fn wipe(&self) {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}

impl KeyStore for MemoryKeyStore {
    fn put(&self, account: &str, secret: &[u8]) -> Result<(), KeyStoreError> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(account.to_owned(), secret.to_vec());
        Ok(())
    }

    fn get(&self, account: &str) -> Result<Vec<u8>, KeyStoreError> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(account)
            .cloned()
            .ok_or(KeyStoreError::NotFound)
    }

    fn delete(&self, account: &str) -> Result<(), KeyStoreError> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(account);
        Ok(())
    }
}

/// OS credential-store wrapper (Windows Credential Manager / macOS Keychain / Linux keyutils).
///
/// Namespace for vault material: account = `medscale.vault.<vault_id>` under service `medscale`.
#[derive(Debug, Clone, Default)]
pub struct OsKeyStore;

impl OsKeyStore {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn entry(account: &str) -> Result<keyring::Entry, KeyStoreError> {
        keyring::Entry::new(OS_KEYRING_SERVICE, account)
            .map_err(|e| KeyStoreError::Other(e.to_string()))
    }

    /// Live put/get/delete round-trip against the platform store (fail-soft for CI).
    pub fn probe_roundtrip() -> Result<(), KeyStoreError> {
        let store = Self::new();
        store.put(OS_KEYRING_PROBE_ACCOUNT, OS_KEYRING_PROBE_SECRET)?;
        let got = store.get(OS_KEYRING_PROBE_ACCOUNT)?;
        let _ = store.delete(OS_KEYRING_PROBE_ACCOUNT);
        if got.as_slice() != OS_KEYRING_PROBE_SECRET {
            return Err(KeyStoreError::Other("os keyring probe mismatch".to_owned()));
        }
        Ok(())
    }

    /// Cached availability probe for doctor/runtime selection.
    #[must_use]
    pub fn is_available() -> bool {
        static AVAILABLE: OnceLock<bool> = OnceLock::new();
        *AVAILABLE.get_or_init(|| Self::probe_roundtrip().is_ok())
    }
}

impl KeyStore for OsKeyStore {
    fn put(&self, account: &str, secret: &[u8]) -> Result<(), KeyStoreError> {
        let entry = Self::entry(account)?;
        entry
            .set_secret(secret)
            .map_err(|e| KeyStoreError::Other(e.to_string()))
    }

    fn get(&self, account: &str) -> Result<Vec<u8>, KeyStoreError> {
        let entry = Self::entry(account)?;
        match entry.get_secret() {
            Ok(bytes) => Ok(bytes),
            Err(keyring::Error::NoEntry) => Err(KeyStoreError::NotFound),
            Err(e) => Err(KeyStoreError::Other(e.to_string())),
        }
    }

    fn delete(&self, account: &str) -> Result<(), KeyStoreError> {
        let entry = Self::entry(account)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(KeyStoreError::Other(e.to_string())),
        }
    }
}

/// In-process stand-in that mirrors the OS `(service, account)` boundary for unit tests.
///
/// Does **not** talk to a real credential store; proves KeyProvider/API wiring without CI daemons.
#[derive(Debug, Default)]
pub struct FakeOsKeyStore {
    service: String,
    inner: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeOsKeyStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            service: OS_KEYRING_SERVICE.to_owned(),
            inner: Mutex::new(HashMap::new()),
        }
    }

    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    fn namespaced_key(&self, account: &str) -> String {
        format!("{}::{account}", self.service)
    }
}

impl KeyStore for FakeOsKeyStore {
    fn put(&self, account: &str, secret: &[u8]) -> Result<(), KeyStoreError> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(self.namespaced_key(account), secret.to_vec());
        Ok(())
    }

    fn get(&self, account: &str) -> Result<Vec<u8>, KeyStoreError> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&self.namespaced_key(account))
            .cloned()
            .ok_or(KeyStoreError::NotFound)
    }

    fn delete(&self, account: &str) -> Result<(), KeyStoreError> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.namespaced_key(account));
        Ok(())
    }
}

/// Whether runtime should prefer OS keyring when available.
#[must_use]
pub fn prefer_os_keystore() -> bool {
    match std::env::var(FORCE_MEMORY_KEYSTORE_ENV) {
        Ok(v) => {
            let t = v.trim();
            !(t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes"))
        }
        Err(_) => true,
    }
}

/// Doctor/runtime keystore posture (no secrets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyStoreDoctorPosture {
    pub os_keyring_available: bool,
    pub os_keyring_used: bool,
}

impl KeyStoreDoctorPosture {
    /// Probe OS availability and apply Memory force-env preference.
    #[must_use]
    pub fn detect() -> Self {
        let os_keyring_available = OsKeyStore::is_available();
        let os_keyring_used = prefer_os_keystore() && os_keyring_available;
        Self {
            os_keyring_available,
            os_keyring_used,
        }
    }
}

/// Select Memory or OS keystore for EncryptedVault / KeyProvider paths.
#[must_use]
pub fn select_keystore() -> Box<dyn KeyStore> {
    let posture = KeyStoreDoctorPosture::detect();
    if posture.os_keyring_used {
        Box::new(OsKeyStore::new())
    } else {
        Box::new(MemoryKeyStore::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::KeyProvider;

    #[test]
    fn memory_keystore_roundtrip() {
        let store = MemoryKeyStore::new();
        store.put("medscale.vault.t1", b"abc").unwrap();
        assert_eq!(store.get("medscale.vault.t1").unwrap(), b"abc");
        store.delete("medscale.vault.t1").unwrap();
        assert!(store.get("medscale.vault.t1").is_err());
    }

    #[test]
    fn fake_os_keystore_namespace_boundary() {
        let store = FakeOsKeyStore::new();
        assert_eq!(store.service(), OS_KEYRING_SERVICE);
        let account = "medscale.vault.fake-v1";
        store.put(account, b"wrapped-dek").unwrap();
        assert_eq!(store.get(account).unwrap(), b"wrapped-dek");
        store.delete(account).unwrap();
        assert_eq!(store.get(account), Err(KeyStoreError::NotFound));
    }

    #[test]
    fn fake_os_keystore_keyprovider_roundtrip() {
        let store = FakeOsKeyStore::new();
        let (dek, header, _) = KeyProvider::create_vault_keys("fake-v2", "pw").unwrap();
        KeyProvider::store_wrapped_dek(&store, "fake-v2", &dek).unwrap();
        assert_eq!(
            KeyProvider::keystore_account("fake-v2"),
            "medscale.vault.fake-v2"
        );
        let unlocked = KeyProvider::unlock_keystore(&store, &header).unwrap();
        assert_eq!(unlocked.as_bytes(), dek.as_bytes());
    }

    #[test]
    fn os_keystore_roundtrip_or_skip_if_unavailable() {
        match OsKeyStore::probe_roundtrip() {
            Ok(()) => {
                let store = OsKeyStore::new();
                let account = "medscale.vault.spec028-ci-rt";
                let secret = b"os-keyring-rt-secret";
                store.put(account, secret).unwrap();
                assert_eq!(store.get(account).unwrap(), secret);
                store.delete(account).unwrap();
                assert_eq!(store.get(account), Err(KeyStoreError::NotFound));
            }
            Err(e) => {
                // Fail-soft: Linux CI without keyutils/secret-service, headless agents, etc.
                eprintln!("OsKeyStore unavailable (documented fail-soft): {e}");
            }
        }
    }
}

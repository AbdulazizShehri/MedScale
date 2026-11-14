//! In-process client session registry (Spec 018 READY_BASE).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use medscale_contracts::envelopes::{AuthorityError, Capability};
use medscale_contracts::objects::{OpaqueId, VaultId};

#[derive(Debug, Clone)]
struct ClientSession {
    vault_id: VaultId,
    /// Lease holder that opened the session (FR-001; audit identity for 074).
    holder_id: OpaqueId,
    granted: Vec<Capability>,
    expires_at_tick: u64,
    revoked: bool,
}

/// Monotonic-tick session store bound to vault lease holders.
#[derive(Debug, Default)]
pub struct SessionRegistry {
    inner: Mutex<SessionState>,
}

#[derive(Debug, Default)]
struct SessionState {
    clock: u64,
    next_seq: u64,
    sessions: HashMap<String, ClientSession>,
}

impl SessionRegistry {
    /// Creates an empty registry at tick 0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, SessionState> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Opens a session; returns `(session_id, expires_at_tick)`.
    pub fn open(
        &self,
        vault_id: &VaultId,
        holder_id: OpaqueId,
        granted: Vec<Capability>,
        ttl_ticks: u64,
    ) -> (OpaqueId, u64) {
        let mut state = self.lock();
        state.next_seq = state.next_seq.saturating_add(1);
        // A session id is a bearer token, so it is random, not a counter.
        let random: String = medscale_keys::generate_key32()[..16]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let session_id = OpaqueId::new(format!("session-{random}"));
        let expires_at_tick = state.clock.saturating_add(ttl_ticks);
        state.sessions.insert(
            session_id.as_str().to_owned(),
            ClientSession {
                vault_id: vault_id.clone(),
                holder_id,
                granted,
                expires_at_tick,
                revoked: false,
            },
        );
        (session_id, expires_at_tick)
    }

    /// Marks a session revoked (idempotent if unknown).
    pub fn revoke(&self, session_id: &OpaqueId) {
        let mut state = self.lock();
        if let Some(session) = state.sessions.get_mut(session_id.as_str()) {
            session.revoked = true;
        }
    }

    /// Revokes every session bound to `holder_id` (Spec 084 device
    /// revocation).
    pub fn revoke_holder(&self, holder_id: &OpaqueId) {
        let mut state = self.lock();
        for session in state.sessions.values_mut() {
            if &session.holder_id == holder_id {
                session.revoked = true;
            }
        }
    }

    /// Validates a live session for vault + capability.
    pub fn validate(
        &self,
        session_id: &OpaqueId,
        vault_id: &VaultId,
        capability: Capability,
    ) -> Result<(), AuthorityError> {
        let state = self.lock();
        let Some(session) = state.sessions.get(session_id.as_str()) else {
            return Err(AuthorityError::SessionDenied);
        };
        if session.revoked {
            return Err(AuthorityError::SessionRevoked);
        }
        if state.clock >= session.expires_at_tick {
            return Err(AuthorityError::SessionExpired);
        }
        if &session.vault_id != vault_id {
            return Err(AuthorityError::SessionDenied);
        }
        if !session.granted.contains(&capability) {
            return Err(AuthorityError::SessionDenied);
        }
        Ok(())
    }

    /// Returns the lease holder bound to a live session, if present.
    ///
    /// Spec 074 audit identity: the holder is the authenticated actor for Core
    /// mutations. Revoked/expired/unknown sessions yield `None`.
    #[must_use]
    pub fn holder_of(&self, session_id: &OpaqueId) -> Option<OpaqueId> {
        let state = self.lock();
        let session = state.sessions.get(session_id.as_str())?;
        if session.revoked || state.clock >= session.expires_at_tick {
            return None;
        }
        Some(session.holder_id.clone())
    }

    /// Advances the monotonic clock by one tick (tests).
    pub fn tick(&self) {
        self.advance_ticks(1);
    }
    /// Advances the monotonic clock by `n` ticks (tests).
    pub fn advance_ticks(&self, n: u64) {
        let mut state = self.lock();
        state.clock = state.clock.saturating_add(n);
    }

    /// Current monotonic tick (tests / diagnostics).
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.lock().clock
    }
}

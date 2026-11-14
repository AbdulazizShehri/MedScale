//! Transports for Spec 090 institutional adapters.
//!
//! Product egress is default-deny and no network client is admitted for
//! institutional systems, so the product transport is
//! [`UnavailableTransport`]: every call reports `unreachable` and nothing
//! leaves the machine. [`InProcessStore`] is an institutional object store
//! that lives in this process, with fault injection, used to qualify the
//! adapter lifecycle (success, outage, timeout after send, rejection,
//! reconciliation). It is never wired to a network.

use std::collections::BTreeMap;
use std::sync::Mutex;

use medscale_contracts::institutional::TransportOutcome;
use medscale_contracts::objects::DigestSha256;

/// Moves one payload to an institutional destination.
pub trait InstitutionalTransport: Send + Sync + std::fmt::Debug {
    /// Stores `bytes` at `key` under `idempotency_key`. Must not store
    /// different bytes for a key it has already stored.
    fn put(
        &self,
        destination: &str,
        key: &str,
        idempotency_key: &DigestSha256,
        bytes: &[u8],
    ) -> TransportOutcome;

    /// Reports whether `key` holds exactly `expected`.
    fn head(&self, destination: &str, key: &str, expected: &DigestSha256) -> TransportOutcome;
}

/// The product transport: nothing is reachable.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailableTransport;

impl InstitutionalTransport for UnavailableTransport {
    fn put(&self, _: &str, _: &str, _: &DigestSha256, _: &[u8]) -> TransportOutcome {
        TransportOutcome::Unreachable
    }

    fn head(&self, _: &str, _: &str, _: &DigestSha256) -> TransportOutcome {
        TransportOutcome::Unreachable
    }
}

/// Faults the in-process store can simulate on its next calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fault {
    #[default]
    None,
    /// Connection fails: nothing stored.
    Outage,
    /// The write happens but the answer is lost.
    StoreThenTimeOut,
    /// The answer is lost and the write did not happen.
    TimeOutWithoutStore,
    /// The destination refuses.
    Reject,
}

#[derive(Debug, Default)]
struct StoreState {
    objects: BTreeMap<(String, String), (DigestSha256, Vec<u8>)>,
    fault: Fault,
    puts: u64,
}

/// An institutional object store inside this process (qualification only).
#[derive(Debug, Default)]
pub struct InProcessStore {
    state: Mutex<StoreState>,
}

impl InProcessStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, StoreState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Applies `fault` to every following call until changed.
    pub fn set_fault(&self, fault: Fault) {
        self.lock().fault = fault;
    }

    /// Stored bytes at `(destination, key)`.
    #[must_use]
    pub fn object(&self, destination: &str, key: &str) -> Option<Vec<u8>> {
        self.lock()
            .objects
            .get(&(destination.to_owned(), key.to_owned()))
            .map(|(_, b)| b.clone())
    }

    /// Number of `put` calls that reached the store.
    #[must_use]
    pub fn puts(&self) -> u64 {
        self.lock().puts
    }
}

impl InstitutionalTransport for InProcessStore {
    fn put(
        &self,
        destination: &str,
        key: &str,
        _idempotency_key: &DigestSha256,
        bytes: &[u8],
    ) -> TransportOutcome {
        let mut s = self.lock();
        match s.fault {
            Fault::Outage => return TransportOutcome::Unreachable,
            Fault::Reject => return TransportOutcome::Rejected,
            Fault::TimeOutWithoutStore => {
                s.puts += 1;
                return TransportOutcome::TimedOut;
            }
            Fault::None | Fault::StoreThenTimeOut => {}
        }
        s.puts += 1;
        let digest = DigestSha256::of(bytes);
        let slot = (destination.to_owned(), key.to_owned());
        let outcome = match s.objects.get(&slot) {
            Some((d, _)) if *d == digest => TransportOutcome::AlreadyStored,
            Some(_) => return TransportOutcome::Rejected,
            None => {
                s.objects.insert(slot, (digest, bytes.to_vec()));
                TransportOutcome::Stored
            }
        };
        if s.fault == Fault::StoreThenTimeOut {
            TransportOutcome::TimedOut
        } else {
            outcome
        }
    }

    fn head(&self, destination: &str, key: &str, expected: &DigestSha256) -> TransportOutcome {
        let s = self.lock();
        if s.fault == Fault::Outage {
            return TransportOutcome::Unreachable;
        }
        match s.objects.get(&(destination.to_owned(), key.to_owned())) {
            Some((d, _)) if d == expected => TransportOutcome::PresentMatching,
            Some(_) => TransportOutcome::PresentDifferent,
            None => TransportOutcome::Absent,
        }
    }
}

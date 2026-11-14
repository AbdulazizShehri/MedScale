//! Budgeted residency for prepared (loaded) models (Spec 103 LOAD/UNLOAD).
//!
//! Policy adapted from OpenMed `openmed/core/model_cache_policy.py` at commit
//! `ea920f36fadd7b45935247d639f0ffa1ef493b23` (Apache-2.0), which manages a
//! disk-cache quota:
//! - only explicitly registered entries are evictable;
//! - pinned entries are never evicted;
//! - eviction is least-recently-used by access marker;
//! - an eviction plan is computed before anything is removed, and the request
//!   fails closed when the budget cannot be met.
//!
//! Here the budget is the estimated resident memory of loaded models, and the
//! access marker is a deterministic counter (no wall clock), so plans are
//! reproducible.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResidencyError {
    #[error("{key}: needs {needed} bytes, budget is {budget}")]
    LargerThanBudget {
        key: String,
        needed: u64,
        budget: u64,
    },
    #[error("cannot free {needed} bytes: pinned or in-use entries hold the budget")]
    BudgetHeldByPinned { needed: u64 },
    #[error("{0} is not resident")]
    NotResident(String),
    #[error("{0} is pinned")]
    Pinned(String),
}

#[derive(Debug)]
struct Entry<T> {
    value: T,
    bytes: u64,
    pinned: bool,
    last_access: u64,
}

/// What a `load` would evict, in order, before inserting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictionPlan {
    pub evict: Vec<String>,
    pub freed_bytes: u64,
}

/// Loaded models keyed by Pack id, bounded by an estimated byte budget.
#[derive(Debug)]
pub struct ResidencyPool<T> {
    budget_bytes: u64,
    clock: u64,
    entries: BTreeMap<String, Entry<T>>,
}

impl<T> ResidencyPool<T> {
    pub fn new(budget_bytes: u64) -> Self {
        Self {
            budget_bytes,
            clock: 0,
            entries: BTreeMap::new(),
        }
    }

    pub fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }

    pub fn used_bytes(&self) -> u64 {
        self.entries.values().map(|e| e.bytes).sum()
    }

    pub fn resident(&self) -> Vec<(&str, u64, bool)> {
        self.entries
            .iter()
            .map(|(k, e)| (k.as_str(), e.bytes, e.pinned))
            .collect()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// Plans evictions needed to fit `bytes` for `key` (replacing `key` itself
    /// if it is already resident). Least recently used unpinned entries first.
    pub fn plan(&self, key: &str, bytes: u64) -> Result<EvictionPlan, ResidencyError> {
        if bytes > self.budget_bytes {
            return Err(ResidencyError::LargerThanBudget {
                key: key.into(),
                needed: bytes,
                budget: self.budget_bytes,
            });
        }
        let used = self.used_bytes() - self.entries.get(key).map_or(0, |e| e.bytes);
        let mut over = (used + bytes).saturating_sub(self.budget_bytes);
        let mut candidates: Vec<(&String, &Entry<T>)> = self
            .entries
            .iter()
            .filter(|(k, e)| !e.pinned && k.as_str() != key)
            .collect();
        candidates.sort_by_key(|(k, e)| (e.last_access, (*k).clone()));
        let mut plan = EvictionPlan {
            evict: Vec::new(),
            freed_bytes: 0,
        };
        for (k, e) in candidates {
            if over == 0 {
                break;
            }
            plan.evict.push(k.clone());
            plan.freed_bytes += e.bytes;
            over = over.saturating_sub(e.bytes);
        }
        if over > 0 {
            return Err(ResidencyError::BudgetHeldByPinned { needed: over });
        }
        Ok(plan)
    }

    /// Inserts a loaded model after evicting per [`Self::plan`]. Returns the
    /// evicted keys. Nothing is evicted when the plan fails.
    pub fn load(&mut self, key: &str, bytes: u64, value: T) -> Result<Vec<String>, ResidencyError> {
        let plan = self.plan(key, bytes)?;
        for k in &plan.evict {
            self.entries.remove(k);
        }
        let last_access = self.tick();
        let pinned = self.entries.get(key).is_some_and(|e| e.pinned);
        self.entries.insert(
            key.into(),
            Entry {
                value,
                bytes,
                pinned,
                last_access,
            },
        );
        Ok(plan.evict)
    }

    /// Returns a resident model and marks it as recently used.
    pub fn get(&mut self, key: &str) -> Option<&T> {
        let now = self.tick();
        self.entries.get_mut(key).map(|e| {
            e.last_access = now;
            &e.value
        })
    }

    pub fn set_pinned(&mut self, key: &str, pinned: bool) -> Result<(), ResidencyError> {
        let e = self
            .entries
            .get_mut(key)
            .ok_or_else(|| ResidencyError::NotResident(key.into()))?;
        e.pinned = pinned;
        Ok(())
    }

    /// Explicit unload. Pinned entries must be unpinned first.
    pub fn unload(&mut self, key: &str) -> Result<T, ResidencyError> {
        match self.entries.get(key) {
            None => Err(ResidencyError::NotResident(key.into())),
            Some(e) if e.pinned => Err(ResidencyError::Pinned(key.into())),
            Some(_) => Ok(self.entries.remove(key).map(|e| e.value).expect("checked")),
        }
    }

    /// Unloads every unpinned entry; returns their keys.
    pub fn unload_all_unpinned(&mut self) -> Vec<String> {
        let keys: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| !e.pinned)
            .map(|(k, _)| k.clone())
            .collect();
        for k in &keys {
            self.entries.remove(k);
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used_unpinned_entries_first() {
        let mut pool = ResidencyPool::new(100);
        assert!(pool.load("a", 40, 'a').unwrap().is_empty());
        assert!(pool.load("b", 40, 'b').unwrap().is_empty());
        assert_eq!(pool.get("a"), Some(&'a')); // a is now more recent than b
        assert_eq!(pool.load("c", 40, 'c').unwrap(), ["b"]);
        assert!(pool.contains("a") && pool.contains("c") && !pool.contains("b"));
        assert_eq!(pool.used_bytes(), 80);
    }

    #[test]
    fn pinned_entries_are_never_evicted_and_failed_plans_change_nothing() {
        let mut pool = ResidencyPool::new(100);
        pool.load("a", 60, 1).unwrap();
        pool.set_pinned("a", true).unwrap();
        pool.load("b", 30, 2).unwrap();
        assert_eq!(
            pool.load("c", 50, 3),
            Err(ResidencyError::BudgetHeldByPinned { needed: 10 })
        );
        assert!(pool.contains("a") && pool.contains("b") && !pool.contains("c"));
        assert_eq!(pool.unload("a"), Err(ResidencyError::Pinned("a".into())));
        pool.set_pinned("a", false).unwrap();
        assert_eq!(pool.load("c", 50, 3).unwrap(), ["a"]);
    }

    #[test]
    fn oversize_requests_fail_closed() {
        let mut pool: ResidencyPool<()> = ResidencyPool::new(100);
        assert!(matches!(
            pool.load("huge", 101, ()),
            Err(ResidencyError::LargerThanBudget { .. })
        ));
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn reloading_a_key_replaces_it_without_evicting_others() {
        let mut pool = ResidencyPool::new(100);
        pool.load("a", 50, 1).unwrap();
        pool.load("b", 50, 2).unwrap();
        assert!(pool.load("a", 50, 3).unwrap().is_empty());
        assert_eq!(pool.get("a"), Some(&3));
        assert_eq!(pool.used_bytes(), 100);
    }

    #[test]
    fn plans_are_deterministic_and_unload_is_explicit() {
        let mut pool = ResidencyPool::new(90);
        for k in ["x", "y", "z"] {
            pool.load(k, 30, ()).unwrap();
        }
        assert_eq!(
            pool.plan("w", 60).unwrap(),
            EvictionPlan {
                evict: vec!["x".into(), "y".into()],
                freed_bytes: 60
            }
        );
        assert!(pool.unload("y").is_ok());
        assert_eq!(
            pool.unload("y"),
            Err(ResidencyError::NotResident("y".into()))
        );
        pool.set_pinned("z", true).unwrap();
        assert_eq!(pool.unload_all_unpinned(), ["x"]);
        assert_eq!(pool.resident(), [("z", 30, true)]);
    }
}

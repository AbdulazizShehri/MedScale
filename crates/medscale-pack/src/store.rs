//! In-memory / path-backed admitted pack store (Spec 026 anti-rollback).

use medscale_contracts::objects::OpaqueId;
use medscale_contracts::packs::{PackManifestV0, PackPromotionState};

use crate::format::AdmitError;

/// Admitted pack registry (Core Host owned).
#[derive(Debug, Default, Clone)]
pub struct PackStore {
    packs: Vec<PackManifestV0>,
}

impl PackStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace when anti-rollback allows.
    pub fn admit(&mut self, manifest: PackManifestV0) -> Result<(), AdmitError> {
        if let Some(existing) = self.packs.iter().find(|p| p.pack_id == manifest.pack_id)
            && is_rollback(existing, &manifest)
        {
            return Err(AdmitError::AntiRollback);
        }
        self.insert_unchecked(manifest);
        Ok(())
    }

    /// Legacy insert without anti-rollback (tests only prefer `admit`).
    pub fn insert(&mut self, manifest: PackManifestV0) {
        let _ = self.admit(manifest);
    }

    fn insert_unchecked(&mut self, manifest: PackManifestV0) {
        if let Some(existing) = self
            .packs
            .iter_mut()
            .find(|p| p.pack_id == manifest.pack_id)
        {
            *existing = manifest;
        } else {
            self.packs.push(manifest);
        }
    }

    #[must_use]
    pub fn list(&self) -> Vec<PackManifestV0> {
        self.packs.clone()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.packs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    #[must_use]
    pub fn get(&self, pack_id: &OpaqueId) -> Option<PackManifestV0> {
        self.packs
            .iter()
            .find(|pack| &pack.pack_id == pack_id)
            .cloned()
    }

    #[must_use]
    pub fn current_pack_id(&self) -> Option<OpaqueId> {
        self.packs
            .iter()
            .find(|p| p.promotion_state == PackPromotionState::Current)
            .map(|p| p.pack_id.clone())
    }

    /// Promote pack; fail closed if target invalid.
    pub fn promote(
        &mut self,
        pack_id: &OpaqueId,
        to: PackPromotionState,
    ) -> Result<PackManifestV0, &'static str> {
        let idx = self
            .packs
            .iter()
            .position(|p| &p.pack_id == pack_id)
            .ok_or("pack not found")?;
        match to {
            PackPromotionState::Current => {
                if self.packs[idx].promotion_state != PackPromotionState::Candidate
                    && self.packs[idx].promotion_state != PackPromotionState::Canary
                {
                    return Err("current requires candidate or canary");
                }
                for p in &mut self.packs {
                    if p.promotion_state == PackPromotionState::Current {
                        p.promotion_state = PackPromotionState::LastGreen;
                    }
                }
                self.packs[idx].promotion_state = PackPromotionState::Current;
            }
            PackPromotionState::Canary
            | PackPromotionState::LastGreen
            | PackPromotionState::Candidate => {
                self.packs[idx].promotion_state = to;
            }
        }
        Ok(self.packs[idx].clone())
    }
}

fn is_rollback(existing: &PackManifestV0, incoming: &PackManifestV0) -> bool {
    if incoming.pack_epoch < existing.pack_epoch {
        return true;
    }
    if incoming.pack_epoch > existing.pack_epoch {
        return false;
    }
    // Equal epoch: reject strictly lower version; equal version is idempotent re-admit.
    version_cmp(&incoming.version, &existing.version) == std::cmp::Ordering::Less
}

fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let va = parse(a);
    let vb = parse(b);
    let n = va.len().max(vb.len());
    for i in 0..n {
        let x = va.get(i).copied().unwrap_or(0);
        let y = vb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

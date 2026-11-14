//! In-memory scoped object store.

use std::collections::HashMap;

use medscale_contracts::objects::AmendmentRecord;
use medscale_contracts::objects::{
    ActionAuditRecord, AuthorityScopeId, ClinicalAssertion, DerivedSourceArtifact,
    EvaluationRecord, IdentityAssertion, IdentityMergeDecision, OpaqueId, Projection, Proposal,
    RealmId, SourceRecord,
};
use serde_json::Value;

/// Stored object variants for read-back.
#[derive(Debug, Clone)]
pub enum StoredObject {
    Source(SourceRecord),
    Derived(DerivedSourceArtifact),
    Proposal(Proposal),
    Assertion(ClinicalAssertion),
    Audit(ActionAuditRecord),
    Identity(IdentityAssertion),
    Merge(IdentityMergeDecision),
    Evaluation(EvaluationRecord),
    Projection(Projection),
    Amendment(AmendmentRecord),
}

impl StoredObject {
    pub fn to_json(&self) -> Value {
        match self {
            Self::Source(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Derived(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Proposal(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Assertion(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Audit(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Identity(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Merge(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Evaluation(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Projection(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            Self::Amendment(v) => serde_json::to_value(v).unwrap_or(Value::Null),
        }
    }

    fn scope_ids(&self) -> (&RealmId, &AuthorityScopeId, &OpaqueId) {
        match self {
            Self::Source(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Derived(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Proposal(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Assertion(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Audit(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Identity(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Merge(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Evaluation(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Projection(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
            Self::Amendment(v) => (
                &v.header.realm_id,
                &v.header.authority_scope_id,
                &v.header.id,
            ),
        }
    }
}

/// In-memory store keyed by object id with scope checks on read.
#[derive(Debug, Default)]
pub struct InMemoryAuthorityStore {
    objects: HashMap<String, StoredObject>,
    next_seq: u64,
}

impl InMemoryAuthorityStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc_id(&mut self, prefix: &str) -> OpaqueId {
        self.next_seq += 1;
        OpaqueId::new(format!("{prefix}-{}", self.next_seq))
    }

    pub fn insert(&mut self, object: StoredObject) {
        let id = object.scope_ids().2.as_str().to_owned();
        self.objects.insert(id, object);
    }

    #[must_use]
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn set_next_seq(&mut self, next_seq: u64) {
        self.next_seq = next_seq;
    }

    pub fn clear(&mut self) {
        self.objects.clear();
        self.next_seq = 0;
    }

    pub fn get_scoped(
        &self,
        id: &OpaqueId,
        realm_id: &RealmId,
        scope_id: &AuthorityScopeId,
    ) -> Result<&StoredObject, ScopeError> {
        let Some(obj) = self.objects.get(id.as_str()) else {
            return Err(ScopeError::NotFound);
        };
        let (r, s, _) = obj.scope_ids();
        if r != realm_id || s != scope_id {
            return Err(ScopeError::WrongScope);
        }
        Ok(obj)
    }

    pub fn get_proposal(
        &self,
        id: &OpaqueId,
        realm_id: &RealmId,
        scope_id: &AuthorityScopeId,
    ) -> Result<&Proposal, ScopeError> {
        match self.get_scoped(id, realm_id, scope_id)? {
            StoredObject::Proposal(p) => Ok(p),
            _ => Err(ScopeError::NotFound),
        }
    }

    pub fn get_source(
        &self,
        id: &OpaqueId,
        realm_id: &RealmId,
        scope_id: &AuthorityScopeId,
    ) -> Result<&SourceRecord, ScopeError> {
        match self.get_scoped(id, realm_id, scope_id)? {
            StoredObject::Source(s) => Ok(s),
            _ => Err(ScopeError::NotFound),
        }
    }

    pub fn get_audit_mut(&mut self, id: &OpaqueId) -> Option<&mut ActionAuditRecord> {
        match self.objects.get_mut(id.as_str()) {
            Some(StoredObject::Audit(a)) => Some(a),
            _ => None,
        }
    }

    /// External-action intents in this realm/scope (outbox projection).
    #[must_use]
    pub fn list_external_action_intents(
        &self,
        realm_id: &RealmId,
        scope_id: &AuthorityScopeId,
    ) -> Vec<&ActionAuditRecord> {
        self.objects
            .values()
            .filter_map(|o| match o {
                StoredObject::Audit(a)
                    if a.kind
                        == medscale_contracts::objects::ActionAuditKind::ExternalActionIntent
                        && &a.header.realm_id == realm_id
                        && &a.header.authority_scope_id == scope_id =>
                {
                    Some(a)
                }
                _ => None,
            })
            .collect()
    }

    /// Disclosure records appended in this realm/scope (Spec 021).
    #[must_use]
    pub fn list_disclosures(
        &self,
        realm_id: &RealmId,
        scope_id: &AuthorityScopeId,
    ) -> Vec<medscale_contracts::workflow::DisclosureRecord> {
        self.objects
            .values()
            .filter_map(|o| match o {
                StoredObject::Audit(a)
                    if a.action == medscale_contracts::workflow::DISCLOSURE_APPEND_ACTION
                        && &a.header.realm_id == realm_id
                        && &a.header.authority_scope_id == scope_id =>
                {
                    a.detail.as_ref().and_then(|d| {
                        serde_json::from_value::<medscale_contracts::workflow::DisclosureRecord>(
                            d.clone(),
                        )
                        .ok()
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Raw object map for presentation orchestration (read-only scans).
    #[must_use]
    pub fn objects_raw(&self) -> &HashMap<String, StoredObject> {
        &self.objects
    }

    /// All ClinicalAssertions for a subject (unordered).
    #[must_use]
    pub fn assertions_for_subject(&self, subject_ref: &OpaqueId) -> Vec<ClinicalAssertion> {
        self.objects
            .values()
            .filter_map(|o| match o {
                StoredObject::Assertion(a) if &a.subject_ref == subject_ref => Some(a.clone()),
                _ => None,
            })
            .collect()
    }

    /// Assertion ids for built_from lists.
    #[must_use]
    pub fn assertion_ids_for_subject(&self, subject_ref: &OpaqueId) -> Vec<OpaqueId> {
        self.assertions_for_subject(subject_ref)
            .into_iter()
            .map(|a| a.header.id)
            .collect()
    }

    /// OpaqueIds of assertions that have been superseded or retracted (append-only lineage).
    #[must_use]
    pub fn superseded_assertion_ids(&self) -> std::collections::HashSet<String> {
        self.objects
            .values()
            .filter_map(|o| match o {
                StoredObject::Amendment(a) => Some(a.prior_assertion_id.as_str().to_owned()),
                _ => None,
            })
            .collect()
    }

    #[must_use]
    pub fn identity_assertions(&self) -> Vec<IdentityAssertion> {
        self.objects
            .values()
            .filter_map(|o| match o {
                StoredObject::Identity(a) => Some(a.clone()),
                _ => None,
            })
            .collect()
    }

    #[must_use]
    pub fn identity_merges(&self) -> Vec<IdentityMergeDecision> {
        self.objects
            .values()
            .filter_map(|o| match o {
                StoredObject::Merge(m) => Some(m.clone()),
                _ => None,
            })
            .collect()
    }

    #[must_use]
    pub fn amendments(&self) -> Vec<AmendmentRecord> {
        self.objects
            .values()
            .filter_map(|o| match o {
                StoredObject::Amendment(a) => Some(a.clone()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeError {
    NotFound,
    WrongScope,
}

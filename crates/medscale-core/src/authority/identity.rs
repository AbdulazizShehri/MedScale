//! Identity assertion and explicit merge (no silent merge).

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::objects::{
    AuthorityScopeId, IdentityAssertion, IdentityMergeDecision, ObjectHeader, OpaqueId, RealmId,
};

use super::store::{InMemoryAuthorityStore, StoredObject};

pub fn create_identity_assertion(
    store: &mut InMemoryAuthorityStore,
    realm_id: RealmId,
    authority_scope_id: AuthorityScopeId,
    subject_id: OpaqueId,
    identifier_system: String,
    identifier_value: String,
) -> IdentityAssertion {
    let id = store.alloc_id("ident");
    let record = IdentityAssertion {
        header: ObjectHeader {
            id,
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id,
        },
        subject_id,
        identifier_system,
        identifier_value,
        confidence: None,
        evidence_refs: Vec::new(),
    };
    store.insert(StoredObject::Identity(record.clone()));
    record
}

pub fn decide_identity_merge(
    store: &mut InMemoryAuthorityStore,
    realm_id: RealmId,
    authority_scope_id: AuthorityScopeId,
    surviving_subject_id: OpaqueId,
    merged_subject_ids: Vec<OpaqueId>,
    authorized_by: OpaqueId,
    rationale: String,
) -> Result<IdentityMergeDecision, &'static str> {
    if merged_subject_ids.is_empty() {
        return Err("merged_subject_ids required; silent merge forbidden");
    }
    if rationale.trim().is_empty() {
        return Err("rationale required for explicit merge");
    }
    let id = store.alloc_id("merge");
    let record = IdentityMergeDecision {
        header: ObjectHeader {
            id,
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id,
        },
        surviving_subject_id,
        merged_subject_ids,
        authorized_by,
        rationale,
        evidence_refs: Vec::new(),
    };
    store.insert(StoredObject::Merge(record.clone()));
    Ok(record)
}

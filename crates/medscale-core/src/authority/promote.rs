//! Authorized proposal → clinical assertion promotion.

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::objects::{
    ActionAuditKind, ActionAuditRecord, ClinicalAssertion, MedicalTime, ObjectHeader, OpaqueId,
    Proposal, TimePrecision,
};

use super::store::{InMemoryAuthorityStore, ScopeError, StoredObject};

pub fn promote_proposal(
    store: &mut InMemoryAuthorityStore,
    proposal_id: &OpaqueId,
    authorized_by: OpaqueId,
    subject_ref: OpaqueId,
    realm_id: &medscale_contracts::objects::RealmId,
    scope_id: &medscale_contracts::objects::AuthorityScopeId,
) -> Result<(ClinicalAssertion, ActionAuditRecord), PromoteError> {
    let proposal: Proposal = store
        .get_proposal(proposal_id, realm_id, scope_id)
        .map_err(PromoteError::from)?
        .clone();

    let assertion_id = store.alloc_id("assert");
    let audit_id = store.alloc_id("audit");
    let recorded = MedicalTime::new("1970-01-01T00:00:00Z", TimePrecision::Instant, false);

    let assertion = ClinicalAssertion {
        header: ObjectHeader {
            id: assertion_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: proposal.header.realm_id.clone(),
            authority_scope_id: proposal.header.authority_scope_id.clone(),
        },
        subject_ref,
        claim_kind: proposal.claim_kind.clone(),
        payload: proposal.payload.clone(),
        promoted_from_proposal_id: Some(proposal.header.id.clone()),
        authorized_by: authorized_by.clone(),
        effective_time: None,
        recorded_time: recorded,
    };

    let audit = ActionAuditRecord {
        header: ObjectHeader {
            id: audit_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: proposal.header.realm_id.clone(),
            authority_scope_id: proposal.header.authority_scope_id.clone(),
        },
        kind: ActionAuditKind::Audit,
        actor: authorized_by,
        action: "promote_proposal".to_owned(),
        target_refs: vec![proposal.header.id, assertion_id.clone()],
        effect_state: None,
        payload_digest: None,
        detail: None,
    };

    store.insert(StoredObject::Assertion(assertion.clone()));
    store.insert(StoredObject::Audit(audit.clone()));
    let _ = audit_id;
    Ok((assertion, audit))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromoteError {
    NotFound,
    WrongScope,
}

impl From<ScopeError> for PromoteError {
    fn from(value: ScopeError) -> Self {
        match value {
            ScopeError::NotFound => Self::NotFound,
            ScopeError::WrongScope => Self::WrongScope,
        }
    }
}

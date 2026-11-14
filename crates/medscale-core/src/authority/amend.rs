//! AmendAssertion: append-only supersession / retraction lineage (Spec 019).

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::objects::{
    ActionAuditKind, ActionAuditRecord, AmendmentKind, AmendmentRecord, ClinicalAssertion,
    MedicalTime, ObjectHeader, OpaqueId, TimePrecision,
};
use serde_json::Value;

use super::store::{InMemoryAuthorityStore, ScopeError, StoredObject};

/// Inputs for append-only assertion amendment.
pub struct AmendAssertionInput<'a> {
    pub prior_assertion_id: &'a OpaqueId,
    pub authorized_by: OpaqueId,
    pub payload: Value,
    pub effective_time: Option<MedicalTime>,
    pub kind: AmendmentKind,
    pub rationale: String,
    pub realm_id: &'a medscale_contracts::objects::RealmId,
    pub scope_id: &'a medscale_contracts::objects::AuthorityScopeId,
}

pub fn amend_assertion(
    store: &mut InMemoryAuthorityStore,
    input: AmendAssertionInput<'_>,
) -> Result<(ClinicalAssertion, AmendmentRecord, ActionAuditRecord), AmendError> {
    if let Some(ref t) = input.effective_time {
        MedicalTime::parse_validated(
            t.value.clone(),
            t.precision,
            t.approximate,
            t.timezone_offset_minutes,
        )
        .map_err(|e| AmendError::InvalidTime(e.to_string()))?;
    }

    let prior = match store.get_scoped(input.prior_assertion_id, input.realm_id, input.scope_id) {
        Ok(StoredObject::Assertion(a)) => a.clone(),
        Ok(_) => return Err(AmendError::NotFound),
        Err(ScopeError::NotFound) => return Err(AmendError::NotFound),
        Err(ScopeError::WrongScope) => return Err(AmendError::WrongScope),
    };

    if store
        .superseded_assertion_ids()
        .contains(prior.header.id.as_str())
    {
        return Err(AmendError::AlreadySuperseded);
    }

    let assertion_id = store.alloc_id("assert");
    let amendment_id = store.alloc_id("amend");
    let audit_id = store.alloc_id("audit");
    let recorded = MedicalTime::new("1970-01-01T00:00:00Z", TimePrecision::Instant, false);

    let claim_kind = match input.kind {
        AmendmentKind::Retraction => format!("retracted:{}", prior.claim_kind),
        AmendmentKind::Supersession => prior.claim_kind.clone(),
    };

    let assertion = ClinicalAssertion {
        header: ObjectHeader {
            id: assertion_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: prior.header.realm_id.clone(),
            authority_scope_id: prior.header.authority_scope_id.clone(),
        },
        subject_ref: prior.subject_ref.clone(),
        claim_kind,
        payload: input.payload,
        promoted_from_proposal_id: prior.promoted_from_proposal_id.clone(),
        authorized_by: input.authorized_by.clone(),
        effective_time: input.effective_time.or(prior.effective_time.clone()),
        recorded_time: recorded.clone(),
    };

    let amendment = AmendmentRecord {
        header: ObjectHeader {
            id: amendment_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: prior.header.realm_id.clone(),
            authority_scope_id: prior.header.authority_scope_id.clone(),
        },
        prior_assertion_id: prior.header.id.clone(),
        superseding_assertion_id: assertion_id.clone(),
        kind: input.kind,
        rationale: input.rationale.clone(),
        authorized_by: input.authorized_by.clone(),
        recorded_time: recorded,
    };

    let audit = ActionAuditRecord {
        header: ObjectHeader {
            id: audit_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: prior.header.realm_id.clone(),
            authority_scope_id: prior.header.authority_scope_id.clone(),
        },
        kind: ActionAuditKind::Audit,
        actor: input.authorized_by,
        action: match input.kind {
            AmendmentKind::Supersession => "amend_assertion_supersession".to_owned(),
            AmendmentKind::Retraction => "amend_assertion_retraction".to_owned(),
        },
        target_refs: vec![prior.header.id, assertion_id.clone(), amendment_id.clone()],
        effect_state: None,
        payload_digest: None,
        detail: Some(serde_json::json!({ "rationale": input.rationale })),
    };

    // Append-only: never overwrite the prior assertion object.
    store.insert(StoredObject::Assertion(assertion.clone()));
    store.insert(StoredObject::Amendment(amendment.clone()));
    store.insert(StoredObject::Audit(audit.clone()));
    let _ = audit_id;
    Ok((assertion, amendment, audit))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmendError {
    NotFound,
    WrongScope,
    AlreadySuperseded,
    InvalidTime(String),
}

impl From<ScopeError> for AmendError {
    fn from(value: ScopeError) -> Self {
        match value {
            ScopeError::NotFound => Self::NotFound,
            ScopeError::WrongScope => Self::WrongScope,
        }
    }
}

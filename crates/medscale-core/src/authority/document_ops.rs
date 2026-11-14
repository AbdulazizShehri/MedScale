//! Spec 010 document MIME quarantine + OCR/ASR fixture stubs.

use medscale_contracts::AUTHORITY_SCHEMA_VERSION;
use medscale_contracts::documents::{
    AsrStubRequest, DocumentDenyReason, DocumentIntakeRequest, DocumentIntakeResult,
    MediaStubResult, OcrStubRequest, QuarantineDecision,
};
use medscale_contracts::envelopes::AuthorityError;
use medscale_contracts::objects::{
    ActionAuditKind, ActionAuditRecord, DerivedSourceArtifact, DigestSha256, LossClass,
    ObjectHeader, OpaqueId, ProducerKind, Proposal, RepresentationKind,
};
use medscale_contracts::worker_policy::WorkerSupervisionPolicy;

use super::source_ops::create_source_record;
use super::store::{InMemoryAuthorityStore, StoredObject};

/// Evaluate MIME policy (deny-by-default for hostile classes).
#[must_use]
pub fn mime_decision(req: &DocumentIntakeRequest) -> (QuarantineDecision, DocumentDenyReason) {
    if req.bytes.is_empty() {
        return (
            QuarantineDecision::Quarantine,
            DocumentDenyReason::EmptyPayload,
        );
    }
    if req.mime.is_denied_by_default() {
        return (
            QuarantineDecision::Quarantine,
            DocumentDenyReason::MimeDenied,
        );
    }
    (QuarantineDecision::Admit, DocumentDenyReason::Ok)
}

/// Document worker profile must remain ambient-deny.
#[must_use]
pub fn document_worker_policy() -> WorkerSupervisionPolicy {
    let mut p = WorkerSupervisionPolicy::deny_by_default();
    p.confinement_profile = "p1_document_worker_v0".to_owned();
    p
}

/// Voice worker profile must remain ambient-deny.
#[must_use]
pub fn voice_worker_policy() -> WorkerSupervisionPolicy {
    let mut p = WorkerSupervisionPolicy::deny_by_default();
    p.confinement_profile = "p1_voice_worker_v0".to_owned();
    p
}

pub(crate) fn intake(
    store: &mut InMemoryAuthorityStore,
    realm_id: medscale_contracts::objects::RealmId,
    scope_id: medscale_contracts::objects::AuthorityScopeId,
    request: DocumentIntakeRequest,
) -> Result<DocumentIntakeResult, AuthorityError> {
    let audit_id = store.alloc_id("audit");
    let (decision, reason) = mime_decision(&request);
    let digest = DigestSha256::of(&request.bytes);
    if decision == QuarantineDecision::Quarantine {
        store.insert(StoredObject::Audit(ActionAuditRecord {
            header: ObjectHeader {
                id: audit_id.clone(),
                schema_version: AUTHORITY_SCHEMA_VERSION,
                realm_id,
                authority_scope_id: scope_id,
            },
            kind: ActionAuditKind::Audit,
            actor: OpaqueId::new("document-intake"),
            action: "document.quarantine".to_owned(),
            target_refs: vec![],
            effect_state: None,
            payload_digest: Some(digest),
            detail: Some(serde_json::json!({
                "mime": request.mime.as_str(),
                "reason": reason,
                "fixture_id": request.fixture_id,
            })),
        }));
        return Ok(DocumentIntakeResult {
            decision,
            reason,
            source_id: None,
            content_digest: None,
            audit_id,
        });
    }

    let record = create_source_record(
        store,
        realm_id.clone(),
        scope_id.clone(),
        request.mime.as_str().to_owned(),
        request.bytes,
    );
    store.insert(StoredObject::Audit(ActionAuditRecord {
        header: ObjectHeader {
            id: audit_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id: scope_id,
        },
        kind: ActionAuditKind::Audit,
        actor: OpaqueId::new("document-intake"),
        action: "document.admit".to_owned(),
        target_refs: vec![record.header.id.clone()],
        effect_state: None,
        payload_digest: Some(record.content_digest.clone()),
        detail: Some(serde_json::json!({
            "mime": request.mime.as_str(),
            "fixture_id": request.fixture_id,
        })),
    }));
    Ok(DocumentIntakeResult {
        decision: QuarantineDecision::Admit,
        reason: DocumentDenyReason::Ok,
        source_id: Some(record.header.id),
        content_digest: Some(record.content_digest),
        audit_id,
    })
}

pub(crate) fn ocr_stub(
    store: &mut InMemoryAuthorityStore,
    realm_id: medscale_contracts::objects::RealmId,
    scope_id: medscale_contracts::objects::AuthorityScopeId,
    request: OcrStubRequest,
) -> Result<MediaStubResult, AuthorityError> {
    if !document_worker_policy().ambient_denied() {
        return Err(AuthorityError::InvalidArgument {
            message: "document worker ambient deny violated".into(),
        });
    }
    let text = format!("OCR_FIXTURE:{}:synthetic", request.fixture_id);
    let derived_id = store.alloc_id("derived");
    let proposal_id = store.alloc_id("proposal");
    store.insert(StoredObject::Derived(DerivedSourceArtifact {
        header: ObjectHeader {
            id: derived_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: realm_id.clone(),
            authority_scope_id: scope_id.clone(),
        },
        source_id: request.source_id.clone(),
        transform_id: "ocr.fixture_stub.v0".to_owned(),
        transform_version: "0.1.0".to_owned(),
        representation: RepresentationKind::NormalizedText,
        content_digest: DigestSha256::of(text.as_bytes()),
        bytes: text.as_bytes().to_vec(),
        loss_class: LossClass::Lossy,
        parent_span_map_ref: None,
    }));
    store.insert(StoredObject::Proposal(Proposal {
        header: ObjectHeader {
            id: proposal_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id: scope_id,
        },
        subject_ref: None,
        claim_kind: "ocr_text_span".to_owned(),
        payload: serde_json::json!({ "text": text, "evidence_only": true }),
        confidence: None,
        evidence_refs: vec![derived_id.clone(), request.source_id],
        producer: ProducerKind::WorkerStub,
    }));
    Ok(MediaStubResult {
        derived_id,
        proposal_id,
        evidence_only: true,
        text,
    })
}

pub(crate) fn asr_stub(
    store: &mut InMemoryAuthorityStore,
    realm_id: medscale_contracts::objects::RealmId,
    scope_id: medscale_contracts::objects::AuthorityScopeId,
    request: AsrStubRequest,
) -> Result<MediaStubResult, AuthorityError> {
    if !voice_worker_policy().ambient_denied() {
        return Err(AuthorityError::InvalidArgument {
            message: "voice worker ambient deny violated".into(),
        });
    }
    let text = format!("ASR_FIXTURE:{}:synthetic transcript", request.fixture_id);
    let derived_id = store.alloc_id("derived");
    let proposal_id = store.alloc_id("proposal");
    store.insert(StoredObject::Derived(DerivedSourceArtifact {
        header: ObjectHeader {
            id: derived_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id: realm_id.clone(),
            authority_scope_id: scope_id.clone(),
        },
        source_id: request.source_id.clone(),
        transform_id: "asr.fixture_stub.v0".to_owned(),
        transform_version: "0.1.0".to_owned(),
        representation: RepresentationKind::NormalizedText,
        content_digest: DigestSha256::of(text.as_bytes()),
        bytes: text.as_bytes().to_vec(),
        loss_class: LossClass::Lossy,
        parent_span_map_ref: None,
    }));
    store.insert(StoredObject::Proposal(Proposal {
        header: ObjectHeader {
            id: proposal_id.clone(),
            schema_version: AUTHORITY_SCHEMA_VERSION,
            realm_id,
            authority_scope_id: scope_id,
        },
        subject_ref: None,
        claim_kind: "asr_transcript_span".to_owned(),
        payload: serde_json::json!({
            "text": text,
            "evidence_only": true,
            "spans": [{"start_ms": 0, "end_ms": 1000, "text": text}]
        }),
        confidence: None,
        evidence_refs: vec![derived_id.clone(), request.source_id],
        producer: ProducerKind::WorkerStub,
    }));
    Ok(MediaStubResult {
        derived_id,
        proposal_id,
        evidence_only: true,
        text,
    })
}

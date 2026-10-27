//! Paper Experiment B0: authority promotion and external-effect conformance.
//!
//! Scientific base: `1e2b7d94e970256b38bda15fa91f62bc397e825a`.
//! These tests exercise production Core contracts under strict client-session
//! enforcement. They are not clinical validation or a universal security proof.

use medscale_contracts::actions::{CreateExternalActionIntentRequest, NphiesInvokeRequest};
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{
    AuthorityScopeId, ClinicalAssertion, DigestSha256, EffectState, OpaqueId, RealmId, VaultId,
};
use medscale_core::CoreFacade;
use serde_json::json;

const REALM: &str = "paper-realm";
const SCOPE: &str = "paper-effects-scope";
const VAULT: &str = "paper-effects-vault";

fn request(id: u64, capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("paper-b0-req-{id}")),
        VaultId::new(VAULT),
        RealmId::new(REALM),
        AuthorityScopeId::new(SCOPE),
        capability,
        body,
    )
}

struct Harness {
    facade: CoreFacade,
    session: OpaqueId,
    next_request: u64,
}

impl Harness {
    fn with_grants(grants: Vec<Capability>) -> Self {
        let facade = CoreFacade::new();
        let lease = facade.dispatch(request(
            1,
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("paper-b0-client"),
                holder_id_hint: Some(OpaqueId::new("paper-b0-holder")),
            },
        ));
        let holder_id = match lease.result.expect("lease must open") {
            ResponseBody::Lease { holder_id, .. } => holder_id,
            other => panic!("unexpected lease response: {other:?}"),
        };
        let opened = facade.dispatch(request(
            2,
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id,
                granted: grants,
                ttl_ticks: 1_000_000,
            },
        ));
        let session = match opened.result.expect("session must open") {
            ResponseBody::Session { session_id, .. } => session_id,
            other => panic!("unexpected session response: {other:?}"),
        };
        Self {
            facade,
            session,
            next_request: 2,
        }
    }

    fn call(
        &mut self,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next_request += 1;
        let mut req = request(self.next_request, capability, body);
        req.session_id = Some(self.session.clone());
        self.facade.dispatch(req).result
    }

    fn create_intent(&mut self, payload: &[u8]) -> (OpaqueId, DigestSha256) {
        let digest = DigestSha256::of(payload);
        let action_id = match self
            .call(
                Capability::CreateExternalActionIntent,
                RequestBody::CreateExternalActionIntent {
                    request: CreateExternalActionIntentRequest {
                        actor: OpaqueId::new("paper-operator"),
                        action: "synthetic_submit".to_owned(),
                        target_refs: vec![],
                        payload_digest: digest.clone(),
                    },
                },
            )
            .expect("intent create")
        {
            ResponseBody::Created { object_id } => object_id,
            other => panic!("unexpected intent response: {other:?}"),
        };
        (action_id, digest)
    }
}

#[test]
fn b0_proposal_creation_does_not_grant_promotion_authority() {
    let mut h = Harness::with_grants(vec![Capability::CreateProposal, Capability::ListOutbox]);
    let proposal_id = match h
        .call(
            Capability::CreateProposal,
            RequestBody::CreateProposal {
                subject_ref: Some(OpaqueId::new("paper-subject")),
                claim_kind: "synthetic_claim".to_owned(),
                payload: json!({"value": "candidate"}),
                evidence_refs: vec![],
            },
        )
        .expect("proposal create")
    {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("unexpected proposal response: {other:?}"),
    };

    let promoted = h.call(
        Capability::PromoteProposal,
        RequestBody::PromoteProposal {
            proposal_id,
            authorized_by: OpaqueId::new("paper-reviewer"),
            subject_ref: OpaqueId::new("paper-subject"),
        },
    );
    assert_eq!(promoted, Err(AuthorityError::SessionDenied));

    let outbox = h
        .call(Capability::ListOutbox, RequestBody::ListOutbox)
        .expect("list outbox");
    let ResponseBody::Outbox { entries } = outbox else {
        panic!("expected outbox response");
    };
    assert!(
        entries.is_empty(),
        "proposal creation must not create an external-action intent"
    );
}

#[test]
fn b0_separately_authorized_promotion_preserves_proposal_and_reviewer_identity() {
    let mut h = Harness::with_grants(vec![
        Capability::CreateProposal,
        Capability::PromoteProposal,
        Capability::ReadObject,
    ]);
    let proposal_id = match h
        .call(
            Capability::CreateProposal,
            RequestBody::CreateProposal {
                subject_ref: Some(OpaqueId::new("paper-subject")),
                claim_kind: "synthetic_claim".to_owned(),
                payload: json!({"value": "candidate"}),
                evidence_refs: vec![],
            },
        )
        .expect("proposal create")
    {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("unexpected proposal response: {other:?}"),
    };
    let reviewer = OpaqueId::new("paper-reviewer");
    let assertion_id = match h
        .call(
            Capability::PromoteProposal,
            RequestBody::PromoteProposal {
                proposal_id: proposal_id.clone(),
                authorized_by: reviewer.clone(),
                subject_ref: OpaqueId::new("paper-subject"),
            },
        )
        .expect("promotion")
    {
        ResponseBody::Promoted { assertion_id, .. } => assertion_id,
        other => panic!("unexpected promotion response: {other:?}"),
    };
    let value = match h
        .call(
            Capability::ReadObject,
            RequestBody::ReadObject {
                object_id: assertion_id,
            },
        )
        .expect("assertion read")
    {
        ResponseBody::Object { value } => value,
        other => panic!("unexpected assertion read: {other:?}"),
    };
    let assertion: ClinicalAssertion = serde_json::from_value(value).expect("assertion decode");
    assert_eq!(assertion.promoted_from_proposal_id, Some(proposal_id));
    assert_eq!(assertion.authorized_by, reviewer);
}

#[test]
fn b0_external_action_intent_starts_pending_and_binds_exact_payload_digest() {
    let mut h = Harness::with_grants(vec![
        Capability::CreateExternalActionIntent,
        Capability::ListOutbox,
    ]);
    let (action_id, digest) = h.create_intent(b"paper-approved-payload");

    let outbox = h
        .call(Capability::ListOutbox, RequestBody::ListOutbox)
        .expect("list outbox");
    let ResponseBody::Outbox { entries } = outbox else {
        panic!("expected outbox response");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action_id, action_id);
    assert_eq!(entries[0].effect_state, EffectState::Pending);
    assert_eq!(entries[0].payload_digest, digest);
}

#[test]
fn b0_sent_transition_without_payload_binding_is_refused() {
    let mut h = Harness::with_grants(vec![Capability::AppendAudit, Capability::TransitionEffect]);
    let action_id = match h
        .call(
            Capability::AppendAudit,
            RequestBody::AppendAudit {
                actor: OpaqueId::new("paper-operator"),
                action: "synthetic_send".to_owned(),
                target_refs: vec![],
                detail: None,
            },
        )
        .expect("audit create")
    {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("unexpected audit response: {other:?}"),
    };

    let out = h.call(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id,
            to: EffectState::Sent,
            reconcile_token: None,
        },
    );
    assert!(matches!(out, Err(AuthorityError::InvalidArgument { .. })));
}

#[test]
fn b0_unknown_effect_cannot_be_blindly_retried() {
    let mut h = Harness::with_grants(vec![
        Capability::CreateExternalActionIntent,
        Capability::TransitionEffect,
    ]);
    let (action_id, _) = h.create_intent(b"paper-approved-payload");

    h.call(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id: action_id.clone(),
            to: EffectState::Sent,
            reconcile_token: None,
        },
    )
    .expect("pending -> sent");
    h.call(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id: action_id.clone(),
            to: EffectState::Unknown,
            reconcile_token: None,
        },
    )
    .expect("sent -> unknown");

    let blind_retry = h.call(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id: action_id.clone(),
            to: EffectState::Pending,
            reconcile_token: None,
        },
    );
    assert_eq!(blind_retry, Err(AuthorityError::UnknownRequiresReconcile));

    h.call(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id,
            to: EffectState::Pending,
            reconcile_token: Some("paper-reconciled".to_owned()),
        },
    )
    .expect("explicit reconciliation permits retry");
}

#[test]
fn b0_external_gate_refuses_nphies_even_when_session_has_capability() {
    let mut h = Harness::with_grants(vec![Capability::NphiesInvoke]);
    let out = h.call(
        Capability::NphiesInvoke,
        RequestBody::NphiesInvoke {
            request: NphiesInvokeRequest {
                workflow_id: "eligibility_v0".to_owned(),
                payload_digest: DigestSha256::of(b"synthetic"),
            },
        },
    );

    assert_eq!(
        out,
        Err(AuthorityError::ExternalGateRequired {
            gate: "SPEC_014_WORKFLOW_EVIDENCE".to_owned(),
        })
    );
}

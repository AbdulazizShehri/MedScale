//! Spec 014 controlled actions READY_BASE.

use medscale_contracts::actions::{
    ControlledActionsDoctorStatus, CreateExternalActionIntentRequest, NphiesInvokeRequest,
};
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, EffectState, OpaqueId, RealmId, VaultId,
};
use medscale_core::{CoreFacade, build_doctor_report};

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("v"),
        RealmId::new("r"),
        AuthorityScopeId::new("s"),
        capability,
        body,
    )
}

#[test]
fn create_intent_binds_payload_and_lists_outbox() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let digest = DigestSha256::of(b"approved-payload");
    let created = facade.dispatch(req(
        Capability::CreateExternalActionIntent,
        RequestBody::CreateExternalActionIntent {
            request: CreateExternalActionIntentRequest {
                actor: OpaqueId::new("actor"),
                action: "claim_submit".to_owned(),
                target_refs: vec![],
                payload_digest: digest.clone(),
            },
        },
    ));
    let action_id = match created.result.unwrap() {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };

    let listed = facade.dispatch(req(Capability::ListOutbox, RequestBody::ListOutbox));
    let ResponseBody::Outbox { entries } = listed.result.unwrap() else {
        panic!("expected outbox");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action_id, action_id);
    assert_eq!(entries[0].effect_state, EffectState::Pending);
    assert_eq!(entries[0].payload_digest, digest);
}

#[test]
fn sent_without_payload_digest_denied() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let created = facade.dispatch(req(
        Capability::AppendAudit,
        RequestBody::AppendAudit {
            actor: OpaqueId::new("actor"),
            action: "send".to_owned(),
            target_refs: vec![],
            detail: None,
        },
    ));
    let action_id = match created.result.unwrap() {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };
    let denied = facade.dispatch(req(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id,
            to: EffectState::Sent,
            reconcile_token: None,
        },
    ));
    assert!(matches!(
        denied.result,
        Err(AuthorityError::InvalidArgument { .. })
    ));
}

#[test]
fn unknown_requires_reconcile_after_bound_send() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let created = facade.dispatch(req(
        Capability::CreateExternalActionIntent,
        RequestBody::CreateExternalActionIntent {
            request: CreateExternalActionIntentRequest {
                actor: OpaqueId::new("actor"),
                action: "send".to_owned(),
                target_refs: vec![],
                payload_digest: DigestSha256::of(b"p"),
            },
        },
    ));
    let action_id = match created.result.unwrap() {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };
    assert!(
        facade
            .dispatch(req(
                Capability::TransitionEffect,
                RequestBody::TransitionEffect {
                    action_id: action_id.clone(),
                    to: EffectState::Sent,
                    reconcile_token: None,
                },
            ))
            .result
            .is_ok()
    );
    assert!(
        facade
            .dispatch(req(
                Capability::TransitionEffect,
                RequestBody::TransitionEffect {
                    action_id: action_id.clone(),
                    to: EffectState::Unknown,
                    reconcile_token: None,
                },
            ))
            .result
            .is_ok()
    );
    let denied = facade.dispatch(req(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id: action_id.clone(),
            to: EffectState::Pending,
            reconcile_token: None,
        },
    ));
    assert_eq!(denied.result, Err(AuthorityError::UnknownRequiresReconcile));
    assert!(
        facade
            .dispatch(req(
                Capability::TransitionEffect,
                RequestBody::TransitionEffect {
                    action_id,
                    to: EffectState::Pending,
                    reconcile_token: Some("reconciled".to_owned()),
                },
            ))
            .result
            .is_ok()
    );
}

#[test]
fn nphies_invoke_requires_workflow_evidence_gate() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let out = facade.dispatch(req(
        Capability::NphiesInvoke,
        RequestBody::NphiesInvoke {
            request: NphiesInvokeRequest {
                workflow_id: "eligibility_v0".to_owned(),
                payload_digest: DigestSha256::of(b"x"),
            },
        },
    ));
    assert_eq!(
        out.result,
        Err(AuthorityError::ExternalGateRequired {
            gate: "SPEC_014_WORKFLOW_EVIDENCE".to_owned()
        })
    );
}

#[test]
fn doctor_controlled_actions_axis() {
    let m = ControlledActionsDoctorStatus::ready_base();
    assert!(m.present);
    assert!(m.outbox_restart_qualified);
    assert!(!m.nphies_authorized);
    assert!(!m.unknown_blind_retry);
    assert!(m.is_honest_ready_base());
    let report = build_doctor_report(None, false, false);
    assert!(!report.controlled_actions.nphies_authorized);
    assert!(report.controlled_actions.outbox_restart_qualified);
}

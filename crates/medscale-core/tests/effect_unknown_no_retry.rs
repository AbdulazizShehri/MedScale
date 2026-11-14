use medscale_contracts::actions::CreateExternalActionIntentRequest;
use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DigestSha256, EffectState, OpaqueId, RealmId, VaultId,
};
use medscale_core::CoreFacade;

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
fn unknown_without_reconcile_denied() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let created = facade.dispatch(req(
        Capability::CreateExternalActionIntent,
        RequestBody::CreateExternalActionIntent {
            request: CreateExternalActionIntentRequest {
                actor: OpaqueId::new("actor"),
                action: "send".to_owned(),
                target_refs: vec![],
                payload_digest: DigestSha256::of(b"bound"),
            },
        },
    ));
    let action_id = match created.result.unwrap() {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };

    // Pending -> Sent
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

    // Sent -> Unknown
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

    // Unknown -> Pending without reconcile denied
    let denied = facade.dispatch(req(
        Capability::TransitionEffect,
        RequestBody::TransitionEffect {
            action_id: action_id.clone(),
            to: EffectState::Pending,
            reconcile_token: None,
        },
    ));
    assert_eq!(denied.result, Err(AuthorityError::UnknownRequiresReconcile));

    // With reconcile token allowed
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

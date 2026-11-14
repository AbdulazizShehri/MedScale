use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use serde_json::json;

fn base(
    vault: &str,
    realm: &str,
    scope: &str,
    capability: Capability,
    body: RequestBody,
) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new(vault),
        RealmId::new(realm),
        AuthorityScopeId::new(scope),
        capability,
        body,
    )
}

#[test]
fn unauthorized_capability_mismatch_denied() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let resp = facade.dispatch(base(
        "v",
        "r",
        "s",
        Capability::Ping,
        RequestBody::CreateProposal {
            subject_ref: None,
            claim_kind: "x".to_owned(),
            payload: json!({}),
            evidence_refs: vec![],
        },
    ));
    assert_eq!(resp.result, Err(AuthorityError::Unauthorized));
}

#[test]
fn authorized_promote_and_cross_scope_deny() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let created = facade.dispatch(base(
        "v",
        "r",
        "s",
        Capability::CreateProposal,
        RequestBody::CreateProposal {
            subject_ref: Some(OpaqueId::new("subj")),
            claim_kind: "allergy".to_owned(),
            payload: json!({"code": "x"}),
            evidence_refs: vec![],
        },
    ));
    let proposal_id = match created.result.unwrap() {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("{other:?}"),
    };

    let promoted = facade.dispatch(base(
        "v",
        "r",
        "s",
        Capability::PromoteProposal,
        RequestBody::PromoteProposal {
            proposal_id: proposal_id.clone(),
            authorized_by: OpaqueId::new("actor"),
            subject_ref: OpaqueId::new("subj"),
        },
    ));
    assert!(matches!(promoted.result, Ok(ResponseBody::Promoted { .. })));

    let cross = facade.dispatch(base(
        "v",
        "r",
        "other-scope",
        Capability::PromoteProposal,
        RequestBody::PromoteProposal {
            proposal_id,
            authorized_by: OpaqueId::new("actor"),
            subject_ref: OpaqueId::new("subj"),
        },
    ));
    assert_eq!(cross.result, Err(AuthorityError::WrongScope));
}

#[test]
fn no_silent_identity_merge() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let bad = facade.dispatch(base(
        "v",
        "r",
        "s",
        Capability::DecideIdentityMerge,
        RequestBody::DecideIdentityMerge {
            surviving_subject_id: OpaqueId::new("a"),
            merged_subject_ids: vec![],
            authorized_by: OpaqueId::new("actor"),
            rationale: "x".to_owned(),
        },
    ));
    assert!(matches!(
        bad.result,
        Err(AuthorityError::InvalidArgument { .. })
    ));

    let ok = facade.dispatch(base(
        "v",
        "r",
        "s",
        Capability::DecideIdentityMerge,
        RequestBody::DecideIdentityMerge {
            surviving_subject_id: OpaqueId::new("a"),
            merged_subject_ids: vec![OpaqueId::new("b")],
            authorized_by: OpaqueId::new("actor"),
            rationale: "duplicate chart".to_owned(),
        },
    ));
    assert!(matches!(ok.result, Ok(ResponseBody::Created { .. })));
}

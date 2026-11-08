use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req-1"),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new("scope-a"),
        capability,
        body,
    )
}

#[test]
fn exclusive_lease_second_acquire_already_held() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    let first = facade.dispatch(req(
        Capability::AcquireLease,
        RequestBody::AcquireLease {
            client_id: OpaqueId::new("client-a"),
            holder_id_hint: None,
        },
    ));
    match first.result.unwrap() {
        ResponseBody::Lease { holder_id, .. } => {
            assert_eq!(holder_id, OpaqueId::new("client-a"));
        }
        other => panic!("unexpected {other:?}"),
    }

    let second = facade.dispatch(req(
        Capability::AcquireLease,
        RequestBody::AcquireLease {
            client_id: OpaqueId::new("client-b"),
            holder_id_hint: None,
        },
    ));
    match second.result {
        Err(AuthorityError::AlreadyHeld { holder_id }) => {
            assert_ne!(
                holder_id,
                OpaqueId::new("client-a"),
                "the holder is never revealed"
            );
        }
        other => panic!("unexpected {other:?}"),
    }

    let release = facade.dispatch(req(
        Capability::ReleaseLease,
        RequestBody::ReleaseLease {
            holder_id: OpaqueId::new("client-a"),
        },
    ));
    assert!(matches!(release.result, Ok(ResponseBody::Released)));

    // Transient owner path: acquire again after release.
    let third = facade.dispatch(req(
        Capability::AcquireLease,
        RequestBody::AcquireLease {
            client_id: OpaqueId::new("client-c"),
            holder_id_hint: Some(OpaqueId::new("transient-host")),
        },
    ));
    match third.result.unwrap() {
        ResponseBody::Lease { holder_id, .. } => {
            assert_eq!(holder_id, OpaqueId::new("transient-host"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

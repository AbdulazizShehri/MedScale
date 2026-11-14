//! Spec 018 host/client session authority (retained under Spec 024 Strict).

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::{CoreFacade, build_doctor_report};

fn req(capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new("req"),
        VaultId::new("vault-1"),
        RealmId::new("realm-a"),
        AuthorityScopeId::new("scope-a"),
        capability,
        body,
    )
}

fn acquire_lease(facade: &CoreFacade) -> OpaqueId {
    let acquired = facade.dispatch(req(
        Capability::AcquireLease,
        RequestBody::AcquireLease {
            client_id: OpaqueId::new("client-a"),
            holder_id_hint: None,
        },
    ));
    match acquired.result.expect("lease") {
        ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn open_session_requires_lease_holder_match() {
    let facade = CoreFacade::new();
    let denied = facade.dispatch(req(
        Capability::OpenSession,
        RequestBody::OpenSession {
            holder_id: OpaqueId::new("nobody"),
            granted: vec![Capability::Ping],
            ttl_ticks: 10,
        },
    ));
    assert!(matches!(denied.result, Err(AuthorityError::LeaseRequired)));

    let holder = acquire_lease(&facade);
    let wrong = facade.dispatch(req(
        Capability::OpenSession,
        RequestBody::OpenSession {
            holder_id: OpaqueId::new("other"),
            granted: vec![Capability::Ping],
            ttl_ticks: 10,
        },
    ));
    assert!(matches!(wrong.result, Err(AuthorityError::NotHolder)));

    let opened = facade.dispatch(req(
        Capability::OpenSession,
        RequestBody::OpenSession {
            holder_id: holder,
            granted: vec![Capability::Ping, Capability::CreateSourceRecord],
            ttl_ticks: 5,
        },
    ));
    match opened.result.expect("session") {
        ResponseBody::Session {
            session_id,
            expires_at_tick,
        } => {
            let random = session_id
                .as_str()
                .strip_prefix("session-")
                .expect("prefix");
            assert_eq!(random.len(), 32, "128 random bits, not a counter");
            assert_eq!(expires_at_tick, 5);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn revoke_denies_further_use() {
    let facade = CoreFacade::new();
    let holder = acquire_lease(&facade);
    let opened = facade.dispatch(req(
        Capability::OpenSession,
        RequestBody::OpenSession {
            holder_id: holder,
            granted: vec![Capability::Ping],
            ttl_ticks: 20,
        },
    ));
    let session_id = match opened.result.expect("session") {
        ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };

    let mut ok = req(Capability::Ping, RequestBody::Ping);
    ok.session_id = Some(session_id.clone());
    assert!(matches!(
        facade.dispatch(ok).result,
        Ok(ResponseBody::Pong { .. })
    ));

    let revoked = facade.dispatch(req(
        Capability::RevokeSession,
        RequestBody::RevokeSession {
            session_id: session_id.clone(),
        },
    ));
    assert!(matches!(revoked.result, Ok(ResponseBody::Released)));

    let mut denied = req(Capability::Ping, RequestBody::Ping);
    denied.session_id = Some(session_id);
    assert!(matches!(
        facade.dispatch(denied).result,
        Err(AuthorityError::SessionRevoked)
    ));
}

#[test]
fn expiry_denies_further_use() {
    let facade = CoreFacade::new();
    let holder = acquire_lease(&facade);
    let opened = facade.dispatch(req(
        Capability::OpenSession,
        RequestBody::OpenSession {
            holder_id: holder,
            granted: vec![Capability::Ping],
            ttl_ticks: 3,
        },
    ));
    let session_id = match opened.result.expect("session") {
        ResponseBody::Session {
            session_id,
            expires_at_tick,
        } => {
            assert_eq!(expires_at_tick, 3);
            session_id
        }
        other => panic!("{other:?}"),
    };

    facade.sessions().advance_ticks(3);
    let mut denied = req(Capability::Ping, RequestBody::Ping);
    denied.session_id = Some(session_id);
    assert!(matches!(
        facade.dispatch(denied).result,
        Err(AuthorityError::SessionExpired)
    ));
}

#[test]
fn mutating_without_session_id_denied_under_strict() {
    let facade = CoreFacade::new();
    let created = facade.dispatch(req(
        Capability::CreateSourceRecord,
        RequestBody::CreateSourceRecord {
            media_type: "text/plain".to_owned(),
            bytes: b"synthetic".to_vec(),
        },
    ));
    assert!(matches!(
        created.result,
        Err(AuthorityError::SessionRequired)
    ));
}

#[test]
fn doctor_host_authority_ready_base_os_ipc_not_multi_client() {
    let report = build_doctor_report(None, false, false);
    assert!(report.host_authority.present);
    assert!(report.host_authority.ready_base);
    assert!(!report.host_authority.multi_client_release_ready);
    assert!(report.host_authority.os_ipc_qualified);
}

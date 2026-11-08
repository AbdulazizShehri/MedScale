//! Spec 024 host OS IPC authority READY_BASE.

use std::time::{SystemTime, UNIX_EPOCH};

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::{
    CoreFacade, HostIpcClient, HostIpcServer, SessionEnforcement, build_doctor_report,
    endpoint_for_vault_root,
};
use medscale_storage::WriterLockError;

fn unique_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("medscale-024-{label}-{nanos}"))
}

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

#[test]
fn strict_default_denies_mutating_without_session() {
    let facade = CoreFacade::new();
    assert_eq!(facade.session_enforcement(), SessionEnforcement::Strict);
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
fn doctor_os_ipc_qualified_not_multi_client() {
    let report = build_doctor_report(None, false, false);
    assert!(report.host_authority.present);
    assert!(report.host_authority.ready_base);
    assert!(report.host_authority.os_ipc_qualified);
    assert!(!report.host_authority.multi_client_release_ready);
}

#[test]
fn ipc_session_open_mutate_revoke_across_two_clients() {
    let root = unique_root("ipc");
    std::fs::create_dir_all(&root).expect("mkdir");
    let endpoint = endpoint_for_vault_root(&root);
    let server = HostIpcServer::bind(&root, &endpoint).expect("bind");
    let (_endpoint, _facade, _join) = server.into_background();

    let mut client_a = HostIpcClient::connect_authenticated(&endpoint, &root).expect("client a");
    let mut client_b = HostIpcClient::connect_authenticated(&endpoint, &root).expect("client b");

    let lease = client_a
        .dispatch(req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("client-a"),
                holder_id_hint: None,
            },
        ))
        .expect("ipc")
        .result
        .expect("lease");
    let holder_id = match lease {
        ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };

    let opened = client_a
        .dispatch(req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id,
                granted: vec![Capability::CreateSourceRecord, Capability::Ping],
                ttl_ticks: 100,
            },
        ))
        .expect("ipc")
        .result
        .expect("session");
    let session_id = match opened {
        ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };

    let mut mutate = req(
        Capability::CreateSourceRecord,
        RequestBody::CreateSourceRecord {
            media_type: "text/plain".to_owned(),
            bytes: b"via-ipc".to_vec(),
        },
    );
    mutate.session_id = Some(session_id.clone());
    let created = client_b.dispatch(mutate).expect("ipc").result;
    assert!(matches!(created, Ok(ResponseBody::Created { .. })));

    let revoked = client_a
        .dispatch(req(
            Capability::RevokeSession,
            RequestBody::RevokeSession {
                session_id: session_id.clone(),
            },
        ))
        .expect("ipc")
        .result;
    assert!(matches!(revoked, Ok(ResponseBody::Released)));

    let mut denied = req(
        Capability::CreateSourceRecord,
        RequestBody::CreateSourceRecord {
            media_type: "text/plain".to_owned(),
            bytes: b"after-revoke".to_vec(),
        },
    );
    denied.session_id = Some(session_id);
    let after = client_b.dispatch(denied).expect("ipc").result;
    assert!(matches!(after, Err(AuthorityError::SessionRevoked)));
}

#[test]
fn ipc_capability_deny_across_clients() {
    let root = unique_root("cap");
    std::fs::create_dir_all(&root).expect("mkdir");
    let endpoint = format!("{}-cap", endpoint_for_vault_root(&root));
    let server = HostIpcServer::bind(&root, &endpoint).expect("bind");
    let (_endpoint, _facade, _join) = server.into_background();

    // Reaching the socket is not enough: without the token nothing answers.
    let mut stranger = HostIpcClient::connect(&endpoint).expect("connect");
    assert!(
        stranger
            .dispatch(req(Capability::Ping, RequestBody::Ping))
            .is_err(),
        "a client without the token gets no reply"
    );
    let mut client = HostIpcClient::connect_authenticated(&endpoint, &root).expect("connect");
    let lease = client
        .dispatch(req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("c"),
                holder_id_hint: None,
            },
        ))
        .expect("ipc")
        .result
        .expect("lease");
    let holder_id = match lease {
        ResponseBody::Lease { holder_id, .. } => holder_id,
        other => panic!("{other:?}"),
    };
    let opened = client
        .dispatch(req(
            Capability::OpenSession,
            RequestBody::OpenSession {
                holder_id,
                granted: vec![Capability::Ping],
                ttl_ticks: 50,
            },
        ))
        .expect("ipc")
        .result
        .expect("session");
    let session_id = match opened {
        ResponseBody::Session { session_id, .. } => session_id,
        other => panic!("{other:?}"),
    };

    let mut denied = req(
        Capability::CreateSourceRecord,
        RequestBody::CreateSourceRecord {
            media_type: "text/plain".to_owned(),
            bytes: b"nope".to_vec(),
        },
    );
    denied.session_id = Some(session_id);
    let result = client.dispatch(denied).expect("ipc").result;
    assert!(matches!(result, Err(AuthorityError::SessionDenied)));
}

#[test]
fn second_ipc_host_cannot_take_writer_lock() {
    let root = unique_root("lock");
    std::fs::create_dir_all(&root).expect("mkdir");
    let endpoint_a = format!("{}-a", endpoint_for_vault_root(&root));
    let endpoint_b = format!("{}-b", endpoint_for_vault_root(&root));
    let _server_a = HostIpcServer::bind(&root, &endpoint_a).expect("first host");
    let err = match HostIpcServer::bind(&root, &endpoint_b) {
        Ok(_) => panic!("second must fail"),
        Err(e) => e,
    };
    assert!(matches!(
        err,
        medscale_core::HostIpcError::Writer(WriterLockError::WriterHeld)
    ));
}

#[test]
fn legacy_escape_still_allows_lease_only_mutation() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    assert_eq!(
        facade.session_enforcement(),
        SessionEnforcement::LegacyLeaseOnlyEngineering
    );
    let created = facade.dispatch(req(
        Capability::CreateSourceRecord,
        RequestBody::CreateSourceRecord {
            media_type: "text/plain".to_owned(),
            bytes: b"legacy".to_vec(),
        },
    ));
    assert!(matches!(created.result, Ok(ResponseBody::Created { .. })));
}

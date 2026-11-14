//! Spec 084 client sync: joins a Hub and synchronizes one link.
//!
//! Every step is an ordinary `AuthorityRequest`: the client's own requests
//! go to its own `CoreFacade`, and Hub requests go through a `HubTransport`
//! carrying the same serialized envelopes in-process or over the Spec 024
//! local-socket IPC. The device secret never leaves the client Core: this
//! module only moves public keys, signatures, envelopes and events.

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, AuthorityResponse, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::hub::{HUB_BATCH_MAX, HubInvitationCode, HubLink, SyncOutcome, SyncReport};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};

use crate::CoreFacade;
use crate::ipc::HostIpcClient;

/// Carries one request to a Hub and its response back.
pub trait HubTransport {
    fn call(&mut self, req: AuthorityRequest) -> Result<AuthorityResponse, String>;
}

/// A Hub in the same process. Requests and responses round-trip through
/// their JSON form so this path proves the same bytes an IPC peer sends.
pub struct InProcessHubTransport<'a> {
    pub hub: &'a CoreFacade,
}

impl HubTransport for InProcessHubTransport<'_> {
    fn call(&mut self, req: AuthorityRequest) -> Result<AuthorityResponse, String> {
        let bytes = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        let req: AuthorityRequest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let resp = self.hub.dispatch(req);
        let bytes = serde_json::to_vec(&resp).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}

/// A Hub served by a `HostIpcServer` on this machine.
pub struct IpcHubTransport {
    client: HostIpcClient,
}

impl IpcHubTransport {
    pub fn connect(endpoint: &str) -> Result<Self, AuthorityError> {
        let client = HostIpcClient::connect(endpoint).map_err(|e| AuthorityError::Unavailable {
            message: e.to_string(),
        })?;
        Ok(Self { client })
    }

    /// Wraps a client that is already connected, for example one that
    /// presented a full endpoint's token.
    #[must_use]
    pub fn from_client(client: HostIpcClient) -> Self {
        Self { client }
    }
}

impl HubTransport for IpcHubTransport {
    fn call(&mut self, req: AuthorityRequest) -> Result<AuthorityResponse, String> {
        self.client.dispatch(req).map_err(|e| e.to_string())
    }
}

/// The client vault's own request context.
pub struct ClientContext<'a> {
    pub facade: &'a CoreFacade,
    pub vault_id: VaultId,
    pub realm_id: RealmId,
    pub scope_id: AuthorityScopeId,
    pub session_id: OpaqueId,
}

/// The Hub a request is addressed to.
struct HubTarget {
    vault_id: VaultId,
    realm_id: RealmId,
    scope_id: AuthorityScopeId,
}

struct Calls {
    next: u64,
}

impl Calls {
    fn id(&mut self, side: &str) -> OpaqueId {
        self.next += 1;
        OpaqueId::new(format!("hub-sync-{side}-{}", self.next))
    }

    fn client(
        &mut self,
        ctx: &ClientContext<'_>,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        let mut req = AuthorityRequest::new(
            self.id("client"),
            ctx.vault_id.clone(),
            ctx.realm_id.clone(),
            ctx.scope_id.clone(),
            capability,
            body,
        );
        req.session_id = Some(ctx.session_id.clone());
        ctx.facade.dispatch(req).result
    }

    fn hub(
        &mut self,
        transport: &mut dyn HubTransport,
        target: &HubTarget,
        session_id: Option<OpaqueId>,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        let mut req = AuthorityRequest::new(
            self.id("hub"),
            target.vault_id.clone(),
            target.realm_id.clone(),
            target.scope_id.clone(),
            capability,
            body,
        );
        req.session_id = session_id;
        transport
            .call(req)
            .map_err(|message| AuthorityError::Unavailable { message })?
            .result
    }
}

fn unexpected(what: &str) -> AuthorityError {
    AuthorityError::Internal {
        message: format!("unexpected response to {what}"),
    }
}

/// Enrolls this vault's new device with a Hub by invitation and records the
/// link. `endpoint` is the Hub's local IPC endpoint, kept for later syncs.
pub fn join(
    ctx: &ClientContext<'_>,
    transport: &mut dyn HubTransport,
    endpoint: String,
    code: HubInvitationCode,
) -> Result<HubLink, AuthorityError> {
    let mut calls = Calls { next: 0 };
    let ResponseBody::HubJoinPrepared {
        link_id,
        public_key_hex,
        signature_hex,
    } = calls.client(
        ctx,
        Capability::HubClient,
        RequestBody::HubJoinPrepare { code: code.clone() },
    )?
    else {
        return Err(unexpected("join prepare"));
    };
    let target = HubTarget {
        vault_id: code.hub_vault_id.clone(),
        realm_id: code.hub_realm_id.clone(),
        scope_id: code.hub_scope_id.clone(),
    };
    let ResponseBody::HubDevice { device } = calls.hub(
        transport,
        &target,
        None,
        Capability::HubBootstrap,
        RequestBody::HubEnroll {
            token_hex: code.token_hex.clone(),
            public_key_hex,
            signature_hex,
        },
    )?
    else {
        return Err(unexpected("enroll"));
    };
    let ResponseBody::HubLink { link } = calls.client(
        ctx,
        Capability::HubClient,
        RequestBody::HubJoinComplete {
            link_id,
            endpoint,
            code: Box::new(code),
            device,
        },
    )?
    else {
        return Err(unexpected("join complete"));
    };
    Ok(*link)
}

/// Synchronizes one link: handshake, submit every pending envelope in
/// sequence order, record each outcome, then pull and mirror every event
/// after the link's cursor.
pub fn sync(
    ctx: &ClientContext<'_>,
    transport: &mut dyn HubTransport,
    link_id: &OpaqueId,
) -> Result<SyncReport, AuthorityError> {
    let mut calls = Calls { next: 0 };
    let ResponseBody::HubLink { link } = calls.client(
        ctx,
        Capability::HubRead,
        RequestBody::HubLinkGet {
            link_id: link_id.clone(),
        },
    )?
    else {
        return Err(unexpected("link get"));
    };
    if link.revoked {
        return Err(AuthorityError::Unauthorized);
    }
    let target = HubTarget {
        vault_id: link.hub_vault_id.clone(),
        realm_id: link.hub_realm_id.clone(),
        scope_id: link.hub_scope_id.clone(),
    };
    let ResponseBody::HubChallenge { challenge } = calls.hub(
        transport,
        &target,
        None,
        Capability::HubBootstrap,
        RequestBody::HubChallenge {
            device_id: link.device_id.clone(),
        },
    )?
    else {
        return Err(unexpected("challenge"));
    };
    let ResponseBody::HubHandshakeSigned { handshake } = calls.client(
        ctx,
        Capability::HubClient,
        RequestBody::HubSignHandshake {
            link_id: link_id.clone(),
            challenge: *challenge,
        },
    )?
    else {
        return Err(unexpected("sign handshake"));
    };
    let ResponseBody::HubSession { session } = calls.hub(
        transport,
        &target,
        None,
        Capability::HubBootstrap,
        RequestBody::HubHandshake {
            handshake: *handshake,
        },
    )?
    else {
        return Err(unexpected("handshake"));
    };
    let session_id = Some(session.session_id.clone());

    let ResponseBody::HubOutbox { entries } = calls.client(
        ctx,
        Capability::HubRead,
        RequestBody::HubOutboxList {
            link_id: link_id.clone(),
            pending_only: true,
        },
    )?
    else {
        return Err(unexpected("outbox list"));
    };
    let mut report = SyncReport {
        link_id: link_id.clone(),
        submitted: 0,
        applied: 0,
        conflicts: 0,
        refused: 0,
        pulled: 0,
        cursor: link.cursor,
    };
    for batch in entries.chunks(HUB_BATCH_MAX as usize) {
        let envelopes: Vec<_> = batch.iter().map(|e| e.envelope.clone()).collect();
        let ResponseBody::HubOutcomes { outcomes } = calls.hub(
            transport,
            &target,
            session_id.clone(),
            Capability::HubSync,
            RequestBody::HubSubmit { envelopes },
        )?
        else {
            return Err(unexpected("submit"));
        };
        if outcomes.len() != batch.len() {
            return Err(AuthorityError::Corrupt {
                message: "the Hub answered a different number of envelopes".to_owned(),
            });
        }
        let recorded: Vec<(u64, SyncOutcome)> = batch
            .iter()
            .zip(outcomes)
            .map(|(e, o)| (e.envelope.body.seq, o))
            .collect();
        for (_, outcome) in &recorded {
            report.submitted += 1;
            match outcome {
                SyncOutcome::Applied { .. } | SyncOutcome::ConflictCopy { .. } => {
                    report.applied += 1;
                }
                SyncOutcome::Conflict { .. } => report.conflicts += 1,
                SyncOutcome::Refused { .. } => report.refused += 1,
            }
        }
        calls.client(
            ctx,
            Capability::HubClient,
            RequestBody::HubRecordOutcomes {
                link_id: link_id.clone(),
                outcomes: recorded,
            },
        )?;
    }

    let mut cursor = link.cursor;
    loop {
        let ResponseBody::HubEvents { page } = calls.hub(
            transport,
            &target,
            session_id.clone(),
            Capability::HubSync,
            RequestBody::HubPull {
                after: cursor,
                limit: HUB_BATCH_MAX,
            },
        )?
        else {
            return Err(unexpected("pull"));
        };
        let got = page.events.len() as u32;
        let head = page.head.cursor;
        let ResponseBody::HubLink { link } = calls.client(
            ctx,
            Capability::HubClient,
            RequestBody::HubMirrorAppend {
                link_id: link_id.clone(),
                page: *page,
            },
        )?
        else {
            return Err(unexpected("mirror append"));
        };
        report.pulled += got;
        cursor = link.cursor;
        if got == 0 || cursor >= head {
            break;
        }
    }
    report.cursor = cursor;
    Ok(report)
}

/// Serves a Hub vault over the Spec 024 local-socket IPC for `connections`
/// client connections, then returns. The host lock lives in `host_dir`
/// (the vault's own writer lock is taken when the served Core opens it).
/// Realm and scope are the CLI operator's, so invitations issued from the
/// CLI name the Hub devices will reach.
pub fn serve_hub(
    vault_id: &str,
    vault_root: &std::path::Path,
    host_dir: &std::path::Path,
    endpoint: &str,
    connections: u32,
) -> Result<(), AuthorityError> {
    let unavailable = |e: crate::ipc::HostIpcError| AuthorityError::Unavailable {
        message: e.to_string(),
    };
    std::fs::create_dir_all(host_dir).map_err(|e| AuthorityError::Unavailable {
        message: e.to_string(),
    })?;
    let server = crate::ipc::HostIpcServer::bind(host_dir, endpoint).map_err(unavailable)?;
    let facade = server.facade();
    let mut calls = Calls { next: 0 };
    let target = HubTarget {
        vault_id: VaultId::new(vault_id),
        realm_id: RealmId::new("cli-realm"),
        scope_id: AuthorityScopeId::new("cli-scope"),
    };
    let mut local = InProcessHubTransport { hub: facade };
    let ResponseBody::Lease { holder_id, .. } = calls.hub(
        &mut local,
        &target,
        None,
        Capability::AcquireLease,
        RequestBody::AcquireLease {
            client_id: OpaqueId::new("medscale-hub"),
            holder_id_hint: Some(OpaqueId::new("hub-host")),
        },
    )?
    else {
        return Err(unexpected("lease"));
    };
    let ResponseBody::Session { session_id, .. } = calls.hub(
        &mut local,
        &target,
        None,
        Capability::OpenSession,
        RequestBody::OpenSession {
            holder_id,
            granted: vec![Capability::OpenSyntheticVault],
            ttl_ticks: 1_000_000,
        },
    )?
    else {
        return Err(unexpected("session"));
    };
    calls.hub(
        &mut local,
        &target,
        Some(session_id),
        Capability::OpenSyntheticVault,
        RequestBody::OpenSyntheticVault {
            vault_root: vault_root.display().to_string(),
        },
    )?;
    // Devices reach only Hub bootstrap and sync; operator commands run on
    // the vault directly, never through this endpoint.
    for _ in 0..connections {
        server
            .serve_connection_limited(&[Capability::HubBootstrap, Capability::HubSync])
            .map_err(unavailable)?;
    }
    Ok(())
}

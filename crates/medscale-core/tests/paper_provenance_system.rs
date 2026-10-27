//! Paper Experiment A1: Core-level provenance and authority binding controls.
//!
//! Scientific base: `1e2b7d94e970256b38bda15fa91f62bc397e825a`.
//! This harness exercises the production `CoreFacade` source/derived-object path
//! under strict session enforcement. It does not claim durable-storage tamper
//! resistance, complete mediation of every subsystem, or clinical safety.

use medscale_contracts::envelopes::{
    AuthorityError, AuthorityRequest, Capability, RequestBody, ResponseBody,
};
use medscale_contracts::objects::{
    AuthorityScopeId, DerivedSourceArtifact, DigestSha256, OpaqueId, RealmId, VaultId,
};
use medscale_core::CoreFacade;

const REALM: &str = "paper-realm";
const SCOPE_A: &str = "paper-scope-a";
const SCOPE_B: &str = "paper-scope-b";
const VAULT: &str = "paper-vault";

fn request(scope: &str, id: u64, capability: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("paper-a1-req-{id}")),
        VaultId::new(VAULT),
        RealmId::new(REALM),
        AuthorityScopeId::new(scope),
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
            SCOPE_A,
            1,
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("paper-a1-client"),
                holder_id_hint: Some(OpaqueId::new("paper-a1-holder")),
            },
        ));
        let holder_id = match lease.result.expect("lease must open") {
            ResponseBody::Lease { holder_id, .. } => holder_id,
            other => panic!("unexpected lease response: {other:?}"),
        };

        let opened = facade.dispatch(request(
            SCOPE_A,
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
        scope: &str,
        capability: Capability,
        body: RequestBody,
    ) -> Result<ResponseBody, AuthorityError> {
        self.next_request += 1;
        let mut req = request(scope, self.next_request, capability, body);
        req.session_id = Some(self.session.clone());
        self.facade.dispatch(req).result
    }

    fn create_source(&mut self, scope: &str, bytes: &[u8]) -> OpaqueId {
        match self
            .call(
                scope,
                Capability::CreateSourceRecord,
                RequestBody::CreateSourceRecord {
                    media_type: "application/json".to_owned(),
                    bytes: bytes.to_vec(),
                },
            )
            .expect("source create")
        {
            ResponseBody::Created { object_id } => object_id,
            other => panic!("unexpected source response: {other:?}"),
        }
    }
}

fn provenance_grants() -> Vec<Capability> {
    vec![
        Capability::CreateSourceRecord,
        Capability::CreateDerivedArtifact,
        Capability::ReadObject,
    ]
}

#[test]
fn a1_strict_mutation_without_session_is_refused() {
    let facade = CoreFacade::new();
    let out = facade.dispatch(request(
        SCOPE_A,
        1,
        Capability::CreateSourceRecord,
        RequestBody::CreateSourceRecord {
            media_type: "text/plain".to_owned(),
            bytes: b"synthetic".to_vec(),
        },
    ));

    assert_eq!(out.result, Err(AuthorityError::SessionRequired));
}

#[test]
fn a1_session_without_required_capability_is_refused() {
    let mut h = Harness::with_grants(vec![Capability::ReadObject]);
    let out = h.call(
        SCOPE_A,
        Capability::CreateSourceRecord,
        RequestBody::CreateSourceRecord {
            media_type: "text/plain".to_owned(),
            bytes: b"synthetic".to_vec(),
        },
    );

    assert_eq!(out, Err(AuthorityError::SessionDenied));
}

#[test]
fn a1_valid_derived_artifact_binds_exact_source_and_output_digest() {
    let mut h = Harness::with_grants(provenance_grants());
    let source_id = h.create_source(SCOPE_A, br#"{"value":1}"#);
    let output = b"normalized synthetic evidence".to_vec();

    let derived_id = match h
        .call(
            SCOPE_A,
            Capability::CreateDerivedArtifact,
            RequestBody::CreateDerivedArtifact {
                source_id: source_id.clone(),
                transform_id: "paper.normalize".to_owned(),
                transform_version: "1".to_owned(),
                bytes: output.clone(),
            },
        )
        .expect("derived create")
    {
        ResponseBody::Created { object_id } => object_id,
        other => panic!("unexpected derived response: {other:?}"),
    };

    let value = match h
        .call(
            SCOPE_A,
            Capability::ReadObject,
            RequestBody::ReadObject {
                object_id: derived_id,
            },
        )
        .expect("derived read")
    {
        ResponseBody::Object { value } => value,
        other => panic!("unexpected read response: {other:?}"),
    };
    let artifact: DerivedSourceArtifact = serde_json::from_value(value).expect("derived decode");

    assert_eq!(artifact.source_id, source_id);
    assert_eq!(artifact.transform_id, "paper.normalize");
    assert_eq!(artifact.transform_version, "1");
    assert_eq!(artifact.bytes, output);
    assert_eq!(artifact.content_digest, DigestSha256::of(&artifact.bytes));
}

#[test]
fn a1_cross_scope_source_binding_is_refused() {
    let mut h = Harness::with_grants(provenance_grants());
    let source_id = h.create_source(SCOPE_A, b"scope-a evidence");

    let out = h.call(
        SCOPE_B,
        Capability::CreateDerivedArtifact,
        RequestBody::CreateDerivedArtifact {
            source_id,
            transform_id: "paper.normalize".to_owned(),
            transform_version: "1".to_owned(),
            bytes: b"derived".to_vec(),
        },
    );

    assert_eq!(out, Err(AuthorityError::WrongScope));
}

#[test]
fn a1_unknown_source_binding_is_refused() {
    let mut h = Harness::with_grants(provenance_grants());

    let out = h.call(
        SCOPE_A,
        Capability::CreateDerivedArtifact,
        RequestBody::CreateDerivedArtifact {
            source_id: OpaqueId::new("source-that-does-not-exist"),
            transform_id: "paper.normalize".to_owned(),
            transform_version: "1".to_owned(),
            bytes: b"derived".to_vec(),
        },
    );

    assert_eq!(out, Err(AuthorityError::NotFound));
}

#[test]
fn a1_cross_scope_read_is_refused() {
    let mut h = Harness::with_grants(provenance_grants());
    let source_id = h.create_source(SCOPE_A, b"scope-a evidence");

    let out = h.call(
        SCOPE_B,
        Capability::ReadObject,
        RequestBody::ReadObject {
            object_id: source_id,
        },
    );

    assert_eq!(out, Err(AuthorityError::WrongScope));
}

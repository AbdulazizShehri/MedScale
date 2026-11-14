//! Spec 013 Network Broker tests.

use medscale_contracts::envelopes::{AuthorityRequest, Capability, RequestBody, ResponseBody};
use medscale_contracts::network::{
    BrokerDecision, BrokerReasonCode, EgressAllowlistEntry, EgressDataClass, EgressPurpose,
    NetworkBrokerRequest,
};
use medscale_contracts::objects::{AuthorityScopeId, OpaqueId, RealmId, VaultId};
use medscale_core::CoreFacade;
use medscale_network::{
    FhirPartnerAdapter, FixtureTransport, SmartAuthAdapter, StubFhirAdapter, StubSmartAdapter,
    broker_invoke, evaluate_allowlist, integrity_matches, live_partner_refused,
};
use std::sync::atomic::{AtomicU64, Ordering};

fn uid() -> u64 {
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::SeqCst)
}

fn req(cap: Capability, body: RequestBody) -> AuthorityRequest {
    AuthorityRequest::new(
        OpaqueId::new(format!("r-{}", uid())),
        VaultId::new("v-013"),
        RealmId::new("realm-013"),
        AuthorityScopeId::new("scope-013"),
        cap,
        body,
    )
}

fn lease(facade: &CoreFacade) {
    facade
        .dispatch(req(
            Capability::AcquireLease,
            RequestBody::AcquireLease {
                client_id: OpaqueId::new("cli"),
                holder_id_hint: None,
            },
        ))
        .result
        .unwrap();
}

#[test]
fn broker_deny_empty_allowlist_no_transport() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    let out = facade
        .dispatch(req(
            Capability::NetworkBrokerInvoke,
            RequestBody::NetworkBrokerInvoke {
                request: NetworkBrokerRequest {
                    destination_host: "partner.fixture".to_owned(),
                    destination_path: "/fhir/Patient/1".to_owned(),
                    purpose: EgressPurpose::FhirReadFixture,
                    data_class: EgressDataClass::SyntheticFixture,
                    authorization_token_id: None,
                    body_digest: None,
                    fixture_id: Some("p1".to_owned()),
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::NetworkBroker { result } = out else {
        panic!("expected broker");
    };
    assert_eq!(result.decision, BrokerDecision::Deny);
    assert_eq!(result.reason, BrokerReasonCode::EmptyAllowlist);
    assert!(!result.transport_sent);
}

#[test]
fn broker_allow_fixture_receipt() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    facade
        .dispatch(req(
            Capability::SetEgressAllowlist,
            RequestBody::SetEgressAllowlist {
                entries: vec![EgressAllowlistEntry {
                    host: "partner.fixture".to_owned(),
                    path_prefix: "/fhir/".to_owned(),
                    purposes: vec![EgressPurpose::FhirReadFixture],
                    data_classes: vec![EgressDataClass::SyntheticFixture],
                    enabled: true,
                }],
            },
        ))
        .result
        .unwrap();
    let out = facade
        .dispatch(req(
            Capability::NetworkBrokerInvoke,
            RequestBody::NetworkBrokerInvoke {
                request: NetworkBrokerRequest {
                    destination_host: "partner.fixture".to_owned(),
                    destination_path: "/fhir/Patient/1".to_owned(),
                    purpose: EgressPurpose::FhirReadFixture,
                    data_class: EgressDataClass::SyntheticFixture,
                    authorization_token_id: None,
                    body_digest: None,
                    fixture_id: Some("patient-1".to_owned()),
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::NetworkBroker { result } = out else {
        panic!();
    };
    assert_eq!(result.decision, BrokerDecision::Allow);
    assert!(result.transport_sent);
    assert!(result.fixture_body.unwrap().contains("fixture"));
}

#[test]
fn real_phi_data_class_refused() {
    let outcome = broker_invoke(
        &[],
        &NetworkBrokerRequest {
            destination_host: "x".to_owned(),
            destination_path: "/".to_owned(),
            purpose: EgressPurpose::FhirReadFixture,
            data_class: EgressDataClass::RealPhiForbidden,
            authorization_token_id: None,
            body_digest: None,
            fixture_id: None,
        },
        &FixtureTransport,
    );
    assert_eq!(outcome.reason, BrokerReasonCode::DataClassRefused);
    assert!(!outcome.transport_sent);
}

#[test]
fn stub_adapters_and_live_refused() {
    let list = vec![
        EgressAllowlistEntry {
            host: "smart.fixture".to_owned(),
            path_prefix: "/".to_owned(),
            purposes: vec![EgressPurpose::SmartDiscovery],
            data_classes: vec![EgressDataClass::SyntheticFixture],
            enabled: true,
        },
        EgressAllowlistEntry {
            host: "fhir.fixture".to_owned(),
            path_prefix: "/fhir/".to_owned(),
            purposes: vec![EgressPurpose::FhirReadFixture],
            data_classes: vec![EgressDataClass::SyntheticFixture],
            enabled: true,
        },
    ];
    let smart = StubSmartAdapter.discover(&list, &FixtureTransport, "smart.fixture");
    assert_eq!(smart.decision, BrokerDecision::Allow);
    let fhir = StubFhirAdapter.read_fixture(
        &list,
        &FixtureTransport,
        "fhir.fixture",
        "/fhir/Patient/1",
        "p1",
    );
    assert_eq!(fhir.decision, BrokerDecision::Allow);
    let live = live_partner_refused();
    assert_eq!(live.reason, BrokerReasonCode::ExternalGateRequired);
    assert!(!live.transport_sent);
}

#[test]
fn integrity_helper() {
    let bytes = b"hello";
    let hex = {
        use sha2::{Digest, Sha256};
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert!(integrity_matches(&hex, bytes));
    assert!(!integrity_matches("00", bytes));
}

#[test]
fn cli_and_desktop_do_not_depend_on_ureq() {
    let cli = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../medscale-cli/Cargo.toml"
    ))
    .unwrap();
    let desktop = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../medscale-desktop/Cargo.toml"
    ))
    .unwrap();
    assert!(!cli.contains("ureq"));
    assert!(!desktop.contains("ureq"));
    assert!(!cli.contains("medscale-network"));
    assert!(!desktop.contains("medscale-network"));
}

#[test]
fn purpose_mismatch_denies() {
    use medscale_network::AllowlistDecision;
    let list = vec![EgressAllowlistEntry {
        host: "partner.fixture".to_owned(),
        path_prefix: "/fhir/".to_owned(),
        purposes: vec![EgressPurpose::FhirReadFixture],
        data_classes: vec![EgressDataClass::SyntheticFixture],
        enabled: true,
    }];
    let d = evaluate_allowlist(
        &list,
        &NetworkBrokerRequest {
            destination_host: "partner.fixture".to_owned(),
            destination_path: "/fhir/Patient/1".to_owned(),
            purpose: EgressPurpose::SmartDiscovery,
            data_class: EgressDataClass::SyntheticFixture,
            authorization_token_id: None,
            body_digest: None,
            fixture_id: None,
        },
    );
    assert_eq!(
        d,
        AllowlistDecision::Deny(BrokerReasonCode::PurposeMismatch)
    );
}

fn allow_partner(facade: &CoreFacade) {
    facade
        .dispatch(req(
            Capability::SetEgressAllowlist,
            RequestBody::SetEgressAllowlist {
                entries: vec![EgressAllowlistEntry {
                    host: "partner.fixture".to_owned(),
                    path_prefix: "/fhir/".to_owned(),
                    purposes: vec![EgressPurpose::FhirReadFixture],
                    data_classes: vec![EgressDataClass::SyntheticFixture],
                    enabled: true,
                }],
            },
        ))
        .result
        .unwrap();
}

#[test]
fn fixture_transport_timeout_maps_stable_reason() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    allow_partner(&facade);
    let out = facade
        .dispatch(req(
            Capability::NetworkBrokerInvoke,
            RequestBody::NetworkBrokerInvoke {
                request: NetworkBrokerRequest {
                    destination_host: "partner.fixture".to_owned(),
                    destination_path: "/fhir/Patient/1".to_owned(),
                    purpose: EgressPurpose::FhirReadFixture,
                    data_class: EgressDataClass::SyntheticFixture,
                    authorization_token_id: None,
                    body_digest: None,
                    fixture_id: Some("fail:timeout".to_owned()),
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::NetworkBroker { result } = out else {
        panic!("expected broker");
    };
    assert_eq!(result.decision, BrokerDecision::Deny);
    assert_eq!(result.reason, BrokerReasonCode::TransportTimeout);
    assert!(result.transport_sent);
    assert!(result.fixture_body.is_none());
}

#[test]
fn fixture_transport_failed_maps_stable_reason() {
    let facade = CoreFacade::new_legacy_lease_only_engineering();
    lease(&facade);
    allow_partner(&facade);
    let out = facade
        .dispatch(req(
            Capability::NetworkBrokerInvoke,
            RequestBody::NetworkBrokerInvoke {
                request: NetworkBrokerRequest {
                    destination_host: "partner.fixture".to_owned(),
                    destination_path: "/fhir/Patient/1".to_owned(),
                    purpose: EgressPurpose::FhirReadFixture,
                    data_class: EgressDataClass::SyntheticFixture,
                    authorization_token_id: None,
                    body_digest: None,
                    fixture_id: Some("fail:transport".to_owned()),
                },
            },
        ))
        .result
        .unwrap();
    let ResponseBody::NetworkBroker { result } = out else {
        panic!("expected broker");
    };
    assert_eq!(result.decision, BrokerDecision::Deny);
    assert_eq!(result.reason, BrokerReasonCode::TransportFailed);
    assert!(result.transport_sent);
}

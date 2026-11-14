//! SMART / FHIR partner adapters — fixture stubs only (Spec 013).

use medscale_contracts::network::{
    EgressAllowlistEntry, EgressDataClass, EgressPurpose, NetworkBrokerRequest,
};

use crate::{BrokerInvokeOutcome, BrokerTransport, broker_invoke};

pub trait SmartAuthAdapter {
    fn discover(
        &self,
        allowlist: &[EgressAllowlistEntry],
        transport: &dyn BrokerTransport,
        host: &str,
    ) -> BrokerInvokeOutcome;
}

pub trait FhirPartnerAdapter {
    fn read_fixture(
        &self,
        allowlist: &[EgressAllowlistEntry],
        transport: &dyn BrokerTransport,
        host: &str,
        resource_path: &str,
        fixture_id: &str,
    ) -> BrokerInvokeOutcome;
}

#[derive(Debug, Default)]
pub struct StubSmartAdapter;

impl SmartAuthAdapter for StubSmartAdapter {
    fn discover(
        &self,
        allowlist: &[EgressAllowlistEntry],
        transport: &dyn BrokerTransport,
        host: &str,
    ) -> BrokerInvokeOutcome {
        broker_invoke(
            allowlist,
            &NetworkBrokerRequest {
                destination_host: host.to_owned(),
                destination_path: "/.well-known/smart-configuration".to_owned(),
                purpose: EgressPurpose::SmartDiscovery,
                data_class: EgressDataClass::SyntheticFixture,
                authorization_token_id: None,
                body_digest: None,
                fixture_id: Some("smart-discovery-v1".to_owned()),
            },
            transport,
        )
    }
}

#[derive(Debug, Default)]
pub struct StubFhirAdapter;

impl FhirPartnerAdapter for StubFhirAdapter {
    fn read_fixture(
        &self,
        allowlist: &[EgressAllowlistEntry],
        transport: &dyn BrokerTransport,
        host: &str,
        resource_path: &str,
        fixture_id: &str,
    ) -> BrokerInvokeOutcome {
        broker_invoke(
            allowlist,
            &NetworkBrokerRequest {
                destination_host: host.to_owned(),
                destination_path: resource_path.to_owned(),
                purpose: EgressPurpose::FhirReadFixture,
                data_class: EgressDataClass::SyntheticFixture,
                authorization_token_id: None,
                body_digest: None,
                fixture_id: Some(fixture_id.to_owned()),
            },
            transport,
        )
    }
}

/// Live partner mode — always external-gate refuse (no socket).
#[must_use]
pub fn live_partner_refused() -> BrokerInvokeOutcome {
    use medscale_contracts::network::{BrokerDecision, BrokerReasonCode};
    BrokerInvokeOutcome {
        decision: BrokerDecision::Deny,
        reason: BrokerReasonCode::ExternalGateRequired,
        transport_sent: false,
        fixture_body: None,
    }
}

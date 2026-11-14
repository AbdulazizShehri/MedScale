//! Network Broker: fail-closed allowlist + transport trait (Spec 013).

mod adapters;
mod allowlist;
mod browse;
mod transport;

pub use adapters::{
    FhirPartnerAdapter, SmartAuthAdapter, StubFhirAdapter, StubSmartAdapter, live_partner_refused,
};
pub use allowlist::{AllowlistDecision, evaluate_allowlist};
pub use browse::{
    BrowseFileResponse, BrowseHttpResponse, BrowseTransport, BrowseTransportError,
    PublicOnlyResolver, ScriptedBrowseTransport, UreqBrowseTransport, ValidatedUrl,
    is_forbidden_ip, resolve_redirect, validate_url,
};
pub use transport::{
    BrokerTransport, FixtureTransport, TransportError, TransportRequest, UreqTransport,
};

use medscale_contracts::network::{
    BrokerDecision, BrokerReasonCode, EgressAllowlistEntry, EgressDataClass, NetworkBrokerRequest,
};

/// Evaluate policy then optionally send via transport.
pub fn broker_invoke(
    allowlist: &[EgressAllowlistEntry],
    request: &NetworkBrokerRequest,
    transport: &dyn BrokerTransport,
) -> BrokerInvokeOutcome {
    if request.data_class == EgressDataClass::RealPhiForbidden {
        return BrokerInvokeOutcome {
            decision: BrokerDecision::Deny,
            reason: BrokerReasonCode::DataClassRefused,
            transport_sent: false,
            fixture_body: None,
        };
    }

    match evaluate_allowlist(allowlist, request) {
        AllowlistDecision::Deny(reason) => BrokerInvokeOutcome {
            decision: BrokerDecision::Deny,
            reason,
            transport_sent: false,
            fixture_body: None,
        },
        AllowlistDecision::Allow => match transport.send(&TransportRequest {
            host: request.destination_host.clone(),
            path: request.destination_path.clone(),
            purpose: request.purpose,
            fixture_id: request.fixture_id.clone(),
        }) {
            Ok(body) => BrokerInvokeOutcome {
                decision: BrokerDecision::Allow,
                reason: BrokerReasonCode::TransportFixtureOk,
                transport_sent: true,
                fixture_body: Some(body),
            },
            Err(TransportError::ExternalGateRequired) => BrokerInvokeOutcome {
                decision: BrokerDecision::Deny,
                reason: BrokerReasonCode::ExternalGateRequired,
                transport_sent: false,
                fixture_body: None,
            },
            Err(TransportError::Timeout(_)) => BrokerInvokeOutcome {
                decision: BrokerDecision::Deny,
                reason: BrokerReasonCode::TransportTimeout,
                transport_sent: true,
                fixture_body: None,
            },
            Err(TransportError::Failed(_)) => BrokerInvokeOutcome {
                decision: BrokerDecision::Deny,
                reason: BrokerReasonCode::TransportFailed,
                transport_sent: true,
                fixture_body: None,
            },
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerInvokeOutcome {
    pub decision: BrokerDecision,
    pub reason: BrokerReasonCode,
    pub transport_sent: bool,
    pub fixture_body: Option<String>,
}

/// Integrity helper: compare SHA-256 digests.
#[must_use]
pub fn integrity_matches(expected_hex: &str, bytes: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex == expected_hex
}

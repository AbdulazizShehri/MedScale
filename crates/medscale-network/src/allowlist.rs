//! Fail-closed egress allowlist.

use medscale_contracts::network::{BrokerReasonCode, EgressAllowlistEntry, NetworkBrokerRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowlistDecision {
    Allow,
    Deny(BrokerReasonCode),
}

/// Empty allowlist ⇒ all Deny (DEFAULT_DENY compatible).
#[must_use]
pub fn evaluate_allowlist(
    allowlist: &[EgressAllowlistEntry],
    request: &NetworkBrokerRequest,
) -> AllowlistDecision {
    if allowlist.is_empty() {
        return AllowlistDecision::Deny(BrokerReasonCode::EmptyAllowlist);
    }

    let mut host_seen = false;
    for entry in allowlist {
        if !entry.enabled {
            continue;
        }
        if entry.host != request.destination_host {
            continue;
        }
        host_seen = true;
        if !request.destination_path.starts_with(&entry.path_prefix) {
            continue;
        }
        if !entry.purposes.contains(&request.purpose) {
            return AllowlistDecision::Deny(BrokerReasonCode::PurposeMismatch);
        }
        if !entry.data_classes.contains(&request.data_class) {
            return AllowlistDecision::Deny(BrokerReasonCode::DataClassRefused);
        }
        return AllowlistDecision::Allow;
    }

    if host_seen {
        AllowlistDecision::Deny(BrokerReasonCode::PurposeMismatch)
    } else {
        AllowlistDecision::Deny(BrokerReasonCode::UnknownDestination)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use medscale_contracts::network::{EgressDataClass, EgressPurpose};

    fn req(
        host: &str,
        path: &str,
        purpose: EgressPurpose,
        class: EgressDataClass,
    ) -> NetworkBrokerRequest {
        NetworkBrokerRequest {
            destination_host: host.to_owned(),
            destination_path: path.to_owned(),
            purpose,
            data_class: class,
            authorization_token_id: None,
            body_digest: None,
            fixture_id: None,
        }
    }

    #[test]
    fn empty_allowlist_denies() {
        let d = evaluate_allowlist(
            &[],
            &req(
                "example.test",
                "/",
                EgressPurpose::FhirReadFixture,
                EgressDataClass::SyntheticFixture,
            ),
        );
        assert_eq!(d, AllowlistDecision::Deny(BrokerReasonCode::EmptyAllowlist));
    }

    #[test]
    fn unknown_host_denies() {
        let list = vec![EgressAllowlistEntry {
            host: "partner.fixture".to_owned(),
            path_prefix: "/fhir/".to_owned(),
            purposes: vec![EgressPurpose::FhirReadFixture],
            data_classes: vec![EgressDataClass::SyntheticFixture],
            enabled: true,
        }];
        let d = evaluate_allowlist(
            &list,
            &req(
                "evil.test",
                "/fhir/Patient",
                EgressPurpose::FhirReadFixture,
                EgressDataClass::SyntheticFixture,
            ),
        );
        assert_eq!(
            d,
            AllowlistDecision::Deny(BrokerReasonCode::UnknownDestination)
        );
    }
}

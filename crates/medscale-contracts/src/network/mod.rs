//! Network Broker contract types (Spec 013).

use serde::{Deserialize, Serialize};

use crate::objects::{DigestSha256, OpaqueId};

/// Purpose of a brokered egress attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressPurpose {
    SmartDiscovery,
    SmartTokenFixture,
    FhirReadFixture,
    FhirSearchFixture,
    ProfileOracleFixture,
    IntegrityCheck,
    ConformanceEvidenceAttach,
    DatasetMirrorRead,
}

/// Data class declared on the request (MVP vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressDataClass {
    SyntheticFixture,
    NonPhiMetadata,
    RedactedEvidence,
    /// Explicitly refused for product egress in Spec 013.
    RealPhiForbidden,
}

/// Allowlist match decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerDecision {
    Allow,
    Deny,
}

/// Typed deny / outcome reason codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerReasonCode {
    UnknownDestination,
    PurposeMismatch,
    DataClassRefused,
    Unauthorized,
    EmptyAllowlist,
    ExternalGateRequired,
    TransportFixtureOk,
    TransportTimeout,
    TransportFailed,
    IntegrityMismatch,
    Ok,
}

/// One allowlist entry (exact host + path prefix).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EgressAllowlistEntry {
    pub host: String,
    pub path_prefix: String,
    pub purposes: Vec<EgressPurpose>,
    pub data_classes: Vec<EgressDataClass>,
    pub enabled: bool,
}

/// Broker request envelope (no secrets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkBrokerRequest {
    pub destination_host: String,
    pub destination_path: String,
    pub purpose: EgressPurpose,
    pub data_class: EgressDataClass,
    pub authorization_token_id: Option<OpaqueId>,
    pub body_digest: Option<DigestSha256>,
    pub fixture_id: Option<String>,
}

/// Broker response + receipt binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkBrokerResult {
    pub decision: BrokerDecision,
    pub reason: BrokerReasonCode,
    pub audit_id: OpaqueId,
    pub evaluation_id: Option<OpaqueId>,
    pub transport_sent: bool,
    pub fixture_body: Option<String>,
}

/// Doctor section for network broker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkBrokerDoctorStatus {
    pub present: bool,
    pub default_deny: bool,
    pub allowlist_entries: u32,
    pub live_partner_authorized: bool,
    pub http_client: String,
}

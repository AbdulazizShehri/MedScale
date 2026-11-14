# Data Model: Spec 013 FHIR / SMART / Network Broker

**Date**: 2026-08-25  
**Status**: Planning-canonical for Spec 013 implement  
**Persistence**: No new clinical authority object classes. Broker outcomes persist as Spec 002 **ActionAuditRecord** and **EvaluationRecord**. Synthetic/fixture-only content.

## Principles

1. Spec 002–006 class invariants remain in force (Proposal≠Assertion; Projection≠truth; EvaluationRecord evidence_only; EncryptedVault production path).
2. Network Broker is the only product egress path; clients hold no ambient sockets.
3. FHIR R4 4.0.1 bytes remain interchange—not canonical DB.
4. REAL_PHI unauthorized; fixtures synthetic.
5. DEFAULT_DENY except broker-mediated allowlisted authorized egress.
6. No MESC objects or mutation.

## Inherited (read-only consumption)

| Source | Types used |
|---|---|
| Spec 002 | ActionAuditRecord, EvaluationRecord, ObjectHeader, DigestSha256, EffectState, AuthorityRequest/Response, Capability |
| Spec 003 | FHIR interchange custody / evidence-only validator patterns |
| Spec 005 | EncryptedVault durability for objects |
| Spec 006 | DoctorReport.network_broker axis (upgrade from placeholder) |

## New Spec 013 entities (contracts / operational)

### NetworkBrokerRequest

| Field | Required | Description |
|---|---|---|
| `request_id` | yes | Opaque correlation id |
| `destination` | yes | `BrokerDestination` (scheme, host, port, path_prefix) |
| `purpose` | yes | `EgressPurpose` enum |
| `data_class` | yes | `EgressDataClass` enum |
| `authorization` | yes | Scope/capability/actor refs sufficient for audit |
| `payload_digest` | optional | SHA-256 of body when present |
| `payload_class` | optional | e.g. `none` \| `fhir_json` \| `smart_token_fixture` |
| `timeout_ms` | optional | Bound; default policy in research/implement |

### BrokerDestination

| Field | Required | Description |
|---|---|---|
| `scheme` | yes | `https` preferred; `http` only for explicit fixture loopback if ever allowed |
| `host` | yes | Exact host string (no glob on Deny path—allowlist may use exact or documented prefix rules) |
| `port` | optional | Default by scheme |
| `path_prefix` | optional | Must be prefix-matched when allowlist specifies |

### EgressPurpose (MVP)

```text
FhirPartnerRead | FhirPartnerSearch | SmartDiscovery | SmartTokenFixture | ConformanceProbe
```

`PackAcquire` reserved for Spec 015 — Deny if presented in 013.

### EgressDataClass (MVP)

```text
SyntheticFixture | PublicMetadata | InterchangeFhirR4 | SmartTokenEnvelopeFixture
```

`RealPhi` and unrestricted `SecretsOrCredentials` → always Deny while gates hold.

### NetworkBrokerDecision

| Field | Required | Description |
|---|---|---|
| `decision` | yes | `Allow` \| `Deny` |
| `reason_code` | yes | e.g. `Allowlisted` \| `UnknownDestination` \| `PurposeMismatch` \| `DataClassRefused` \| `Unauthorized` \| `RealPhiForbidden` |
| `allowlist_entry_id` | optional | When matched |
| `decided_at` | yes | MedicalTime / ISO as documented |

### EgressAllowlistEntry

| Field | Required | Description |
|---|---|---|
| `entry_id` | yes | Opaque |
| `destination` | yes | Match rule |
| `purposes` | yes | Allowed purpose set |
| `data_classes` | yes | Allowed data-class set |
| `enabled` | yes | bool |
| `notes` | optional | Non-secret operational note |

**Invariant**: No match or `enabled=false` ⇒ Deny.

### NetworkBrokerReceipt (logical view)

Not a new durable class—**projection over** ActionAuditRecord (+ linked EvaluationRecord ids).

| Field | Required | Description |
|---|---|---|
| `audit_id` | yes | ActionAuditRecord id |
| `request_id` | yes | |
| `decision` | yes | |
| `outcome` | optional | `NotSent` \| `FixtureOk` \| `TransportFailed` \| `Timeout` (013); full 014 states later |
| `evaluation_refs` | optional | Profile/integrity/conformance EvaluationRecord ids |
| `response_digest` | optional | When body received |

### ActionAuditRecord mapping (egress)

| ActionAuditRecord field | Broker binding |
|---|---|
| `kind` | `Audit` for deny/attempt audit in 013 |
| `actor` | Calling authority actor / session |
| `action` | `network_broker.deny` \| `network_broker.attempt` \| `network_broker.fixture_ok` |
| `target_refs` | Optional related source/object ids |
| `payload_digest` | Request payload digest |
| `detail` | JSON: destination, purpose, data_class, decision, reason_code, response_digest?, transport=`ureq`\|`fixture` |

### EvaluationRecord mapping (FHIR/SMART evidence)

| EvaluationRecord field | Binding |
|---|---|
| `evaluator` | e.g. `fhir.profile.fixture_oracle`, `fhir.integrity.digest`, `smart.fixture.validate`, `inferno.report_digest` |
| `result` | Structured outcome codes + digests |
| `evidence_only` | **always true** |
| `target_refs` | SourceRecord / payload object ids when applicable |

### FhirPartnerAdapterRequest / Response

Operational DTOs: operation (`Read`\|`Search`), resource type/id, query params class, broker-mediated result bytes + receipt ids. No ambient HTTP handle.

### SmartAuthAdapterSession (fixture)

| Field | Required | Description |
|---|---|---|
| `mode` | yes | `FixtureStub` (013 only admitted mode) |
| `discovery_digest` | optional | |
| `token_envelope_digest` | optional | Never store raw live tokens in evidence archives |
| `receipt_ids` | yes | |

### DoctorNetworkBrokerSection (upgrade)

| Field | Required | Description |
|---|---|---|
| `status` | yes | `Available` \| `Unavailable` \| `Error` (no longer `NotImplemented`) |
| `posture` | yes | `DefaultDenyExceptBroker` |
| `allowlist_entries` | yes | count only |
| `http_client` | yes | label e.g. `ureq-3.4.0-rustls` (no secrets) |
| `last_receipt_at` | optional | |

## Serialization

- Broker APIs: typed Rust + serde for detail JSON in audits.
- Evidence samples under `evidence/013-fhir-smart-network-broker/`.
- Doctor: human + `--json` including upgraded network_broker section.

## Non-entities (forbidden)

- Uncontrolled `Client` handles exposed to workers/CLI
- Raw access tokens / client secrets in doctor or evidence samples
- ClinicalAssertion auto-created from brokered FHIR
- MESC records
- Real PHI payloads

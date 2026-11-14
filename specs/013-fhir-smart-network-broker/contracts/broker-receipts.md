# Contract: Broker Receipts (Spec 013)

**Spec**: 013-fhir-smart-network-broker  
**Status**: Durability mapping to Spec 002 object classes  
**Scope**: ActionAuditRecord + EvaluationRecord only; synthetic/fixture; no new clinical authority class

## Principles

1. Every broker **Allow** and **Deny** yields a durable **ActionAuditRecord**.
2. Profile / integrity / conformance outcomes yield **EvaluationRecord** with `evidence_only = true`.
3. Logical `NetworkBrokerReceipt` is a read model over those records—not a separate durable class.
4. No blind retry: timeout/transport failure recorded; Spec 014 owns full effect-state machine.

## ActionAuditRecord — egress

```text
ActionAuditRecord {
  header: ObjectHeader,
  kind: Audit,                          // 013 default
  actor: OpaqueId,
  action: "network_broker.deny" | "network_broker.attempt" | "network_broker.fixture_ok",
  target_refs: [...],
  effect_state: None | optional stub,   // full PENDING/SENT/... in 014
  payload_digest: Option<DigestSha256>,
  detail: {
    request_id,
    destination: { scheme, host, port?, path_prefix? },
    purpose,
    data_class,
    decision: Allow | Deny,
    reason_code,
    transport: "fixture" | "ureq",
    response_digest?,
    http_status?,                       // when transport ran
    limitations?: ["synthetic_only", ...]
  }
}
```

### Required reason codes (Deny)

```text
UnknownDestination
PurposeMismatch
DataClassRefused
Unauthorized
RealPhiForbidden
AllowlistDisabled
ExternalGateRequired
```

## EvaluationRecord — evidence

```text
EvaluationRecord {
  header: ObjectHeader,
  target_refs: [payload_or_source_ids...],
  evaluator: "fhir.profile.fixture_oracle"
           | "fhir.integrity.digest"
           | "smart.fixture.validate"
           | "inferno.report_digest",
  result: { ... structured outcome ... },
  evidence_only: true                   // CONSTANT
}
```

**Forbidden**: Creating ClinicalAssertion from EvaluationRecord or brokered FHIR alone.

## Query sketch

```text
Capability::GetNetworkBrokerReceipt { receipt_or_audit_id }
Capability::ListNetworkBrokerReceipts { filter: request_id? | since? }
```

Returns logical NetworkBrokerReceipt view.

## Tests required

- Deny → ActionAuditRecord present; evaluation optional
- Fixture Allow → ActionAuditRecord + optional EvaluationRecord links
- Restart durability: receipts reload from store/vault
- evidence_only invariant enforced in serde/tests
- No token plaintext in detail JSON of committed fixtures

# Contract: Network Broker (Spec 013)

**Spec**: 013-fhir-smart-network-broker  
**Status**: Message/API sketch for implement (extends Spec 002 authority facade)  
**Scope**: Sole product egress; fail-closed allowlist; ureq+rustls behind broker; synthetic/fixture; no live partner required; no REAL_PHI; no MESC

## Roles

| Role | Binding |
|---|---|
| `CoreHost` | Owns broker, allowlist, transport, receipt persistence |
| `Client` | CLI / Desktop / future workers / adapters — envelope callers only |
| `BrokerTransport` | Internal trait; production impl uses admitted ureq; tests may use FixtureTransport |
| `Worker` | No ambient network; may only call broker if explicitly capability-granted (default deny) |

## Hard rules

1. **Sole egress**: Product runtime TCP/TLS for online purposes only via Network Broker.
2. **Fail-closed allowlist**: Unknown destination → Deny; no send.
3. **Typed request**: destination + purpose + data-class + authorization required.
4. **Receipt always**: Allow and Deny produce ActionAuditRecord-backed receipt.
5. **No secret emission** in doctor, logs, or help text (tokens, client secrets).
6. **REAL_PHI data-class** → Deny while EXTERNAL_GATES REAL_PHI NOT_AUTHORIZED.
7. **ureq** may appear only in `medscale-network` (or designated broker module).

## Capability sketch

```text
Capability::NetworkBrokerInvoke
Capability::NetworkBrokerAllowlistRead     // status/count; not raw secret material
Capability::NetworkBrokerAllowlistAdmin    // optional; test/fixture seeding only in 013
Capability::FhirPartnerAdapterInvoke       // routes through broker
Capability::SmartAuthAdapterInvoke         // FixtureStub only in 013
```

Unknown capability → `AuthorityError::Unauthorized`.

## Request / response sketch

```text
AuthorityRequest::NetworkBroker {
  request: NetworkBrokerRequest
} -> AuthorityResponse::NetworkBroker {
  decision: NetworkBrokerDecision,
  receipt_id: OpaqueId,
  body: Option<BytesOrDigestHandle>,  // prefer digest + bounded buffer policy
  evaluation_refs: Vec<OpaqueId>
}
```

Deny path: `decision=Deny`, `body=None`, receipt still present, typed error mirrored to client.

## Allowlist match algorithm (normative intent)

1. Load enabled entries.
2. Match scheme+host(+port)+path_prefix per entry rules.
3. Require purpose ∈ entry.purposes.
4. Require data_class ∈ entry.data_classes.
5. Else Deny with most specific reason_code.
6. On Allow, invoke BrokerTransport once; map timeout/fail to outcome without blind retry.

## Forbidden

- `ureq::Agent` / raw sockets in `medscale-cli`, `medscale-desktop`, workers
- Provider SDKs that open their own HTTP stacks for product egress
- Default-allow localhost without allowlist row
- Silent retry loops on UNKNOWN/timeout (014 reconciles)

## Tests required

- Unknown destination Deny + receipt + zero transport calls
- Purpose / data-class mismatch Deny
- Unauthorized capability Deny
- Architecture: only network crate depends on ureq
- Doctor network_broker status without secrets

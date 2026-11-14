# Spec 013 evidence summary

## Delivered

- `medscale-network`: allowlist, FixtureTransport, UreqTransport (live gated), SMART/FHIR stub adapters
- Facade `SetEgressAllowlist` + `NetworkBrokerInvoke` with ActionAuditRecord receipts
- EvaluationRecord attachments for profile/integrity/conformance purposes
- Bypass: CLI/Desktop do not depend on ureq/medscale-network

## Exit gates

| Gate | Status |
|---|---|
| Empty allowlist Deny | PASS |
| Unknown host / purpose mismatch | PASS |
| RealPhiForbidden | PASS |
| Fixture Allow + receipt | PASS |
| Live partner ExternalGateRequired | PASS |
| Doctor network_broker axis | PASS |
| Inferno/SMART REFERENCE_ONLY notes | PASS |
| CDLA-Permissive-2.0 (webpki-roots) in deny allowlist | PASS |


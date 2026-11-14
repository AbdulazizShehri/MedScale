# Bypass tests (Spec 013)

1. **Dependency**: `medscale-cli` and `medscale-desktop` Cargo.toml must not list `ureq` or `medscale-network` (enforced in `network_broker_013` test).
2. **Capability**: facade requires `Capability::NetworkBrokerInvoke` matching body; mismatch → Unauthorized.
3. **Policy**: Deny paths never call transport send (`transport_sent=false`).
4. **Live**: `live_partner_refused()` and `UreqTransport` return ExternalGateRequired without sockets in Spec 013 posture.

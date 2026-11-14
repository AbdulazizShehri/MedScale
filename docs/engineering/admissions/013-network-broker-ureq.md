# Dependency admission: ureq Network Broker transport (Spec 013)

| Crate | Version | Features |
|---|---|---|
| ureq | **3.4.0** (Cargo.lock; workspace req 3.0.12+) | `default-features = false`, `rustls`, `json` |

| Field | Value |
|---|---|
| Owning Spec | 013 |
| Placement | `medscale-network` only |
| NOT admitted | reqwest, tokio (product), native-tls/OpenSSL product path |
| Purpose | Optional real TLS transport behind `BrokerTransport`; FixtureTransport is default for tests/CI |
| License | ureq MIT/Apache-2.0; transitive `webpki-roots` CDLA-Permissive-2.0 (allowed in deny.toml) |
| Exit strategy | Swap `UreqTransport` behind trait |
| Security | No ambient sockets from CLI/Desktop/workers; Core Host broker only |

If crates.io resolves a different 3.x patch, amend this file with Cargo.lock identity. FixtureTransport proves broker semantics without network.

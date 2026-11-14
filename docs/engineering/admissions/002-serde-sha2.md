# Dependency admissions for Spec 002

## serde 1.0.229

| Field | Value |
|---|---|
| Crate / component | serde (+ derive) |
| Version / revision pin | 1.0.229 (Cargo.lock) |
| Owning Spec | 002 |
| Purpose | Deterministic serialization of object/authority envelopes |
| Alternatives considered | Manual JSON; protobuf |
| License / NOTICE | MIT OR Apache-2.0 |
| Security / advisory review | cargo-deny advisories clean at admission |
| Transitive dependency notes | serde_derive, syn, quote, proc-macro2 |
| Unsafe / FFI surface | none in direct API |
| Placement | trusted core contracts |
| Tests required | serde round-trip suites |
| Update strategy | cargo update within minor; re-run deny |
| Exit strategy | replace with hand-rolled JSON for narrow types |
| SBOM / provenance path | Cargo.lock + deny |

## serde_json 1.0.151

| Field | Value |
|---|---|
| Crate / component | serde_json |
| Version / revision pin | 1.0.151 |
| Owning Spec | 002 |
| Purpose | JSON fixtures and `Value` payloads for proposals/assertions |
| Alternatives considered | simd-json; custom Value |
| License / NOTICE | MIT OR Apache-2.0 |
| Placement | trusted core contracts/core |
| Exit strategy | remove Value; use typed enums only |

## sha2 0.11.0

| Field | Value |
|---|---|
| Crate / component | sha2 |
| Version / revision pin | 0.11.0 |
| Owning Spec | 002 |
| Purpose | content_digest evidence metadata (not source identity) |
| Alternatives considered | sha1 (rejected); blake3 |
| License / NOTICE | MIT OR Apache-2.0 |
| Placement | trusted core |
| Exit strategy | swap digest crate behind DigestSha256 constructor |

## thiserror 2.0.20

| Field | Value |
|---|---|
| Crate / component | thiserror |
| Version / revision pin | 2.0.20 |
| Owning Spec | 002 |
| Purpose | typed lease/text errors |
| License / NOTICE | MIT OR Apache-2.0 |
| Placement | medscale-core |
| Exit strategy | manual Display/Error impls |

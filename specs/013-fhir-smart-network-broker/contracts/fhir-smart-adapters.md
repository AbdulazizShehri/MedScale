# Contract: FHIR / SMART Adapters (Spec 013)

**Spec**: 013-fhir-smart-network-broker  
**Status**: Adapter interface sketch for implement  
**Scope**: Stub + fixture receipts; FHIR R4 4.0.1 interchange; SMART FixtureStub mode; live partners EXTERNAL_GATES

## FHIR Partner Adapter

### Purpose

Typed partner FHIR read/search (and future write under later specs) **only** through Network Broker. Returned bytes are interchange; custody/promotion remain Spec 003 paths when ingested.

### Interface sketch

```text
FhirPartnerAdapter::read {
  resource_type, logical_id, destination_ref, data_class=InterchangeFhirR4|SyntheticFixture
} -> { bytes_digest, receipt_id, evaluation_refs }

FhirPartnerAdapter::search {
  resource_type, query_class, destination_ref, data_class=...
} -> { bundle_digest, receipt_id, evaluation_refs }
```

Both MUST call `Capability::NetworkBrokerInvoke` with purpose `FhirPartnerRead` / `FhirPartnerSearch`.

### Evidence

- Optional profile oracle → EvaluationRecord `evidence_only=true`
- Integrity digest mismatch → fail-closed; no ClinicalAssertion

### Anti-scope

- No general FHIRPath engine
- No treating partner FHIR as canonical DB
- No live partner host required for Spec 013 close

## SMART Auth Adapter

### Purpose

SMART-on-FHIR style discovery/token **adapter interface** with **FixtureStub** implementation in Spec 013.

### Modes

| Mode | Spec 013 |
|---|---|
| `FixtureStub` | **Admitted** — synthetic well-known + token envelope fixtures via broker |
| `LivePartner` | **Refused** until PARTNER_EHR_NPHIES_ENDPOINT + PRODUCTION_CREDENTIALS gates grant |

### Interface sketch

```text
SmartAuthAdapter::discover { destination_ref } -> { discovery_digest, receipt_id }
SmartAuthAdapter::token_fixture { grant_class } -> { token_envelope_digest, receipt_id }
SmartAuthAdapter::live_authorize { ... } -> AuthorityError::ExternalGateRequired
```

Purposes: `SmartDiscovery`, `SmartTokenFixture`.

### Secrets

- Fixture tokens may appear only in test memory; never in doctor output or committed evidence as live secrets.
- Evidence archives store digests + redacted shapes.

## Conformance evidence

- Inferno core **v1.4.2** / SMART test kit: REFERENCE_ONLY / EXTERNAL_CONFORMANCE.
- Spec 013 may archive report digests under evidence/; live Inferno run optional.

## Tests required

- Fixture SMART discover/token through broker → receipts
- Live mode refused with gate error
- FHIR fixture read/search through broker → digests + optional EvaluationRecord
- Tampered FHIR bytes fail integrity

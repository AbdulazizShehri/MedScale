# Contract: Trusted Ingest API (Synthetic FHIR R4)

**Spec**: 003-h0a-trusted-ingest-durability  
**Status**: Message/API sketch for implement (extends Spec 002 authority facade)  
**Scope**: Synthetic FHIR R4 4.0.1 only; no product network; validators = evidence only

## Roles

Uses Spec 002 roles (`CoreHost`, `Client`, `TransientHostOwner`). Workers do not ingest with ambient DB access.

## New capabilities

```text
Capability::IngestFhirSynthetic
Capability::AttachValidatorEvidence
Capability::RebuildProjection
Capability::ReadCanonicalVisibility
Capability::VerifyBlob
```

Unknown capability → `AuthorityError::Unauthorized`.  
Cross-scope ingest without authorization → deny.

## Envelope

Same `AuthorityRequest` / `AuthorityResponse` as Spec 002 (`schema_version`, `request_id`, `vault_id`, `authority_scope_id`, `capability`, `deadline_ms`, `max_response_bytes`, `body`).

## Request / response sketches

### IngestFhirSynthetic

```text
IngestFhirSynthetic {
  media_type: "application/fhir+json",   // required for H0-A JSON path
  bytes: Bytes,                          // exact interchange bytes
  fhir_version_hint: Option<"4.0.1">,
  resource_focus: Option<String>,        // optional logical focus note (non-authority)
  attach_validator_fixture_id: Option<String>,  // offline oracle id
}

→ Ok IngestReceipt {
    receipt_id,
    outcome: Accepted | Duplicate | Rejected | Quarantined,
    source_id?,
    content_digest?,
    byte_length?,
    blob_ref?,
    evaluation_refs: [OpaqueId],
    identity_refs: [OpaqueId],
    error?,
  }

→ Err Unauthorized | WrongScope | LeaseRequired | PayloadTooLarge
    | LexicalReject { reason }
    | VersionReject { admitted: "4.0.1", got? }
    | DurableStoreError
```

**Behavioral rules**:

1. Persist **exact** `bytes` on Accepted (and on Duplicate, prior bytes unchanged).
2. Duplicate: same `authority_scope_id` + same content digest → `Duplicate` without mutation.
3. Duplicate JSON keys / unsafe decimal / non-admitted version → `Rejected` or `Quarantined` per lexical policy (never silent accept).
4. MUST NOT create `ClinicalAssertion` from ingest alone.
5. MAY create `IdentityAssertion` / `Proposal` + `EvaluationRecord` evidence.

### AttachValidatorEvidence

```text
AttachValidatorEvidence {
  source_id: OpaqueId,
  evaluator: String,
  outcome: Pass | Fail | Skipped | Error,
  issue_codes?: [String],
  report_bytes?: Bytes,          // stored as evidence blob or digest-only
}

→ Ok { evaluation_id, report_digest? }
→ Err NotFound | Unauthorized | WrongScope
```

**Rule**: Creates EvaluationRecord with `evidence_only = true` only.

### ReadCanonicalVisibility

```text
ReadCanonicalVisibility { source_id }
→ Ok {
    source_id,
    visible: bool,
    content_digest,
    byte_length,
    blob_verify: Ok | Err Quarantined | Err Missing,
  }
→ Err NotFound | Unauthorized | WrongScope
```

**Rule**: `visible == true` implies digest+size verify would succeed at read time; implement may re-verify on read.

### VerifyBlob

```text
VerifyBlob { blob_ref }
→ Ok { digest, size, state: Live | Tombstoned | Quarantined }
→ Err Missing | DigestMismatch (→ quarantine side effect) | Unauthorized
```

### RebuildProjection

```text
RebuildProjection {
  kind: "SubjectCustodyStub" | "IngestIndexStub" | ...,
  subject_ref?: SubjectRef,
  built_from: [OpaqueId],
}

→ Ok { projection_id, authoritative: false, built_at }
→ Err Unauthorized | WrongScope | MissingInputs
```

**Rule**: Projection never substitutes for SourceRecord or ClinicalAssertion.

## Lexical reject reasons (non-exhaustive)

```text
DuplicateObjectKey
DecimalPrecisionUnsafe
FloatCoercionForbidden
TruncatedJson
DepthLimitExceeded
SizeLimitExceeded
FhirVersionNotAdmitted
MediaTypeUnsupported
EmptyPayloadPolicyReject
```

## Anti-messages (forbidden)

```text
OverwriteSourceBytes { ... }           // still forbidden
IngestRealPhi { ... }                  // unauthorized forever in H0
PromoteValidatorToAssertion { ... }    // does not exist
OpenCanonicalDbFromClient { ... }      // does not exist
IngestViaProductNetwork { ... }        // DEFAULT_DENY
```

## Fixture / media types admitted in H0-A

| Media type | Admitted |
|---|---|
| `application/fhir+json` | yes (R4 4.0.1) |
| `application/json` with FHIR shape | only if explicitly accepted by policy as alias; prefer fhir+json |
| FHIR XML | out of scope for H0-A unless later amendment |
| Images/PDF/audio | no (OCR/ASR later specs) |

## Idempotency

Clients may retry `IngestFhirSynthetic` with identical bytes; server returns `Duplicate` or equivalent deterministic receipt without corrupting prior custody.

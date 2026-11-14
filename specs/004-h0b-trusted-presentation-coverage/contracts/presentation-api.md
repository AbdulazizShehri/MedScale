# Contract: Trusted Presentation API (H0-B)

**Spec**: 004-h0b-trusted-presentation-coverage  
**Status**: Message/API sketch for implement (extends Spec 002 authority facade + Spec 003 RebuildProjection)  
**Scope**: Synthetic-only; LLM-free; promoted ClinicalAssertions + SourceRecords; no FHIRPath engine; no product UI shell

## Roles

Uses Spec 002 roles (`CoreHost`, `Client`, `TransientHostOwner`). Workers do not read canonical store with ambient access; presentation is Core Host / facade only.

## New / extended capabilities

```text
Capability::GetTimeline
Capability::GetBrief
Capability::GetCoverage
Capability::DrillDownPresentation
Capability::RebuildProjection          // extended kinds (see below)
```

Unknown capability → `AuthorityError::Unauthorized`.  
Cross-scope read without authorization → deny.

## Envelope

Same `AuthorityRequest` / `AuthorityResponse` as Spec 002 (`schema_version`, `request_id`, `vault_id`, `authority_scope_id`, `capability`, `deadline_ms`, `max_response_bytes`, `body`).

## Request / response sketches

### GetTimeline

```text
GetTimeline {
  subject_ref: OpaqueId,
  include_proposals: false,   // H0-B: MUST be false or ignored→false
  max_events?: u32,           // optional bound
}

→ Ok {
    subject_ref,
    projection_id?,             // if persisted
    events: [TimelineEvent],
    rules_version: PresentationRulesVersion,
  }

→ Err Unauthorized | WrongScope | NotFound | LeaseRequired
    | UnhealthyEvidence | DurableStoreError
```

**Behavioral rules**:

1. Events from ClinicalAssertions for `subject_ref` only (default).
2. Deterministic order per research D4.
3. Preserve MedicalTime precision; do not invent Instant.
4. MUST NOT emit Proposal-derived clinical events when `include_proposals` is false.

### GetBrief

```text
GetBrief {
  subject_ref: OpaqueId,
}

→ Ok {
    subject_ref,
    projection_id?,
    sections: BriefSections,
    coverage_summary: { status → count },
    rules_version: PresentationRulesVersion,
  }

→ Err Unauthorized | WrongScope | NotFound | UnhealthyEvidence | DurableStoreError
```

**Behavioral rules**:

1. Sections filled only by typed extractors.
2. Missing admitted fields → Absent (or Unknown if unsupported).
3. Conflicts retained in sections/coverage — no silent winner.
4. No LLM / free-text generation capability.

### GetCoverage

```text
GetCoverage {
  subject_ref: OpaqueId,
  concept_keys?: [String],    // optional filter; default = admitted H0-B set
}

→ Ok {
    subject_ref,
    projection_id?,
    slots: [CoverageSlot],
    rules_version: PresentationRulesVersion,
  }

→ Err Unauthorized | WrongScope | NotFound | DurableStoreError
```

### DrillDownPresentation

```text
DrillDownPresentation {
  subject_ref: OpaqueId,
  field_key: String,
  assertion_id?: OpaqueId,
  source_id?: OpaqueId,
}

→ Ok DrillDownResult {
    field_key,
    source_id,
    content_digest,
    media_type,
    span?: TextSpan,
    citation?: WholeResourceCitation,
    excerpt_lossy?: String,
  }

→ Err NotFound | Unauthorized | WrongScope
    | UnhealthyEvidence | QuarantinedBlob | MissingBlob
```

**Behavioral rules**:

1. Prefer raw-byte TextSpan when excerpting source bytes.
2. Quarantined/missing blob → fail closed (no invented excerpt).
3. Path notes in WholeResourceCitation are not executed as FHIRPath.

### RebuildProjection (extended kinds)

```text
RebuildProjection {
  kind: "SubjectTimelineV1" | "SubjectBriefV1" | "SubjectCoverageV1"
      | "SubjectCustodyStub" | "IngestIndexStub" | ...,
  subject_ref?: OpaqueId,     // required for Subject*V1 kinds
  built_from: [OpaqueId],     // may be empty → server selects deterministic input set for subject
}

→ Ok {
    projection_id,
    authoritative: false,
    built_at: MedicalTime,
    body_digest?: DigestSha256,
  }

→ Err Unauthorized | WrongScope | MissingInputs | UnhealthyEvidence
```

**Behavioral rules**:

1. `authoritative` always false.
2. Golden rebuild: same inputs → identical canonical body encoding.
3. Stub kinds from Spec 003 remain supported.

## PresentationRulesVersion (embedded)

```text
PresentationRulesVersion {
  extractors: ["patient.v1", "observation.v1", "condition.v1"],
  ucum_subset: String,          // pin id
  order_rules: "timeline.order.v1",
  brief_schema: "SubjectBriefV1",
}
```

## Anti-messages (forbidden)

```text
GenerateBriefWithLlm { ... }           // does not exist
EvaluateFhirPath { expression, ... }   // does not exist in H0-B
ResolveClinicalConflict { winner }     // does not exist in H0-B
PromoteProjectionToAssertion { ... }   // does not exist
OpenCanonicalDbFromClient { ... }      // does not exist
IncludeRealPhiFixtures { ... }         // unauthorized in H0
```

## Idempotency / determinism

`Get*` may rebuild or read cached Projection; results MUST match golden rebuild for same durable inputs and `PresentationRulesVersion`. Clients may retry safely.

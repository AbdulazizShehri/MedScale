# Contract: Durable Object Classes

**Spec**: 002-trusted-object-source-authority-foundation  
**Status**: Type/message sketches for `medscale-contracts`

## Class tags

Every serialized object carries:

```text
ObjectHeader {
  schema_version: u32,
  class: ObjectClass,
  id: OpaqueId,
  realm_id: OpaqueId,
  authority_scope_id: OpaqueId,
}

ObjectClass =
  | SourceRecord
  | DerivedSourceArtifact
  | Proposal
  | ClinicalAssertion
  | EvaluationRecord
  | ActionAuditRecord
  | Projection
  | IdentityAssertion
  | IdentityMergeDecision
```

Deserializing into the wrong Rust type MUST fail (no untagged enum collapse across authority classes).

## Source vs derived

```text
SourceRecord {
  header,
  bytes: Bytes,
  content_digest: DigestSha256,
  media_type: String,
  acquired_at: Option<MedicalTime>,
}

DerivedSourceArtifact {
  header,
  source_id: OpaqueId,
  transform_id: String,
  transform_version: String,
  loss_class: Lossless | Lossy | Unknown,
  representation: RepresentationKind,
  payload: Bytes | Text,
  content_digest: DigestSha256,
}
```

**Forbidden messages**: `OverwriteSourceBytes { source_id, bytes }` — not part of the contract.

## Proposal vs assertion

```text
Proposal {
  header,
  subject_ref: Option<SubjectRef>,
  claim_kind: String,
  payload: Value,
  confidence: Option<f32>,      // informational
  evidence_refs: Vec<OpaqueId>,
  producer: ProducerKind,
}

ClinicalAssertion {
  header,
  subject_ref: SubjectRef,      // required
  claim_kind: String,
  payload: Value,
  promoted_from_proposal_id: Option<OpaqueId>,
  authorized_by: OpaqueId,
  effective_time: Option<MedicalTime>,
  recorded_time: MedicalTime,
}
```

**Forbidden**: Treating `Proposal` bytes as `ClinicalAssertion` via untagged serde; facade MUST use distinct variants.

## Evaluation / projection / audit

```text
EvaluationRecord {
  header,
  target_refs: Vec<OpaqueId>,
  evaluator: String,
  result: Value,
  evidence_only: true,
}

Projection {
  header,
  projection_kind: String,
  built_from: Vec<OpaqueId>,
  built_at: Instant,
  body: Value,
  authoritative: false,
}

ActionAuditRecord {
  header,
  kind: Audit | ExternalActionIntent,
  actor: OpaqueId,
  action: String,
  target_refs: Vec<OpaqueId>,
  effect_state: Option<EffectState>,  // required if ExternalActionIntent
  payload_digest: Option<DigestSha256>,
  detail: Option<Value>,
}
```

## Identity

```text
IdentityAssertion {
  header,
  subject_id: OpaqueId,
  identifier_system: String,
  identifier_value: String,
  confidence: Option<f32>,
  evidence_refs: Vec<OpaqueId>,
}

IdentityMergeDecision {
  header,
  surviving_subject_id: OpaqueId,
  merged_subject_ids: Vec<OpaqueId>,  // non-empty
  authorized_by: OpaqueId,
  rationale: Option<String>,
}
```

## Text spans

```text
TextSpan {
  representation_id: OpaqueId,
  tagged_representation: RawSourceBytes | DerivedNormalizedText | ...,
  coordinate_system: RawByte | UnicodeScalar | ...,
  start: usize,
  end: usize,
}
```

Conversion API (logical):

```text
convert_span(span, to: CoordinateSystem, view: RepresentationView) -> Result<TextSpan, TextCoordError>
TextCoordError = OutOfBounds | IncompatibleRepresentation | AmbiguousMapping | SchemaMismatch
```

## Effect states

```text
EffectState = Pending | Sent | Confirmed | Failed | Unknown
```

Transition API rejects illegal edges; `Unknown` without `reconcile_token` cannot move to `Sent`/`Pending`.

## Engine placement / FFI / worker (types only)

```text
PlacementClass = P0TrustedCoreSafe | P1IsolatedRequired | P2PlatformConfinedException | P3TrustedNativeInfra

FfiAdmissionRecord { /* checklist fields; see research D10 */ }

WorkerSupervisionPolicy {
  deny_ambient_db: true,
  deny_master_keys: true,
  deny_unrestricted_fs: true,
  deny_network: true,
  deny_secrets: true,
  deny_authority: true,
  grants: Vec<WorkerCapabilityGrant>,  // empty default
}
```

## Compatibility rules

1. Increment `schema_version` on breaking field changes.
2. Authority-bearing unknown fields → reject.
3. Non-authority additive optional fields may be ignored only if explicitly marked `extension_ok` in a later spec; default fail closed in 002 tests.

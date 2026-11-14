# Data Model: Spec 002 Trusted Object / Source / Authority Foundation

**Date**: 2026-08-25  
**Status**: Planning-canonical for Spec 002 implement  
**Persistence**: Logical model only in 002 (in-memory / serde fixtures). Durable storage schemas arrive in Spec 003/005.

## Principles

1. Durable classes remain distinct; no silent coercion.
2. Source bytes ≠ derived representation; source identity ≠ content hash.
3. Proposal ≠ ClinicalAssertion; Projection ≠ truth; AI/worker output ≠ authority.
4. Identity merge is explicit.
5. Realm and opaque `authority_scope_id` are required on authority-bearing objects.
6. Spans carry representation + coordinate system tags.

## Common fields

| Field | Type (logical) | Notes |
|---|---|---|
| `id` | OpaqueId | Vault-scoped unique id (ULID/UUIDv7-style string) |
| `schema_version` | u32 | Envelope/object schema version |
| `realm_id` | OpaqueId | Explicit realm |
| `authority_scope_id` | OpaqueId | Opaque authorization scope; not a leaked org chart |
| `created_at` | Instant | Recorded creation (injectable clock in tests) |
| `audit_refs` | list of AuditRecordId | Optional links |

## Object classes

### SourceRecord

Immutable raw evidence.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Source identity (not content hash) |
| `realm_id` / `authority_scope_id` | yes | Scope |
| `bytes` | yes | Immutable raw bytes (fixture-sized in 002) |
| `content_digest` | yes | SHA-256 of bytes — evidence only |
| `media_type` | yes | e.g. `application/fhir+json`, `text/plain` |
| `acquired_at` | optional | Acquisition time + precision |
| `provenance_note` | optional | Non-authoritative human/system note |

**Invariants**: Bytes never mutate in place; replacement requires new SourceRecord. Digest mismatch on verify → fail closed.

### DerivedSourceArtifact

Versioned transform of a source.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Artifact id |
| `source_id` | yes | Parent SourceRecord |
| `transform_id` | yes | Named transform (e.g. `normalize.nfc.v1`) |
| `transform_version` | yes | Transform version string |
| `loss_class` | yes | `Lossless` \| `Lossy` \| `Unknown` |
| `representation` | yes | e.g. `NormalizedText`, `TokenStream` (extensible enum) |
| `bytes` / `text` | yes | Derived payload |
| `content_digest` | yes | Digest of derived payload |
| `parent_span_map_ref` | optional | Link to mapping metadata for drill-down |

**Invariants**: Cannot replace SourceRecord bytes; must cite `source_id`.

### Proposal

Non-authoritative candidate.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Proposal id |
| `subject_ref` | optional | Explicit subject pointer if known |
| `claim_kind` | yes | Typed claim category (extensible string/enum) |
| `payload` | yes | Structured claim body (JSON-compatible value) |
| `confidence` | optional | Informational only — never authority |
| `evidence_refs` | yes | Source/artifact/evaluation ids |
| `producer` | yes | `ProducerKind` (Human, Rule, WorkerStub, …) — never grants authority |

**Invariants**: Cannot be used where ClinicalAssertion is required.

### ClinicalAssertion

Authorized clinical truth claim.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Assertion id |
| `subject_ref` | yes | Explicit subject |
| `claim_kind` / `payload` | yes | Authorized claim |
| `promoted_from_proposal_id` | optional | Provenance when promoted |
| `authorized_by` | yes | Capability/actor token id (logical) |
| `effective_time` | optional | MedicalTime with precision |
| `recorded_time` | yes | When asserted |

**Invariants**: Created only via authorized promotion or authorized direct-assert path (direct-assert still audited). No auto-create from Proposal confidence.

### EvaluationRecord

Assessment/evidence about quality, coverage, conflict, or validation — not clinical authority.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Evaluation id |
| `target_refs` | yes | Objects evaluated |
| `evaluator` | yes | Rule/tool identity |
| `result` | yes | Structured outcome |
| `evidence_only` | yes | Constant `true` for authority purposes |

### Action / AuditRecord

Durable audit and (stub) external-action intent trail.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Record id |
| `kind` | yes | `Audit` \| `ExternalActionIntent` |
| `actor` | yes | Logical actor |
| `action` | yes | Verb / operation name |
| `target_refs` | optional | Affected objects |
| `effect_state` | conditional | Required for ExternalActionIntent: `PENDING` \| `SENT` \| `CONFIRMED` \| `FAILED` \| `UNKNOWN` |
| `payload_digest` | optional | Binds approval to exact payload (014 will harden) |
| `detail` | optional | Structured detail |

**Invariants**: `UNKNOWN` → retry forbidden without reconciliation marker.

### Projection

Rebuildable non-authoritative view.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Projection id |
| `projection_kind` | yes | e.g. `SubjectIndexStub` |
| `built_from` | yes | Assertion/source ids |
| `built_at` | yes | Instant |
| `body` | yes | Derived structure |
| `authoritative` | yes | Constant `false` |

### IdentityAssertion

Explicit identity statement — not a silent merge.

| Field | Required | Description |
|---|---|---|
| `id` | yes | Identity assertion id |
| `subject_id` | yes | Subject being asserted |
| `identifier_system` | yes | System URI/name |
| `identifier_value` | yes | Value |
| `confidence` | optional | Informational |
| `evidence_refs` | optional | Supporting sources |

### IdentityMergeDecision (explicit merge)

| Field | Required | Description |
|---|---|---|
| `id` | yes | Decision id |
| `surviving_subject_id` | yes | Canonical survivor |
| `merged_subject_ids` | yes | Non-empty list |
| `authorized_by` | yes | Actor/capability |
| `rationale` | optional | Human rationale |

**Invariants**: Without this record, subjects remain distinct even if identifiers “look alike.”

### Realm

| Field | Required | Description |
|---|---|---|
| `id` | yes | Realm id |
| `display_name` | optional | Non-authoritative label |

### authority_scope_id

Opaque id appearing on authority-bearing objects. Spec 002 does not define hierarchical ACL math; it requires the field and cross-scope deny-by-default on facade writes.

## Text model

### CoordinateSystem

`RawByte` | `UnicodeScalar` | (reserved: `Utf16CodeUnit`, `JsUtf16`, `SwiftUtf16`, `KotlinUtf16` — adapters later)

### TextSpan

| Field | Required | Description |
|---|---|---|
| `representation_id` | yes | SourceRecord or DerivedSourceArtifact id |
| `coordinate_system` | yes | CoordinateSystem |
| `start` | yes | Inclusive offset (usize) |
| `end` | yes | Exclusive offset |
| `tagged_representation` | yes | e.g. `RawSourceBytes`, `DerivedNormalizedText` |

**Invariants**: `start <= end`; `end <= len(representation)`; conversions produce new spans, never silently reinterpret offsets.

### MedicalTime

| Field | Required | Description |
|---|---|---|
| `instant` | optional | UTC instant if known |
| `precision` | yes | `Year`…`Instant` \| `Unknown` |
| `approximation` | yes | bool |
| `role` | yes | `Effective` \| `Recorded` \| `Acquired` \| `Other` |

## Process topology entities

### VaultId

Opaque vault identity (string).

### CoreHostLease

| Field | Required | Description |
|---|---|---|
| `vault_id` | yes | Vault |
| `holder_id` | yes | Host instance id |
| `acquired_at` | yes | Instant |
| `expires_at` | optional | Lease expiry if used |

### WorkerSupervisionPolicy (stub)

Default denials: ambient DB, master keys, unrestricted FS, network, secrets, authority. Explicit capability grants list (empty by default).

### FfiAdmissionRecord

Checklist fields per OSS matrix §3; `complete: bool` computed by validator.

## Relationships (summary)

```text
SourceRecord 1──* DerivedSourceArtifact
SourceRecord/Artifact ←── TextSpan
Proposal *──* evidence → Source/Artifact/Evaluation
Proposal ──promote──> ClinicalAssertion (+ AuditRecord)
IdentityAssertion *──> subject
IdentityMergeDecision ──unifies──> subjects
ClinicalAssertion/Source *──> Projection (rebuildable)
ExternalActionIntent ⊂ Action/AuditRecord (effect_state)
All authority-bearing → realm_id + authority_scope_id
```

## State machines

### EffectState

```text
PENDING → SENT → CONFIRMED
              → FAILED
              → UNKNOWN  (requires reconcile before any retry)
CONFIRMED, FAILED = terminal for happy/fail paths
```

Illegal: `UNKNOWN → SENT` without reconcile; `CONFIRMED → PENDING`; etc.

### Promotion

```text
Proposal + AuthorityCapability::PromoteProposal → ClinicalAssertion + AuditRecord
Unauthorized → Deny (typed error) + optional AuditRecord of denial
```

# Data Model: Spec 004 H0-B Trusted Presentation + Coverage

**Date**: 2026-08-25  
**Status**: Planning-canonical for Spec 004 implement  
**Persistence**: Non-authoritative Projections + AuditRecords over Spec 002/003 durable objects. Does not weaken class separation.

## Principles

1. All Spec 002/003 class invariants remain in force.
2. Projection ≠ truth; Brief/Timeline/Coverage bodies are rebuildable views.
3. Default clinical presentation cites ClinicalAssertion + SourceRecord only.
4. Extractors are typed and closed; no FHIRPath AST as authority.
5. Coverage honesty is first-class data, not UI chrome.
6. Synthetic-only; no PHI encryption fields (Spec 005).

## Extensions to Spec 002/003 objects

### ClinicalAssertion (read)

Used as timeline/Brief authority input. `effective_time`, `recorded_time`, `subject_ref`, `claim_kind`, `payload`, evidence via promote/audit chain to SourceRecords.

### SourceRecord (read)

Exact bytes for drill-down; `content_digest`, `blob_ref`, visibility must be healthy for Present citations.

### Projection (write/rebuild)

New kinds:

| projection_kind | Body schema |
|---|---|
| `SubjectTimelineV1` | `{ subject_ref, events: [TimelineEvent], built_rules_version }` |
| `SubjectBriefV1` | `{ subject_ref, sections: BriefSections, coverage_summary, built_rules_version }` |
| `SubjectCoverageV1` | `{ subject_ref, slots: [CoverageSlot], built_rules_version }` |

Always `authoritative = false`. `built_from` lists assertion/source ids used.

### EvaluationRecord

May record extractor/coverage evaluation evidence (`evaluator = "medscale.presentation.coverage.v1"`) — evidence only, never ClinicalAssertion.

### ActionAuditRecord

Rebuild presentation, drill-down (optional audit), conflict detected — audit kinds as needed.

### TextSpan (Spec 002)

Used in DrillDownResult; prefer `RawByte` on SourceRecord representation.

## New Spec 004 entities

### TimelineEvent

| Field | Required | Description |
|---|---|---|
| `event_id` | yes | Stable id within Projection (derived deterministically) |
| `assertion_id` | yes | ClinicalAssertion id |
| `subject_ref` | yes | Subject |
| `claim_kind` | yes | From assertion |
| `summary` | yes | Typed extractor summary fields (structured Value) |
| `effective_time` | optional | Copied MedicalTime |
| `recorded_time` | yes | Copied MedicalTime |
| `sort_key` | yes | Deterministic encoded sort key |
| `evidence_refs` | yes | SourceRecord / related ids |
| `span_citations` | optional | TextSpan or WholeResourceCitation list |

### BriefSections

| Section | Content |
|---|---|
| `identity` | Patient demographics ExtractedFields + statuses |
| `vitals` | Map/list of Observation-derived fields for admitted codes |
| `conditions` | List of Condition-derived fields; conflicts retained |
| `coverage_summary` | Counts / rollup of CoverageStatus |

### ExtractedField

| Field | Required | Description |
|---|---|---|
| `field_key` | yes | Closed key (e.g. `patient.birthDate`, `observation.valueQuantity`) |
| `value` | conditional | Present value as typed JSON |
| `status` | yes | CoverageStatus for this field |
| `evidence_refs` | yes | Durable ids |
| `span_citations` | optional | Drill-down aids |
| `unit` | optional | Raw unit string |
| `unit_semantic` | optional | UnitSemanticResult |

### CoverageStatus

```text
Present | Absent | Unknown | Conflict | IncomparableUnits | UnhealthyEvidence | UnsupportedResourceType
```

### CoverageSlot

| Field | Required | Description |
|---|---|---|
| `concept_key` | yes | Closed concept id (e.g. `brief.identity.birthDate`, `vitals.heart_rate`) |
| `status` | yes | CoverageStatus |
| `values` | yes | 0..n ExtractedField / conflict members |
| `notes` | optional | Machine-readable reason codes |

### ConflictSet

| Field | Required | Description |
|---|---|---|
| `claim_key` | yes | Typed key under which values conflict |
| `members` | yes | ≥2 values with evidence_refs |
| `resolution` | yes | Always `Unresolved` in H0-B |

### WholeResourceCitation

| Field | Required | Description |
|---|---|---|
| `source_id` | yes | SourceRecord |
| `content_digest` | yes | SHA-256 |
| `structural_path_note` | optional | Human/debug path hint — **not** executed FHIRPath |

### DrillDownResult

| Field | Required | Description |
|---|---|---|
| `field_key` | yes | Requested field |
| `source_id` | yes | |
| `content_digest` | yes | |
| `media_type` | yes | |
| `span` | optional | TextSpan |
| `citation` | optional | WholeResourceCitation |
| `excerpt_lossy` | optional | Optional display excerpt marked lossy if derived |

### UnitSemanticResult

| Field | Required | Description |
|---|---|---|
| `raw_unit` | yes | As extracted |
| `admitted` | yes | bool |
| `canonical_unit` | optional | If normalized within subset |
| `comparability` | yes | `Comparable` \| `Incomparable` \| `Unrecognized` |

### PresentationRulesVersion

| Field | Required | Description |
|---|---|---|
| `extractors` | yes | e.g. `patient.v1`, `observation.v1`, `condition.v1` |
| `ucum_subset` | yes | Pin id / digest |
| `order_rules` | yes | Timeline order rule id |
| `brief_schema` | yes | `SubjectBriefV1` |

## Relationships (summary)

```text
ClinicalAssertion ──evidence──> SourceRecord (blob-verified)
        │
        ▼
TypedResourceExtractor ──> ExtractedField + CoverageSlot
        │
        ├──> TimelineProjection (SubjectTimelineV1)
        ├──> BriefProjection (SubjectBriefV1)
        └──> CoverageProjection (SubjectCoverageV1)

ExtractedField ──DrillDown──> TextSpan / WholeResourceCitation → SourceRecord bytes
ConflictSet ⊂ CoverageSlot(status=Conflict)
UnitSemantics(Observation.quantity) → UnitSemanticResult
```

## State machines

### CoverageSlot.status (conceptual)

```text
(unsupported concept/resource) → Unknown | UnsupportedResourceType
(admitted extractor, no value) → Absent
(admitted extractor, one value, healthy evidence) → Present
(admitted extractor, ≥2 disagreeing values) → Conflict
(quantities, units not comparable) → IncomparableUnits
(cited blob quarantined/missing) → UnhealthyEvidence
```

### Projection rebuild

```text
Rebuild(kind, subject, built_from)
  → load assertions/sources (fail closed on unhealthy required evidence)
  → run extractors
  → emit Projection body + AuditRecord
  → authoritative = false always
```

### Conflict

```text
Detect → ConflictSet { resolution: Unresolved }
H0-B has no ResolveConflict clinical authority path
```

# Data Model: Spec 003 H0-A Trusted Ingest + Durability

**Date**: 2026-08-25  
**Status**: Planning-canonical for Spec 003 implement  
**Persistence**: Durable (synthetic vault). Extends Spec 002 logical objects; does not weaken class separation.

## Principles

1. All Spec 002 class invariants remain in force.
2. FHIR R4 JSON is interchange evidence → SourceRecord bytes + optional derived parse artifacts; never the canonical DB.
3. Blob-first: visibility requires digest-verified blob presence.
4. Validator/identity outputs are evidence (EvaluationRecord / IdentityAssertion / Proposal), not silent ClinicalAssertion.
5. Projections remain rebuildable and non-authoritative.
6. Backup/restore is synthetic-scope; encryption fields reserved for Spec 005.

## Extensions to Spec 002 objects

### SourceRecord (durable)

| Added / emphasized field | Required | Description |
|---|---|---|
| `blob_ref` | yes (when durable) | Pointer to exact bytes in BlobStore |
| `content_digest` | yes | SHA-256 of exact bytes |
| `byte_length` | yes | Size for verify |
| `media_type` | yes | e.g. `application/fhir+json` |
| `ingest_receipt_id` | optional | Link to IngestReceipt |
| `visibility` | yes | `Pending` \| `Visible` \| `Quarantined` \| `Absent` |

**Invariants**: Bytes immutable; replace = new SourceRecord; digest mismatch → cannot be `Visible`.

### DerivedSourceArtifact

May hold normalized/parse trees derived from FHIR JSON with transform metadata (`fhir.parse.structural.v1`, etc.). Loss class typically `Lossy` or `Unknown` if round-trip not proven. Never overwrites SourceRecord.

### EvaluationRecord — ValidatorEvidence payload

| Field | Required | Description |
|---|---|---|
| `evaluator` | yes | e.g. `fixture.oracle.hl7-validator-shape.v1` or sidecar id |
| `fhir_version_admitted` | yes | `4.0.1` |
| `outcome` | yes | `Pass` \| `Fail` \| `Skipped` \| `Error` |
| `issue_codes` | optional | Structured issues |
| `report_digest` | optional | Digest of raw validator report bytes |
| `evidence_only` | yes | Always true for authority purposes |

### IdentityAssertion / Proposal from ingest

Emitted with `evidence_refs` → SourceRecord (and optional DerivedSourceArtifact). No auto IdentityMergeDecision.

### Projection

New stub kind allowed: `SubjectCustodyStub` / `IngestIndexStub`. `authoritative = false`. Rebuild via hook replaces body deterministically from `built_from`.

### Action / AuditRecord

Ingest accept/reject/quarantine, GC sweep, backup/restore, migration start/finish produce AuditRecords.

## New Spec 003 entities

### IngestReceipt

| Field | Required | Description |
|---|---|---|
| `id` | yes | Receipt id |
| `vault_id` | yes | Vault |
| `realm_id` / `authority_scope_id` | yes | Scope |
| `outcome` | yes | `Accepted` \| `Duplicate` \| `Rejected` \| `Quarantined` |
| `source_id` | conditional | Present when bytes custodied |
| `content_digest` | conditional | When bytes known |
| `evaluation_refs` | optional | Validator/lexical EvaluationRecords |
| `identity_refs` | optional | IdentityAssertion / Proposal ids |
| `error` | conditional | Typed error on reject |
| `created_at` | yes | Instant |

### BlobRef

| Field | Required | Description |
|---|---|---|
| `digest` | yes | SHA-256 |
| `size` | yes | u64 |
| `locator` | yes | Claim-scoped relative path/key |
| `storage_class` | yes | `FilesystemBlobV1` (extensible) |

### BlobRecord

| Field | Required | Description |
|---|---|---|
| `digest` | yes | Primary integrity key for blob body |
| `size` | yes | Expected size |
| `state` | yes | `Live` \| `Tombstoned` \| `Quarantined` |
| `ref_count_hint` | optional | Non-authoritative hint; GC uses mark phase |
| `tombstoned_at` | conditional | When tombstoned |
| `quarantine_reason` | conditional | Corruption/evidence note |

### CanonicalVisibilityRecord

| Field | Required | Description |
|---|---|---|
| `object_id` | yes | Usually SourceRecord id |
| `blob_ref` | yes | Required blob |
| `visible` | yes | bool — true only if verify would succeed at publish time |
| `published_at` | conditional | When made visible |

### GcMarkState

| Field | Required | Description |
|---|---|---|
| `epoch` | yes | Mark generation |
| `marked_digests` | yes | Set of live digests |
| `phase` | yes | `Idle` \| `Marking` \| `Tombstoning` \| `Sweeping` |

### MigrationJournal

| Field | Required | Description |
|---|---|---|
| `from_version` | yes | u32 |
| `to_version` | yes | u32 |
| `state` | yes | `Started` \| `Finished` \| `Failed` |
| `step` | yes | Idempotent step id |
| `updated_at` | yes | Instant |

### BackupManifest

| Field | Required | Description |
|---|---|---|
| `schema_version` | yes | Backup format version |
| `vault_id` | yes | Source vault |
| `created_at` | yes | Instant |
| `metadata_snapshot_digest` | yes | Digest of metadata snapshot blob |
| `blob_entries` | yes | List of `{ digest, size }` |
| `object_index_digest` | optional | Digest of object id index |
| `claim_scope_note` | yes | Human/machine note: synthetic-scope, unencrypted |

### BackupArtifact

Logical package: `BackupManifest` + metadata snapshot bytes + blob payload set.

### FilesystemClaimScope

| Field | Required | Description |
|---|---|---|
| `os` | yes | e.g. `windows`, `linux` |
| `filesystem_class` | yes | e.g. `local-ntfs`, `local-ext4` |
| `vault_root_policy` | yes | `LocalAppDataLike` \| `TestTemp` |
| `remote_sync_roots` | yes | `Refuse` \| `NonClaim` |

### DurableStore schema (logical tables — not FHIR)

Illustrative metadata relations (SQLite candidate):

```text
objects(id, class, realm_id, authority_scope_id, json_envelope, ...)
blobs(digest PK, size, state, locator, ...)
visibility(object_id, digest, visible, ...)
ingest_receipts(...)
gc_state(...)
migration_journal(...)
audit(...)
```

Exact DDL is implement-time; must support single-writer and migration journal.

## Relationships (summary)

```text
IngestFhir
  → lexical EvaluationRecord? (fail → Rejected)
  → SourceRecord + BlobRecord(Live) + CanonicalVisibility
  → Validator EvaluationRecord (evidence only)
  → IdentityAssertion / Proposal (evidence-linked)
  → IngestReceipt
  → AuditRecord

BlobRecord ←── CanonicalVisibility / SourceRecord.blob_ref
GC mark ──tombstone──> unused blobs ──sweep──> delete file
Projection ← RebuildProjection(built_from Source/Assertion ids)
BackupArtifact ← BackupVault(metadata + blobs)
RestoreVault → restore closure (every visible ref has blob+digest)
```

## State machines

### BlobRecord.state

```text
(put success) → Live
Live → Quarantined   (verify fail / corruption)
Live → Tombstoned    (unreferenced after mark)
Tombstoned → (swept / deleted)
Quarantined ↛ Visible custody without re-ingest/repair path
```

### Ingest outcome

```text
Submit → LexicalGate → (Reject)
       → BlobPut+Verify → MetadataPublish → Accepted
       → DigestExistsInScope → Duplicate (no byte mutate)
       → VerifyFail → Quarantined
```

### GC

```text
Idle → Marking → Tombstoning → Sweeping → Idle
Crash in any phase → reopen resumes or restarts mark safely
Promotion during Marking → blob must remain Live / re-marked
```

### Migration

```text
Started(step=k) → Finished | Failed
Reopen: if Started → resume idempotent steps from k; never dual-serve
```

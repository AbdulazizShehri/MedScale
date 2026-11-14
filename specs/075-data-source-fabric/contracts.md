# Contracts — Spec 075 Data Source Fabric + Data Workbench Foundation

**Status:** `CONTRACT_FREEZE_INPUT`
**Rule:** Muse must reconcile these semantics against the exact live Rust types during T075-01, make only compatibility-minimizing adjustments, then mark the resulting field layout `FROZEN_FOR_075` before storage implementation begins.

This file closes semantic ambiguity. It does not authorize creation of a parallel ID, provenance, audit, privacy or authorization system.

## 1. Existing primitives are authoritative

Reuse the current MedScale primitives wherever their semantics fit:

```text
OpaqueId
ObjectHeader
DigestSha256
RealmId
AuthorityScopeId
existing audit/evidence/provenance contracts
existing serialization/error conventions
existing Network Broker request/decision contracts
```

Do not introduce `DataSourceId`, `SnapshotId`, `ViewId`, `TransformId` wrappers unless live repository conventions clearly require newtype safety and the active spec records why `OpaqueId` alone is insufficient. If newtypes are used, they must wrap/reuse `OpaqueId`; they must not create a second identity namespace.

System/audit timestamps must use the repository's existing system-time/audit convention. Do **not** reuse `MedicalTime` for repository metadata merely because it exists; `MedicalTime` retains its clinical/time-precision semantics. Source-observed time (a data value inside a dataset) is distinct from snapshot-ingestion time (repository metadata).

## 2. Revision model

Mutable 075 state (source manifests, saved views) requires one explicit monotonic concurrency token.

Default semantic type:

```text
Revision = u64
```

Rules:

- create starts at revision `1` unless current repository conventions dictate another deterministic initial value;
- every successful mutation increments by exactly one;
- mutation of existing state supplies `expected_revision`;
- mismatch returns `Conflict` and does not write;
- revision is concurrency state, not content identity and not `DigestSha256`;
- immutable rows (snapshots, snapshot parts, receipts) never carry a mutable revision; their identity is content/exact-source bound and they are never updated in place.

If the live repository already has a canonical revision/precondition contract (Spec 074 defines `ProjectRevision` for project-graph rows), reuse the same pattern and document the mapping rather than creating `Revision` anew.

## 3. DataSourceKind

`DataSourceKind` identifies the acquisition mechanism. It is not an alternate `ObjectClass` and must not flatten existing types.

Initial bounded vocabulary (T075-01 may narrow, never silently widen):

```text
LocalTabularFile
LocalDirectoryWatch (only if frozen contract admits refresh semantics; otherwise deferred)
DatabaseRead
RemoteDataset (Hugging Face / Kaggle qualified adapters)
```

Unknown authority-bearing kinds fail closed. UI may display an unsupported source without silently treating it as a known kind.

Local file formats are a separate `LocalFileFormat` enum, not kinds:

```text
Csv
Tsv
JsonLines
Json
Parquet
ArrowIpc
Xlsx
```

T075-01 freezes which formats the foundation admits first (CSV/TSV mandatory first slice). Unsupported formats return explicit `Unsupported`, never best-effort misparses.

## 4. DataSourceCapability and SourceHealth

```text
DataSourceCapability =
    DiscoverSchema
  | PreviewRows
  | ImportSnapshot
  | RefreshSnapshot
  | BoundedQuery (database adapters)
```

Capabilities are declared per source instance from its kind and qualification state, never inferred from display strings. A source that cannot perform an operation reports the operation as denied/unavailable/unsupported rather than failing opaquely.

```text
SourceHealth =
    Healthy
  | Stale
  | Unavailable
  | Denied
  | SchemaChanged
  | CredentialRevoked
```

Health is observed state, recomputed on access/refresh, and reported explicitly. Health transitions never rewrite existing snapshots.

## 5. DataSourceManifest

Semantic contract:

```text
DataSourceManifest {
    header: ObjectHeader,
    revision: Revision,
    project_id: OpaqueId,
    kind: DataSourceKind,
    display_name: String,
    format_or_engine: String,   // LocalFileFormat or qualified engine identity
    locator: SourceLocator,     // typed locator, never a raw ambient path handle
    credential_ref: Option<OpaqueCredentialRef>,
    capabilities: Vec<DataSourceCapability>,
    health: SourceHealth,
    status: SourceStatus,
}

SourceStatus = Active | Archived
```

Invariants:

- `display_name` is non-empty after trimming; field byte/character limits are explicit constants and tested;
- Project must exist in the same admitted realm/authority scope; Project archive policy is checked before source mutation;
- `locator` for local files is a vault-scoped/claim-scoped reference validated at creation, not an arbitrary absolute path string that escapes the admitted filesystem boundary;
- `credential_ref` is an opaque reference resolved only through the admitted keystore/secret path; credential plaintext never appears in the manifest, receipts, logs, or backups;
- no credential, file bytes, or row payload are stored in the manifest;
- archive retains snapshots/views/lineage for inspection/recovery; archive never cascades to referenced canonical artifacts or deletes snapshots.

## 6. SourceSchema

```text
SourceSchema {
    source_id: OpaqueId,
    schema_fingerprint: DigestSha256,   // over canonical field descriptors
    fields: Vec<SchemaField>,
    source_revision: SourceRevisionBinding,
}

SchemaField {
    name: String,
    field_type: FieldType,
    nullable: bool,
    declared_unit: Option<String>,      // preserved verbatim; never assumed
}

FieldType =
    Text | Integer | Float | Boolean
  | Date | Time | DateTime
  | Binary (opaque; no content interpretation in 075)
```

Invariants:

- schema discovery is deterministic for identical bytes: same source revision yields the same fingerprint;
- unknown/ambiguous source types map to explicit `Unsupported` or `Text`-with-warning according to the frozen policy, never silent coercion;
- missing values are preserved as missing (`nullable`), never coerced to zero/empty;
- schema change between refreshes yields `SourceHealth::SchemaChanged` and requires explicit user/Core acknowledgment before a new snapshot materializes under the changed schema; the old schema and snapshots remain intact.

## 7. DataSnapshot and SnapshotPart

Semantic contract:

```text
DataSnapshot {
    header: ObjectHeader,          // id is OpaqueId; identity is source+revision bound
    source_id: OpaqueId,
    parent_snapshot_id: Option<OpaqueId>,  // set only for transformation outputs
    source_revision: SourceRevisionBinding,
    schema_fingerprint: DigestSha256,
    row_count: u64,
    content_digest: DigestSha256,  // over canonical snapshot bytes
    lineage: SnapshotLineage,
    status: SnapshotStatus,
}

SnapshotStatus = Complete | Partial

SnapshotLineage {
    acquisition: ImportReceipt | RefreshReceipt | TransformationReceipt,
    engine_versions: Vec<String>,   // parser/adapter/tool versions, exact
    parameters_digest: DigestSha256,
}
```

`SourceRevisionBinding` is the strongest exact-source identity the adapter actually supports: file digest + length (+ mtime only as a change hint, never as identity); database query digest + observed schema fingerprint + source-reported revision where available; remote dataset exact repository/revision/files + file digests. A binding cannot claim immutability the source does not provide; database and remote sources always carry explicit staleness semantics.

Invariants:

- snapshots are immutable: no update API exists; a changed source produces a new snapshot, never a mutated old one;
- `Partial` snapshots record exactly which parts/rows are present and why the remainder is absent; partial data is never presented as complete;
- row order is deterministic and recorded (source order or explicit sort key); pagination/virtualization is stable across reopen for the same snapshot;
- large snapshot bytes live in the existing blob/sealed-blob stores under `content_digest` identity; metadata rows never embed bulk bytes;
- snapshot deletion/GC, if any, is explicit, never inferred from view detachment or Project detachment.

`SnapshotPart` partitions bulk bytes for bounded memory/paging:

```text
SnapshotPart {
    snapshot_id: OpaqueId,
    part_index: u32,
    row_range: (u64, u64),
    part_digest: DigestSha256,
    blob_ref: <existing blob reference type>,
}
```

Parts are an internal storage/windowing mechanism; Core query semantics always resolve through the owning snapshot identity.

## 8. ImportReceipt and RefreshReceipt

```text
ImportReceipt {
    source_id: OpaqueId,
    snapshot_id: OpaqueId,
    source_revision: SourceRevisionBinding,
    schema_fingerprint: DigestSha256,
    rows_materialized: u64,
    bytes_hashed: u64,
    outcome: AcquireOutcome,
    warnings: Vec<String>,   // bounded; e.g. encoding repairs, skipped rows with counts
}

AcquireOutcome = Complete | Partial | Quarantined | Rejected

RefreshReceipt {
    source_id: OpaqueId,
    previous_snapshot_id: OpaqueId,
    new_snapshot_id: Option<OpaqueId>,  // None when source is unchanged
    change_class: RefreshChangeClass,
    outcome: AcquireOutcome,
}

RefreshChangeClass =
    Unchanged | ContentChanged | SchemaChanged
  | SourceUnavailable | SourceDenied | SourceGone
```

Rules:

- receipts are immutable and audit-bound;
- skipped/repaired rows are counted and reported, never silently dropped;
- a refresh that finds the source unchanged creates a receipt but no new snapshot (idempotent refresh);
- quarantine keeps hostile/malformed bytes out of snapshot storage; quarantine location and reason are recorded.

## 9. SavedDataView

```text
SavedDataView {
    header_or_id: existing durable identity form,
    snapshot_id: OpaqueId,
    revision: Revision,
    view_kind: DataViewKind,
    state: ViewState,          // validated grid state / view parameters
    status: Active | Archived,
}

DataViewKind =
    Grid | Form | Gallery | Kanban | Calendar | Summary
```

Invariants:

- views reference a snapshot by identity; they store presentation parameters, never copied row payloads;
- view state referencing columns/fields validates against the snapshot schema fingerprint at load; schema drift yields explicit stale/invalid view state, never misaligned columns;
- alternate views (Form/Gallery/Kanban/Calendar/Summary) are projections: same snapshot, same row identities, same missingness;
- Kanban/Calendar groupings and Summary aggregates compute from snapshot values deterministically; aggregate definitions are part of frozen view state, not ambient UI memory;
- summary/aggregate values never substitute for source values in exports/lineage; they are labeled derived.

## 10. DataTransformation and TransformationReceipt

Initial bounded operation vocabulary (T075-01 may narrow; widening requires contract amendment):

```text
TransformOp =
    SelectColumns | DropColumns | RenameColumn
  | CastType { strict } | FilterRows | SortRows
  | ComputedExpression (only if frozen contract admits a safe deterministic subset)
  | BoundedJoin (only if frozen contract admits it with explicit key/cardinality rules)
```

```text
DataTransformation {
    input_snapshot_ids: Vec<OpaqueId>,
    ops: Vec<TransformOp>,
    parameters_digest: DigestSha256,
}

TransformationReceipt {
    input_snapshot_ids: Vec<OpaqueId>,
    output_snapshot_id: OpaqueId,
    ops_digest: DigestSha256,
    rows_in: Vec<u64>,
    rows_out: u64,
    cast_failures: u64,        // explicit; failed casts never silently coerce
}
```

Rules:

- transformation execution is deterministic: same inputs + ops yield byte-identical output snapshots;
- durable output is a new immutable `DataSnapshot` with `parent_snapshot_id` set and lineage bound;
- failed casts, failed joins, and empty results are explicit outcomes, not silent data loss;
- no arbitrary Python/JavaScript/shell/R execution path exists anywhere in the transform pipeline;
- computed expressions, if admitted, are side-effect-free, total on their declared domain, and versioned as part of `ops_digest`.

## 11. Dataset release foundation (conditional)

Only if retained at contract freeze:

```text
DatasetCard {
    snapshot_id: OpaqueId,
    version: String,                 // explicit, monotonic per dataset line
    splits/groups: SplitMetadata,    // leakage-relevant grouping recorded, not enforced here
    annotation_schema_ref: Option<AnnotationSchemaRef>,
    rights_state: RightsState,
    project_id: OpaqueId,
}

ReleaseManifest {
    card_digest: DigestSha256,
    snapshot_digest: DigestSha256,   // must equal the snapshot content_digest
    immutable: true,                 // releases are never edited; new version required
}
```

Full annotation/review/adjudication workflow is out of scope; 075 records schema identity/version only. If T075-01 removes this section, `contracts.md` must record the removal with rationale and the promotion acceptance criterion for releases is satisfied by that explicit removal.

## 12. Resolution and row states

Query APIs surface typed row/snapshot resolution states reusing the repository error framework:

```text
Current | Stale | Missing | UnsupportedKind | Denied | Corrupt | Partial | Unavailable | Cancelled | Conflict
```

`Denied` must not reveal sensitive source metadata (no locator/credential hints beyond the permitted boundary). Missing/unavailable source content renders as missing/stale, never as fabricated rows.

## 13. Error contract

Use the current repository error/result framework. Semantic distinctions required by 075:

```text
Invalid
NotFound
Denied
Conflict
Stale
Partial
Corrupt
Unsupported
Unavailable
Cancelled
Internal
```

The exact Rust enum naming must follow existing conventions (`AuthorityError` extended additively if it is the canonical type). These states must not be collapsed where they affect user action or safety. Database timeouts map to `Unavailable` or `Cancelled` according to whether cancellation was requested; disk-full maps to an explicit failure, never a partial success.

## 14. Serialization/versioning

- follow current serde naming/`deny_unknown_fields` policy for authority-bearing contracts;
- every durable schema has explicit schema version ownership (storage schema v4 for 075 rows);
- unknown future authority values fail closed;
- old supported versions migrate or return explicit unsupported-schema state;
- JSON CLI output is versionable and tested;
- snapshot bytes use a canonical deterministic encoding so `content_digest` is stable across processes and reopen.

## 15. Contract freeze gate

Before T075-02 starts, Muse must update this file with:

```text
CONTRACT_FREEZE = FROZEN_FOR_075
LIVE_BASE_SHA = <verified canonical ancestor>
CONTRACT_FILES = <exact Rust paths>
REVISION_MODEL = <exact reused/new type>
KIND_VOCABULARY = <final bounded DataSourceKind set>
FORMAT_SET = <admitted LocalFileFormat set for foundation>
TRANSFORM_OPS = <final bounded op set>
VIEW_KINDS = <final admitted set>
RELEASE_RETAINED = <YES with exact types | NO with rationale>
ERROR_TYPE = <exact Rust type/path>
```

Any later change to those frozen items requires an explicit reason, test impact review, and migration impact review before code proceeds.

## 16. Freeze record (T075-01, FROZEN_FOR_075)

```text
CONTRACT_FREEZE = FROZEN_FOR_075
LIVE_BASE_SHA = ae0441918296c2d1510a71061249c7e55e65d760
CONTRACT_FILES = crates/medscale-contracts/src/data_sources.rs,
                 crates/medscale-contracts/src/lib.rs,
                 crates/medscale-contracts/src/envelopes/mod.rs,
                 crates/medscale-contracts/src/network/mod.rs (DatasetMirrorRead purpose only)
REVISION_MODEL = medscale_contracts::project_graph::ProjectRevision (u64 alias)
                 + check_revision/initial_revision; no new revision type.
KIND_VOCABULARY = local_tabular_file, database_read, remote_dataset
FORMAT_SET = csv, tsv, json_lines, json (qualified foundation);
             parquet, arrow_ipc, xlsx are vocabulary-only and report
             Unsupported until separately qualified slices pass.
ENGINE_SET = external_sqlite (qualified); postgres, mysql, sql_server report
             Unsupported until separately qualified slices pass.
PROVIDER_SET = hugging_face, kaggle (brokered, fixture-qualified foundation;
               live hosts remain ExternalGateRequired like Spec 013/015).
TRANSFORM_OPS = select, drop, rename, cast (strict/non-strict), filter, sort.
                No computed expressions, no join in the frozen set.
VIEW_KINDS = grid, form, gallery, kanban, calendar, summary (all projections).
RELEASE_RETAINED = YES: DatasetCard + ReleaseManifest (schema identity only;
                   full annotation workflow stays out).
ERROR_TYPE = medscale_contracts::envelopes::AuthorityError, extended additively
             with Cancelled { message }; Invalid->InvalidArgument,
             NotFound->NotFound, Denied->Unauthorized|SessionDenied|WrongScope,
             Stale->StaleReference, Unsupported->UnsupportedSchema,
             Unavailable->Unavailable, Cancelled->Cancelled.
CREDENTIAL_RULE = opaque OpaqueId reference only; database credential refs are
                  rejected in 075 (no password path); plaintext never persists.
```

Sequencing note: typed Core request/response envelope bodies and their
`capability_matches` pairs land in 075-C with the authority paths so the
facade dispatch match stays exhaustive. The 14 new `Capability` variants, the
18 new `RequestBody` variants, the 14 new `ResponseBody` variants, and the 1
new `AuthorityError` variant above are the frozen vocabulary 075-C must use;
075-C may not invent parallel capabilities or a second error taxonomy.
`SnapshotRefresh` carries `allow_schema_change` as the explicit schema-change
acknowledgment. No `DataSourceId`/`SnapshotId` newtypes: identity is
`header.id: OpaqueId`. No durable timestamps in 075 contracts: ordering
travels through the existing audit trail; `MedicalTime` keeps clinical
semantics. Credential plaintext never enters manifests, receipts, snapshots,
logs, backups, or error strings.

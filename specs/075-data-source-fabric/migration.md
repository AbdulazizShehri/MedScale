# Migration and Recovery — Spec 075

**Status:** implementation contract

Spec 075 extends the existing encrypted MedScale vault. It does not create a second database, replace current canonical object storage, or rewrite pre-075 object identities.

## 1. Default migration posture

The migration is additive:

```text
pre-075 encrypted vault (storage schema v3)
  + data-source manifest structures
  + snapshot metadata structures
  + snapshot-part structures
  + import/refresh/transformation receipt + lineage structures
  + saved-view structures
  + transformation definition structures
```

Existing canonical objects (patients, FHIR, sources, documents, evidence, models, Packs, Projects, Experiments, graph edges) remain exactly where their current owners store them.

No existing object is automatically assigned to a data source by migration. The default post-migration state is: old workflows remain valid and Data Source Fabric structures are opt-in organization.

## 2. Required schema families

The implementation may adapt exact table names to current storage conventions, but semantic ownership is fixed:

```text
data_sources
data_snapshots
data_snapshot_parts
data_receipts
data_saved_views
data_transformations
```

For each family, T075-02 must document exact columns/types/indexes/constraints in this file before closure.

Every durable mutable row needs:

- stable opaque identity;
- realm/authority scope or an unambiguous existing path to it;
- schema version;
- monotonic mutation revision/precondition value;
- status/tombstone state;
- bounded metadata only;
- enough owner/reference information to validate Project scope without duplicating target payload.

Immutable rows (snapshots, parts, receipts) need stable identity, version, digests, and lineage bindings instead of mutable revisions.

Snapshot bulk bytes must reuse the existing blob/sealed-blob stores under content-digest identity. No new blob lifecycle, no parallel `research_os_blob_store`, no bulk bytes embedded in metadata rows.

## 3. Required indexes

At minimum, indexes must efficiently support:

- source list by Project and active/archive state;
- snapshot list by source, newest-first;
- snapshot lookup by exact identity and by content digest;
- parts lookup by snapshot in deterministic part order;
- receipts lookup by snapshot and by source;
- saved-view list by snapshot;
- transformation lookup by output snapshot and by input snapshot;
- exact source-manifest identity lookup for idempotent create/refresh.

Do not add speculative indexes without a query/measurement justification.

## 4. Referential behavior

Data-source tables may use internal foreign keys between 075-owned rows when compatible with current storage design.

They must **not** use cascade behavior that can delete or mutate current canonical patient/FHIR/source/document/evidence/model/Pack/Project data.

A snapshot validates its source through Core at creation. Long-lived source disappearance is represented as missing/stale health state rather than repaired by silent reassignment. Detaching a source from a Project (if admitted) removes only the association; snapshots, receipts, and lineage remain inspectable.

## 5. Transaction boundaries

Required atomic units:

### Source manifest create or update

One transaction contains all rows needed to establish one valid durable entity and its revision/audit metadata.

### Snapshot materialization

Validation/quarantine decision, snapshot metadata row, snapshot-part rows, blob writes, and the acquisition receipt commit atomically from the reader's perspective: queries never observe a snapshot identity whose parts or receipt are absent. Blob content addressing (digest-verified on read) is the backstop against half-written bytes.

### Refresh

The refresh receipt plus any new snapshot/parts commit atomically; the previous snapshot row is untouched.

### Saved-view create or update

Validation against the snapshot schema fingerprint occurs before write. The view mutation and 075-owned index/audit metadata commit atomically.

### Transformation materialize

Input snapshot resolution, output snapshot/parts/receipt rows commit atomically. A failed transform writes no output snapshot; the failure receipt (if recorded) is explicit.

### Multi-operation UI workflow

UI convenience must not turn several logically independent commands into an undocumented giant transaction. If the product needs atomic multi-command behavior, Core must expose an explicit typed operation and tests.

## 6. Crash points

Tests must exercise or deterministically simulate at least:

1. before migration begins;
2. after migration metadata indicates start but before commit, if the framework exposes such a state;
3. after migration commit;
4. before source-manifest insert transaction commits;
5. during snapshot byte streaming (kill mid-part);
6. after snapshot commit before UI receives response;
7. before saved-view/transformation commit;
8. after commit before client acknowledgement;
9. during backup/restore workflow used for rollback/recovery;
10. during database-adapter read with lost connection;
11. during remote dataset download with partial bytes.

Required outcome: reopening the vault produces either the complete committed old state or the complete committed new state, never half-authority. Partial downloads/snapshots are invisible or explicitly marked corrupt/partial, never presented as complete.

## 7. Pre-075 migration fixtures

Use synthetic/permitted fixtures representing at least:

- empty encrypted vault;
- populated vault with current patient/FHIR/source/evidence/document/model metadata;
- populated 074 vault with Projects/Experiments/refs/edges;
- vault with current archived/amended data where relevant;
- backup/restore fixture already supported by current repository;
- a fixture close enough to current production-shaped schema to catch index/table migration regressions;
- representative tabular fixtures: small CSV, CSV with encoding edge cases, malformed CSV, large CSV for scale.

Do not introduce real PHI fixtures. Do not introduce licensed dataset content.

## 8. Migration qualification sequence

For each representative fixture:

```text
1. open with pre-075-compatible base behavior
2. record stable object IDs/digests/authority facts required for comparison
3. create verified backup/recovery checkpoint
4. apply 075 migration once
5. inspect schema/version metadata (expect v4)
6. execute pre-075 regression operations
7. execute 075 source/snapshot/view/transform operations
8. close process cleanly
9. reopen
10. verify exact source/snapshot/view identities/digests plus old-object identities/revisions/digests as applicable
11. attempt normal open/migration again; it must be safe according to migration framework
12. restore the pre-migration backup in a separate recovery path and prove old state remains usable
```

## 9. Rollback rule

Default rollback is **restore from the verified pre-migration backup**.

Do not implement a destructive SQL down-migration merely to claim rollback. A down-migration is allowed only if tests prove it can preserve all pre-existing and 075-created state required by the declared rollback contract.

If 075-created state cannot be represented by the old schema, restoration of the pre-migration checkpoint necessarily discards post-checkpoint 075 changes; this must be stated honestly in recovery UX/evidence.

## 10. Schema version ownership

T075-02 must extend the exact current storage schema/migration mechanism (live value: v3) with the smallest v3 → v4 transition compatible with it.

Do not invent a parallel `research_os_schema_version` if the vault already has canonical migration/version metadata.

Unsupported future schema versions fail closed rather than being opened with partial interpretation.

## 11. Backup behavior

075-owned metadata is included in the existing vault backup/recovery mechanism. Large referenced canonical artifacts and external database/remote content remain governed by their existing rules; 075 must not duplicate external source content inside backup records beyond the snapshots it already owns.

A restored source must either resolve its snapshots/lineage correctly or show explicit missing/stale states. Restored receipts must still verify against restored snapshot digests.

## 12. Archive and deletion

075 supports archive/tombstone/detach, not destructive data-source erasure as a product feature.

- Source archive retains snapshots/receipts/views/lineage for inspection/recovery.
- Snapshot rows are never edited; retention/GC policy, if any, is explicit and evidence-bound.
- view detach/archive removes only the presentation state.
- canonical target object deletion remains owned by the target's existing system.
- GC must not infer that a canonical artifact or snapshot is unneeded merely because its last view reference was removed.

## 13. Concurrency

Preserve current writer-lock/transaction model. Spec 075 does not introduce multi-device concurrent writers.

Within the admitted local writer model, every mutation of existing 075-owned mutable state uses expected revision/precondition semantics. Stale request -> `Conflict`, no last-write-wins.

Long-running acquisitions (database reads, remote downloads) are cancellable; cancellation leaves no half-visible snapshot and records an explicit cancelled outcome.

## 14. Migration evidence

Closure evidence must record:

```text
base SHA
candidate SHA
storage schema version before/after (expect 3 -> 4)
fixture identity/digest
migration command/test
backup checkpoint evidence
pre-075 regression results
new 075 workflow results
close/reopen results
repeat-open/migration results
restore results
known limitations
```

A migration test that only creates a fresh empty database is insufficient.

## 15. T075-02 freeze block

Before T075-02 is declared complete, update:

```text
MIGRATION_CONTRACT = FROZEN_FOR_075
CURRENT_STORAGE_VERSION = <live value, expect 3>
075_STORAGE_VERSION = <new value, expect 4>
MIGRATION_CODE_PATH = <exact path>
BACKUP_CODE_PATH = <exact path>
SOURCE_TABLES = <exact names>
INDEXES = <exact names/queries>
SNAPSHOT_BYTE_STORE = <existing blob/sealed-blob reuse statement>
ROLLBACK_METHOD = <verified method>
```

## 16. Freeze record (T075-02, FROZEN_FOR_075)

```text
MIGRATION_CONTRACT = FROZEN_FOR_075
CURRENT_STORAGE_VERSION = 3
075_STORAGE_VERSION = 4
MIGRATION_CODE_PATH = crates/medscale-storage/src/sqlite_meta.rs (migrate),
                      crates/medscale-storage/src/data_sources.rs (V4_DDL + rows)
BACKUP_CODE_PATH = crates/medscale-storage/src/backup.rs (restore_v4 / restore_v3),
                   crates/medscale-storage/src/encrypted_vault.rs (sealed backup/restore)
SOURCE_TABLES = data_sources, data_snapshots, data_snapshot_parts,
                data_receipts, data_saved_views, data_transformations,
                dataset_releases
INDEXES = idx_data_sources_project, idx_data_sources_scope_status,
          idx_snapshots_source,
          idx_snapshots_source_digest (unique backstop),
          idx_saved_views_snapshot,
          idx_transformations_first_input,
          idx_releases_project, idx_releases_snapshot
SNAPSHOT_BYTE_STORE = existing FsBlobStore (synthetic vaults) and
                      SealedBlobStore via EncryptedVault::put_blob/get_blob
                      (encrypted vaults), keyed by content_digest. No new blob
                      lifecycle. Parts are row-range metadata with per-part
                      digests over canonical encodings; bytes are not duplicated.
ROLLBACK_METHOD = restore verified pre-migration backup (no destructive
                  down-migration; interrupted journal fails closed with
                  MigrationIncomplete)
```

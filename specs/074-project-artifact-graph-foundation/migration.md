# Migration and Recovery — Spec 074

**Status:** implementation contract

Spec 074 extends the existing encrypted MedScale vault. It does not create a second database, replace current canonical object storage, or rewrite pre-074 object identities.

## 1. Default migration posture

The migration is additive:

```text
pre-074 encrypted vault
  + project metadata structures
  + experiment metadata structures
  + project artifact-reference structures
  + typed project-graph edge structures
```

Existing canonical objects remain exactly where their current owner stores them.

No existing object is automatically assigned to a Project by migration. The default post-migration state is: old workflows remain valid and Projects are opt-in organization.

## 2. Required schema families

The implementation may adapt exact table names to current storage conventions, but semantic ownership is fixed:

```text
projects
experiments
project_artifact_refs
project_graph_edges
```

For each family, T074-02 must document exact columns/types/indexes/constraints in this file before closure.

Every durable mutable row needs:

- stable opaque identity;
- realm/authority scope or an unambiguous existing path to it;
- schema version;
- monotonic mutation revision/precondition value;
- status/tombstone state;
- bounded metadata only;
- enough owner/reference information to validate Project scope without duplicating target payload.

## 3. Required indexes

At minimum, indexes must efficiently support:

- Project list by active/archive state and permitted scope;
- Experiment list by Project;
- artifact references by Project and optional Experiment;
- lookup of an active duplicate reference tuple;
- graph neighbors by `project_id + subject`;
- graph neighbors by `project_id + object` where reverse traversal is productized;
- graph filtering by predicate;
- exact edge identity lookup.

Do not add speculative indexes without a query/measurement justification.

## 4. Referential behavior

Project tables may use internal foreign keys between Project-owned rows when compatible with current storage design.

They must **not** use cascade behavior that can delete or mutate current canonical patient/FHIR/source/document/evidence/model/Pack data.

A project reference validates a target through Core at creation. Long-lived target disappearance is represented as missing/stale resolution state rather than repaired by silent reassignment.

## 5. Transaction boundaries

Required atomic units:

### Project/Experiment create or update

One transaction contains all rows needed to establish one valid durable entity and its revision/audit metadata.

### Artifact attach/detach

Attachment row plus any Project-owned index/relationship bookkeeping is one transaction. The canonical target artifact is never modified by this transaction merely because it is attached.

### Graph edge create/remove

Validation occurs before write. The edge mutation and Project-owned indexes/audit metadata commit atomically.

### Multi-operation UI workflow

UI convenience must not turn several logically independent commands into an undocumented giant transaction. If the product needs atomic multi-command behavior, Core must expose an explicit typed operation and tests.

## 6. Crash points

Tests must exercise or deterministically simulate at least:

1. before migration begins;
2. after migration metadata indicates start but before commit, if the framework exposes such a state;
3. after migration commit;
4. before Project insert transaction commits;
5. after Project commit before UI receives response;
6. before artifact-ref/edge commit;
7. after commit before client acknowledgement;
8. during backup/restore workflow used for rollback/recovery.

Required outcome: reopening the vault produces either the complete committed old state or the complete committed new state, never half-authority.

## 7. Pre-074 migration fixtures

Use synthetic/permitted fixtures representing at least:

- empty encrypted vault;
- populated vault with current patient/FHIR/source/evidence/document/model metadata;
- vault with current archived/amended data where relevant;
- backup/restore fixture already supported by current repository;
- a fixture close enough to current production-shaped schema to catch index/table migration regressions.

Do not introduce real PHI fixtures.

## 8. Migration qualification sequence

For each representative fixture:

```text
1. open with pre-074-compatible base behavior
2. record stable object IDs/digests/authority facts required for comparison
3. create verified backup/recovery checkpoint
4. apply 074 migration once
5. inspect schema/version metadata
6. execute pre-074 regression operations
7. execute 074 Project/Experiment/ref/edge operations
8. close process cleanly
9. reopen
10. verify exact Project + old-object identities/revisions/digests as applicable
11. attempt normal open/migration again; it must be safe according to migration framework
12. restore the pre-migration backup in a separate recovery path and prove old state remains usable
```

## 9. Rollback rule

Default rollback is **restore from the verified pre-migration backup**.

Do not implement a destructive SQL down-migration merely to claim rollback. A down-migration is allowed only if tests prove it can preserve all pre-existing and 074-created state required by the declared rollback contract.

If 074-created state cannot be represented by the old schema, restoration of the pre-migration checkpoint necessarily discards post-checkpoint 074 changes; this must be stated honestly in recovery UX/evidence.

## 10. Schema version ownership

T074-02 must identify the exact current storage schema/migration mechanism and add the smallest next version transition compatible with it.

Do not invent a parallel `research_os_schema_version` if the vault already has canonical migration/version metadata.

Unsupported future schema versions fail closed rather than being opened with partial interpretation.

## 11. Backup behavior

Project-owned metadata is included in the existing vault backup/recovery mechanism. Large referenced canonical artifacts remain governed by their existing backup rules; 074 must not duplicate them inside Project backup records.

A restored Project must either resolve its references correctly or show explicit missing/stale states.

## 12. Archive and deletion

074 supports archive/tombstone/detach, not destructive Project data erasure as a product feature.

- Project archive retains Experiments/references/edges for inspection/recovery.
- Experiment archive retains its references.
- reference detach removes/tombstones only the association.
- edge removal removes/tombstones only the relation.
- target canonical object deletion remains owned by the target's existing system.

GC must not infer that a target canonical artifact is unneeded merely because its last Project reference was detached.

## 13. Concurrency

Preserve current writer-lock/transaction model. Spec 074 does not introduce multi-device concurrent writers.

Within the admitted local writer model, every mutation of existing Project-owned state uses expected revision/precondition semantics. Stale request -> `Conflict`, no last-write-wins.

## 14. Migration evidence

Closure evidence must record:

```text
base SHA
candidate SHA
storage schema version before/after
fixture identity/digest
migration command/test
backup checkpoint evidence
pre-074 regression results
new 074 workflow results
close/reopen results
repeat-open/migration results
restore results
known limitations
```

A migration test that only creates a fresh empty database is insufficient.

## 15. T074-02 freeze block

Before T074-02 is declared complete, update:

```text
MIGRATION_CONTRACT = FROZEN_FOR_074
CURRENT_STORAGE_VERSION = <live value>
074_STORAGE_VERSION = <new value>
MIGRATION_CODE_PATH = <exact path>
BACKUP_CODE_PATH = <exact path>
PROJECT_TABLES = <exact names>
INDEXES = <exact names/queries>
ROLLBACK_METHOD = <verified method>
```

## 16. Freeze record (T074-02, FROZEN_FOR_074)

```text
MIGRATION_CONTRACT = FROZEN_FOR_074
CURRENT_STORAGE_VERSION = 2
074_STORAGE_VERSION = 3
MIGRATION_CODE_PATH = crates/medscale-storage/src/sqlite_meta.rs (migrate),
                      crates/medscale-storage/src/project_graph.rs (V3_DDL + rows)
BACKUP_CODE_PATH = crates/medscale-storage/src/backup.rs (restore_v3 / restore_v2),
                   crates/medscale-storage/src/encrypted_vault.rs (sealed backup/restore)
PROJECT_TABLES = projects, experiments, project_artifact_refs, project_graph_edges
INDEXES = idx_projects_scope_status,
          idx_experiments_project, idx_experiments_project_status,
          idx_refs_project, idx_refs_project_experiment,
          idx_refs_active_tuple (partial unique, backstop),
          idx_edges_project_subject, idx_edges_project_object,
          idx_edges_project_predicate,
          idx_edges_active_tuple (partial unique, backstop)
ROLLBACK_METHOD = restore verified pre-migration backup (no destructive
                  down-migration; interrupted journal fails closed with
                  MigrationIncomplete)
```

Representation decision: rows ARE the canonical encoding (scalar columns plus
validated JSON for `ArtifactVersionBinding` and endpoints only). No parallel
`body_json` exists to diverge; reads re-validate and report `Corrupt` or
`UnsupportedSchema`. Synthetic backup snapshot is schema 3 (additive 074
arrays); restore accepts 2 (legacy, no 074 rows) and 3 (exact replay with
re-validation). EncryptedVault sealed backup carries the whole DB file, so 074
rows ride the existing sealed path with no code change.

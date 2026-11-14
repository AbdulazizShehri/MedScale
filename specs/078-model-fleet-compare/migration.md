# Migration and Recovery — Spec 078

**Status:** implementation contract

Spec 078 extends the existing encrypted MedScale vault. It does not create a
second database, replace current canonical object storage, rewrite pre-078
object identities, or modify any Spec 077 (or earlier) table.

## 1. Default migration posture

The migration is additive:

```text
pre-078 encrypted vault (storage schema v6)
  + agent-lane + lane-policy structures
  + fleet-run structures
  + lane-run-ref structures
  + comparison-report (+ comparison-observation) structures
```

Existing canonical objects (patients, FHIR, sources, documents, evidence,
models, Packs, Projects, Experiments, graph edges, data sources, snapshots,
collaboration rows, and every Spec 077 `medagent_*` row) remain exactly
where their current owners store them. No existing object is automatically
enrolled in a fleet run by migration. Old workflows remain valid; the fleet/
compare layer is opt-in capability layered on top.

## 2. Required schema families

```text
model_fleet_lanes
  (LanePolicy travels as a validated JSON column on this same row,
  mirroring Spec 077's medagent_context_manifests.selected_artifacts_json
  precedent -- no separate model_fleet_lane_policies table; there is
  exactly one policy per lane, never a partially-visible list)
model_fleet_runs
model_fleet_lane_run_refs
model_fleet_comparison_reports
  (observations travel as validated JSON on the report row, same
  precedent -- a ComparisonReport's observation list is always fully
  visible or not visible at all)
```

Every durable mutable row needs:

- stable opaque identity (`OpaqueId`);
- realm/authority scope or an unambiguous existing path to it (via
  `project_id -> realm/scope`, reusing Spec 074's Project scope rather than
  duplicating it on every row);
- schema version;
- monotonic mutation revision (`ProjectRevision`) for mutable rows
  (`model_fleet_lanes`, `model_fleet_runs`), or plain insert-once semantics
  for append-only/terminal-write rows (`model_fleet_lane_run_refs`,
  `model_fleet_comparison_reports`);
- status/tombstone state where applicable;
- bounded metadata only -- no bulk patient/document/model payload ever
  lives in a 078 table beyond what a `LaneRunRef`/`ComparisonReport`
  legitimately records (and those never duplicate Spec 077's own
  `AgentProposal`/`RunReceipt` content, they only reference it by id).

## 3. Required indexes

At minimum, indexes must efficiently support:

- agent-lane list by Project and by bound agent identity;
- fleet-run list by Project and by state;
- lane-run-ref list by fleet run, and by agent lane;
- comparison-report lookup by fleet run (1:1 per computed report, but a
  `FleetRun` may have more than one `ComparisonReport` over time if
  recomputed -- see `contracts.md` section 4 -- so the index supports a
  list, not a strict 1:1 unique lookup).

Do not add speculative indexes without a query/measurement justification.

## 4. Referential behavior

078 tables may use internal foreign keys between 078-owned rows when
compatible with current storage design. They reference Spec 077 rows
(`AgentIdentity`, `ContextManifest`, `AgentRun`) by `OpaqueId` only, never
by foreign key into Spec 077's own tables (mirroring the existing
cross-subsystem reference discipline: Spec 077's `ContextManifest` itself
references Spec 074 `ArtifactDescriptor`s the same way).

078 must **not** use cascade behavior that can delete or mutate current
canonical patient/FHIR/source/document/evidence/model/Pack/Project/
data-source/snapshot/collaboration/medagent data. A lane's bound
`agent_identity_id`/`context_manifest_id` is validated for existence at
lane-creation time; their live resolution (status, revision) is always
recomputed at dispatch time through Spec 077's own existing resolution
path, never cached as a join that could go stale silently.

## 5. Transaction boundaries

Required atomic units:

### Agent lane creation

One transaction: `model_fleet_lanes` row (+ its `LanePolicy` JSON) commits
together, or neither does. The subset-of-capability/context validation
(`security.md` T2) happens inside this same transaction, before commit.

### Fleet run creation and per-lane dispatch

`model_fleet_runs` row insert is one transaction. Each lane's dispatch
(create the lane's own Spec 077 `AgentRun` + write the
`model_fleet_lane_run_refs` row binding it) is one transaction **per
lane**, not one giant transaction across every lane -- a `FleetRun` with
N lanes performs N independent dispatch transactions, each of which either
fully succeeds (real `AgentRun` created, `LaneRunRef` recorded) or fully
fails (refused before any lane-run side effect), so that one lane's
dispatch failure never leaves a different lane's already-dispatched run in
a half-recorded state.

**T078-02 reconciliation (binding).** The per-lane "one transaction"
wording above cannot hold literally together with this spec's other hard
rule that the lane's run is created through Spec 077's *unmodified*
`create_agent_run` (`plan.md` 078-D, `security.md` section 1): that
function commits its own `AgentRun` insert, and its id allocator
(`alloc_medagent_id`) opens its own SQLite transaction, which SQLite cannot
nest inside an outer one. Wrapping it would require modifying a Spec 077
Core/storage function, which this spec forbids. The frozen dispatch order
is therefore:

```text
1. re-validate lane policy against the live identity/context (T2)
2. Spec 077 create_agent_run            -> commits AgentRun (Pending)
3. insert_lane_run_ref                  -> commits LaneRunRef (unique agent_run_id, T4)
4. Spec 077 start_agent_run / execute_agent_run, only after step 3 committed
```

The invariant this preserves is the one the atomicity rule exists for: a
lane run is never *started* unless its binding is durable, so a fleet can
never be missing a lane that actually ran. A crash or refusal between
steps 2 and 3 leaves at most one never-started, unbound `Pending` Spec 077
run -- a state Spec 077 already treats as valid (a `Pending` run carries
no `RunReceipt`) -- and never touches a sibling lane's committed binding.
Core makes a best-effort compensating `cancel_agent_run` on that unbound
run when step 3 is refused in-process. Proven by
`crates/medscale-storage/tests/model_fleet_078.rs::failed_lane_binding_leaves_sibling_intact_and_only_an_unstarted_unbound_run`
(storage half) and the T078-04 Core tests (ordering half).

### Fleet run state transition

One transaction per transition contains the fleet-run row's state/revision
update. `FleetRunState` aggregation (recomputing `Running`/`Completed`/
`PartiallyFailed`/`Failed` from each bound lane's current
`AgentRunState`) is a read-then-write operation: it reads every bound
lane's current terminal-or-not state, computes the aggregate, and writes
the fleet-run row's new state/revision in one transaction, using
`expected_revision` CAS exactly like any other mutable 078 row (so two
concurrent aggregation attempts cannot both win with stale input).

### Comparison report

`ComparisonReport` write is one transaction: the report row + its full
`observations` JSON commit together, or neither does. A `ComparisonReport`
is never visible with a partial/truncated observation list.

### Multi-operation UI workflow

UI convenience must not turn several logically independent commands into an
undocumented giant transaction. If the product needs atomic multi-command
behavior, Core must expose an explicit typed operation and tests.

## 6. Crash points

Tests must exercise or deterministically simulate at least:

1. before migration begins;
2. after migration metadata indicates start but before commit;
3. after migration commit;
4. before an agent-lane insert transaction commits;
5. before a fleet-run creation transaction commits;
6. before one lane's dispatch transaction commits (proving a sibling
   lane's already-committed dispatch is unaffected; per the section 5
   reconciliation, "dispatch" is the `LaneRunRef` commit that must
   precede any start of the lane's run);
7. before a fleet-run state-transition (including terminal/partial-failure
   transitions) commits;
8. before a `ComparisonReport` commits;
9. after commit before client acknowledgement;
10. during backup/restore workflow used for rollback/recovery;
11. mid-fleet cancellation racing one lane's concurrently-completing
    terminal transition (the lane must land in exactly one terminal state,
    never both `Cancelled` and `Completed`, reusing Spec 077's own T7 test
    directly).

Required outcome: reopening the vault produces either the complete
committed old state or the complete committed new state, never a fleet run
visible in `Running` forever with no way to reach a terminal state after
restart, never a `FleetRun` whose bound `LaneRunRef` set is missing a lane
that was actually dispatched, and never a `ComparisonReport` referencing a
lane run that does not actually exist.

## 7. Pre-078 migration fixtures

Use synthetic/permitted fixtures representing at least:

- empty encrypted vault;
- populated vault with current patient/FHIR/source/evidence/document/model
  metadata;
- populated 074 vault with Projects/Experiments/refs/edges;
- populated 075 vault with data sources/snapshots/saved views;
- populated 076 vault with rooms/threads/messages/tasks/notes/approvals/
  activity;
- populated 077 vault with at least one `AgentIdentity` bound to the real
  admitted `pack-tiny-token-classifier-v0` Pack, at least one
  `ContextManifest`, and at least one completed `AgentRun`/`AgentProposal`/
  `RunReceipt`, to prove the 078 lane/fleet layer can bind to genuinely
  pre-existing Spec 077 objects, not only ones created in the same test;
- a fixture close enough to current production-shaped schema to catch
  index/table migration regressions;
- backup/restore fixture already supported by current repository.

Do not introduce real PHI fixtures.

## 8. Migration qualification sequence

For each representative fixture:

```text
1. open with pre-078-compatible base behavior
2. record stable object IDs/digests/authority facts required for comparison
3. create verified backup/recovery checkpoint
4. apply 078 migration once
5. inspect schema/version metadata (expect v7)
6. execute pre-078 regression operations (074/075/076/077 ops, including
   a full Spec 077 agent-identity/context/run/tool/receipt/proposal cycle)
7. execute 078 lane/fleet/comparison operations, including at least one
   two-lane FleetRun bound to a pre-existing 077 AgentIdentity/
   ContextManifest from the fixture itself
8. close process cleanly
9. reopen
10. verify exact lane/fleet-run/lane-run-ref/comparison-report identities
    and seq/revision values, plus old-object identities/revisions/digests
    (074/075/076/077 unaffected)
11. attempt normal open/migration again; it must be safe according to
    migration framework
12. restore the pre-migration backup in a separate recovery path and prove
    old state remains usable and 078 tables are simply absent, not
    half-populated
```

## 9. Rollback rule

Default rollback is **restore from the verified pre-migration backup**. Do
not implement a destructive SQL down-migration. If 078-created state cannot
be represented by the old schema, restoring the pre-migration checkpoint
discards post-checkpoint 078 changes; this must be stated honestly in
recovery UX/evidence, exactly as 075/076/077 state it for their own
migrations.

## 10. Schema version ownership

T078-02 must extend the exact current storage schema/migration mechanism
(live value: v6, established by Spec 077) with the smallest v6 -> v7
transition compatible with it, following the same
`if journal.finished_version < 7 { begin_migration(7); execute_batch(crate::model_fleet::V7_DDL); finish_migration(7); }`
pattern already used for v5 -> v6 in
`crates/medscale-storage/src/sqlite_meta.rs`.

Do not invent a parallel schema-version tracker. Unsupported future schema
versions fail closed rather than being opened with partial interpretation.

## 11. Backup behavior

078-owned metadata is included in the existing vault backup/recovery
mechanism. A restore that produces a `FleetRun` in a terminal state whose
`LaneRunRef` set does not match its actual dispatched lanes, or a
`ComparisonReport` referencing a lane run that does not exist, is a
corruption signal, not a silently accepted state -- mirroring the lesson
Spec 076's and 077's own exact-range reviews surfaced: whatever restore-time
invariant this spec defines must actually be re-verified at restore time,
not merely assumed from a passing insert-time check.

## 12. Archive and deletion

078 supports archive/tombstone, not destructive erasure as a product
feature.

- Agent-lane retirement is a status tombstone; past `LaneRunRef`/
  `ComparisonReport` history is never reattributed or anonymized by
  retirement.
- Fleet-run/lane-run-ref/comparison-report rows are never edited after
  being written; the append/terminal-write pattern is the audit trail,
  exactly matching Spec 077's own rows.
- canonical target object deletion (Projects, artifacts, data sources,
  snapshots, collaboration rows, Spec 077 agent/run/proposal rows) remains
  owned entirely by its existing system; 078 never deletes or mutates a
  Spec 077 (or earlier) object.

## 13. Concurrency

Preserve current writer-lock/transaction model. Spec 078 does not introduce
multi-device concurrent writers, remote/bounded-worker execution, or any
network write path. Within the admitted local single-writer model, every
mutation of existing mutable 078 state uses `expected_revision` precondition
semantics; a `FleetRun`'s state machine additionally enforces its frozen
transition table (illegal transitions are `Conflict`/`InvalidArgument`,
never silently coerced to the nearest valid state).

## 14. Migration evidence

Closure evidence must record:

```text
base SHA
candidate SHA
storage schema version before/after (expect 6 -> 7)
fixture identity/digest
migration command/test
backup checkpoint evidence
pre-078 regression results (074 + 075 + 076 + 077 operations)
new 078 workflow results
close/reopen results
repeat-open/migration results
restore results
known limitations
```

A migration test that only creates a fresh empty database is insufficient.

## 15. T078-02 freeze block

Before T078-02 is declared complete, update:

```text
MIGRATION_CONTRACT = FROZEN_FOR_078
CURRENT_STORAGE_VERSION = 6 (live value on base ff2e677, set by Spec 077)
078_STORAGE_VERSION = 7
MIGRATION_CODE_PATH = crates/medscale-storage/src/sqlite_meta.rs
  (SqliteMetaStore::migrate, `finished_version < 7` step executing
  crates/medscale-storage/src/model_fleet.rs::V7_DDL inside
  begin_migration(7)/finish_migration(7))
BACKUP_CODE_PATH = crates/medscale-storage/src/backup.rs
  (backup manifest schema_version 7; restore_v7 -> restore_v6 chain, then
  SqliteMetaStore::verify_model_fleet_consistency) and
  crates/medscale-storage/src/sqlite_meta.rs::snapshot_bytes
  (snapshot schema_version 7 + four model_fleet_* families)
MODEL_FLEET_TABLES = model_fleet_lanes, model_fleet_runs,
  model_fleet_lane_run_refs, model_fleet_comparison_reports
INDEXES =
  idx_model_fleet_lanes_project (project_id, status)  -- lane list by Project [+ status]
  idx_model_fleet_lanes_identity (agent_identity_id)  -- lane list by bound identity
  idx_model_fleet_runs_project (project_id, status)   -- fleet list by Project [+ state]
  idx_model_fleet_lane_run_refs_fleet (fleet_run_id)  -- refs by fleet run
  idx_model_fleet_lane_run_refs_lane (agent_lane_id)  -- refs by agent lane
  idx_model_fleet_lane_run_refs_agent_run UNIQUE (agent_run_id) -- security.md T4
  idx_model_fleet_comparison_reports_fleet (fleet_run_id) -- reports by fleet run (list, not 1:1)
FOREIGN_KEYS = none (verified: pragma_foreign_key_list is empty for all four tables)
ROLLBACK_METHOD = restore the verified pre-migration (v6) backup via
  restore_vault; verified by
  crates/medscale-storage/tests/model_fleet_078.rs::pre_078_v6_backup_restores_with_empty_078_tables
  and ::crash_mid_migration_fails_closed_and_pre_migration_backup_recovers.
  Post-checkpoint 078 changes are discarded. Because restore always opens
  the destination with the current (v7) binary, the recovered vault's 078
  tables exist but are empty -- never half-populated.
```

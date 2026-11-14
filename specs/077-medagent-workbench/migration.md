# Migration and Recovery — Spec 077

**Status:** implementation contract

Spec 077 extends the existing encrypted MedScale vault. It does not create a
second database, replace current canonical object storage, or rewrite
pre-077 object identities.

## 1. Default migration posture

The migration is additive:

```text
pre-077 encrypted vault (storage schema v5)
  + agent identity + agent profile/capability-manifest structures
  + context-manifest (+ selected-artifact) structures
  + agent-run + agent-turn structures
  + tool-manifest + tool-invocation + tool-receipt structures
  + run-receipt structures
  + agent-proposal structures
```

Existing canonical objects (patients, FHIR, sources, documents, evidence,
models, Packs, Projects, Experiments, graph edges, data sources, snapshots,
collaboration rooms/threads/messages/tasks/notes/approvals/activity) remain
exactly where their current owners store them. No existing object is
automatically enrolled in an agent run by migration. Old workflows remain
valid; the agent workbench is opt-in capability layered on top.

## 2. Required schema families

```text
medagent_identities
medagent_capability_manifests
medagent_context_manifests
  (selected_artifacts travel as validated JSON on this row, mirroring
  collab_approval_requests.assignees_json -- no separate child table;
  there is exactly one row per manifest, never a partially-visible list)
medagent_runs
medagent_turns
medagent_tool_invocations
medagent_tool_receipts
medagent_run_receipts
medagent_proposals
```

Every durable mutable row needs:

- stable opaque identity (`OpaqueId`);
- realm/authority scope or an unambiguous existing path to it (via
  `project_id -> realm/scope`, reusing Spec 074's Project scope rather than
  duplicating it on every row);
- schema version;
- monotonic mutation revision (`ProjectRevision`) for mutable rows, or
  `seq` for append-only rows (turns, tool invocations/receipts);
- status/tombstone state where applicable;
- bounded metadata only -- no bulk patient/document/model payload ever
  lives in a 077 table beyond what a receipt/proposal legitimately records.

## 3. Required indexes

At minimum, indexes must efficiently support:

- agent identity list by Project and admitted-Pack;
- context-manifest artifact list by manifest;
- agent-run list by Project, by agent identity, and by state;
- agent-turn list by run, in `seq` order;
- tool-invocation list by run, in `seq` order; tool-receipt lookup by
  invocation;
- run-receipt lookup by run (1:1);
- agent-proposal list by run and by Project.

Do not add speculative indexes without a query/measurement justification.

## 4. Referential behavior

077 tables may use internal foreign keys between 077-owned rows when
compatible with current storage design.

They must **not** use cascade behavior that can delete or mutate current
canonical patient/FHIR/source/document/evidence/model/Pack/Project/
data-source/snapshot/collaboration data. `ContextManifest`'s artifact
references (`ArtifactDescriptor`s) are validated for shape at write time;
their live resolution is always recomputed at read time through the
existing Project-graph resolution path, never cached as a foreign-key join
that could go stale silently.

## 5. Transaction boundaries

Required atomic units:

### Agent identity registration

One transaction: `medagent_identities` row (+ capability-manifest row)
commit together, or neither does.

### Context manifest create

One transaction: manifest row + every selected-artifact row commit
together, or none do (a manifest with a dangling/partial artifact list is
never visible).

### Agent run state transition

One transaction per transition contains the run row's state/revision
update and, for a terminal transition (`Cancelled`/`Completed`/`Failed`),
the `RunReceipt` write, committed together.

### Agent turn / tool invocation / tool receipt

`AgentTurn` insert, or `ToolInvocation` insert plus its `ToolReceipt`
insert once the tool executes, commit atomically per step. A tool
invocation is never visible without either its receipt or an explicit
`Failed`/`Refused` outcome recorded against it.

### Agent proposal

`AgentProposal` write (and, per the T077-01 design decision, the
underlying `Proposal` submission it reuses) commits atomically with the
run's `Completed` transition. An `AgentProposal` never triggers a second
transaction against any other subsystem's tables beyond the existing
`Proposal`/`CreateProposal` path; if it did, that would itself be the
defect `security.md`'s structural no-effect control is written to catch.

### Multi-operation UI workflow

UI convenience must not turn several logically independent commands into
an undocumented giant transaction. If the product needs atomic
multi-command behavior, Core must expose an explicit typed operation and
tests.

## 6. Crash points

Tests must exercise or deterministically simulate at least:

1. before migration begins;
2. after migration metadata indicates start but before commit;
3. after migration commit;
4. before an agent-identity/context-manifest insert transaction commits;
5. before an agent-run state transition (including terminal transitions)
   commits;
6. before a tool-invocation/tool-receipt pair commits;
7. before a `RunReceipt` commits (proving a terminal run is never left
   visible without its receipt -- the two must be one transaction, not
   two);
8. after commit before client acknowledgement;
9. during backup/restore workflow used for rollback/recovery;
10. mid-run cancellation racing a concurrently-completing terminal
    transition (the run must land in exactly one terminal state, never
    both `Cancelled` and `Completed`).

Required outcome: reopening the vault produces either the complete
committed old state or the complete committed new state, never a run
visible in `Running` forever with no way to reach a terminal state after
restart, never a terminal run without its `RunReceipt`, and never a
`RunReceipt` for a run that did not actually reach that terminal state.

## 7. Pre-077 migration fixtures

Use synthetic/permitted fixtures representing at least:

- empty encrypted vault;
- populated vault with current patient/FHIR/source/evidence/document/model
  metadata;
- populated 074 vault with Projects/Experiments/refs/edges;
- populated 075 vault with data sources/snapshots/saved views;
- populated 076 vault with rooms/threads/messages/tasks/notes/approvals/
  activity, including at least one `ParticipantKind::Agent` participant
  with an unresolved `agent_profile_ref`, to prove the 077 integration
  point resolves correctly;
- a fixture close enough to current production-shaped schema to catch
  index/table migration regressions;
- backup/restore fixture already supported by current repository.

Do not introduce real PHI fixtures.

## 8. Migration qualification sequence

For each representative fixture:

```text
1. open with pre-077-compatible base behavior
2. record stable object IDs/digests/authority facts required for comparison
3. create verified backup/recovery checkpoint
4. apply 077 migration once
5. inspect schema/version metadata (expect v6)
6. execute pre-077 regression operations (074 project ops, 075 data-source
   ops, 076 collaboration ops)
7. execute 077 agent-identity/context-manifest/run/tool/receipt/proposal
   operations
8. close process cleanly
9. reopen
10. verify exact identity/manifest/run/turn/invocation/receipt/proposal
    identities and seq/revision values, plus old-object identities/
    revisions/digests
11. attempt normal open/migration again; it must be safe according to
    migration framework
12. restore the pre-migration backup in a separate recovery path and prove
    old state remains usable and 077 tables are simply absent, not
    half-populated
```

## 9. Rollback rule

Default rollback is **restore from the verified pre-migration backup**. Do
not implement a destructive SQL down-migration. If 077-created state cannot
be represented by the old schema, restoring the pre-migration checkpoint
discards post-checkpoint 077 changes; this must be stated honestly in
recovery UX/evidence, exactly as 075/076 state it for their own migrations.

## 10. Schema version ownership

T077-02 must extend the exact current storage schema/migration mechanism
(live value: v5, established by Spec 076) with the smallest v5 -> v6
transition compatible with it, following the same
`if journal.finished_version < 6 { begin_migration(6); execute_batch(crate::medagent::V6_DDL); finish_migration(6); }`
pattern already used for v4 -> v5 in
`crates/medscale-storage/src/sqlite_meta.rs`.

Do not invent a parallel schema-version tracker. Unsupported future schema
versions fail closed rather than being opened with partial interpretation.

## 11. Backup behavior

077-owned metadata is included in the existing vault backup/recovery
mechanism. A restore that produces a run with a terminal state but no
`RunReceipt` (or the reverse) is a corruption signal, not a silently
accepted state -- mirroring the lesson Spec 076's own exact-range review
surfaced for its activity hash chain (`evidence/076-collaboration-substrate/EXACT_RANGE_REVIEW.md`):
whatever restore-time invariant this spec defines must actually be
re-verified at restore time, not merely assumed from a passing insert-time
check.

## 12. Archive and deletion

077 supports archive/tombstone, not destructive erasure as a product
feature.

- Agent identity revocation is a status tombstone; past run history is
  never reattributed or anonymized by revocation.
- Run/turn/tool-invocation/tool-receipt/run-receipt/proposal rows are
  never edited after being written; the append/terminal-write pattern is
  the audit trail.
- canonical target object deletion (Projects, artifacts, data sources,
  snapshots, collaboration rows) remains owned entirely by its existing
  system; 077 never deletes or mutates a canonical object.

## 13. Concurrency

Preserve current writer-lock/transaction model. Spec 077 does not
introduce multi-device concurrent writers, remote/bounded-worker execution,
or any network write path. Within the admitted local single-writer model,
every mutation of existing mutable 077 state uses `expected_revision`
precondition semantics; an `AgentRun`'s state machine additionally enforces
its frozen transition table (illegal transitions are `Conflict`/
`InvalidArgument`, never silently coerced to the nearest valid state).

## 14. Migration evidence

Closure evidence must record:

```text
base SHA
candidate SHA
storage schema version before/after (expect 5 -> 6)
fixture identity/digest
migration command/test
backup checkpoint evidence
pre-077 regression results (074 + 075 + 076 operations)
new 077 workflow results
close/reopen results
repeat-open/migration results
restore results
known limitations
```

A migration test that only creates a fresh empty database is insufficient.

## 15. T077-02 freeze block

Before T077-02 is declared complete, update:

```text
MIGRATION_CONTRACT = FROZEN_FOR_077
CURRENT_STORAGE_VERSION = <live value, expect 5>
077_STORAGE_VERSION = <new value, expect 6>
MIGRATION_CODE_PATH = <exact path>
BACKUP_CODE_PATH = <exact path>
MEDAGENT_TABLES = <exact names>
INDEXES = <exact names/queries>
ROLLBACK_METHOD = <verified method>
```

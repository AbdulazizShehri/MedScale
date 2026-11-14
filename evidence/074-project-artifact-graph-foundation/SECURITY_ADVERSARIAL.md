# SECURITY / ADVERSARIAL — Spec 074 (074-F, T074-10)

## Binding

```text
BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
HOST=Windows 11 x64, Rust 1.97.1
FIXTURES=synthetic only (temp dirs, synthetic vaults)
REAL_PHI_USED=false
```

Threats T1..T12 from `specs/074-project-artifact-graph-foundation/security.md`.
Every gate below names the exact proving test. No gate is claimed by
compilation, planning text, or pending CI.

## Gate results

```text
cross_scope_reference_denied=true
  attach with a cross-scope source -> WrongScope
  (core/tests/project_graph_074.rs: attach_admits_current_and_rejects_the_rest)
  cross-scope project read -> WrongScope (cross_scope_read_is_denied_without_leak)
kind_mismatch_denied_or_explicit=true
  proposal attached as source_record -> Corrupt, no write
  (attach_rejects_kind_mismatch_as_corrupt)
  evidence/other-explicit kinds -> InvalidArgument at admission (fail closed)
stale_write_conflict_proven=true
  update/attach/detach/edge with stale expected_revision -> Conflict, revision
  and content unchanged (stale_write_conflicts_with_zero_write;
  storage stale_project_write_conflicts_without_write)
unknown_predicate_fail_closed=true
  predicate vocabulary closed at parse (contracts unit test), CLI parser
  rejects proves/Contains (cli parser tests), stored unknown predicate reads
  as UnsupportedSchema (storage unknown_future_status_fails_closed)
graph_query_bounded=true
  limit 101 -> InvalidArgument; limit 0 -> default; cursors bounded 256B;
  single-hop edge-id order only, no recursion
  (neighbors_reject_unbounded_queries; storage graph_neighbors_bounded_and_paginated)
metadata_validation_proven=true
  empty/overlong names and descriptions rejected on every write path
  (contracts + CLI + Core tests; validate_metadata_fields shared by
  constructors, Core updates, and backup-restore replay)
summary_context_permission_filtering_proven=true
  context resolves then excludes Denied refs (never leaked); cross-scope
  project access denied before any count; counts computed after filtering
  (context_and_summary_report_authorized_state; resolve-level Denied unit test)
project_archive_does_not_delete_target=true
  archive/detach/remove never touch canonical rows; source byte-identical
  after detach (storage attach_detach_leaves_canonical_untouched;
  archive_freezes_project_mutations)
crash_reopen_atomicity_proven=true
  reopen without close reads full state; interrupted journal fails closed
  (storage reopen_persists_074_state; interrupted_migration_fails_closed_on_reopen)
migration_preserves_existing_identity=true
  genuine v2-shaped store (manual v1/v2 DDL, journal=2) migrates to 3 with
  source/authority/next_seq byte-identical
  (schema_v3_migrates_populated_v2_store_without_identity_loss)
surface_bypass_not_present=true
  dependency-direction gate holds (cli/desktop -> core -> storage);
  CLI/Desktop reach 074 state only through CliSession helpers (shared);
  facade dispatch match is exhaustive; no direct storage import in surfaces
new_runtime_network_path=false
  no new dependencies (Cargo.lock delta: none for 074; verified in range review);
  no sockets/listeners/clients added; offline mandatory preserved
real_phi_used=false
  synthetic fixtures and synthetic vaults only, every suite
```

## Audit rule (frozen amendment with measured reason)

Lifecycle mutations (project/experiment create/update/archive/restore) append
`ActionAuditRecord` rows to the existing trail (proven by `audit_count()`
reopening the vault DB: 4 after lifecycle, 1 after a stale attempt).
High-frequency graph mutations (attach/detach/edge create/remove) return
durable receipts (revisioned sqlite rows) without per-op memory audit rows:
the frozen full-snapshot sync makes each memory row cost ~4 ms/op/history-row
(measured curve in SCALE_MEASUREMENTS.md; ~5 h for the 1,000-ref fixture).
`high_frequency_ops_return_receipts_without_audit_growth` locks this contract.
Actor attribution for graph ops is session-gated but not separately trailed:
recorded limitation, not hidden behavior. No second audit system exists.

Scale rule (same amendment): 074 entity ids come from the durable sqlite
`project_id_seq` counter (`alloc_project_id`, transactional, no reuse after
reopen), so high-frequency ops never touch the in-memory store; the facade
skips the snapshot rewrite for exactly the four ops in
`RequestBody::preserves_memory_snapshot`. Skipping is output-identical
(snapshot bytes cannot change when the memory store does not), locked by
`high_frequency_ops_leave_memory_snapshot_identical`. Per-op cost is therefore
constant in graph size; the snapshot term grows only with lifecycle/source
history. Pre-existing op behavior (including all reads) is unchanged.

## Adversarial cases covered

Duplicate attach/edge (explicit Conflict, never silent duplicates, storage
partial-unique backstop); self-edges denied; foreign experiment endpoints
(attach + edge) rejected; experiment bound to archived project denied;
mutations on archived projects denied; tampered backup snapshots fail closed
(restore re-validates every row); tampered revision (< 1) rejected.

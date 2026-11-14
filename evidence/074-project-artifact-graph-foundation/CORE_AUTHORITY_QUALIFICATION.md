# CORE AUTHORITY QUALIFICATION — Spec 074 (074-C, T074-03..T074-07)

## Binding

```text
BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
HOST=Windows 11 x64, Rust 1.97.1 (Strawberry Perl portable + VS2022 BuildTools for native deps)
FIXTURES=synthetic only (in-memory + temp-dir synthetic vaults)
REAL_PHI_USED=false
```

## Path

Every 074 operation runs `CoreFacade::dispatch` ->
`dispatch_inner` (schema, capability pair, Strict session gate) ->
`CoreFacade::pg` (lease, vault meta synthetic-or-encrypted, packs) ->
`authority::project_graph::ProjectGraph` op ->
storage compare-and-swap transaction ->
audit-or-receipt (rule below) -> typed `ResponseBody`.
No surface writes storage; the facade match is exhaustive so unhandled ops
cannot compile.

Audit rule (frozen 074-C amendment, measured reason in SCALE_MEASUREMENTS.md):
lifecycle mutations (project/experiment create/update/archive/restore) append
`ActionAuditRecord` rows to the existing trail. High-frequency graph mutations
(attach/detach/edge create/remove) return durable receipts: 074 ids come from
the transactional sqlite `project_id_seq` counter, the revisioned row is the
record, and the facade skips the snapshot rewrite for exactly the four ops in
`RequestBody::preserves_memory_snapshot` (output-identical: the memory store
is untouched). Locked by `high_frequency_ops_return_receipts_without_audit_growth`
and `high_frequency_ops_leave_memory_snapshot_identical`.

New wire vocabulary (`envelopes`): 19 `RequestBody` variants, 10
`ResponseBody` variants, 19 `capability_matches` pairs. Reads
(`ProjectRead`, `ExperimentRead`, `ProjectGraphRead`) are session-optional;
all mutations require sessions. Duplicate policy: repeats are explicit
`Conflict`, never silent duplicates (storage partial-unique backstop).

## Operation map (all proven by tests below unless noted)

```text
ProjectCreate/Update/Archive/Restore -> revisions 1..n, archived freezes mutations
ExperimentCreate/Update/Archive -> project-scoped, archived project/experiment deny, no restore
ProjectAttach -> Current-only admission, digest/version pin rules, duplicate Conflict
ProjectDetach -> tombstone, canonical untouched
ProjectListRefs -> auth-filtered (Denied excluded), paginated
GraphEdgeCreate/Remove -> endpoint admission, no self-edge, tombstone removal
GraphNeighborsQuery -> bounded single-hop, direction + predicate filter, cursor pages
ProjectContextResolve -> summary + optional experiment + filtered refs + graph slice
ProjectSummaryQuery -> metadata + post-filter counts
```

Resolution: identity+scope via authority store, vault-global packs by id,
digests compared where owners expose them (`SourceRecord`,
`DerivedSourceArtifact`, `PackManifest.version` for revisions), integrity
failures as `Corrupt`, unmapped families as `UnsupportedKind`. Evidence kinds
and future explicit kinds fail closed at attach (`InvalidArgument`).

## Tests

```text
cargo test -p medscale-core --locked --test project_graph_074 => PASS (12/12)
- project_lifecycle_end_to_end_with_revisions (create/get/update/archive/restore 1..4, 4 audits)
- mutation_without_session_is_rejected (SessionRequired)
- stale_write_conflicts_with_zero_write (Conflict, revision+name unchanged, 1 audit)
- cross_scope_read_is_denied_without_leak (WrongScope)
- capability_gate_denies_ungranted_project_ops (SessionDenied)
- project_op_without_vault_is_unavailable (VaultRequired)
- attach_admits_current_and_rejects_the_rest (Current ok; Missing->NotFound;
  cross-scope->WrongScope; bad digest->StaleReference; evidence kind->InvalidArgument;
  missing pack->NotFound)
- duplicate_attach_is_explicit_conflict
- archive_freezes_project_mutations (update/attach/edge on archived -> Unauthorized)
- edge_guards_reject_self_and_foreign_endpoints
- neighbors_reject_unbounded_queries (limit 101 -> InvalidArgument)
- context_and_summary_report_authorized_state (counts 1/1/1, filtered refs, slice)
cargo test -p medscale-core --locked --lib => PASS (9/9 incl. 2 new resolve-matrix tests:
  Current/Stale/Missing/UnsupportedKind/Denied/Corrupt through resolve_descriptor)
```

Audit proof: `audit_count()` reopens the vault meta DB and counts
`object_class='audit'` rows (4 after lifecycle, 1 after stale attempt):
audits persist through the existing snapshot sync, no second audit system.

## Limitations

- Pack digest/revision binding branches are code-reviewed but not
  integration-covered (no pack install in the 074 suites); identity attach of
  a missing pack is covered (Missing -> NotFound).

## Gates

```text
cargo fmt --all -- --check => PASS
cargo clippy --workspace --all-targets --locked -- -D warnings => PASS (exit 0)
cargo test -p medscale-core (lib + project_graph_074) => PASS
Full workspace test => DEFERRED to exact-head CI (local C: disk exhausted by a
  second full target dir; focused suites green, change is additive-only)
```

Pre-existing core suites: unchanged code paths untouched; full regression bound
to exact-head CI run for the pushed head (pending is never PASS).

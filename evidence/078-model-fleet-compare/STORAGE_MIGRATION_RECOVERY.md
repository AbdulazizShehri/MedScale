# Storage / Migration / Recovery — Spec 078 (T078-02)

## Binding

```text
BASE_SHA = ff2e677294b1ea8704bbeefa605d129c1a99e8f2 (origin/main; Spec 077 closure PR #134)
BRANCH = spec/078-model-fleet-compare (PR #135)
PARENT_HEAD = bf98a08e5080d6fc9cfdbe37b7ba0c1ce1f1add9 (T078-01 frozen contracts)
CANDIDATE_HEAD = recorded in tasks.md T078-02 "Verified" block after push (exact-head CI run id there)
STORAGE_SCHEMA_VERSION = 6 -> 7
```

## What changed

- `crates/medscale-storage/src/model_fleet.rs`: additive `V7_DDL` (four
  tables, seven indexes, zero foreign keys); CRUD with `expected_revision`
  CAS on `model_fleet_lanes`/`model_fleet_runs`; insert-only
  `model_fleet_lane_run_refs` (UNIQUE `agent_run_id`, `security.md` T4) and
  `model_fleet_comparison_reports`; full-table scans; plain-INSERT restore
  paths that turn any conflict into `CorruptObjectBody` (never
  `INSERT OR REPLACE`); and `verify_model_fleet_consistency`.
- `crates/medscale-storage/src/sqlite_meta.rs`: the `finished_version < 7`
  migration step; snapshot `schema_version` 7 plus the four `model_fleet_*`
  families.
- `crates/medscale-storage/src/backup.rs`: manifest `schema_version` 7;
  `restore_v7` (chains `restore_v6`, then re-runs contract shape checks per
  row, then calls `verify_model_fleet_consistency`).
- Fix-forward of stale `finished_version == 6` /
  `manifest.schema_version == 6` pins in `project_graph_074.rs`,
  `data_sources_075.rs`, `collaboration_076.rs` and `medagent_077.rs` (the
  same class of fix Spec 077 needed for 5 -> 6). No assertion was loosened;
  only the current-version constant changed.
- No Spec 077 contract, storage table, or Core function was modified.

## Restore-time invariants (`verify_model_fleet_consistency`)

Re-verified after every restore, not assumed from insert-time checks:

1. every `LaneRunRef` names an existing `FleetRun`, `AgentLane` and Spec 077 `AgentRun`;
2. no `AgentRun` is bound by more than one `LaneRunRef` (T4);
3. the bound `AgentRun` carries the lane's own `agent_identity_id`/`context_manifest_id` (T3);
4. lane, run and fleet share one Project;
5. at most `FLEET_RUN_MAX_LANES` refs per fleet, each lane at most once;
6. a terminal fleet has at least one bound run (unless cancelled before dispatch), and every bound run is terminal;
7. a non-`Cancelled` terminal fleet state equals `FleetRun::aggregate_state` over its bound runs;
8. every `ComparisonReport` names an existing `Completed`/`PartiallyFailed` fleet, passes `ComparisonReport::validate`, and names only lanes bound to that fleet.

## Dispatch-atomicity reconciliation

`migration.md` section 5 now records why lane dispatch is "create through
Spec 077's unmodified `create_agent_run`, then bind, then start" rather
than a single SQL transaction: Spec 077's id allocator opens its own
transaction, and SQLite cannot nest one, so atomic wrapping would require
modifying Spec 077. The preserved invariant is that no lane run is ever
started without a durable binding.

## Tests — `crates/medscale-storage/tests/model_fleet_078.rs` (14)

| Test | migration.md / security.md |
|---|---|
| `migration_v6_to_v7_preserves_populated_pre_078_vault_and_binds_to_its_077_objects` | s7/s8 steps 1-11: real v6 file (populated 074/075/076/077 fixture, then v7 tables + journal row removed), migrate, compare the full pre-078 snapshot view byte-for-byte, run a 077 cycle, build a two-lane fleet on the fixture's own identities, 3x reopen |
| `repeated_open_and_repeated_migration_is_safe` | s8 step 11 |
| `encrypted_vault_migrates_to_v7_and_reopens_with_its_key` | SQLCipher path; wrong key refused |
| `crash_mid_migration_fails_closed_and_pre_migration_backup_recovers` | s6 crash points 1-3 + 10: `started` without `finished` plus half-applied DDL gives `MigrationIncomplete(7)`; the pre-migration backup restores |
| `uncommitted_078_writes_are_absent_after_reopen` | s6 crash points 4, 5, 7, 8 |
| `failed_lane_binding_leaves_sibling_intact_and_only_an_unstarted_unbound_run` | s6 crash point 6 (per the s5 reconciliation) |
| `an_agent_run_cannot_be_bound_to_two_lane_run_refs` | T4 |
| `lane_retire_and_fleet_transition_are_stale_revision_safe` | T5 / s13 |
| `list_filters_are_project_and_state_scoped` | s3 indexes' queries |
| `verify_model_fleet_consistency_detects_every_invariant_break` | T8 / s11: nine direct-DB tampers, each `CorruptObjectBody` |
| `backup_restore_roundtrips_every_078_row_exactly` | s11 |
| `restore_rejects_hand_edited_078_snapshots` | s11 / T8: seven edits with the outer digest recomputed, each rejected by its intended check (message asserted) |
| `pre_078_v6_backup_restores_with_empty_078_tables` | s8 step 12 + s9 rollback |
| `model_fleet_tables_declare_no_foreign_keys_and_078_operations_never_touch_pre_078_rows` | s4/s12 no cascade |

Fixture identity: synthetic only; the 077 identities bind
`pack-tiny-token-classifier-v0` (the one real admitted Pack). No real PHI.

## Local execution (supporting evidence, not the qualification signal)

This workstation's Windows toolchain still cannot link (`EXTERNAL_GATES.md`
`LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026_09_18`). On 2026-09-22 the
worktree was mirrored into the WSL2 `Ubuntu` distro (rustc/cargo 1.97.1,
`x86_64-unknown-linux-gnu`) and run there:

```text
cargo test -p medscale-storage            -> the suites whose output was captured were green
                                             (lib, 076, 075, 005, 077, 048, 078); the captured
                                             output was truncated (`head -150`) before
                                             project_graph_074, so that suite was NOT observed
cargo test -p medscale-storage --test model_fleet_078  -> 14 passed, 0 failed
cargo clippy -p medscale-storage --all-targets -D warnings -> one type_complexity lint, fixed
```

Honest limits of that local run:
- it covered the tree *before* the final dispatch-ordering refactor (the
  removal of the unused atomic dispatch primitive and the rewrite of the
  crash-point-6 test);
- the WSL distro then failed to start (`Wsl/Service/CreateInstance/E_FAIL`)
  during a cold workspace-wide Clippy build, and recovering it needs
  `wsl --shutdown`, which would also stop the founder's running
  `docker-desktop` distro, so that was not done unattended.

Correction found by exact-head CI run `35765069726` (head `7d44400`):
`project_graph_074.rs::interrupted_migration_fails_closed_on_reopen`
failed on ubuntu and macOS. The 6 -> 7 fix-forward had updated its
`finished_version` assertion but not its `begin_migration(6)` /
`MigrationIncomplete(6)` pair, which must name the live top version. Once
v7 finishes, a v6 `started` row is superseded and invisible to the
fail-closed check. Fixed forward: the test now reads the top version from
the journal instead of pinning it (the `migration_recovery_048.rs` idiom),
so a later bump cannot silently weaken it again. `cargo test --workspace`
stops at the first failing test binary, so that run gives no signal for
later binaries; the rerun on the fixed head is the qualification.

So workspace Clippy/test were not completed locally. The exact-head CI run
recorded in `tasks.md` is the authoritative qualification for this task.
`cargo fmt --all -- --check` and `scripts/check-dependency-direction.ps1`
ran locally on Windows and passed.

## Known limitations

- Rollback restores into the current v7 binary, so the recovered vault's
  078 tables exist but are empty (never half-populated). A physical v6
  file is not reproduced; this matches the 075/076/077 restore semantics.
- Crash points 9 (after commit, before client acknowledgement) and 11
  (fleet cancel racing lane completion) are Core-level and are proven in
  T078-04.

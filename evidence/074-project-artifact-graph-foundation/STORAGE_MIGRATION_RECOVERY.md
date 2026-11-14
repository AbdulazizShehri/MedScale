# STORAGE / MIGRATION / RECOVERY — Spec 074 (074-B, T074-02)

## Binding

```text
BASE_SHA=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
BRANCH=spec/074-project-artifact-graph-foundation
HOST=Windows 11 x64, Rust 1.97.1, Strawberry Perl 5.42.3.1 portable (TEMP-only) + VS2022 BuildTools 14.44
FIXTURES=synthetic only, temp dirs medscale-074-*
REAL_PHI_USED=false
```

## Schema

```text
CURRENT_STORAGE_VERSION=2, 074_STORAGE_VERSION=3 (additive, same encrypted meta DB)
TABLES=projects, experiments, project_artifact_refs, project_graph_edges
SNAPSHOT_SCHEMA=3 (additive 074 arrays; restore accepts 2 legacy + 3 exact replay)
ID_SEQUENCE=store_state project_id_seq via alloc_project_id (transactional, durable)
```

## Qualification (cargo test -p medscale-storage)

```text
RESULT=PASS (all suites: 7 lib + 6 + 4 + 12 new project_graph_074 + 5 + 2; 0 failed)
- schema_v3_migrates_empty_store_additively: journal 3, pre-074 queries intact
- schema_v3_migrates_populated_v2_store_without_identity_loss: genuine v2-shaped DB
  (manual v1/v2 DDL, journal=2, 1 source + 1 authority row, next_seq=41) migrates to 3
  with source/authority/next_seq byte-identical and 074 tables usable
- interrupted_migration_fails_closed_on_reopen: MigrationIncomplete(4), no silent continue
- project_lifecycle_with_revision_gates: duplicate id Conflict, NotFound reads,
  +1 revision updates, archive/restore guarded, scoped list + status filter, cursor end
- stale_project_write_conflicts_without_write: revision and name unchanged after Conflict
- experiment_crud_counts_and_lists: CAS, counts, deterministic paginated walk
- attach_detach_leaves_canonical_untouched: source row byte-identical after detach
- duplicate_active_attach_conflicts: same tuple Conflict; detach frees tuple;
  experiment slot distinct
- graph_neighbors_bounded_and_paginated: both/outgoing/incoming, predicate filter,
  closed unknown predicate, paginated walk, remove tombstone, endpoints intact
- reopen_persists_074_state: crash-shaped reopen (no close) reads full state
- backup_restore_carries_074_rows_with_revisions: synthetic backup/restore preserves
  rows incl. non-initial revision (renamed project at revision 2)
- unknown_future_status_fails_closed: raw 'deleted' status reads as UnsupportedSchema
```

Pre-existing suites (048 migration/recovery, 005 encrypted vault, 023 metadata,
017 privacy) still PASS: migration is additive and snapshot v2 restore path kept.

## Gates

```text
cargo fmt --all -- --check => PASS
check-dependency-direction => PASS (no new crates; storage still below core)
cargo clippy -p medscale-storage --all-targets --locked -- -D warnings => PASS
cargo test -p medscale-storage --locked => PASS (see above)
```

## Limitations

- `list_all_*` snapshot scans load full 074 row sets into memory; scale evidence
  records observed sizes (no universal budget claimed).
- Forward restore (v3 backup into pre-074 binary) fails closed; documented, not supported.
- EncryptedVault sealed backup/restore path reuses the whole-DB seal (no new code);
  synthetic snapshot path covered by backup_restore test above.

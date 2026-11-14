# STORAGE_MIGRATION_RECOVERY — Spec 075

## Migration

```text
MIGRATION = v3 -> v4, additive (crates/medscale-storage/src/data_sources.rs: V4_DDL)
JOURNAL = existing begin/finish_migration framework (interrupted -> MigrationIncomplete)
REPEAT_OPEN = safe (IF NOT EXISTS everywhere;-open-twice covered by migration_v3_to_v4_preserves_old_rows)
```

## Fixtures and sequence

Covered by `crates/medscale-storage/tests/data_sources_075.rs`:

- fresh vault migrates to v4 (`journal.finished_version == 4`), reopen preserves rows;
- source CRUD with CAS conflicts; unknown id is `NotFound`;
- snapshot insert/list/parts/latest with duplicate-conflict backstop;
- receipts (import insert-once, refresh upsert), views (CAS), transformations, releases (snapshot-scoped inheritance, missing snapshot is `NotFound`);
- tampered status fails closed on read;
- backup/restore round-trip preserves fabric rows (manifest v4);
- corrupted id-sequence counter fails closed instead of silently resetting
  and risking a colliding id (`corrupt_id_sequence_fails_closed_instead_of_resetting`,
  added by the exact-range review);
- restore refuses a Database-source row carrying a stored credential_ref,
  closing a gap where `create_source`/`update_source` enforced this but
  `restore_v4` did not (`restore_rejects_database_source_with_credential_ref`,
  added by the exact-range review).

Also fixed by this closure: `crates/medscale-storage/tests/project_graph_074.rs`
(pre-existing Spec 074 tests) pinned `finished_version == 3`, which was
correct before this spec but became stale once the real v3->v4 migration
landed — a fresh/migrated store now genuinely finishes at v4. This was
masked in the first exact-head CI attempt (run 35422433021) by cargo's
default fail-fast stopping at an earlier, unrelated Core test failure
before reaching this test binary; fixed forward once that earlier failure
was fixed and this one was reached for real (commit bbc4d89).

```text
RESULT = PASS: exact-head CI run 35580861670 (head 9f84a6e) green 6/6;
PR #129 merged as 89a88cf; post-merge main run 35582548200 green 6/6.
```

## Pre-075 regression

Full workspace suite on the branch proves pre-075 workflows (074 graph,
vault, CLI, Desktop) still pass with v4 present. Encrypted sealed backup
carries v4 rows with no code change (same DB file).

## Rollback

Restore verified pre-migration backup. No destructive down-migration exists.
Interrupted migration fails closed with `MigrationIncomplete`.

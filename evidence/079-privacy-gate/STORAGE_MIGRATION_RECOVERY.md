# Storage, Migration and Recovery — Spec 079

`crates/medscale-storage/tests/privacy_gate_079.rs`: 12 passed, 0 failed in
run `35802759307` (ubuntu job; macOS and Windows jobs green).

| Requirement | Test |
|---|---|
| v7 -> v8 additive; populated pre-079 vault (source, Project, 078 lane) unchanged | `migration_v7_to_v8_preserves_populated_pre_079_vault` |
| Repeated open is a no-op; consistency holds | `repeated_open_is_a_no_op` |
| Crash mid-v8 migration fails closed (`MigrationIncomplete(8)`); pre-migration backup restores | `crash_mid_v8_migration_fails_closed_and_backup_recovers` |
| One classification per (Project, artifact); CAS revisions; replay refused | `classification_is_unique_per_artifact_and_revision_safe` |
| Revocation terminal and stale-safe (profile, receipt, map); revoked map refuses transforms | `profile_and_receipt_revocation_are_terminal_and_stale_safe` |
| Transform commit atomic; repeated pseudonym stored once | `transform_commit_is_atomic_and_counts_distinct_entries` |
| Row columns vs JSON body tamper fails closed on read | `edited_row_bodies_fail_closed_on_read` |
| Cross-row invariants detected (5 breaks) | `consistency_check_detects_every_invariant_break` |
| Backup/restore round-trips all 7 families exactly | `backup_restore_roundtrips_every_079_row_exactly` |
| Tampered backups refused (5 edits) | `restore_rejects_hand_edited_079_snapshots` |
| v7 backup restores with empty 079 tables | `pre_079_v7_backup_restores_with_empty_079_tables` |
| No foreign keys (repository convention) | `no_079_table_declares_a_foreign_key` |

All earlier storage suites (074-078) pass unchanged in behavior; their
version assertions now use `CURRENT_META_SCHEMA_VERSION` (see `migration.md`).

# Migration and Recovery — Spec 082

Additive: storage schema v10 -> v11. `CURRENT_META_SCHEMA_VERSION` becomes 11.

```text
analytics_receipts  (insert-once; one per query request)
analytics_results   (insert-once; unique receipt_id; canonical bytes BLOB,
                     digest re-checked on read)
analytics_cohorts   (insert-once)
```

A receipt and its derived table commit in one transaction.

Cross-row invariants (`verify_analytics_consistency`, run on restore):
- every row names a Project in its own realm and scope;
- every completed receipt has exactly one derived table that matches its id,
  digest, counts and Project;
- no table exists without its receipt;
- every cohort receipt names a stored cohort of the same Project.

A v10 backup restores with empty analytics tables. A crash mid-v11 migration
fails closed (`MigrationIncomplete(11)`). Tests of Specs 078-081 that rewind
a vault also drop the v11 tables.

Snapshots are never written by analytics. Replay reads pinned snapshots
through the normal digest-verified path and writes nothing.

# Migration and Recovery — Spec 080

Additive: storage schema v8 -> v9. `CURRENT_META_SCHEMA_VERSION` becomes 9.

```text
browse_allowlist  (unique project_id + host + path_prefix; revisioned)
browse_sessions   (revisioned only for awaiting_human_takeover -> cancelled)
browse_evidence   (insert-once; content BLOB; digest re-checked on read)
browse_downloads  (insert-once; quarantined content BLOB; digest re-checked)
browse_receipts   (insert-once; one per session)
```

A session and everything it produced commit in one transaction. Cross-row
invariants (`verify_browse_consistency`, run on restore): one receipt per
session whose state, step count and evidence/download ids match; no
evidence or download without its session. Backups carry content as hex and
restore re-checks every digest. A v8 backup restores with empty Browse
tables. A crash mid-v9 migration fails closed (`MigrationIncomplete(9)`).
Tests of earlier specs that rewind a vault also drop v9 tables and journal
rows.

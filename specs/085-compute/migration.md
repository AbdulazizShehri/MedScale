# Migration — Spec 085 MedScale Compute

Storage schema v13 -> v14, additive, inside `begin/finish_migration(14)`:

| Table | Purpose |
|---|---|
| `compute_jobs` | job manifest and state (`state` column indexed) |
| `compute_receipts` | one receipt per terminal job (`job_id` unique) |
| `compute_outputs` | committed output record and canonical bytes (`job_id`, `receipt_id` unique) |

No existing table changes. Reopening is idempotent.

State changes are conditional updates on the expected current state, so
a job is claimed at most once. A terminal state, its receipt and its
output are one transaction.

Backup: all three families are written (output bytes as hex). Restore of
a v14 snapshot requires all three families as arrays, re-checks every
job's manifest digest and every output's digest and canonical encoding,
and re-verifies the cross-row invariants; a v13 snapshot restores with
empty Compute tables. A job backed up while `running` restores as
`running` and is recovered as `interrupted` by Core. Rollback: an older
build opening a v14 vault leaves the Compute tables untouched and unused;
the supported downgrade is restoring a backup taken before the upgrade.

# Qualification — Spec 090 Institutional Adapters (object storage path)

`PASS`: exact-head run `36310428876` on `b1dc801` ran these tests green on Linux,
Windows and macOS (see `EXACT_HEAD_QUALIFICATION.md`).

| # | Requirement | Tests | Status |
|---|---|---|---|
| Q1 | Config validation: `local_phi` never leaves; credential handles, not secrets; plain object keys; payload-bound idempotency keys | contract `local_phi_never_leaves`, `credentials_are_handles_not_secrets`, `object_keys_are_relative_plain_paths`, `idempotency_keys_bind_the_payload` | PASS |
| Q2 | Intent, send, confirm with receipts; confirmed writes never resend | core `a_write_is_intended_sent_and_confirmed_with_receipts` | PASS |
| Q3 | Policy: unclassified and over-ceiling data never leave | core `local_phi_and_over_ceiling_data_never_leave` | PASS |
| Q4 | Outage -> failed; lost answer -> unknown; no blind retry; reconciliation confirms, re-arms or fails | core `uncertain_writes_become_unknown_and_reconcile_never_blindly_retries` | PASS |
| Q5 | Suspend, terminal revoke, reconfigure and rollback | core `revocation_and_rollback_are_explicit_and_final` | PASS |
| Q6 | The product transport sends nothing | core `the_product_transport_sends_nothing` | PASS |
| Q7 | Crash recovery (`sent -> unknown`, no resend); storage v19 migration, CAS, tamper refusal, backup/restore | core `a_crash_after_sent_recovers_as_unknown_never_as_a_resend`; storage `migration_to_v19_is_additive`, `intents_are_bound_and_changes_are_compare_and_set`, `backup_restore_round_trips_adapter_rows`, `tampered_adapter_backups_are_refused` | PASS |
| Q8 | CLI through Core with a closed act vocabulary | CLI `acts_parse_strictly` | PASS |
| Q9 | Exact-head and post-main CI | exact-head `36310428876` 6/6; post-main `36318426673` | PASS |

Not claimed: no network transport or real institutional endpoint; identity and compute institutional paths are not in this slice.

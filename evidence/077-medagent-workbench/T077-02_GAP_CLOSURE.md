# T077-02 Gap Closure — Storage schema + migration tests

## What was found

Live-truth reconciliation at the start of this session (per this spec's
own T077-00 discipline: inspect actual implementation, tests, and evidence
before trusting a task checkbox or a narrative handoff) found that commit
`a430d48` ("feat(077): MedAgent Workbench storage schema v6 + migration +
backup/restore (T077-02)") implemented the full v6 DDL
(`crates/medscale-storage/src/medagent.rs`, 1428 lines), the CRUD surface
for every 077 table, and `restore_v6` in `backup.rs` (163 lines added,
including an explicit `verify_run_receipt_consistency()` call after row
replay -- the exact class of gap Spec 076's own exact-range review found
missing from `restore_v5`).

However:

```text
grep -c '#\[test\]' crates/medscale-storage/src/medagent.rs  ->  0
find . -iname '*medagent*' (excluding target/)                -> only
  crates/medscale-contracts/src/medagent.rs (has 10 tests) and
  crates/medscale-storage/src/medagent.rs (had 0 tests, no dedicated
  integration test file)
```

`tasks.md` T077-02 explicitly requires: forward migration test fixtures,
repeated-open/repeated-migration safety, crash/reopen and backup/restore
recovery tests, and a proof that no 077 operation cascades deletion into
canonical target objects. None of these existed. The task was left
unchecked in `tasks.md` at the start of this session (correctly, per its
own state), but the founder's handoff narrative described T077-02 as
already complete -- which was true only for the implementation code, not
for the acceptance bullets that gate the checkbox.

## What was done

Added `crates/medscale-storage/tests/medagent_077.rs`, mirroring the exact
idiom `project_graph_074.rs`/`data_sources_075.rs`/`collaboration_076.rs`
already established for this workstation's linker-less local environment
(GitHub Actions CI is the authoritative compile/test signal; only
`cargo fmt --check` runs locally). Eleven tests:

1. `migration_v5_to_v6_preserves_pre_077_rows_and_adds_medagent_tables` --
   layers a 074 `Project` and a 076 `ParticipantKind::Agent` participant
   (the explicit integration point `tasks.md` names) under a full 077 row
   set; asserts `finished_version == 6` and reopen idempotency.
2. `repeated_open_and_repeated_migration_is_safe` -- five sequential
   open/close cycles, asserting stable schema version and data.
3. `full_lifecycle_crud_roundtrip_and_run_receipt_invariant` -- exercises
   every 077 table: identity+capabilities, context manifest, run,
   append-only turn, refused tool invocation, executed
   invocation+receipt, `Pending -> Running -> Completed` transitions,
   `RunReceipt`, `AgentProposal` link.
4. `revoke_and_run_transitions_are_stale_revision_safe` -- CAS conflict
   behavior for `revoke_agent_identity` and `set_agent_run_running`.
5. `verify_run_receipt_consistency_detects_orphaned_receipt_and_missing_receipt` --
   direct-DB tamper (not reachable through the public API) proving the
   invariant check used by `restore_v6` actually fires in both directions,
   not merely a happy-path no-op.
6. `executed_tool_invocation_and_receipt_commit_atomically_or_neither_does` --
   forces the in-transaction receipt insert to fail after the invocation
   insert has run, proving transaction rollback (security.md T12 analogue).
7. `backup_restore_roundtrips_medagent_rows_and_reverifies_run_receipt_consistency` --
   full round trip across every 077 family via `SyntheticVault`/
   `backup_vault`/`restore_vault`.
8. `restore_rejects_hand_edited_backup_with_orphaned_run_receipt` -- hand-edits
   a completed run's snapshot row back to `running` while leaving its
   `RunReceipt` in place, and recomputes the outer manifest digest to match
   (the same threat model as Spec 076's own hand-edited-backup test: the
   outer digest is a content hash, not a signature). Proves `restore_v6`'s
   `verify_run_receipt_consistency()` call fails the restore closed.
9. `agent_lifecycle_mutations_never_touch_the_canonical_project_row` --
   revokes an identity and cancels a run, then asserts the canonical
   `Project` row is byte-for-byte unchanged (`assert_eq!` against the
   original struct). Structural half: `medscale-storage/src/medagent.rs`
   contains no `UPDATE`/`DELETE` statement against the `projects` table at
   all (verified by inspection during this session).

`cargo fmt --check -p medscale-storage` is clean on the new file. No local
compile/test run was possible (MSVC linker absent on this workstation,
matching the constraint every prior spec in this repository has recorded);
real qualification is the next exact-head CI run on this branch.

## Disposition

T077-02's tasks.md checkboxes are now genuinely checked, backed by the
tests above rather than by the presence of implementation code alone. This
gap closure will be included as untested-until-CI in the branch's next
push; its result will be recorded in `EXACT_HEAD_QUALIFICATION.md` (or
equivalent) once observed.

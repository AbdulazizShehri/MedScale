# Qualification — Spec 086 R Workspace

Acceptance requirements (`docs/planning/SPEC_086_PROMOTION.md`) mapped to
the tests that exercise them. `PASS` means exact-head CI run `36241268247`
ran the tests green on Linux, Windows and macOS (Unix-only link tests on
Linux and macOS); see `EXACT_HEAD_QUALIFICATION.md`.

| # | Requirement | Tests | Status |
|---|---|---|---|
| A1 | Exact, read-only, deterministic staging; no vault reference | contract `csv_is_canonical_and_quoted`, `manifest_holds_its_layout`; core `staging_writes_exact_read_only_copies_outside_the_vault`, `staging_is_deterministic_across_workspaces` | PASS |
| A2 | Staging refusals create nothing | core `staging_refusals_create_nothing` | PASS |
| A3 | Launch: configured absolute program, workspace only, allowlisted environment, receipts for every attempt | contract `launch_environment_is_allowlisted`; core `external_launch_gets_the_workspace_and_an_allowlisted_environment_only` | PASS |
| A4 | Run requests never execute; evidence bound; bad, missing, linked scripts refused | core `managed_runs_are_recorded_and_refused_never_executed` | PASS |
| A5 | Publication admits exact CSV only and refuses unsafe inputs and states | core `publication_refuses_unsafe_or_inexact_outputs`, `a_linked_outputs_directory_is_not_followed` (Unix), `a_changed_or_missing_workspace_blocks_launch_run_and_publish`, `a_stale_input_blocks_publication`, launch test (digest mismatch) | PASS |
| A6 | Restart, backup/restore, tamper refusal, v14 compatibility | core `workspaces_and_publications_survive_restart`; storage `migration_v14_to_v15_is_additive`, `r_workspace_rows_hold_their_invariants`, `tampered_r_workspace_rows_fail_closed_on_read`, `backup_restore_round_trips_r_workspace_rows`, `tampered_r_workspace_backups_are_refused`, `pre_086_v14_backup_restores_with_empty_r_workspace_tables` | PASS |
| A7 | No crossing of Project, realm or scope | core `staging_refusals_create_nothing` (other Project); scope checks in every read path | PASS |
| A8 | CLI only through Core | CLI `r_commands_run_through_core_across_fresh_sessions`, `digests_and_ids_parse_strictly` | PASS |
| A9 | Exact-head and post-main CI | exact-head `36241268247` 6/6; post-main see `POST_MERGE_VERIFICATION.md` | PASS |

Regression: the Spec 078-085 storage rewind tests now also drop the v15
tables; their assertions are otherwise unchanged. The Spec 085 test no
longer pins `CURRENT_META_SCHEMA_VERSION` to 14 (same pattern as the Spec
084 test when 085 moved to v14).

Not claimed: containment of the external IDE or R; R version; package
reproducibility; Arrow/Parquet; Desktop rendering.

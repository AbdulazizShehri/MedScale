# Qualification — Spec 092 Whole-Platform Qualification

`PASS`: exact-head run `36360503149` on `7c4f850` ran these tests green on Linux,
Windows and macOS (see `EXACT_HEAD_QUALIFICATION.md`).

| # | Requirement | Tests | Status |
|---|---|---|---|
| Q1 | One synthetic vault crosses every Research OS plane through Core, then backup, restore, every storage consistency verifier and restart; honest defaults hold afterwards | core `every_plane_survives_backup_restore_and_restart_through_core` | PASS |
| Q2 | The campaign's defect (id sequences lost on restore) is fixed and regression-tested | storage `restored_vaults_continue_every_id_sequence`, `malformed_sequences_are_refused_and_old_backups_still_restore` (PR #160) | PASS |
| Q2b | The nondeterministic `privacy_gate_079` leak check is fixed without weakening leak detection | core `value_matching_ignores_digest_collisions_but_finds_leaks`, `corpus_transforms_write_new_artifacts_and_bind_receipts_without_leaking_values` | PASS |
| Q3 | 35-area matrix with PROVEN / IMPLEMENTED / PARTIAL / MISSING / DEFERRED / EXTERNAL / N/A, bound to `EXTERNAL_GATES.md` | `MATRIX.md` (document; not a test) | RECORDED |
| Q4 | Exact-head and post-main CI | exact-head `36360503149` 6/6; post-main `36368496614` | PASS |

Not claimed: release readiness, private-data readiness, platform qualification, clinical validation, regulatory status.

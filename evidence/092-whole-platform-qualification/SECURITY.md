# Security challenge — Spec 092 Whole-Platform Qualification

| # | Attack / risk | Control | Evidence |
|---|---|---|---|
| S01 | Subsystem PASS substituted for integrated evidence | One vault crosses every plane through Core in a single test, then backup, restore, verifiers and restart | core `every_plane_survives_backup_restore_and_restart_through_core` |
| S02 | Restored vault reusing ids (receipt or object collisions) | Backups carry every id sequence; restore never lowers a sequence and refuses malformed keys | storage `restored_vaults_continue_every_id_sequence`, `malformed_sequences_are_refused_and_old_backups_still_restore` |
| S03 | Identity key recovered from a backup | The federation identity secret is never exported; after restore export is refused with `NoIdentity` | whole-platform restore assertions |
| S04 | Honest defaults silently flipped after restore | The campaign re-reads the Compute sandbox and managed-R admission states after restart (`platform_qualified=false`, managed R not admitted); extension-code and transport defaults are covered by their own spec tests, not re-read here | whole-platform restart assertions |
| S05 | Matrix upgrading unknowns to PASS | Status vocabulary forbids upgrades without evidence; external rows name the gate in `EXTERNAL_GATES.md` | `MATRIX.md` |

Not controlled / recorded: every `EXTERNAL` matrix row stays open; real PHI,
signed release packages and qualified hardware are outside this campaign.

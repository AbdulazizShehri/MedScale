# Qualification — Spec 087 Community Extensions

Closure gate (declarative reading): a third-party sample extension is
built, packaged, installed, denied a non-granted capability, upgraded with
capability re-consent, revoked, and run without trusted-process or vault
access. `PASS` is recorded only for rows that exact-head CI ran green on
Linux, Windows and macOS (see `EXACT_HEAD_QUALIFICATION.md`, written with
the closure).

| # | Requirement | Tests | Status |
|---|---|---|---|
| Q1 | Closed manifest; canonical bytes; unknown capability / executable entrypoint fail closed | contract `manifests_validate_strictly`, `unknown_capabilities_and_entrypoints_fail_closed`, `signing_payload_is_domain_separated` | PASS |
| Q2 | Verification: trusted publisher, key match, strict signature, API range, size, canonical form | core `untrusted_forged_or_incompatible_packs_are_refused` | PASS |
| Q3 | Build + package (SDK), install with no grants, denial until granted, ceiling enforcement, run through Core | core `a_sample_extension_installs_is_denied_until_granted_and_runs_through_core` | PASS |
| Q4 | Upgrade with capability expansion needs re-consent; downgrade refused; rollback; uninstall revokes grants | same | PASS |
| Q5 | Revocation quarantines atomically; no cross-project access; quarantined stays off | core `revocation_quarantines_and_nothing_crosses_projects`; storage `install_changes_are_compare_and_set_and_atomic` | PASS |
| Q6 | Grants respect ceilings | contract `grants_respect_their_ceiling` | PASS |
| Q7 | Storage v16 migration, CAS, tamper refusal, backup/restore | storage `migration_to_v16_is_additive`, `tampered_release_rows_fail_closed`, `backup_restore_round_trips_extension_rows`, `tampered_extension_backups_are_refused` | PASS |
| Q8 | CLI through Core; SDK digest parsing | CLI `digests_parse_strictly` | PASS |
| Q9 | Exact-head and post-main CI | exact-head `36258146789` 6/6; post-main `36263227436` | PASS |

Exact-head run `36258146789` on `4405062` ran every test above green on Linux,
Windows and macOS (see `EXACT_HEAD_QUALIFICATION.md`).

Not claimed: executable extensions (WASM/worker) — `NOT_ADMITTED`; a Hub
registry; license verification; Desktop surface.

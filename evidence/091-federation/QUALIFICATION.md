# Qualification — Spec 091 Federation (bounded bundle exchange)

`PASS`: exact-head run `36331357728` on `c9c523e` ran these tests green on Linux,
Windows and macOS (see `EXACT_HEAD_QUALIFICATION.md`).

| # | Requirement | Tests | Status |
|---|---|---|---|
| Q1 | Raw PHI never federates; bundle signatures cover the exact body; plain institution ids | contract `raw_phi_never_federates`, `bundle_signatures_cover_the_exact_body`, `institution_ids_are_plain` | PASS |
| Q2 | Signed exchange with provenance; replay refused; imports never overwrite; tombstones erase bytes | core `signed_bundles_carry_provenance_and_never_overwrite` | PASS |
| Q3 | Policy denial of PHI and over-ceiling data; unknown peers get nothing | core `policy_denies_phi_and_over_ceiling_data` | PASS |
| Q4 | Forged, misaddressed and revoked exchanges refused | core `forged_misaddressed_and_revoked_exchanges_are_refused` | PASS |
| Q5 | Storage v20: migration, atomic imports, single identity, secret never in backups, tamper refusal | storage `migration_to_v20_is_additive`, `imports_are_atomic_and_identities_are_single`, `backups_never_carry_the_identity_secret`, `tampered_federation_backups_are_refused` | PASS |
| Q6 | CLI through Core with a closed act vocabulary | CLI `acts_parse_strictly` | PASS |
| Q7 | Exact-head and post-main CI | exact-head `36331357728` 6/6; post-main `36338491427` | PASS |

Not claimed: federated analysis, network transport and any central service are not in this slice; raw PHI never federates.

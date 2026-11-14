# EXACT HEAD QUALIFICATION — Spec 075

## Binding

```text
CANDIDATE_HEAD=9f84a6e74805d2c326cec40ab4703cc3fff0e069
BRANCH=spec/075-data-source-fabric
BASE=ae0441918296c2d1510a71061249c7e55e65d760 (origin/main, unchanged)
PR=129 (https://github.com/TheHalfMoon/MedScale/pull/129)
```

## Exact-head CI (workflow `ci`, run 35580861670 — all green)

```text
106273167980 cargo-deny success
106273168150 perf delivery-plan scale (windows) success
106273168253 supply-chain policy present success
106273168317 rust (macos-latest) success
106273168334 rust (windows-latest) success
106273168355 rust (ubuntu-latest) success
```

This is the fourth exact-head CI attempt on this PR, not the first — see
`CLOSURE.md`'s closure trail for the full sequence. The first attempt (run
35422433021) failed for real on all three platforms with two Core
`AcquireFail` classification bugs; fixed in `b6de6a2`. The second attempt
(run 35573757788) failed on a different, previously-masked bug in
`crates/medscale-storage/tests/project_graph_074.rs` (stale pre-075 schema
version assertions), reached only because the first fix let cargo's
fail-fast progress further; fixed in `bbc4d89`. The third attempt (run
35574786250) passed 6/6 but had not yet been through a full exact-range
semantic review of the entire PR diff; that review (`/code-review high`,
base `ae04419`, head `bbc4d89`) found 18 confirmed findings, including a
real authority-scope-bypass read escape and a real credential-restore gap,
fixed with regression tests in `aa771e9`. This run (35580861670) is the
result: the review fixes plus this evidence packet's own docs-only update
(`9f84a6e`), qualified together on the exact head that was actually
merged. Pending CI is never PASS; this run is completed/success, verified
live via `gh`.

## What this run proves

- Every Spec 075 focused suite green on all three OSes: contracts (incl.
  the new `remote_dataset_file_names_reject_traversal_and_absolute_paths`),
  storage (incl. the new `corrupt_id_sequence_fails_closed_instead_of_resetting`
  and `restore_rejects_database_source_with_credential_ref`, and the fixed
  `project_graph_074` migration-version assertions), Core (incl. the new
  `database_source_cannot_name_the_vaults_own_metadata_store`,
  `quarantined_import_marks_source_health_stale_not_silently_healthy`,
  `oversized_local_file_is_rejected_before_full_read`,
  `release_list_pagination_cursor_is_not_silently_discarded`), CLI,
  Desktop.
- Pre-existing suites unregressed (full `cargo test --workspace` green,
  including every pre-075 spec's tests).
- No new product dependencies (cargo-deny + supply-chain green, lockfile
  changes are transitive/version-lock only).

## Not covered by this run (honest boundaries)

- Rendered Desktop captures: no rendering step exists in
  `.github/workflows/ci.yml` and none was produced locally either (this
  workstation's toolchain cannot link) — see DESKTOP_QUALIFICATION.md.
- Remote dataset live-fetch: exercised only as fail-closed
  deny-before-socket; no real Hugging Face/Kaggle fetch was attempted.
- Full-fixture scale TIMINGS beyond the single aggregate figure recorded
  in SCALE_MEASUREMENTS.md: CI's default test output does not print
  per-test timing.

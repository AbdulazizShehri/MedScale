# EXACT HEAD QUALIFICATION — Spec 076

## Binding

```text
CANDIDATE_HEAD=04fa4cbab26aa617fdd6f1d7537f8a0b56afebcf
BRANCH=spec/076-collaboration-substrate
BASE=6021ff9aad397a8488087cae01e56528370e8211 (origin/main, unchanged)
PR=131 (https://github.com/TheHalfMoon/MedScale/pull/131)
```

## Exact-head CI (workflow `ci`, run 35634828435 — all green)

```text
106450115068 rust (macos-latest) success
106450115326 cargo-deny success
106450115348 rust (windows-latest) success
106450115381 rust (ubuntu-latest) success
106450115462 perf delivery-plan scale (windows) success
106450115528 supply-chain policy present success
```

This is the exact head reached after the OpenCodeReview exact-range review
fix (see `EXACT_RANGE_REVIEW.md`) — not the first CI attempt on this branch.
The closure trail (full sequence in `CLOSURE.md`):

1. Commit `58d7379` (T076-11 Core + storage evidence suite): CI run
   `35628104959` failed on all three OS targets --
   `clippy::doc_lazy_continuation` (a module doc-comment line began with
   `"+ qualification sequence)"`, parsed by rustdoc/clippy as an
   unindented markdown list continuation). Fixed in `61a3d72`.
2. Commit `61a3d72`: CI run `35628606985` failed on ubuntu/macos --
   `E0499` (a nested `h.source_record(...)` call inside
   `source_anchor(...)` double-borrowed the test harness's `&mut self`
   while an outer `h.call(...)` already held it). Fixed in `adead4e`.
3. Commit `adead4e`: CI run `35629013553` failed on ubuntu/macos with a
   **real functional bug**, not a test-code defect --
   `collab_workspace_flows_through_real_core_session` (the new Desktop
   in-module Core-session test) failed with `create room: Unauthorized`,
   because `ensure_self_participant` registered the operator under the
   hardcoded string `"desktop-operator"` instead of the session's actual
   bound holder id, meaning the real "Create Room" action would have
   failed the same way on a running Desktop build. Fixed in `bf9907c`
   (added `CliSession::holder_id()`).
4. Commit `bf9907c`: CI run `35630430836` green 6/6. This became the input
   to the OpenCodeReview exact-range review.
5. Exact-range review (OpenCodeReview delegation mode, per explicit
   instruction) found one confirmed finding: `restore_v5` never
   re-verified the activity hash chain after replay, so a hand-edited
   backup would restore silently instead of failing closed
   (`security.md` T10 / `migration.md` section 11). Fixed with a
   regression test in `04fa4cb`.
6. Commit `04fa4cb`: CI run `35634828435` (recorded above) — green 6/6.
   This is the head this qualification binds to.

Pending CI is never PASS; this run is `completed/success`, verified live
via `gh run view 35634828435 --json status,conclusion`.

## What this run proves

- Every Spec 076 focused suite green on all three OSes: contracts (23
  unit tests in `collaboration.rs`), storage (migration v4->v5
  preservation, backup/restore roundtrip, activity hash-chain tamper
  detection, half-committed-state atomicity, the hand-edited-backup
  regression from the review finding), Core (functional lifecycle for
  every entity family plus `security.md` T1-T4/T9 adversarial tests),
  CLI, Desktop (including the real-Core-session in-module test that
  caught and proved the fix for the holder-id bug).
- Pre-existing suites unregressed (full `cargo test --workspace` green,
  including every pre-076 spec's tests, and the forward-fixed
  `project_graph_074.rs`/`data_sources_075.rs` migration-version
  assertions).
- No new product dependencies (cargo-deny + supply-chain-policy green).
- The native Slint UI compiles on all three OS targets, including
  `rust (windows-latest)` (also independently proven on the
  Desktop-panel-only commit `ebec119`, CI run `35624477017`).

## Not covered by this run (honest boundaries)

- Rendered Desktop captures: no rendering step exists in
  `.github/workflows/ci.yml` and none was produced locally either (this
  workstation's toolchain cannot link) -- see `DESKTOP_QUALIFICATION.md`.
- Content-leakage / no-secret-logging (T11) is proven by source-text
  inspection, not a log-capture-based automated test -- see
  `SECURITY_ADVERSARIAL.md`.
- Notes and Approvals have no Desktop UI surface in this closure
  (CLI-only, documented gap, matching Spec 075's saved-views closure
  discipline).

# EXACT HEAD QUALIFICATION — Spec 074

## Binding

```text
CANDIDATE_HEAD=83719f8c3d78dbd2b820236a86b0913dd3c885a
BRANCH=spec/074-project-artifact-graph-foundation
BASE=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09 (origin/main, unchanged)
PR=122 (https://github.com/TheHalfMoon/MedScale/pull/122)
```

## Exact-head CI (workflow `ci`, run 35306206868 — all green)

```text
105478858912 cargo-deny success
105478858796 rust (ubuntu-latest) success (fmt, dep-direction, clippy -D warnings,
  cargo test --workspace incl. 074 contract/storage/core/cli/desktop/scale-smoke suites,
  portable-package qualification, perf coverage evidence)
105478858887 rust (macos-latest) success
105478858719 rust (windows-latest) success
105478858569 perf delivery-plan scale (windows) success
105478858763 supply-chain policy present success
```

This run compiled every 074-C/F change that postdates the local toolchain
event (facade snapshot-skip, sqlite id sequence, revision guards, new
adversarial/snapshot/sequence/desktop-session tests, scale smoke shapes) on
all three platforms and ran the full workspace suite green, including the
previously failing `high_frequency_ops_leave_memory_snapshot_identical`
(now fixed) and the `is_multiple_of` clippy gate. Pending CI is never PASS;
this run is completed/success, verified live via `gh`.

## What this run proves

- Focused 074 suites green on all OSes (contracts 38, storage incl. 13
  project_graph_074, core lib + 16 authority + 3 scale-smoke, CLI 13,
  desktop 27+3 with the new session test).
- Pre-existing suites unregressed (full `cargo test --workspace` green).
- No new dependencies (cargo-deny + supply-chain green, lockfile untouched).

## Not covered by this run (honest boundaries)

- Full-fixture scale TIMINGS (`MEDSCALE_074_FULL_SCALE=1`): CI runs the smoke
  shape by design; full numbers need a working local toolchain (external
  gate `LOCAL_WINDOWS_TOOLCHAIN_DESTRUCTION_2026-09-18`, still open).
- Rendered desktop captures: produced locally pre-event, bound by SHA-256 in
  DESKTOP_QUALIFICATION.md (not a CI artifact by design).

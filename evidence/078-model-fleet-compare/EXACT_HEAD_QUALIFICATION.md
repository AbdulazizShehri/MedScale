# Exact-Head Qualification — Spec 078

## Binding

```text
BASE_SHA        = ff2e677294b1ea8704bbeefa605d129c1a99e8f2 (origin/main)
BRANCH          = spec/078-model-fleet-compare (PR #135)
CODE_HEAD       = 547fb4175be4211798cc85631eabea8968b2ffb4
CI_RUN          = 35768404980 (workflow `ci`, event pull_request)
RUN_CONCLUSION  = success, 6/6 required jobs, verified bound to CODE_HEAD
                  (one-shot `gh run view` after completion; branch head and
                  origin/main unchanged while it ran)
TOOLCHAIN       = rustc/cargo 1.97.1 (pinned in rust-toolchain.toml and ci.yml)
```

## Scope of this record

This is CI evidence for **CODE_HEAD `547fb41` only**. The commit that adds
this file and the `tasks.md` bookkeeping creates a new head. That docs head
needs its own required CI run before it can be treated as exact-head
qualified, and it is recorded below once it exists. Still pending on the
final merge candidate: that fresh CI run.

Review policy (amended 2026-09-22): the Alibaba Open Code Review exact-range
review that this file first listed as pending is no longer required
(`docs/planning/FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22.md`). It is replaced
by the deterministic scope record in `EXACT_RANGE_REVIEW.md`. No external
reviewer ran or is claimed.

## Required jobs on CODE_HEAD

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Each `rust (*)` job runs, in order: `cargo fmt --all -- --check`,
`scripts/check-dependency-direction.ps1`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`
(`RUSTFLAGS=-Dwarnings`), `cargo test --workspace --locked`, and the
portable release package qualification.

## Spec 078 test binaries (ubuntu job 106883812075)

```text
medscale-storage  tests/model_fleet_078.rs   14 passed, 0 failed
medscale-core     tests/model_fleet_078.rs   18 passed, 0 failed
medscale-core     lib unit tests             26 passed; by name, includes the model_fleet T1 scan
                                               and all 6 model_fleet_compare tests
medscale-contracts lib unit tests            81 passed; by name, includes the 12 model_fleet tests
medscale-cli      model_fleet::tests          lane_commands_run_through_core_across_fresh_sessions ... ok
                                               (binary count not separately extracted)
medscale-desktop  main unit tests            37 passed; by name, includes both model_fleet_workspace tests
workspace         FAILED lines in log         0
```

The macOS job ran the same named tests, all `ok`, with 0 FAILED lines. Its
per-binary counts are not reliable in the raw log because of stream
interleaving (see `logs/ci-35768404980-test-summary.txt`).

The Windows job's per-test log was not parsed. Its evidence is the job
conclusion (`success`, covering fmt, dependency direction, clippy, the
workspace test step and portable-package qualification) only.

## Local gates (Windows workstation, CODE_HEAD)

```text
cargo fmt --all -- --check                 pass
scripts/check-dependency-direction.ps1     pass
git diff --check (crates, evidence)        pass
dependency manifests changed vs BASE       none (Cargo.toml / Cargo.lock / deny.toml / supply-chain untouched)
Spec 077 source files changed vs BASE      none (medagent.rs in contracts/storage/core untouched)
new logging/print calls in core+storage    none
```

## Earlier heads on this branch (superseded, kept for the record)

| Head | Run | Result |
|---|---|---|
| 7d44400 (T078-02) | 35765069726 | failed: 074 interrupted-migration test pinned to v6 (real regression, fixed in d9c75a8) |
| ebaa259 (T078-03) | 35765829841 | failed: clippy `enum_variant_names` on the CLI enum |
| d0de9cf (T078-04) | 35766798630 | failed: CLI test use-after-move (test compile) |
| a4b6581 (T078-05/06) | 35767590639 | fmt/dep/clippy + ubuntu tests success; other jobs cancelled by the next push |
| 1f9d52f (T078-07) | 35768281643 | superseded by 547fb41 (CRLF restore only) |

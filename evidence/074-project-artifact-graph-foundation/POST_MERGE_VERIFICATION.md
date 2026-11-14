# POST-MERGE VERIFICATION — Spec 074

## Binding

```text
PR=122 (https://github.com/TheHalfMoon/MedScale/pull/122)
MERGE_SHA=3d59255d0f37800cdda85dd4a7f12238356b02ad (merge commit, normal merge, no force/rebase/bypass)
MERGE_AT=2026-09-18T05:13:14Z
PRE_MERGE_HEAD=660ca54fcaa1da88ca69d90cdf95c2cf634aac42
PRE_MERGE_CI_RUN=35308381108 (6/6 success on the exact pre-merge head)
BASE_AT_MERGE=ee8daef3a2782bbdbcb3324766a5d6b95c09fa09
```

## Post-merge main CI (workflow `ci`, run 35309949710 — all green)

```text
HEAD=3d59255d0f37800cdda85dd4a7f12238356b02ad (origin/main)
rust (ubuntu-latest): success
rust (windows-latest): success
rust (macos-latest): success
perf delivery-plan scale (windows): success
cargo-deny: success
supply-chain policy present: success
```

Verified live via `gh` run/job inspection. No required check was skipped,
weakened, or bypassed at any point in this lane.

## Result

```text
POST_MERGE_MAIN_CI=PASS (6/6 on the exact merge commit)
MERGE_METHOD=merge-commit (normal GitHub merge of a MERGEABLE/CLEAN PR)
```

Closure bookkeeping (`CLOSED_CANONICAL`) is recorded in `CLOSURE.md` and
`docs/planning/BUILD_QUEUE.md` only after this verification.

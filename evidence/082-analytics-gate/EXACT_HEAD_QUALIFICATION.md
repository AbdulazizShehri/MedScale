# Exact-Head Qualification — Spec 082

```text
BASE_SHA   = 44f71e606ac74b632d417f2aa86202e79f71f9ea
BRANCH     = spec/082-analytics-gate (PR #142)
CODE_HEAD  = 11150b5b6c578ac15d08e332af1266ed52b5f424
CI_RUN     = 35921559272 (pull_request), 6/6 success, bound to CODE_HEAD
TOOLCHAIN  = rustc/cargo 1.97.1
BASE POST-MAIN RUN = 35921463274 (push on 44f71e6, 6/6 success)
```

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Each `rust (*)` job runs fmt check, dependency direction, Clippy with
`-D warnings`, the full workspace tests, and portable package qualification.
Counts from the job logs (`test result:` lines summed):

| Job | `test result` lines | passed | failed | ignored | `FAILED`/`panicked` lines |
|---|---|---|---|---|---|
| rust (ubuntu-latest) | 121 | 807 | 0 | — | 0 |
| rust (windows-latest) | 121 | 804 | 0 | 1 | 0 |

Every Spec 082 test named in `QUALIFICATION.md` appears with `ok` in the
ubuntu log; the engine bound and timeout tests also appear with `ok` in the
Windows log. The macOS job passed; its per-test counts were not extracted.

The commit that adds this evidence creates a new head, which needs its own
required CI run before merge. That run is recorded in
`POST_MERGE_VERIFICATION.md` and `CLOSURE.md`.

Earlier heads on this branch (historical only):

| Head | Run | Result |
|---|---|---|
| f71281c | 35908135701 | 6/6 success (base `876b9fe`; before the qualification challenge) |
| 624989b | 35915807824 | cancelled (superseded) |
| 52ef6c4 | 35915994684 | ubuntu/macOS/deny/perf success (806 passed, 0 failed on ubuntu); Windows cancelled (superseded) |

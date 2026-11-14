# Exact-Head Qualification — Spec 081

```text
BASE_SHA   = c682178ed4b8a4bbee67fe64d89cc94450d60b13
BRANCH     = spec/081-audioflow-foundation (PR #141)
CODE_HEAD  = 502b49aaca3acc977ebf124a3f329b8cee7371d6
CI_RUN     = 35889704517 (pull_request), 6/6 success, bound to CODE_HEAD
TOOLCHAIN  = rustc/cargo 1.97.1
BASE POST-MAIN RUN = 35885589830 (push on c682178, 6/6 success)
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
The ubuntu job log shows 119 `test result` lines, 770 passed, 0 failed,
0 `FAILED`/`panicked` lines, and every Spec 081 test named in
`QUALIFICATION.md` with `ok`.

The commit that adds this evidence creates a new head, which needs its own
required CI run. That run is recorded in `POST_MERGE_VERIFICATION.md` and
`CLOSURE.md`.

Earlier heads on this branch (historical only; see `EXACT_RANGE_REVIEW.md`):

| Head | Run | Result |
|---|---|---|
| fafe3bc | 35885773446 | failed (compile) |
| 710e59c | 35887232414 | failed (Clippy) |
| f1e6356 | 35888572661 | failed (Spec 066 hardening test) |

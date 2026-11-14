# Exact-Head Qualification — Spec 083

```text
BASE_SHA   = f08f90314243267eca9e2edae1d6666f3fb74a10 (PR #144 merge)
BRANCH     = spec/083-knowledge-canvas (PR #145)
CODE_HEAD  = 8a05ccfa5e5a2fde922e938a988999c2ee124286
CI_RUN     = 35997730588 (pull_request), 6/6 success, bound to CODE_HEAD
TOOLCHAIN  = rustc/cargo 1.97.1
BASE POST-MAIN RUN = 35997628344 (push on f08f903, 6/6 success)
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
| rust (ubuntu-latest) | 123 | 832 | 0 | 1 | 0 |
| rust (windows-latest) | 123 | 829 | 0 | 1 | 0 |

Every Spec 083 test named in `QUALIFICATION.md` appears with `ok` in the
ubuntu log, including `restore_rejects_hand_edited_083_snapshots` (17
cases), `pre_083_v11_backup_restores_with_empty_knowledge_tables`, and the
PR #144 cases in `restore_rejects_hand_edited_080_snapshots` and
`restore_rejects_hand_edited_081_snapshots`. The restore tamper, reopen,
isolation and stale-source tests also appear with `ok` in the Windows log.
The macOS job passed; its per-test counts were not extracted.

The commit that adds this evidence creates a new head, which needs its own
required CI run before merge. That run is recorded in
`POST_MERGE_VERIFICATION.md` and `CLOSURE.md`.

Earlier heads on this branch (historical only):

| Head | Run | Result |
|---|---|---|
| 665a161 | 35971707983 | 6/6 success (base `3ea6610`) |
| 9f04a4d | 35983576587 | 6/6 success (base `4b9351a`; before the restore hardening and the PR #144 merge) |

# Live Truth — Spec 083 (T083-00)

Observed 2026-09-24 with `git fetch`, `gh pr list`, `gh pr view`, `gh run view`.

```text
BASE (origin/main) = 4b9351a56f58ae23146e6b77db62e941d2863132
                     (merge of closure PR #146; Spec 082 closure bookkeeping)
BRANCH             = spec/083-knowledge-canvas (PR #145, draft until qualified)
RECONCILIATION     = forward merge of main into the branch (9625f04);
                     no rebase, no force-push
OPEN PRS           = #124, #125 (drafts, unrelated; not touched)
STORAGE SCHEMA     = v11 on base; this spec adds v12
DEPENDENCIES       = none added; lexical scoring is native Rust
REVIEW POLICY      = FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external
                     reviewer)
CLOSURE PR #146    = exact-head run 35977276424 (6/6 on 93dfd6f)
POST-MAIN RUN FOR BASE = 35983448840 (push on 4b9351a), 6/6 success;
                     Spec 082 CLOSED_CANONICAL on this base
PR #144            = exact-head run 35983517642 (6/6 on ef0f4b2), merged
                     as f08f903 (2026-09-24); post-main run 35997628344
                     (6/6 success)
CURRENT BASE       = f08f90314243267eca9e2edae1d6666f3fb74a10, forward-merged
                     into the branch as 8a05ccf (no conflicts)
PREVIOUS 083 HEAD  = 9f04a4d run 35983576587 (6/6, base 4b9351a; historical)
EARLIER 083 HEAD   = 665a161 run 35971707983 (6/6, base 3ea6610; ubuntu
                     832 passed, 0 failed)
```

# Post-Merge Verification — Spec 092

```text
PR            = #159, merged 2026-09-28
MERGE_METHOD  = merge commit, --match-head-commit 7c4f850
FINAL_HEAD    = 7c4f85059644689b6ca858a664476dce0883258e
EXACT_HEAD_CI = 36360503149 (pull_request, 6/6 success)
MERGE_SHA     = 9e0759b217c7c5959e3d100db6a2dd7118da44c5
POST_MAIN_CI  = 36368496614 (push on main, headSha 9e0759b, 6/6 success)
BASE_POST_MAIN_CI = 36359115799 (push on bef016f, 6/6 success)
```

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion.

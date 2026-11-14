# Post-Merge Verification — Spec 093

```text
PR            = #162, merged 2026-09-28
MERGE_METHOD  = merge commit, --match-head-commit a5317e4
FINAL_HEAD    = a5317e4669ac056935defc90a75e9c23ea6ca173
EXACT_HEAD_CI = 36383307415 (pull_request, 6/6 success)
MERGE_SHA     = 83457bf04a0982d18e1fdd2da84d759759be77d1
POST_MAIN_CI  = 36393892783 (push on main, headSha 83457bf, 6/6 success)
BASE_POST_MAIN_CI = 36383275508 (push on 41d2a9a, 6/6 success)
```

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion.

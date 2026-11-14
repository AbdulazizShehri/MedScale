# Post-Merge Verification — Spec 094

```text
PR            = #167, merged 2026-09-28
MERGE_METHOD  = merge commit, --match-head-commit ec1c943
FINAL_HEAD    = ec1c94356ef5072fbab9acf93a8c2804e1509c57
EXACT_HEAD_CI = 36428223212 (pull_request, 6/6 success)
MERGE_SHA     = c2a1ee9d6215bc24253913856ed4206485c6c52c
POST_MAIN_CI  = 36444728694 (push on main, headSha c2a1ee9, 6/6 success)
BASE_POST_MAIN_CI = 36428188226 (push on 4dfc8ad, 6/6 success)
```

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion.

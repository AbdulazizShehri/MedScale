# Post-Merge Verification — Spec 091

```text
PR            = #158, merged 2026-09-27
MERGE_METHOD  = merge commit, --match-head-commit c9c523e
FINAL_HEAD    = c9c523ea6589c75980bc3ae25ba0299f2e9f3fa3
EXACT_HEAD_CI = 36331357728 (pull_request, 6/6 success)
MERGE_SHA     = c4438eff5ea4220bfab2c2663cc71ec3854a9adc
POST_MAIN_CI  = 36338491427 (push on main, headSha c4438ef, 6/6 success)
BASE_POST_MAIN_CI = 36331339772 (push on ee7e1d5, 6/6 success)
```

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion.

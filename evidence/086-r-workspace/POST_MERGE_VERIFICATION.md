# Post-Merge Verification — Spec 086

```text
PR            = #152 (spec/086-r-workspace -> main), merged 2026-09-26
MERGE_METHOD  = merge commit, --match-head-commit c8e8551
FINAL_HEAD    = c8e855176a8d7e0cdbaa9539351eb4dd74d58c96
EXACT_HEAD_CI = 36241268247 (pull_request, 6/6 success, bound to FINAL_HEAD)
MERGE_SHA     = 16f2d1fdd600caeb1e5fe29c88b0d06578d0bfae
POST_MAIN_CI  = 36246757563 (push on main, headSha 16f2d1f, 6/6 success)
BASE_POST_MAIN_CI = 36237543003 (push on 61855e4, Spec 085 closure, 6/6 success)
```

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion. The PR left draft only after its exact-head run passed.

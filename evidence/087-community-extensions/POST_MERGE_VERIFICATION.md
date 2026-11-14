# Post-Merge Verification — Spec 087

```text
PR            = #153 (spec/087-community-extensions -> main), merged 2026-09-26
MERGE_METHOD  = merge commit, --match-head-commit 4405062
FINAL_HEAD    = 44050620ab438830c60e7ce945c26baeb7e031ab
EXACT_HEAD_CI = 36258146789 (pull_request, 6/6 success)
MERGE_SHA     = 2045998be9abe1bb74cbcbfb059a3b1dfe9146e6
POST_MAIN_CI  = 36263227436 (push on main, headSha 2045998, 6/6 success)
BASE_POST_MAIN_CI = 36258108623 (push on 742a93a, Spec 086 closure, 6/6 success)
```

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion. The PR left draft only after its exact-head run passed.

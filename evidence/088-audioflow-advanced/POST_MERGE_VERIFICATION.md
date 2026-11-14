# Post-Merge Verification — Spec 088

```text
PR            = #155, merged 2026-09-27
MERGE_METHOD  = merge commit, --match-head-commit 2521656
FINAL_HEAD    = 25216565b9df873ed6125cc01676684c62f13ae7
EXACT_HEAD_CI = 36275414657 (pull_request, 6/6 success)
MERGE_SHA     = 5059e73068352df8a56b142d83e89c5701a72011
POST_MAIN_CI  = 36280485565 (push on main, headSha 5059e73, 6/6 success)
BASE_POST_MAIN_CI = 36275368889 (push on be572bb, a descendant of a40bd1e, 6/6 success; the a40bd1e push run 36273286008 was cancelled by the #160 merge under the main concurrency group)
```

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion.

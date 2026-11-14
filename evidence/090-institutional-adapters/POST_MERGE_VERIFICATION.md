# Post-Merge Verification — Spec 090

```text
PR            = #157, merged 2026-09-27
MERGE_METHOD  = merge commit, --match-head-commit b1dc801
FINAL_HEAD    = b1dc801b95b5f552cc0d785206a9d21636e53ec3
EXACT_HEAD_CI = 36310428876 (pull_request, 6/6 success)
MERGE_SHA     = 2cc0f3501948c886085091e524c7c33c3ad83b5e
POST_MAIN_CI  = 36318426673 (push on main, headSha 2cc0f35, 6/6 success)
BASE_POST_MAIN_CI = 36310388657 (push on 8df3e45, 6/6 success)
```

Verified with one-shot `gh run view <run> --json headSha,event,status,conclusion,jobs`
after completion.

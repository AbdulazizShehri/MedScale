# Post-Merge Verification — Spec 085

```text
PR            = #149 (spec/085-compute -> main), merged 2026-09-26
MERGE_METHOD  = merge commit, --match-head-commit 68c20d5
FINAL_HEAD    = 68c20d59cec97781f352d27a26ff90c94bf8a3b3
EXACT_HEAD_CI = 36203947745 (pull_request, 6/6 success, bound to FINAL_HEAD)
CODE_HEAD_CI  = 36199048976 (pull_request on 3bd00d1; see EXACT_HEAD_QUALIFICATION.md)
MERGE_SHA     = 91021e21ecf2695c9b20dad3d54ed9ca6fa70774
POST_MAIN_CI  = 36208910691 (push on main, headSha 91021e2, 6/6 success)
BASE_POST_MAIN_CI = 36203706536 (push on 434d1c7, Spec 084 closure, 6/6 success)
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
after completion. The PR base at merge time was `434d1c7` (the PR #150
merge, Spec 084 closure). The PR left draft only after its exact-head run
passed. Commits after code head `3bd00d1` changed documentation only.

# Post-Merge Verification — Spec 084

```text
PR            = #147 (spec/084-hub -> main), merged 2026-09-25
MERGE_METHOD  = merge commit, --match-head-commit 1c98478
FINAL_HEAD    = 1c9847896aaeabda4b7cd9a227dd1559ec0210d8
EXACT_HEAD_CI = 36033616030 (pull_request, 6/6 success, bound to FINAL_HEAD)
CODE_HEAD_CI  = 36023022359 (pull_request, 6/6 success on 7858106)
MERGE_SHA     = 6caa698606a154d8d93b23529364a11c33cadd5c
POST_MAIN_CI  = 36193194092 (push on main, headSha 6caa698, 6/6 success)
BASE_POST_MAIN_CI = 36033406508 (push on 426bb34, Spec 083 closure, 6/6 success)
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
after completion. The PR base at merge time was `426bb34` (the PR #148
merge, Spec 083 closure). The PR left draft only after its exact-head run
passed. Commits after code head `7858106` changed documentation only.

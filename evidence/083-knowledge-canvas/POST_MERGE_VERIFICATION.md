# Post-Merge Verification — Spec 083

```text
PR            = #145 (spec/083-knowledge-canvas -> main), merged 2026-09-24
MERGE_METHOD  = merge commit, --match-head-commit 194840c
FINAL_HEAD    = 194840ce74bee64c033aa0da557e2086eab4b1a9
EXACT_HEAD_CI = 36004914365 (pull_request, 6/6 success, bound to FINAL_HEAD)
CODE_HEAD_CI  = 35997730588 (pull_request, 6/6 success on 8a05ccf)
MERGE_SHA     = 2892860e64ffed9ff9ae6387f681f1657730e19c
POST_MAIN_CI  = 36014350193 (push on main, headSha 2892860, 6/6 success)
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
after completion. The PR base at merge time was `f08f903` (the PR #144
merge), forward-merged into the branch as `8a05ccf`; the PR left draft only
after its exact-head run passed. The evidence head `194840c` changed no
code after `8a05ccf`.

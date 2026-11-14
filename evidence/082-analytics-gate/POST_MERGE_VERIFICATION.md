# Post-Merge Verification — Spec 082

```text
PR            = #142 (spec/082-analytics-gate -> main), merged 2026-09-24
MERGE_METHOD  = merge commit, --match-head-commit 7cc58ec
FINAL_HEAD    = 7cc58ec6e0fc437cc1f4df67c6c1423bc16fb0ee
EXACT_HEAD_CI = 35928650470 (pull_request, 6/6 success, bound to FINAL_HEAD)
CODE_HEAD_CI  = 35921559272 (pull_request, 6/6 success on 11150b5)
MERGE_SHA     = 3ea66109617e5fce70fd8a659f5e8657ec39bcb6
POST_MAIN_CI  = 35969739867 (push on main, headSha 3ea6610, 6/6 success)
```

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Verified with one-shot `gh run view 35969739867 --json headSha,event,status,conclusion,jobs`
after completion. The PR base at merge time was `44f71e6` (the Spec 081
closure merge), unchanged since the forward merge `965315e`; the PR left
draft only after its exact-head run passed.

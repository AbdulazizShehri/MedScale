# Post-Merge Verification — Spec 080

```text
PR            = #139 (spec/080-governed-browse -> main), merged 2026-09-23
MERGE_METHOD  = merge commit, --match-head-commit fe0a72e
FINAL_HEAD    = fe0a72eec406aabe764c48a93bab314d46d53c1c
EXACT_HEAD_CI = 35867289972 (pull_request, 6/6 success, bound to FINAL_HEAD)
CODE_HEAD_CI  = 35861492981 (pull_request, 6/6 success on 5daec97)
MERGE_SHA     = a8e32beb42a0687a1a7fafec83b775f26f0ed194
POST_MAIN_CI  = 35873449163 (push on main, headSha a8e32be, 6/6 success)
```

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Verified with one-shot `gh run view 35873449163 --json headSha,event,status,conclusion,jobs`
after completion; the PR base at merge time was `561f97f`, unchanged since
promotion.

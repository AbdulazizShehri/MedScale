# Post-Merge Verification — Spec 078

```text
PR               = #135 (spec/078-model-fleet-compare -> main), merged 2026-09-22
MERGE_METHOD     = merge commit (repository precedent), --match-head-commit bd06f6d
FINAL_HEAD       = bd06f6dbcf7d588aa1b0bf9eb2792ef52cab33b0
EXACT_HEAD_CI    = 35777530037 (pull_request, 6/6 success, bound to FINAL_HEAD)
MERGE_SHA        = bf400e8120270a144c86cc34ced1e8a88aa160a1
POST_MAIN_CI     = 35793707525 (push on main, headSha bf400e8, 6/6 success)
```

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Verified with one-shot `gh run view 35793707525 --json headSha,event,conclusion,jobs`
on 2026-09-22 after completion. `origin/main` was `bf400e8` when checked.

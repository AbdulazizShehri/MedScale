# Live truth — Spec 088 AudioFlow Advanced (huddle foundation)

Verified 2026-09-26 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = a40bd1efc05e355c6249f9f3452c953dde62bccc (PR #161 merge, Spec 087 closure)
CLOSURE_EXACT_HEAD= ac2f433 run 36268277485 (6/6 success)
DEPENDENCIES      = 081 + 084 (076 per roadmap; all closed)
OPEN_PRS          = #155 (this spec); later stacked drafts; #124, #125 (historical, out of scope)
STORAGE_SCHEMA    = v16 on main; this spec adds v17
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
ASR_ENGINE       = fixture engine only (Spec 081); real ASR/capture absent
REAL_PHI          = not authorized
```

The implementation was drafted before this base (stacked on the previous
spec's branch) and `main` was merged forward (no rebase). Draft CI runs
before promotion are historical; only exact-head CI on the final head
qualifies.

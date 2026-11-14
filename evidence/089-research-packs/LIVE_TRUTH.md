# Live truth — Spec 089 Research Packs (Clinical Research first)

Verified 2026-09-27 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = 1ac03622cb28046fc6386adab945578c82920f38 (PR #163 merge, Spec 088 closure)
CLOSURE_EXACT_HEAD= a47e65b run 36289075073 (6/6 success)
DEPENDENCIES      = 074 + 075 + 077 + 082 + 083 (all closed)
OPEN_PRS          = #156 (this spec); later stacked drafts; #124, #125 (historical, out of scope)
STORAGE_SCHEMA    = v17 on main; this spec adds v18
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
PACKS            = first-party only; no third-party Pack distribution
REAL_PHI          = not authorized
```

The implementation was drafted before this base (stacked on the previous
spec's branch) and `main` was merged forward (no rebase). Draft CI runs
before promotion are historical; only exact-head CI on the final head
qualifies.

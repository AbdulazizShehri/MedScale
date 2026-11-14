# Live truth — Spec 090 Institutional Adapters (object storage path)

Verified 2026-09-27 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = 8df3e45f1ae402f04a9c000c13ad4af466159b3a (PR #164 merge, Spec 089 closure)
CLOSURE_EXACT_HEAD= e2c2ea2 run 36305030636 (6/6 success)
DEPENDENCIES      = 075 + 079 + 084 + 085 + 089 (all closed)
OPEN_PRS          = #157 (this spec); later stacked drafts; #124, #125 (historical, out of scope)
STORAGE_SCHEMA    = v18 on main; this spec adds v19
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
NETWORK          = product egress default-deny; institutional transport unavailable
REAL_PHI          = not authorized
```

The implementation was drafted before this base (stacked on the previous
spec's branch) and `main` was merged forward (no rebase). Draft CI runs
before promotion are historical; only exact-head CI on the final head
qualifies.

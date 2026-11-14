# Live truth — Spec 091 Federation (bounded bundle exchange)

Verified 2026-09-27 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = ee7e1d5a77a39580b4314067e0557716cca93bfa (PR #165 merge, Spec 090 closure)
CLOSURE_EXACT_HEAD= 88139e0 run 36324093538 (6/6 success)
DEPENDENCIES      = 090 (closed)
OPEN_PRS          = #158 (this spec); later stacked drafts; #124, #125 (historical, out of scope)
STORAGE_SCHEMA    = v19 on main; this spec adds v20
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
FEDERATION       = out-of-band bundles only; no central service
REAL_PHI          = not authorized
```

The implementation was drafted before this base (stacked on the previous
spec's branch) and `main` was merged forward (no rebase). Draft CI runs
before promotion are historical; only exact-head CI on the final head
qualifies.

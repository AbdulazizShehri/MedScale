# Live truth — Spec 094 Research OS Desktop row actions (reversible slice)

Verified 2026-09-28 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = 4dfc8ada9d88df4fc818cc49b25f3f6ec278b1b2 (PR #169 merge, Spec 093 closure)
CLOSURE_EXACT_HEAD= 81364ae run 36406586207 (6/6 success)
DEPENDENCIES      = 093 (closed)
OPEN_PRS          = #167 (this spec); #124, #125 (historical, out of scope)
STORAGE_SCHEMA    = v20 on main; this spec adds no schema
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
SCOPE            = Desktop view-model actions + Slint row button; no storage, worker, transport or key code in Desktop
REAL_PHI          = not authorized
```

The implementation was drafted before this base (stacked on the previous
spec's branch) and `main` was merged forward (no rebase). Draft CI runs
before promotion are historical; only exact-head CI on the final head
qualifies.

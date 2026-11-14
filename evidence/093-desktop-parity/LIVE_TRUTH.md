# Live truth — Spec 093 Research OS Desktop parity (read-only slice)

Verified 2026-09-28 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = 41d2a9aad17d220ffa7798b84794913dbd4cb099 (PR #168 merge, Spec 092 closure)
CLOSURE_EXACT_HEAD= 3ae32d4 run 36375620139 (6/6 success)
DEPENDENCIES      = 084-091 (all closed) and the Spec 092 matrix row "Desktop parity: MISSING"
OPEN_PRS          = #162 (this spec); later stacked drafts; #124, #125 (historical, out of scope)
STORAGE_SCHEMA    = v20 on main; this spec adds no schema
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
SCOPE            = Desktop view-model + Slint route; no storage, worker, transport or key code in Desktop
REAL_PHI          = not authorized
```

The implementation was drafted before this base (stacked on the previous
spec's branch) and `main` was merged forward (no rebase). Draft CI runs
before promotion are historical; only exact-head CI on the final head
qualifies.

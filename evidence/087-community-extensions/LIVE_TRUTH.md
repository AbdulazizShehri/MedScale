# Live truth — Spec 087 Community Extensions

Verified 2026-09-26 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = 742a93a76fdb581e1aeb6d253e630015d9c409e9 (PR #154 merge, Spec 086 closure)
CLOSURE_EXACT_HEAD= 5afada8 run 36253473871 (6/6 success)
SPEC_086          = merged 16f2d1f (post-main 36246757563 6/6); closure merged 742a93a
DEPENDENCIES      = 075, 079, 084, 085 CLOSED_CANONICAL
OPEN_PRS          = #153 (this spec, draft); #124, #125 (historical drafts, out of scope)
STORAGE_SCHEMA    = v15 on main; this spec adds v16
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
WASM_RUNTIME      = none admitted
REAL_PHI          = not authorized
```

The implementation was written before this base and `main` was merged
forward (no rebase). Head `7175645` passed run `36246872221` (6/6) before
promotion; that run is historical. Only exact-head CI on the final head
qualifies.

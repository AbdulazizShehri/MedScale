# Live truth — Spec 086 R Workspace

Verified 2026-09-26 before promotion (one-shot `git fetch` and `gh`):

```text
MAIN              = 61855e4b33ec5691bf53a673c7f08e9f62117c69 (PR #151 merge, Spec 085 closure)
CLOSURE_EXACT_HEAD= 1f12c74 run 36233451671 (6/6 success)
SPEC_085          = merged 91021e2 (post-main 36208910691 6/6); closure merged 61855e4 (post-main 36237543003 6/6, verified after promotion)
DEPENDENCIES      = 075, 079, 082, 085 CLOSED_CANONICAL
OPEN_PRS          = #152 (this spec, draft); #124, #125 (historical drafts, out of scope)
STORAGE_SCHEMA    = v14 on main; this spec adds v15
SANDBOX           = every mechanism ReadyBaseMeasured; platform_qualified=false
REAL_PHI          = not authorized
```

The implementation commits `a80a30a` and `dc2fd47` were written on the
earlier base `91021e2`; `main` was merged forward (no rebase). CI on those
heads is historical; only exact-head CI on the final head qualifies.

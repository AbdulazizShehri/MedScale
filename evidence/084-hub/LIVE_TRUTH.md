# Live Truth — Spec 084 (T084-00)

Observed 2026-09-24 with `git fetch`, `gh pr list`, `gh pr view`, `gh run view`.

```text
BASE (origin/main) = 426bb3448d4e6e4b9c641fa256d6d2af530c2820
                     (merge of the Spec 083 closure PR #148)
SPEC 083           = PR #145 merged as 2892860 (exact-head run 36004914365
                     6/6; post-main run 36014350193 6/6); closure PR #148
                     exact-head run 36022980908 6/6, merged as 426bb34,
                     post-main run 36033406508
BRANCH             = spec/084-hub (PR #147, draft until qualified); started
                     from the Spec 083 branch while 083 closed, then
                     forward-merged with main (7858106 for 2892860, then
                     426bb34); no rebase, no force-push
OPEN PRS           = #124, #125 (drafts, unrelated; not touched)
STORAGE SCHEMA     = v12 on base; this spec adds v13
DEPENDENCIES       = none added (interprocess, ed25519-dalek, rand, sha2
                     already admitted)
NETWORK            = none; transport is in-process and the Spec 024 local
                     socket
REVIEW POLICY      = FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22 (no external
                     reviewer)
```

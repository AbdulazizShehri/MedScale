# Live Truth — Spec 080 (T080-00)

Observed 2026-09-23 with `git fetch`, `gh pr list`, `gh run view`.

```text
BASE (origin/main) = 561f97feabe6beb7f079e2af679a4119b50368ce
                     (merge of closure PR #138; Spec 079 CLOSED_CANONICAL)
BRANCH             = spec/080-governed-browse
OPEN PRS           = #124, #125 (drafts, unrelated); #139 (this spec)
STORAGE SCHEMA     = v8 on base; this spec adds v9
NETWORK POSTURE    = product runtime egress default-deny; the Spec 013 broker
                     live transport is hard-denied; ureq 3.4 is admitted
REVIEW POLICY      = FOUNDER_REVIEW_POLICY_AMENDMENT_2026-09-22
POST-MAIN RUN FOR BASE = 35819807088 (push on 561f97f, 6/6 success)
PROMOTION HEAD CI      = 35819965441 (pull_request on 231e0e1, 6/6 success)
```

Re-verified at qualification (2026-09-23): `origin/main` is still
`561f97f`; PR #139 is open, non-draft and mergeable with base `main`; #124
and #125 are unchanged drafts and are not touched by this spec.

# Ruleset Qualification — 2026-09-14

## Bound live state

```text
REPOSITORY = TheHalfMoon/MedScale
RULESET_ID = 23259329
RULESET_NAME = protect-main
ENFORCEMENT = active
TARGET = refs/heads/main
MAIN_PROTECTED = true
BYPASS_ACTORS = []
PR_ENFORCEMENT_PROBE = pull/96
PR_MERGE_STATE_WHILE_REQUIRED_CHECKS_PENDING = BLOCKED
RELEASE_READY = false
```

## Required checks

- `rust (ubuntu-latest)`
- `rust (windows-latest)`
- `rust (macos-latest)`
- `perf delivery-plan scale (windows)`
- `cargo-deny`
- `supply-chain policy present`

## Rules observed

- branch deletion blocked
- non-fast-forward updates blocked
- pull request required
- conversation resolution required
- no bypass actors
- the six checks above are required before merge

## Honesty limits

This evidence closes `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS` only. The ruleset currently
requires zero approving reviews and does not claim a human approval gate. It does not establish
package reproducibility, signing/notarization, performance-budget attainment, final-v0 WCAG
qualification, macOS product qualification, or `RELEASE_READY`.

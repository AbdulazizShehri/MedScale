# Spec 051 — REQUIRED_CHECKS live CI sync evidence

## Bound

```text
UNIT = SPEC_051_REQUIRED_CHECKS_LIVE_CI_SYNC
CLASS = EXECUTABLE_NOW (documentation / owner-packet honesty)
```

## Change

Refreshed `evidence/022-release-qualification-prep/REQUIRED_CHECKS.md` to include
live CI job `perf delivery-plan scale (windows)` (Spec 042) alongside the five
jobs listed in Spec 037.

## Honesty

```text
branch_protection_configured = false
REPO_BRANCH_PROTECTION_REQUIRED_CHECKS = NOT_CONFIGURED_OWNER_SETTINGS
RELEASE_READY = false
required_checks_packet_synced = true
```

Cursor did not mutate GitHub repository settings.

## Post-close external gate update (2026-09-14)

Founder-authorized GitHub ruleset `protect-main` (`id=23259329`) is active and targets
`refs/heads/main`. GitHub reports `main protected=true`; all six exact required checks
are configured; bypass actors are empty; deletion and non-fast-forward updates are
blocked; pull requests and conversation resolution are required. PR #96 was observed
`BLOCKED` while required checks were pending.

```text
branch_protection_configured = true
REPO_BRANCH_PROTECTION_REQUIRED_CHECKS = CONFIGURED
RELEASE_READY = false
```

The original Spec 051 closure-time statements above are retained as historical evidence.

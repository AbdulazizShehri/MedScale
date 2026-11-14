# PHI readiness checklist (engineering prep only)

**Gate:** `REAL_PHI_AUTHORIZATION` = `NOT_AUTHORIZED`  
**Rule:** Continue synthetic/owned fixtures only. This packet does **not** authorize real PHI.

## Purpose

When founder authorization arrives, engineering gaps below should already be inventoried so authorization does not uncover obvious product holes.

## Checklist (repository-owned status)

| Area | Status | Evidence / notes |
|---|---|---|
| Data-flow inventory (authority path only) | PARTIAL | One Rust authority facade; UI/CLI do not open canonical DB |
| Logging audit (no PHI in logs by design) | PARTIAL | Synthetic-only H0; doctor honesty; need release-bar log scrub proof later |
| Temporary file audit | PARTIAL | Vault work/WAL wipe READY_BASE (017/023); OS temp/pagefile residual open |
| Network audit | PARTIAL | Broker DEFAULT_DENY; live partners gated |
| Backup audit | PARTIAL | Synthetic vault backup/restore READY_BASE; encrypted sync READY_BASE |
| Export / disclosure audit | PARTIAL | Disclosure records + loss-aware FHIR export READY_BASE |
| Retention / deletion limitations | DOCUMENTED_OPEN | OS snapshots/swap may retain copies — blocks PRIVATE_DATA_READY |
| Threat model | PARTIAL | Planning docs + EXTERNAL_GATES; formal release threat model not claimed |
| Real PHI test fixtures | FORBIDDEN | Until gate closes |

## Exact external action (when ready)

```text
GATE = REAL_PHI_AUTHORIZATION
EXACT_EXTERNAL_ACTION = Founder issues explicit written authorization for real PHI scope (environments, retention, jurisdictions)
REQUIRED_INPUTS = scope statement; lawful basis / counsel mapping if required
EXPECTED_OUTPUT = REAL_PHI_AUTHORIZATION = AUTHORIZED_<scope>
HOW_CURSOR_WILL_VERIFY = EXTERNAL_GATES row update + doctor real_phi_authorized binding + scoped tests still refuse unauthorized paths
WHAT_UNIT_UNBLOCKS = later PHI-qualified evidence units only — not automatic RELEASE_READY
```

## Non-claims

- Not PRIVATE_DATA_READY
- Not RELEASE_READY
- Not permission to ingest real PHI in CI or local default workflows

# REQUIRED CHECKS — owner action packet (Spec 037; refreshed Spec 051)

**Gate:** `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS`  
**State:** `CONFIGURED_RULESET_23259329`  
**Live qualification:** founder-authorized ruleset configuration is now applied and evidenced below.

## Repository / branch

| Field | Value |
|---|---|
| REPOSITORY | `TheHalfMoon/MedScale` |
| BRANCH | `main` |
| WORKFLOW FILE | `.github/workflows/ci.yml` |
| PACKET_REFRESH | Spec 051 (`required_checks_packet_synced`) |

## Required check names (exact GitHub check run names)

Configure branch protection / rulesets so these checks are **required** before merge to `main`:

1. `rust (ubuntu-latest)`
2. `rust (windows-latest)`
3. `rust (macos-latest)`
4. `perf delivery-plan scale (windows)`
5. `cargo-deny`
6. `supply-chain policy present`

These names come from the `name:` fields on jobs in `ci.yml`
(`rust (${{ matrix.os }})`, `perf delivery-plan scale (windows)`, `cargo-deny`,
`supply-chain policy present`).

Spec **042** added the perf job; Spec **051** refreshes this packet so owner
instructions match live workflow names. OS privacy probes execute inside the
rust matrix tests and are **not** a separate required check name.

## Recommended protection configuration

| Setting | Recommended | Why |
|---|---|---|
| Require a pull request before merging | YES | Exact-head review + CI before main mutation |
| Required approving reviews | ≥1 (owner policy) | Human gate when owner requires it; Cursor cannot self-approve if required |
| Dismiss stale reviews when new commits are pushed | YES | Head must match reviewed tip |
| Require status checks to pass | YES | Prevent merge with failing matrix |
| Require branches to be up to date | YES (preferred) | Avoid merging stale green against drifted main |
| Require conversation resolution | YES (preferred) | Unresolved review threads block merge |
| Do not allow force pushes | YES | Git safety / no history rewrite on main |
| Do not allow deletions | YES | Protect main ref |
| Allow specified actors to bypass | NONE for ordinary work | No Cursor/API bypass of required checks |

## How to verify after configuration

```text
1. Open Settings → Branches / Rules → ruleset covering `main`
2. Confirm the six check names above are listed as required
3. Open a no-op draft PR → confirm checks appear with those exact names
4. Confirm merge is blocked while any required check is failing/pending
5. Confirm force-push to main is rejected
6. Record evidence: screenshot or API dump of ruleset JSON (owner-only)
```

## What Cursor already completed

- Locked multi-OS CI workflow with stable job names
- cargo-deny + supply-chain policy jobs
- Spec 042 perf delivery-plan scale Windows job
- Spec 022 RELEASE_READY honesty (`RELEASE_READY=false` until protection + other gates)
- Spec 037 inventory packet + Spec 051 live sync (this file)

## Exact external action

Owner (or GitHub org admin) configures branch protection / repository ruleset for `main` with the required check names above. No SPDX, signing, or MESC action is implied by this packet.

Live ruleset `protect-main` (`id=23259329`) now configures the six checks on `refs/heads/main`; this packet still does **not** claim `RELEASE_READY`.

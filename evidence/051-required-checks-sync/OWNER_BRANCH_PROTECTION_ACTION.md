# OWNER BRANCH PROTECTION — exact action packet (Spec 051)

**Gate:** `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS`  
**Current state:** `CONFIGURED_RULESET_23259329`  
**External authority note:** GitHub repository rulesets / branch protection are owner settings. Founder explicitly authorized completion of this action on 2026-09-14.

## What Cursor already completed

- Live CI workflow with stable job names (including Spec 042 perf job)
- Exact required-check inventory at `evidence/022-release-qualification-prep/REQUIRED_CHECKS.md` (Spec 051 synced)
- Doctor honesty after live qualification: `branch_protection_configured=true`, `required_checks_packet_synced=true`, `release_ready=false`

## Exact external action required

1. Open GitHub → `TheHalfMoon/MedScale` → Settings → Rules / Branches.
2. Create or edit a ruleset covering `main`.
3. Require status checks to pass before merging, listing **exactly**:
   - `rust (ubuntu-latest)`
   - `rust (windows-latest)`
   - `rust (macos-latest)`
   - `perf delivery-plan scale (windows)`
   - `cargo-deny`
   - `supply-chain policy present`
4. Prefer: require PR, dismiss stale reviews, require conversation resolution, block force-push/deletion, no bypass actors for ordinary work.
5. Save ruleset.

## Required input

Owner/org admin GitHub permission on the repository.

## Expected output

Ruleset JSON / UI showing the six check names required on `main`.

## Verification method

Open a draft PR; confirm all six checks appear with those exact names; confirm merge blocked while any is pending/failing; confirm force-push to `main` rejected.

## What becomes unblocked

Partial progress on RELEASE_READY checklist item for mandatory CI on protected main — **not** full RELEASE_READY (signing, MESC, SPDX, PRIVATE_DATA, WCAG, installers remain).

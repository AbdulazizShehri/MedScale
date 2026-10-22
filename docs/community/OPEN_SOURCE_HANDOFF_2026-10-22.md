# MedScale open-source stewardship and repository hygiene (2026-10-22)

## Verified baseline

- Official repository: `https://github.com/AbdulazizShehri/MedScale`
- The original repository was transferred without rebuilding its Git history; GitHub repository ID `1334230426` is unchanged.
- Ownership handoff base commit: `5c444a2e4e4e5041ab635c0e5edb28899bd11cbb`.
- PR #270 merged; pre- and post-merge CI, Tauri preview and Windows desktop E2E passed on the recorded base commit before transfer.
- Apache-2.0 was already the declared license for first-party source.
- The `protect-main` ruleset is active with no bypass actors, pull request review-thread resolution, deletion/force-push protection and required check names.
- Workflows are active; GitHub Actions repository-level permissions are read by default.
- No GitHub Release was published by this community documentation work.

These claims describe the observed 2026-10-22 state, not permanent CI or security guarantees.

## Community-facing changes

- Reorganized the root README around project purpose, verifiable preview status, onboarding, architecture, community participation, security and license.
- Corrected **active** default GitHub paths in mobile FFI metadata, the qualification-results CLI and Cursor repository permission boundaries; this is a canonical ownership migration, not a rewrite of historical source provenance.
- Added a root `CONTRIBUTING.md` accessible to external contributors, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `SUPPORT.md` and maintainer-led `GOVERNANCE.md`.
- Added issue forms for safe synthetic-data bug reporting, feature proposals and documentation improvements, plus a community-oriented PR template.
- Preserved existing technical docs, contracts, tests, versioned specs, evaluation evidence, frozen research records, provenance, notices and third-party source.
- Historical comparisons and upstream model identities remain factual; their original names are not first-party product branding.
- Separately retained maintainer/agent runbooks at `AGENTS.md`, `CURSOR.md`, `docs/engineering/` and `docs/planning/`; onboarding does not require studying the entire historical execution program.

## Intentional no-delete policy

This audit **did not delete** `specs/`, `evidence/`, `vendor/`, `third_party/`, `supply-chain/`, Rust crates or legacy UI because removing them without provenance, reproducibility and CI dependency analysis could break builds, historical test receipts, attribution or data/wire compatibility. Excess marketing/agent queue detail was removed from the *front-page presentation* rather than deleting its source of truth.

`.cursor/`, `.specify/` and `.impeccable/` remain as tooling/design context until individual references are audited. Their mere presence does not justify deletion.

No claims of clinical approval, production security, compliance certification, or completed community governance are made. There is no formal elected community body yet.

## Remaining follow-up

1. GitHub Discussions and private vulnerability reporting were both enabled and verified on 2026-10-22; re-check them after material repository setting changes.
2. Re-run the exact-head `main` status checks when this PR is merged; document the new SHA and installer hashes before any distribution.
3. Keep the repository description/topics current without making clinical claims.
4. Audited the 18 named MedScale Git clones under the Windows Work directory: 17 already used the new canonical `origin`, and the remaining one was updated without changing branches/files. No additional matching named checkout was found in the usual Mac project directories. External integrations and less obvious clones should still be checked if discovered; historical SHA-bound links are redirectable evidence and should not be retroactively falsified.
5. Label a few bounded Issues as `good first issue` / `help wanted` only when tasks have clear descriptions, scope, safety constraints and maintainers able to review them.
6. Consider cleanup of abandoned tooling or redundant generated artifacts **only after** source usage, license obligations and benchmark reproducibility checks prove each one unnecessary.

## Release boundary

GitHub hosting, contributor onboarding, public source visibility and passing CI do not convert MedScale into a clinically validated or signed product. Retain `REAL_PHI=NOT_AUTHORIZED`, `RELEASE_READY=false` and network default-deny until independently qualified gates close.

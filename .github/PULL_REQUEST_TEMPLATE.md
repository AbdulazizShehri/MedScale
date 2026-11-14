## Issue

Fixes #

<!-- If there is no issue, say why. For a change that is not a fix, use "Part of #" or "Related to #". -->

## What changes?

Describe the change in plain language.

## Root cause

What caused the problem? One or two sentences. Write "not a bug fix" if this adds or changes a feature.

## Regression test

Name the test that fails before this change and passes after it. If no test was added, say why.

- Test:

## Evidence

Link the runs. Leave a line empty and say so if a check did not run.

- CI run:
- Native QA run on the exact artifact (if the desktop app changes):
- Other evidence (screenshots with fictional data only, NVDA or VoiceOver notes):

## Desktop installer

- [ ] This PR does **not** change the desktop installer (no change to `apps/`, `crates/` or brand assets).
- [ ] This PR **does** change the desktop installer. The installer must be rebuilt and requalified before release.

## Release notes

One line a user would understand, or "none" for internal changes.

>

## Scope

- [ ] Focused change; unrelated refactors and generated churn excluded
- [ ] No real patient data, passphrases, recovery codes, vault files, secrets or confidential logs
- [ ] Existing Rust Core authority, local-first and default-deny safeguards preserved
- [ ] Wire/data compatibility preserved, or migration and tests provided
- [ ] Third-party source licenses, attribution and provenance preserved when applicable
- [ ] No claim of clinical validation, certification, signing or notarization that has not happened

## Verification

Give exact commands and results. Mark any check you did not run.

- [ ] `cargo fmt --all -- --check` (or not applicable)
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` (or not applicable)
- [ ] `cargo test --workspace --locked` (or not applicable)
- [ ] Desktop tests/build when affected (`npm test`, `npm run build`, native/E2E where applicable)
- [ ] Relevant documentation, `CHANGELOG.md` and synthetic regression fixtures updated

## Risks and limits

Note any effect on security, privacy, compatibility, performance, user experience or release gates, and anything not tested.

## References

- [Contributing](https://github.com/AbdulazizShehri/MedScale/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/AbdulazizShehri/MedScale/blob/main/SECURITY.md)
- [Beta maintenance](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/beta/MAINTENANCE.md)
- [Engineering-specific contributor guidance](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/engineering/CONTRIBUTING.md)

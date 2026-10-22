## What changes?

Describe the problem and the proposed solution in plain language. Link an Issue or Discussion if relevant.

## Scope

- [ ] Focused change; unrelated refactors and generated churn excluded
- [ ] No real patient data, secrets or confidential logs/fixtures
- [ ] Existing Rust Core authority, local-first and default-deny safeguards preserved
- [ ] Wire/data compatibility preserved, or migration and tests provided
- [ ] Third-party source licenses, attribution and provenance preserved when applicable

## Verification

Specify exact commands and results; leave unrun tests clearly identified.

- [ ] `cargo fmt --all -- --check` (or not applicable)
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` (or not applicable)
- [ ] `cargo test --workspace --locked` (or not applicable)
- [ ] Desktop tests/build when affected (`npm test`, `npm run build`, native/E2E where applicable)
- [ ] Relevant documentation and synthetic regression fixtures updated

## Risks, limitations and evidence

Note changes in security/privacy, compatibility, performance, user experience, release-gate requirements and areas not validated. Link verifiable synthetic test receipts when making qualification claims.

## References

- [Contributing](https://github.com/AbdulazizShehri/MedScale/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/AbdulazizShehri/MedScale/blob/main/SECURITY.md)
- [Engineering-specific contributor guidance](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/engineering/CONTRIBUTING.md)

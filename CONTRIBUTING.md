# Contributing to MedScale

Thank you for helping improve MedScale. This is an open-source, **synthetic-only engineering project**, not a clinical product or a patient-care service. We welcome contributions from engineers, clinicians with research experience, accessibility specialists, designers, documentation contributors and first-time open-source developers.

## Before you start

1. Read the [README](README.md), [Code of Conduct](CODE_OF_CONDUCT.md), [Security Policy](SECURITY.md) and [project governance](GOVERNANCE.md).
2. Check existing [Issues](https://github.com/AbdulazizShehri/MedScale/issues) and [Discussions](https://github.com/AbdulazizShehri/MedScale/discussions) to avoid duplicate work.
3. For significant architecture or behavior changes, open a proposal Issue or Discussion first. Small bug fixes, tests and documentation changes can go directly to a PR.
4. Keep every example, screenshot and test fixture **synthetic and de-identified by construction**. Do not contribute any real PHI, credentials, partner endpoints or leaked secrets. If you find such content, use [private reporting](SECURITY.md), not an Issue.

## Development

Use the pinned Rust toolchain and committed dependency locks.

```sh
git clone https://github.com/AbdulazizShehri/MedScale.git
cd MedScale
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Some platform dependencies differ: see [desktop setup](apps/desktop-tauri/README.md) and [existing CI jobs](.github/workflows/ci.yml). Desktop changes should also run `npm ci --ignore-scripts`, `npm test` and `npm run build` from `apps/desktop-tauri/`, followed by the relevant integration/desktop tests when possible. If a check cannot run locally, clearly state why; do not claim PASS without evidence.

## Contribution workflow

- Fork the repository or create a topic branch (for example, `fix/short-description` or `docs/short-description`).
- Make focused, reviewable changes; avoid unrelated reformatting and bulk rewrites.
- Add a regression test for behavior changes, and keep serialized contracts backward-compatible unless an explicit migration is reviewed.
- Include a concise PR description, linked Issue (if any), actual test results and the risks/limits of your change.
- Submit a pull request against `main`; required CI checks must pass. Maintainers may request changes, split large patches, or decline out-of-scope work.
- Do not force-push over someone else's work or bypass the active `protect-main` ruleset.

Contributions may be used and redistributed under the repository's Apache-2.0 terms unless explicitly agreed otherwise. **Only submit code or assets you own or are legally authorized to contribute.** External source, code, weights, licenses and model identities must retain proper attribution; see [source provenance records](third_party/provenance/) and [dependency admission](docs/engineering/DEPENDENCY_ADMISSION_TEMPLATE.md). The project's Apache license does not automatically apply to all third-party assets.

## Safety and technical boundaries

- The trusted Rust Core remains the authority; UI surfaces must not silently invent facts or bypass its validation.
- Default-deny network behavior, privacy constraints, auditability and uncertainty must be preserved.
- Never introduce live real-patient-data processing, production credentials, unrestricted automated medical actions or unapproved external uploads.
- No fabricated model ownership, clinical accuracy, regulatory approvals, benchmarks or supported-platform claims.
- Do not delete immutable research receipts, licenses, upstream notices, frozen specifications or migration evidence to make the repository appear cleaner.
- Security findings belong in the [private vulnerability channel](SECURITY.md).

## Where to help

Documentation, test coverage, first-run setup, reproducible benchmarks, safe synthetic FHIR interoperability, local model admission, accessibility and transparent error reporting are all useful areas. To align implementation with ongoing work, consult [open Issues](https://github.com/AbdulazizShehri/MedScale/issues) and [engineering guidelines](docs/engineering/CONTRIBUTING.md).

We value respectful, specific feedback, including reports that show limitations or regressions rather than successes.

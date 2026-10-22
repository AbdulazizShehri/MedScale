# MedScale

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/medscale-horizontal-white.svg">
  <img src="assets/brand/medscale-horizontal-black.svg" alt="MedScale" width="280">
</picture>

**Local-first clinical intelligence, built with verifiable evidence and explicit human oversight.**

[![CI](https://github.com/AbdulazizShehri/MedScale/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/AbdulazizShehri/MedScale/actions/workflows/ci.yml)
[![Desktop E2E](https://github.com/AbdulazizShehri/MedScale/actions/workflows/desktop-e2e.yml/badge.svg?branch=main)](https://github.com/AbdulazizShehri/MedScale/actions/workflows/desktop-e2e.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

MedScale is an open-source Rust-based platform for exploring evidence-backed health information in local workspaces. The trusted Rust Core owns records, provenance, validation, policy decisions and audit trails. The desktop and CLI present that Core's results rather than becoming independent sources of authority.

> [!IMPORTANT]
> **Status: engineering preview, synthetic data only.** MedScale is not clinically validated and is not a certified medical device, authorized patient-data processor, or signed production release. This repository is open for development and review only. Do not enter real patient data, identifying health records, credentials, or confidential materials. No medical diagnosis or treatment decisions should rely on this software. See [current external gates](docs/planning/EXTERNAL_GATES.md).

## Why MedScale?

- **Local first:** useful Core functionality does not require a cloud account; network actions are explicitly governed and default-denied.
- **Evidence before conclusions:** source identity, coverage, conflicts and review state remain visible; unsupported claims are not silently filled in.
- **Protected workspaces:** SQLCipher-encrypted vaults with explicit lock/unlock and recovery controls; this is not a claim of complete endpoint or private-data security.
- **Model transparency:** local model listings, downloads, admission and qualification are separate states; a catalog entry is not an endorsement.
- **Interoperability:** bounded, evidence-led FHIR R4 interchange rather than a claim of full FHIR conformance.
- **One trusted Core:** Rust interfaces shared by the CLI, the Windows/macOS Tauri desktop and engineering test surfaces.

## Project status

| Area | Current scope |
| --- | --- |
| Core and CLI | Rust; tested in CI on Windows, macOS and Linux |
| Desktop | Tauri 2 + React/TypeScript; unsigned engineering installers for Windows and macOS |
| Linux desktop | Not shipped; see the [GTK security/dependency gate](docs/planning/EXTERNAL_GATES.md) |
| Data | Synthetic fixtures only; real PHI **not authorized** |
| Release | Source available; production/clinical deployment **not qualified** |

See [the desktop guide](apps/desktop-tauri/README.md) and [external qualification gates](docs/planning/EXTERNAL_GATES.md). Historical engineering artifacts and comparison data are reproducible evidence, not marketing or clinical claims.

## Get started

**Requirements:** a Rust toolchain matching [`rust-toolchain.toml`](rust-toolchain.toml) and the platform build dependencies documented for the selected surface.

```sh
git clone https://github.com/AbdulazizShehri/MedScale.git
cd MedScale
cargo fmt --all -- --check
cargo run -p medscale-cli --locked -- --version
cargo run -p medscale-cli --locked -- doctor
```

For the desktop, follow [`apps/desktop-tauri/README.md`](apps/desktop-tauri/README.md). Windows desktop builds require Strawberry Perl for the vendored SQLCipher/OpenSSL dependency; unsigned installers are testing artifacts, not end-user releases.

## Architecture

| Path | Purpose |
| --- | --- |
| [`crates/`](crates/) | Trusted Core, storage, contracts, FHIR, local model packs and CLI |
| [`apps/desktop-tauri/`](apps/desktop-tauri/) | Windows/macOS desktop shell and frontend |
| [`fixtures/`](fixtures/) | Synthetic test data |
| [`docs/`](docs/) | Architecture, design, engineering and research documentation |
| [`specs/`](specs/) · [`evidence/`](evidence/) | Auditable technical decisions, tests and historical evidence |
| [`third_party/`](third_party/) · [`vendor/`](vendor/) | Pinned third-party resources and their licenses/provenance |

The older Slint desktop is preserved as an engineering reference, not the current consumer-facing desktop. See [the design system](DESIGN.md) for UI principles.

## Join the community

**Contributions are welcome:** issues, documentation, reproducible bug reports, tests, accessibility improvements, FHIR interoperability work and reviewed implementation changes.

- [Start contributing](CONTRIBUTING.md) · [Community governance](GOVERNANCE.md)
- [Issues](https://github.com/AbdulazizShehri/MedScale/issues) · [Discussions](https://github.com/AbdulazizShehri/MedScale/discussions)
- [Community standards](CODE_OF_CONDUCT.md) · [Getting help](SUPPORT.md)
- [Report a security vulnerability privately](SECURITY.md)

If you are new, start with a documentation change or a small synthetic-fixture bug. Check existing Issues and the [live engineering queue](docs/planning/BUILD_QUEUE.md) before proposing larger changes. No real patient records, even in screenshots or debugging logs.

## Security, attribution and license

MedScale first-party code is licensed under the **[Apache License 2.0](LICENSE)**. This does **not** relicense bundled third-party code, model weights, data sets, fonts, or research records; their original provenance, licenses and notices remain authoritative. Consult [source provenance](third_party/provenance/) and the repository's supply-chain records before redistributing a bundled asset.

For vulnerabilities use [private security reporting](SECURITY.md), **not public Issues**. For ordinary questions use [Discussions](https://github.com/AbdulazizShehri/MedScale/discussions).

## Working on MedScale (engineering and agents)

Maintainers and engineering agents: [`AGENTS.md`](AGENTS.md), [`CURSOR.md`](CURSOR.md), [engineering guide](docs/engineering/CONTRIBUTING.md) and [current project plan](docs/planning/START_HERE.md). These detailed execution records are preserved for continuity, but are not prerequisites for reading or joining the project.

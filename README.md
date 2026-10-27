# MedScale

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/medscale-horizontal-white.svg">
  <img src="assets/brand/medscale-horizontal-black.svg" alt="MedScale" width="280">
</picture>

**Free, local-first desktop software for audio, models, research and evidence-backed health work.**

[![CI](https://github.com/AbdulazizShehri/MedScale/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/AbdulazizShehri/MedScale/actions/workflows/ci.yml)
[![Desktop E2E](https://github.com/AbdulazizShehri/MedScale/actions/workflows/desktop-e2e.yml/badge.svg?branch=main)](https://github.com/AbdulazizShehri/MedScale/actions/workflows/desktop-e2e.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

MedScale is an open-source Rust-based platform for exploring evidence-backed health information in local workspaces. The trusted Rust Core owns records, provenance, validation, policy decisions and audit trails. The desktop and CLI present that Core's results rather than becoming independent sources of authority.

> [!NOTE]
> **Beta 0.1.0 is in final product QA.** Windows x64 and Apple Silicon engineering installers have passed source-bound native checks; an updated AudioFlow build is staged for private review. Public download links have not been switched to this build yet. MedScale can transcribe actual public, nonpatient speech locally with an approved Whisper model. See [feature evidence and outstanding qualification](docs/qualification/EXTERNAL_GATES.md) for supported environments and current release limits.

## Why MedScale?

- **Free and local first:** useful Core functionality does not require a cloud account; network actions are governed and default-denied unless explicitly approved.
- **Real offline transcription:** import supported PCM WAV audio, explicitly approve a pinned Whisper model download, verify its source and local Pack, then transcribe and review locally. English and Arabic selection is available for multilingual checkpoints; [measured quality differs by model](docs/qualification/EXTERNAL_GATES.md).
- **Evidence before conclusions:** source identity, coverage, conflicts and review state remain visible; unsupported claims are not silently filled in.
- **Protected workspaces:** SQLCipher-encrypted vaults with explicit lock/unlock and recovery controls; this is not a claim of complete endpoint or private-data security.
- **Model transparency:** local model listings, downloads, admission and qualification are separate states; a catalog entry is not an endorsement.
- **Interoperability:** bounded, evidence-led FHIR R4 interchange rather than a claim of full FHIR conformance.
- **One trusted Core:** Rust interfaces shared by the CLI, the Windows/macOS Tauri desktop and engineering test surfaces.

## Project status

| Area | Current scope |
| --- | --- |
| Core and CLI | Rust; tested in CI on Windows, macOS and Linux |
| Desktop | Tauri 2 + React/TypeScript; qualified unsigned engineering installers for Windows x64 and macOS Apple Silicon |
| Local audio | Real Windows user-driven Whisper transcription and restart verified; optional model acquisition with user consent, pinned sources and disk preflight |
| Linux desktop | Not shipped; Core/CLI tested on Linux |
| Data | Synthetic workspace fixtures and opted-in, licensed nonpatient speech; patient records and live clinical deployment are not qualified |
| Release | Source is available; updated Beta installer candidate is privately staged, **not newly published** |

See [the desktop guide](apps/desktop-tauri/README.md) and [external qualification gates](docs/qualification/EXTERNAL_GATES.md). Historical engineering artifacts and comparison data are reproducible evidence, not marketing or clinical claims.

## Get started

**Requirements:** a Rust toolchain matching [`rust-toolchain.toml`](rust-toolchain.toml) and the platform build dependencies documented for the selected surface.

```sh
git clone https://github.com/AbdulazizShehri/MedScale.git
cd MedScale
cargo fmt --all -- --check
cargo run -p medscale-cli --locked -- --version
cargo run -p medscale-cli --locked -- doctor
```

For the desktop, follow [`apps/desktop-tauri/README.md`](apps/desktop-tauri/README.md), including the model download and audio import guide. Windows native builds require Strawberry Perl for vendored SQLCipher/OpenSSL. The current unsigned Beta candidate is for private review; no new public installer download has been published.

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

## Real audio evidence

MedScale's native Windows AudioFlow has completed a real human-speech import, locally verified Whisper acquisition, transcription and restart journey on an isolated CI desktop ([Windows evidence](https://github.com/AbdulazizShehri/MedScale/actions/runs/40209018576)). Source-pinned Whisper downloads and signed model Packs passed a [four-model macOS verification matrix](https://github.com/AbdulazizShehri/MedScale/actions/runs/40216374598). Actual Arabic FLEURS evaluation of Tiny, Base and Small multilingual checkpoints is documented with measured WER and sample scope in the [qualification record](docs/qualification/EXTERNAL_GATES.md). These public-read-speech evaluations do not measure clinical accuracy. Model weights are not bundled with the installers; downloading is an explicit opt-in action.

## Join the community

**Contributions are welcome:** issues, documentation, reproducible bug reports, tests, accessibility improvements, FHIR interoperability work and reviewed implementation changes.

- [Start contributing](CONTRIBUTING.md) · [Community governance](GOVERNANCE.md)
- [Issues](https://github.com/AbdulazizShehri/MedScale/issues) · [Discussions](https://github.com/AbdulazizShehri/MedScale/discussions)
- [Community standards](CODE_OF_CONDUCT.md) · [Getting help](SUPPORT.md)
- [Report a security vulnerability privately](SECURITY.md)

If you are new, start with a documentation change or a small synthetic-fixture bug. Check [open Issues](https://github.com/AbdulazizShehri/MedScale/issues) before proposing larger changes. No real patient records, even in screenshots or debugging logs.

## Security, attribution and license

MedScale first-party code is licensed under the **[Apache License 2.0](LICENSE)**. This does **not** relicense bundled third-party code, model weights, data sets, fonts, or research records; their original provenance, licenses and notices remain authoritative. Consult [source provenance](third_party/provenance/) and the repository's supply-chain records before redistributing a bundled asset.

For vulnerabilities use [private security reporting](SECURITY.md), **not public Issues**. For ordinary questions use [Discussions](https://github.com/AbdulazizShehri/MedScale/discussions).

## Maintainer and contributor guidance

The [contribution guide](CONTRIBUTING.md), [engineering guide](docs/engineering/CONTRIBUTING.md), [public qualification evidence](docs/qualification/), and GitHub Issues provide the shared context needed to contribute. Internal execution plans and editor-specific agent instructions are not part of the public project API.

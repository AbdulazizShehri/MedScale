---
title: MedScale
emoji: 🩺
colorFrom: gray
colorTo: blue
sdk: static
pinned: false
---

<div align="center">
  <a href="https://medscale.si">
    <img src="https://raw.githubusercontent.com/AbdulazizShehri/MedScale/main/web/public/og.png" width="780" alt="MedScale — local-first, evidence-grounded health AI" />
  </a>
  <h1>MedScale</h1>
  <p><strong>Evidence first. Action second.</strong></p>
  <p>Open-source, local-first clinical intelligence infrastructure.<br />One inspectable workspace for health records, evidence, local AI models and human-governed workflows.</p>
  <p>
    <a href="https://medscale.si"><strong>Website</strong></a>
    &nbsp;·&nbsp;
    <a href="https://medscale.si/download/"><strong>Beta release status</strong></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/AbdulazizShehri/MedScale"><strong>Source code</strong></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/AbdulazizShehri/MedScale/discussions"><strong>Community</strong></a>
  </p>
</div>

> **MedScale Beta · Active development · Synthetic data only.** No public desktop installer has been authorized for distribution. Current engineering installers are unsigned/unnotarized private Beta test artifacts, not clinically validated releases. **Never enter real patient records, personal health information, sensitive credentials or confidential materials.** Do not use MedScale for diagnosis, treatment decisions or production clinical care. [Current qualification boundaries](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/qualification/EXTERNAL_GATES.md).

---

## What is MedScale?

MedScale is a **local-first healthcare intelligence workspace** built to connect source-aware health information with explicit human decision-making. It combines a Rust Core, desktop application and command-line interface so that records, evidence, models and audits can be inspected through one consistent authority path.

The goal is not to let AI silently make medical decisions. It is to make clinical and research information **understandable, verifiable and traceable**, while keeping data movement and consequential actions explicit.

### What makes MedScale different?

| Principle | How MedScale approaches it |
| :--- | :--- |
| **Local first** | The Rust Core executes locally; outbound network actions are governed and default-denied. |
| **Evidence-grounded** | Summaries and insights expose sources, coverage, conflicts and unknowns instead of filling gaps silently. |
| **One trusted Core** | The desktop interface and scriptable CLI share the same underlying data, validation and policy model. |
| **Human oversight** | Model and agent output is treated as a proposal; authority-changing actions require explicit review and permission. |
| **Inspectability** | Model metadata, provenance, auditing, export boundaries and reproducible engineering evidence are visible concepts. |
| **Open development** | Public source, community issues, contributed tests and transparent limitations. First-party source is Apache-2.0. |

> **Important:** These are architectural goals and implemented engineering behaviors in an evolving beta. They do **not** establish validated end-to-end clinical privacy, medical safety, performance, regulatory status or universal model accuracy.

## Try MedScale Beta

MedScale desktop installers have been built and tested for Windows x64 and macOS Apple Silicon, but **public installer distribution is not yet authorized**. Existing unsigned/unnotarized engineering artifacts are reserved for private synthetic-only qualification. The planned direct-download approach does not require an app-store account or a paid developer subscription, but this is not a public download announcement.

| Platform | Availability | Get started |
| :--- | :--- | :--- |
| **Windows x64** | Private unsigned NSIS engineering artifact; public release not yet authorized | [Check release status](https://medscale.si/download/) |
| **macOS Apple Silicon** | Private unsigned/unnotarized DMG engineering artifact; public release not yet authorized | [Check release status](https://medscale.si/download/) |
| **Linux** | Rust Core and CLI; desktop not released | [CLI documentation](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/product/CLI_PRODUCT_GUIDE.md) |

**Release resources:** [Engineering qualification status](https://github.com/AbdulazizShehri/MedScale/issues/297) · [Desktop installation and prerequisites](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/BETA_FIRST_RUN.md). A public GitHub Beta Release, public release checksums and general download links will be announced **only after explicit authorization**.

Authorized private testers must verify source-bound SHA-256 checksums before using a provided engineering installer. A download is **not** proof of a completed installation. Windows may show SmartScreen or Smart App Control warnings; macOS may block unnotarized software through Gatekeeper. Never globally disable operating-system security to run Beta software.

<div align="center">
  <a href="https://medscale.si">
    <img src="https://raw.githubusercontent.com/AbdulazizShehri/MedScale/main/web/public/shots/insights-hero-dark.webp" width="780" alt="Screenshot of MedScale Beta insights using synthetic health data" />
  </a>
  <br />
  <sub>Real desktop screenshot from a synthetic-only test workspace. No real patient information.</sub>
</div>

## Explore the product

MedScale Beta provides desktop routes and Core-backed engineering workflows across these areas. Their presence in the beta does not imply production qualification or a guarantee that every external integration is available.

| Area | Product surfaces |
| :--- | :--- |
| **Clinical information** | Patients, documents, timelines, evidence, insights, coverage and source-aware summaries |
| **Research and data** | Projects, synthetic data import, bounded browsing, analytics, knowledge search and Research OS |
| **Local intelligence** | Model catalog, model runtime information, governed model packs, MedAgent and Model Fleet |
| **Operations** | Workflows, tasks, messages, collaboration and audio test flows |
| **Governance** | Privacy, audit trail, FHIR export, integrations, settings and release information |
| **Developer tools** | CLI commands, reproducible fixtures, real Core tests and developer documentation |

### A single trusted authority path

```text
Windows / macOS Desktop       Scriptable CLI
           │                       │
           └──────────┬────────────┘
                      ▼
               MedScale Rust Core
       ┌──────────────────────────────┐
       │ Records • validation • policy│
       │ Evidence • provenance • audit│
       │ Vault • models • FHIR bounds │
       └──────────────────────────────┘
                      │
             Governed, local actions
```

The interface displays Core results instead of becoming a separate source of authority. A model listing is not a recommendation, a generated statement is not automatically verified, and an unknown outcome is not permission to repeat an action.

### Privacy and interoperability

- **Workspaces:** synthetic local workspaces and encrypted-vault engineering capabilities. Vault encryption is **not** a claim of full endpoint security or production privacy qualification.
- **Data flow:** network access is default-denied, and data export remains a separately governed action.
- **FHIR:** evidence-led, bounded **FHIR R4** ingest/export support; **full FHIR conformance is not claimed**.
- **Traceability:** source identity, coverage, audit records and review state are preserved in Core workflows.
- **Human control:** proposals and decisions are distinguished so high-impact actions are not silently executed.

## MedScale on Hugging Face

Hugging Face is a home for **reproducible model and dataset artifacts associated with MedScale**, not an alternative clinical authority system. We intend to make each released artifact understandable and independently assessable.

**Model cards** should clearly identify the task, model origins and base checkpoints, weights and format, upstream terms, training data sources, model provenance, evaluation setup and limitations. **Dataset cards** should state their source, permitted use, synthetic status (if applicable), licenses, validation, known gaps and leakage safeguards.

Our current policy is deliberately conservative:

- Listing a model in the MedScale catalog does **not** mean it has passed clinical or product qualification.
- No claimed benchmark superiority without a reproducible, public evaluation with complete methodology.
- Do not publish real patient data, private clinical records or identifying health data.
- No gated checkpoint access, online model-pack distribution or external partner access is implied without the necessary rights and qualification.
- Model and dataset rights are **artifact-specific**: Apache-2.0 for MedScale first-party code does not automatically cover third-party weights or datasets.

Use the **Models**, **Datasets**, **Spaces** and **Collections** tabs above for any artifacts actually published under this organization. An empty category is not a claim of a released model or dataset.

### How to evaluate work from this organization

For any model or dataset, look for:

1. **Provenance:** the exact checkpoint/dataset version, source and relevant licenses.
2. **Purpose:** a defined task, inputs, outputs and explicitly excluded uses.
3. **Evaluation:** sample construction, baselines, metrics, limitations and reproducible code where available.
4. **Safety and privacy:** disclosure of failure modes, data restrictions and release qualification.
5. **Versioning:** commit revisions, changelogs, known regressions and rollback guidance.

Experimental or synthetic benchmarks must not be represented as demonstrated medical performance.

## Build from source

MedScale's public reference implementation is on GitHub.

```bash
git clone https://github.com/AbdulazizShehri/MedScale.git
cd MedScale
cargo fmt --all -- --check
cargo run -p medscale-cli --locked -- --version
cargo run -p medscale-cli --locked -- doctor
```

The desktop also requires Node.js and the platform-specific Tauri build prerequisites. For desktop development:

```bash
cd apps/desktop-tauri
npm ci --ignore-scripts
npm test
npm run build
npm run tauri -- dev
```

Follow the [desktop build guide](https://github.com/AbdulazizShehri/MedScale/blob/main/apps/desktop-tauri/README.md) for the Rust toolchain, native dependencies and Windows-specific Strawberry Perl / SQLCipher requirements.

### Read the project

- [Project overview and architecture](https://github.com/AbdulazizShehri/MedScale/blob/main/README.md)
- [Product principles and intended scope](https://github.com/AbdulazizShehri/MedScale/blob/main/PRODUCT.md)
- [Desktop guide](https://github.com/AbdulazizShehri/MedScale/blob/main/apps/desktop-tauri/README.md)
- [CLI guide](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/product/CLI_PRODUCT_GUIDE.md)
- [Qualification, privacy and external release gates](https://github.com/AbdulazizShehri/MedScale/blob/main/docs/qualification/EXTERNAL_GATES.md)
- [Source provenance and third-party materials](https://github.com/AbdulazizShehri/MedScale/tree/main/third_party/provenance)
- [CI and engineering tests](https://github.com/AbdulazizShehri/MedScale/actions)

## Current status and development direction

| Topic | Present status |
| :--- | :--- |
| Project phase | **Beta — active development** |
| End-user desktop | Windows x64 and macOS Apple Silicon private unsigned/unnotarized engineering builds; no public distribution authorized |
| Rust Core and CLI | Open-source and exercised through CI |
| Real patient data | **Not authorized** |
| Clinical decision-making | **Not validated / not authorized** |
| Production release and notarization | **Not qualified** |
| Final accessibility and performance measurements | External qualification still required |
| Live EHR, SMART and partner connections | No live authorization |
| Linux desktop | Deferred; Linux Core/CLI are separate |
| Published models and datasets | See this organization's live repository tabs; no implied release count |

Current priorities include improving desktop beta quality and installation, first-run experience, transparent model and evidence workflows, security/privacy testing, cross-platform verification, accessibility and reproducible research. These are ongoing efforts, **not promised dates or guaranteed clinical capabilities**.

## Join the community

MedScale is **maintainer-led and open to community contributions**. We welcome engineers, clinicians working with synthetic or permitted research data, health data scientists, designers, accessibility specialists, documentation writers and first-time contributors.

- [Contributing guide](https://github.com/AbdulazizShehri/MedScale/blob/main/CONTRIBUTING.md)
- [Community discussions](https://github.com/AbdulazizShehri/MedScale/discussions)
- [Open issues and bug reports](https://github.com/AbdulazizShehri/MedScale/issues)
- [Community governance](https://github.com/AbdulazizShehri/MedScale/blob/main/GOVERNANCE.md)
- [Code of conduct](https://github.com/AbdulazizShehri/MedScale/blob/main/CODE_OF_CONDUCT.md)
- [Report security vulnerabilities privately](https://github.com/AbdulazizShehri/MedScale/security/advisories/new)

Never include real patient records, proprietary partner information, credentials or exploit details in public reports or examples. Use synthetic reproductions; report vulnerabilities privately.

## License, attribution and responsible use

MedScale's first-party source is available under **Apache License 2.0**. Bundled dependencies, datasets, pretrained checkpoints, terminology resources, fonts and other third-party materials retain their **own** license and attribution requirements. Consult the relevant licenses, model/dataset cards and [repository notices](https://github.com/AbdulazizShehri/MedScale/tree/main/third_party) before reuse.

This is **research and engineering beta software**, not a certified medical device, clinical decision-support service, diagnostic product, treatment tool or authorization to process real patient information. Please use only synthetic test inputs and inspect each release's documented limitations.

---

<div align="center">
  <strong>MedScale — Evidence first. Action second.</strong><br />
  <a href="https://medscale.si">medscale.si</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/AbdulazizShehri/MedScale">GitHub</a>
  &nbsp;·&nbsp;
  <a href="https://medscale.si/download/">Beta release status</a>
</div>

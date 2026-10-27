# MedScale public qualification gates

MedScale is a free, local-first open-source Beta project with real, user-approved local Whisper transcription for nonpatient recordings and synthetic health-workspace fixtures. Public installer cutover, real patient record handling and clinical deployment remain separate qualification decisions. This page documents material boundaries and evidence without treating engineering tests as medical certification.

**Status: `RELEASE_READY = false` · `REAL_PHI_AUTHORIZATION = NOT_AUTHORIZED`.**

| Qualification boundary | Current public status | What it means |
| --- | --- | --- |
| `REAL_PHI_AUTHORIZATION` | **NOT_AUTHORIZED** | Synthetic workspaces and explicitly chosen licensed **nonpatient** audio are testable. Real patient data may not be entered, logged or processed under current qualification. |
| `LEGAL_COUNSEL_FLOW_MAPPING` | PENDING_WHEN_REQUIRED | Deployment-specific privacy, healthcare, controller/processor and regulatory reviews are not complete. |
| `PRIVATE_DATA_READY` | NOT_QUALIFIED | Existing vault/security features do not establish production privacy across OS swap, snapshots and other endpoints. |
| `DESKTOP_RELEASE_SIGNING_PROVENANCE` | NOT_GRANTED | Production signing and independently verifiable release provenance are not established. |
| `QUALIFIED_RELEASE_PERFORMANCE_HARDWARE` | NOT_PROVISIONED | Hosted CI is not evidence of final release performance on qualified devices. |
| `MACOS_SIGNED_PRODUCT_QUALIFICATION` | NOT_GRANTED | Signed/notarized macOS and sandbox enforcement qualification remain open. |
| `FINAL_V0_UI_ACCESSIBILITY_QUALIFICATION` | EXTERNAL_MEASUREMENT_REQUIRED | Final keyboard, assistive-technology, text scaling and contrast checks remain open. |
| `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` | OPEN | WebView storage, logs, cache and other private-data surfaces require end-to-end proof. |
| `TAURI_GLIB_ADVISORY_F096_T01` | RESOLVED_BY_PLATFORM_SCOPE | The desktop targets Windows/macOS; this does not qualify or authorize a Linux Tauri desktop. |
| `TAURI_LINUX_DESKTOP_GTK3` | OPEN_UPSTREAM | Linux desktop packaging is deferred. Linux Rust Core/CLI testing is a separate supported surface. |
| `WORKER_OS_SANDBOX_PLATFORM_QUALIFIED` | OPEN | Worker confinement and platform-specific enforcement are not fully qualified across supported operating systems. |
| `PARTNER_EHR_NPHIES_ENDPOINT` | NOT_GRANTED | No live EHR/NPHIES/partner endpoint is authorized. Synthetic stubs only. |
| `SMART_LIVE_PARTNER_AUTHORIZATION` | NOT_GRANTED | No live SMART partner authorization or token exchange is authorized. |
| `PRODUCTION_CREDENTIALS` | NOT_GRANTED | No production keys, service credentials or external access are supplied. |
| `GATED_MODEL_TERMS` | NOT_GRANTED_BY_DEFAULT | Do not accept provider terms on behalf of users or automatically acquire restricted assets. |
| `HF_ONLINE_PACK_DISTRIBUTION` | NOT_GRANTED | MedScale does not publish hosted model Packs. Separately, the user-consented download of allowlisted, pinned **public Whisper checkpoints** is implemented and verified; arbitrary or gated model acquisition remains unsupported. |
| `TERMINOLOGY_SNOMED_LICENSE` | OPEN | Distribution of licensed terminology requires independent rights verification. |
| `TERMINOLOGY_LOINC_LICENSE` | OPEN | Distribution requires rights verification. |
| `TERMINOLOGY_ICD_LICENSE` | OPEN | Distribution requires rights verification. |
| `TERMINOLOGY_ATC_LICENSE` | OPEN | Distribution requires rights verification. |
| `TERMINOLOGY_UMLS_LICENSE` | OPEN | Distribution requires rights verification. |
| `PUBLIC_SOURCE_LICENSE_CHOICE` | DECIDED | MedScale first-party source is Apache-2.0; bundled dependencies/assets retain their own notices and license obligations. |
| `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS` | CONFIGURED | Protected `main` requires GitHub check qualification and normal PR review. This is not release approval. |

## Verified AudioFlow scope

- **Windows real end-to-end desktop:** a user-driven import, actual nonfixture Whisper transcription and restart on licensed human LibriSpeech audio passed [dedicated GitHub Actions](https://github.com/AbdulazizShehri/MedScale/actions/runs/40209018576). Latest AudioFlow integration [PR #328](https://github.com/AbdulazizShehri/MedScale/pull/328) merged into `main` after exact-head Rust, native Tauri, desktop E2E and browser smoke passed.
- **Real source-bound models:** pinned Whisper Tiny English, Tiny Multilingual, Base Multilingual and Small Multilingual passed [download/hash/signed-Pack qualification](https://github.com/AbdulazizShehri/MedScale/actions/runs/40216374598) on ephemeral macOS CI. Model weights are not in the desktop installer.
- **Actual English benchmark:** on 100 licensed LibriSpeech test-clean excerpts (40 speakers, 707.7 seconds), Whisper Tiny English's measured corpus WER was **5.58%** in an independent [real-data evaluation job](https://github.com/AbdulazizShehri/MedScale/actions/runs/40114179864).
- **Actual Arabic benchmark:** on 24 FLEURS `ar_eg` read-speech excerpts (298 seconds), corpus WER was **65.28% Tiny**, **46.15% Base**, **25.78% Small** (separately evaluated on the MedScale ASR runtime: [Tiny](https://github.com/AbdulazizShehri/MedScale/actions/runs/40118126000), [Base](https://github.com/AbdulazizShehri/MedScale/actions/runs/40118134257), [Small](https://github.com/AbdulazizShehri/MedScale/actions/runs/40119541369)). The dataset is Egyptian Arabic read speech, not a measurement of all Arabic accents.
- **Private installer candidate:** Windows x64 and macOS ARM64 unsigned packages sourced from the same tree as merged `main@75cef1f` passed native installer checks, independent full manifest and SHA256SUMS verification. See [Beta QA issue #302](https://github.com/AbdulazizShehri/MedScale/issues/302). No new public installer, website cutover, model weights or GitHub Release was published.

These measured results are specific to public read-speech material and those CI environments. They do not establish clinical performance, all-hardware performance, macOS real-speech GUI acceptance, or patient-data authorization.

## Interpreting engineering evidence

Historical Spec 072 engineering coverage, **including Models/Evidence**, documents available test work; it does not satisfy the external qualifications above or certify a product. Individual CI successes cannot close clinical, privacy, performance or signing gates.

Use the [public project documentation](../../README.md), [security policy](../../SECURITY.md) and [source provenance](../../third_party/provenance/) for reviewable technical details. Report security vulnerabilities privately through the [security policy](../../SECURITY.md), not in public issues.

Maintainer-specific execution schedules, workstation incidents, provider accounts, unpublished strategy, and internal decisions are deliberately not published here. This file is not a release checklist approval. No GitHub Release is authorized by its contents.

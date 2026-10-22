# MedScale public qualification gates

MedScale is open-source engineering software for **synthetic data only**. It is **not** approved for clinical decision-making, real patient records, deployment to live healthcare services, or a signed production release. This is a public summary of qualification boundaries, not a confidential implementation plan or a claim of regulatory certification.

**Status: `RELEASE_READY = false` · `REAL_PHI_AUTHORIZATION = NOT_AUTHORIZED`.**

| Qualification boundary | Current public status | What it means |
| --- | --- | --- |
| `REAL_PHI_AUTHORIZATION` | **NOT_AUTHORIZED** | Only synthetic fixtures are permitted. No real patient data may be entered, logged or processed. |
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
| `HF_ONLINE_PACK_DISTRIBUTION` | NOT_GRANTED | No public online model-pack distribution or remote pack acquisition is qualified. |
| `TERMINOLOGY_SNOMED_LICENSE` | OPEN | Distribution of licensed terminology requires independent rights verification. |
| `TERMINOLOGY_LOINC_LICENSE` | OPEN | Distribution requires rights verification. |
| `TERMINOLOGY_ICD_LICENSE` | OPEN | Distribution requires rights verification. |
| `TERMINOLOGY_ATC_LICENSE` | OPEN | Distribution requires rights verification. |
| `TERMINOLOGY_UMLS_LICENSE` | OPEN | Distribution requires rights verification. |
| `PUBLIC_SOURCE_LICENSE_CHOICE` | DECIDED | MedScale first-party source is Apache-2.0; bundled dependencies/assets retain their own notices and license obligations. |
| `REPO_BRANCH_PROTECTION_REQUIRED_CHECKS` | CONFIGURED | Protected `main` requires GitHub check qualification and normal PR review. This is not release approval. |

## Interpreting engineering evidence

Historical Spec 072 engineering coverage, **including Models/Evidence**, documents available test work; it does not satisfy the external qualifications above or certify a product. Individual CI successes cannot close clinical, privacy, performance or signing gates.

Use the [public project documentation](../../README.md), [security policy](../../SECURITY.md) and [source provenance](../../third_party/provenance/) for reviewable technical details. Report security vulnerabilities privately through the [security policy](../../SECURITY.md), not in public issues.

Maintainer-specific execution schedules, workstation incidents, provider accounts, unpublished strategy, and internal decisions are deliberately not published here. This file is not a release checklist approval. No GitHub Release is authorized by its contents.

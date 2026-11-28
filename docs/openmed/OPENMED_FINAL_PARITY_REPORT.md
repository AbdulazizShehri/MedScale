# OpenMed v4.0.0 Full Capability Parity & Source Transplant Final Report

**Date:** 2026-11-28  
**Repository:** `https://github.com/AbdulazizShehri/MedScale`  
**Website:** `https://medscale.si` (download manifest inactive)  
**Baseline Anchor:** `main@d36ada66c6fc6a7c729dfb51d66100d6edb5f3e1`  
**Post-Wave-3 Verified `main`:** `9df24f32c6166dc71f4a8dda0d37516061f424cb`  
**License:** Apache-2.0 for MedScale first-party code; third-party notices and attributions preserved verbatim  

---

## 1. Executive Summary & Upstream Pins

Across **Wave 1** ([PR #537](https://github.com/AbdulazizShehri/MedScale/pull/537) -> `ef3e4fce`), **Wave 2** ([PR #538](https://github.com/AbdulazizShehri/MedScale/pull/538) -> `d1cd3bd0`), **Wave 3** ([PR #539](https://github.com/AbdulazizShehri/MedScale/pull/539) -> `9df24f32`), and **Wave 4** (`feat/openmed-v4-qualification-and-docs`), MedScale achieved end-to-end functional capability parity with **OpenMed `v4.0.0`** and the open-source **`openmed-agent` v1.0.3** governance surface while keeping MedScale's native Rust architecture, fail-closed authority boundaries, and zero-mandatory-account local-first product identity intact.

| Pinned Source | Commit / Tag | Disposition in MedScale |
|---|---|---|
| `https://github.com/maziyarpanahi/openmed` | `v4.0.0` (`5b79cae5df6bc22b0fcdc799be480ee4ffc23c57`) | Authorized Apache-2.0 source transplant (`integrations/clinical-evidence-python`, `integrations/medscale-sdk-ts`) + clean-room native Rust ports across `crates/medscale-{contracts,pack,clinical,fhir,core,cli}` & Desktop UI |
| `https://github.com/openmed-labs/openmed-agent` | `v1.0.3` (`ded6e57b5b93cb6dc849c8639662b381179697c7`) | Provenance pinned; open offline clinical agent governance specs (`openmed/agent/` in `5b79cae5`) ported cleanly into `medscale-clinical`, Core `MedAgent`, CLI `med-agent`, and Desktop UI |
| Immutable `models.jsonl` catalog snapshot | `ea920f36` (byte-identical to `v4.0.0`) | `third_party/medscale-model-catalog-v3/models.jsonl` (`sha256:805e79f7db5514cf308c6deffa67d72c4ce9a59bf4e54487d2783eecbb1e34ab`, `2,266` rows, `3,057,702` bytes) preserved byte-for-byte along with `LICENSE` and `NOTICE`; HuggingFace `OpenMed/...` `repo_id` lookup strings preserved read-only |
| Historical v3 Python snapshot | `3e2323288ec551157c0b943b331f399ef1434649` | Preserved in provenance (`third_party/provenance/openmed-clinical-python-2026-11-08.md`); reference package upgraded to `v0.4.0` (`406` stdlib `.py` modules) |

**Cumulative Engineering Footprint (`main@d36ada66..main@9df24f32`, Waves 1–3):**
- **671 files changed**, **+331,793 / -167 lines** across workspace Rust crates, CLI, Tauri Desktop backend & React frontend, TypeScript SDK, stdlib Python reference package, conformance vectors, and provenance docs.
- **Zero new external Cargo crates introduced** (`Cargo.lock` only updated internal workspace member edges to add `serde = { workspace = true }` on `medscale-clinical` and `medscale-fhir`), preserving the exact audited 11-crate workspace topology, `unsafe_code = "deny"`, `clippy::all = "deny"`, DAG enforcement (`scripts/check-dependency-direction.ps1`), and 26 Desktop routes (`apps/desktop-tauri/src/routes.ts`).

---

## 2. Issue `#183` Reconciliation & Model Ecosystem Parity

Issue `#183` required truthful separation of **catalog listing** from **downloaded**, **verified**, **admitted**, **runtime-compatible**, **executed-tested**, and **clinically validated** model states, without ever claiming that all `2,266` catalog entries are runnable or clinically validated. MedScale reconciles Issue `#183` completely in Wave 1 ([PR #537](https://github.com/AbdulazizShehri/MedScale/pull/537)):

- Full 21+ field catalog parsing (`crates/medscale-pack/src/catalog.rs`, `medscale-contracts/src/model_catalog.rs`) preserving `arxiv`, `benchmark`, `canonical_labels`, `download_mb`, `download_sizes`, `peak_ram_mb`, `recommended_tier`, `latency_ms`, `script_coverage`, `script_eval`, `training_data_licenses`, and `training_provenance`.
- Hardware-tier model recommendation across `phone:6000 / laptop:16000 / workstation:64000 / server:512000` MB (`crates/medscale-pack/src/ecosystem_v4.rs`), catalog `manifest-diff`, and markdown `model-card` generation surfaced via `medscale-cli` (`packs recommend/card/manifest-diff/evaluate-batch`) and the Desktop `Models` UI (`Intelligence.tsx`).
- Offline-first runtime extensions: RFC 8785 I-JSON JCS + `PPM_SCALE=1_000_000` fixed-point bundle calibration, cache eviction policy, sequence-classification / NLI head inference, ordered `evaluate_batch`, RAM/disk preflight checks, and `AtomicBool` cancellation in `gliner_runtime.rs` and `asr_runtime.rs`.
- Explicit lifecycle states preserved in UI and contracts; zero unconsented Hub downloads; zero arbitrary `trust_remote_code` execution.

---

## 3. Wave-by-Wave Architectural Summary

| Wave | PR & Merged `main` | Key Capabilities Transplanted & Verified |
|---|---|---|
| **Wave 1** | [#537](https://github.com/AbdulazizShehri/MedScale/pull/537) (`ef3e4fce`) | `docs/openmed/` REA + parity/transplant matrices + execution ledger; `scripts/vendor_openmed_v4.py` + `medscale_clinical` v0.4.0 (`integrations/clinical-evidence-python`) + `@medscale/sdk` (`integrations/medscale-sdk-ts`, zero runtime deps); DAG hardening (`check-dependency-direction.ps1`); full catalog schema, recommender, manifest diff, model cards, portable bundle JCS/PPM calibration, RAM/disk preflight, seq-cls head, batch eval, GLiNER/ASR cancellation |
| **Wave 2** | [#538](https://github.com/AbdulazizShehri/MedScale/pull/538) (`d1cd3bd0`) | Unified `PrivacyGate::model_recognize` with `run_document` + `pii_merge` and all 11 OpenMed validators + MedScale's Saudi Citizen/Iqama Luhn validator; 24 OpenMed policy JSONs + Saudi PDPL (`clinical_protect`, `safety_sweep`, `CascadeRouter` `R0`-`R3`, preview/explain); Arabic-Indic digits, MIT `hijridate` Umm al-Qura Hijri & `py-nepali` Bikram Sambat calendars, HMAC `KeyStore` date shift, RTL `FSI`/`PDI` wrapping; `k`-anonymity/`l`-diversity/`t`-closeness + prosecutor/journalist/marketer risk & DP budget; compliance receipts (`part11`, `iso27701`, `dsar`, `audit_chain`, `expert_attestation`, `waiver_ledger`); full compositional UCUM parser & unit conversion (`medscale-fhir`); Clinical NLP (`sections`, `medspaCy ConText`/negation/experiencer, `sig_parser`, med reconciliation, `dosing_check`, coref, events, relations, SDOH, radiology/pathology/discharge/oncology profiles, NLI & citation gates) |
| **Wave 3** | [#539](https://github.com/AbdulazizShehri/MedScale/pull/539) (`9df24f32`) | Pure-Rust DoS-bounded extractors/scrubbers/residual verifiers (PDF, OOXML + `ooxml_residual`, EPUB, HTML, MD/AsciiDoc, RTF, EML, CSV/TSV/JSONL/CHW/vCard/iCal, raster image + sanitized local Tesseract adapter, ECG waveform) without PyMuPDF or GPL `.msg`; clean-room native DICOM Part 10 + PS3.15 header de-ID + `2.25.<u128>` UID remap + pixel blackout + SR/waveform (`dicom_v4.rs`); FHIR `$de-identify`, US Core/R4-R5 check, SMART scope grammar/audit, HL7v2, C-CDA, X12 837, OMOP v5.4 (`interop_v4.rs`); `MedAgent` `ActionGraph` DAG, `ActionPhase` FSM, single-use HMAC `ApprovalToken` (with nonce ordering hardened), `InjectionGuard`, clinician review draft templates (`agent_eval_v4.rs`); CLI streaming/JSONL batch & opt-in loopback/stdio MCP + SSRF-guarded local service (`wave3_ops.rs`); NER/relation/leakage/calibration eval harnesses with DUA outside-repo path guards; 6 Tauri IPC commands wired into Desktop `Clinical`, `Governance`, and `Intelligence` workbenches |
| **Wave 4 + RC** | Wave 4 Docs PR (`feat/openmed-v4-qualification-and-docs`) | Native Windows `--features custom-protocol --release --locked` Desktop build verification; Dark & Light visual audit of all 5 upgraded Desktop workflows (`10` PNGs); `THIRD_PARTY_NOTICES.md`, `docs/legal/NOTICE_INVENTORY.md`, `OPENMED_FEATURE_PARITY_MATRIX.md`, `OPENMED_PARITY_EXECUTION_LEDGER.md`, `OPENMED_FINAL_PARITY_REPORT.md`; fresh non-public `.github/workflows/release-candidate.yml` (`0.1.0-beta.1`) qualification |

---

## 4. Verification, Native Build & Visual Audit Summary

- **PR #537 (Wave 1), PR #538 (Wave 2), and PR #539 (Wave 3):** Each merged into protected `main` by squash merge only after **`26/26` executable CI checks passed** (`Publish to medscale.si` intentionally skipped on PRs).
- **Conformance & Unit Suites (recorded across Waves 1–3, detailed per-crate counts in [OPENMED_PARITY_EXECUTION_LEDGER.md](OPENMED_PARITY_EXECUTION_LEDGER.md)):**
  - Python reference suite (`integrations/clinical-evidence-python`): `450 passed, 182 subtests passed`, zero third-party runtime dependencies, including isolated-environment optional `markdown-it-py` check (`4/4` OK).
  - Workspace Rust crates (`medscale-pack`, `medscale-fhir`, `medscale-clinical`, `medscale-core`, `medscale-cli`) and `apps/desktop-tauri`: all targeted unit, integration, and conformance suites passed post-review with `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings` clean (`0` warnings).
  - TypeScript & Playwright UI suites (`apps/desktop-tauri`): `85/85` Vitest unit/identity tests, `25` routes × `2` themes × `2` widths (`0` visual misses), and `12/12` first-run E2E checks.
- **Native Windows `--release --locked` Desktop Binary (Wave 4):**
  - Command: `npm run build` (`50 modules`, `921ms`) followed by `cargo build --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml --features custom-protocol --release --locked` (`Finished 'release' profile [optimized] target(s) in 44m 39s`).
  - Verified Output: `medscale-desktop-tauri.exe` — **`72,296,448` bytes**, **`sha256:c80ea172b12a015fde3a945634e36c5a65c12afcbd0534a32734e254be24a7cd`**.
- **Dark & Light Native UI Visual Audit (`1440x900`, headless Edge over compiled `dist/`):**
  - Captured and inspected all `10` interactive full-page PNGs across `wave4-{documents,exports,privacy,models,medagent}-{dark,light}.png` (`0` script errors, `0` horizontal overflow). Verified multimodal & DICOM PS3.15 workbenches, FHIR `$de-identify`/SMART/OMOP interop inspector, `CascadeRouter` & Arabic RTL `FSI`/`PDI` isolation rendering, 2,266-row Model Catalog & tier recommender, and `MedAgent` ActionGraph/ApprovalToken governance panel with 100% MedScale branding.

---

## 5. Third-Party License & `BLOCKED_EXTERNAL_RIGHTS` Compliance

- All derived source attributions and permissive third-party notices (`OpenMed` Apache-2.0, `openmed-agent` Apache-2.0, `medspaCy ConText` Apache-2.0, `Presidio` MIT, `ARX` Apache-2.0, `DataProfiler` Apache-2.0, `KServe V2` Apache-2.0, `ICU 57.1` ICU License, `hijridate` MIT, `py-nepali` MIT, `baijiaxing` CC0-1.0, `Inter`/`JetBrains Mono` OFL-1.1) are documented in root [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md), [`docs/legal/NOTICE_INVENTORY.md`](../legal/NOTICE_INVENTORY.md), and [`third_party/provenance/openmed-v4-source-transplant.md`](../../third_party/provenance/openmed-v4-source-transplant.md).
- Every `BLOCKED_EXTERNAL_RIGHTS` item (`sdcMicro` GPL-2.0-or-later, `extract-msg` `.msg` GPL-3.0, `MedCAT` Elastic-2.0, `Aksharamukha` AGPL-3.0, Google ML Kit, `openmed-labs/synthvision`, `openmed-labs/welna`, external DUA datasets such as `i2b2`/`n2c2`/`SHAC`/`MADE`/`THYME`/`MedNLI`/`MIMIC`/`SHIELD`, and restricted terminologies such as `SNOMED CT`/`CPT`/`UMLS`/`LOINC`) remains excluded from bundled artifacts and guarded by automated negative-control tests.

---

## 6. Unchanged External, Privacy, Clinical & Release Boundaries

Achieving full OpenMed `v4.0.0` engineering capability parity does **not** alter any legal, clinical, or publication qualification gate:

- `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`
- `CLINICAL_USE_AUTHORIZATION=NOT_AUTHORIZED`
- `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`
- `PUBLIC_BETA_PUBLISHED=false`
- `RELEASE_READY=false`
- `FOUNDER_RELEASE_APPROVAL=PENDING`
- Staged Draft Release `460344459` remains unpublished; Issue `#534` remains untouched; `web/src/lib/release-manifest.json` keeps `"release": null`; active `gh` CLI identity remains `TheHalfMoon`.

See [OPENMED_PARITY_EXECUTION_LEDGER.md](OPENMED_PARITY_EXECUTION_LEDGER.md) for the Wave 4 command execution ledger and fresh non-public `0.1.0-beta.1` release-candidate verification once the Wave 4 PR merges.

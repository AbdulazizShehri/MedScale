# OpenMed v4.0.0 Full Reverse-Engineering Analysis (REA)

> **Scope & Historical Pins:** This document freezes MedScale's reverse-engineering analysis against OpenMed **`v4.0.0`** (`maziyarpanahi/openmed@5b79cae5df6bc22b0fcdc799be480ee4ffc23c57`, 2026-11-27, Apache-2.0) and `openmed-labs/openmed-agent@ded6e57b5b93cb6dc849c8639662b381179697c7` (`v1.0.3`, Apache-2.0). Existing historical comparators for OpenMed `v2.2.0` (`evidence/007-openmed-capability-absorption-parity/`, `evidence/071-openmed-evidence-center/`) and `v3.0.0` (`third_party/medscale-model-catalog-v3/`, `specs/103-local-model-catalog/`) remain untouched as frozen historical snapshots.
>
> **Authorization & Qualification Posture:** `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `CLINICAL_USE_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`. Capability absorption and synthetic execution never constitute clinical validation or permission to process live PHI.

---

## 1. Upstream Sources Inspected & Acquisition Boundaries

| Upstream Source | Exact Revision / Tag | License | Inspected Findings | MedScale Disposition |
|---|---|---|---|---|
| `https://github.com/maziyarpanahi/openmed` | `v4.0.0` / `5b79cae5df6bc22b0fcdc799be480ee4ffc23c57` | Apache-2.0 (+ `NOTICE` third-party attributions) | 6,475 tracked files; 1,533 Python modules under `openmed/` (`1,327` zero top-level external imports); 334 non-Python assets (`openmed/`); `models.jsonl` (2,266 rows); `clients/{typescript,go}`, `swift/`, `android/`, `js/`, `deploy/` | Authorized Apache-2.0 source transplant & native Rust/TypeScript/Desktop parity |
| `https://github.com/openmed-labs/openmed-agent` | `v1.0.3` / `ded6e57b5b93cb6dc849c8639662b381179697c7` | Apache-2.0 | Contains only `README.md` + `LICENSE`; invite-only waitlist CLI pointer (`waitlist.openmed.life`), while all 72 open-source offline clinical agent governance & workflow modules reside in `openmed/agent/` inside `maziyarpanahi/openmed` | Transplants draw from `openmed/agent/` (`v4.0.0`); no closed waitlisted binary needed |
| `https://huggingface.co/OpenMed` | Per-row `repo_id` in `models.jsonl` | Upstream model card terms vary per weight repo | 2,266 manifest entries (manifest entry != downloaded != locally runnable != clinically validated) | Preserve immutable HuggingFace `repo_id` for lookup/verification; display under MedScale catalog UI without ownership claim |
| `openmed-labs/synthvision`, `openmed-labs/welna` | Unlicensed / `license: null` | Unspecified (`RIGHTS_PENDING`) | Not permissive open source on GitHub | **Excluded** — no code copied or adapted |

---

## 2. Ecosystem Topology & Component Inventory (OpenMed `v4.0.0`)

### 2.1 `openmed/` Python Package Breakdown (`1,533` `.py` files / `334` assets)

| Subpackage | `.py` Modules | Zero Top-Level 3P Imports | Key Capabilities |
|---|---|---|---|
| `openmed/clinical/` | 302 | 293 | Evidence guardrails, UCUM units, ConText/negation/experiencer/sections, sig parser, medication reconciliation & dosing checks, event frames, relations (21 modules), coreference, SDOH, radiology/pathology/discharge/oncology profiles, timelines |
| `openmed/eval/` | 215 | 201 | NER/relation/suite scorecards, training-overlap leak check, calibration/release gates, red-team suite, DUA corpus path enforcement (external-only loaders) |
| `openmed/multimodal/` | 153 | 141 | PDF layout/raster-burned redaction/verification (zero PyMuPDF), OOXML residual verifier, HTML/MD/AsciiDoc/RTF/EPUB/EML extractors, CSV/JSONL/CHW XForms/vCard/iCal tabular privacy, OCR abstraction + streaming Tesseract, DICOM PS3.15 header/pixel/SR/waveform, offline ECG + Edge VLM gates |
| `openmed/core/` | 135 | 113 | Manifest schema & search/recommender/diff/cards, RFC 8785 I-JSON JCS + DSSE portable bundles & fixed-point calibration, device capabilities, cache policy, `R0`-`R3` cascade PII pipeline, Arabic/Hijri/Bikram Sambat date-shift, RTL isolates, 24 policy profiles, clinical protection, safety sweep |
| `openmed/interop/` | 131 | 119 | FHIR `$de-identify`, R4/R5 fidelity, US Core/reference checks, bulk NDJSON gateway, SMART v1/v2 scope grammar & audit, HL7v2, C-CDA XML, X12 837, OMOP CDM v5.4 ETL, fail-closed external bridges |
| `openmed/service/` | 95 | 55 | Hardened REST routes (`/analyze`, `/pii/*`, `/v1/workflows/*`, `/fhir/*`, `/omop/load`), SHA-256 auth, strict POST `/graphql`, egress policy (`DENY`/`ENFORCE`/`PERMISSIVE`), privacy gateway/proxy, gRPC servicer |
| `openmed/agent/` | 72 | 71 | Identifier-only `ActionGraph` DAG validator, `ActionPhase` FSM, single-use HMAC-SHA256 `ApprovalToken`, `CapabilityGrant`, `InjectionGuard`, `DataProjection`, WAL `CheckpointJournal`, governed workflow drafts (`prior_auth_completeness`, `trial_eligibility_review`, etc.) |
| `openmed/training/` | 71 | 62 | DAPT/SFT recipes, synthetic lineage SPDX gates, non-permissive shard export blocks |
| `openmed/structured/` | 64 | 55 | Grammar/schema-constrained extraction contracts |
| `openmed/cli/` | 43 | 42 | Subcommands across analyze, PII, batch, multimodal, interop, catalog, bundles |
| `openmed/risk/` | 38 | 37 | `k`-anonymity, `l`-diversity, variational `t`-closeness, prosecutor/journalist/marketer re-id risk, differential privacy budget, longitudinal mitigation, surrogate audit |
| `openmed/compliance/` | 24 | 23 | Safe Harbor attestation mapper, 21 CFR Part 11, ISO/IEC 27701, DSAR, digest-chained audit trail, expert attestation, waiver ledger |
| `openmed/onnx/`, `mlx/`, `torch/`, `coreml/`, `gguf/`, `zero_shot/` | 68 | 20 | Format-specific model loaders, sequence classification / cross-encoder / embeddings / token classification / zero-shot / SLM KV-memory estimation |
| `openmed/mcp/`, `guard/`, `traces/`, `processing/`, `integrations/`, `models/`, `utils/`, `plugins/`, `ner/`, `deploy/`, `skills/` | 117 | 78 | `FastMCP` tool specs (`ConsentReceipt` + SSRF `upstream_endpoint_policy`), generated-text guards, clean-room ISO 15919 transliteration, enterprise stream/Lakehouse stubs |

---

## 3. Domain Deep-Dive & MedScale Baseline Delta

### 3.1 Catalog (`models.jsonl`) & Model Execution Architecture (`Stream A`)
- **`models.jsonl` Exact Census (`2,266` rows, `sha256:805e79f7db5514cf308c6deffa67d72c4ce9a59bf4e54487d2783eecbb1e34ab`):**
  - Families: `NER` (1,093), `PII` (1,018), `ZeroShot` (143), `General` (9), `Vision` (3).
  - Tasks: `token-classification` (2,254), `text-generation` (6), `unknown` (3), `visual-question-answering` (3).
  - Format tags (multi-label): `pytorch` (1,512), `onnx` (753), `mlx-fp` (652), `mlx-8bit` (7), `mlx-4bit` (2).
  - Top architectures: `bert` (754), `xlm-roberta` (366), `deberta-v2` (304), `modernbert` (227), `roberta` (209), `distilbert` (204), `gliner` (91), `eurobert` (33), `qwen` (30), `clinical-longformer` (10), `None` (9), `privacy-filter` (5).
- **Critical MedScale Gap:** `crates/medscale-pack/src/catalog.rs` already embeds `third_party/medscale-model-catalog-v3/models.jsonl` (byte-identical to v4.0.0!) and parses `13` fields, **dropping 8-12 fields** (`arxiv`, `benchmark`, `canonical_labels`, `download_mb`, `download_sizes`, `peak_ram_mb`, `recommended_tier`, `latency_ms`, `script_coverage`, `script_eval`, `training_data_licenses`, `training_provenance`). MedScale also lacks OpenMed's deterministic tier recommender (`recommend_models` across `phone:6000 / laptop:16000 / workstation:64000 / server:512000` MB), manifest diff (`diff_manifests`), model-card markdown generator (`model_card.py`), RFC 8785 I-JSON JCS / fixed-point `PPM_SCALE=1_000_000` bundle calibration (`portable_bundle*.py`), `sequence-classification` / NLI head inference, ordered `evaluate_batch`, RAM/disk preflight, and `AtomicBool` cancellation in `gliner_runtime.rs` / `asr_runtime.rs`.

### 3.2 Privacy, De-Identification, Internationalization & Risk (`Stream B`)
- **Bugs & Gaps Inside Existing MedScale:**
  - `PrivacyGate::model_recognize` (`crates/medscale-core/src/authority/privacy_gate.rs:563-612`) calls low-level single-window `prepared.run` and manually aggregates tokens, **bypassing `run_document` and `medscale_pack::pii_merge`**.
  - `privacy_recognizers.rs` imports only 9 of 11 validators from `medscale_pack::identifier_validators`, omitting `validate_ssn` and `validate_phone_us`.
- **Where MedScale Already Exceeds OpenMed:**
  - Native **Saudi National ID / Iqama** Luhn + Arabic keyword validation (`validate_saudi_national_id`), missing in OpenMed v4.0.0.
  - Real structural FHIR JSON HIPAA Safe Harbor 18-category transformer (`crates/medscale-core/src/authority/safe_harbor.rs`), whereas OpenMed's `compliance/safe_harbor.py` outputs post-run label-count attestations over declarative JSON policies.
- **Missing From MedScale:** OpenMed's `24` policy JSONs (`174` labels), `R0`-`R3` `CascadeRouter`, `arbitrate`, `clinical_protect` preservation terms, `safety_sweep`, `StreamingDeidentifier`, `redaction_preview`, Arabic-Indic digit normalization, MIT `hijridate` Umm al-Qura Hijri table (`hijri_umm_al_qura.json`, 1356–1450 AH) + `bikram_sambat.json`, HMAC-consistent date shifting (`date_shift`), RTL bidi `FSI`/`PDI` wrapping (`rtl_render`), reversible encrypted surrogate vaults (`SurrogateVault`), `k`/`l`/`t` + prosecutor/journalist/marketer risk (`openmed/risk/`), and `part11`/`iso27701`/`dsar`/`audit_chain`/`expert_attestation` (`openmed/compliance/`). Note: `openmed/core/pii_i18n.py` imports `faker` at line 29, requiring AST separation of pure recognizers/validators from synthetic surrogate generation.

### 3.3 Clinical NLP, UCUM Units & Clinical Intelligence (`Stream C`)
- **Current MedScale Footprint:** `integrations/clinical-evidence-python/` (`medscale_clinical` v0.2.0) holds 41 modules ported from `3e232328`, while `crates/medscale-clinical` ports only 3 of them (`evidence_coverage`, `evidence_span_audit`, `summary_empty_evidence`) plus `canonical`. `crates/medscale-fhir/src/units/mod.rs` holds only a 14-code lookup table.
- **OpenMed v4.0.0 Target Scope (`321` files in `openmed/clinical/`):** Remaining 38 pure-stdlib safety/citation/NLI/review/SDOH modules already in Python, full compositional UCUM algebra (`units/ucum.py` + `convert.py` + `display_audit.py`), clinical sectionization (`sections/`), **medspaCy ConText**-derived assertion/negation/temporality/experiencer rules (`context.py`, `negation_scope.py`, `experiencer.py`), medication sig parser + reconciliation + dosing checks (`sig_parser.py`, `medication_reconciliation`, `dosing_check.py`), entity coreference (`coref/`), clinical event frames (`events/`), clinical relations (`relations/` — 21 modules including ADE/DrugProt/lab links/medication statements), clinical specialty profiles (`radiology`, `pathology`, `discharge`, `cancer_staging`, `genomics`), and non-proprietary grounding crosswalks.

### 3.4 Multimodal Documents, Native DICOM, Healthcare Interop, MedAgent, MCP & Eval (`Stream D`)
- **Multimodal:** OpenMed has **zero PyMuPDF (`fitz`)**. All office/web/text/chat/CHW formats (`html`, `markdown`, `asciidoc`, `rtf`, `epub`, `eml`, `ooxml_residual`, `tabular_csv`, `chatlog_jsonl`, `chw_forms`, `contacts_calendar`, offline `ecg/*`) are stdlib-only. PDF/image raster redaction & OCR (`pdfplumber`/`pikepdf`/`Pillow`/external `tesseract`) and DICOM (`pydicom` PS3.15 header de-ID, UID remapping, pixel blackout, SR/waveform) translate cleanly to pure Rust parsers + sanitized external binary adapters without C++ copyleft deps.
- **Interop:** `hl7v2.py`, `cda.py`, `x12_837.py`, `fhir_operations.py` (`$de-identify`), `fhir/exporters/*`, `omop/*`, `smart_scope_grammar.py`, `smart_scope_audit.py` are stdlib-only deterministic specifications.
- **Agent (`openmed/agent/`):** Value-free `ActionGraph` DAG (bounded to 10k nodes / 64 deps), `ActionPhase` FSM (`waiting-review -> preflight` gated on `reviewed=True`), single-use HMAC-SHA256 `ApprovalToken`, `CapabilityGrant`, `InjectionGuard`, WAL `CheckpointJournal`, and read-only clinician review templates (`prior_auth_completeness`, `trial_eligibility_review` with `authorizes_enrollment=False`).
- **Service / MCP / SDKs / Eval:** Strict POST `/graphql`, loopback default `127.0.0.1`, fail-closed egress, `FastMCP` + `ConsentReceipt` + SSRF `upstream_endpoint_policy`, zero-dep TypeScript (`@medscale/sdk`) and Go clients, plus DUA-outside-repo-enforced eval harnesses (`run_benchmark`, `run_relation_benchmark`, leakage & calibration gates).

---

## 4. Non-Apache, Proprietary & DUA Quarantine Rules

1. **Zero GPL / Elastic / Copyleft In-Process Inclusion:**
   - **GPL-3.0 `extract-msg`** (`.msg` Outlook files) and **GPL-2.0 `sdcMicro`** (R disclosure control) and **Elastic-2.0 `MedCAT`**: Excluded completely from MedScale core and Python reference transplants.
   - **AGPL-3.0 `Aksharamukha`**: Zero code copied; only clean-room ISO 15919 tables permitted.
   - **Proprietary Google ML Kit (`com.google.mlkit`)**: Excluded completely.
2. **DUA Corpora & Restricted Terminologies (`BLOCKED_EXTERNAL_RIGHTS`):**
   - Never commit or fetch `i2b2`, `n2c2`, `SHAC`, `MADE`, `THYME`, `MedNLI`, `MIMIC-III/IV`, `SHIELD`, `RadGraph`, `BioRED`, `CBLUE`, or `MasakhaNER` (NC). Eval loaders must verify caller-supplied paths live outside the repo root (`network_fetch=False`, `eval_only=True`).
   - Never bundle `SNOMED CT`, full `LOINC`, full `UMLS` (`MRCONSO`/`MRREL`/`MRSTY`), or `CPT`. Only tiny Apache-2.0 structural concept/section crosswalk IDs (`section_loinc_map`, `ucum`, starter `CC0-1.0` crosswalks) or user-supplied external paths are permitted.
3. **Mandatory Upstream Attributions (`NOTICE_INVENTORY.md` + Per-File Headers):**
   - `medspaCy ConText` (Apache-2.0, `https://github.com/medspacy/medspacy`, attribution header + port date + local modification summary required on ported rule/context files).
   - `hijridate` (`2.6.0`, MIT, Copyright (c) Mohammed Alshehri) on `hijri_umm_al_qura.json` + SHA-256 verification.
   - `py-nepali` (`1.2.0`, MIT, Copyright (c) 2018 The Python Packaging Authority) on `bikram_sambat.json` + SHA-256 verification.
   - `ICU 57.1` grapheme break rules (ICU License, IBM Corp.) with `ICU.txt` digest-bound notice where used.
   - `Presidio` (MIT), `DataProfiler` (Apache-2.0), `KServe V2` (Apache-2.0), `ARX` (Apache-2.0).
4. **Architectural Constraints in `D:\MedScale`:**
   - Preserve exact **11 workspace crates** (`Cargo.toml`) and **26 Desktop routes** (`apps/desktop-tauri/src/routes.ts`).
   - Expose all capabilities inside existing MedScale modules, CLI commands, and Desktop route tabs/drawers using MedScale branding; keep immutable HuggingFace `OpenMed/...` `repo_id` strings in backend records while formatting UI labels without third-party branding leaks.

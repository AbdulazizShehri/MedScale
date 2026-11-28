# OpenMed v4.0.0 Feature Parity Matrix

> **Baseline Anchor:** `D:\MedScale` at `main@d36ada66c6fc6a7c729dfb51d66100d6edb5f3e1` vs. OpenMed `v4.0.0` (`5b79cae5df6bc22b0fcdc799be480ee4ffc23c57`) and `openmed-agent@v1.0.3` (`ded6e57b5b93cb6dc849c8639662b381179697c7`).
>
> **Status Taxonomy (9 Closed States):**
> - `PARITY_COMPLETE` — End-to-end Rust Core, CLI (where applicable), Desktop UI (where applicable), and verification tests match or exceed upstream behavior.
> - `MEDSCALE_STRONGER` — MedScale's existing offline/governed/regional implementation exceeds OpenMed v4.0.0.
> - `PARTIAL` — Subset implemented; explicit gaps remain in fields, runtimes, Core wiring, CLI, UI, or test depth.
> - `MISSING` — Not yet implemented in MedScale.
> - `STUB_ONLY` — Placeholder/static table only; no real execution or validation logic yet.
> - `UI_MISSING` — Rust Core / CLI exists, but Desktop Tauri UI does not yet surface the capability.
> - `CLI_MISSING` — Rust Core exists, but `medscale-cli` does not yet expose the command/flag.
> - `TEST_MISSING` — Logic partially exists without conformance/edge/integration verification.
> - `BLOCKED_EXTERNAL_RIGHTS` — Excluded or externally user-supplied-only due to GPL/Elastic/proprietary license, DUA corpus restrictions, or restricted clinical terminology licensing.

---

## 1. Summary Dashboard (Baseline -> Target Parity Wave)

| Domain | Baseline Status (`main@d36ada66`) | Planned Completion Wave | Primary OpenMed v4 Reference | Primary MedScale Surface |
|---|---|---|---|---|
| Catalog schema (2,266 entries, 25+ fields) | `PARTIAL` (13/21+ fields; 8+ dropped) | Wave 1 | `openmed/core/manifest_schema.py`, `models.jsonl` | `crates/medscale-pack/src/catalog.rs`, `Models` UI |
| Hardware-tier model recommendation & comparison | `MISSING` | Wave 1 | `openmed/core/model_search.py`, `device_capabilities.py` | `crates/medscale-pack`, CLI `packs recommend`, `Models` UI |
| Catalog manifest diff & model card renderer | `MISSING` | Wave 1 | `openmed/core/manifest_diff.py`, `model_card.py` | `crates/medscale-pack`, CLI `packs manifest-diff/card`, `Models` UI |
| Offline ONNX/Candle/GLiNER/Whisper runtimes | `PARTIAL` (token-cls + seq2seq + whisper + gliner exist; missing seq-cls/NLI head, batch eval, GLiNER/ASR cancel, RAM preflight) | Wave 1 | `openmed/onnx/*`, `torch/*`, `mlx/*`, `core/portable_bundle*.py` | `crates/medscale-pack/src/{candle,onnx,gliner,asr}_runtime.rs` |
| Portable bundle JCS + fixed-point calibration + cache policy | `MISSING` | Wave 1 | `openmed/core/{portable_bundle,portable_bundle_calibration,model_cache_policy}.py` | `crates/medscale-pack`, `crates/medscale-contracts` |
| Python clinical reference SDK (`medscale_clinical`) | `PARTIAL` (41/321 clinical + 2/135 core at v3 pin) | Wave 1 | Transitive stdlib OpenMed v4 modules (`scripts/vendor_openmed_v4.py`) | `integrations/clinical-evidence-python/` |
| Zero-dep TypeScript REST & workflow SDK (`@medscale/sdk`) | `MISSING` | Wave 1 | `clients/typescript/` | `integrations/medscale-sdk-ts/` |
| Core multi-window PII merge (`PrivacyGate` + `pii_merge`) | `PARTIAL` (`PrivacyGate::model_recognize` bypasses `run_document` + `pii_merge`; 2/11 validators skipped) | Wave 2 | `openmed/core/{pii,pii_entity_merger}.py` | `crates/medscale-core/src/authority/{privacy_gate,privacy_recognizers}.rs` |
| Checksum identifier validators (11+ + Saudi Citizen/Iqama) | `MEDSCALE_STRONGER` (Saudi ID Luhn + 11 OpenMed validators in `medscale-pack`) / `PARTIAL` (Core omits SSN/US-phone) | Wave 2 (unify) | `openmed/core/pii_i18n.py` | `crates/medscale-pack/src/identifier_validators.rs`, `medscale-core` |
| Structural FHIR HIPAA Safe Harbor 18-category transform | `MEDSCALE_STRONGER` (deterministically transforms 17 FHIR types + residual re-scan vs OpenMed attestation-only) | Preserved & extended in Wave 2 | `openmed/compliance/safe_harbor.py` | `crates/medscale-core/src/authority/safe_harbor.rs` |
| Privacy policies (24 profiles), `CascadeRouter` (`R0`-`R3`), `clinical_protect`, `safety_sweep`, explain/preview | `MISSING` | Wave 2 | `openmed/core/{policies/*.json,clinical_protect.py,safety_sweep.py}` | `medscale-pack`, `medscale-core`, CLI, `Privacy` UI |
| Arabic-Indic digits, Hijri Umm al-Qura / Bikram Sambat, HMAC date shift, RTL `FSI`/`PDI` | `MISSING` | Wave 2 | `openmed/core/{data/calendars/*,date_shift.py,rtl_render.py}` | `medscale-pack` / `medscale-core`, CLI, `Privacy` UI |
| Tabular & longitudinal risk (`k`/`l`/`t`, prosecutor/journalist/marketer) + DP budget | `MISSING` | Wave 2 | `openmed/risk/{kanon,l_diversity,reid,qi_profiler,budget}.py` | `crates/medscale-clinical`, CLI, `Privacy`/`Analytics` UI |
| Compliance ledgers (`part11`, `iso27701`, `dsar`, `audit_chain`, `expert_attestation`, `waiver_ledger`) | `MISSING` (generic audit exists; OpenMed compliance receipts missing) | Wave 2 | `openmed/compliance/*.py` | `medscale-core`, `medscale-clinical`, CLI, `Privacy`/`Governance` UI |
| Compositional UCUM unit parser & cross-unit conflict conversion | `STUB_ONLY` (14-code lookup table in Rust; Python reference already vended) | Wave 2 | `openmed/clinical/units/{ucum,convert,display_audit}.py` | `crates/medscale-fhir/src/units/`, `medscale-core` conflict check |
| Clinical safety guardrails (citations, claim conflicts, NLI gates, summary budgets, SDOH rules, review SLA/FSM) | `PARTIAL` (3/41 modules in Rust; 38 unported from Python reference) | Wave 2 | `openmed/clinical/*.py` (`3e232328` -> `5b79cae5`) | `crates/medscale-clinical`, Core, CLI, `Evidence`/`Patients` UI |
| Clinical NLP (`sections`, medspaCy `ConText`/negation/experiencer, `sig_parser`, med reconciliation, `dosing_check`, `coref`, `events`, `relations`, specialty profiles) | `MISSING` | Wave 2 | `openmed/clinical/{context,sections,sig_parser,relations/*,events/*,*_profile}.py` | `crates/medscale-clinical`, Core, CLI, `Evidence`/`Patients`/`Insights` UI |
| Multimodal doc extractors, scrubbers & residual verifiers (PDF, OOXML, EPUB, HTML, MD/AsciiDoc, RTF, EML, CSV/JSONL/CHW/vCard/iCal, Image/Tesseract, ECG) | `PARTIAL` (basic PDF/text intake only) | Wave 3 | `openmed/multimodal/*.py` (zero PyMuPDF) | `crates/medscale-pack` / `medscale-core`, CLI, `Documents`/`Privacy` UI |
| Native DICOM Part 10 + PS3.15 header de-ID + `2.25` UID remap + pixel blackout + SR/waveform | `MISSING` | Wave 3 | `openmed/multimodal/{dicom,dicom_sr,dicom_waveform}.py` | `crates/medscale-core` / `medscale-pack`, CLI, `Documents`/`Privacy` UI |
| Interop (FHIR `$de-identify`, US Core/R4-R5 check, SMART scope grammar, HL7v2, C-CDA, X12 837, OMOP v5.4) | `PARTIAL` (basic FHIR bundle ingest exists; `$de-identify`/HL7v2/C-CDA/OMOP/SMART missing) | Wave 3 | `openmed/interop/*.py`, `openmed/clinical/exporters/{fhir,omop}/*` | `crates/medscale-fhir`, `crates/medscale-clinical`, CLI, `Interoperability` UI |
| Clinical Agent Kernel (`ActionGraph`, FSM, single-use HMAC `ApprovalToken`, `CapabilityGrant`, `InjectionGuard`, workflow drafts) | `PARTIAL` (MedScale `medagent.rs` baseline vs OpenMed v4 72-module governance kernel) | Wave 3 | `openmed/agent/*.py` (mirroring `openmed-agent@v1.0.3` open surface) | `crates/medscale-core/src/authority/medagent.rs`, CLI, `MedAgent` UI |
| CLI streaming/JSONL batch & opt-in loopback/stdio MCP + SSRF-guarded local service | `PARTIAL` (CLI single/ops commands exist; batch streaming/loopback MCP/consent receipts missing) | Wave 3 | `openmed/cli/*`, `openmed/mcp/*`, `openmed/service/*` | `crates/medscale-cli`, `medscale-network`/`core` |
| Eval harnesses (NER, relation, leakage, calibration, DUA outside-repo path guard) | `MISSING` | Wave 3 | `openmed/eval/{harness,release_gates,redteam,datasets/_dua}.py` | `crates/medscale-clinical`, CLI, `Models`/`Model Fleet`/`Analytics`/`Research OS` UI |
| GPL/Elastic/Proprietary/DUA modules (`extract-msg` GPL-3.0, `sdcMicro` GPL-2.0, `MedCAT` Elastic-2.0, `Aksharamukha` AGPL-3.0, Google ML Kit, `synthvision`/`welna`, `i2b2`/`n2c2`/`SHIELD`/`MedNLI`/`MIMIC`, `SNOMED`/`UMLS`/`CPT`) | `BLOCKED_EXTERNAL_RIGHTS` | Quarantined permanently | License policy & DUA boundaries | Explicit negative-control tests verifying exclusion |

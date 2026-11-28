# MedScale Third-Party Notices

MedScale first-party source code is licensed under the **Apache License, Version 2.0** (see [`LICENSE`](LICENSE)).  
This notice consolidates third-party software, data tables, typography, protocol definitions, and upstream reference attributions included in, adapted by, or explicitly excluded from MedScale.

For the complete machine-checked Rust Cargo dependency notice table across the workspace and Tauri build graph, see [`docs/legal/NOTICE_INVENTORY.md`](docs/legal/NOTICE_INVENTORY.md) and [`evidence/032-privacy-probes-notice-perf/notice-inventory.json`](evidence/032-privacy-probes-notice-perf/notice-inventory.json).

---

## 1. OpenMed `v4.0.0` & `openmed-agent` `v1.0.3` Upstream Notices (Apache-2.0)

MedScale incorporates authorized source transplants, conformance vectors, clean-room Rust ports, TypeScript SDK clients, and stdlib Python clinical reference modules derived from:

- **OpenMed (`v4.0.0`)**  
  - Repository: `https://github.com/maziyarpanahi/openmed`  
  - Pinned Commit: `5b79cae5df6bc22b0fcdc799be480ee4ffc23c57`  
  - License: Apache-2.0 (verbatim upstream `LICENSE` and `NOTICE` preserved in [`integrations/clinical-evidence-python/third_party/openmed/`](integrations/clinical-evidence-python/third_party/openmed/) and [`integrations/medscale-sdk-ts/`](integrations/medscale-sdk-ts/))  
  - Historical Clinical Reference Pin (preserved): `3e2323288ec551157c0b943b331f399ef1434649` (`third_party/provenance/openmed-clinical-python-2026-11-08.md`)
- **`openmed-agent` (`v1.0.3`)**  
  - Repository: `https://github.com/openmed-labs/openmed-agent`  
  - Pinned Commit: `ded6e57b5b93cb6dc849c8639662b381179697c7` (`README.md` + `LICENSE` Apache-2.0; all open offline governance modules reside in `openmed/agent/` within `maziyarpanahi/openmed`)
- **Provenance Record:** [`third_party/provenance/openmed-v4-source-transplant.md`](third_party/provenance/openmed-v4-source-transplant.md)

### Immutable Read-Only Model Catalog Snapshot

- Directory: [`third_party/medscale-model-catalog-v3/`](third_party/medscale-model-catalog-v3/)
- Upstream Source: `https://github.com/maziyarpanahi/openmed` (`ea920f36fadd7b45935247d639f0ffa1ef493b23`; `models.jsonl` is byte-identical to `v4.0.0` at `5b79cae5df6bc22b0fcdc799be480ee4ffc23c57`)
- Exact Digests:
  - `models.jsonl` (2,266 manifest entries, 3,057,702 bytes): `sha256:805e79f7db5514cf308c6deffa67d72c4ce9a59bf4e54487d2783eecbb1e34ab`
  - `LICENSE`: `sha256:c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`
  - `NOTICE`: `sha256:94bcb50459cf2603cb9ce8cbdeb00b0e09ebd82616844c87931ce7270e4ba1c2`
- **Rights Notice:** Listing entries in `models.jsonl` (preserving immutable HuggingFace `OpenMed/...` `repo_id` lookup strings) does not grant redistribution rights or clinical validation for third-party model weights. Each model's own upstream model card and license governs weight acquisition and use (`evidence/103-local-model-catalog/RIGHTS_LEDGER.md`).

---

## 2. Permissive Third-Party Notices Carried Through OpenMed & MedScale

1. **medspaCy ConText** — `https://github.com/medspacy/medspacy` (Apache-2.0)  
   Assertion, negation, temporality, and experiencer rules adapted in [`crates/medscale-clinical/src/data/context_rules.yaml.json`](crates/medscale-clinical/src/data/context_rules.yaml.json), `crates/medscale-clinical/src/clinical_nlp_v4.rs`, and [`integrations/clinical-evidence-python/src/medscale_clinical/clinical/`](integrations/clinical-evidence-python/src/medscale_clinical/clinical/). Upstream source, URL, license, and local modifications are preserved.

2. **Presidio** — `https://github.com/microsoft/presidio` (MIT)  
   Entity recognizer and validator design patterns referenced by `medscale-pack` / `medscale-core` de-identification pipelines. MedScale remains the sole orchestrator through `PrivacyGate` and `privacy_v4`.

3. **ARX Data Anonymization Tool** (`https://github.com/arx-deidentifier/arx`, Apache-2.0) & **DataProfiler** (`https://github.com/capitalone/DataProfiler`, Apache-2.0)  
   Quasi-identifier profiling and `k`-anonymity / `l`-diversity / variational `t`-closeness / prosecutor-journalist-marketer risk models implemented natively in pure Rust (`crates/medscale-clinical/src/privacy_v4.rs`) and stdlib Python.

4. **KServe V2 Inference Protocol** — `https://github.com/kserve/open-inference-protocol` (Apache-2.0)  
   Wire-schema conventions preserved for client-side inference metadata compatibility; no external inference server is bundled.

5. **ICU (International Components for Unicode) 57.1 Grapheme-Break Rules** (ICU License)  
   Source: `https://github.com/unicode-org/icu/blob/0c5873f89bf64f6bbc0a24b84f07d79b25785a42/icu4c/source/data/brkitr/rules/char.txt`  
   Copyright (C) 2002-2016, International Business Machines Corporation and others. All Rights Reserved.  
   Complete notice preserved in `integrations/clinical-evidence-python/src/medscale_clinical/processing/resources/segmenter/ICU.txt` alongside `indic_rules.json`.

6. **`hijridate` Umm al-Qura Month Starts (v2.6.0)** (MIT)  
   Source: `https://github.com/dralshehri/hijridate` (`src/hijridate/ummalqura.py`)  
   Copyright (c) Mohammed Alshehri.  
   1356–1450 AH calendar lookup table embedded in `crates/medscale-clinical/src/data/hijri_umm_al_qura.json` and `integrations/clinical-evidence-python/src/medscale_clinical/core/data/calendars/hijri_umm_al_qura.json`.

7. **`py-nepali` Bikram Sambat Month Lengths (v1.2.0)** (MIT)  
   Source: `https://github.com/opensource-nepal/py-nepali` (`nepali/date_converter.py`)  
   Copyright (c) 2018 The Python Packaging Authority (as stated in the upstream `LICENSE` file).  
   2000–2083 BS calendar lookup table embedded in `crates/medscale-clinical/src/data/bikram_sambat.json` and `integrations/clinical-evidence-python/src/medscale_clinical/core/data/calendars/bikram_sambat.json`.

8. **Chinese Surnames (`baijiaxing`) Data** (CC0-1.0)  
   Public-domain Chinese surname table carried in permissive `medscale_clinical` data assets.

9. **Desktop Typography (SIL Open Font License 1.1)**  
   - **Inter** (`apps/desktop-tauri/src/assets/fonts/Inter-OFL.txt`)  
   - **JetBrains Mono** (`apps/desktop-tauri/src/assets/fonts/JetBrainsMono-OFL.txt`)  
   - Font asset provenance in [`third_party/provenance/095-native-typography.md`](third_party/provenance/095-native-typography.md).

---

## 3. Explicitly Excluded & Quarantined Components (`BLOCKED_EXTERNAL_RIGHTS`)

To preserve Apache-2.0 compatibility and prevent unauthorized redistribution of DUA-protected datasets or restricted medical terminologies, the following upstream/ecosystem assets are **intentionally excluded** and enforced via automated regression and path-guard tests:

- **Copyleft / Non-OSI / Proprietary Software Excluded:**
  - `sdcMicro` (`GPL-2.0-or-later`)
  - `extract-msg` (`.msg` extractor branch, `GPL-3.0`) — only pure stdlib/Rust RFC-5322 `.eml` is supported
  - `MedCAT` (`Elastic-License-2.0`)
  - `Aksharamukha` (`AGPL-3.0`)
  - `com.google.mlkit` (`android/openmedkit-mlkit`, proprietary Google ML Kit terms)
  - `openmed-labs/synthvision` and `openmed-labs/welna` (unspecified GitHub license / `license: null`)
- **DUA-Gated Corpora & Restricted Terminologies Excluded From Bundle:**
  - `i2b2`, `n2c2`, `SHAC`, `MADE`, `THYME`, `MedNLI`, `MIMIC-III/IV`, `SHIELD` (Stanford Redivis DUA), `RadGraph`, `BioRED`, `CBLUE`, `MasakhaNER`
  - `SNOMED CT`, `CPT`, full `UMLS`, full `LOINC` releases
  - Evaluation guards in `crates/medscale-clinical/src/agent_eval_v4.rs` and `medscale_clinical.eval.datasets._dua` require `eval_only=True`, `network_fetch=False`, and reject in-repository dataset paths.

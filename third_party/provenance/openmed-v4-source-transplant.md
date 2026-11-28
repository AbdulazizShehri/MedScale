# Provenance: OpenMed v4.0.0 & OpenMed Agent Source Transplant

| Field | Value |
|---|---|
| Component id | `openmed-v4-source-transplant` |
| Upstream URLs | `https://github.com/maziyarpanahi/openmed`, `https://github.com/openmed-labs/openmed-agent` |
| Revisions / tags | `openmed` tag `v4.0.0` (`5b79cae5df6bc22b0fcdc799be480ee4ffc23c57`); `openmed-agent` tag `v1.0.3` (`ded6e57b5b93cb6dc849c8639662b381179697c7`, Apache-2.0 `README.md` + `LICENSE` only) |
| License / NOTICE / permission | Apache-2.0 (`maziyarpanahi/openmed` `LICENSE` + `NOTICE` preserved verbatim). Bundled third-party attribution requirements retained per component: `medspaCy ConText` (Apache-2.0), `hijridate` 2.6.0 (MIT, Mohammed Alshehri), `py-nepali` 1.2.0 (MIT), `ICU 57.1` (ICU License), `Presidio` (MIT), `DataProfiler` (Apache-2.0), `KServe V2` (Apache-2.0), `ARX` (Apache-2.0), `baijiaxing` surnames (`CC0-1.0`). |
| Local target paths | `integrations/clinical-evidence-python/` (`medscale_clinical`), `integrations/medscale-sdk-ts/` (`@medscale/sdk`), and native Rust/CLI/Desktop waves under `crates/` and `apps/desktop-tauri/` |
| Reproducibility script | `scripts/vendor_openmed_v4.py` |
| Security & Runtime Boundary | Layer 1 Python/TS packages remain reference & client surfaces; Rust Core, CLI, and Desktop run native compiled Rust code with zero Python runtime dependency (`scripts/check-dependency-direction.ps1`). |
| Exclusions (`BLOCKED_EXTERNAL_RIGHTS`) | `extract-msg` (GPL-3.0), `sdcMicro` (GPL-2.0), `MedCAT` (Elastic-2.0), `Aksharamukha` (AGPL-3.0), `com.google.mlkit` (Proprietary), `openmed-labs/{synthvision,welna}` (`license: null`), DUA corpora (`i2b2`, `n2c2`, `SHAC`, `SHIELD`, `MedNLI`, `MIMIC`, `RadGraph`, `BioRED`, `THYME`, `MADE`), restricted terminologies (`SNOMED CT`, `CPT`, full `UMLS`/`LOINC`). |
| Preserved Model Catalog Pin | `third_party/medscale-model-catalog-v3/{models.jsonl,LICENSE,NOTICE}` retains exact verified SHA-256 (`models.jsonl`: `sha256:805e79f7db5514cf308c6deffa67d72c4ce9a59bf4e54487d2783eecbb1e34ab`) and immutable HuggingFace `OpenMed/...` `repo_id` lookup keys. |

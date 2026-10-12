# Spec 103 tasks

Status as of 2026-10-12 (after #211–#221). Capability matrix: [`CAPABILITY_MATRIX.md`](CAPABILITY_MATRIX.md). `[x]` done, `[~]` partly done (the gap is stated), `[ ]` not started.

- [x] T103-01 (P1) `catalog.rs`: `CatalogRow`, `CatalogStatus` (10-state ladder), snapshot source binding (repository, commit, SHA-256), strict JSONL parser with required fields validated and duplicates rejected. Unknown fields are ignored, not kept. Merged in #184.
- [x] T103-02 (P1) Filters: task, family, language, format, architecture, license claim, tier (#211), device fit, status, parameter and disk ceilings; stable ordering; offset pagination. Offset pagination is stable because a catalog snapshot is immutable and rows are sorted by repository id; cursors add nothing here.
- [x] T103-03 (P1) Size estimator from parameter count and format, labelled `estimated`, or `unknown` when it cannot be derived (#184).
- [x] T103-04 (P1) Tests: synthetic 10,000-row import within budget; malformed, duplicate and oversize input; pagination; filters (#184). No-network guarantee: `tests/no_network_103.rs` walks `Cargo.lock` from `medscale-pack` and fails if any networking crate is reachable (#211).
- [x] T103-05 (P1) Core surface `medscale_core::model_catalog` (read-only query, snapshot → Pack), with tests (#184). Dispatched as `Capability::ModelCatalogRead` (#193).
- [x] T103-06 (P2) CLI `medscale packs catalog`, `packs admit-snapshot` (#184) and `packs acquire` (#187).
- [ ] T103-07 (P2) Tauri Models route catalog tab. **Blocked:** needs the Tauri chain (#174 → #177), which is not merge-eligible while F096-T01 is open.
- [x] T103-08 (P3) Governed opt-in acquisition (#187):
  - per-model consent and byte ceiling;
  - Governed Browse transport;
  - Hub host allowlist;
  - pinned-commit files with exact size and digest;
  - atomic staging;
  - hermetic scripted-transport tests.

  File-level resume of interrupted acquisitions from verified staging (#192). Live acquisition through the product CLI runs on CI (#202, run 38037709005). It found and fixed regional CDN hosts (`*.cdn.hf.co`), and repositories updated after the catalog snapshot are acquired at the catalog-pinned revision (#203). The founder workstation lacks the memory for it.
  Byte-range resume inside one file (#207): downloads stream to `<name>.part` and resume with `Range` after an interruption. Live evidence: run 38073595545 resumed a killed download from byte 71,756,361 of 133,813,535 and verified the full digest.
- [~] T103-09 (P4) Budgeted LRU residency pool; Core prepared-model cache migrated to it (#185). Pre-download runtime expectation per catalog row (#190), with per-row execution evidence (`EXECUTED_MODELS`) taking precedence (#211). Runtime-aware variant ranking before acquisition (`rank_variants`, CLI `packs variants`, #218). Per-row batch harness (`model-qualification-batch.yml`): batch 01, run 38301622121, executed 16 of 24 rows (CAPABILITY_MATRIX section 7). **Gap:** rows without their own run still carry an architecture-level expectation; batches continue.
- [~] T103-10 (P5) Execution evidence for four real OpenMed v3.0.0 models: bert, distilbert, roberta, modernbert (`research.md` §9–10). Card-level rights ledger with base-model lineage: `evidence/103-local-model-catalog/RIGHTS_LEDGER.md`. **Gap:** training-dataset terms (BC5CDR, ANATOMY, PII sets) are not verified.
- [x] T103-11 (P6) Catalog-admitted Packs run as Model Fleet lanes (end-to-end test, #188).
- [~] T103-12 Bare model-count claims are blocked by the identity guard (`apps/desktop-tauri/src/identity.test.ts`, on #179). **Gap:** lands with the Tauri chain.
- [x] T103-13 Whole-document execution without truncation: windows, best-context vote and BIOES entity decoding ported from OpenMed v3.0.0 (`token_windows.rs`, #194), with whitespace-trimmed spans.
- [x] T103-14 DeBERTa-v2 and XLM-R on tract: integer `Sign` with `If`-branch rewrite, symbolic `value_info` relaxation, fp16→fp32 widening at load, degenerate-tokenizer refusal, stage-specific prepare diagnostics (#196). Qualification run 37977100903 executed all six architectures.
- [x] T103-15 DeBERTa-v2 first prepare on tract takes 9–13 minutes on CI.
  - **Root cause, measured:** run 38065948181. Almost all of the time is in tract's HIR type and shape inference (`into_typed`: 763.7 s over 923 nodes). Declutter (0.03 s) and optimize (0.05 s) are negligible.
  - **Fix:** opt-in `prepare_with_plan_cache` keeps the decluttered plan as NNEF, keyed by the Pack content digest, runtime contract digest, fixed sequence length and tract version, and used only when its SHA-256 matches. Pack artifacts are verified before any cache use; any cache problem falls back to the full build.
  - **Evidence:** run 38152211383, PII-Dutch-SuperClinical-Small-44M ONNX. A cold prepare took 766.8 s; a warm prepare took 3.55 s, with byte-identical whole-document output. RoBERTa and XLM-R graphs cannot be serialized to NNEF and fall back uncached, with identical output.
  - **Remaining:** the first load of an ONNX-only DeBERTa-v2 row per machine still takes about 13 minutes; the candle runtime (about 3 s from safetensors or MLX) remains the interactive path.
- [~] T103-16 Runtimes beyond tract ONNX. Done: native safetensors on candle (#198), OpenMed MLX `mlx-fp` exports on candle with Apple Metal (#203, #208), GLiNER zero-shot span heads from the MLX exports (#215, Core surface #217). Still open, each needing its own dependency-admission spec: ONNX Runtime (evaluated, not admitted), Core ML, Android, WebGPU, generative and multimodal models (also over the 1 GiB Pack bound).
- [x] T103-17 OpenMed PII post-processing: identifier checksum validators from `clinical_ids.py` in the privacy recognizers (#212); semantic-unit entity merger from `pii_entity_merger.py` as an opt-in on both document runtimes (#216). Not ported: the `india_clinical` code-mixing path.
- [x] T103-18 Optional word-level entity aggregation (`EntityAggregation::Word*`, #218) beside the unchanged OpenMed-compatible decoder. Real-weight comparison (run 38170219043): `met|form|in` becomes `metformin`, `Esc|herichia coli` becomes `Escherichia coli`. No accuracy claim without labelled evaluation.
- [ ] T103-19 Clinical Longformer (20 rows). **Blocked (runtime support):** the omitted-`Pad`-input rewrite (#218) lets tract prepare the export, but tract 0.22.4 then fails in `Gather` at 512 and 1024 tokens (runs 38170322676, 38172472172; contained as `ModelExecution` since #218). candle has no Longformer encoder.
- [x] T103-20 Catalog rows with no `architecture` field: six NER rows whose `config.json` declares bert, roberta, deberta-v2 or modernbert executed on candle (run 38163293053), recorded per row.
- [x] T103-21 Model unload through Core: `PacksResidency` lists prepared models in the tract, candle and GLiNER pools; `PacksUnloadLocal` unloads one Pack's models or every unpinned one (#219).
- [x] T103-22 Opt-in detection normalization for the PII merger (OpenMed `normalize_for_detection`: NFKC per code point, native digits folded, spans mapped back) (#221).
- [x] T103-23 All 13 QwenMed PII ONNX rows executed with their int8 export, the only one under the 1 GiB bound (runs 38152456058, 38260759126). Warm single-window runs were 5.7–9.6 s on some runners and about 106 s on others: executable, not interactive on CPU.

# OpenMed ↔ MedScale model-execution capability matrix (Spec 103, issue #183)

**Baseline:** OpenMed v3.0.0, `maziyarpanahi/openmed@ea920f36fadd7b45935247d639f0ffa1ef493b23`, source-read. Newer upstream changes are not part of this baseline.
**MedScale:** `main` (Spec 103 PRs #184–#203 and Spec 104 PR #198 merged). A row's status changes only with merged code and recorded evidence.
**Evidence base:** real-model qualification on free GitHub runners (Linux x64 CPU, synthetic text):
- ONNX on tract: run [37984399447](https://github.com/TheHalfMoon/MedScale/actions/runs/37984399447), on the exact code of #196;
- safetensors on candle: run [37986892863](https://github.com/TheHalfMoon/MedScale/actions/runs/37986892863). Each model's catalog binding, per-file digests, signed Pack and admission were verified before execution.

Status vocabulary:
- **IMPLEMENTED:** merged with tests;
- **IN REVIEW:** implemented in an open PR;
- **PARTIAL:** the gap is stated;
- **NOT IMPLEMENTED:** no MedScale code;
- **BLOCKED:** the named external constraint applies.

## 1. Lifecycle (DISCOVER → INSPECT → ACQUIRE → VERIFY → ADMIT → LOAD → RUN → COMPARE → UNLOAD)

| Stage | OpenMed v3.0.0 (source) | MedScale | Status |
|---|---|---|---|
| Discover / search | `models.jsonl` (2,266 rows), `core/model_registry.py`, `core/model_search.py` | Bounded JSONL import, 10-state status ladder, filters, stable order, pagination (`medscale-pack/src/catalog.rs`). Core capability `ModelCatalogRead` (#193). CLI `packs catalog` | IMPLEMENTED (no tier filter; offset pagination) |
| Inspect before download | model cards, size estimates | Size estimate labelled `estimated`. Pre-download `runtime_expectation` per row from recorded evidence (`compatibility.rs`), including known-defective exports | IMPLEMENTED (expectations are architecture-level) |
| Acquire | `huggingface_hub.snapshot_download`, `core/hf_hub.py`; Android `ModelDownloader.kt` | Per-model consent with a byte ceiling, Governed Browse transport, Hub host allowlist, pinned commit, atomic staging (#187). File-level resume of interrupted acquisitions (#192) | IMPLEMENTED (#187, #192, #202, #203). Live acquisition through the product CLI runs on CI (run 38037709005). Repositories updated after the catalog snapshot are acquired at the catalog-pinned revision found in their commit history (#203). No byte-range resume. The live CLI acquisition is not exercised on the founder workstation (memory) |
| Verify | `core/model_integrity.py`, pinned revisions | OpenMed reproducibility hash ported; per-file LFS SHA-256 or git-blob SHA-1; exact sizes; gated and private repositories refused (`hf_snapshot.rs`) | IMPLEMENTED |
| Admit | n/a (OpenMed loads directly) | Signed MedScale Pack (synthetic trust root), anti-rollback, provenance, `PacksInstallLocal` | IMPLEMENTED (production signing `NOT_GRANTED`) |
| Load / prepare | `onnx/inference.py` `OnnxModel.from_pretrained`; `core/backends.py` | tract static-shape plan. Normalisations: `Shape`/`Cast→TDim`, integer `Sign`, `If` branches, symbolic `value_info` relaxation, fp16→fp32 widening. Degenerate tokenizers refused (#196) | IMPLEMENTED for 6 architectures (#196) |
| Run | `OnnxModel.predict_batch_detailed` (windows, best-context vote, entity decoding) | Single-window `run`. `run_document`: whole-document windows, best-context vote and BIOES entity decoding, ported from OpenMed (#194) | IMPLEMENTED (#194). No multi-window batching |
| Compare | n/a | Model Fleet lanes over catalog-admitted Packs (#188); factual comparison, never a ranking | IMPLEMENTED |
| Unload / residency | `core/model_cache_policy.py`, `ModelLoader.unload_model`, `onnx/ram_budget.py` | `ResidencyPool`: budgeted LRU, pinning, plan-before-evict (#185); Core prepared-model cache on it (2 GiB) | IMPLEMENTED (no RSS probe; budget by artifact size) |

## 2. Runtimes and platforms

| Runtime / platform | OpenMed v3.0.0 | MedScale | Status |
|---|---|---|---|
| ONNX on CPU | ONNX Runtime `CPUExecutionProvider`; fp32 / int8 variants; threads | tract 0.22.4 (pure Rust, no native runtime download). fp32 and fp16 (widened) | IMPLEMENTED. int8 exports not prepared (quantized operators) |
| ONNX Runtime (native) | yes | Evaluated, not admitted: tract covers all six qualified architectures. ORT adds a native binary to the supply chain; it would mainly help DeBERTa prepare time and int8 | NOT IMPLEMENTED by decision. Revisit if int8 or prepare time becomes a product requirement |
| PyTorch / Transformers weights | `HuggingFaceBackend` (Python) | **Native safetensors runtime on `candle` 0.9.1** (Spec 104): no Python, no pickle, no executable code. Encoders from `candle-transformers`, MedScale-built heads, same windowing and decoding | IMPLEMENTED (#198). 5 architectures executed on CI; scores match the ONNX runs to about four decimals. Pickle-only weights are refused |
| Apple MLX | `openmed/mlx/*` (bert, deberta-v2, modernbert, longformer, gliner heads) | **MLX `mlx-fp` exports run on any platform** through the candle runtime: OpenMed's key renaming is ported (`mlx_layout.rs`), and tensor values are unchanged (#203). Apple GPU execution through candle's Metal backend (#208, run 38083242539: `device=metal`, scores equal to CPU to about 6 decimals) | IMPLEMENTED for token classification: bert, distilbert, roberta and deberta-v2 MLX exports executed (runs 38039625442, 38041300631; scores identical to the safetensors runs). Quantized MLX refused; GLiNER MLX heads not implemented |
| Core ML | export only (`coreml` extra) | — | NOT IMPLEMENTED |
| OpenVINO / TensorRT / GGUF | exporters and sessions | — | NOT IMPLEMENTED |
| Android | `android/openmedkit` (ORT Mobile, accelerator fallback, model cache/downloader) | No Android surface | NOT IMPLEMENTED |
| Browser / WebGPU | `js/openmedkit-web` (ORT Web, WebGPU session) | No browser runtime; the Tauri desktop is blocked (F096-T01) | NOT IMPLEMENTED |
| Backend auto-selection | `get_backend`: MLX → HF → ONNX by availability | A single backend. Pre-download expectations replace availability probing | PARTIAL |
| Offline mode | `core/offline.py` (`local_files_only`, socket blocking) | Default-deny egress everywhere. The runtime has no network code; acquisition is the only egress and needs consent | IMPLEMENTED (stricter by default) |

## 3. Architectures (ONNX token classification, measured)

| Architecture | Catalog rows | Model qualified (commit) | Prepare / warm run (CI) | Status |
|---|---|---|---|---|
| bert | 754 | DiseaseDetect-ElectraMed-33M (`54c9cc11`) | 0.9 s / 0.36 s | EXECUTED_TESTED |
| distilbert | 204 | AnatomyDetect-TinyMed-65M (`ad5e79cb`) | 0.8 s / 0.17 s | EXECUTED_TESTED |
| roberta | 209 | AnatomyDetect-TinyMed-82M (`95f0bf74`) | 1.8 s / 0.17 s | EXECUTED_TESTED |
| modernbert | 227 | AnatomyDetect-ModernClinical-149M (`66525717`) | 3.6 s / 0.9 s | EXECUTED_TESTED |
| deberta-v2 | 304 | PII-Dutch-SuperClinical-Small-44M (`e41c8009`) | 769 s / 0.41 s | EXECUTED_TESTED on CI (#196). First prepare is not interactive on tract; the candle path prepares the same model in 2.8 s |
| xlm-roberta | 366 | PII-Spanish-BigMed-Large-278M fp16 (`da3fec60`) | 4.6 s / 0.6 s | EXECUTED_TESTED on CI (#196). The **65 XLM-R NER exports ship a degenerate tokenizer** (BPE, 0 merges) and are refused |
| gliner, eurobert, qwen, others | 91, 33, 30, … | — | — | NOT IMPLEMENTED (span heads, generative and vision models need different runtimes) |

### Safetensors path (Spec 104, candle; PyTorch-format rows)

| model_type | Model qualified (commit) | Prepare / warm run (CI) |
|---|---|---|
| bert | NER-AnatomyDetect-ElectraMed-33M (`3c914174`) | 0.45 s / 22 ms |
| distilbert | NER-AnatomyDetect-TinyMed-65M (`1ab9359d`) | 0.80 s / 43 ms |
| roberta | NER-AnatomyDetect-TinyMed-82M (`724a3702`) | 0.86 s / 39 ms |
| deberta-v2 | PII-Dutch-SuperClinical-Small-44M (`ee9cafad`) | 2.80 s / 122 ms |
| modernbert | NER-AnatomyDetect-ModernClinical-149M (`3c0bdcc3`) | 1.89 s / 68 ms |

PyTorch-format XLM-R (1.11 GB) exceeds the 1 GiB Pack bound and is refused.

### Pre-download expectations across the catalog (2,266 rows)

These are architecture-level **expectations**, not tests:

| Expectation | Rows |
|---|---|
| Expected runnable via ONNX on tract | 646 |
| Expected runnable via safetensors on candle | 582 |
| Expected runnable via MLX `mlx-fp` exports on candle | 474 |
| Known blocked: XLM-R NER defective tokenizer | 65 |
| Known blocked: XLM-R over the 1 GiB bound (PyTorch-format 122, MLX 122) | 244 |
| Known blocked: GLiNER, pickle-only weights (`pytorch_model.bin`) | 91 |
| Known blocked: quantized MLX exports | 6 |
| Untested (zero-shot MLX, unevidenced architectures, generative, vision) | 158 |

Expected runnable in total: **1,702 of 2,266**. Each runtime and format is backed by real runs of one model per architecture; individual rows are not tested.

**Correction (2026-10-10).** An earlier revision of this table, merged in #199, counted 1,109 safetensors candidates and 244 XLM-R rows over the bound. OpenMed lists its `*-mlx` repositories with formats `["mlx-fp", "pytorch"]`, but those repositories publish MLX weights only. They are now counted as MLX-only, here and in the runtime expectation (#198).

"Catalog rows" are listing counts. Only the named models are `EXECUTED_TESTED`. No row is `TASK_QUALIFIED` (no labelled evaluation) or `CLINICALLY_VALIDATED`.

## 4. Task families

| Task | OpenMed v3.0.0 | MedScale | Status |
|---|---|---|---|
| NER (token classification) | `ner/*`, ONNX/HF/MLX pipelines | Runtime above; evidence-only proposals | IMPLEMENTED |
| PII token classification | PII models plus `core/pii.py`, entity merger, validators, anonymizer | The PII models run as token classifiers (DeBERTa and XLM-R evidence). OpenMed's merger, validators and anonymizer are not ported; MedScale's privacy gate (Specs 079–082) is separate | PARTIAL |
| Zero-shot (GLiNER) | `ner/families/gliner*.py`, `mlx/models/gliner_*` | — | NOT IMPLEMENTED. Also BLOCKED: the 91 rows ship pickle only |
| Multimodal | `openmed/multimodal/*` | — | NOT IMPLEMENTED |

## 5. External constraints (precise)

- **F096-T01:** the Tauri dependency advisory (`glib`, Linux). The founder has **not** accepted it. It blocks the Tauri chain (#174 → #179) and therefore the Models UI (T103-07).
- **The OpenMed XLM-R NER exports** need upstream republishing of their tokenizers.
- **PyTorch-format XLM-R (122 rows)** exceeds the 1 GiB Pack bound. Raising the bound is a governed decision; it has not been taken.
- **GLiNER zero-shot (91 rows)** ships pickle weights only, which are refused (R103-06). It needs upstream safetensors.
- **Production signing** is `NOT_GRANTED`; **real PHI** is `NOT_AUTHORIZED`; **clinical validation** is `NOT_PERFORMED`.
- **Training-dataset terms** (BC5CDR, ANATOMY, PII sets) are unverified ([rights ledger](../../evidence/103-local-model-catalog/RIGHTS_LEDGER.md)).

## 6. Update (2026-10-11)

This section supersedes the rows above where they differ. Status vocabulary as in the header.

| Area | Change | Evidence | Status |
|---|---|---|---|
| Catalog search | Tier filter (case-insensitive; rows without a tier never match). Per-row execution evidence (`EXECUTED_MODELS`) is reported before architecture heuristics. A test walks `Cargo.lock` to prove `medscale-pack` cannot reach a network crate (T103-04) | #211 | IMPLEMENTED |
| PII validators | OpenMed `clinical_ids.py` checksum and structure rules (US SSN and phone, Luhn, NPI, IBAN, BIC, UK NHS and NINO, Canadian SIN, Australian Medicare and TFN) wired into the privacy recognizers | #212 | IMPLEMENTED |
| PII entity merger | OpenMed `pii_entity_merger.py`: 27 semantic-unit patterns, context scoring, validators, dominant label, span union, coalescing. Opt-in `pii_semantic_merge` on both document runtimes. The `india_clinical` path (abbreviation expansion, Latin/Devanagari and Latin/Telugu span bridging) is ported with synthetic fixtures | #216, india_clinical PR | IMPLEMENTED |
| ONNX prepare time | Integrity-checked tract plan cache (NNEF, SHA-256 sidecar, keyed by Pack digest and tract version). DeBERTa-v2 first prepare is 686–767 s; a cached prepare takes 3.2–3.6 s with identical output. RoBERTa and XLM-R graphs cannot be serialized and fall back uncached (T103-15) | #214; runs 38152211383, 38154652812 | IMPLEMENTED |
| GLiNER zero-shot | OpenMed `gliner_span.py`, `gliner_common.py` and `inference.py` ported to candle (DeBERTa-v2 encoder, BiLSTM, span marker, prompt scorer, flat NER). Runs the `ZeroShot-NER-*-mlx` exports (safetensors, family `gliner-uni-encoder-span`). Core `PacksEvaluateLocal` takes `zero_shot_labels` | #215, #217; runs 38161626788 (Anatomy-Small-166M), 38163323960 (Disease-Medium-209M) | IMPLEMENTED |
| Word-level aggregation | Opt-in `EntityAggregation::Word{First,Average,Max}` (Hugging Face word strategies over the tokenizer's word ids) beside the unchanged OpenMed-compatible decoder. On real weights `met|form|in` becomes `metformin` and `Esc|herichia coli` becomes `Escherichia coli`; no accuracy claim without labelled evaluation | #218; run 38170219043 | IMPLEMENTED |
| Backend selection | `rank_variants` (CLI `packs variants`): a model's format siblings ranked before download, MLX on Metal first on Apple silicon, safetensors first elsewhere, recorded execution as tie-break. Pre-download counterpart of OpenMed `get_backend` | #218 | IMPLEMENTED |
| ONNX robustness | Omitted optional `Pad` inputs dropped before tract parses the graph (exact); tract panics during execution are contained as `ModelExecution`; plan-cache format v2 | #218 | IMPLEMENTED |
| Model unload | `PacksResidency` lists the prepared models in the tract, candle and GLiNER pools; `PacksUnloadLocal` unloads one Pack's models or every unpinned one (counterpart of OpenMed `ModelLoader.unload_model`) | #219 | IMPLEMENTED |
| PII detection normalization | Opt-in NFKC per code point plus native-digit folding (OpenMed `normalize_for_detection`) before the PII merger's patterns, with spans mapped back to the source; fullwidth SSNs validate, fullwidth IBANs are found | #221 | IMPLEMENTED |
| EuroBERT, Qwen | First tract executions: PII-EuroMed-Large-210M (`model.onnx`, prepare 5.0 s) and PII-Chinese-QwenMed-XLarge-600M (`model_int8.onnx`, the only export under the 1 GiB bound; warm run 6.2 s) | #211; run 38152456058 | EXECUTED_TESTED (these rows) |
| Rows without `architecture` | Six catalog rows list no architecture; their `config.json` declares bert, roberta, deberta-v2 or modernbert. All six ran on candle at their pinned revisions | run 38163293053 | EXECUTED_TESTED (these rows) |
| Longformer (20 rows) | At a 128-token sequence the export reshapes to a 512-token window (run 38152456058). At 512 tokens tract rejected an omitted optional `Pad` input (run 38161872272); #218 drops it exactly, after which the export prepares but tract 0.22.4 fails in `Gather` at 512 and 1024 tokens (run 38172472172), now contained as `ModelExecution` instead of a panic. **PyTorch-format rows** run on an exact candle local-window path (windows of at most `attention_window / 2 + 1` tokens, no global attention): run 38300309054 matches Hugging Face `LongformerForTokenClassification` entity for entity (max score delta 1.2e-6), with a BERT control (8.9e-8) validating the harness `scripts/qualification/hf_reference_compare.py` | as cited | IMPLEMENTED (PyTorch rows); BLOCKED (ONNX rows, tract) |
| privacy-filter (13 rows) | Weights are 1.49 GB (8-bit MLX, quantized) to 2.80 GB | Hub sizes at review | BLOCKED (1 GiB Pack bound; quantized MLX refused) |
| Generative and vision rows (12) | Text generation and visual question answering, 1.3–20.6 GB | Hub sizes at review | NOT IMPLEMENTED (outside token classification; over the Pack bound) |

**Decoding parity note.** On unprefixed or `B-`-per-subword label schemes, several models emit adjacent fragments (`met` + `form` + `in`). MedScale's decoder is line-for-line OpenMed's ONNX `_decode_entities`, and OpenMed's default `analyze_text` path uses Hugging Face `aggregation_strategy="simple"`, which also starts a new entity at each `B-`. The fragments therefore match OpenMed's own output; they are not a MedScale defect.

**GLiNER residual.** 26 of the 52 zero-shot MLX exports (Small and Medium) fit the 1 GiB bound. The untiered Base and the Large exports (26) do not. The 91 PyTorch GLiNER rows remain pickle-only and refused.

### Pre-download expectations across the catalog (2,266 rows, recomputed 2026-10-11)

Computed by `runtime_expectation` (CLI `packs catalog`, paged) at the evidence/docs PR head. Expectations are not tests; only `ExecutedTested` rows were run.

| Expectation | Rows |
|---|---|
| Executed (this exact row, recorded run): tract ONNX 20, candle safetensors 11, candle MLX 4, candle GLiNER 2 | 37 |
| Expected runnable via ONNX on tract | 656 |
| Expected runnable via safetensors on candle | 577 |
| Expected runnable via MLX `mlx-fp` exports on candle | 470 |
| Expected runnable via GLiNER zero-shot (Small and Medium MLX exports) | 24 |
| Known unsupported: PyTorch-format XLM-R over the 1 GiB bound | 244 |
| Known unsupported: GLiNER PyTorch rows, pickle only | 91 |
| Known unsupported: XLM-R NER exports with a degenerate tokenizer | 65 |
| Known unsupported: GLiNER Base and Large MLX exports over the bound | 26 |
| Known unsupported: Clinical Longformer (tract `Gather` failure) | 20 |
| Known unsupported: EuroBERT PyTorch rows (no candle encoder; ONNX runs) | 16 |
| Known unsupported: Qwen PyTorch rows (no candle encoder; int8 ONNX runs) | 13 |
| Known unsupported: privacy-filter (over the bound) | 13 |
| Known unsupported: quantized MLX | 1 |
| Untested: one row without metadata | 1 |
| Task outside token classification (generation, VQA) | 12 |

Executed or expected runnable: **1,764 of 2,266** (was 1,702 on 2026-10-10). All 13 QwenMed PII ONNX rows were executed with `model_int8.onnx` (runs 38152456058, 38260759126); their warm single-window run took 5.7–9.6 s on some runners and about 106 s on others, so they are not interactive on CPU.

## 7. Per-row batch qualification (2026-10-13)

`.github/workflows/model-qualification-batch.yml` qualifies up to 24 pinned catalog rows per run on free GitHub runners, one job per row (synthetic input only). `scripts/qualification/record_results.py <run>` turns successes into `EXECUTED_MODELS` entries; failures stay here as negative evidence.

**Batch 01** (run 38301622121, 24 rows): **16 executed** (8 tract ONNX incl. 4 fp16 exports, 7 candle safetensors, 1 candle MLX). Failures:

| Row(s) | Failure | Classification |
|---|---|---|
| `BioClinicalBERT-Base-110M-v1-mlx`, `ClinicalE5-Small-33M-v1-mlx`, `Dutch-BioClinicalBERT-Base-110M-v1-mlx` (PII) | repository ships no `tokenizer.json` | EXTERNAL (upstream packaging) |
| `PII-BigMed-Large-278M-v1-onnx-android` | degenerate tokenizer (already on the exact `KNOWN_DEFECTIVE_EXPORTS` list) | EXTERNAL (upstream export) |
| `PII-Arabic-BigMed-Large-560M-v1`, `PII-Chinese-BigMed-Large-560M-v1` (ONNX) | weights in an external-data sibling (tract loads single-file models only); 560M parameters are over the 1 GiB bound even at fp16, now `EstimatedOverBound`; the selector skips external-data files | GATED (1 GiB bound; not raised) |
| `PII-Dutch-BioClinicalBERT-Base-110M-v1` (safetensors) | legacy `LayerNorm.gamma`/`beta` tensor names | **fixed** in the runtime (renamed on load like Transformers); requalify in batch 02 |
| `ClinicalNER-SuperClinical-Large-434M-v1` fp16 ONNX | tract DeBERTa-v2 prepare exceeded the 90-minute job timeout | REPOSITORY (runtime performance) |

**Batch 02** (run 38314653881, 24 rows, mostly Dutch PII): **21 executed** (12 tract ONNX incl. 5 fp16 exports, 9 candle safetensors), including `OpenMed-PII-Dutch-BioClinicalBERT-Base-110M-v1`, which batch 01 could not load (legacy `gamma`/`beta` names, now fixed). Failures:

| Row(s) | Failure | Classification |
|---|---|---|
| `PII-Dutch-FastClinical-Small-82M-v1`, `PII-Dutch-SuperMedical-Base-125M-v1` (safetensors) | `config.json` saved by Transformers 5.1 omits the default `position_embedding_type` | **fixed** (normalized to `"absolute"`, as Transformers does); requalify in batch 03 |
| `PII-Dutch-BioClinicalModern-Base-149M-v1` (safetensors) | ModernBERT rotary bases moved to `rope_parameters` (Transformers 5.1) | **fixed** (mapped to `global_rope_theta`/`local_rope_theta`; non-default rotary types refused); requalify in batch 03 |

**Batch 03** (run 38320247418, 21 rows, French PII plus requalification): **21 of 21 executed** (10 tract ONNX incl. 3 fp16 exports, 11 candle safetensors). This includes the three Transformers 5.1 rows from batch 02 (`Dutch-FastClinical-Small-82M`, `Dutch-SuperMedical-Base-125M`, `Dutch-BioClinicalModern-Base-149M`) and two more Clinical Longformer rows (Dutch, French) on the exact candle path. Note: tract took 1,327 s to prepare the DeBERTa-v2 `French-SuperClinical-Base-184M` ONNX export (warm run 0.7 s), against 1–6 s for the other ONNX rows. Prepare cost for DeBERTa-v2 graphs is the open runtime-performance item (batch 01's large fp16 export exceeded 90 minutes).

**Batch 04** (run 38331707946, 24 rows, French and German PII): **24 of 24 executed** (13 tract ONNX incl. 6 fp16 exports, 11 candle safetensors, among them a German Clinical Longformer row). The new `PREPARE_STAGES` line shows where tract spends its preparation time. For the two DeBERTa-v2 exports, typing and declutter took 791 s (`French-SuperClinical-Small-44M`) and 1,584 s (`French-mSuperClinical-Large-279M`, fp16), while load took 0.4–1.4 s and optimize under 0.1 s. Every other architecture prepared in 0.5–6.4 s. This is the T103-15 root cause (tract HIR type and shape inference) measured per stage; the plan cache (#214) pays it once per model.

**Batch 05** (run 38335975188, 24 rows, German and Hindi PII): **24 of 24 executed**. The three DeBERTa-v2 ONNX exports again spent nearly all of their preparation in typing (582 s, 1,392 s, 1,575 s), while the same `German-SuperClinical-Small-44M` model in safetensors prepared on candle in 3.4 s.

**DeBERTa-v2 preparation fix** (T103-15): the typing cost above came from tract eagerly evaluating the constant relative-position `MatMul`s with its reference loop. With `hir_const_matmul`, the same products run through tract's optimized kernels. Locally, typing dropped from 999 s to 1.7 s, and the logits matched the unwrapped graph to 1.14e-5 with identical labels. The next batch run measures it on CI.

**Batch 06** (run 38343611678, 24 rows): **24 of 24 executed**. It includes seven `SuperClinical-Large-434M` DeBERTa-v2 fp16 exports, among them `ClinicalNER-SuperClinical-Large-434M`, which exceeded the 90-minute job limit in batch 01. With the constant-MatMul fix, the large exports prepared in 9.0–11.1 s (typing about 4.7 s), DeBERTa-v2 Base in 5.9 s and Small in 3.6 s. Batches 04–05 had measured 582–1,575 s of typing for the same architecture. The T103-15 fix is confirmed on CI.

**Batch 07** (run 38347129440, 24 rows, Italian, Portuguese, Japanese and Korean PII): **24 of 24 executed**, including the Japanese and Korean `NomicMed-Large-395M` fp16 exports and a Portuguese MLX export.

**Batch 08** (run 38350788210, 24 rows, Portuguese PII): **24 of 24 executed** (tract ONNX, candle safetensors and six MLX exports; the DeBERTa-v2 `SuperClinical-Large-434M` fp16 export prepared in 10.2 s).

**Batch 09** (run 38354857673, 24 rows, Portuguese and Spanish PII): **24 of 24 executed**, including the Spanish DeBERTa-v2 `SuperClinical-Large-434M` fp16 export and a Spanish Clinical Longformer row.

**Batch 10** (run 38358974839, 24 rows, Spanish, English and Telugu PII): **22 of 24 executed**. The two failures (`Telugu-EuroMed-Large-210M`, `Telugu-FastClinical-Small-82M` ONNX) were `TokenBound`: the qualification test used the single-window `run`, and the Telugu sample exceeds 128 tokens under these byte-level tokenizers. The test now uses the product's whole-document `run_document`, as the candle test already did. Both rows are requalified in the next batch.

**Batch 11** (run 38363574699, 24 rows, Telugu and Turkish PII): **24 of 24 executed**, including the two Telugu ONNX rows batch 10 could not run, now through the whole-document path.

**Batch 12** (run 38368604917, 24 rows, Turkish and Vietnamese PII): **24 of 24 executed**.

**Batch 13** (run 38373925933, 10 rows, Vietnamese PII and remaining groups): **10 of 10 executed**; the last rows in groups without a run (later batches use `--per-row`).

**Batch 14** (run 38374107804, 48 rows, NER rows, first per-row batch): **48 of 48 executed**.

**Batch 15** (run 38381786129, 48 rows, BloodCancerDetect NER, per-row): **48 of 48 executed**.

**Batch 16** (run 38387956773, 48 rows, BloodCancerDetect and ChemicalDetect NER, per-row): **47 of 48 executed**. The one failure (`BloodCancerDetect-TinyMed-65M-mlx`) was a download transport error during acquisition, not a runtime result; the row is retried in a later batch.

**Batch 17** (run 38393867391, 48 rows, BloodCancerDetect, ChemicalDetect and DNADetect NER, per-row): **48 of 48 executed**, including the `BloodCancerDetect-TinyMed-65M-mlx` retry from batch 16.

**Batch 18** (run 38434804059, 48 rows, DNADetect and DiseaseDetect NER, per-row): **48 of 48 executed**.

**Batch 19** (run 38443895041, 48 rows, DiseaseDetect NER, per-row): **48 of 48 executed**.

**Batch 20** (run 38451112364, 48 rows, DiseaseDetect and GenomeDetect NER, per-row): **45 of 48 executed**. The three failures (`GenomeDetect-BioMed-109M` ONNX, `GenomeDetect-BioMed-335M` safetensors and ONNX) were `HTTP 429` from the Hub metadata API during acquisition, not runtime results; acquisition now retries temporary refusals (#237) and the rows are retried.

**Batch 21** (run 38459163759, 48 rows, GenomeDetect and GenomicDetect NER, per-row): **48 of 48 executed**, including the three `GenomeDetect-BioMed` rows that batch 20 lost to HTTP 429; this run used the acquisition retry from #237.

**Batch 22** (run 38466509604, 48 rows, GenomicDetect and OncologyDetect NER, per-row): **39 of 48 executed**. The nine failures were Hugging Face errors during acquisition (seven `HTTP 500`, two `HTTP 429` that persisted through the retries), consistent with a temporary Hub incident; none is a runtime result, and the rows are retried.

**Batch 23** (run 38473404618, 48 rows, GenomicDetect and OncologyDetect NER, per-row): **48 of 48 executed**, including the nine rows batch 22 lost to the Hub incident.

**Batch 24** (run 38479600818, 48 rows, OncologyDetect and OrganismDetect NER, per-row): **48 of 48 executed**.

**Batch 25** (run 38485176124, 48 rows, OrganismDetect and PathologyDetect NER, per-row): **48 of 48 executed**.

**Batch 26** (run 38490332415, 48 rows, PathologyDetect and PharmaDetect NER, per-row): **48 of 48 executed**.

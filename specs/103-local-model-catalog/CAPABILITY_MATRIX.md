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
| PII entity merger | OpenMed `pii_entity_merger.py`: 27 semantic-unit patterns, context scoring, validators, dominant label, span union, coalescing. Opt-in `pii_semantic_merge` on both document runtimes. Not ported: the `india_clinical` path | #216 | IMPLEMENTED |
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


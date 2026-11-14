# Spec 105: Runtime families beyond token classification

**Status:** `PROMOTED` under the founder directive of 2026-10-12 (issue #183: "Split missing runtime families into small specs with dependency review and tests. Start bounded, governed specs for the next applicable runtime families: generative and multimodal first, then the other relevant ONNX Runtime, Core ML, Android and WebGPU paths").
**Depends on:** Spec 103 (catalog, acquisition, verification, signed Packs, residency), Spec 104 (candle admission, safetensors Packs).
**Research and dependency review:** [research.md](research.md). **Tasks:** [tasks.md](tasks.md).

## Goal

Extend MedScale's governed local runtimes past token classification, family by family, without new security exceptions. Each family is a small slice with its own dependency review, synthetic tests and, where catalog rows fit the unchanged constraints, real-model qualification on free CI.

## Constraints (unchanged)

- The 1 GiB Pack **artifact** bound is not raised. Re-sharding a larger model into several ≤1 GiB artifacts would defeat the bound's purpose (bounded memory per model) and is treated as the same founder decision, not as a workaround.
- No pickle, no `trust_remote_code`, no network at runtime, synthetic inputs only (real PHI is not authorized), evidence-only outputs.
- New crates pass the repository `cargo-deny` and supply-chain policy unchanged; no build-time downloads.

## Requirements

- **R105-01 Text generation (phase A).** Runtime id `candle_text_generation_v1` on the already admitted `candle-transformers =0.9.1` (no new crate). Fixed `model_type` allowlist: `qwen2`, `mistral`. Prepare verifies every artifact digest against the admitted manifest. Decoding is greedy and deterministic, bounded by `max_new_tokens` ≤ 256 and a context of at most 4,096 tokens (or the model's limit), and stops at the configured end-of-sequence ids. Prompt ≤ 16 KiB. The KV cache is reset per call; no prompt is retained. Output: an evidence-only proposal (`generated_text`, token counts, stop reason).
- **R105-02 Generation Packs.** `build_text_generation_pack` builds a signed Pack only from a `text-generation` catalog row, verified against the catalog hash and per-file Hub digests, with `.safetensors` weights only.
- **R105-03 Catalog honesty.** OpenMed v3.0.0 lists 12 generative or vision rows, all 1.31–20.6 GB (research §1). None fits the unchanged bound, so none is executable; phase A is qualified with synthetic models only, and the runtime expectation keeps those rows outside the runtime.
- **R105-04 Multimodal (phase B, spec only).** Vision-language rows (Qwen2.5-VL, Ministral-3B-MedVL, Qwen3.5-MedVL) need an image pipeline, a vision encoder and a multimodal Pack contract (image bounds, decoder limits, no EXIF or path leakage). All are over the bound; implementation waits for the founder decision.
- **R105-05 Native ONNX Runtime (spec only).** Admission requires a native library with verifiable provenance and no build-time download (research §3). Not admitted in this slice.
- **R105-06 Core ML, Android, WebGPU (spec only).** OpenMed v3.0.0 publishes no Core ML artifacts; Apple GPU execution already runs through candle Metal (#208). MedScale has no Android or browser product surface in the adopted plan. Recorded as not applicable until a product surface or artifact exists (research §4).

## Non-goals

Sampling (temperature, top-p), chat templates, streaming, tool calls, fine-tuning, any clinical use, any accuracy claim.

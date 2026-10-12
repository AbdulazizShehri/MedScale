# Spec 105 research and dependency review

Snapshot: OpenMed v3.0.0 `maziyarpanahi/openmed@ea920f36` catalog (2,266 rows); Hub sizes read 2026-10-11.

## 1. Generative and vision rows in the catalog

| Repository | Task | Architecture (catalog) | Largest weight file |
|---|---|---|---|
| `OpenMed/laneformer-2b-it-q4-mlx` | text-generation | laneformer (MLX 4-bit) | 1.31 GB |
| `OpenMed/maple-preview-4bit-mlx` | text-generation | maple-preview | 5.36 GB (10.78 GB total) |
| `OpenMed/maple-preview-8bit-mlx` | text-generation | maple-preview | 5.24 GB (20.57 GB total) |
| `OpenMed/maple-preview-4bit-onnx-android`, `-onnx-webgpu` | text-generation | maple-preview (ONNX, external data) | — |
| `OpenMed/Ministral-3B-Medical-v1`, `-MedVL`, `-PII-Preview` | generation / VQA | Ministral 3B | 7.70 GB |
| `OpenMed/Qwen2.5-3B-MedVL` | VQA | Qwen2.5-VL | 5.00 GB (7.51 GB total) |
| `OpenMed/Qwen3.5-2B-MedVL` | VQA | Qwen3.5 | 4.43 GB |
| `OpenMed/Qwen2.5-VL-3B-Medical-v1`, `Qwen3.5-2B-Medical-v1` | unknown | qwen | metadata unavailable |

Conclusion: no generative or vision row fits the 1 GiB artifact bound. Several also use architectures without a candle implementation at 0.9.1 (laneformer, maple-preview, Qwen3.5) or quantized MLX layouts that MedScale refuses.

## 2. Text generation on admitted crates

`candle-transformers =0.9.1` (admitted by Spec 104, `default-features = false`) already contains `qwen2`, `mistral`, `llama`, `phi3`, `gemma3` decoders and `generation::LogitsProcessor`. Phase A uses `qwen2::ModelForCausalLM` and `mistral::Model` (both expose `forward(ids, offset)` with an internal KV cache and `clear_kv_cache()`), with greedy arg-max decoding implemented locally. No new crate, no new feature flag, so `cargo deny` and the supply-chain policy are unchanged. `llama` is left out of phase A because its API takes an external cache object; adding it is a small follow-up.

## 3. Native ONNX Runtime

The `ort` crate (2.x) obtains the native library either by downloading prebuilt binaries at build time (`download-binaries`) or by dynamic loading of a system copy (`load-dynamic`). Build-time download conflicts with the hermetic, reviewed supply chain; dynamic loading moves a native binary outside `Cargo.lock` provenance. Admission therefore needs a vendored, digest-pinned library per platform with a license and update plan. Benefit today is limited: tract runs every qualified ONNX architecture, the plan cache made DeBERTa-v2 interactive after the first load, and the remaining ONNX blocker (Clinical Longformer) has a candle path. Not admitted in this slice.

## 4. Core ML, Android, WebGPU

- **Core ML:** the catalog lists formats `pytorch`, `onnx`, `mlx-fp`, `mlx-8bit`, `mlx-4bit`; no Core ML artifacts. Apple GPU execution already runs through candle Metal (#208, run 38083242539). A Core ML path would need MedScale to convert models (OpenMed's `coreml/convert.py` uses Python `coremltools`), which adds a conversion supply chain for no catalog artifact. Not applicable today.
- **Android and WebGPU:** OpenMed's `android/openmedkit` and `js/openmedkit-web` serve Android and browser apps. MedScale's adopted product surfaces are the Rust CLI/Core and the Tauri desktop (blocked by F096-T01). Not applicable until such a surface is adopted.

# Spec 104: Native safetensors token-classification runtime (Candle)

**Status:** `PROMOTED` under the founder directive of 2026-10-09 (issue #183: "Add isolated PyTorch/Transformers execution where appropriate").
**Depends on:** Spec 103 (catalog, acquisition, verification, signed Packs, residency, document windowing), Spec 069 (runtime contract), Spec 078 (Model Fleet).
**Research and dependency admission:** [research.md](research.md).

## Goal

Run the OpenMed catalog rows that ship Transformers-layout `model.safetensors` weights but no ONNX export. In OpenMed v3.0.0, 1,512 of 2,266 rows list a `pytorch` format, but 658 of those are MLX repositories (research §5). For the five evidenced architectures this leaves 582 candidate rows. MedScale runs them locally in pure Rust, with no Python interpreter, no pickle and no executable model code, behind the same governed lifecycle as ONNX Packs.

## Requirements

- **R104-01 Dependency admission.** `candle-core`, `candle-nn` and `candle-transformers` are admitted at `=0.9.1` with `default-features = false`: CPU only, with no CUDA, Metal, MKL or Accelerate. The admission must pass the repository `cargo-deny` policy unchanged. Newer lines (0.10, 0.11) are not admitted: they pull `tokenizers 0.22` with a C++ build (`esaxx-rs`), duplicating the workspace tokenizer.
- **R104-02 Safe format only.** A new Pack artifact kind `SafetensorsModel` carries data-only weights. Pickle (`*.bin`, `*.pt`, `*.pth`) stays forbidden. `trust_remote_code` is never honoured; the architecture comes from `config.json` `model_type` and must be on a fixed allowlist.
- **R104-03 Runtime contract.** Runtime id `candle_safetensors_token_classification_v1`. Prepare verifies each artifact's digest against the admitted manifest, reads `config.json`, loads the weights from the verified bytes, and builds the encoder plus a linear token-classification head with bias. Supported `model_type` values: `bert`, `distilbert`, `roberta`, `xlm-roberta`, `deberta-v2`, `modernbert`. Unknown types are refused before any weight is touched.
- **R104-04 Execution.** A whole-document run reuses the Spec 103 windowing, best-context vote and entity decoding (`token_windows.rs`). The window is the model's position limit (at most 512 tokens). Output is evidence-only and carries the same payload fields as the ONNX document run.
- **R104-05 Bounds.** Safetensors artifacts are bounded at 1 GiB (the ONNX bound), and inputs by the existing 64 KiB limit. Weights are loaded as f32; bf16/fp16 storage is widened at load.
- **R104-06 Pack building and acquisition.** The snapshot path builds a signed Pack from `model.safetensors` and `config.json` with the same catalog and digest verification. The acquisition consent names the weight file; `.safetensors` is allowed alongside `.onnx`.
- **R104-07 Evidence.** Real-model qualification on CI, through the Spec 103 workflow, for each supported architecture. Rows become `EXECUTED_TESTED` only through such evidence.

## Non-goals

- GPU backends.
- Generative, vision and GLiNER span models.
- Training.
- Clinical validation.
- Real PHI.

# Spec 105 tasks

`[x]` done, `[~]` partly done (gap stated), `[ ]` not started. Classes as in Spec 103 `tasks.md`.

- [x] T105-01 Text-generation runtime on candle (`generation_runtime.rs`): `qwen2` and `mistral`, digest-verified prepare, greedy bounded decoding, end-of-sequence stop, prompt and context bounds, KV cache reset per call, evidence-only payload. Synthetic tests (determinism, bounds, unknown architectures). `IMPLEMENTED_AND_QUALIFIED` (synthetic).
- [x] T105-02 Signed generation Packs from verified `text-generation` snapshots (`build_text_generation_pack`), safetensors only; end-to-end synthetic test through admission, prepare and generate, with post-admission tampering refused. `IMPLEMENTED_AND_QUALIFIED` (synthetic).
- [ ] T105-03 Core surface for generation (`PacksGenerateLocal`) with residency and audit. `REPOSITORY_WORK_REMAINING`.
- [x] T105-04 `llama` decoder in the allowlist (candle `Llama` with an external KV cache reset per call). candle's loader panics on a missing layer tensor, so every layer tensor and shape is checked first and a bad checkpoint is refused as an error. Cancellable generation (`generate_cancellable`, checked before each token, no partial output). Synthetic tests. `IMPLEMENTED_AND_QUALIFIED` (synthetic).
- [ ] T105-05 Real-model qualification. `EXTERNAL_BLOCKED`: every catalog generative row exceeds the 1 GiB bound (research §1).
- [ ] T105-06 Multimodal Pack contract and vision encoder. `EXTERNAL_BLOCKED` for catalog rows (over the bound); contract design is `REPOSITORY_WORK_REMAINING` once a fitting model exists.
- [ ] T105-07 Native ONNX Runtime admission. `REPOSITORY_WORK_REMAINING` (vendored, digest-pinned native library and review); low priority (research §3).
- [ ] T105-08 Core ML, Android, WebGPU. Not applicable today (research §4).

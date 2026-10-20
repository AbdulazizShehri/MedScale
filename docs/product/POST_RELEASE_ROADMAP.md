# MedScale post-release roadmap (after v0.1.0)

**Status:** backlog, not in the v0.1.0 scope. **Founder decision 2026-10-20:** feature freeze for v0.1.0. The 1 GiB Model Pack bound stays unchanged. Nothing below is implemented for v0.1.0 unless a verified release-blocking defect requires a narrowly scoped fix.

| Item | Why it waits | Starting point |
|---|---|---|
| Optional 4 GiB Model Packs | Founder decision: the 1 GiB bound stays for v0.1.0 | `EXTERNAL_GATES.md` `PACK_BYTE_BOUND_1GIB_XLMR` (262 measured rows over the bound) |
| Extended Arabic model support | Every Arabic-capable OpenMed model exceeds 1 GiB | `ARABIC_MODELS_OVER_1GIB_BOUND`; depends on the item above |
| Apple Silicon acceleration for short inputs | The x86 few-row kernel does not apply there. A portable loop measured 2.4–7.7x slower than gemm's NEON kernels | `evidence/103-local-model-catalog/eval/candle-skinny-matmul-2026-10-20.md`; a NEON kernel must beat gemm in the balanced `perf-ab` workflow |
| Upstream candle migration | The vendored candle-core carries a zip security backport and two f32 performance items | `vendor/candle-core-0.9.1/MEDSCALE_PATCH.md` (re-apply or upstream items 5 and 6) |
| Further competitive improvements | Feature freeze | Remaining same-job latency rows slower than OpenMed (5 of 22) and DeBERTa-v2 rows at about 1.0x |
| Platform expansion: Linux desktop | Tauri on Linux depends on GTK 3 / `glib` 0.18 (advisory) | `TAURI_LINUX_DESKTOP_GTK3` |
| Platform expansion: Intel macOS build | v0.1.0 ships an arm64 DMG only | Add an x86_64 or universal target to `tauri-preview` |
| Production signing and notarization | Requires certificates and release authorization | [PRODUCTION_SIGNING_REQUIREMENTS.md](PRODUCTION_SIGNING_REQUIREMENTS.md) |
| GLiNER PyTorch rows, quantized MLX, generative and multimodal models | Pickle-only weights, unsupported formats, or over the bound | `PROJECT_COMPLETION_STATUS.md`, Spec 103 remaining items |

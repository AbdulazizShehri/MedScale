# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-18. **Verified main:** `3343e8b` (post-merge CI run 39138014067, all required jobs passed).

## Open pull requests

| PR | Scope | State at handoff |
|---|---|---|
| this PR | Accuracy evidence: six-language PII and biomedical NER gold sets (T103-20) | CI |

Merged 2026-10-18: #244 (security: rustls 0.23.46 and the pinned candle-core 0.9.1 ZIP 9 backport; `cargo-deny` green again), #243 (`MEASURED_OVER_BOUND`), #242 (MLX tokenizer from the verified catalog source), #241 (batches 27–37; T103-09 complete), #245 (accuracy evaluation, DeBERTa word-grouping fix, Core `decoding` option).

## State

- Per-row qualification is complete: 1,466 catalog rows executed. Every row that was executed or expected runnable has its own run or a measured reason (262 runnable weight files over the unchanged 1 GiB bound).
- Accuracy: `medscale_pack::entity_eval` and ten synthetic CC0 gold sets in `evidence/103-local-model-catalog/eval/`. Best decoding is `word_first_pii_merge` for PII and `word_first` for NER. The OpenMed-compatible decoding stays the default; `PackEvaluationRequest.decoding` selects others.
- Accuracy batches run on `qual-batch/103-eval-*` branches. Each batch entry carries an `eval` path to a gold set; tabulate with `scripts/qualification/record_results.py RUN`.

## Next eligible repository work

1. Remaining spec items are external or need separate decisions: the Tauri Models UI (F096-T01 glib advisory, version-based, upstream gtk-rs/Tauri); generative and multimodal rows (1 GiB bound); training-dataset terms (T103-10). T103-16 ONNX Runtime and Core ML need dependency-admission specs (low priority, research recorded).
2. Optional: score the QwenMed rows with fewer decodings to stay under the job limit; add gold sets for further languages present in the catalog.
3. Drop the candle `[patch.crates-io]` backport when a candle release on zip >= 9 is adopted (`vendor/candle-core-0.9.1/MEDSCALE_PATCH.md`).

## Tooling status

- Alibaba Open Code Review `ocr` v1.12.13: delegated mode only (`DELEGATED_CLAUDE_REVIEW`); no zero-cost LLM endpoint is configured.
- Jev (`jevctl` 0.2.3): installed with a TypeSafe API key, but no zero-cost route is verified; `JEV_STATUS=NOT_EXECUTED` (repository policy records Jev as blocked by zero cost).
- PStack: not installed or discoverable on the workstation (PATH, npm, pip checked 2026-10-13).
- Graft `@nanonets/graft` 0.21.1: telemetry off; wiring graph built locally without model calls (`graft build`, 7,757 nodes); `graft/` stays git-ignored.

## External gates (unchanged)

the Linux desktop (`TAURI_LINUX_DESKTOP_GTK3`; F096-T01 is resolved for the shipped Windows/macOS desktop by platform scope), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

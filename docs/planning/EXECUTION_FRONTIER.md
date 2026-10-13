# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-13 (verify main CI on the latest merge before acting).

## Open pull requests

| PR | Scope | State at handoff |
|---|---|---|
| #228 | Text generation through Core (T105-03) | retargeted to main, main merged in |
| #230 | `llama` decoder and cancellable generation (T105-04) | CI |
| this PR | Batch harness, batches 01–02 evidence, size-aware expectations, exact defective-export list, legacy LayerNorm names, Transformers 5 config normalization | — |

Merged 2026-10-13: #223 (india_clinical), #224 (Spec 105 phase A), #225 (Longformer), #226 (cancellation), #227 (E2E lifecycle).

## Qualification runs

- Batch 01: run 38301622121, 16 of 24 executed. Batch 02: run 38314653881, 21 of 24 executed. Batch 03: run 38320247418, 21 of 21 executed. Failures: `CAPABILITY_MATRIX.md` section 7.
- Next batch: build the CLI from this head, regenerate catalog pages (`medscale packs catalog --limit 500 --offset N`), run `scripts/qualification/select_batch.py ... --count 24 --out qualification/batch.json` on a new `qual-batch/103-batch-NN` branch, then `record_results.py RUN`. Requalify the three batch 02 Transformers 5 rows.

## Next eligible repository work

1. Batch 03 and later (T103-09).
2. tract prepare of large DeBERTa-v2 fp16 exports exceeded 90 minutes (batch 01); investigate or prefer another fitting export.
3. Reconcile `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and the T103 classification table after the open PRs merge.

## Tooling status

- Alibaba Open Code Review `ocr` v1.12.13: delegated mode only (`DELEGATED_CLAUDE_REVIEW`); no zero-cost LLM endpoint is configured.
- Jev (`jevctl` 0.2.3): installed with a TypeSafe API key, but no zero-cost route is verified; `JEV_STATUS=NOT_EXECUTED` (repository policy records Jev as blocked by zero cost).
- PStack: not installed or discoverable on the workstation (PATH, npm, pip checked 2026-10-13).
- Graft `@nanonets/graft` 0.21.1: telemetry off; wiring graph built locally without model calls (`graft build`, 7,757 nodes); `graft/` stays git-ignored.

## External gates (unchanged)

F096-T01 (`glib`, Tauri chain), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

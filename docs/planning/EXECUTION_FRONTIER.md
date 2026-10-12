# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-13. **Verified main:** `9393e2d` (post-merge CI run 38282986807, 6/6).

## Open pull requests

| PR | Scope | Base | State at handoff |
|---|---|---|---|
| #223 | OpenMed `india_clinical` code-mixing path; Spec 103 queue classification | main | CI pending (Windows) |
| #224 | Spec 105 phase A: bounded text generation on candle; Spec 105 documents | main | CI pending (Windows) |
| #225 | Exact Clinical Longformer on candle (Hugging Face reference match, run 38300309054) | main | CI pending (Windows) |
| #226 | Cancellable whole-document runs (tract, candle) | main | CI pending (Windows) |
| #227 | Hermetic end-to-end model lifecycle test through Core | main | CI pending |
| #228 | Text generation through Core (T105-03) | #224 | stacked; retarget to main after #224 merges |
| this PR | Batch per-row qualification harness, batch 01 evidence, size-aware expectations, exact degenerate-tokenizer list | main | — |

Merge order: #223, #225, #226, #227 (independent), #224, then #228 (retarget). After the batch: merge main into later branches, check conflicts (`compatibility.rs`, `tasks.md`), re-run exact-head CI.

## Qualification runs

- Batch 01: run 38301622121 (`qual-batch/103-batch-01`, 24 rows). Record with `python scripts/qualification/record_results.py 38301622121`; failures stay negative evidence.
- Next batches: regenerate catalog pages with `medscale packs catalog --limit 500 --offset N`, then `scripts/qualification/select_batch.py ... --count 24 --out qualification/batch.json` on a new `qual-batch/103-batch-NN` branch.

## Next eligible repository work

1. Record batch 01 evidence; run batch 02 and later (T103-09).
2. After #224: generation cancellation; `llama` decoder (T105-04).
3. Update `tasks.md` classification for T103-19 after #225 merges.
4. Reconcile `CAPABILITY_MATRIX.md`, `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` once the batch is recorded.

## Tooling status

- Alibaba Open Code Review `ocr` v1.12.13: delegated mode only (`DELEGATED_CLAUDE_REVIEW`); no zero-cost LLM endpoint is configured.
- Jev (`jevctl` 0.2.3): installed with a TypeSafe API key, but no zero-cost route is verified; `JEV_STATUS=NOT_EXECUTED` (repository policy records Jev as blocked by zero cost).
- PStack: not installed or discoverable on the workstation (PATH, npm, pip checked 2026-10-13).
- Graft `@nanonets/graft` 0.21.1: telemetry off; wiring graph built locally without model calls (`graft build`, 7,757 nodes); `graft/` stays git-ignored.

## External gates (unchanged)

F096-T01 (`glib`, Tauri chain), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

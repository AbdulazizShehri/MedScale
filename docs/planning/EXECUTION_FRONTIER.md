# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-13 (verify main CI on the latest merge before acting).

## Open pull requests

| PR | Scope | State at handoff |
|---|---|---|
| this PR | Batches 24–26 evidence | — |

Merged 2026-10-13 and 2026-10-14: #223–#239.

## Qualification runs

- Batches 01–26 executed 893 of 919 rows; 931 catalog rows executed in total.
- Every (family, architecture, runtime, language, tier) group now has a run, so later batches use `select_batch.py --per-row --count 48`. Build the CLI from this head, regenerate catalog pages, select on a new `qual-batch/103-batch-NN` branch (create `qualification/`), then `record_results.py RUN`.

## Next eligible repository work

1. Batch 03 and later (T103-09).
2. DeBERTa-v2 preparation is fixed (T103-15); keep watching `PREPARE_STAGES` in batches.
3. Reconcile `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and the T103 classification table after the open PRs merge.

## Tooling status

- Alibaba Open Code Review `ocr` v1.12.13: delegated mode only (`DELEGATED_CLAUDE_REVIEW`); no zero-cost LLM endpoint is configured.
- Jev (`jevctl` 0.2.3): installed with a TypeSafe API key, but no zero-cost route is verified; `JEV_STATUS=NOT_EXECUTED` (repository policy records Jev as blocked by zero cost).
- PStack: not installed or discoverable on the workstation (PATH, npm, pip checked 2026-10-13).
- Graft `@nanonets/graft` 0.21.1: telemetry off; wiring graph built locally without model calls (`graft build`, 7,757 nodes); `graft/` stays git-ignored.

## External gates (unchanged)

F096-T01 (`glib`, Tauri chain), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

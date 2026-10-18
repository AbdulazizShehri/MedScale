# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-18 (evening). **Verified main:** `83fb1bd` (#246 merged; post-merge CI checked separately).

## Open pull requests

| PR | Scope | State at handoff |
|---|---|---|
| #247 | Tauri desktop integration on Windows and macOS (Specs 096–101), superseding #174 / #175 / #177 / #179; F096-T01 resolved by platform scope | Exact-head CI |
| #248 | OpenMed v3.0.0 head-to-head benchmark harness and pilot results | CI |
| this PR | Desktop P1: MedAgent agents, model choice, contexts, evidence, completion and resume; Models catalog and Try a model; Documents FHIR import; Model Fleet lanes; source reuse matrix | Opens after #247 |
| (branch `spec/106-local-asr`) | Spec 106: local Whisper ASR behind `LocalAsrPack` | Qualification run `39163305776` |

Merged 2026-10-18: #241–#246.

## State

- Per-row qualification is complete: 1,466 catalog rows executed (262 runnable weight files over the unchanged 1 GiB bound).
- Accuracy: ten synthetic CC0 gold sets. The head-to-head pilot (#248) put MedScale's best decoding above OpenMed 3.0.0 on 3/3 PII models and level on 2/2 NER models. Losses:
  - MedScale's OpenMed-compatible default decoding scored below OpenMed's Python pipeline.
  - Warm latency was 1.3–2.9× slower.
  - Peak RSS was higher (an upper bound).
- Desktop: the Tauri app drives every workspace through Core. New in this PR:
  - MedAgent: named agents on admitted Packs, chosen sources, evidence view, Complete, resume after restart.
  - Models: catalog of 2,266 OpenMed v3.0.0 rows and Try a model.
  - Documents: synthetic FHIR import.
  - Model Fleet: lanes on chosen agents, each running on its own Pack.

## Next eligible repository work

1. Merge #247, then this PR. Close #174, #175, #177 and #179 as superseded.
2. Spec 106: once qualification passes, open its PR, then add the desktop Audio workspace path (admit an Audio Pack, transcribe on `LocalAsrPack`, correct, evidence).
3. Performance: candle token-classification warm latency (the head-to-head loss), then choose the product default decoding from the evidence.
4. Exports: a Core source-read path so the desktop can run the loss-aware FHIR export.
5. Drop the candle `[patch.crates-io]` backport when a candle release on zip >= 9 is adopted.

## Tooling status

- Alibaba Open Code Review `ocr` v1.12.13: delegated mode only (`DELEGATED_CLAUDE_REVIEW`); no zero-cost LLM endpoint is configured.
- Jev (`jevctl` 0.2.3): installed with a TypeSafe API key, but no zero-cost route is verified; `JEV_STATUS=NOT_EXECUTED` (repository policy records Jev as blocked by zero cost).
- PStack: not installed or discoverable on the workstation (PATH, npm, pip checked 2026-10-13).
- Graft `@nanonets/graft` 0.21.1: telemetry off; wiring graph built locally without model calls (`graft build`, 7,757 nodes); `graft/` stays git-ignored.

## External gates (unchanged)

the Linux desktop (`TAURI_LINUX_DESKTOP_GTK3`; F096-T01 is resolved for the shipped Windows/macOS desktop by platform scope), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

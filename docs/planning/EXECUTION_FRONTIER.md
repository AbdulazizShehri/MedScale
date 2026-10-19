# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-19. **Verified main:** `5c33d77` (post-merge CI run 39190483122 green).

## Open pull requests

| PR | Scope | State at handoff |
|---|---|---|
| #252 | Desktop AudioFlow (Audio Pack admission, WAV import, local transcription, correction, evidence-cited draft note, resume) and the Exports action; bounded file reads | CI |
| #253 | Task-aware `recommended` entity decoding (opt-in, backward compatible) | CI |
| #254 | Insights: evidence-grounded answers with citations | CI |
| #255 | AudioFlow diarization by acoustic clustering (unvalidated) | CI |
| (branch `e2e/desktop-journeys`) | WebDriver E2E journeys against the real release binary; tauri-preview timeout 90 min | E2E runs on `e2e/**` |
| this PR | Status refresh | after the above |

Merged 2026-10-18/19: #241–#251 (desktop integration #247, desktop P1 #251, OpenMed head-to-head #248, subject export #250, Spec 106 local ASR #249).

## State

- **Desktop:** Windows and macOS, on real Core. Every workspace drives Core. Covered by 13+ desktop feature tests (MockRuntime, real Core) and, on the E2E branch, WebDriver journeys on the built binary.
- **Models:** 1,466 catalog rows executed. Head-to-head against OpenMed 3.0.0 over 21 models (synthetic gold sets):
  - Accuracy: 11 wins, 10 ties, 0 losses with word-level decoding.
  - Losses: the OpenMed-compatible default decoding; warm latency (median 1.19×, faster on 6 models); peak RSS (upper bound).
  - #253 makes the winning decoding available as `recommended`.
- **AudioFlow:**
  - Local Whisper, qualified on synthetic speech (WER 0.104 / 0.313).
  - Diarization by acoustic clustering (#255). Two male voices are not separated reliably.
- **Arabic:** every Arabic-capable OpenMed model is over the 1 GiB bound (`ARABIC_MODELS_OVER_1GIB_BOUND`).

## Next eligible repository work

1. Merge #252–#255 and the E2E branch as each turns green. Verify main after each.
2. Candle warm-latency profiling on tiny models (per-operation overhead).
3. Release-candidate documentation pass over the packaged Windows and macOS builds.
4. Drop the candle `[patch.crates-io]` backport when a candle release on zip >= 9 is adopted.

## Tooling status

- Alibaba Open Code Review `ocr` v1.13.1: delegated mode only (`DELEGATED_CLAUDE_REVIEW`). The local d1 server cannot serve as its engine (classifier mode, no text generation).
- d1 (`d1-omni-600M`, local MCP `d1-local`): advisory triage only, never a gate. Used on #252, where it flagged the file-import path for review at low confidence; the bounded-read fix came from my own reading.
- Jev (`jevctl` 0.2.3): installed with a TypeSafe API key, but no zero-cost route is verified; `JEV_STATUS=NOT_EXECUTED` (repository policy records Jev as blocked by zero cost).
- PStack: not installed. The npm and PyPI packages named `pstack` are unrelated projects, so they were not installed (checked 2026-10-19).
- tester-army: not published on npm (checked 2026-10-19). E2E uses tauri-driver 2.1.0 with Edge WebDriver on windows-latest instead.
- Graft `@nanonets/graft` 0.21.1: telemetry off; wiring graph built locally without model calls (`graft build`, 7,757 nodes); `graft/` stays git-ignored.

## External gates (unchanged)

the Linux desktop (`TAURI_LINUX_DESKTOP_GTK3`; F096-T01 is resolved for the shipped Windows/macOS desktop by platform scope), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

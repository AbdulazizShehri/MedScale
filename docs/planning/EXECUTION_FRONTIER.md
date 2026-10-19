# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-19. **Verified main:** `7cf6dfe` (#255 merge; post-merge CI green).

## Open pull requests

| PR | Scope | State at handoff |
|---|---|---|
| #256 | Desktop E2E: nine WebDriver journeys on the real release binary (run 39247413184, 9/9), release-isolated `e2e-webdriver` test build, New project form fix | CI |
| #257 | Founder-approved Logo 12 replaces the paired-M identity: SVG masters, lockups, icons, desktop `brand.tsx`, docs, tests | CI |
| this PR | Status refresh | after the above |

Merged 2026-10-18/19: #241–#255 (desktop integration #247, desktop P1 #251, OpenMed head-to-head #248, subject export #250, Spec 106 local ASR #249, desktop AudioFlow and Exports #252, `recommended` decoding #253, grounded Insights #254, diarization #255).

## State

- **Desktop:** Windows and macOS, on real Core. Every workspace drives Core. Covered by 15 desktop feature tests (MockRuntime, real Core) and nine WebDriver journeys on the built release binary (#256), including restart persistence.
- **Models:** 1,466 catalog rows executed. Head-to-head against OpenMed 3.0.0 over 21 models (synthetic gold sets):
  - Accuracy: 11 wins, 10 ties, 0 losses with word-level decoding.
  - Losses: the OpenMed-compatible default decoding; warm latency (median 1.19×, faster on 6 models); peak RSS (upper bound).
  - #253 makes the winning decoding available as `recommended`.
- **AudioFlow:**
  - Local Whisper, qualified on synthetic speech (WER 0.104 / 0.313).
  - Diarization by acoustic clustering (#255), offered as the optional "Group speakers" choice in #256. Two male voices are not separated reliably.
- **Arabic:** every Arabic-capable OpenMed model is over the 1 GiB bound (`ARABIC_MODELS_OVER_1GIB_BOUND`).

## Next eligible repository work

1. Merge #256 and #257 (Logo 12) as each turns green. Verify main after each.
2. Candle warm-latency profiling on tiny models (per-operation overhead).
3. Release-candidate documentation pass over the packaged Windows and macOS builds.
4. Drop the candle `[patch.crates-io]` backport when a candle release on zip >= 9 is adopted.

## Tooling status

- Alibaba Open Code Review `ocr` v1.13.1: delegated mode only (`DELEGATED_CLAUDE_REVIEW`). The local d1 server cannot serve as its engine (classifier mode, no text generation).
- d1 (`d1-omni-600M`, local MCP `d1-local`): advisory triage only, never a gate. Used on #252, where it flagged the file-import path for review at low confidence; the bounded-read fix came from my own reading.
- Jev (`jevctl` 0.2.3): installed with a TypeSafe API key, but no zero-cost route is verified; `JEV_STATUS=NOT_EXECUTED` (repository policy records Jev as blocked by zero cost).
- PStack: not installed. The npm and PyPI packages named `pstack` are unrelated projects, so they were not installed (checked 2026-10-19).
- tester-army: not published on npm (checked 2026-10-19). E2E instead attaches Microsoft Edge WebDriver to the app's WebView2 (`ms:edgeOptions.debuggerAddress`) on windows-latest; tauri-driver could not attach to the InPrivate WebView2.
- Graft `@nanonets/graft` 0.21.1: telemetry off; wiring graph built locally without model calls (`graft build`, 7,757 nodes); `graft/` stays git-ignored.

## External gates (unchanged)

the Linux desktop (`TAURI_LINUX_DESKTOP_GTK3`; F096-T01 is resolved for the shipped Windows/macOS desktop by platform scope), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

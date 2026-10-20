# Execution frontier (durable handoff)

The live, repository-backed handoff for autonomous sessions. Each session verifies it against GitHub before acting and updates it in the next evidence PR. Status documents of record remain `BUILD_QUEUE.md`, `PROJECT_COMPLETION_STATUS.md` and `EXTERNAL_GATES.md`.

**As of:** 2026-10-20. **Verified main:** `29cbf50` (#264 merge; post-merge `ci` run 39376193246, `tauri-preview` run 39376193297 and `desktop-e2e` run 39376193338 all green, 17/17 journeys).

## Open pull requests

| PR | Scope | State at handoff |
|---|---|---|
| this PR | Status refresh | CI |

Merged 2026-10-19/20:
- #256 desktop E2E
- #257 Logo 12 identity
- #258 status refresh
- #259 candle f32 GELU
- #260 CI step timeout after cache eviction
- #261 release-candidate notes
- #262 DistilBERT exact GELU (`f7ea447`)
- #263 desktop invalid-argument reasons, Insights visibility and 8 more journeys (`fe15cb8`)
- #264 few-row f32 matmul kernel (`29cbf50`)
- #265 Windows installer Logo 12 icon (`eb1005e`)

## State

- **Desktop:**
  - Windows and macOS, on real Core.
  - 17 WebDriver journeys on the release binary (#256, #263). They cover Data → snapshot → Analytics, Knowledge, Model Fleet, Collaboration, Browse default deny, grounded Insights, Audio import, MedAgent, Models, Documents, Exports, the Audit Trail and restart persistence.
  - A local sweep of all 25 routes found no console errors.
- **Model latency against OpenMed 3.0.0** (same job, same runner, 22 rows):
  - The median MedScale/OpenMed warm p50 ratio has gone from 1.19x, to 1.06x with the f32 GELU change (#259), to **0.65x** with the few-row matmul kernel (#264, run 39356478281).
  - MedScale is faster on 14 rows, level on 2 and slower on 5. The slower rows are jobs where OpenMed's PyTorch landed on an unusually fast runner.
  - Absolute times vary up to 10x across GitHub runners, so only same-job ratios are evidence.
- **Balanced same-runner A/B** (all six run orders plus an A/A control; the control stays within ±2% on Ubuntu and Windows):
  - The kernel is 55–69% faster on Ubuntu and 37–46% on Windows at 8–16 tokens (run 39360947078).
  - The GELU change, re-checked against its base, is 15–27% faster on Ubuntu and 5–25% on Windows (run 39360968200).
  - Earlier unbalanced A/B runs always ran the baseline first and are not used.
- **Accuracy:**
  - 11 wins, 10 ties and 0 losses against OpenMed with word-level decoding.
  - DistilBERT now uses exact GELU, as in Hugging Face: AnatomyDetect-TinyMed's score delta against the Hugging Face reference fell from 1.31e-05 to 2.98e-08 (#262), with no F1 change.
- **AudioFlow:** local Whisper, qualified on synthetic speech (WER 0.104 / 0.313). Optional speaker grouping is unvalidated, and two male voices are not separated reliably.
- **Arabic:** every Arabic-capable OpenMed model is over the 1 GiB bound (`ARABIC_MODELS_OVER_1GIB_BOUND`).
- **Release candidate:** see `docs/product/RELEASE_CANDIDATE_2026-10-20.md`. The installers are unsigned engineering previews: `release_ready: false`, `signing: NOT_GRANTED`.

## Next eligible repository work

**Feature freeze for v0.1.0** (founder decision 2026-10-20). Only fixes for correctness, security, broken user journeys, release packaging or critical regressions are allowed. The 1 GiB Model Pack bound is unchanged.

1. Release candidate refreshed for `eb1005e`: `docs/product/RELEASE_CANDIDATE_2026-10-20.md`. All packaging checks pass, and 17/17 journeys pass.
2. Deferred work: `docs/product/POST_RELEASE_ROADMAP.md`. Production signing: `docs/product/PRODUCTION_SIGNING_REQUIREMENTS.md`.

## Tooling status

- **Alibaba Open Code Review** (`ocr` v1.13.1): used in delegated mode on every PR since #262 (`DELEGATED_CLAUDE_REVIEW`; the rules are applied by the authoring session, not an independent review). On #264 it found a missing `unsafe impl` rationale and unchecked bounds, both fixed. The local d1 server cannot serve as its engine.
- **d1** (`d1-omni-600M`, local MCP `d1-local`): advisory triage only, never a gate. It rated #256's debugging-port feature and #264's unsafe kernel "medium" at low confidence; deterministic tests are the controls.
- **Qodo:** a GitHub App installed on the owner account `TheHalfMoon`. Its trial ended, and it posts a billing notice on every PR, which is not review evidence and does not block release. GitHub's app-installation API accepts only tokens issued to a GitHub App, so the CLI token (classic, `repo`/`workflow` scopes) gets HTTP 403. The repository holds no Qodo configuration. The owner removes it in the browser: Settings → Applications → Installed GitHub Apps → Qodo.
- **Jev** (`jevctl` 0.2.3): `JEV_STATUS=NOT_EXECUTED`; no zero-cost route is verified.
- **PStack:** not installed. The npm and PyPI packages named `pstack` are unrelated.
- **tester-army:** not published. E2E uses Microsoft Edge WebDriver attached to the app's WebView2.
- **Graft** (`@nanonets/graft` 0.21.1): local wiring graph only, with telemetry off.

## External gates (unchanged)

the Linux desktop (`TAURI_LINUX_DESKTOP_GTK3`; F096-T01 is resolved for the shipped Windows/macOS desktop by platform scope), the 1 GiB Pack artifact bound, real PHI, clinical validation, production signing, training-dataset terms. See `EXTERNAL_GATES.md`.

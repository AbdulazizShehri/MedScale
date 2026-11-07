# MedScale Beta QA matrix

Traceability for [#302](https://github.com/AbdulazizShehri/MedScale/issues/302):
`journey → implementation → automated/integration test → native E2E → platform → status → evidence`.

**Current installer evidence is in [RELEASE_PACKET.md](RELEASE_PACKET.md), bound to `main@207344c`.** There, the journey runs on the exact Windows installer (19/19, 3 fresh runs), and the macOS DMG installs, launches, quits and reopens (3 fresh runs). That replaces rows 1, 1b and 1c below for install and launch; macOS still has no full journey on the DMG build.

Reconciled 2026-11-05 against `main@80c937aa96ee2251294ded601379313700d0d366`. Run links are to that commit unless a row says otherwise. Statuses follow the classification below. A unit test alone never qualifies a user journey, and synthetic data never qualifies real-model accuracy. See [the Beta readiness report](BETA_READINESS.md).

| Status | Meaning |
|---|---|
| `VERIFIED` | Implemented; native user-level evidence on a source-identical or current SHA |
| `VERIFIED_OLDER_SHA` | Native evidence exists but predates current `main`; re-run required |
| `PARTIAL` | Implemented in Core, but the user path or native coverage is incomplete |
| `MISSING` | No implementation in source |
| `EXTERNAL` | Blocked by founder approval, signing identity or publication decision |

## Beta user journeys

| # | Journey | Implementation | Test / native evidence | Platform | Status |
|---|---|---|---|---|---|
| 1 | Download and install | `tauri-preview.yml` unsigned NSIS/DMG | Normal-release InPrivate install, launch, use, uninstall, 3 fresh runs ([41986785623](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986785623), [41986789260](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986789260), [41986793693](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986793693)) from [installer run 41976839962](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976839962) | Windows | `VERIFIED` (unsigned engineering build); signing `EXTERNAL` |
| 1b | macOS app launch and use | `tauri-preview.yml` macOS bundle | Bundle built in run 41976839962; native launch and use in [41976967529](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976967529) | macOS ARM | `VERIFIED` for launch and use only; this doesn't qualify the installer |
| 1c | macOS DMG install | `tauri-preview.yml` DMG | DMG built in run 41976839962; installing from it was not exercised natively | macOS ARM | `PARTIAL`; notarization `EXTERNAL` |
| 2 | Launch without account | `host.rs`, welcome screen | `beta-first-run.yml`; release welcome audit [41986785615](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986785615) | Windows | `VERIFIED` |
| 3 | Data origin labels | `Status kind="synthetic"`, origin column | desktop E2E `documents` ([41976840057](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976840057)) | Windows | `VERIFIED` |
| 4–5 | Sample workspace and FHIR inspection | `host.rs::open_synthetic`, 12 fixtures | desktop E2E; release runs report manifest `VERIFIED` | Windows | `VERIFIED` |
| 6 | Import FHIR JSON | `documents_import_fhir` | Native `documents-import-novel-fhir-ui`, restart once | Windows | `VERIFIED` (OS chooser asserted present, not opened unattended) |
| 7–8 | Evidence, provenance, abstention | Insights, Evidence | desktop E2E `insights-grounded-answer`, `export-with-disclosure`, `audit-trail` | Windows | `VERIFIED` (synthetic corpus only) |
| 9–10 | Model Center | `Intelligence.tsx`, catalog | desktop E2E `models-catalog`, pagination | Windows | `VERIFIED` (catalog visibility ≠ runnable model) |
| 11 | Consented, integrity-checked model install | `audio_model_manager.rs`, digests pinned in source | Real download through the app on Windows ([41976938446](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976938446)) and macOS with consent gate ([41976967529](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976967529)); all 15 pinned files match ([41976936868](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976936868)) | Windows, macOS | `VERIFIED` (Tiny English through the app; other models by digest only) |
| 12–13 | Real local Whisper through the app | `audio_transcribe` | One CC-BY LibriSpeech clip to a visible transcript, same runs | Windows, macOS | `VERIFIED` (execution, not accuracy) |
| 13b | Measured ASR quality | `asr-real-*-qualification.yml` | English 100 clips, WER 5.58% ([41976944467](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976944467)). Arabic 24 clips, WER 65.3%, CER 25.5% ([41976946196](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976946196)). Whisper Tiny, CPU | Linux CPU | `VERIFIED` as measurements of read public speech; not clinical. Arabic on Tiny is not useful for dictation |
| 14 | View, reopen, copy, export transcript | Transcript view, corrections, **Open transcript** (#423), Copy | Native `audio-transcript-reopens-after-leaving-the-page` | Windows, macOS | `PARTIAL`: file export not built (Spec 107 D1); founder decision |
| 15 | Recover from failed downloads, bad files | cancel, preflight, offline failure | [Offline 41976979487](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976979487), [low storage 41976979451](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976979451) | macOS | `VERIFIED` on macOS, on current code plus a comment-only change (#434); Windows cancel not native |
| 16 | Restart without corruption or replay | seed preflight, attestation; Spec 108, Spec 109 | Native `restart-persistence`, `interrupted-first-seed-killed`, `encrypted-vault-recovers-after-a-kill`, crash harness with real SQLCipher | Windows; harness on Windows, macOS, Linux | `VERIFIED` against killed processes; power loss unproven |
| 16b | Encrypted vault lock, unlock, recovery codes, restore | Spec 109 | Native `encrypted-vault-recovery-code-unlock`, `encrypted-vault-restore-a-kept-copy`; storage and Core tests | Windows; tests on all three | `VERIFIED`; screen-side failed Lock has no automated test |
| 16c | Sample set-aside | Spec 108 step 5 | Native `unopenable-sample-set-aside` | Windows | `VERIFIED` |
| 17 | Privacy boundaries visible | Privacy page, egress gate | WebView residue audits: 0 markers in release InPrivate runs | Windows | `VERIFIED` (synthetic canaries; not PHI, OS, crash or backup clearance) |
| 18 | Keyboard and screen reader | ARIA, focus, headings | Native `keyboard-focus-and-page-headings` (Windows); [macOS run 41976979537](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976979537): first attempt failed, rerun passed | Windows, macOS | `PARTIAL`: macOS check intermittent, cause unknown; no real screen-reader test |
| 19 | Actionable errors | `ErrorState`, reasons | Rust `invalid_arguments_carry_cores_reason`; E2E `analytics-explains-invalid-bindings` | Windows | `VERIFIED` |
| 20 | Report defects | About → Report a problem | `support.test.ts`; E2E `about-doctor` | Windows | `VERIFIED`; copy-only by Spec 107 D2 |

## Open qualification items

1. Journey 14 file export: deferred by Spec 107 D1; founder to confirm whether clipboard copy is acceptable for Beta.
2. #386: mechanism reproduced, root cause not proven. See the readiness report.
3. Signing, notarisation and publication remain `EXTERNAL` (founder authorisation).
4. Journey 18: macOS keyboard check intermittent; no screen-reader test.
5. Spec 109: a crash while unlocked now recovers committed work, proven against killed processes only. Domain rows can be ahead of the authority snapshot after a crash.

`MEDSCALE_RELEASE_READY = FALSE`. See [BETA_READINESS.md](BETA_READINESS.md) for what would change that.

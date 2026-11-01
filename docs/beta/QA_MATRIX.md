# MedScale Beta QA matrix

Traceability for [#302](https://github.com/AbdulazizShehri/MedScale/issues/302):
`journey → implementation → automated/integration test → native E2E → platform → status → evidence`.

Reconciled 2026-11-01 against protected `main@604de0d9d717681d9b544ad9fbf402f6a2619156`
(protected squash of PR #398, exact head `58ed7ac3fce5a1ca1c5f7f92028a7c1bbdc25091`, which
passed 6/6 required CI, Tauri 3/3, Windows desktop E2E, first-run and pinned InPrivate QA).
Statuses follow the classification below. A unit test alone never qualifies a
user journey, and synthetic data never qualifies real-model accuracy.

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
| 1 | Download and install | `tauri-preview.yml` unsigned NSIS/DMG | Normal-release InPrivate install/launch/uninstall [run 41314027868](https://github.com/AbdulazizShehri/MedScale/actions/runs/41314027868) pinned to Tauri run 41307558587 / `58ed7ac3` | Windows | `VERIFIED` (unsigned engineering build) |
| 1b | macOS install | `tauri-preview.yml` macOS bundle | Tauri macOS job pass (run 41307558587); native GUI launch in `audio-real-macos-native.yml` (older SHA) | macOS | `VERIFIED_OLDER_SHA`; signing `EXTERNAL` |
| 2 | Launch without account | `host.rs`, welcome screen | `beta-first-run.yml`, release welcome audit | Windows | `VERIFIED` |
| 3 | Data origin labels | `Status kind="synthetic"`, `Imported`/`Synthetic` origin column (`Clinical.tsx`) | desktop E2E `documents` | Windows | `VERIFIED` |
| 4–5 | Fictional sample workspace and FHIR inspection | `host.rs::open_synthetic`, `seed_plan()` 12 fixtures | desktop E2E `launch-and-open-synthetic-workspace`, `patients`, `documents`; InPrivate postsample `app_seed_manifest_probe_status=VERIFIED` (run 41314027800) | Windows | `VERIFIED` |
| 6 | Import FHIR JSON with a file chooser | `documents_import_fhir` → `host.rs::import_fhir` (`read_bounded`, 1 MiB) | Rust: 13/13 same-content reimport, forged-row reject, novel import reopen (#394, #398). UI chooser and native import E2E added in this branch | Windows | `PARTIAL` → closes with this PR's E2E pass |
| 7–8 | Evidence, provenance, abstention | Insights/Evidence pages, Core provenance | desktop E2E `insights-grounded-answer`, `export-with-disclosure`, `audit-trail` | Windows | `VERIFIED` (synthetic corpus only) |
| 9–10 | Model Center: runtimes, licence, size, compatibility | `Intelligence.tsx`, model catalog | desktop E2E `models-catalog`, `models-catalog-pagination` | Windows | `VERIFIED` (catalog visibility ≠ runnable model) |
| 11 | Consented, integrity-checked model install | `audio_model_manager.rs` (allowlist, pinned revision, SHA-256, storage guard, staging, consent) | Real GUI download `MAC_NATIVE_WEBDRIVER_CONSENTED_GUI_WHISPER_INSTALL_PASS` (run 41079542458); Windows real E2E 29/29 (run 41079527501) at `e1bb99cb` | Windows, macOS | `VERIFIED_OLDER_SHA` |
| 12–13 | Native WAV chooser → real local Whisper | `Operations.tsx` `browseWav`, `audio_transcribe` | LibriSpeech test-clean CC-BY-4.0 → `whisper-tiny.en` → visible transcript, `real-human-speech-local-whisper-ui` (run 41079527501); picker cancel (run 40804095510) | Windows, macOS | `VERIFIED_OLDER_SHA` |
| 13b | Measured ASR quality | `asr-real-public-qualification.yml` | 100 clips, WER 5.58 %, CER 2.39 %, RTF 0.148 (run 40571072564, #307) | Linux CPU | `VERIFIED` (English read speech only; not clinical speech) |
| 14 | View, copy, export transcript | Transcript view + corrections (`Operations.tsx`) | View: real E2E above. Copy added in this branch (`transcript-text.ts`, unit tested). File export not implemented | Windows | `PARTIAL` |
| 15 | Recover from failed/cancelled downloads, bad files | `audio_model_download_cancel`, preflight, offline failure | `macos-whisper-offline-failure-qualification` (run 40863440472), `macos-whisper-low-storage-qualification` (run 40863004087) | macOS | `VERIFIED_OLDER_SHA`; Windows cancel path unverified natively |
| 16 | Restart without corruption or replay | `host.rs` seed preflight, ledger attestation | Rust 1/1 interruption, 12/12, 13/13 tests; E2E `restart-persistence`; InPrivate restart; imported-row restart E2E added in this branch | Windows | `VERIFIED` for seed; imported row → this PR |
| 17 | Privacy boundaries visible | Privacy page, egress gate | WebView2 residue audits: 0 synthetic markers in browser runtime (runs 41314027868, 41314027800) | Windows | `VERIFIED` (synthetic canaries; not PHI, OS, crash or backup clearance) |
| 18 | Keyboard and screen reader | ARIA labels, command palette | `macos-native-keyboard-a11y-qualification` (run 40884484162); E2E command palette | macOS, Windows | `VERIFIED_OLDER_SHA` |
| 19 | Actionable errors | `ErrorState`, `Notice`, `CmdError` reasons | Rust `invalid_arguments_carry_cores_reason`; E2E `analytics-explains-invalid-bindings` | Windows | `VERIFIED` |
| 20 | Report defects | `SUPPORT.md`, `.github/ISSUE_TEMPLATE/*` | No in-app entry point; external navigation is deliberately blocked by `navigation_policy.rs` | — | `PARTIAL` |

## Open qualification items

1. Re-run the real-human Whisper Windows and macOS native journeys on post-#398 `main`. The last source-identical pass is `e1bb99cb` (PR #376), which predates #385, #387, #388, #394 and #398.
2. Transcript file export (journey 14) and an in-app support entry point (journey 20) need a design decision. Both would add a write or external-navigation surface to a deliberately narrow capability set.
3. #386: the original intermittent missing-seed GUI observation is not reproduced. Later passes are not root-cause evidence.
4. Signing, notarisation and publication remain `EXTERNAL` (founder authorisation).

`MEDSCALE_RELEASE_READY = FALSE` until items 1–4 are resolved and the founder authorises publication.

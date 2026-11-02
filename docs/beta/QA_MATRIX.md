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
| 6 | Import FHIR JSON with a file chooser | `documents_import_fhir` → `host.rs::import_fhir` (`read_bounded`, 1 MiB) | Rust: 13/13 same-content reimport, forged-row reject, novel import reopen (#394, #398). Native E2E `documents-import-novel-fhir-ui` ([run 41319864345](https://github.com/AbdulazizShehri/MedScale/actions/runs/41319864345), PR #399) | Windows | `VERIFIED` (OS chooser asserted present, not opened unattended) |
| 7–8 | Evidence, provenance, abstention | Insights/Evidence pages, Core provenance | desktop E2E `insights-grounded-answer`, `export-with-disclosure`, `audit-trail` | Windows | `VERIFIED` (synthetic corpus only) |
| 9–10 | Model Center: runtimes, licence, size, compatibility | `Intelligence.tsx`, model catalog | desktop E2E `models-catalog`, `models-catalog-pagination` | Windows | `VERIFIED` (catalog visibility ≠ runnable model) |
| 11 | Consented, integrity-checked model install | `audio_model_manager.rs` (allowlist, pinned revision, SHA-256, storage guard, staging, consent) | `main@604de0d`: macOS `MAC_NATIVE_WEBDRIVER_CONSENTED_GUI_WHISPER_INSTALL_PASS` and `DOWNLOAD_DISABLED_BEFORE_CONSENT_PASS` ([run 41319882859](https://github.com/AbdulazizShehri/MedScale/actions/runs/41319882859)); Windows real GUI install, 29/29 journeys ([run 41319881092](https://github.com/AbdulazizShehri/MedScale/actions/runs/41319881092)) | Windows, macOS | `VERIFIED` |
| 12–13 | Native WAV chooser → real local Whisper | `Operations.tsx` `browseWav`, `audio_transcribe` | `main@604de0d`: 1 LibriSpeech test-clean CC-BY-4.0 clip → local Whisper → visible transcript, `real-human-speech-local-whisper-ui` ([run 41319881092](https://github.com/AbdulazizShehri/MedScale/actions/runs/41319881092)), `MAC_NATIVE_WEBDRIVER_REAL_HUMAN_WHISPER_TRANSCRIPT_PASS` ([run 41319882859](https://github.com/AbdulazizShehri/MedScale/actions/runs/41319882859)); picker cancel (run 40804095510, older SHA) | Windows, macOS | `VERIFIED` |
| 13b | Measured ASR quality | `asr-real-public-qualification.yml` | 100 clips, WER 5.58 %, CER 2.39 %, RTF 0.148 (run 40571072564, #307) | Linux CPU | `VERIFIED` (English read speech only; not clinical speech) |
| 14 | View, copy, export transcript | Transcript view + corrections; Copy transcript (`transcript-text.ts`, PR #399) | Real-human transcript view (run 41319881092); copy unit-tested. File export deliberately not built for Beta (Spec 107 D1, Goldfish-reviewed) | Windows, macOS | `PARTIAL`: view and copy only. Whether clipboard counts as "export" is a founder decision |
| 15 | Recover from failed/cancelled downloads, bad files | `audio_model_download_cancel`, preflight, offline failure | `macos-whisper-offline-failure-qualification` (run 40863440472), `macos-whisper-low-storage-qualification` (run 40863004087) | macOS | `VERIFIED_OLDER_SHA`; Windows cancel path unverified natively |
| 16 | Restart without corruption or replay | `host.rs` seed preflight, ledger attestation | Rust 1/1 interruption, 12/12, 13/13 tests; E2E `restart-persistence`, `imported-fhir-survives-restart-once` ([run 41319864345](https://github.com/AbdulazizShehri/MedScale/actions/runs/41319864345)); `whisper-still-active-after-restart` ([run 41319881092](https://github.com/AbdulazizShehri/MedScale/actions/runs/41319881092)); InPrivate restart | Windows | `VERIFIED` |
| 17 | Privacy boundaries visible | Privacy page, egress gate | WebView2 residue audits: 0 synthetic markers in browser runtime (runs 41314027868, 41314027800) | Windows | `VERIFIED` (synthetic canaries; not PHI, OS, crash or backup clearance) |
| 18 | Keyboard and screen reader | ARIA labels, command palette | `macos-native-keyboard-a11y-qualification` (run 40884484162); E2E command palette | macOS, Windows | `VERIFIED_OLDER_SHA` |
| 19 | Actionable errors | `ErrorState`, `Notice`, `CmdError` reasons | Rust `invalid_arguments_carry_cores_reason`; E2E `analytics-explains-invalid-bindings` | Windows | `VERIFIED` |
| 20 | Report defects | About → Report a problem: fixed issue-chooser URL, Copy link, visible diagnostics from `about_info`, `SUPPORT.md` data rule (`support.ts`, Spec 107 D2) | `support.test.ts` (sync with `SUPPORT.md`, key allowlist); E2E `about-doctor` asserts URL, buttons and data rule, one window; opener/shell-open refused in `permission_tests.rs` | Windows | `VERIFIED` once this PR's E2E passes; copy-only by design (no browser launch) |

## Open qualification items

1. Done 2026-11-01: real-human Whisper native journeys re-qualified on `main@604de0d` (Windows and macOS, links above). One clip is execution evidence, not an accuracy claim; measured accuracy is row 13b.
2. Journey 14 file export: deferred by Spec 107 D1 (needs a new Core disclosure capability with provenance-correct `synthetic_only`); founder to confirm whether clipboard copy is acceptable for Beta. Journey 20 is addressed copy-only by Spec 107 D2.
3. #386: the original intermittent missing-seed GUI observation is not reproduced. Later passes are not root-cause evidence.
4. Signing, notarisation and publication remain `EXTERNAL` (founder authorisation).

`MEDSCALE_RELEASE_READY = FALSE` until items 1–4 are resolved and the founder authorises publication.

# MedScale Beta readiness report

> **Superseded for release decisions by [RELEASE_PACKET.md](RELEASE_PACKET.md), bound to `main@f01323c`.** This report records the earlier round at `80c937a`. Since then, the safety gaps it lists for vault size, folder Pack admission and interrupted downloads have been fixed.

**Evidence commit:** `main@80c937aa96ee2251294ded601379313700d0d366`. Evidence is scoped to that commit; subsequent commits require fresh qualification.

**Date:** 2026-11-05.

## Verdict

**Not released, and not ready to release today.** `RELEASE_READY=false`, `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`.

The automated journeys listed in this report pass on the commit above. That isn't a claim that engineering is complete or that the product is secure: [Known risks](#known-risks) lists open engineering gaps, including the Pack trust root, passphrase zeroization, divergence after a crash, the intermittent macOS keyboard result and untested power loss. What blocks a public Beta is listed under [Blockers](#blockers-to-a-public-beta). Most of that can't be done from engineering: signing, a tester's report, and your decisions.

Passing CI here is not clinical validation, and it is not a full privacy assurance. Everything below uses fictional data or lawful public non-patient recordings.

## What this round changed

| Area | Result |
|---|---|
| Encrypted vault data loss | A torn save could destroy the only good copy. Fixed: saves are atomic, and unlock authenticates before touching anything (#420) |
| Crash while unlocked | Committed work used to be lost silently. It's now recovered and reported (#427, #430) |
| Recovery state | Listed in Settings, with confirmed restore and delete (#433) |
| Failed Lock | Used to show the unlock screen over an open vault. It now stays visible and can be retried (#419, #420) |
| Recovery codes | Shown but unusable before. They now open the vault (#424) |
| Sample workspace recovery | Automatic recovery and manual set-aside (Spec 108, #418, #426) |
| Saved transcripts | Could not be reopened after leaving the page. Fixed (#423) |
| Model integrity | All 15 files for all 5 Whisper models are checked against digests pinned in source (#422) |
| Accessibility | Focus on route change, real headings, keyboard search results, contrast (#419) |
| Doctor report | Now describes the real Tauri app (#425) |

## Evidence at this commit

| What | Result | Run |
|---|---|---|
| Required CI, Windows/macOS/Linux | All jobs pass. `tauri-preview` macOS was rerun after a transient artifact upload failure (`ENOTFOUND`); its tests had passed | [ci 41976839984](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976839984), [tauri-preview 41976839962](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976839962) |
| Native desktop journeys, Windows | 36/36, including kill-and-recover, recovery-code unlock and restore | [41976840057](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976840057) |
| Release build, InPrivate on, Windows | 3 of 3 fresh runs: manifest complete, 161 app files, complete coverage, 0 synthetic markers in 146 WebView files | [41986785623](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986785623), [41986789260](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986789260), [41986793693](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986793693) |
| Release welcome and source contracts | 3 of 3 each | [41986785615](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986785615), [41986785617](https://github.com/AbdulazizShehri/MedScale/actions/runs/41986785617) (and siblings) |
| Real speech through the app, Windows | 39/39 journeys. App download, pinned digests, one CC-BY LibriSpeech clip, local Whisper | [41976938446](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976938446) |
| Real speech through the app, macOS | Consent gate, app download, real transcript | [41976967529](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976967529), [41976940077](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976940077) |
| Pinned digests | 15 of 15 files match, all 5 models | [41976936868](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976936868) |
| Measured accuracy, English | 100 clips, 5.58% word error rate, Whisper Tiny | [41976944467](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976944467) |
| Measured accuracy, Arabic | 24 clips, **65.3% word error rate**, 25.5% character error rate, Whisper Tiny | [41976946196](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976946196) |
| macOS native picker, offline failure, low storage | Pass | [41976979573](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976979573), [41976979487](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976979487), [41976979451](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976979451) |
| macOS keyboard and accessibility | **1 fail, 1 pass, same code.** See risks | [41976979537](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976979537) |

**Installer evidence.** The unsigned Windows NSIS and macOS DMG builds come from run 41976839962. The source SHA is bound in `package-manifest.json`, and the release workflow checks it. The Windows release runs above install, launch, use and uninstall that build. The macOS app was launched and used natively, but installing from the DMG was not exercised. Neither is signed or notarized. **None has been published.**

**Accuracy limits.** Those figures are clean, read public speech with the smallest model. They say nothing about clinical speech. Arabic at 65% word error is not useful for dictation with this model.

## Blockers to a public Beta

1. **Signing and notarization.** Windows SmartScreen and macOS Gatekeeper will warn or block. Needs certificates, and your decision.
2. **Independent tester report.** No one outside this work has used the product. Automated journeys don't find confusion.
3. **Screen-reader testing.** Labels, headings and focus are checked by code and journeys, but no one has used it with NVDA or VoiceOver.
4. **Publication decision.** Yours. Includes public download links, the website and any model hosting.

## Known risks

Most serious first. All are recorded, none is hidden in the app.

| Risk | Detail |
|---|---|
| **Pack signing has no real trust root** | The seed is public (`pack_trust.rs`), so a signature doesn't prove who made a Pack. The app no longer calls a chosen folder "verified", and downloads rely on pinned digests instead. A Pack from an untrusted source is still dangerous to admit. Must be fixed before real data |
| **Power loss** | Crash recovery is proven against killed processes. Sync ordering is correct by reading the code, not by test |
| **Vault tampering** | Someone with write access to the vault folder could restore an older seal, or splice pages from earlier sessions' work files. There's no anti-rollback. Must be re-examined before any PHI authorization |
| **After a crash** | Domain rows can be ahead of the authority snapshot: a receipt from the interrupted request can read as "not found", and that request's audit rows can be lost |
| **Passphrase in memory** | The passphrase and unwrapped key aren't zeroized in several places (found by audit, not fixed) |
| **Large vaults** | The seal is written as a JSON number array, about 3.6× the size by estimate. Ten recordings could mean about 1 GB at Lock |
| **Restore** | Deliberately brings back older state, including deleted data, and shortens the audit trail. It's confirmed and warned |
| **macOS keyboard check** | The first attempt failed: Tab from the Audio Pack field didn't reach Browse folder. The rerun passed everything. Cause unknown. It may be test timing or a real focus problem, so it should be watched |
| **#386** | Not proven. See below |
| **Doctor report** | Now true about the shell, but many sections are still fixed statements, not measurements |
| **Smaller gaps** | A crash during vault creation leaves a folder to remove by hand. A blob torn before Spec 109 isn't healed. Interrupted model downloads aren't swept. Copying a transcript gives no clipboard-history warning. Windows download cancel has no native test. The Ubuntu CI runner ran out of disk once |
| **Not covered at all** | macOS Intel, Linux end-user installers, OS swap, pagefile, crash dumps, backups, real hardware variety |

## Issues

- **[#384](https://github.com/AbdulazizShehri/MedScale/issues/384), resolved by evidence.** With a fully covered baseline, markers are 0 before use, so they aren't bundled files. After use, 1 WebView2 file holds one, in the test-only build with InPrivate off. Release builds with InPrivate on show 0 across nine fresh runs. This is not a privacy clearance: `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` stays `OPEN` for OS-level surfaces and real data.
- **[#386](https://github.com/AbdulazizShehri/MedScale/issues/386), open.** A kill during the seed reproduces the failure, but the original run stopped the app at about 31 s and measured seeds took 9–25 s. So the root cause is not proven. The harness now waits for completion and logs enough to diagnose a recurrence, and real users get automatic recovery. Nine fresh runs show no recurrence, which isn't root-cause evidence.

## Decisions for you

1. **Signing and notarization:** get certificates now, or run a limited Beta with unsigned installers and clear warnings?
2. **Publishing:** when, and to whom? A small private tester group first is my recommendation, since it also gives you item 2 under Blockers.
3. **Transcript file export (Spec 107 D1):** is copy-to-clipboard enough for Beta?
4. **Arabic:** the app offers Arabic with multilingual models. At 65% word error on Tiny, should Beta label it experimental, or recommend Small only? Small has no measurement at this commit.
5. **#386:** accept the proposed closure rule: close when a recurrence is diagnosed from the new logging, or after an agreed number of clean runs?
6. **Pack trust root:** a real signing key needs custody decisions. Is it required for Beta, or can Beta be limited to pinned downloads, with folder admission hidden?
7. **Real patient data:** remains not authorized. Nothing here supports changing that.

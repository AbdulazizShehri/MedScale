# MedScale Beta release packet

**Candidate commit:** `main@f01323c9b3d09e6daf20b5753bbd8fbc54a5d960`
**Version:** 0.1.0, proposed tag `v0.1.0-beta.1`
**Date:** 2026-11-07

`RELEASE_READY=false`, `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`. Nothing has been published, signed or notarized.

This replaces the `207344c` packet. Native QA on that build found defects, which are now fixed. Its installers no longer qualify.

## Verdict

| Platform | Verdict | What decides it |
|---|---|---|
| **Windows x64** | **Ready for your decision.** Every automated gate passes on the exact installer. | Four founder decisions below. Without them it stays NO-GO. |
| **macOS Apple silicon** | **NO-GO** | No Developer ID or notarization, so Gatekeeper blocks a downloaded copy. Also no VoiceOver run, and no full journey on the DMG. |
| **Linux desktop** | Deferred | No desktop package exists. |

Passing these checks is not clinical validation or a full privacy assurance. All tests use fictional data or lawful public recordings. Real patient data remains not authorized.

## Exact artifacts

Built by [tauri-preview 42361952161](https://github.com/AbdulazizShehri/MedScale/actions/runs/42361952161). The artifacts expire on **2026-12-07**, after which the release must be rebuilt and requalified.

| Platform | File | Bytes | SHA-256 |
|---|---|---|---|
| Windows x64 | `MedScale_0.1.0_x64-setup.exe` | 14,170,849 | `d1730031692ba4d549e0492c1ddc9d0eb92ffbc6a60f848369fbedbe59094098` |
| macOS arm64 | `MedScale_0.1.0_aarch64.dmg` | 20,963,004 | `1dd9bd6f6afc20918708fa953b73ca7a7c0a66c6d2a06fb869ae8ce36259ad0c` |

- **Verified:** downloaded bytes were re-hashed and match each `package-manifest.json`, which binds the source commit. Each set includes `SHA256SUMS`, an SBOM, notices, licenses and source archives.
- **Signing:** Windows unsigned. macOS ad-hoc signed, valid under `codesign --verify`, with no Developer ID and no notarization.
- **Not reproducible:** `reproducible_binary_build` is false, and nobody has rebuilt these bytes independently.

## Fixed since the last candidate

| Defect | Found by | Fix |
|---|---|---|
| Focusable rows had no accessible name (UIA DataItems, macOS AXRows) | Native QA on Windows and Mac | #461. The real build now has 0 unnamed focusable controls, down from 16, and NVDA reads "Open patient Ada Synthetic, row" |
| WebView2 held a connection to Microsoft's network while idle | New idle-network check | #461. Background networking off: 0 idle connections in all 3 runs, down from 1 |
| NVDA heard "Patients2", "Documents12" and run-together sentences | First real NVDA run | #461 |
| About called an unvalidated app "Local Clinical Intelligence" | Real captures | #461 |
| Vault creation that stopped before its first seal was a dead end | Code review | #463. The folder is moved aside intact and Create works |
| macOS quarantine diagnostic called a flag change "user approved" | Owner-Mac investigation | #461. Now `SIMULATED_01C1`, with real `spctl`, `syspolicyd` and `amfid` evidence |

## Evidence at this commit

| Check | Result | Run |
|---|---|---|
| Native journeys, test build | Pass | [desktop-e2e](https://github.com/AbdulazizShehri/MedScale/actions/runs/42361952153) |
| Required CI | Was still running when this packet was written; must pass before GO | — |
| **Journey on the exact Windows installer, 3 fresh runs** | **23/23 each**. Run 1 passed on rerun after a certificate-revocation network error, before the app had run | [1](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367038824), [2](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367044749), [3](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367051199) |
| **NVDA 2026.3.0 on the installed build** | **6/6 checks.** Rows named, nothing blank | [42367056610](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367056610) |
| Windows clean install and upgrade | Clean 18/18, upgrade from the published engineering preview 7/7 with vault files unchanged | [42367058081](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367058081) |
| Normal-release privacy audit, 3 runs | 0 markers in 143 browser files each; seed `VERIFIED` | [1](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367041257), [2](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367048310), [3](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367053797) |
| macOS DMG, 3 runs | Install, launch, quit and reopen pass; downloaded copy `GATEKEEPER_BLOCKED` at `_dyld_start` | same as journeys |
| Real speech, Windows test build | 39/39 | [42367059200](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367059200) |
| Real speech, macOS test build | Consent, download, transcript | [42367066493](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367066493), [42367060296](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367060296) |
| Real model execution | Windows and macOS pass | [42367063078](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367063078), [42367061492](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367061492) |
| English, public read speech | 100 clips, 5.58% WER | [42367064166](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367064166) |
| Arabic, public read speech, Whisper Tiny | 24 clips, **65.3% WER**, 25.5% CER | [42367064986](https://github.com/AbdulazizShehri/MedScale/actions/runs/42367064986) |

**What each Windows journey covers:** checksum, install with Mark of the Web, first launch with no account, sample workspace, preference storage probe, focusable controls named, layout at three window sizes, idle network check, FHIR import, folder admission absent, blocked download, kill during download, consented model download, real speech scored against the reference, close and reopen, encrypted vault create and unlock, scanner positive control, WebView residue scan, uninstall keeping data, reinstall reopening the vault, final uninstall.

**Native QA on real hardware** ran on the previous `207344c` build: Windows 11 found the defects fixed above, and the owner's Mac on macOS 26.6.2 passed navigation 13/13 and traced Gatekeeper. **Neither has run on this candidate yet.** A fresh run on your Mac and PC would add real-hardware evidence for these exact bytes.

## Still open

| Item | Status | What it takes |
|---|---|---|
| What SmartScreen shows a person downloading the installer | **Not measured** | Download it through a browser on a real Windows PC |
| VoiceOver on macOS | **Not run** | Your Mac session, on the notarized build once it exists |
| Developer ID signing and notarization | **Workflow ready, never run** (#462) | Apple Developer Program, a Developer ID certificate, six secrets |
| Human security review | **Not done.** The reviews so far were AI reviews | Your decision |
| Recovery codes lost if the window closes on that screen | Disclosed | Fixing it needs re-issuable codes, a design change |
| A saved model that fails its pin check needs manual removal | Disclosed | A remove control |
| InPrivate label shown only on the first launch | Unexplained | Scans with a working positive control found nothing on disk. The privacy gate stays OPEN |
| Model digests, all 5 | Pins unchanged since 15/15 at `80c937a` | Re-download upstream to re-verify |
| Updating between versions | Upgrade from the engineering preview passes | Same version number on both, so not a true version upgrade |
| Power loss, real desktop hardware variety, Intel Mac | Not tested | — |

**Security posture, from AI review:** the previously disclosed risks stand. Pack signing has no real trust root, so packaged builds rely on pinned digests. There's no anti-rollback for vaults. The passphrase isn't zeroized everywhere. Audit rows can be lost on crash. The sample workspace isn't encrypted. Key derivation is modest. Large vault seals run about 3.6 times the data size.

## Your decisions for a Windows GO

1. **Ship unsigned**, with SmartScreen warnings, or buy a certificate first?
2. **Human security review:** get one, or accept AI review for a Beta with no patient data, and say so publicly?
3. **SmartScreen:** have someone download the installer through a browser on a real Windows PC first, or accept it as unmeasured?
4. **GO:** post a comment naming SHA-256 `d1730031692ba4d549e0492c1ddc9d0eb92ffbc6a60f848369fbedbe59094098` and "unsigned". `verify-release` checks that the approval comment is yours and names those bytes.

Also needed:
- **App identifier:** `org.medscale.desktop.preview` becomes permanent at release. Keep it, or change it now and requalify.
- **Website deployment:** the website workflow is build-only. Approve restoring deployment as a separate step.
- **macOS:** wait for notarization. That doesn't block Windows.

## To publish, once you say GO

1. Turn on immutable releases for the repository.
2. Tag `f01323c` as `v0.1.0-beta.1` and publish only the approved assets, with per-platform companion names such as `windows-SHA256SUMS`.
3. In one reviewed PR, fill in `web/src/lib/release-manifest.json` and add the tag to `QUALIFIED_RELEASE_TAGS`. `npm run verify-release` must pass against the published bytes.
4. Deploy the site through the approved route, then download from medscale.si with no login and compare the checksum.

Asset URL that step 2 would create: `https://github.com/AbdulazizShehri/MedScale/releases/download/v0.1.0-beta.1/MedScale_0.1.0_x64-setup.exe`

**Any change to the desktop app, brand assets or crates changes these installers and requires requalification.** Website-only changes don't.

## Rollback and support

- **Stop offering a download:** revert the manifest PR and redeploy.
- **A bad release:** publish a fixed `v0.1.0-beta.2`, requalified.
- **User data:** uninstalling keeps it, verified on Windows and macOS. Upgrading keeps vault files unchanged, verified on Windows.
- **Support:** [GitHub Issues](https://github.com/AbdulazizShehri/MedScale/issues), and [private security advisories](https://github.com/AbdulazizShehri/MedScale/security/advisories/new). Never include patient data, passphrases, recovery codes or vault files in a report.

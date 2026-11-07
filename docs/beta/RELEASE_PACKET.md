# MedScale Beta release packet

**Candidate commit:** `main@207344c3e9fc20c17835bf071e46592e3e332cf1`
**Version:** 0.1.0, proposed tag `v0.1.0-beta.1`
**Date:** 2026-11-07

`RELEASE_READY=false`, `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`. Nothing has been published, signed or notarized.

## Verdict

| Platform | Verdict | Why |
|---|---|---|
| **Windows x64** | **NO-GO today.** Every automated gate passes. | No independent tester, screen-reader test or human security review has happened, and the unsigned-installer decision is yours. |
| **macOS Apple silicon** | **NO-GO** | As above, plus the DMG isn't notarized, so Gatekeeper blocks it. The full journey has never run on the DMG build. |
| **Linux desktop** | Deferred | No desktop package exists. The CLI is built from source. |

Passing these checks is not clinical validation or a full privacy assurance. All tests use fictional data or lawful public recordings. Real patient data remains not authorized.

## Exact artifacts

Built by [tauri-preview 42296366370](https://github.com/AbdulazizShehri/MedScale/actions/runs/42296366370). The artifacts expire on 2026-12-06. After that, the release must be rebuilt and requalified, because builds aren't reproducible.

| Platform | File | Bytes | SHA-256 |
|---|---|---|---|
| Windows x64 | `MedScale_0.1.0_x64-setup.exe` | 14,170,636 | `17e1b9c2689a483a626dbaf883928718430bb997c5317bf7843ab9e8458de012` |
| macOS arm64 | `MedScale_0.1.0_aarch64.dmg` | 20,965,044 | `8b476caf139495dcde41946bba132ad7f446f0ded9f5c145c80084c480588951` |

Downloaded bytes were re-hashed on 2026-11-07 and match. Each release set includes `SHA256SUMS`, `SBOM.cdx.json`, `NOTICE.md`, licenses, source archives and a `package-manifest.json` binding the source commit.

**Signing:** Windows is unsigned. macOS is ad-hoc signed and not notarized. Both manifests say `signed=false`, `notarized=false`, `release_ready=false`.

**Provenance limit:** the manifest binds each installer to its source commit, but `reproducible_binary_build` is false. Nobody has rebuilt these bytes independently.

**Supported systems:**
- **Windows:** x64 with the WebView2 Runtime. Automated tests ran on Windows Server runners only, so no desktop edition of Windows has been tested.
- **macOS:** Apple silicon, declared minimum 10.15. Tested only on the current hosted runner.

## Automated evidence at this commit

| Check | Result | Run |
|---|---|---|
| Required CI | All pass | [ci](https://github.com/AbdulazizShehri/MedScale/actions/runs/42296366458) |
| Native journeys on a test build | Pass | [desktop-e2e](https://github.com/AbdulazizShehri/MedScale/actions/runs/42296366209) |
| **Journey on the exact Windows installer, 3 fresh runs** | **19/19 each** | [1](https://github.com/AbdulazizShehri/MedScale/actions/runs/42304938331), [2](https://github.com/AbdulazizShehri/MedScale/actions/runs/42306977711), [3](https://github.com/AbdulazizShehri/MedScale/actions/runs/42306982696) |
| **Install from the exact DMG, 3 fresh runs** | Pass: launch, quit, reopen, removal | same runs |
| Real speech in the app, test build | Windows 39/39 | [42309553894](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309553894) |
| Real speech on macOS, instrumented test build | Consent, download, transcript | [42309566089](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309566089), [42309555549](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309555549) |
| Real model execution | Windows and macOS pass | [42309559283](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309559283), [42309557820](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309557820) |
| Normal-release privacy audit, 3 runs | 0 markers in 146 browser files; seed `COMPLETE`, wait 0 s | [1](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309574621), [2](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309579200), [3](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309584585) |
| English, public read speech, Whisper Tiny English | 100 clips, **5.58% WER** | [42309562224](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309562224) |
| Arabic, public read speech, Whisper Tiny | 24 clips, **65.3% WER**, 25.5% CER | [42309564348](https://github.com/AbdulazizShehri/MedScale/actions/runs/42309564348) |

**What the Windows journey covers:** digest check, install with Mark of the Web, first launch with no account, sample workspace, project, FHIR import, folder admission absent, a blocked download reporting an error, a kill during download recovering, consented model download, real public speech, close and reopen, encrypted vault create and unlock, WebView residue scan, uninstall keeping user data, reinstall reopening the same vault, final uninstall.

It proves itself with measurements, without printing content: 151,060,136 bytes of weights on disk, 81.8% of an 11-word reference matched in the transcript, and no leftover partial files.

## What wasn't tested at this commit

| Item | Status |
|---|---|
| Independent tester | **Not done** |
| NVDA or VoiceOver | **Not done.** UI Automation shows controls are exposed by name, but that isn't a screen-reader test |
| Human security review | **Not done.** The reviews so far were AI code reviews, which aren't independent assurance |
| What SmartScreen shows a person | **Not measured.** A silent install doesn't pass through it |
| Full journey on the macOS DMG build | **Not done.** That job checks launch and windows only |
| Model digests, all 5 models | Last checked 15/15 at `80c937a` ([41976936868](https://github.com/AbdulazizShehri/MedScale/actions/runs/41976936868)). The pins are unchanged at this commit; upstream files weren't re-downloaded, apart from the Tiny English download in the journey |
| macOS keyboard, picker, low storage, offline | Last passed at `214426c`, before #453 and #454 |
| Updating between versions | Not tested: this is the first release. Reinstall over kept data passes |
| Real desktop hardware, Windows 10 or 11 desktop, Intel Mac | Not tested |
| Power loss | Not tested; recovery is proven against killed processes |
| Live download from the website | Can't be tested until the release exists |

## Security review conclusions

An AI code review of this scope found no path that overwrites the last good seal. This is not a clearance.

**Fixed:**
- Unbounded encrypted vault size: new recordings stop at 256 MiB of live data (#438)
- Folder Pack admission disabled in packaged builds (#438), and packaged builds run only Packs matching pinned digests (#454)
- Broken macOS signature (#441)
- Interrupted downloads leaving an extra 151 MB copy (#451)

**Disclosed, not fixed:**
- **Pack signing has no real trust root.** Packaged builds rely on pinned digests instead.
- **No protection against restoring an older vault.** Someone with write access to the vault folder can do it.
- **The passphrase isn't zeroized everywhere.** Parts of this can't be fixed, because it passes through JavaScript strings.
- **After a crash,** domain data can be ahead of the authority snapshot, and that request's audit rows can be lost.
- **The sample workspace isn't encrypted,** and removed recordings can stay in its files. The Audio page says so.
- **Key derivation parameters** are modest, with an 8-character minimum passphrase.
- **Large vault seals** use about 3.6 times the data size.

**Unexplained:** the WebView labels itself InPrivate only on the first launch. Saved-setting and marker scans found nothing on disk on the launches checked, but the scan has no positive control, so this isn't a privacy pass. `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` stays OPEN.

## Known limitations for users

- Not clinically validated. No real patient data.
- Windows SmartScreen may warn. macOS blocks the first open until the user approves the app.
- Arabic is experimental: about 65% of words wrong on Tiny.
- No automatic updates. Users download each new version themselves.
- Audio import is 16 kHz, 16-bit PCM WAV up to 32 MiB. There's no microphone capture or transcript file export; copying is the supported workflow.
- The model catalog is a list. A listed model isn't downloaded or necessarily runnable.
- A report form needs a GitHub account.
- [#386](https://github.com/AbdulazizShehri/MedScale/issues/386) stays open: its failure reproduces only by killing the app during seeding, and the root cause isn't proven.

## Your decisions

1. **Signing:** ship Windows unsigned with clear warnings, or buy a certificate first?
2. **People:** who is the independent tester, and who runs NVDA? These block GO unless you explicitly waive them.
3. **Security review:** get a human reviewer, or accept AI review for a no-patient-data Beta and say so?
4. **macOS:** wait for Apple Developer ID and notarization, or ship with manual approval? Windows doesn't depend on this.
5. **App identifier:** `org.medscale.desktop.preview` becomes permanent at release, because changing it later would orphan user data. Keep it unless you're ready to change it now and requalify.
6. **The 2,266 number:** prominent, or quiet? I recommend quiet, because it counts catalog listings.
7. **#386:** close once new logging diagnoses a recurrence, or after an agreed number of clean runs?
8. **Website deployment:** the website workflow is build-only and has no deploy job. Approve restoring deployment and the domain change as a separate step.

## To publish, once you say GO

1. You post a comment naming each approved SHA-256 and the signing state. `verify-release` checks that the comment is yours and names those bytes.
2. Turn on immutable releases for the repository. This is a repository setting.
3. Tag `207344c` as `v0.1.0-beta.1` and publish the release, with per-platform companion names such as `windows-SHA256SUMS`.
4. In one reviewed PR, fill in `web/src/lib/release-manifest.json` and add the tag to `QUALIFIED_RELEASE_TAGS`. `npm run verify-release` must pass against the published bytes.
5. Deploy the site through the separately approved route, then download from medscale.si with no login and compare the checksum.

**Asset URLs that step 3 would create** (they don't exist today):
- `https://github.com/AbdulazizShehri/MedScale/releases/download/v0.1.0-beta.1/MedScale_0.1.0_x64-setup.exe`
- `https://github.com/AbdulazizShehri/MedScale/releases/download/v0.1.0-beta.1/MedScale_0.1.0_aarch64.dmg`

**Website changes after this commit don't affect these installers.** Any change to the desktop app, brand assets or crates does, and requires requalification.

## Rollback and support

- **Stop offering a download:** revert the manifest PR, empty the gate, and redeploy. The page goes back to showing no installer.
- **A bad release:** assets are immutable, so publish a fixed `v0.1.0-beta.2`, requalify it, and mark the old release as withdrawn in its notes.
- **Users' data:** uninstalling keeps user data, verified on Windows and macOS. Reinstalling reopens the same vault, verified on Windows.
- **Support:** bugs go to [GitHub Issues](https://github.com/AbdulazizShehri/MedScale/issues), and security reports to [private advisories](https://github.com/AbdulazizShehri/MedScale/security/advisories/new). Reports must never include patient data, passphrases, recovery codes or vault files.

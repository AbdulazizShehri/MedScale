# Changelog

All notable changes to MedScale are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

MedScale Beta is not a clinical product. It is not authorized for real patient data. `RELEASE_READY=false`, `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`.

Each entry links the pull request that made the change.

## [Unreleased]

### Added

- Beta release packet for the `main@f01323c` candidate ([#456](https://github.com/AbdulazizShehri/MedScale/pull/456)).

## [0.1.0-beta.1] - not yet released

This is a **candidate**, not a published release. It is commit `f01323c`, with the proposed tag `v0.1.0-beta.1`. Nothing has been published, signed or notarized. See the [release packet](docs/beta/RELEASE_PACKET.md) and [known issues](docs/beta/KNOWN_ISSUES.md).

This section covers merged pull requests #380 to #463 that are in that commit. Earlier changes since the last engineering preview are not summarized here yet.

### Added

- Copy-only defect reporting inside the desktop app ([#403](https://github.com/AbdulazizShehri/MedScale/pull/403)).
- Native FHIR file chooser and a button to copy a transcript ([#399](https://github.com/AbdulazizShehri/MedScale/pull/399)).
- Reopen a recording's latest transcript ([#423](https://github.com/AbdulazizShehri/MedScale/pull/423)).
- Unlock an encrypted vault with a recovery code. A wrong passphrase or code now says so ([#424](https://github.com/AbdulazizShehri/MedScale/pull/424)).
- Recover a crashed encrypted session, then see and manage what crash recovery kept ([#427](https://github.com/AbdulazizShehri/MedScale/pull/427), [#430](https://github.com/AbdulazizShehri/MedScale/pull/430), [#433](https://github.com/AbdulazizShehri/MedScale/pull/433)).
- Recovery for a sample workspace whose first setup was interrupted, with a notice in the app. It recovers only after proving the workspace holds fixture data only ([#408](https://github.com/AbdulazizShehri/MedScale/pull/408), [#410](https://github.com/AbdulazizShehri/MedScale/pull/410), [#414](https://github.com/AbdulazizShehri/MedScale/pull/414), [#415](https://github.com/AbdulazizShehri/MedScale/pull/415), [#417](https://github.com/AbdulazizShehri/MedScale/pull/417), [#418](https://github.com/AbdulazizShehri/MedScale/pull/418)).
- Only one MedScale process can use a data folder at a time ([#406](https://github.com/AbdulazizShehri/MedScale/pull/406)).
- Website: direct installer downloads, shown only once a release passes the release gate ([#437](https://github.com/AbdulazizShehri/MedScale/pull/437)).
- Website and repository: a link to voluntary GitHub Sponsors support ([#428](https://github.com/AbdulazizShehri/MedScale/pull/428), [#431](https://github.com/AbdulazizShehri/MedScale/pull/431)).

### Changed

- Desktop redesign and a new first launch that picks the first screen from the workspace's real state ([#452](https://github.com/AbdulazizShehri/MedScale/pull/452)). This brought these reviewed changes to `main`: shell, Home, Patients and Evidence ([#446](https://github.com/AbdulazizShehri/MedScale/pull/446)); Audio, Documents and Insights ([#447](https://github.com/AbdulazizShehri/MedScale/pull/447)); Models, MedAgent, Knowledge, Privacy and Audit Trail ([#448](https://github.com/AbdulazizShehri/MedScale/pull/448)); control sizes, task navigation, recovery panels and Settings ([#449](https://github.com/AbdulazizShehri/MedScale/pull/449)); first launch ([#443](https://github.com/AbdulazizShehri/MedScale/pull/443)); and the website's workflow story ([#445](https://github.com/AbdulazizShehri/MedScale/pull/445)).
- Website: real screenshots of the redesigned app ([#453](https://github.com/AbdulazizShehri/MedScale/pull/453)), and a final visual and keyboard pass ([#455](https://github.com/AbdulazizShehri/MedScale/pull/455)).

### Fixed

- Creating a vault that stopped before its first seal is no longer a dead end ([#463](https://github.com/AbdulazizShehri/MedScale/pull/463)).
- Focusable rows now have names that screen readers read out ([#461](https://github.com/AbdulazizShehri/MedScale/pull/461)).
- WebView2 background networking stays off ([#461](https://github.com/AbdulazizShehri/MedScale/pull/461)).
- Cleanup after an interrupted model download ([#451](https://github.com/AbdulazizShehri/MedScale/pull/451)).
- Settings said the theme was saved, but it reset ([#444](https://github.com/AbdulazizShehri/MedScale/pull/444)).
- macOS: the app in the DMG now has a complete ad-hoc signature. This is not Developer ID signing or notarization ([#441](https://github.com/AbdulazizShehri/MedScale/pull/441)).
- Real Doctor values, a native recovery-code journey, and other audit defects ([#425](https://github.com/AbdulazizShehri/MedScale/pull/425)).
- The sealed vault copy is never destroyed, and a failed lock leaves the vault open ([#420](https://github.com/AbdulazizShehri/MedScale/pull/420)).
- A failed lock stays visible; keyboard, heading and wording fixes ([#419](https://github.com/AbdulazizShehri/MedScale/pull/419)).
- A busy vault is reported as busy during recovery ([#416](https://github.com/AbdulazizShehri/MedScale/pull/416)).
- Stable sample-setup failure reasons. A read failure is no longer reported as corruption ([#404](https://github.com/AbdulazizShehri/MedScale/pull/404)).
- Sample-setup manifest and sidecar files are written atomically ([#402](https://github.com/AbdulazizShehri/MedScale/pull/402)).
- UI defects found in a native WebView2 render audit ([#400](https://github.com/AbdulazizShehri/MedScale/pull/400)).
- All FHIR imports are recorded, and keyboard focus is kept on the native sample screen ([#398](https://github.com/AbdulazizShehri/MedScale/pull/398)).
- Safer sample-setup checks, and a duplicate-import regression fix ([#394](https://github.com/AbdulazizShehri/MedScale/pull/394)).

### Security

- Packaged builds run only Packs that match pinned digests ([#454](https://github.com/AbdulazizShehri/MedScale/pull/454)).
- Encrypted vaults are bounded, and Packs are limited to verified downloads ([#438](https://github.com/AbdulazizShehri/MedScale/pull/438)).
- Whisper digests are pinned in source, network paths are refused, and set-aside is narrowed ([#422](https://github.com/AbdulazizShehri/MedScale/pull/422)).
- Subject IDs are cleared from the desktop launch URL ([#381](https://github.com/AbdulazizShehri/MedScale/pull/381)).
- Dependency updates for a yanked crate and RUSTSEC-2026-0433 ([#413](https://github.com/AbdulazizShehri/MedScale/pull/413), [#411](https://github.com/AbdulazizShehri/MedScale/pull/411)).

### Testing and release work (no direct user change)

- The exact installers are qualified with the real user journey ([#442](https://github.com/AbdulazizShehri/MedScale/pull/442)), plus Windows clean install, upgrade and uninstall journeys ([#450](https://github.com/AbdulazizShehri/MedScale/pull/450)).
- A macOS Developer ID signing and notarization workflow was added. It has never run ([#462](https://github.com/AbdulazizShehri/MedScale/pull/462)).
- Stricter WebView privacy audits that fail closed ([#380](https://github.com/AbdulazizShehri/MedScale/pull/380), [#382](https://github.com/AbdulazizShehri/MedScale/pull/382), [#383](https://github.com/AbdulazizShehri/MedScale/pull/383), [#385](https://github.com/AbdulazizShehri/MedScale/pull/385), [#387](https://github.com/AbdulazizShehri/MedScale/pull/387), [#388](https://github.com/AbdulazizShehri/MedScale/pull/388), [#421](https://github.com/AbdulazizShehri/MedScale/pull/421)).
- Remaining recovery test gaps closed ([#426](https://github.com/AbdulazizShehri/MedScale/pull/426)).
- Beta readiness report, QA matrix and security scope ([#435](https://github.com/AbdulazizShehri/MedScale/pull/435)).

[Unreleased]: https://github.com/AbdulazizShehri/MedScale/compare/f01323c9b3d09e6daf20b5753bbd8fbc54a5d960...main
[0.1.0-beta.1]: https://github.com/AbdulazizShehri/MedScale/tree/f01323c9b3d09e6daf20b5753bbd8fbc54a5d960

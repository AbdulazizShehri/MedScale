# MedScale Beta: known issues

This list comes from the "Still open" and limitations sections of the [release packet](RELEASE_PACKET.md) for candidate `main@f01323c` (version 0.1.0, proposed tag `v0.1.0-beta.1`). When the packet changes, update this file to match.

`RELEASE_READY=false`, `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`. Nothing has been published, signed or notarized. Passing the Beta checks is not clinical validation or a full privacy assurance. Real patient data is not authorized.

Found something not on this list? [Open an issue](https://github.com/AbdulazizShehri/MedScale/issues/new/choose). For security problems, use [private reporting](https://github.com/AbdulazizShehri/MedScale/security/advisories/new).

## Platforms

- **Windows x64:** the installer is unsigned. What SmartScreen shows a person who downloads it through a browser has not been measured.
- **macOS Apple silicon:** not ready. The app is only ad-hoc signed, with no Developer ID and no notarization, so Gatekeeper blocks a downloaded copy. VoiceOver has not been run, and the full journey has not been run on the DMG. The signing and notarization workflow exists but has never run.
- **Linux desktop:** Engineering build only: an unsigned x86_64 `.deb` (and an unqualified `.rpm`) built on Ubuntu 22.04 in CI, installed, launched under Xvfb and removed on a disposable runner. No user-journey E2E on Linux yet, and no AppImage.
- **Not tested:** power loss, a wide range of real desktop hardware, and Intel Macs.

## Build and release

- The builds are not reproducible. Nobody has rebuilt the same bytes independently.
- The candidate build artifacts expire on 2026-12-07. After that, the release must be rebuilt and requalified.

## App behaviour

- **Recovery codes can be lost.** If the window closes while the recovery codes are on screen, they are lost. Fixing this needs codes that can be issued again, which is a design change.
- **A saved model that fails its pin check must be removed by hand.** There is no remove control yet.
- **The InPrivate label shows only on the first launch.** This is not explained yet. Scans with a working positive control found nothing on disk. The privacy gate stays open.
- **Updating between versions is only partly tested.** Upgrading from the engineering preview passes, but both builds have the same version number, so it is not a true version upgrade.
- **Model digests** for all 5 models are unchanged since they last passed 15/15 at `80c937a`. They have not been re-checked against fresh upstream downloads.

## Security

No human security review has been done. The reviews so far were AI reviews. These risks from those reviews are still open:

- Pack signing has no real trust root, so packaged builds rely on pinned digests.
- There is no protection against rolling a vault back to an older copy.
- The passphrase is not wiped from memory everywhere.
- Audit rows can be lost if the app crashes.
- The sample workspace is not encrypted.
- Key derivation is modest.
- Sealing a large vault needs about 3.6 times the data size.

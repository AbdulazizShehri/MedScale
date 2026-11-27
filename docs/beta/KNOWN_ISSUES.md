# MedScale Beta: known issues

This list reflects the `0.1.0-beta.1` release candidate ([RELEASE_CANDIDATE_0.1.0-beta.1.md](../release/RELEASE_CANDIDATE_0.1.0-beta.1.md), built from `main@80c67d2d` in run `45916049004`, artifact `15164819520`) incorporating all engineering fixes merged on `main` through PR `#532`.

`RELEASE_READY=false`, `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`. Nothing has been published, signed or notarized. Passing the Beta checks is not clinical validation or a full privacy assurance. Real patient data is not authorized.

Found something not on this list? [Open an issue](https://github.com/AbdulazizShehri/MedScale/issues/new/choose). For security problems, use [private reporting](https://github.com/AbdulazizShehri/MedScale/security/advisories/new).

## Platforms

- **Windows x64:** the installer and portable zip are unsigned. SmartScreen warns (More info → Run anyway) and Windows 11 Smart App Control blocks unsigned executables.
- **macOS (Apple Silicon and Intel):** DMGs are ad-hoc signed only, with no Developer ID and no notarization, so Gatekeeper requires System Settings → Privacy & Security → Open Anyway once. VoiceOver has not been run, and the full interactive journey has only run on test builds (not the installed DMG). The signing/notarization workflow exists but has never run.
- **Linux desktop (x64 and arm64):** unsigned `.deb` and AppImage built on Ubuntu 22.04 in CI, verified for install, launch, reinstall-keeping-data, and removal on disposable runners; arm64 requires ARMv8.2-A with FP16.
- **Not tested:** sudden power loss on real hardware, broad end-user desktop hardware diversity, and upgrades between two different released versions.

## Build and release

- The builds are not reproducible. Nobody has rebuilt byte-for-byte identical installers independently.
- No automatic updates: users install a newer version over the old one (back up the vault first).
- The staged `0.1.0-beta.1` candidate artifact (`medscale-release-candidate-0.1.0-beta.1`, run `45916049004`, artifact ID `15164819520`, built from `main@80c67d2d`) expires on **2027-02-25** (`2027-02-25T00:19:31Z`) and includes all hardening PRs through `#532`.

## App behaviour

- **Updating between versions is only partly tested.** Reinstall-keeping-data passes, but no earlier public release exists yet to test cross-version upgrades.
- **Model digests** for all 5 allowlisted Whisper models are unchanged since passing 15/15 digest/signature verification. They are verified against pinned digests on every download and activation, and damaged caches can now be cleared from the UI (**Remove saved model…**, `#528`, closing `#459`).
- **Recovery codes (`#457`):** shown-once recovery codes now trigger a window-close warning while visible (`#528`) and can be replaced at any time from Settings or the CLI (`medscale vault replace-recovery-codes`, `#512`) using the current passphrase.
- **WebView2 InPrivate privacy gate (`#460`):** five consecutive Windows installed-journey runs since 2026-11-07 (including `0.1.0-beta.1` RC run `45916049004` on `main@80c67d2d`, with `#531`'s hard residue assertions active) reported `INPRIVATE true` on all five launches, `PREF_ON_DISK first=0 second=0`, and `WEBVIEW_RESIDUE {"scanned":149,"hits":0,"unreadable":0}` after a passing positive control. `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` stays `OPEN` pending independent human security review.

## Security

No independent human security review or legal review has been done yet. The reviews so far were AI-assisted reviews. Engineering items resolved on `main` include B1–B5, 64 MiB Argon2id with legacy rewrap (`#526`), secret zeroization (`#526`), cross-platform crash-dump hardening (`#526`), keyed sealed-blob filenames and fixed path-free errors (`#527`), Browse URL path PII scanning (`#527`), Rust-side idle auto-lock (`#528`), safe model destination/config/SQLite hardening (`#529`), and sealed desktop pack/ASR helpers (`#531`). Residual architectural limits that remain open:

- Pack signing has no external publisher trust root yet, so packaged builds enforce pinned SHA-256 digests on allowlisted Whisper models and refuse arbitrary external Packs.
- Local rollback detection (B4) uses a device-local generation record beside the vault; an attacker with full filesystem access who also deletes that record defeats it.
- The bundled sample workspace is intentionally unencrypted and refuses all user FHIR imports (B1, `#524`).
- Sealing a large vault requires temporary disk headroom (approximately 3.6× the active database size during atomic re-seal).
- Real patient data (`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`) and clinical use remain unauthorized pending independent security review, legal/regulatory review (B6 drafts in `docs/legal/drafts`), and written founder authorization.

# MedScale 0.1.0 engineering-preview release candidate (2026-10-19)

**This is not a product release.** The packages are unsigned engineering-preview builds for synthetic data only. Every release flag stays false (`release_ready: false`, `signing: NOT_GRANTED`). MedScale is not clinically validated and not a medical device.

## Build identity

| | |
|---|---|
| Source | `main` at `633cbe828b0cb7160a7060036c50367170e600b5` (Logo 12 merge) |
| CI build | tauri-preview run [39287492143](https://github.com/TheHalfMoon/MedScale/actions/runs/39287492143) |
| Windows (x64) | `installer/MedScale_0.1.0_x64-setup.exe`, SHA-256 `54d7c5b4500f67382906f77a1589e8c50f9e3af04d3b6fba14f1ef23acd6acd3` |
| macOS (arm64) | `installer/MedScale_0.1.0_aarch64.dmg`, SHA-256 `37e6d4b9dbef671fa50f5ad23aeded0e1dd2082d3da2b9fa5113eb317066d87b` |

The installers are in the run's `tauri-installer-windows-latest` and `tauri-installer-macos-latest` artifacts, retained until 2026-10-26. Each release set carries:

- `SHA256SUMS`
- `SBOM.cdx.json` (CycloneDX)
- `NOTICE.md`, `LICENSE` and the bundled license texts (including the font licenses)
- `MPL-SOURCE-NOTICE.txt` with source archives
- `package-manifest.json`, which pins the source, tree, lock-file, SBOM and notice digests

The release set is verified by `scripts/verify-tauri-release.ps1` (Spec 101).

## What was verified

**Packaging (Spec 101, CI run 39287492143):**
- Windows: install, launch (the app stays alive with its WebView2 child under the runner's interactive desktop) and uninstall all pass.
- macOS: `hdiutil verify`, mount, install, launch and uninstall all pass.

**End-to-end user journeys on the real release binary (Windows, `desktop-e2e`):** 9 of 9 pass on `main`. They cover:

- opening the synthetic workspace
- Patients
- creating a project
- a MedAgent run with evidence
- the Models catalog
- Documents
- FHIR export with its disclosure
- the Audit Trail
- restart persistence

**Repository CI on `main`:**
- Rust tests run on Ubuntu, Windows and macOS.
- clippy runs with `-D warnings`.
- cargo-deny and the supply-chain policy run.
- The desktop feature tests run.
- Frontend tests run, including the product identity and bundle guards.

**Identity:** the founder-approved Logo 12 is in the app icon, the installer and every in-app surface (`evidence/brand-logo-12/`).

**Models:**
- 1,466 catalog rows executed.
- Against OpenMed 3.0.0 on synthetic gold sets, with a same-runner scorer:
  - Accuracy: 11 wins, 10 ties and 0 losses with word-level decoding.
  - Warm latency is mixed. See `evidence/103-local-model-catalog/eval/`.

## Installing an unsigned build

- **Windows:** SmartScreen warns about an unknown publisher. Choose "More info", then "Run anyway". The installer is per-user, needs no administrator rights, and does not download WebView2. WebView2 must already be present, which it is on current Windows 11.
- **macOS:** the app is ad-hoc signed and not notarized. Open the DMG, drag MedScale to Applications, then Control-click the app and choose "Open" the first time.

Only install builds whose SHA-256 matches the table above.

## Known limitations

- **No production signing or notarization.** Grant and credentials are external gates.
- **No Linux desktop** (`TAURI_LINUX_DESKTOP_GTK3`). The CLI and Core still support Linux.
- **Models:**
  - Model Packs are bounded to 1 GiB, so generative and multimodal models and every Arabic-capable OpenMed model are excluded (`ARABIC_MODELS_OVER_1GIB_BOUND`).
  - No model is `TASK_QUALIFIED` or `CLINICALLY_VALIDATED`.
- **AudioFlow:**
  - Local Whisper is qualified on synthetic speech only.
  - Speaker grouping is unvalidated acoustic clustering. Two male voices are not separated reliably.
- **Small icon size:** at 16 px the Logo 12 icon reads as a frame cluster (`docs/brand/LOGO_SPEC.md`).
- **Small-model latency:** warm latency on small encoders is still slower than OpenMed on some rows. Matrix multiplication dominates the remaining time.

## Readiness labels

| Label | State | Why |
|---|---|---|
| ENGINEERING_COMPLETE | Not claimed | Open engineering items remain in `docs/planning/EXECUTION_FRONTIER.md`. |
| PRODUCT_RELEASE_READY | **false** | Signing and notarization are not granted. `release_ready: false` in both manifests. |
| CLINICALLY_QUALIFIED | **false** | No authorized clinical evaluation. Real PHI is not authorized. |

External gates are listed in `docs/planning/EXTERNAL_GATES.md`.

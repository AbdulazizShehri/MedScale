# MedScale 0.1.0 engineering-preview release candidate (2026-10-20)

This supersedes [RELEASE_CANDIDATE_2026-10-19.md](RELEASE_CANDIDATE_2026-10-19.md), which stays as history.

**This is not a product release.** The packages are unsigned engineering-preview builds for synthetic data only. Every release flag stays false (`release_ready: false`, `signing: NOT_GRANTED`). MedScale is not clinically validated and not a medical device.

## Build identity

| | |
|---|---|
| Source | `main` at `f067d679701547684ffd6060b4e931fdd6ee0797` (#267 merge) |
| CI build | tauri-preview run [39454195859](https://github.com/TheHalfMoon/MedScale/actions/runs/39454195859) |
| Windows (x64) | `installer/MedScale_0.1.0_x64-setup.exe`, SHA-256 `1c3c0ffb297998c4750536646cda685ea813a5ff810638f8c28c189183ec89ff` |
| macOS (arm64) | `installer/MedScale_0.1.0_aarch64.dmg`, SHA-256 `267c4252a6bbba312d94737e23163d18a5b60b0571481225b7828df7ac7cf95c` |

The installers are in the run's `tauri-installer-windows-latest` and `tauri-installer-macos-latest` artifacts, retained until 2026-10-27. Each release set carries:

- `SHA256SUMS`
- `SBOM.cdx.json` (CycloneDX)
- `NOTICE.md`, `LICENSE` and the bundled license texts
- `MPL-SOURCE-NOTICE.txt` with source archives
- `package-manifest.json`, which pins the source, tree, lock-file, SBOM and notice digests

The release set is verified by `scripts/verify-tauri-release.ps1` (Spec 101).

## What changed since the 2026-10-19 candidate

- **#262:** DistilBERT computes exact GELU, as Hugging Face does.
- **#263:**
  - Invalid-argument errors carry Core's reason.
  - Ask the evidence stays visible in narrow windows.
  - 8 more end-to-end journeys.
- **#264:** the few-row f32 matmul kernel. On x86_64 small encoders are 2–3x faster, and the median warm latency is 0.65x OpenMed's.
- **#265:** the Windows setup program shows the Logo 12 icon. Before, it showed NSIS's default. Release-candidate verification of `29cbf50` found this.
- **#267:** the Welcome screen illustration keeps only its dot field (founder request).

## What was verified on `f067d67`

**Packaging (Spec 101, run 39454195859):**
- Windows: install, launch and uninstall all pass.
- macOS: DMG verify, mount, install, launch and uninstall all pass.

**Identity:**
- Founder-approved Logo 12 is in the installed app's icon, the Windows setup program (extracted from #265's installer build) and every in-app surface (`evidence/brand-logo-12/`).

**End-to-end journeys on the release binary (`desktop-e2e` run 39454196018):** 17 of 17 pass on Windows. They cover:

- opening the synthetic workspace
- Patients
- creating a project
- Data sample → snapshot → Analytics query
- the invalid-binding message
- Knowledge index and search
- Model Fleet lanes
- a Collaboration room
- Browse default deny
- a grounded Insights answer
- Audio import
- a MedAgent run with evidence
- the Models catalog
- Documents
- FHIR export with disclosure
- the Audit Trail
- restart persistence

**Repository CI:**
- Rust tests run on Ubuntu, Windows and macOS.
- clippy runs with `-D warnings`.
- cargo-deny and the supply-chain policy run.
- The frontend identity and bundle guards run, including one that keeps the test-only WebDriver build out of packaging.

**Models against OpenMed 3.0.0** (synthetic gold sets, same runner):
- Accuracy: 11 wins, 10 ties and 0 losses with word-level decoding.
- Warm latency: median 0.65x OpenMed over 22 rows. MedScale is faster on 14, level on 2 and slower on 5.
- Evidence: `evidence/103-local-model-catalog/eval/`.

## Supported operating systems

| Platform | v0.1.0 | Notes |
|---|---|---|
| Windows 10/11, x64 | Desktop installer (NSIS, per-user) | Needs the Microsoft Edge WebView2 Runtime. Windows 11 includes it, and the installer does not download it. |
| macOS 10.15 or later, Apple Silicon (arm64) | Desktop DMG | No Intel or universal build in v0.1.0. |
| Linux | CLI and Core only | No desktop (`TAURI_LINUX_DESKTOP_GTK3`). |

## Security

- **Trust and privacy model:** [`docs/security/TRUST_AND_PRIVACY_MODEL.md`](../security/TRUST_AND_PRIVACY_MODEL.md).
- **Local by default:** network egress is denied unless an allowlisted, governed action allows it. Browse records `empty_allowlist` denials. This is verified by the `browse-default-deny` journey.
- **Encrypted workspaces:** vaults are SQLCipher-encrypted and sealed on close. Desktop feature tests cover create, lock, unlock and the wrong passphrase.
- **WebView isolation:**
  - Only the local application origin can invoke commands.
  - Generic privileged and plugin commands are refused. `permission_tests.rs` covers both.
  - The test-only WebDriver build (`e2e-webdriver`) can never reach packaging. `bundle-config.test.ts` guards this.
- **Supply chain:**
  - cargo-deny (advisories, licences, bans) and the supply-chain policy run on every PR and on `main`.
  - Each release set carries a CycloneDX SBOM, notices, licences and MPL sources.
  - No RustSec ignores. The vulnerable `zip` versions are removed through the documented candle-core backport.
- **Unsigned binaries:** verify the SHA-256 digests above before installing. Production signing requirements: [PRODUCTION_SIGNING_REQUIREMENTS.md](PRODUCTION_SIGNING_REQUIREMENTS.md).
- **Data:** synthetic data only. Real PHI is not authorized.

## Installing an unsigned build

- **Windows:** SmartScreen warns about an unknown publisher. Choose "More info", then "Run anyway". The installer is per-user, needs no administrator rights, and does not download WebView2.
- **macOS:** the app is ad-hoc signed and not notarized. Control-click the app and choose "Open" the first time.

Only install builds whose SHA-256 matches the table above.

## Known limitations

- **No production signing or notarization.** Grant and credentials are external gates.
- **No Linux desktop** (`TAURI_LINUX_DESKTOP_GTK3`).
- **Models:**
  - Model Packs are bounded to 1 GiB, so generative and multimodal models and every Arabic-capable OpenMed model are excluded.
  - No model is `TASK_QUALIFIED` or `CLINICALLY_VALIDATED`.
- **Speaker grouping** in AudioFlow is unvalidated. Two male voices are not separated reliably.
- **The few-row matmul kernel** applies to x86_64 with AVX2 and FMA only. Apple Silicon uses candle's gemm.
- **Small icon size:** at 16 px the Logo 12 icon reads as a frame cluster.

## After v0.1.0

Deferred items are listed in [POST_RELEASE_ROADMAP.md](POST_RELEASE_ROADMAP.md). The 1 GiB Model Pack bound stays unchanged for v0.1.0 (founder decision 2026-10-20).

## Readiness labels

| Label | State | Why |
|---|---|---|
| ENGINEERING_COMPLETE | Not claimed | Open items remain in `docs/planning/EXECUTION_FRONTIER.md`. |
| PRODUCT_RELEASE_READY | **false** | Signing and notarization are not granted. `release_ready: false` in both manifests. |
| CLINICALLY_QUALIFIED | **false** | No authorized clinical evaluation. Real PHI is not authorized. |

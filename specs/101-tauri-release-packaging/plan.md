# Implementation Plan: Tauri release packaging (Spec 101)

## Technical context

- App: `apps/desktop-tauri` (Tauri 2.12.0, React/TypeScript/Vite). The Rust host owns an in-process Core `CliSession`.
- Bundler: `@tauri-apps/cli` 2.12.0 from `package-lock.json` (`npm run tauri -- build`).
- CI: `.github/workflows/tauri-preview.yml` already builds, tests and packages the release executable on `ubuntu-latest`, `windows-latest` and `macos-latest`. Spec 101 adds bundle steps to the same jobs rather than a parallel workflow.
- Existing package script: `scripts/package-tauri-preview.ps1` (inventory, notices, MPL sources, build receipt). Spec 101 extends it.

## Design

1. **Config (FR-001, FR-007).** In `tauri.conf.json`:
   - set `bundle.active = true`;
   - set `bundle.targets` to the per-platform list via a CLI `--bundles` argument (Windows `nsis`; macOS `app,dmg`; Linux `deb`), so one config serves all three;
   - set `bundle.windows.nsis.installMode = "currentUser"` and `bundle.windows.webviewInstallMode = { "type": "skip" }`;
   - set `bundle.publisher`, `bundle.copyright` and `bundle.licenseFile = "../../../LICENSE"`;
   - keep the identifier unchanged.
2. **Build (FR-002, FR-003).**
   - The existing step builds the release executable.
   - A new step runs `npm run tauri -- build --bundles <targets> --no-sign --ci`. The executable is rebuilt by the bundler with the same locked inputs. Whether it is byte-identical to the separately built one is measured and recorded, not assumed.
   - Tool download hashes (NSIS archive, `nsis_tauri_utils.dll`) are recomputed with SHA-256 from the Tauri cache directory and logged.
3. **Manifest and verifier (FR-004).** A new `scripts/package-tauri-release.ps1` assembles `target/tauri-release/<platform>/`:
   - the installer or package;
   - `SBOM.cdx.json`, `NOTICE.md`, `LICENSE`, `licenses/`, `source-archives/`;
   - `package-manifest.json` (Spec 058 fields plus the Spec 101 additions) and `SHA256SUMS`.
   
   `scripts/verify-tauri-release.ps1` recomputes everything and fails closed.
4. **Install qualification (US1.5–6, FR-005).** `scripts/qualify-tauri-installer-windows.ps1` runs on the CI runner only:
   - silent NSIS install (`/S`);
   - hidden-desktop launch probe (reusing the `observe-tauri-runtime-egress.ps1` technique, with the probe body factored into a shared function);
   - silent uninstall;
   - a residue listing.
   
   macOS mounts the dmg, copies the app to a temporary directory, checks `Info.plist`, launch-probes, then removes it. Linux extracts the `.deb` into a temporary root (`dpkg-deb -x`), checks `--info`, and launch-probes with `xvfb-run`.
5. **Evidence (US4).**
   - CI uploads `target/tauri-release/*` as artifacts.
   - `evidence/101-tauri-release-packaging/<head>/` records the downloaded artifacts' verification, residue results, and what was not done.
   - The doctor is unchanged: no readiness flag moves.

## Constitution and policy check

| Policy | Status |
|---|---|
| Zero cost | GitHub-hosted runners only; no paid signing or services |
| Supply chain | Every build-time download pinned and hashed (research.md); AppImage rejected for a floating download |
| Security boundary | No change to commands, capabilities, CSP or plugins |
| Truthfulness | Buildable versus signed separated; app versus runtime egress separated; no accessibility or WCAG claim |
| Governance | Stacked on #177. Merge only after #177 merges, with exact-head CI, the scoped Jev/OCR gate for the Tauri chain, and normal merge commits. |

## Risks

- Windows job time: the native build plus bundling may exceed the 60-minute limit. This is measured; raise the limit only with evidence.
- A hidden-desktop launch on a CI service session may not create a visible window. The probe then records "process alive and WebView2 children spawned" without a title, and says so.
- The bundler rebuilds the executable, so evidence must hash the bundled payload, not reuse the separate build's hash.

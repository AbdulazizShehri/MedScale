# Tasks: Tauri release packaging (Spec 101)

Order is dependency order. `[W]` Windows, `[M]` macOS, `[L]` Linux.

## Phase 1: Config and build

- [ ] T101-01 Bundle config in `tauri.conf.json`:
  - set `active`, NSIS `currentUser`, `webviewInstallMode skip`, `publisher`, `copyright` and `licenseFile`;
  - leave the identifier unchanged;
  - add a config test (`src-tauri` unit test or `npm test`) asserting no updater, no plugins, `skip` mode and the unchanged identifier.
- [ ] T101-02 [W][M][L] CI step `npm run tauri -- build --bundles <per-OS> --no-sign --ci` after the existing release build. Log the SHA-256 of every downloaded tool archive.
- [ ] T101-03 Measure the CI wall time per platform. Adjust `timeout-minutes` only with recorded evidence.

## Phase 2: Manifest and verification

- [ ] T101-04 `scripts/package-tauri-release.ps1`: assemble the artifact set, `package-manifest.json` (Spec 058 fields plus Spec 101 additions) and `SHA256SUMS`.
- [ ] T101-05 `scripts/verify-tauri-release.ps1`:
  - recompute all hashes and the source binding;
  - fail closed on a missing, unexpected or tampered file;
  - add negative tests with a tampered copy.
- [ ] T101-06 Upload the per-platform artifacts.

## Phase 3: Install qualification (CI runners only)

- [ ] T101-07 [W] `scripts/qualify-tauri-installer-windows.ps1`: silent install, hidden-desktop launch probe, silent uninstall, residue report (install dir plus app-data).
- [ ] T101-08 [M] dmg mount, `Info.plist` identity check, launch probe, removal.
- [ ] T101-09 [L] `dpkg-deb --info/--contents`, extract to a temporary root, `xvfb-run` launch probe.

## Phase 4: Evidence and closure

- [ ] T101-10 `evidence/101-tauri-release-packaging/<head>/`:
  - downloaded-artifact verification;
  - per-platform results;
  - residue results;
  - not-performed list (signing, upgrade, Gatekeeper/SmartScreen reputation, physical keyboard, screen reader).
- [ ] T101-11 Exact-range scope record and requirement-to-evidence traceability (FR-001…FR-008, US1–US4).
- [ ] T101-12 After #177 merges: exact-head CI, the scoped Jev/OCR gate status, normal merge, post-main CI, and `BUILD_QUEUE.md` and `EXTERNAL_GATES.md` updates.

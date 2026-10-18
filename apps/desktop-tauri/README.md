# MedScale desktop (Tauri)

The Tauri 2 desktop shell over the real MedScale Core. Rust owns the Core session (`CliSession`, in-process `CoreFacade`), the same way the Slint desktop does. The React/TypeScript frontend only renders projections and sends bounded requests. No vault key, passphrase or Core handle crosses into the WebView after a command returns.

Feature surfaces reuse the Slint desktop view models through `crates/medscale-desktop-vm`, so both desktops present the same Core truth:

| Area | Routes |
| --- | --- |
| Clinical | Patients (brief, timeline, labs, coverage, sources), Documents, Insights, Evidence |
| Research | Projects, Data, Browse, Analytics, Knowledge, Research OS |
| Intelligence | Models, MedAgent, Model Fleet |
| Operations | Workflows, Tasks, Messages, Collaboration, Audio |
| Governance | Privacy, Audit Trail, Exports, Integrations, Settings, About |

Workspaces:

- **Synthetic workspace.** Opens a synthetic vault in app-local data. On first open, two synthetic subjects are seeded through governed FHIR ingest and proposal promotion, using the repository fixtures. Browse, ASR and privacy keys use Core's offline fixtures.
- **Encrypted vault.** Created and unlocked with a passphrase, which is not an identity, and gets recovery codes. Core accepts clinical FHIR ingest into synthetic vaults only, so an encrypted vault starts without subjects.

From this directory:

```text
npm ci --ignore-scripts
npm test
npm run build
npm run tauri -- dev
```

Native builds on Windows need Strawberry Perl first on `PATH`, with `PERL` pointing at it, for the vendored OpenSSL used by SQLCipher. The Git-for-Windows msys perl does not work.

## Tests

- `cargo test` in `src-tauri` runs the command layer against a real Core through Tauri's `MockRuntime`.
  - `feature_tests.rs` drives every surface end to end: seed, read, create, execute, compare, lock.
  - It also covers encrypted create, lock, unlock and the wrong-passphrase path.
  - `permission_tests.rs` checks that the capability grants exactly the generated command list.
- `npm test` runs the route and IPC contract tests.

## Visual QA

`feature_tests.rs` can record every command response:

```text
MEDSCALE_UI_FIXTURES=/path/ui-fixtures.json cargo test every_surface_runs_against_real_core
npm run build
node qa/serve.mjs /path/ui-fixtures.json 47124
```

`qa/` replays those real responses into the built frontend, so pages can be captured in a browser without a native shell. The hash selects the start state:

- `#route=Patients&subject=synthetic-subject-ada`
- `&theme=light`
- `&density=compact`
- `&locked=1&gate=access` for the unlock screen

The QA files are never bundled into the app.

## Engineering installers (Spec 101)

CI builds **unsigned** engineering packages from the exact PR head (`specs/101-tauri-release-packaging`). They are engineering artifacts, not releases: `RELEASE_READY=false`, `SIGNING=NOT_GRANTED`.

| Platform | Package | Notes |
|---|---|---|
| Windows | NSIS `MedScale_<version>_x64-setup.exe`, per-user install | The installer never downloads WebView2 (`webviewInstallMode=skip`). The **Microsoft Edge WebView2 Evergreen Runtime is a prerequisite**: it ships with Windows 11 and is serviced on supported Windows 10. Without it, MedScale fails to start rather than downloading anything. Unsigned, so Windows SmartScreen may warn on first run. |
| macOS | `MedScale.app` and `MedScale_<version>_<arch>.dmg` | Unsigned and not notarized, so Gatekeeper blocks it by default. Opening the DMG shows the Apache-2.0 license as a click-through agreement (from `bundle.licenseFile`). |
| Linux | `.deb` (package `med-scale`) | Depends on `libwebkit2gtk-4.1-0` and `libgtk-3-0`. No AppImage is produced, because its build tooling is a floating, unverified download. |

Each package ships in a verified release set:
- `package-manifest.json`, which binds the source and tree SHAs, both `Cargo.lock` files, `package-lock.json`, the toolchain, and the installer, SBOM and NOTICE digests;
- `SHA256SUMS`, `SBOM.cdx.json`, `NOTICE.md`, the licenses and the MPL source archives.

Check a release set with:

```text
pwsh ./scripts/verify-tauri-release.ps1 -Root <release-set-dir> -ExpectedSourceSha <commit>
```

App data lives under `org.medscale.desktop.preview`:
- Windows: `%LOCALAPPDATA%`
- macOS: `~/Library/Application Support`
- Linux: `~/.local/share`

Whether an uninstall removes app data is measured per platform in the Spec 101 evidence, not assumed. Remove encrypted vaults deliberately: an uninstaller is not a vault deletion tool.

## Capability and navigation policy

- Only the main local window has a capability, and it grants exactly the commands listed in `build.rs`.
- No Tauri plugin, filesystem, HTTP, shell, process, clipboard or window-control permission is granted.
- Navigation accepts only the packaged application origins. Debug builds also accept the loopback Vite origin.
- New windows are denied.
- The production CSP has no remote connection origin.
- WebView incognito mode is a mitigation. It does not by itself establish cache, crash, log or storage containment.
- Fonts (Inter, JetBrains Mono NL), icons and the MedScale mark are bundled locally.
- Preferences (theme, density, last project) are the only values kept in WebView storage. The WebView runs incognito, so they live in memory only and reset on restart; nothing is written to WebView Local Storage on disk (F096-T05).

## Qualification status

- Real PHI is not authorized.
- Dependency policy is FAILED on an inherited Linux finding: `glib` 0.18.5 is affected by RUSTSEC-2024-0429. It comes from Tauri 2.x's GTK3/WebKitGTK stack, and no stable Tauri release allows a patched `glib`. The advisory is not ignored. See F096-T01 in `specs/096-tauri-foundation/findings.md`.
- Signing, installers, cross-platform behavior, physical keyboard and screen-reader passes, performance and privacy containment are not qualified.
- Do not infer product or private-data readiness from this desktop.

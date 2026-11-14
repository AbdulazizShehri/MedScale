# Feature Specification: Tauri Desktop Release Packaging Qualification

**Branch**: `claude/tauri-release-packaging` (stacked on PR #177)
**Status**: DRAFT
**Created**: 2026-10-07
**Authorization**: Founder decision 4 (2026-10-07). Create a bounded Tauri installer and release-packaging spec, extend the existing contracts, keep zero cost, and leave signing out.
**Extends**: Spec 058 portable release package contract (`evidence/058-portable-release-package/PACKAGE_CONTRACT.md`), Spec 054 release SBOM, and the Spec 096 unqualified preview package (`scripts/package-tauri-preview.ps1`).
**Depends on**: PR #177 (Tauri desktop on real Core). This spec cannot close before #177 merges.
**Does not**: sign, notarize, publish, auto-update, claim reproducible binaries, authorize real PHI, or set `RELEASE_READY`.

## Two separate outcomes

| Axis | Meaning | This spec |
|---|---|---|
| `BUILDABLE_INSTALLER` | An unsigned installer or package built in CI from an exact source identity, inventoried, bound to SBOM, NOTICE and checksums, and installed, launched and removed under test | In scope |
| `SIGNED_PRODUCTION_INSTALLER` | The same artifact signed with a trusted publisher identity (Authenticode, Apple Developer ID plus notarization, distro repository keys) | Out of scope. `DESKTOP_RELEASE_SIGNING_PROVENANCE = NOT_GRANTED`. It does not block engineering qualification. |

Zero cost is mandatory. No paid certificate, Apple Developer membership, signing service, paid CI or packaging service may be used.

## User Stories

### US1: Windows installer (P1)

CI builds an unsigned Windows installer for the Tauri desktop from the exact PR head, records its inventory and hashes, and a qualification run installs it per-user into a clean location, launches it non-interactively, uninstalls it, and records which files remain.

**Acceptance**

1. The installer is NSIS (Tauri bundler, `targets: ["nsis"]`), per-user (`installMode: "currentUser"`), with `webviewInstallMode: { "type": "skip" }`.
   - Installation needs no network.
   - The WebView2 Evergreen Runtime is a documented prerequisite. It ships with Windows 11 and supported Windows 10.
2. Build-time tools (NSIS 3.11, `nsis_tauri_utils` 0.5.3) are the exact versions pinned by the locked `@tauri-apps/cli` 2.12.0. Their URLs and hashes are recorded in `research.md`. Tauri verifies them with SHA-1; the qualification additionally records SHA-256 of the downloaded archives.
3. The installer, the installed executable and every payload file are hashed (SHA-256) in a `package-manifest.json` that extends the Spec 058 manifest fields:
   - source SHA, tree SHA, `Cargo.lock` SHA-256 (root and `src-tauri`), `package-lock.json` SHA-256;
   - toolchain (rustc, node, tauri-cli);
   - platform and architecture;
   - `signed=false`, `notarized=false`, `release_ready=false`, `reproducible_binary_build=false`.
4. The release artifact set includes `SBOM.cdx.json` (Spec 054 generator), `NOTICE.md`, `LICENSE`, the Spec 096 MPL source archives and notices, and `SHA256SUMS`.
5. Clean install, launch, uninstall run on the disposable GitHub-hosted `windows-latest` runner, never on a founder workstation:
   - Install silently into the per-user default location.
   - Launch on an isolated, never-shown desktop: the `scripts/observe-tauri-runtime-egress.ps1` technique, so no input can reach the process.
   - Verify the host process stays alive and spawns its WebView2 children, record the window title where enumerable, then terminate it.
   - Uninstall silently and record every remaining file under the install directory and `%LOCALAPPDATA%org.medscale.desktop.preview`.
   - No step types into the application or creates a vault.
6. App-data semantics are measured on that runner: what the uninstaller removes and what it keeps is recorded as observed behavior, not assumed. The founder workstation's existing app-data directory is never used by this qualification.

### US2: macOS engineering package (P2)

CI builds an unsigned `.app` and `.dmg` on `macos-latest` with the Tauri bundler, using only macOS system tools (no download). Qualification records structure, hashes, the `Info.plist` identity and version, and a non-interactive launch smoke on the CI runner. Gatekeeper rejection of an unsigned, un-notarized app on end-user machines is documented, not bypassed.

### US3: Linux engineering package (P2)

CI builds an unsigned `.deb` on `ubuntu-latest`. The Tauri `.deb` bundler needs no tool download. Qualification records `dpkg-deb --info` and `--contents`, hashes, an install into a disposable root or container, and a headless launch smoke under `xvfb-run`.

AppImage is excluded: the Tauri 2.12 AppImage path downloads `linuxdeploy-plugin-appimage` from a floating `continuous` release without hash verification, which fails the repository's pinned supply-chain policy.

### US4: Honest qualification state (P1)

Evidence is per platform, exact-head bound, and states what was not done:
- signing;
- end-user Gatekeeper and SmartScreen reputation;
- upgrade between two installer versions (P3; recorded as not performed until a second version exists);
- physical keyboard and screen-reader qualification (`PENDING_EXTERNAL_MEASUREMENT`).

The doctor and `EXTERNAL_GATES.md` keep:
- `RELEASE_READY=false`, `PRIVATE_DATA_READY=false`, `PLATFORM_QUALIFIED=false`;
- `CLINICAL_VALIDATION_STATUS=NOT_PERFORMED`, `REGULATORY_STATUS=NOT_PERFORMED`.

## Requirements

- **FR-001**: `tauri.conf.json` activates bundling with explicit per-platform targets (`nsis`, `app`, `dmg`, `deb`) and an explicit `webviewInstallMode`. No updater, no remote resource, and no new Tauri plugin or capability.
- **FR-002**: Installers package the same release executable that the root/Tauri CI qualifies. The bundle step uses the locked toolchain (`npm ci --ignore-scripts`, `cargo --locked`).
- **FR-003**: Every build-time download is enumerated with version, URL, verifying hash and source in `research.md`, and is recorded in the job log with its SHA-256. A build-time download is not product runtime network.
- **FR-004**: The package manifest binds the artifact to commit, tree, lockfiles, executable hash, installer hash, SBOM digest, NOTICE digest and toolchain. A verifier recomputes all of them and fails closed on any mismatch, missing file or unexpected file.
- **FR-005**: Install and uninstall qualification never reads or modifies unrelated user data, never types into the app, and never creates a vault.
- **FR-006**: Evidence separates `BUILDABLE_INSTALLER` from `SIGNED_PRODUCTION_INSTALLER`, and `MEDSCALE_APP_EGRESS` from `EMBEDDED_PLATFORM_RUNTIME_EGRESS` (F096-T05).
- **FR-007**: The product name is `MedScale`. The bundle identifier stays `org.medscale.desktop.preview` until a separately governed identifier change, because changing it moves app-local data.
- **FR-008**: CI jobs stay within GitHub-hosted free runners. The Windows job timeout covers the measured native build time plus bundling.

## Qualification matrix

Each cell is recorded separately per platform and exact head. No cell is collapsed into a generic PASS.

| Result | Windows (NSIS) | macOS (app, dmg) | Linux (deb) |
|---|---|---|---|
| `BUILD` | release executable built with locked inputs | same | same |
| `BUNDLE` | `--bundles nsis` | `--bundles app,dmg` | `--bundles deb` |
| `ARTIFACT_INVENTORY` | every payload hashed in `package-manifest.json` | same | same |
| `CHECKSUM` | `SHA256SUMS` recomputed by the verifier | same | same |
| `SBOM_BINDING` | SBOM, NOTICE and lockfile digests in the manifest | same | same |
| `INSTALL` | silent per-user NSIS install on the CI runner | dmg mount and copy to a temporary directory | `dpkg-deb -x` into a temporary root |
| `LAUNCH` | hidden-desktop process probe | non-interactive launch probe | `xvfb-run` probe |
| `UNINSTALL` | silent uninstall plus residue report | removal of the copied app | not applicable (extracted root deleted) |
| `SIGNING` | `NOT_GRANTED` | `NOT_GRANTED` (ad-hoc signature only if required, recorded as ad-hoc) | `NOT_GRANTED` |

## Rollback and update story

- **Rollback:** the change is additive.
  - Setting `bundle.active=false` and removing the CI bundle step restores the Spec 096 package-only path. No Core, data, IPC or capability change needs reverting.
  - Users go back by uninstalling. App-local data under `org.medscale.desktop.preview` is not migrated or deleted by this spec; observed uninstaller behavior is recorded.
- **Update:**
  - There is no auto-updater (anti-scope). A newer engineering installer is installed over the previous one by the NSIS installer's normal per-user flow.
  - Upgrade behavior between two installer versions is a P3 measurement, recorded as not performed until two versions exist.
  - Bundler tool versions change only when the locked `@tauri-apps/cli` changes. Such a change is a forward lockfile change that re-runs the research table and supply-chain review.

## Anti-scope

- Code signing, notarization, distro repository signing.
- MSI/WiX (deferred; NSIS covers the per-user engineering path).
- MSIX, app stores, auto-update.
- AppImage, Flatpak, Snap.
- Upgrade between real released versions.
- Real PHI.
- Reproducible compiler output.
- Changes to the Tauri security boundary (commands, capabilities, CSP).

## Open findings carried

- F096-T01: `glib` 0.18.5 / RUSTSEC-2024-0429. The Linux package inherits it.
- F096-T05: WebView2 runtime background egress and default crash-dump upload.

# Zero-cost distribution of the MedScale beta

**Founder decision (2026-11-12):** MedScale is distributed like a conventional desktop app, through the website and GitHub Releases, **without** buying a Windows code-signing certificate or an Apple Developer Program membership. Paid signing is optional and is not a prerequisite for the unsigned engineering beta.

This page explains what that means for people who install it, what still has to be true before any release is published, and how free signing could be added later.

**Unchanged:** `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. Clinical use is not authorized. The public beta is for synthetic workspaces and licensed non-patient audio only; real patient data is not authorized.

## 1. Release blockers, reclassified

| Class | Items | Blocks an unsigned public beta? |
|---|---|---|
| **Engineering failures** | None open on `main`. Every PR merged only after its exact head passed CI | Yes, whenever one appears |
| **Security and privacy qualification** | Installed-package E2E per platform; checksums, SBOM and provenance per artifact; dependency, vulnerability and malware scans; verified uninstall and data-retention behaviour; a safe update path; the sample workspace refusing accidental real-data import | **Yes.** These are free and required (section 4) |
| **Optional paid platform signing** | Windows Authenticode certificate; Apple Developer ID and notarization | **No.** The founder has declined them. Builds are published as unsigned and labelled so |
| **Platform installation warnings** | SmartScreen and Smart App Control (Windows); Gatekeeper (macOS) | No, but every download must explain them honestly (section 2) |
| **Founder approval of exact artifacts** | One approval per release, naming each file and its SHA-256 | **Yes.** Nothing is published without it |

An unsigned build is never described as signed, notarized or trusted by the operating system. The macOS build's ad-hoc signature is not Developer ID trust and does not pass notarization.

## 2. What people will see, by platform

### Windows

- **SmartScreen.** A downloaded unsigned installer shows *"Windows protected your PC"*. Choose **More info**, check the publisher reads *Unknown publisher* and the file name matches, then choose **Run anyway**. Do this only after checking the SHA-256 on the download page.
- **Smart App Control.** On Windows 11 PCs with Smart App Control **on**, unsigned apps without established reputation are **blocked**, and there is no per-app exception. MedScale won't run on those PCs until it is signed. We do not recommend turning Smart App Control off.
- **Microsoft Defender** scans the installer as usual. Never disable it.
- The installer is **per-user** (`%LOCALAPPDATA%`): no administrator rights. It creates a Start menu shortcut and an uninstaller in *Settings → Apps*. Uninstalling keeps your data in `%LOCALAPPDATA%\org.medscale.desktop.preview` (verified by the Windows lifecycle journey).
- The app needs the Microsoft Edge WebView2 Runtime, which Windows 10 and 11 normally include. The installer does not download it.

### macOS

- The app is **ad-hoc signed and not notarized**. The first time you open it, macOS says it can't verify that MedScale is free of malware.
- To open it: choose **Done**, open **System Settings → Privacy & Security**, find the message about MedScale, and choose **Open Anyway**. You only need to do this once per installed version. On macOS 15 and later, right-click → Open no longer bypasses this.
- Install by dragging **MedScale** into **Applications** from the DMG. To uninstall, move it to the Bin. Your data stays in `~/Library/Application Support/org.medscale.desktop.preview` until you delete it.
- Never disable Gatekeeper for the whole Mac (`spctl --master-disable`). Approving this one app is enough.

### Linux

- Linux **arm64** builds need an ARMv8.2-A CPU with the FP16 extension (for example Raspberry Pi 5, AWS Graviton2 or newer). Older arm64 CPUs such as the Raspberry Pi 4's aren't supported.
- `.deb` packages are installed with `sudo apt install ./MedScale_<version>_amd64.deb` and removed with `sudo apt remove medscale`. Your data stays in `~/.local/share/org.medscale.desktop.preview`.
- AppImages aren't signed. Check the SHA-256, `chmod +x` the file, and run it.
- There's no package repository yet, so updates are manual: download the new package and install it over the old one.

### CLI

- Standalone archives for Windows x64, macOS (Apple Silicon and Intel) and Linux (x64 and arm64), each with `SHA256SUMS`, licenses and an SBOM. They're unsigned, so the same SmartScreen and Gatekeeper behaviour applies to the binary.

## 3. Updates

The app has **no automatic updater**. *Help → Check for updates* runs only when the user clicks it, queries `api.github.com` over HTTPS with redirects refused, and reports whether a newer release exists. It never downloads or installs anything. An unsigned update channel can't be authenticated independently of the download host, so automatic updates stay off until releases are signed, with an update key held apart from the build.

Rollback means installing the previous version's package. Vault metadata carries a schema version, and Core reports an unsupported schema rather than rewriting it. Back up the vault (Settings → Back up vault) before installing a different version.

## 4. What every release candidate must include

Each item below is free, and none needs a paid service.

| Requirement | How |
|---|---|
| Pinned source | One workflow run builds every artifact from one commit SHA, recorded in each manifest |
| Checksums | `SHA256SUMS` for every artifact, and per-file sums inside each archive |
| SBOM | CycloneDX SBOM for the workspace (`scripts/generate-release-sbom.ps1`) and the desktop's dependency inventory |
| Provenance | GitHub artifact attestations (SLSA build provenance), free for public repositories. Verify with `gh attestation verify <file> --repo AbdulazizShehri/MedScale` |
| Dependency and vulnerability scanning | `cargo-deny` (advisories, bans, licenses, sources) on the workspace and on each shipped desktop graph. No advisory ignored without a founder-approved exception |
| Malware scanning | ClamAV with fresh signatures on the runner, over every artifact |
| Artifact inventory | One JSON listing every file, its size, its SHA-256 and the run that built it |
| Installed-package E2E | Install, launch, use, uninstall and data retention on clean runners per platform |
| Reproducibility | Locked dependencies and a pinned toolchain. Byte-identical rebuilds are **not** claimed |
| Public limitations | This page, linked from every download |

## 5. Free signing: SignPath Foundation (Windows)

SignPath Foundation signs open-source Windows binaries for free with a certificate issued to the Foundation, and attributes them to the project. Their terms (<https://signpath.org/terms>) were checked on 2026-11-12.

| Condition | MedScale today |
|---|---|
| OSI-approved license for all components, no commercial dual licensing | MedScale is Apache-2.0. **To check:** one dependency, `webpki-roots` (Mozilla's root store), is `CDLA-Permissive-2.0`, a data license that isn't OSI-approved. Fonts are OFL-1.1, which is OSI-approved |
| No proprietary components | Met. Models are downloaded on request, not bundled |
| Actively maintained | Met |
| **Already released in the form to be signed** | **Not met.** This needs a public unsigned release first, and that needs the founder's artifact approval |
| Functionality described on the download page | Met by the website's download and product pages |
| No malware or PUP; no security-circumvention tools | Met |
| Privacy: no transfer to networked systems unless requested by the user | Met by design (update check, model downloads and Browse are all user-initiated). Needs the statement published on the website |
| A matching uninstall for every install method | Met (NSIS uninstaller) |
| MFA for all team members on SignPath and GitHub | **To confirm by the founder** |
| Defined roles: authors, reviewers, approvers | **To define.** One maintainer today; AI agents author changes and the founder approves |
| "Code signing policy" page with SignPath attribution, team roles and privacy statement | **To add** to the website once accepted |
| Verifiable builds from source, manual approval of every signing request | Compatible: the release workflow builds from one SHA in GitHub Actions |
| Verifiable reputation | **Unknown.** It is SignPath's judgement for a new project |

**Plan, if the founder chooses to apply:**
1. Publish the first unsigned beta (after artifact approval).
2. The founder confirms MFA and names the roles.
3. Add a code-signing policy page, and decide on the `CDLA-Permissive-2.0` question with SignPath.
4. Apply at <https://signpath.org/apply>.
5. If accepted, add a signing step that submits the NSIS installer and `medscale.exe` to SignPath from the release workflow, with manual approval.

MedScale has **not** applied, and nothing here implies SignPath approval. macOS has no equivalent free path: notarization requires a paid Apple Developer membership.

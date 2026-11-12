# MedScale release-readiness report

**Date:** 2026-11-11. **Scope:** the "complete product engineering and cross-platform release readiness" directive, starting from `main@0be749f`. **Prepared by:** an AI engineering agent working under repository governance. Every change below was merged by pinned squash merge after its exact head passed CI. Nothing was published, deployed, signed or purchased.

**Gates unchanged:** `RELEASE_READY=false`. `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. Clinical use: not authorized.

## 1. Decisions

| Decision | Verdict | Why |
|---|---|---|
| **Engineering completeness** | **GO, with named exceptions** | R1–R9 and R11 are done. R10 is partly done. Numeric and unit consistency is open (section 4) |
| **Public release** | **NO-GO** until each release candidate passes the free security qualification and the founder approves its exact artifacts | Paid signing is no longer a blocker: the founder chose unsigned zero-cost distribution (2026-11-12). Blockers are reclassified in [ZERO_COST_DISTRIBUTION.md](../release/ZERO_COST_DISTRIBUTION.md) |
| **Private patient data** | **NO-GO** | `NOT_AUTHORIZED`. Open blockers B1 (rest), B2, B3 (rest), B4 and B6, and no external reviews. See [REAL_PHI_AUTHORIZATION_PACKAGE.md](REAL_PHI_AUTHORIZATION_PACKAGE.md) |
| **Clinical use** | **NO-GO** | Not clinically validated. No regulatory classification or quality evidence. Outputs are proposals and research notes |

## 2. Platforms

"Engineering GO" means the build is produced, tested and qualified in CI on disposable runners as described. It is not a release.

| Platform | Engineering | Public release | Evidence | What is missing |
|---|---|---|---|---|
| Windows x64 desktop | **GO** | NO-GO | Installer qualified (install, launch, uninstall keeping data, reinstall). Desktop journeys, NVDA journey and lifecycle journeys in CI | Installed-build journeys for Canvas, backup and encrypted import. Unsigned: SmartScreen warns, and Smart App Control blocks it (paid signing declined; SignPath Foundation is an option) |
| macOS Apple Silicon desktop | **GO** | NO-GO | DMG built with ad-hoc signature; launch and removal qualified. Desktop journeys through the embedded WebDriver in CI (#494); there, the keyboard journey delivers Enter through the page because that driver sends no native keys. Keyboard, picker and WKWebView qualification workflows | A full installed-DMG journey; VoiceOver. Unsigned: Gatekeeper requires "Open Anyway" (paid Developer ID declined) |
| macOS Intel desktop | **NO-GO** | NO-GO | Not built. The CLI's Intel binary is cross-built and runs under Rosetta 2 in CI, so the toolchain works | An x86_64 desktop build and its qualification |
| Linux x86_64 desktop | **GO** (engineering `.deb`) | NO-GO | glib soundness fix vendored (RUSTSEC-2024-0429, no advisory ignored). `.deb` installed, launched under Xvfb and removed in CI. Desktop journeys through tauri-driver (#494) | AppImage, a qualified `.rpm`, package signing; Linux arm64 desktop |
| CLI: Windows x64, macOS arm64 and x64, Linux x64 and arm64 | **GO** | NO-GO | Standalone archives with licenses, notices, SBOM and SHA-256. Each one installed from its archive into a clean PATH and run. Binary tests of exit codes and the encrypted-vault lifecycle | Approval of exact artifacts for download; unsigned |

## 3. What changed (PRs)

| PR | Change |
|---|---|
| #484 | Feature parity audit and frozen scope |
| #485 | Verified backup and restore: storage, Core, desktop, CLI |
| #486 | Corrections to release documents |
| #487 | Evidence corpus labelled as the bundled sample |
| #488 | MedAgent refusals shown in the desktop |
| #489 | CLI opens encrypted vaults and seals them; recovery codes usable |
| #490 | Linux desktop with fixed glib |
| #491 | CLI exit codes, binary tests, `fhir export`, release workflow for five targets |
| #492 | FHIR import into the encrypted vault, with the patient list sealed |
| #493 | Research Canvas in the desktop |
| #494 | Desktop journeys on Linux and macOS |
| #495 | Real patient data decision package |
| #496 | Windows desktop CI back on Strawberry Perl (regression from #490) |

Earlier in the same effort: #474–#483 (OpenMed-derived clinical checks in Core, the RUSTSEC-2026-0459 exception approved by the founder, MedAgent evidence gate, Canvas coverage).

## 4. Known gaps (engineering)

| Gap | Impact | Next step |
|---|---|---|
| Numeric and unit consistency | Coverage can report equal values written differently as conflicting | Port the reference UCUM conversion with conformance vectors |
| Installed-build journeys for new features | Canvas, backup and encrypted import are tested through IPC and test-build journeys, not on installed packages | Extend `installed-journey.mjs` and add a DMG journey |
| Backup and restore in the GUI journeys | The native folder chooser can't be driven by WebDriver | IPC tests cover it; a native-dialog test needs OS automation |
| Linux AppImage, Linux arm64 desktop | Not built | Add bundles and an Arm job |
| B2 (real-world FHIR), B3 (passphrase change, code revocation), B4 (rollback, tamper-evident audit) | Block real data | Engineering work items in `REAL_DATA_READINESS.md` |
| Sample workspace accepts any FHIR file | A real record could land unencrypted | Founder decision to restrict it to bundled fixtures |

## 5. Decisions only the founder can make

1. Buy code-signing certificates (Windows) and use Apple Developer ID credentials (macOS). Both cost money.
2. Approve exact artifacts for public download, and publish them.
3. Commission the independent security review, the legal review and the DPIA (section 4 of the real-data package).
4. Decide whether the sample workspace should refuse non-fixture files.
5. Any change to `REAL_PHI_AUTHORIZATION` or clinical use, in writing.

## 6. How this was verified

- Every merged PR passed its full CI on its exact head (Rust on Windows, macOS and Linux, Clippy with warnings as errors, cargo-deny, the Tauri preview and packaging qualification, desktop E2E). It was merged by pinned squash merge.
- Local checks were limited to formatting, the crates that build on the founder's PC (`medscale-contracts`, `medscale-clinical`) and cargo-deny on the Linux desktop graph. The rest builds only in CI, because of vendored OpenSSL and SQLCipher.
- No installer was run on the founder's PC, and no real patient data was used anywhere.

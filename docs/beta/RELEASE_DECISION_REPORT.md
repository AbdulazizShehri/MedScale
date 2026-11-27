# MedScale release-readiness report

**Date:** 2026-11-11, updated 2026-11-12 for zero-cost distribution and release candidate 0.1.0-beta.1. **Scope:** the "complete product engineering and cross-platform release readiness" directive, starting from `main@0be749f`. **Prepared by:** an AI engineering agent working under repository governance. Every change below was merged by pinned squash merge after its exact head passed CI. Nothing was published, deployed, signed or purchased.

**Gates unchanged:** `RELEASE_READY=false`. `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`. Clinical use: not authorized.

## 1. Decisions

| Decision | Verdict | Why |
|---|---|---|
| **Engineering completeness** | **GO, with named exceptions** | R1–R11 done; installed-build journeys cover encrypted import and the Canvas (backup is IPC-tested only, because WebDriver can't drive the native folder chooser). Unit-aware coverage is done for the admitted UCUM subset (#505) |
| **Public release** | **Ready for the founder's decision** on candidate `0.1.0-beta.1`: every file passed the free security qualification. Publication needs the founder's approval of the exact files | Paid signing is no longer a blocker: the founder chose unsigned zero-cost distribution (2026-11-12). Blockers are reclassified in [ZERO_COST_DISTRIBUTION.md](../release/ZERO_COST_DISTRIBUTION.md) |
| **Private patient data** | **NO-GO** | `NOT_AUTHORIZED`. Open blockers B1 (rest), B2 and B6, and no external reviews. See [REAL_PHI_AUTHORIZATION_PACKAGE.md](REAL_PHI_AUTHORIZATION_PACKAGE.md) |
| **Clinical use** | **NO-GO** | Not clinically validated. No regulatory classification or quality evidence. Outputs are proposals and research notes |

## 2. Platforms

"Engineering GO" means the exact file is built, installed and tested in CI on disposable runners, as listed. It is not a release.
Evidence for every row is refreshed release candidate `0.1.0-beta.1` from `main@80c67d2d` ([run 45916049004](https://github.com/AbdulazizShehri/MedScale/actions/runs/45916049004), artifact `15164819520`, superseding the earlier `6d0279e` candidate in run `43247164670`), with file names and SHA-256 values in [RELEASE_CANDIDATE_0.1.0-beta.1.md](../release/RELEASE_CANDIDATE_0.1.0-beta.1.md).

| Platform | Engineering | Public release | Evidence in the candidate | Limitations shown to users |
|---|---|---|---|---|
| Windows x64 installer | **GO** | Awaiting founder approval | Full installed journey (25 steps, including `#531` hard zero-residue assertions): install with Mark of the Web, sample workspace, encrypted-only FHIR refusal (`#524`), model download/recovery, real speech, encrypted vault, encrypted FHIR import, Research Canvas, 0 WebView2 disk residue after positive control, uninstall keeping data, reinstall | Unsigned: SmartScreen warns; **Smart App Control blocks it** |
| Windows x64 portable zip | **GO** | Awaiting founder approval | Extracted and opened | Same as the installer |
| macOS Apple silicon DMG | **GO** | Awaiting founder approval | Real-DMG install, Gatekeeper record, launch, removal; desktop journeys on test builds | Ad-hoc signed, not notarized: Open Anyway once |
| macOS Intel DMG | **GO** | Awaiting founder approval | Built on an Intel runner; real-DMG install, launch, removal | Same as Apple silicon |
| Linux x64 .deb and AppImage | **GO** | Awaiting founder approval | .deb install, launch, reinstall keeping data, removal; AppImage opened; desktop journeys on test builds | Unsigned; WebKitGTK 4.1 (Ubuntu 22.04+) |
| Linux arm64 .deb and AppImage | **GO** | Awaiting founder approval | Same as x64, on an Arm runner | Needs ARMv8.2-A with FP16 |
| CLI (5 targets) | **GO** | Awaiting founder approval | Binary tests and a clean-PATH run of each archive; the Intel macOS archive is smoke-run under Rosetta 2 only | Unsigned |

All 34 candidate files (13 downloadable packages, root SBOM, 5 notices archives, and 15 per-installer package manifests/SBOMs/NOTICE companions) passed ClamAV 1.5.4 (0 infected), checksum re-verification (34/34 OK), and carry GitHub SLSA v1 build-provenance attestations ([attestation 72905331](https://github.com/AbdulazizShehri/MedScale/attestations/72905331)). Upgrade between released versions is untested because there is no earlier release.

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
| #499 | rustix 1.1.6 for RUSTSEC-2026-0486 |
| #500 | Zero-cost distribution: blockers reclassified, platform warnings, updater assessment, SignPath evaluation |
| #501 | Sample workspace refuses unconfirmed or national-identifier imports |
| #502 | Intel DMG, Linux arm64, AppImage, Windows portable zip |
| #503 | Release-candidate workflow: qualified files only, ClamAV, provenance, installed journeys |
| #504 | Download page for every package, gated on approval |
| #505 | Coverage compares quantities, not text |
| #506 | Linux reinstall keeps data |
| #509, #510 | Doc corrections; release-candidate permissions and clean CLI packaging |

Earlier in the same effort: #474–#483 (OpenMed-derived clinical checks in Core, the RUSTSEC-2026-0459 exception approved by the founder, MedAgent evidence gate, Canvas coverage).

## 4. Known gaps (engineering)

| Gap | Impact | Next step |
|---|---|---|
| Analyte-specific unit conversion | mg/dL and mmol/L still can't be compared (needs a molar mass); decimals are refused at import (B2) | Same-dimension conversion is done (#505); molar conversion is future work |
| Backup and restore on an installed build | Tested through IPC and the CLI binary, not through the native folder chooser | A native-dialog test needs OS automation |
| Backup and restore in the GUI journeys | The native folder chooser can't be driven by WebDriver | IPC tests cover it; a native-dialog test needs OS automation |
| Upgrade between released versions | Untested: no earlier release exists | Run it on the second release |
| B2 (real-world FHIR) | Block real data | Engineering work items in `REAL_DATA_READINESS.md` |
| Sample workspace import | Needs a no-real-data confirmation and refuses national patient identifiers (#501); a false confirmation can still import a real record | Accepted as the safest reasonable control for the unencrypted sample |

## 5. Decisions only the founder can make

1. Buy code-signing certificates (Windows) and use Apple Developer ID credentials (macOS). Both cost money.
2. Approve exact artifacts for public download, and publish them.
3. Commission the independent security review, the legal review and the DPIA (section 4 of the real-data package).
4. Decide whether the sample workspace should refuse non-fixture files.
5. Any change to `REAL_PHI_AUTHORIZATION` or clinical use, in writing.

## 6. How this was verified

- Every merged PR passed its full CI on its exact head (Rust on Windows, macOS and Linux, Clippy with warnings as errors, cargo-deny, the Tauri preview and packaging qualification, desktop E2E). It was merged by pinned squash merge.
- Local checks on Windows verified Rust workspace crates (`medscale-contracts`, `medscale-keys`, `medscale-clinical`, `medscale-fhir`, `medscale-network`, `medscale-pack`, `medscale-storage`, `medscale-core`, `medscale-cli` unit and binary tests), desktop frontend tests (`apps/desktop-tauri`, 85/85), website tests (`web/`, 26/26, proof check, release gate), repository hygiene, and dependency direction.
- No installer was published, no release tag was created, and no real patient data was used anywhere.

## 7. Engineering hardening update (2026-11-12 to 2026-11-26)

Following the `0.1.0-beta.1` candidate built at `6d0279e`, additional engineering PRs were merged into `main` after passing full exact-head CI:

- `#511`–`#525`: `0.1.0-beta.1` candidate report, passphrase change & recovery-code replacement (B3), rollback generation counter & hash-chained audit (B4), FHIR exact decimal preservation / Bundle splitting / streaming / exact citations / public HL7 example coverage (B2), legal drafts for counsel review (B6, not in effect), encrypted-vault-only FHIR import (B1), and per-installer package manifest / website manifest generation (`#525`, re-assembled in run `43693295275`).
- `#526`: 64 MiB Argon2id default with strict KDF bounds and transparent legacy 19 MiB → 64 MiB rewrap, comprehensive secret zeroization, cross-platform crash-dump hardening, dependency updates (`url 2.5.9` / `RUSTSEC-2026-0592`, `cookie 0.18.3` / `RUSTSEC-2026-0594`), and `docs/handoff/ANTIGRAVITY_RECOVERY.md`.
- `#527`: keyed HMAC-SHA256 sealed-blob filenames (`k1_<hmac>.sealed` with legacy fallback), Browse URL path PII/PHI scanning (keeping unlabelled PubMed/PMC IDs allowed), and fixed path-free storage error messages.
- `#528`: in-app confirmed **Remove saved model…** recovery (closes `#459`), Rust-side 15-minute encrypted-vault idle auto-lock, recovery-code screen `beforeunload` warning (`#457`), and transcript clipboard disclosure.
- `#529`: safe model acquisition destination/staging guards, defensive read-only external SQLite (`SQLITE_DBCONFIG_DEFENSIVE` + `trusted_schema = OFF` + `query_only = ON`), Windows/CurDir reserved vault path bypass fixes, and model config bounds + `catch_unwind` panic isolation in `medscale-pack`.
- `#530`: `scripts/e2e/resolve-edge-webdriver.ps1` matching Edge WebDriver major version to installed WebView2 Runtime on Windows E2E runners.
- `#531`: sealed desktop pack/ASR helper records inside `meta.sealed` (`vault_helper_records`) with legacy plaintext sidecar migration/removal and hard WebView2 residue assertions in `scripts/e2e/installed-journey.mjs`.

See [MEDSCALE_FINAL_ENGINEERING_STATUS.md](../handoff/MEDSCALE_FINAL_ENGINEERING_STATUS.md) for the complete engineering handoff, exact commits, artifact inventory, and founder-only publication steps.

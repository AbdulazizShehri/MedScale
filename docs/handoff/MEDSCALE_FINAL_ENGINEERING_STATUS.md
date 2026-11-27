# MedScale Final Engineering Status & Handoff

**Date:** 2026-11-27  
**Repository:** `https://github.com/AbdulazizShehri/MedScale`  
**Website:** `https://medscale.si`  
**License:** Apache-2.0 for MedScale first-party code (third-party licenses and notices preserved verbatim)  
**Prepared by:** Autonomous engineering continuation (resumed from verified Claude Code checkpoint at `main@5e94563f`, 2026-11-15; refreshed Option B `0.1.0-beta.1` candidate built and qualified from `main@80c67d2d`, 2026-11-27)

---

## 1. Explicit Governance & Release State Flags

| Flag | Current Value | Authority / Meaning |
| --- | --- | --- |
| `ENGINEERING_COMPLETE` | `true` *(within authorized engineering scope)* | All Priority 1–4 engineering tasks, security hardening, usability fixes, CI resilience fixes, and Option B fresh `0.1.0-beta.1` candidate build & qualification are complete on `main`. |
| `RELEASE_CANDIDATE_QUALIFIED` | `true` *(unsigned `0.1.0-beta.1` from `main@80c67d2d`)* | Fresh non-public candidate (`medscale-release-candidate-0.1.0-beta.1`, run [45916049004](https://github.com/AbdulazizShehri/MedScale/actions/runs/45916049004), Artifact ID `15164819520`, 471,748,741 bytes, expires `2027-02-25T00:19:31Z`) passed 15/15 workflow jobs, 34/34 `SHA256SUMS` (CI + local), ClamAV 1.5.4 (34 scanned, 0 infected), and SLSA v1 provenance attestation ([72905331](https://github.com/AbdulazizShehri/MedScale/attestations/72905331), 34 subjects). |
| `FOUNDER_RELEASE_APPROVAL` | `PENDING` | Founder-only action: no AI agent may create an immutable GitHub Release, post `APPROVE` / `SIGNING` comments, or activate public download links. |
| `PUBLIC_BETA_PUBLISHED` | `false` | No `v0.1.0-beta.1` GitHub Release has been published (only the historical `2026-10-23` pre-release `engineering-preview-19d419dd7b15` exists); `web/src/lib/release-manifest.json` remains `{"schema": 1, "release": null}` and `QUALIFIED_RELEASE_TAGS` remains `[]`, verified by `npm run verify-release`. |
| `RELEASE_READY` | `false` | Public gate flag stays `false` until founder publishes and activates the release. |
| `REAL_PHI_AUTHORIZATION` | `NOT_AUTHORIZED` | Real patient data remains strictly unauthorized pending independent human security review, legal/regulatory review (B6), and written founder authorization. |
| `CLINICAL_USE_AUTHORIZATION` | `NOT_AUTHORIZED` | MedScale is not clinically validated or cleared as a medical device. |
| `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` | `OPEN` | Re-verified 0 disk residue across 5 consecutive Windows installed journeys (including `run 45916049004` with `#531`'s hard CI assertions active), but remains `OPEN` pending independent human security review. |

---

## 2. Integrated Engineering Work (Merged into `main`)

Baseline resumed from: `5e94563fa1209f5045c7e91d70153e1c2a740b38` (`release: publish each installer's package manifest, and generate the site manifest (#525)`, 2026-11-15).

Every PR below was merged into `main` by pinned squash merge only after **100% of exact-head CI checks passed**, and post-merge CI on `main@80c67d2dfae139f7471f7f414c9cd112d8a5b07e` completed 100% green (`ci` run `45895549441` 6/6, `desktop-e2e` `45895549305`, `tauri-preview` `45895549278`) before dispatching the Option B release candidate:

| PR | Merged `main` Commit | Exact-Head CI | Summary |
| --- | --- | --- | --- |
| [#526](https://github.com/AbdulazizShehri/MedScale/pull/526) | `95c2f370` | **24/24 PASS** | 64 MiB Argon2id default (`t=3, p=1`), strict KDF parameter bounds, transparent legacy 19 MiB → 64 MiB header rewrap on passphrase unlock; full secret zeroization (`Zeroizing`/`ZeroizeOnDrop` + IPC/React cleanup); cross-platform crash-dump hardening (`SetErrorMode` + `WerSetFlags`, `RLIMIT_CORE=0`, `PR_SET_DUMPABLE=0`); dependency updates (`url 2.5.9` / `RUSTSEC-2026-0592`, `cookie 0.18.3` / `RUSTSEC-2026-0594`); `docs/handoff/ANTIGRAVITY_RECOVERY.md`. |
| [#528](https://github.com/AbdulazizShehri/MedScale/pull/528) | `b4076aed` | **24/24 PASS** | Closes `#459`: confirmed in-app **Remove saved model…** recovery (`audio_model_remove` behind 5-model allowlist); Rust-side 15-minute encrypted-vault idle auto-lock (`State::enforce_idle_lock`); `#457` `beforeunload` warning & updated Settings guidance on shown-once recovery-code screen; transcript OS-clipboard exposure disclosure. |
| [#530](https://github.com/AbdulazizShehri/MedScale/pull/530) | `2ab3ceeb` | **9/9 PASS** | Added `scripts/e2e/resolve-edge-webdriver.ps1` wired into `desktop-e2e.yml` and `audio-real-desktop-e2e.yml` to resolve a matching `msedgedriver` whenever GitHub `windows-latest` runners have Edge WebDriver / WebView2 major-version skew (e.g., 158 vs 157). |
| [#529](https://github.com/AbdulazizShehri/MedScale/pull/529) | `17cb099d` | **24/24 PASS** | Safe model acquisition destination & staging guards (refuses overwriting non-empty user folders or unmarked `.partial` directories); defensive read-only external SQLite (`SQLITE_DBCONFIG_DEFENSIVE`, `PRAGMA trusted_schema = OFF`, `PRAGMA query_only = ON`); reserved vault path bypass fix (`./` and Windows trailing dots/spaces); model config bounds & `catch_unwind` panic isolation in `medscale-pack`. |
| [#527](https://github.com/AbdulazizShehri/MedScale/pull/527) | `10818e03` | **23/23 PASS** | Keyed HMAC-SHA256 sealed-blob filenames (`k1_<hmac>.sealed` under vault DEK with legacy `<digest>.sealed` fallback read); Browse URL path PII/PHI scanning (handling percent-encoded and colon/underscore REST paths while preserving unlabelled public PubMed/PMC IDs); fixed path-free storage error messages plus `sanitize_error_message` defense-in-depth in desktop `CmdError`. |
| [#531](https://github.com/AbdulazizShehri/MedScale/pull/531) | `81cf3b22` | **23/23 PASS** | Sealed desktop pack/ASR helper records inside encrypted vaults (`vault_helper_records` additive table in `meta.sealed`) with automatic legacy `.desktop-packs.json`/`.desktop-asr.json` migration and deletion, wired into `#459` model removal; hard WebView2 disk-residue assertions in `scripts/e2e/installed-journey.mjs`. |
| [#532](https://github.com/AbdulazizShehri/MedScale/pull/532) | `80c67d2d` | **6/6 PASS** | Updated `REAL_DATA_READINESS.md`, `KNOWN_ISSUES.md`, `RELEASE_DECISION_REPORT.md`, `RELEASE_CANDIDATE_0.1.0-beta.1.md`, and created `docs/handoff/MEDSCALE_FINAL_ENGINEERING_STATUS.md`. |

---

## 3. Fresh Option B `0.1.0-beta.1` Release Candidate (`main@80c67d2d`)

**Non-public workflow run:** [45916049004](https://github.com/AbdulazizShehri/MedScale/actions/runs/45916049004) (`release-candidate.yml`, dispatched on `main` at `80c67d2dfae139f7471f7f414c9cd112d8a5b07e`, 2026-11-27T00:19:29Z → 01:49:22Z; **15/15 jobs completed / success**)
- **Staged artifact:** `medscale-release-candidate-0.1.0-beta.1` (Artifact ID `15164819520`, `471,748,741` bytes)
- **Archive digest:** `sha256:90ec38e41863cce4915320a71bd63d7d387aaa40dde730cf589ed2ddd7c06815`
- **Expiration:** `2027-02-25T00:19:31Z` (90-day retention)
- **Checksum verification:** `sha256sum -c SHA256SUMS` **34/34 OK** in CI and independently re-hashed locally across all 34 files (**34/34 verified, 0 mismatches**)
- **Package manifests & SBOMs:** All 6 CycloneDX 1.5 SBOMs parsed cleanly (**849 components** each), and all 5 Spec 101 per-installer `package-manifest.json` files cross-checked against exact `source_sha`, installer bytes/SHA-256, platform SBOM SHA-256, and platform NOTICE SHA-256
- **Malware scan (`clamav.txt`):** ClamAV `1.5.4` (`3,628,228` signatures), `Scanned files: 34` (`1.80 GiB` expanded archive data), **`Infected files: 0`**
- **Build provenance:** `actions/attest-build-provenance` signed SLSA v1 attestation for all **34 subjects** via Public Good Sigstore + Rekor ([attestation 72905331](https://github.com/AbdulazizShehri/MedScale/attestations/72905331)); independently verified locally with `gh attestation verify MedScale_0.1.0_x64-setup.exe -R AbdulazizShehri/MedScale` (`https://slsa.dev/provenance/v1`, `SIG_OK=true`)
- **Website manifest dry-run:** In-memory `checkedManifest` validation (`web/src/lib/release-manifest-gen.ts`) accepted all 13 download entries from `artifact-inventory.json` with zero errors while keeping `web/src/lib/release-manifest.json` untouched (`"release": null`) and `QUALIFIED_RELEASE_TAGS` empty (`[]`)

### Platform Qualification Evidence (Run `45916049004`)

| Platform / Surface | Engineering Status | Signing State | Exact Qualification Evidence (`run 45916049004`) |
| --- | --- | --- | --- |
| **Windows x64 installer** (`MedScale_0.1.0_x64-setup.exe`, 14,315,377 B) | **Qualified (unsigned Beta)** | `unsigned` (`NOT_GRANTED`) | **25/25** installed-journey steps PASS (`windows.json`): Mark of the Web install, no-account launch, sample FHIR refusal (`#524`), 5/5 launches `inprivate: true`, `pref_on_disk` `0` hits (`131`/`149` scanned), `0` unnamed focusable controls, `0` clipped layouts, `0` idle remote connections, failed-download & kill-during-download recovery (`0` partial leftovers), consented Whisper download (`151,060,136` B) & real speech transcription (`0.818` recall across 11 words), encrypted vault (64 MiB Argon2id + keyed blobs + sealed helpers) & encrypted FHIR import, Research Canvas note, positive-control WebView scan PASS, `webview_residue` `0` hits (`149` scanned, `0` unreadable) enforced by `#531` hard assertion, uninstall keeping data (`user_data_kept_after_uninstall: true`), reinstall opening same vault, final uninstall. |
| **Windows x64 portable zip** (`MedScale_0.1.0_x64-portable.zip`, 24,741,925 B) | **Qualified (unsigned Beta)** | `unsigned` | Extracted and opened on a clean runner (`PORTABLE_LAUNCH: PASS`). |
| **macOS Apple Silicon DMG** (`MedScale_0.1.0_aarch64.dmg`, 21,196,503 B) | **Qualified (ad-hoc Beta)** | `ad-hoc (not Developer ID, not notarized)` | Real-DMG check (`macos-latest`): `DMG_VERIFY=PASS`, `COPY_TO_APPLICATIONS=PASS`, `CODESIGN_VERIFY=PASS` (`0x10002(adhoc,runtime)`), `SPCTL: rejected`, `LAUNCH_FROM_APPLICATIONS=PASS`, `ALIVE_AFTER_LAUNCH=PASS`, `QUIT=PASS`, `REOPEN=PASS`, quarantined Gatekeeper block recorded (`CASE_DOWNLOADED_0081=GATEKEEPER_BLOCKED`), Open Anyway verified (`CASE_SIMULATED_01C1=WINDOW_VERIFIED`), `REMOVE_APP=PASS`, `USER_DATA_AFTER_REMOVAL=kept`. |
| **macOS Intel DMG** (`MedScale_0.1.0_x64.dmg`, 23,220,592 B) | **Qualified (ad-hoc Beta)** | `ad-hoc (not Developer ID, not notarized)` | Built on `macos-15-intel`; identical real-DMG check (`macos-15-intel`): `DMG_VERIFY`, `COPY_TO_APPLICATIONS`, `CODESIGN_VERIFY`, `LAUNCH_FROM_APPLICATIONS`, `QUIT`, `REOPEN`, Gatekeeper + Open Anyway verification, `REMOVE_APP`, `USER_DATA_AFTER_REMOVAL=kept` all PASS. |
| **Linux x64 `.deb` & AppImage** (24,174,660 B / 100,006,392 B) | **Qualified (unsigned Beta)** | `unsigned` | `.deb` `INSTALL`/`LAUNCH`/`UNINSTALL: PASS` (with reinstall keeping data); AppImage `APPIMAGE_LAUNCH: PASS` on `ubuntu-22.04`. |
| **Linux arm64 `.deb` & AppImage** (24,576,942 B / 98,355,720 B) | **Qualified (unsigned Beta)** | `unsigned` | `.deb` `INSTALL`/`LAUNCH`/`UNINSTALL: PASS` (with reinstall keeping data); AppImage `APPIMAGE_LAUNCH: PASS` on `ubuntu-22.04-arm`. Requires ARMv8.2-A CPU with FP16. |
| **CLI (5 targets)** | **Qualified (unsigned Beta)** | `unsigned` | `BINARY_TESTS_AND_CLEAN_PATH_RUN: PASS` for Windows x64, macOS arm64, Linux x64, Linux arm64; macOS x64 extracted & smoke-run under Rosetta 2; packaged Windows CLI binary locally smoke-tested (`medscale 0.1.0`). |
| **Website (`medscale.si`, `web/`)** | **Verified & gated (inactive)** | n/a | `release-manifest.json` (`"release": null`), `QUALIFIED_RELEASE_TAGS = []`, `npm test` 26/26 PASS, `node scripts/check-proof.mjs` PASS, `npm run verify-release` PASS (`no release in the manifest, nothing to verify`). |

### Founder Publication Package (13 `APPROVE` + 13 `SIGNING` Lines)

Full 34-file table (including all 15 per-installer `package-manifest.json`, `SBOM.cdx.json`, and `NOTICE.md` companions) is recorded in [RELEASE_CANDIDATE_0.1.0-beta.1.md](../release/RELEASE_CANDIDATE_0.1.0-beta.1.md). To publish `v0.1.0-beta.1`:

1. Create immutable GitHub Release `v0.1.0-beta.1` at commit `80c67d2dfae139f7471f7f414c9cd112d8a5b07e` uploading **all 34 files + `SHA256SUMS`** from artifact `15164819520`.
2. Post one founder GitHub comment containing the exact 13 `APPROVE <sha256>` lines and 13 `SIGNING <key> <state>` lines listed in [RELEASE_CANDIDATE_0.1.0-beta.1.md](../release/RELEASE_CANDIDATE_0.1.0-beta.1.md).
3. Run `web/scripts/manifest-from-candidate.mjs` against `artifact-inventory.json`, verify `npm run verify-release` passes against the published bytes, and deploy `medscale.si`.

---

## 4. Privacy & Security Readiness Ledger (`REAL_DATA_READINESS.md`)

All engineering blockers (**B1–B5**) and technical hardening items are implemented, tested on `main`, and packaged into the `main@80c67d2d` release candidate:

| Item | Status | Evidence |
| --- | --- | --- |
| **B1** Encrypted-only FHIR import & sealed patient list | **Done** (`#492`, `#524`) | Sample workspace refuses all user imports (verified live in `run 45916049004` step `sample-import-needs-encrypted-vault`); encrypted vault keeps resources and `roster_from_core` sealed. |
| **B2** Real FHIR decimals, Bundle splitting, streaming, exact citations | **Done for import** (`#516`–`#522`) | Tested against 5 public HL7 US Core & IPS fixtures (`fixtures/public/hl7`); vendor-specific portal exports not yet tested. |
| **B3** Idle auto-lock, passphrase change, recovery-code replacement | **Done** (`#512`, `#528`) | Enforced in both UI and Rust host (`State::enforce_idle_lock`); CLI + Settings replacement; `#457` close-guard. |
| **B4** Rollback generation counter & hash-chained audit | **Done (with documented local-disk limits)** | Sealed generation counter + device-local check; transactional hash-chained `audit_chain` + `medscale vault verify-audit`. |
| **B5** Verified backup & restore | **Done** (`#485`) | Desktop + CLI verified backup/restore keeping replaced vault; tested across storage, Core, IPC, CLI. |
| **B6** Legal notices, terms, intended use | **Drafts ready for counsel** (`#523`) | `docs/legal/drafts/` — not in effect; requires legal counsel and founder approval. |
| 64 MiB Argon2id + transparent rewrap | **Done** (`#526`) | `medscale-keys` + `medscale-storage` tests + full workspace CI + live installed-journey vault creation. |
| Secret zeroization in memory | **Done** (`#526`) | `Zeroizing`/`ZeroizeOnDrop` across key derivation, recovery codes, IPC, and React state. |
| Cross-platform crash-dump hardening | **Done** (`#526`) | `medscale_contracts::process_hardening::disable_crash_dumps()` invoked at Desktop & CLI startup. |
| Keyed sealed-blob filenames | **Done** (`#527`) | `k1_<hmac>.sealed` + legacy fallback; `sealed_blob` unit tests + `encrypted_vault_durability_109` (11/11). |
| Fixed path-free error text | **Done** (`#527`) | Core/storage fixed errors + desktop `sanitize_error_message` regression tests. |
| Browse URL path PII scanning | **Done** (`#527`) | `authority::browse::tests` (3/3) + `browse_080` (7/7); desktop uses offline Browse fixture. |
| Sealed desktop helper files | **Done** (`#531`) | `vault_helper_records` in `meta.sealed`; zero plaintext `.desktop-packs.json`/`.desktop-asr.json` beside encrypted vaults; verified in live Windows installed journey (`run 45916049004`). |
| Safe model destination, defensive SQLite, panic isolation | **Done** (`#529`) | `model_acquisition` (18/18), `locator_guard_tests`, `medscale-pack` (111/111). |
| BitLocker / FileVault policy | **Founder decision** | Policy choice whether to detect or require OS full-disk encryption. |

---

## 5. Reproducible Local Build & Verification Results

Verified locally on Windows 11 (Rust `1.100.0`, Node `v24.19.0`, Python `3.14.7`, Strawberry Perl + NASM, VS 2022 Build Tools):

- Workspace Rust crates (`medscale-contracts`, `medscale-keys`, `medscale-clinical`, `medscale-fhir`, `medscale-network`, `medscale-pack`, `medscale-storage`, `medscale-core`, `medscale-cli` unit + binary tests) and `cargo clippy --all-targets --locked -- -D warnings`: **PASS**
- `apps/desktop-tauri`: `npm test` — **85/85 PASS**
- `web`: `npm test` — **26/26 PASS**; `node scripts/check-proof.mjs` — **PASS**; `npm run verify-release` — **PASS** (`no release in the manifest, nothing to verify`); in-memory `checkedManifest` dry-run against `run 45916049004` inventory — **PASS** (13 platforms)
- Candidate artifact `15164819520` local verification: **34/34 SHA-256 verified** (0 mismatches), **6/6 CycloneDX 1.5 SBOMs** valid (849 components), **5/5 package manifests** cross-checked, `gh attestation verify` SLSA v1 **PASS**, packaged Windows CLI smoke-run `medscale 0.1.0` **PASS**
- `python scripts/check_public_repository_hygiene.py`: **PASS**
- `pwsh -NoProfile -File scripts/check-dependency-direction.ps1`: **PASS**

---

## 6. Remaining Founder-Only & External Decisions

Engineering has no remaining implementation or qualification blockers for the `0.1.0-beta.1` public engineering Beta. Only founder-only and external-authority gates remain open:

1. **Public Beta publication decision (the one remaining step for `0.1.0-beta.1`):** Review artifact `15164819520` (run [45916049004](https://github.com/AbdulazizShehri/MedScale/actions/runs/45916049004)), create immutable GitHub Release `v0.1.0-beta.1` at `80c67d2dfae139f7471f7f414c9cd112d8a5b07e` with all 34 files + `SHA256SUMS`, post the founder `APPROVE` + `SIGNING` comment, and authorize activating `medscale.si` downloads.
2. **Code signing & distribution route (`#507` / Draft PR `#508`):** Decide timing of Microsoft Store MSIX submission (Partner Center) and SignPath Foundation application (once first public release exists) or paid Windows/Apple Developer ID certificates.
3. **Independent human security review (OPEN):** Commission an external security assessment (required to close `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` and `PRIVATE_DATA_READY`).
4. **Legal & regulatory review (B6, OPEN):** Have qualified counsel review `docs/legal/drafts/` (FTC Health Breach Notification Rule, GDPR Art. 9, Saudi PDPL/SFDA, Washington MHMDA, EU MDR Rule 11 / FDA CDS, and SNOMED/LOINC/ICD terminology licensing).
5. **Disk encryption policy & real-PHI authorization (OPEN):** Decide BitLocker/FileVault check policy and whether to ever authorize real patient data or clinical use in writing (`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED` and `CLINICAL_USE_AUTHORIZATION=NOT_AUTHORIZED` remain strictly in force).

# MedScale Final Engineering Status & Handoff

**Date:** 2026-11-26  
**Repository:** `https://github.com/AbdulazizShehri/MedScale`  
**Website:** `https://medscale.si`  
**License:** Apache-2.0 for MedScale first-party code (third-party licenses and notices preserved verbatim)  
**Prepared by:** Autonomous engineering continuation (resumed from verified Claude Code checkpoint at `main@5e94563f`, 2026-11-15)

---

## 1. Explicit Governance & Release State Flags

| Flag | Current Value | Authority / Meaning |
| --- | --- | --- |
| `ENGINEERING_COMPLETE` | `true` *(within authorized engineering scope)* | All Priority 1–4 engineering tasks, security hardening, usability fixes, CI resilience fixes, and qualification docs are implemented, verified, and merged into `main` after exact-head CI passes. |
| `RELEASE_CANDIDATE_QUALIFIED` | `true` *(unsigned `0.1.0-beta.1`)* | Re-assembled candidate (`medscale-release-candidate-0.1.0-beta.1-reassembled`, run `43693295275`, built from `6d0279e9` in run `43247164670`) verified 34/34 `SHA256SUMS`, ClamAV 0 infected, 34 attested subjects, expires `2027-02-13T08:10:59Z`. |
| `FOUNDER_RELEASE_APPROVAL` | `PENDING` | Founder-only action: no AI agent may create an immutable GitHub Release, post `APPROVE` / `SIGNING` comments, or activate public download links. |
| `PUBLIC_BETA_PUBLISHED` | `false` | Zero GitHub Releases published; `web/src/lib/release-manifest.json` remains empty and verified by `npm run verify-release`. |
| `RELEASE_READY` | `false` | Public gate flag stays `false` until founder publishes and activates the release. |
| `REAL_PHI_AUTHORIZATION` | `NOT_AUTHORIZED` | Real patient data remains strictly unauthorized pending independent human security review, legal/regulatory review (B6), and written founder authorization. |
| `CLINICAL_USE_AUTHORIZATION` | `NOT_AUTHORIZED` | MedScale is not clinically validated or cleared as a medical device. |
| `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` | `OPEN` | Re-verified 0 disk residue across 4 consecutive installed journeys and promoted to a hard CI assertion (`#531`), but remains `OPEN` pending independent human security review. |

---

## 2. Integrated Engineering Work (Merged into `main`)

Baseline resumed from: `5e94563fa1209f5045c7e91d70153e1c2a740b38` (`release: publish each installer's package manifest, and generate the site manifest (#525)`, 2026-11-15).

Every PR below was merged into `main` by pinned squash merge only after **100% of exact-head CI checks passed** (Rust on Windows/macOS/Linux, Clippy `-D warnings`, `cargo-deny`, Tauri dependency policy, multi-platform Tauri packaging & install qualification, Windows/Linux/macOS desktop E2E, 5-target CLI release qualification, website verification, and synthetic first-run UI):

| PR | Merged `main` Commit | Exact-Head CI | Summary |
| --- | --- | --- | --- |
| [#526](https://github.com/AbdulazizShehri/MedScale/pull/526) | `95c2f370` | **24/24 PASS** | 64 MiB Argon2id default (`t=3, p=1`), strict KDF parameter bounds, transparent legacy 19 MiB → 64 MiB header rewrap on passphrase unlock; full secret zeroization (`Zeroizing`/`ZeroizeOnDrop` + IPC/React cleanup); cross-platform crash-dump hardening (`SetErrorMode` + `WerSetFlags`, `RLIMIT_CORE=0`, `PR_SET_DUMPABLE=0`); dependency updates (`url 2.5.9` / `RUSTSEC-2026-0592`, `cookie 0.18.3` / `RUSTSEC-2026-0594`); `docs/handoff/ANTIGRAVITY_RECOVERY.md`. |
| [#528](https://github.com/AbdulazizShehri/MedScale/pull/528) | `b4076aed` | **24/24 PASS** | Closes `#459`: confirmed in-app **Remove saved model…** recovery (`audio_model_remove` behind 5-model allowlist); Rust-side 15-minute encrypted-vault idle auto-lock (`State::enforce_idle_lock`); `#457` `beforeunload` warning & updated Settings guidance on shown-once recovery-code screen; transcript OS-clipboard exposure disclosure. |
| [#530](https://github.com/AbdulazizShehri/MedScale/pull/530) | `2ab3ceeb` | **9/9 PASS** | Added `scripts/e2e/resolve-edge-webdriver.ps1` wired into `desktop-e2e.yml` and `audio-real-desktop-e2e.yml` to resolve a matching `msedgedriver` whenever GitHub `windows-latest` runners have Edge WebDriver / WebView2 major-version skew (e.g., 158 vs 157). |
| [#529](https://github.com/AbdulazizShehri/MedScale/pull/529) | `17cb099d` | **24/24 PASS** | Safe model acquisition destination & staging guards (refuses overwriting non-empty user folders or unmarked `.partial` directories); defensive read-only external SQLite (`SQLITE_DBCONFIG_DEFENSIVE`, `PRAGMA trusted_schema = OFF`, `PRAGMA query_only = ON`); reserved vault path bypass fix (`./` and Windows trailing dots/spaces); model config bounds & `catch_unwind` panic isolation in `medscale-pack`. |
| [#527](https://github.com/AbdulazizShehri/MedScale/pull/527) | `10818e03` | **23/23 PASS** | Keyed HMAC-SHA256 sealed-blob filenames (`k1_<hmac>.sealed` under vault DEK with legacy `<digest>.sealed` fallback read); Browse URL path PII/PHI scanning (handling percent-encoded and colon/underscore REST paths while preserving unlabelled public PubMed/PMC IDs); fixed path-free storage error messages plus `sanitize_error_message` defense-in-depth in desktop `CmdError`. |
| [#531](https://github.com/AbdulazizShehri/MedScale/pull/531) | `PR_531_SHA` | **23/23 PASS** | Sealed desktop pack/ASR helper records inside encrypted vaults (`vault_helper_records` additive table in `meta.sealed`) with automatic legacy `.desktop-packs.json`/`.desktop-asr.json` migration and deletion, wired into `#459` model removal; hard WebView2 disk-residue assertions in `scripts/e2e/installed-journey.mjs`. |

---

## 3. Privacy & Security Readiness Ledger (`REAL_DATA_READINESS.md`)

All engineering blockers (**B1–B5**) and all technical items under "Also needed" are implemented and tested on `main`:

| Item | Status | Evidence |
| --- | --- | --- |
| **B1** Encrypted-only FHIR import & sealed patient list | **Done** (`#492`, `#524`) | Sample workspace refuses all user imports; encrypted vault keeps resources and `roster_from_core` sealed. |
| **B2** Real FHIR decimals, Bundle splitting, streaming, exact citations | **Done for import** (`#516`–`#522`) | Tested against 5 public HL7 US Core & IPS fixtures (`fixtures/public/hl7`); vendor-specific portal exports not yet tested. |
| **B3** Idle auto-lock, passphrase change, recovery-code replacement | **Done** (`#512`, `#528`) | Enforced in both UI and Rust host (`State::enforce_idle_lock`); CLI + Settings replacement; `#457` close-guard. |
| **B4** Rollback generation counter & hash-chained audit | **Done (with documented local-disk limits)** | Sealed generation counter + device-local check; transactional hash-chained `audit_chain` + `medscale vault verify-audit`. |
| **B5** Verified backup & restore | **Done** (`#485`) | Desktop + CLI verified backup/restore keeping replaced vault; tested across storage, Core, IPC, CLI. |
| **B6** Legal notices, terms, intended use | **Drafts ready for counsel** (`#523`) | `docs/legal/drafts/` — not in effect; requires legal counsel and founder approval. |
| 64 MiB Argon2id + transparent rewrap | **Done** (`#526`) | `medscale-keys` + `medscale-storage` tests (16/16 + full workspace CI). |
| Secret zeroization in memory | **Done** (`#526`) | `Zeroizing`/`ZeroizeOnDrop` across key derivation, recovery codes, IPC, and React state. |
| Cross-platform crash-dump hardening | **Done** (`#526`) | `medscale_contracts::process_hardening::disable_crash_dumps()` invoked at Desktop & CLI startup. |
| Keyed sealed-blob filenames | **Done** (`#527`) | `k1_<hmac>.sealed` + legacy fallback; `sealed_blob` unit tests + `encrypted_vault_durability_109` (11/11). |
| Fixed path-free error text | **Done** (`#527`) | Core/storage fixed errors + desktop `sanitize_error_message` regression tests. |
| Browse URL path PII scanning | **Done** (`#527`) | `authority::browse::tests` (3/3) + `browse_080` (7/7); desktop uses offline Browse fixture. |
| Sealed desktop helper files | **Done** (`#531`) | `vault_helper_records` in `meta.sealed`; zero plaintext `.desktop-packs.json`/`.desktop-asr.json` beside encrypted vaults. |
| Safe model destination, defensive SQLite, panic isolation | **Done** (`#529`) | `model_acquisition` (18/18), `locator_guard_tests`, `medscale-pack` (111/111). |
| BitLocker / FileVault policy | **Founder decision** | Policy choice whether to detect or require OS full-disk encryption. |

---

## 4. Platform Capability & Qualification Matrix

| Platform / Surface | Engineering Status | Qualification Evidence | Documented User Limitations |
| --- | --- | --- | --- |
| **Windows x64 installer** (`MedScale_0.1.0_x64-setup.exe`) | **Qualified (unsigned Beta)** | 20-step installed journey (`run 43247164670`), Tauri preview CI on every PR, ClamAV 0 infected, provenance attested | Unsigned: SmartScreen warns; Windows 11 Smart App Control blocks unsigned apps. |
| **Windows x64 portable zip** (`MedScale_0.1.0_x64-portable.zip`) | **Qualified (unsigned Beta)** | Extracted and launched on clean CI runner; ClamAV 0 infected; provenance attested | Same unsigned Windows warnings. |
| **macOS Apple Silicon DMG** (`MedScale_0.1.0_aarch64.dmg`) | **Qualified (ad-hoc Beta)** | Real DMG install, launch, removal (`run 43247164670`); desktop E2E on macOS | Ad-hoc signed, not notarized: System Settings → Privacy & Security → Open Anyway once. |
| **macOS Intel DMG** (`MedScale_0.1.0_x64.dmg`) | **Qualified (ad-hoc Beta)** | Built on Intel runner; real DMG install, launch, removal (`run 43247164670`) | Same ad-hoc signed / Gatekeeper prompt. |
| **Linux x64 `.deb` & AppImage** | **Qualified (unsigned Beta)** | `.deb` install, launch, reinstall-keeping-data, removal; AppImage launched; Linux desktop E2E | Unsigned; requires WebKitGTK 4.1 (Ubuntu 22.04+). |
| **Linux arm64 `.deb` & AppImage** | **Qualified (unsigned Beta)** | Same on ARM64 runner (`run 43247164670`) + Tauri preview CI | Requires ARMv8.2-A CPU with FP16. |
| **CLI (5 targets: Win x64, macOS arm64/x64, Linux x64/arm64)** | **Qualified (unsigned Beta)** | Binary tests + clean-PATH execution (`x86_64-apple-darwin` smoke-run under Rosetta 2); local Windows CLI 40 unit + 4 binary tests passed | Unsigned binary; identical Core authority and encrypted-vault lifecycle as Desktop. |
| **Website (`medscale.si`, `web/`)** | **Verified & gated** | `npm test` (26/26), `check-proof.mjs` (PASS), `verify-release` (PASS: empty manifest, zero broken links) | Download links stay inactive until founder publishes release & posts `APPROVE` + `SIGNING` comments. |

---

## 5. Staged `0.1.0-beta.1` Release Candidate & Founder Publication Package

**Staged candidate artifact (verified unexpired):**
- Artifact: `medscale-release-candidate-0.1.0-beta.1-reassembled` (Artifact ID `14153860834`, 471,059,001 bytes)
- Re-assembly run: [43693295275](https://github.com/AbdulazizShehri/MedScale/actions/runs/43693295275) (from source run [43247164670](https://github.com/AbdulazizShehri/MedScale/actions/runs/43247164670), commit `6d0279e93fd011b719652a14f6cfa74dc82fad1c`)
- Verification: `sha256sum -c SHA256SUMS` **34/34 OK**; ClamAV **34 scanned, 0 infected**; `actions/attest-build-provenance` **34 subjects attested**
- Expiration: `2027-02-13T08:10:59Z`

### Founder's Two Clean Options

1. **Option A — Publish the already-staged `0.1.0-beta.1` candidate (`6d0279e9`) immediately:**
   - Create immutable GitHub Release `v0.1.0-beta.1` at `6d0279e93fd011b719652a14f6cfa74dc82fad1c` uploading all 34 files + `SHA256SUMS` from artifact `14153860834`.
   - Post the founder approval comment containing the 13 `APPROVE <sha256>` lines recorded verbatim in [RELEASE_CANDIDATE_0.1.0-beta.1.md](../release/RELEASE_CANDIDATE_0.1.0-beta.1.md) plus `SIGNING <key> <state>` lines per platform.
   - Run `web/scripts/manifest-from-candidate.mjs` and verify `npm run verify-release` passes against the published bytes before deploying `medscale.si`.
2. **Option B — Dispatch a refreshed `release-candidate` workflow on current `main`:**
   - Recommended if the founder wants the first public Beta installers to include all post-`6d0279e9` improvements (B1–B5, 64 MiB Argon2id, secret zeroization, crash-dump hardening, keyed sealed-blob filenames, Browse URL path PII scan, fixed errors, Rust idle auto-lock, `#459` saved-model removal, `#457` recovery-code guard, safe model destination / SQLite / runtime panic guards, and sealed desktop helper records).
   - After `release-candidate` completes, review the freshly generated inventory and SHA-256 table before creating the release and posting the new `APPROVE` lines.

---

## 6. Reproducible Local Build & Verification Results

Verified locally on Windows 11 (Rust `1.100.0`, Node `v24.19.0`, Python `3.14.7`, Strawberry Perl + NASM, VS 2022 Build Tools; shared `CARGO_TARGET_DIR`):

- `cargo test -p medscale-contracts -p medscale-keys -p medscale-clinical -p medscale-fhir -p medscale-network --locked`: **PASS** (`medscale-contracts` 156/156, `medscale-keys` 16/16, `medscale-clinical` 11/11, `medscale-fhir` & `medscale-network` 100%)
- `cargo test -p medscale-pack --lib --locked`: **PASS** (111 passed, 0 failed, 1 ignored)
- `cargo test -p medscale-storage --locked`: **PASS** (51 lib + 11 `encrypted_vault_durability_109` + 6 `encrypted_vault_005` + 7 `vault_backup_restore`)
- `cargo test -p medscale-core --lib model_acquisition|data_sources|browse --locked` + `--test browse_080`: **PASS** (18 + 1 + 3 + 7 = 29/29)
- `cargo test -p medscale-cli --locked`: **PASS** (40/40 unit + 4/4 binary tests)
- `cargo clippy ... --all-targets --locked -- -D warnings`: **PASS** (zero warnings)
- `apps/desktop-tauri`: `npm test` — **85/85 PASS**
- `web`: `npm test` — **26/26 PASS**; `node scripts/check-proof.mjs` — **PASS**; `npm run verify-release` — **PASS**
- `python scripts/check_public_repository_hygiene.py`: **PASS**
- `pwsh -NoProfile -File scripts/check-dependency-direction.ps1`: **PASS**

---

## 7. GitHub Issue Audit Summary

- **Closed:** `#459` (closed by PR `#528`, confirmed in-app **Remove saved model…** recovery).
- **Updated with verified evidence on `main`:**
  - `#457` (recovery-code regeneration in `#512` + `beforeunload` warning and updated copy in `#528`).
  - `#458` (resolved in `#463` `f01323c9` with `only_unfinished_vault_files` / `recover_unfinished_vault` + `feature_tests.rs`).
  - `#460` (4 consecutive Windows installed journeys since 2026-11-07 logged `INPRIVATE true` across all 5 launches, `PREF_ON_DISK 0`, `WEBVIEW_RESIDUE 0`; promoted to hard assertion in `#531`; `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` stays `OPEN` pending independent review).
- **Pre-existing local workspaces & research assets:** Preserved completely untouched (including the separate biomedical ML research manuscript worktree, private checkpoint worktree marked `Private checkpoint. Do not push`, and historical QA/repair folders). Confidential review files remain strictly out of the public repository.

---

## 8. Remaining Founder-Only & External Decisions

Engineering has no remaining implementation blockers for the `0.1.0-beta.1` public engineering Beta. Only founder-only and external-authority decisions remain:

1. **Public Beta release decision:** Choose Option A (publish staged `6d0279e9` candidate) or Option B (dispatch refreshed `release-candidate` on current `main`), create the immutable GitHub Release, post `APPROVE` + `SIGNING` comments, and approve `medscale.si` production deployment.
2. **Code signing & distribution route (`#507` / Draft PR `#508`):** Decide timing of Microsoft Store MSIX submission (Partner Center) and SignPath Foundation application (once first public release exists) or paid Windows/Apple Developer ID certificates.
3. **Independent human security review:** Commission an external security assessment (required to close `TAURI_WEBVIEW_PRIVACY_QUALIFICATION` and `PRIVATE_DATA_READY`).
4. **Legal & regulatory review (B6):** Have qualified counsel review `docs/legal/drafts/` (FTC Health Breach Notification Rule, GDPR Art. 9, Saudi PDPL/SFDA, Washington MHMDA, EU MDR Rule 11 / FDA CDS, and SNOMED/LOINC/ICD terminology licensing).
5. **Disk encryption policy & real-PHI authorization:** Decide BitLocker/FileVault check policy and whether to ever authorize real patient data in writing (`REAL_PHI_AUTHORIZATION` remains `NOT_AUTHORIZED` until all gates close).

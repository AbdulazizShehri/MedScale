# Antigravity Recovery Handoff — MedScale

**Recovery timestamp:** 2026-11-26  
**Canonical repository:** <https://github.com/AbdulazizShehri/MedScale>  
**Active engineering checkout:** `D:\MedScale` (clean clone of `origin/main`, preserving all prior local worktrees untouched)  
**Gates unchanged:** `RELEASE_READY=false` · `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED` · Clinical use: not authorized.

---

## 1. Canonical GitHub HEAD and Date

- **Default branch:** `main`
- **Commit SHA:** `5e94563fa1209f5045c7e91d70153e1c2a740b38`
- **Commit date:** `2026-11-15T08:10:39Z`
- **Subject:** `release: publish each installer's package manifest, and generate the site manifest (#525)`
- **Verified recent merges into `main`:**
  - `#525` (`5e94563f`, 2026-11-15): per-installer `package-manifest.json`, `SBOM.cdx.json`, `NOTICE.md`, `release-candidate-reassemble` workflow, and `web/scripts/manifest-from-candidate.mjs`.
  - `#524` (`4449035a`, 2026-11-15): B1 — FHIR import goes only into an encrypted vault.
  - `#523` (`732035ce`, 2026-11-14): B6 — legal review drafts in `docs/legal/drafts/` (drafts only, not approved legal documents).
  - `#516`, `#519`, `#520`, `#521`, `#522` (2026-11-13 to 2026-11-14): B2 — exact FHIR decimals, 8 MiB single-resource limit, Bundle splitting (up to 2,000 entries), exact member citations, streaming large Bundles (up to 64 MiB) & NDJSON, and public HL7 US Core / IPS example tests.
  - `#512`–`#514` (2026-11-13): B3 (passphrase change + recovery-code replacement) and B4 (anti-rollback generation counter + tamper-evident audit hash chain).
  - `#511` (`c629e6fc`, 2026-11-13): `0.1.0-beta.1` release candidate report.
  - `#485`–`#506` (2026-11-10 to 2026-11-12): B5 backup/restore, CLI encrypted-vault parity, Research Canvas desktop UI, MedAgent refusals, Linux desktop build with vendored `glib-0.18.5`, multi-platform packaging (Intel DMG, Linux arm64, AppImage, Windows portable zip), and zero-cost distribution policy.

---

## 2. Local Repository Branches and Worktrees

All pre-existing local workspaces were inspected and left **untouched**:

| Location | Branch / HEAD | Dirty / Untracked State | Disposition |
|---|---|---|---|
| `D:\MedScale` | `main` @ `5e94563f` (canonical working clone) | Clean baseline; feature branches created from `main` | Active Antigravity workspace (`CARGO_TARGET_DIR` on `D:`) |
| `~/Documents/MedScale-paper-research` | `research/medscale-paper` @ `1f3c3102` | Untracked `research/medscale-paper/` (manuscript, experiments, figures, manifests, results) | Preserved untouched; separate biomedical ML research manuscript (*Same Weights, Different Numbers*) |
| `~/Documents/MedScale-paper-research/.claude/worktrees/fix-privacy-gate-token-markers-63f9e7` | `claude/fix-privacy-gate-token-markers-63f9e7` @ `c326e87e` (6 commits ahead of `f01323c9`) | 6 modified uncommitted files (`r_workspace.rs`, `medscale-contracts/src/privacy_gate.rs`, `medscale-core/src/authority/privacy_gate.rs`, `privacy_recognizers.rs`, `cli_session.rs`, `privacy_gate_079.rs`; +371/-27 lines) | **Unmerged work — preserved untouched and scheduled for clean review/integration into `main`** |
| `~/Documents/MedScale-dependency-repair` | Broken `.git` gitfile pointing to missing `MedScale-site-audit` worktree | Invalid gitfile pointer | Preserved untouched (never deleted) |
| `~/Documents/MedScale-depfix-independent` | `feat/website-free-local-privacy-20261023` @ `234ff83` | 35 untracked `web/` QA logs/screenshots from 2026-10-23 | Preserved untouched |
| `~/Documents/MedScale-first-run-qualification` | `feat/beta-first-run-synthetic-20261023` @ `abd8cde` | Clean (historical Oct 23 branch) | Preserved untouched |
| `D:\MedScale-Beta-Review` | Non-git directory of historical QA folders (`20261024`–`20261107`) | Read-only historical artifacts | Preserved untouched |

**Storage status at recovery:** `C:` 12.31 GB free / `D:` 20.36 GB free. All new builds and Cargo target caches are directed to `D:` to avoid exhausting `C:`.

---

## 3. Latest Claude Product-Development Checkpoint

- **Actual product-development session located:**  
  `~/.claude/projects/...scratch-2026-11-01-aac634/ca253879-351b-4d51-99ea-b5297d7b2a24.jsonl` (54.5 MB, last product action `2026-11-15T08:17:37Z`, stopped by user at `2026-11-15T09:01:15Z`, `/goal clear` at `09:03:34Z`).
- **Separate research session confirmed distinct:**  
  `440aca02-a61d-4913-b4fa-e45603223750.jsonl` (`MedScale-paper-research`) ended on `2026-11-08T23:29Z` reviewing the manuscript *Same Weights, Different Numbers* and did not reflect product development.
- **Exact last product action performed by Claude (2026-11-15):**
  1. Merged PR `#525` (`5e94563f`) after exact-head CI passed.
  2. Triggered `release-candidate-reassemble` ([run `43693295275`](https://github.com/AbdulazizShehri/MedScale/actions/runs/43693295275)) reusing the existing `0.1.0-beta.1` candidate artifacts from `6d0279e9` (run `43247164670`) without recompiling, adding per-installer `package-manifest.json`, `SBOM.cdx.json`, and `NOTICE.md` (34 files + `SHA256SUMS`), passing ClamAV (0 infected), checksum verification, and GitHub build-provenance attestation.
  3. Verified locally that all 13 `APPROVE` SHA-256 hashes matched unchanged and that `web/scripts/manifest-from-candidate.mjs` generated a valid `release-manifest.json` against the real 34-file candidate set.
  4. Paused waiting for founder-only decisions (publishing `v0.1.0-beta.1` and posting founder `APPROVE` / `SIGNING` comments, B6 legal counsel review, and `REAL_PHI_AUTHORIZATION`).
- **Temporary checkout note:** Claude's ephemeral working directory (`%TEMP%\ms-audit` / `%TEMP%\msp`) is no longer present; all merged work is safely on `origin/main@5e94563f`.

---

## 4. Uncommitted and Untracked Work Requiring Preservation

1. **`~/Documents/MedScale-paper-research/research/medscale-paper/`**: Complete untracked biomedical ML manuscript, experiments, figures, manifests, and raw results. **Must remain separate from product commits and untouched.**
2. **`~/Documents/MedScale-paper-research/.claude/worktrees/fix-privacy-gate-token-markers-63f9e7`**: Contains 6 unmerged local commits (`86cfaa6c`..`c326e87e`, dated 2026-11-07) plus 6 uncommitted modified files hardening `privacy_gate`, `privacy_recognizers`, `r_workspace`, and `cli_session` (single egress authorization boundary, disabling unsafe declassification, gating R workspace staging). Its companion Claude session (`5a14dc45-7f90-451f-9533-fce7b4e6be1b.jsonl`) stopped mid-qualification on 2026-11-07. We preserve the original worktree intact and review/port its verified fixes cleanly onto `main`.
3. **Historical QA logs and screenshots** in `MedScale-depfix-independent/web/` and `D:\MedScale-Beta-Review\`: Preserved untouched.

---

## 5. Merged, Open, and Draft PRs

- **Latest merged PRs:** `#525` (`5e94563f`), `#524` (`4449035a`), `#523` (`732035ce`), `#522`–`#516`, `#515`–`#511`, `#510`–`#484`.
- **Open Draft PRs (8):**
  - `#508`: `feat(release): qualify a zero-cost Microsoft Store MSIX packaging route (no publish)` (`feature/windows-msix-store-ci-20261112`)
  - `#439`: `docs(marketing): professional Hugging Face product Space and organization card`
  - `#412`: `draft(future-health): synthetic care-loop evidence viewer and 22 fail-closed tests`
  - `#409`: `docs(funding): prepare source-backed sponsorship brief (no funding activation)`
  - `#407` / `#405`: historical `fix(synthetic): exclusive atomic sidecar temporary files` drafts (2026-11-02)
  - `#401`: `docs(funding): research cash grants and prepare MedScale applications`
  - `#172`: `Start MedScale arXiv paper and provenance evaluation harness`
- **Open Non-Draft PRs (6 — all older 2026-10-23 branches superseded or needing review before any action):**
  - `#290`, `#289`, `#288`, `#287`, `#285`, `#284` (from 2026-10-23, prior to the Nov 2026 website/release redesign).
- **Open Issues (13):** `#507`, `#460`, `#459`, `#458`, `#457`, `#436`, `#386`, `#317`, `#314`, `#307`, `#302`, `#183`, `#182`.

---

## 6. Current Test and CI Status

- **GitHub Actions on `main@5e94563f` (all green):**
  - `ci` (`43693275908`): **success**
  - `tauri-preview` (`43693275915`): **success**
  - `desktop-e2e` (`43693275932`): **success**
  - `MedScale Website (Next.js)` (`43693275958`): **success**
  - `release-candidate-reassemble` (`43693295275`): **success**
- **Local toolchain verified:** `rustc 1.100.0`, `cargo 1.100.0`, `cargo-deny`, `node v24.19.0`, `pnpm 9.12.0`, `python 3.14.7`, Strawberry Perl, NASM, Visual Studio 2022 Build Tools.
- **Local baseline checks passed:**
  - `cargo test -p medscale-keys`: 12/12 passed
  - `web/npm test`: 26/26 passed
  - `python scripts/check_public_repository_hygiene.py`: PASS

---

## 7. Latest Known Release State

- **Published GitHub Releases:** Only historical pre-release `engineering-preview-19d419dd7b15` (2026-10-23, `MedScale Beta 0.1.0 - unsigned synthetic-only`).
- **Staged release candidate (`0.1.0-beta.1`, NOT published):**
  - Built from `6d0279e93fd011b719652a14f6cfa74dc82fad1c` (run `43247164670`, 2026-11-12) and re-assembled in run `43693295275` (2026-11-15).
  - Artifact `medscale-release-candidate-0.1.0-beta.1-reassembled` (471,059,001 bytes) is **unexpired** (`expires_at: 2027-02-13T08:10:59Z`).
  - Website `QUALIFIED_RELEASE_TAGS` remains intentionally empty until the founder publishes `v0.1.0-beta.1` and posts the `APPROVE` / `SIGNING` comments from the founder account.
  - Note: commits `#512`–`#525` (B1/B2/B3/B4/B6 drafts) landed on `main` *after* `6d0279e9`, so a fresh release-candidate build can also be produced once the remaining engineering hardening below is merged.

---

## 8. Remaining Blockers and Dependency Order

### Engineering work we can execute autonomously now (ordered by priority & dependency)

1. **Vault Crypto, Key Strength, Zeroization & Crash-Dump Hardening (`REAL_DATA_READINESS.md`):**
   - Raise Argon2id default memory from 19 MiB (`19_456` KiB) to 64 MiB (`65_536` KiB) for new vaults; validate/bound KDF header parameters; automatically rewrap legacy 19 MiB vaults on unlock.
   - Zeroize remaining copies of passphrase-derived keys, recovery keys, and passphrases across `medscale-keys`, `medscale-storage`, `medscale-core`, and desktop IPC; prevent secret leakage in `Debug` impls.
   - Disable Windows crash-dump memory capture for the MedScale desktop/CLI process (WER opt-out / `WerSetFlags` + `SetErrorMode`).
2. **Storage, Error Sanitization, Browse Privacy & Plaintext Helper Elimination (`REAL_DATA_READINESS.md`):**
   - Keyed sealed-blob filenames (HMAC/keyed SHA-256 derived from vault key for new blobs, backward-compatible reads for existing vaults).
   - Fixed/sanitized user-facing error messages at desktop IPC/storage boundaries so raw paths and OS usernames cannot leak.
   - Browse URL **path** PII/PHI scanning (in addition to query scanning) and encrypted/real-data vault Browse guardrails.
   - Move unencrypted helper files beside encrypted vaults (seed manifest, pack/model sidecar metadata) inside the sealed vault or remove plaintext residue.
3. **Privacy Gate & Core Security Hardening:**
   - Review, test, and port the unmerged privacy-gate hardening from `claude/fix-privacy-gate-token-markers-63f9e7` (single egress boundary, federation/adapter enforcement, R workspace staging guard).
   - Enforce Rust-side idle auto-lock enforcement (complementing the WebView timer), safe model acquisition staging, Core-derived `synthetic_only` checks, model config bounds / panic isolation, SQLite `trusted_schema=OFF` / defensive mode for external DBs, and mapped-drive/UNC guards.
4. **Open Desktop Usability & Qualification Issues (`#459`, `#460`, `#457`, `#458`, UX):**
   - `#459`: Add one-click remove / re-download recovery for a saved Whisper model that fails its pin check (Core + Tauri IPC + Audio/Models UI + tests).
   - `#460`: Investigate and fix the InPrivate window-title / profile consistency across first and subsequent launches.
   - `#457` / `#458`: Verify `#463` and `#512` coverage, add close-confirmation / recovery-code safeguard on the Gate screen, and update issue trackers with verified evidence.
   - Add clipboard warning when copying transcripts and verify desktop/CLI journeys and accessibility.
5. **Documentation, Website, Release Candidate & Qualification Parity:**
   - Reconcile `KNOWN_ISSUES.md`, `REAL_DATA_READINESS.md`, `RELEASE_DECISION_REPORT.md`, `RELEASE_CANDIDATE_0.1.0-beta.1.md`, `EXTERNAL_GATES.md`, and `web/` with exact verified state.
   - Run full Rust, Tauri, CLI, and Web test suites and verify exact-head GitHub Actions CI before merging each focused PR.
   - Produce `docs/handoff/MEDSCALE_FINAL_ENGINEERING_STATUS.md`.

### External / Founder-Only Decisions (never performed by AI)

- Publishing GitHub Release `v0.1.0-beta.1` (immutable tag) and posting founder `APPROVE` / `SIGNING` comments from the founder account.
- Activating website download links (`medscale.si` production deploy approval).
- Commissioning independent human security review and legal/regulatory counsel review of `docs/legal/drafts/` (B6).
- Any written authorization changing `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED` or clinical-use status.
- Optional SignPath Foundation application / Microsoft Store publisher registration / paid signing credentials.

---

## 9. Verified Next Implementation Action

Immediately proceed to **Step 2 (Vault Crypto, Zeroization & Crash-Dump Hardening)**:
1. Inspect `crates/medscale-keys/src/provider.rs`, `recovery.rs`, `aead_wrap.rs`, `crates/medscale-storage/src/encrypted_vault.rs`, and `apps/desktop-tauri/src-tauri/src/main.rs`.
2. Implement 64 MiB Argon2id default + KDF parameter bounds + transparent rewrap on unlock, complete secret zeroization (`Zeroizing`), and Windows crash-dump memory protection.
3. Add focused unit & integration tests, run `cargo test` and `cargo clippy`, push PR, and verify exact-head CI.

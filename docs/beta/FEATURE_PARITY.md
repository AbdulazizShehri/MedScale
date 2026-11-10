# MedScale feature parity and release scope

**Snapshot:** `main@0be749f` (2026-11-10). This is a read-only audit by AI agents, with the most serious claims re-checked in source. It is the work list for release readiness: each row is completed, given a reviewed disposition, or kept as an explicit blocker. The scope below is **frozen**. New features need a separate decision.

Levels: **FULL** = UI → Tauri command → Core → visible result → audit record. **CORE+CLI** = no desktop path. **CORE** = no named CLI or desktop path. **Native E2E** names the test that drives it on an installed build (IJ = `installed-journey.mjs`, LC = `windows-lifecycle.mjs`, NV = `nvda-journey.mjs`, DJ = `desktop-journey.mjs` on a WebDriver test build, DMG = `installed-dmg-macos.sh`).

## 1. Feature matrix

| Area | Feature | Core | Desktop IPC | Desktop UI | CLI | Native E2E | Win | macOS | Linux |
|---|---|---|---|---|---|---|---|---|---|
| Patients | List, timeline, brief, coverage | yes | yes | Patients | **broken: reads with no vault (C2)** | IJ view; DJ | view | none | none |
| Patients | Search | yes | yes | Patients | – | none | – | – | – |
| Documents | FHIR import | sample vault only | yes | Documents | `ingest` | IJ, DJ | yes | none | none |
| Documents | FHIR import into an encrypted vault | **refused (B1)** | – | – | – | – | – | – | – |
| Labs | Lab values in the brief | in brief | yes | Patients | – | none | – | – | – |
| Analytics | Query, receipts | yes | yes | Analytics | yes | DJ only | test build | none | none |
| Insights | Ask (retrieval in Core, answer assembled in the desktop) | partial | yes | Insights | – | none | – | – | – |
| Evidence | Lexical search | yes | yes | Evidence | – | none | – | – | – |
| Evidence | Corpus view | **fixture, bypasses Core** | yes | Evidence, Home | – | none | – | – | – |
| Knowledge | Build index, search | yes | yes | Knowledge | yes | DJ only | test build | none | none |
| Research Canvas | Create, edit, cite, show; overlap and coverage inspections | yes | **list only** | **titles only** | yes | none | – | – | – |
| Models | Admit, list, evaluate, catalog | yes | yes | Models | yes | DJ (fixture) | test build | none | none |
| Models | Real model download and verify (Whisper) | yes | yes | Audio | `packs acquire` | IJ | yes | test build | CLI (qual) |
| Model fleet | Lanes, dispatch, execute, compare | yes | yes | Model Fleet | yes | DJ (lanes only) | partial | none | none |
| MedAgent | Run lifecycle, execute, evidence gate | yes | yes | MedAgent | yes | DJ | test build | none | none |
| MedAgent | Refusals (tool refusals, empty-evidence refusal, run fail) | yes | **no** | **raw/generic** | yes | none | – | – | – |
| Audio | Import, real Whisper transcription, correct, retention | yes | yes | Audio | yes | IJ (real speech) | yes | test build | none |
| Workflows | Outbox, tasks, messages (read-only by design) | yes | yes | Workflows | `actions outbox` | none | – | – | – |
| Collaboration | Rooms, threads, messages, tasks | yes | yes | Collaboration | yes | none | – | – | – |
| Privacy | Egress check, classifications, receipts | yes | yes | Privacy | yes | none | – | – | – |
| Privacy | Classify, profiles, transform, re-identify | yes | **no** | **no** | yes | none | – | – | – |
| Audit | Disclosure trail | yes | yes | Audit Trail | yes | DJ | test build | none | none |
| Vaults | Create, unlock, lock, recovery codes, crash recovery | yes | yes | Gate, Settings | **recovery codes discarded (C3)** | IJ, LC | yes | **none** | none |
| Vaults | Data commands on an encrypted vault | yes | yes | yes | **no: sample vault only (C1)** | – | – | – | – |
| Vaults | Backup and restore | yes | **no** | **no** | **no** (only inside `journey run`) | none | – | – | – |
| Data | Sample data source, snapshots, transform | yes | yes | Data | yes | DJ | test build | none | none |
| Data | User-chosen dataset import | yes | **no** | **no** | yes | none | – | – | – |
| Browse | Allowlist, fetch | yes | yes | Browse | yes | DJ (default deny) | test build | none | none |
| Exports | FHIR export with disclosure | yes | yes | Exports | **no export command** | DJ | test build | none | none |
| Help | Help & Feedback, update check | n/a | yes | Help | – | none | – | – | – |
| Install | Install, uninstall keeping data, reinstall, upgrade | n/a | n/a | n/a | portable zip | IJ, LC | yes | **launch only** | none |
| A11y | Keyboard, themes, screen reader, window sizes | n/a | n/a | all | n/a | NV, LC (sizes measured only) | yes | keyboard (Audio page) | none |

## 2. Corrections to existing documents

- `RELEASE_PACKET.md` says uninstall keeps user data "verified on Windows and macOS". On macOS nothing is created before removal and nothing is checked after. Windows only, until a macOS journey covers it.
- `QA_MATRIX.md` row 14 lists `audio-transcript-reopens-after-leaving-the-page` for macOS; that journey exists only in the Windows `desktop-journey.mjs`.
- The About page says "MedScale does not open the browser for you" (`Governance.tsx`), but Help & Feedback opens it.
- Installed-journey steps `layout-measured-at-window-sizes`, `idle-network-connections-measured` and `webview-residue-measured` record values but cannot fail.

## 3. Platform state

| Platform | State | Blocker |
|---|---|---|
| Windows desktop | Broad native coverage on the installed build; analytics, knowledge, MedAgent, export and audit run only on a test build | Installed journeys run only on `test/*` pushes pinned to older commits |
| macOS desktop | Install and launch only; GUI journeys only on WebDriver test builds; no VoiceOver | Full DMG journey; Developer ID signing (founder credentials) |
| Linux desktop | No build | RUSTSEC-2024-0429 (unsound `glib` 0.18.5 `VariantStrIter`), pulled in by Tauri 2.12 on Linux. No stable Tauri on fixed glib yet. Plan: vendor glib 0.18.5 with the one-line upstream fix (same pattern as `vendor/candle-core-0.9.1`), keeping `unsound = "all"` and no ignore |
| CLI | 40 commands, about 250 subcommands; no release builds, no install docs, no binary tests | C1–C3 below |

## 4. Frozen scope and dispositions

**Must complete for the release candidate**

| # | Item | Why |
|---|---|---|
| R1 | Backup and restore: Tauri command, Settings UI, CLI command | Real users must be able to protect their data; the UI already refers to backups |
| R2 | CLI C1–C3: data commands open the chosen (encrypted) vault; reads require a vault; `vault create` shows the recovery codes once; recovery-code open | The CLI gives wrong or empty results and loses recovery codes |
| R3 | Research Canvas in the desktop: create, cite a hit, view with overlap and coverage | Surfaces the merged clinical checks (#480, #483) |
| R4 | MedAgent refusal states in the desktop | The evidence gate (#482) refuses runs; users must see why |
| R5 | FHIR import into encrypted vaults (B1) | The real workspace can't ingest records |
| R6 | Evidence corpus: label it honestly as the bundled synthetic corpus | It is presented as vault content but bypasses Core |
| R7 | CLI FHIR export, backup, verify commands; meaningful exit codes; binary end-to-end tests | CLI product readiness |
| R8 | CLI release workflow (Windows x64, macOS arm64/x64, Linux x64/arm64) with SHA-256 and SBOM | No release path exists |
| R9 | Linux desktop: vendored glib fix, Linux CI build (deb, rpm, AppImage), native smoke | Required target |
| R10 | Native E2E on installed builds for analytics, knowledge, Research Canvas, MedAgent, export, audit, backup; macOS full DMG journey; Linux journey | Release acceptance |
| R11 | Corrections in section 2 | Honesty of release documents |

**Reviewed dispositions (CLI or Core only, not in the desktop for this release)**

| Feature | Disposition | Reason |
|---|---|---|
| Privacy Gate transforms, profiles, re-identification | CLI only | Expert workflows with reversible pseudonym maps; desktop exposure needs its own privacy review |
| Collaboration approvals, notes, membership, activity | CLI only | Multi-user features; MedScale is single-user locally |
| Research OS actions (hub, federation, R, compute submit, packs, huddles) | CLI only; desktop keeps its status board | Operator and institutional features |
| Workflow effect transitions, NPHIES, network broker | Core only | Gated external actions; no external effects in this release |
| Projection rebuild, identity merge, amend, validator evidence, blob verify/GC | Core only | Maintenance and governance internals |
| General audit log reading | Not available | No read capability exists; adding one is a new feature, out of scope |
| Live audio capture | CLI only | Native capture isn't qualified in the desktop |

**Clinical checks (OpenMed-derived)**

| Check | Status |
|---|---|
| Evidence-span overlap | In Core (Canvas); desktop via R3 |
| Empty-evidence gate | In Core (MedAgent); desktop refusal display via R4 |
| Evidence coverage | In Core (Canvas); desktop via R3 |
| Numeric and unit consistency | Applicable (`presentation::build_coverage` compares values as strings); needs a unit-conversion port. Scheduled after R1–R5 |
| Claim conflicts, citation checks, recency, uncertainty, refusal taxonomy, output-use policy, review state machine, decision support | Not applicable or already covered: MedScale produces no input of that shape, or an equivalent control exists (see the call-site map in PR #482's investigation) |
| Lab reference ranges | Blocked: MedScale doesn't extract FHIR `referenceRange` |

## 5. Not decided here

Real-patient-data authorization, publishing, signing purchases and clinical claims remain founder decisions. `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`.

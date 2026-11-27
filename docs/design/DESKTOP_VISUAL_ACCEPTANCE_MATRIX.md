# MedScale Desktop — Visual & UX Acceptance Matrix

**Verification Basis:** Real native Windows Tauri release binary (`medscale-desktop-tauri.exe`, `--features custom-protocol --release --locked`) captured via Win32 `PrintWindow(PW_RENDERFULLCONTENT)` and audited via Windows UI Automation (`scripts/e2e/uia-driver.ps1`), combined with automated layout/contrast/identity/a11y checks (`npm test`, `npm run test:first-run`, `npm run test:visual`).  
**Note:** Synthetic sample workspace only. Real PHI and clinical use remain unauthorized (`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `CLINICAL_USE_AUTHORIZATION=NOT_AUTHORIZED`).

---

## 1. Automated Gate Summary

| Gate | Scope | Result | Evidence |
|---|---|---|---|
| Desktop unit tests (`npm test`) | 19 suites (contrast, identity, accessible rows, routes, IPC, bundle config, audio, seed failure, auto-lock) | **PASS (85/85)** | 0 failures |
| TypeScript & Vite build (`npm run build`) | `tsc --noEmit && vite build` | **PASS** | Clean production bundle, zero remote fonts |
| First-run smoke (`npm run test:first-run`) | 375px / 768px / 1280px Welcome → Sample → Palette focus/ARIA | **PASS (12/12)** | 0 exceptions, 0 external requests |
| Visual route layout (`npm run test:visual`) | 25 routes × Dark & Light × `1440×900` & `1100×720` (100 views) | **PASS (0 misses)** | 0 edge alignment misses, 0 button-height mismatches, 0 overflow |
| Native UIA unnamed-control audit | 45 real native Windows captures | **PASS (0 interactive unnamed)** | Fixed all 5 previously unnamed `ControlType.Group` items in Settings |
| Native UIA right-edge clipping audit | 45 real native Windows captures | **PASS (0 clipped)** | `"clipped": {}` across every route |
| Public repository hygiene | `scripts/check_public_repository_hygiene.py` | **PASS** | No internal plans or personal paths |

---

## 2. Screen-by-Screen Visual Acceptance Matrix

| # | Screen / Route | Area | Dark `1440×900` | Light `1440×900` | Narrow (`1100–1180`) | Large (`1920×1080`) | Key Verified Improvements |
|---|---|---|---|---|---|---|---|
| 01 | **Welcome** | Gate | PASS | PASS | PASS | PASS | Refined `38×20` mathematical topographic field with sky-blue crests; intact founder logo; clean CTA hierarchy |
| 02 | **Access (Unlock / Create)** | Gate | PASS | PASS | PASS | PASS | Live `0/8 min` passphrase counter, red `role="alert"` mismatch cue, `{MOD}+C` copy hint, destructive set-aside confirm |
| 03 | **Home (Workspace)** | Home | PASS | PASS | PASS | PASS | 4-metric rail (`Subjects`, `Needs review`, `Evidence corpus`, `Sources`), queue layout fix, coverage `SegmentedBar`, zero `5/9` clipping |
| 04 | **Patients** | Clinical | PASS | PASS | PASS | PASS | Fixed column widths (`5/9`, `Needs review`, `2019-12-01` untruncated), human FHIR labels in Inspector, `SearchField` |
| 04b | **PatientDetail** | Clinical | PASS | PASS | PASS | PASS | Demographics kept visible below 1180px, chronological SVG `TimelineChart`, `SegmentedBar`, keyboard-focused tabs, copyable Subject ID |
| 05 | **Documents** | Clinical | PASS | PASS | PASS | PASS | FHIR resource `SegmentedBar`, untruncated `MedicationRequest`, copyable source IDs, clean empty encrypted-vault guidance |
| 06 | **Insights** | Clinical | PASS | PASS | PASS | PASS | Semantically colored coverage bars (sky-blue/orange/hatched), proportional condition bars, support match % on citations |
| 07 | **Evidence** | Clinical | PASS | PASS | PASS | PASS | Corpus posture `SegmentedBar`, shared `SearchField`, lexical-score mini-bars, fixed `role="cell"`, copyable SHA-256 & Rights URI |
| 08 | **Audio** | Clinical | PASS | PASS | PASS | PASS | Responsive `.wb-2` two-column workbench, 2-row source cards (no crush), keyboard-accessible SVG `AudioSegmentTimeline`, disk & download `ProgressBar`, idle polling eliminated |
| 09 | **Models (Admitted Packs)** | Intelligence | PASS | PASS | PASS | PASS | Balanced 4-column table, context-aware `ResizableSplit` inspector, roving-tabindex tabs, hidden `"Copy digest"` on `"—"` |
| 09b | **Models (MedScale Catalog)** | Intelligence | PASS | PASS | PASS | PASS | Catalog status `SegmentedBar`, clickable rows with full Inspector card (`Catalog ID`), untruncated `Disk`/`Status`, `"Clear filters"` + empty state |
| 09c | **Models (Try a model)** | Intelligence | PASS | PASS | PASS | PASS | Inline `.entity-mark` span highlighting, `{MOD}+Enter` submit, `Disclosure` payload, Inspector evaluation receipt |
| 10 | **MedAgent** | Intelligence | PASS | PASS | PASS | PASS | `LifecycleStepper`, destructive revoke confirm, `Select all`/`Clear` sources, `{MOD}+Enter`, `<=1439px` 2-col collapse |
| 11 | **Model Fleet** | Intelligence | PASS | PASS | PASS | PASS | Responsive `.wb-side`, human `role_label` on lane runs, comparison observation `SegmentedBar`, readiness hint |
| 12 | **Knowledge (Canvas)** | Intelligence | PASS | PASS | PASS | PASS | Deterministic SVG `CanvasArgumentMap` with box-edge curves & unsupported-note highlights, readable link dropdown previews |
| 13 | **Projects** | Research | PASS | PASS | PASS | PASS | `ResizableSplit` keeps `"New project"` reachable, full-width `Experiments` input, destructive Archive confirmation, copyable `Project ID` |
| 14 | **Data** | Research | PASS | PASS | PASS | PASS | Responsive `.wb-side`, uniform-column parsed snapshot table + Raw toggle, row-count mini-bars, `"Query in Analytics"` link |
| 15 | **Browse** | Research | PASS | PASS | PASS | PASS | Conditional synthetic fixture notice, blocked-fetch toast + 1-click `"Allow <host>"`, session outcomes `SegmentedBar` |
| 16 | **Analytics** | Research | PASS | PASS | PASS | PASS | Inline toolbar button alignment, `{MOD}+Enter` execution, receipt outcomes `SegmentedBar`, plain-language read-only notice |
| 17 | **Research OS** | Research | PASS | PASS | PASS | PASS | 3-column `.plane-grid` with bespoke plain-language descriptions for all 8 infrastructure planes |
| 18 | **Workflows** | Operations | PASS | PASS | PASS | PASS | `ResizableSplit`, original SVG `EffectSafetyDiagram` (sky-blue + danger UNKNOWN stop), per-step state glyphs, cross-links |
| 19 | **Tasks** | Operations | PASS | PASS | PASS | PASS | Copyable full SHA-256 payload digests, plain-language pending action queue copy, cross-links to Workflows & Messages |
| 20 | **Messages** | Operations | PASS | PASS | PASS | PASS | Copyable action IDs, clear empty state, cross-links to Workflows & Tasks |
| 21 | **Collaboration** | Operations | PASS | PASS | PASS | PASS | Responsive `.wb-3` grid, friendly source labels on threads, disabled state on `Complete`, `{MOD}+Enter` message posting |
| 22 | **Privacy** | Governance | PASS | PASS | PASS | PASS | Plain-language boundary names alongside keys, egress posture `SegmentedBar`, copyable artifact IDs |
| 23 | **Audit Trail** | Governance | PASS | PASS | PASS | PASS | `SearchField` filter, expandable full untruncated detail rows, `CopyButton` for record ID and detail |
| 24 | **Exports** | Governance | PASS | PASS | PASS | PASS | Structured FHIR support table (no raw JSON blob), export field fidelity `SegmentedBar`, `Open Audit Trail` button |
| 25 | **Integrations** | Governance | PASS | PASS | PASS | PASS | Clean empty state + contextual `Open Browse` and `Open Privacy` navigation actions |
| 26 | **Help & Feedback** | Utility | PASS | PASS | PASS | PASS | 1-click `Copy diagnostics` in header, harmonized browser-link vs copy-only copy, `About MedScale` link |
| 27 | **Settings** | Utility | PASS | PASS | PASS | PASS | Zero unnamed UIA groups (`role="region"` + `aria-label`), `IntersectionObserver` scroll-spy, arrow-key `Seg` radiogroup, full-width Recovery Codes + `Copy codes`, `Show passphrases`, `.kbd` shortcut list |
| 28 | **About** | Utility | PASS | PASS | PASS | PASS | Intact founder mark & wordmark, ordered `05`/`06` sections, conditional synthetic vs encrypted notice, plain-language license/provenance labels |
| 29 | **Command Palette** | Shell | PASS | PASS | PASS | PASS | Instant zero-delay open, grouped `Routes` (with area badges) + `Actions`, `Home`/`End` nav, clean focus restore on `Escape` |

---

## 3. Non-Negotiable Invariants Verified

- **Founder Brand Mark:** Verified byte-identical SVG path across all brand assets and runtime components via `src/identity.test.ts`.
- **Zero Remote Runtime Dependencies:** Production CSP (`default-src 'self' ipc: http://ipc.localhost`) and offline bundled Inter + JetBrains Mono NL verified via `src/bundle-config.test.ts` and `qa/first-run-browser.mjs`.
- **WebView Privacy Canary:** No project IDs, patient identifiers, search queries, or canvas notes persisted to browser storage; only theme and density preferences remain persisted locally.
- **No Clinical / Regulatory Overclaims:** `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `CLINICAL_USE_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`, and `RELEASE_READY=false` remain unchanged.

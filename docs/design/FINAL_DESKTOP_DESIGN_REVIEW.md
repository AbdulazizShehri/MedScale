# MedScale Desktop — Final Design & UX Review

**Product:** MedScale Desktop (`apps/desktop-tauri/`)  
**Identity:** Clinical Intelligence OS — *Evidence first. Action second.*  
**Baseline compared against:** `main@80c67d2dfae139f7471f7f414c9cd112d8a5b07e` (qualified `0.1.0-beta.1` RC run `45916049004`)  
**Release boundaries preserved:** `REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED`, `CLINICAL_USE_AUTHORIZATION=NOT_AUTHORIZED`, `TAURI_WEBVIEW_PRIVACY_QUALIFICATION=OPEN`, `RELEASE_READY=false`

---

## 1. Executive Design Verdict

MedScale Desktop has been systematically elevated across its shell, design system, data visualizations, accessibility, and all 26 routes (plus Welcome, Access, and PatientDetail) while strictly preserving the founder-approved symbol, wordmark, black-and-white foundation, restrained sky-blue (`#70B8C7` / light text `#256B7B`) and orange (`#FB905A` / light text `#B4541F`) semantic grammar, local-only bundled Inter + JetBrains Mono NL typography, and zero-network-by-default privacy posture.

Every control on every screen is wired directly to the in-process Rust Core Host (`apps/desktop-tauri/src-tauri/`). Zero decorative widgets, zero invented clinical scores, and zero remote assets were introduced.

---

## 2. Architecture & Design-System Upgrades (`medscale.css`, `ui.tsx`, `shell.tsx`, `App.tsx`)

### Visual Craft & Contrast Discipline
- **WCAG AA contrast fixes across both Dark and Light themes:**
  - Fixed weak status labels, secondary mono metadata, stamp prefixes, and ledger keys inside selected rows (`.sel .st.weak`, `.sel .stamp i`, `.sel .st .mono`, `.sel .lk`) from `--ink-3` (~4.2–4.3:1) to `--ink-2` (≥ 4.5:1).
  - Fixed active sidebar counter badges (`.nav.on .nav-n`) to `--ink-2` (≥ 4.5:1).
  - Fixed destructive action buttons (`.btn-d`, `.btn-d-fill`) to true danger red (`--sem-danger`) with AA-compliant text in both themes (`#090909` on `#F0695A` dark, `#FFFFFF` on `#C2321D` light).
  - Fixed split-pane resize handle keyboard focus indicator (`.split-resizer:focus-visible::after`) to use `--focus` + `--focus-halo` (≥ 3:1 WCAG 1.4.11 non-text contrast in Light mode).
  - Differentiated the header `"Encrypted vault"` pill (`.synth.vault`) in neutral structural ink from the orange `"Synthetic workspace"` caution pill (`.synth`).
- **Native desktop scrollbar & tactile feel:**
  - Replaced harsh OS scrollbars in WebView2/WebKit with quiet 7px themed pill scrollbars (`.ms ::-webkit-scrollbar`) and added a subtle bottom fade mask on `.sb-nav`.
  - Added pointer-fine tactile press feedback (`scale(0.985)`, `110ms`) without adding entrance animation latency to keyboard-driven overlays (`Command Palette` opens instantaneously).

### Workstation Shell, Ergonomics & Responsive Resilience
- **Collapsible Sidebar (`Ctrl/Cmd+B`):** Compresses cleanly into a 56px icon rail with preserved hairline section dividers, expanding workspace room on 1180–1280px laptop displays.
- **Resizable Split Panes + Responsive Inspector Drawer (`ResizableSplit`):**
  - Replaced static split layouts across `Home`, `Patients`, `PatientDetail`, `Documents`, `Evidence`, `Insights`, `Projects`, `Models`, and `Workflows`.
  - Supports pointer-captured drag resizing, keyboard arrow resizing (`ArrowLeft`/`ArrowRight`/`Home`), and double-click width reset.
  - Uses CSS custom property `--ins-w` clamped to `<= 316px` at `<= 1279px` (protecting 125% DPI laptops) and transitions cleanly into an accessible overlay drawer (`Inspector` header toggle with focus handoff & `Escape` dismiss) below `1180px`.
- **Upgraded Command Palette (`Ctrl/Cmd+K`):** Grouped `Routes` (with area tags) and `Actions` (theme, density, sidebar toggle, lock), instant open, `Home`/`End` keyboard nav, live result count, and focus return.
- **Global Workstation Shortcuts:** `Ctrl/Cmd+K` (Palette), `Ctrl/Cmd+B` (Sidebar), `Ctrl/Cmd+,` (Settings), `Ctrl/Cmd+Shift+L` (Lock), `Alt+←` (Back to Patients from PatientDetail), `/` (Focus active page search), and `Escape` (dismiss drawer/palette).
- **Reusable Primitives (`components/ui.tsx`):** Added `SearchField` (with clear button), `CopyButton` (with `aria-live` copied/failed feedback), `SegmentedBar`, `ProgressBar`, `LifecycleStepper`, `Disclosure`, and `humanKey` acronym formatting (`ID`, `URI`, `URL`, `FHIR`, `SQL`).

---

## 3. Route-by-Route & Data-Visualization Improvements

### Onboarding & Security Gates (`Gate.tsx`)
- **Welcome:** Refined deterministic topographic dot field (`38×20` mathematical height field with quiet sky-blue crest highlights, zero animation, zero external libraries), scoped pending button states cleanly, and replaced technical seeding copy with plain language.
- **Access:** Live passphrase length indicator (`0/8 min`), real-time mismatch alert (`var(--sem-danger)`, `role="alert"`), platform-aware `{MOD}+C` guidance, and destructive confirmation on permanent set-aside deletion.

### Clinical Intelligence (`Home.tsx`, `Patients.tsx`, `Clinical.tsx`)
- **Home:** Replaced cramped header metrics with a quiet 4-metric architectural rail (`Subjects`, `Needs review`, `Evidence corpus`, `Sources`), fixed the `Needs a person` queue row grid (eliminating the dead 200px stamp column and raw `SubjectCoverageV1` tooltip), added a real-data **Record coverage across subjects** `SegmentedBar`, and fixed `Coverage` (`5/9`) & `Latest` (`2019-12-01`) table truncation.
- **Patients & PatientDetail:**
  - Fixed table column widths so `Coverage` (`5/9`, `2/8`), `Review` (`Needs review`), and `Latest` (`2019-12-01`) render completely untruncated across `1180px`–`1920px`.
  - Replaced raw FHIR dotted paths (`patient.name.famil`/`y`) and raw array JSON (`["Ada"]`) with human-readable labels (`Family name: Synthetic`, `Given name: Ada`) while preserving raw keys on hover.
  - Added chronological SVG **`TimelineChart`** (sorted by event timestamp), **`SegmentedBar`** coverage breakdown, roving-tabindex keyboard tab navigation with DOM focus movement, copyable Subject ID, and collapsible technical rules (`Disclosure`).
- **Documents:** Added **Sources by FHIR resource type** `SegmentedBar`, widened `Resource` column so `MedicationRequest` stays on one line, fixed empty encrypted-vault guidance, and added `CopyButton` on source IDs.
- **Evidence:** Added **Corpus document posture** `SegmentedBar`, lexical-score mini-bars on retrieval hits (explicitly labeled *relevance only, not authority*), shared `SearchField`, fixed `role="cell"` ARIA semantics, and `CopyButton` on SHA-256 digests and rights URIs.
- **Insights:** Color-coded coverage distribution bars semantically (sky-blue Present, orange Conflict, red Unhealthy, diagonal-hatched Incomparable, neutral Absent), added proportional Condition bars in the inspector, and added lexical support match % + `Open Evidence` jump button.

### Intelligence & Agents (`Intelligence.tsx`)
- **Models:** Context-aware `ResizableSplit` inspector across all three tabs (`Admitted packs`, `MedScale Catalog`, `Try a model`), roving-tabindex keyboard tabs, balanced non-truncating table widths, **Catalog status distribution** `SegmentedBar`, `"Clear filters"` + clean zero-match empty state (eliminating `"rows 1–0"`), inline **Annotated synthetic text** entity highlighting (`.entity-mark`) from `start`/`end` offsets, and `{MOD}+Enter` submission.
- **MedAgent:** Added **`LifecycleStepper`** (`Pending → Running → Executed → Completed/Cancelled`), `ConfirmPanel destructive` before revoking an active agent, `Select all`/`Clear` source toggles, `{MOD}+Enter` prompt creation, and responsive 2-column breakpoint at `<= 1439px` so the central workspace never crushes.
- **Model Fleet:** Responsive `.wb-side` workbench, human `role_label` shown on lane runs, readiness hint beside `"Compare lanes"`, and observation distribution `SegmentedBar`.

### Research & Operations (`Research.tsx`, `Operations.tsx`)
- **Projects:** Wrapped in `ResizableSplit` (fixing `"New project"` vanishing at narrow widths), full-width `Experiments` form above 2-column `References`/`Graph`, and destructive confirmation before archiving.
- **Data & Analytics:** Responsive `.wb-side`, uniform-column parsed snapshot data table with Raw-text toggle, snapshot row-count mini-bars, `{MOD}+Enter` SQL execution, and receipt outcomes `SegmentedBar`.
- **Knowledge (Research Canvas):** Added deterministic SVG **`CanvasArgumentMap`** (bipartite Notes ↔ Citations with `supports`/`contradicts`/`questions` curves anchored at box edges and unsupported-note highlights) alongside the preserved raw result preview.
- **Browse:** Conditional synthetic fixture notice, blocked-fetch warning toast + 1-click `"Allow <host>"` recovery action, and broker session outcomes `SegmentedBar`.
- **Research OS:** Crisp 3-column `.plane-grid` with bespoke plain-language descriptions for every plane (`Hub`, `Compute`, `R Workspace`, `Extensions`, `Huddles`, `Research Packs`, `Adapters`, `Federation`).
- **Workflows, Tasks & Messages:** Wrapped `Workflows` in `ResizableSplit`, rendered an original SVG **Fail-closed effect lifecycle** diagram (`Review → Awaiting effect → Confirmed / Failed / UNKNOWN`), fixed per-step timeline glyphs, added copyable full SHA-256 payload digests on `Tasks`, and added cross-navigation links.
- **Collaboration & Audio:** Responsive `.wb-3` grid in `Collaboration` with friendly source labels; organized `Audio` into a crisp `.wb-2` two-column workbench with interactive keyboard-accessible SVG **`AudioSegmentTimeline`**, `ProgressBar` for disk preflight budget and live Whisper model downloads, and on-demand download polling (zero idle 1500ms polling).

### Governance, Settings, Help & About (`Governance.tsx`, `Help.tsx`)
- **Privacy:** Plain-language boundary names alongside keys and recorded egress posture `SegmentedBar`.
- **Audit Trail:** `SearchField` filtering + expandable untruncated disclosure detail rows with `CopyButton`.
- **Exports:** Structured non-JSON FHIR support table + **Export field fidelity** `SegmentedBar` + `Open Audit Trail` link.
- **Settings:** Resolved all **5 unnamed UIA `ControlType.Group`** elements via `role="region"` + `aria-label` on section containers; added `IntersectionObserver` scroll-spy highlighting `.settings-link.on`, roving-tabindex arrow-key navigation on `Seg` radiogroups, full-width Recovery Codes display with `Copy codes`, `Show passphrases` toggle, and `.kbd` keycap shortcut table.
- **About & Help:** Harmonized external-link vs copy-only diagnostics copy, added 1-click `Copy diagnostics` to `Help & Feedback`, fixed `05` / `06` section order on `About`, and removed developer-path jargon (`docs/legal/NOTICE_INVENTORY.md`, `cargo-deny`).

---

## 4. Two-Pass Verification & Critique Summary

1. **BEFORE Baseline Inspection (`main@80c67d2d`):** Captured 40 real native Windows WebView2 screenshots + UIA accessibility/clipping audits; visually identified column clipping on `Home`/`Patients`/`Models`, FHIR key wrapping in `Patients`, 5 unnamed UIA groups in `Settings`, and scrollbar harshness.
2. **Pass 1 Native Capture & Independent Design Critique:** Captured 44 real native screenshots and ran an independent Design & Accessibility Critic audit. Caught and resolved:
   - Two undefined CSS variables in `Operations.tsx` SVGs (`--accent-blue`/`--accent-orange` → `--accent`/`--signal`).
   - `ResizableSplit` inline `width` overriding `@media (max-width: 1279px)` on 125% DPI Windows laptops.
   - `Disclosure` controlled `open={false}` collapsing on parent re-renders.
   - `.synth` orange border applied to `"Encrypted vault"`.
   - Narrow-table truncation of `Patients` (`Review`/`Latest`), `Documents` (`MedicationRequest`), `Models Catalog` (`Status`), and `Projects` (`Experiment name`).
   - WAI-ARIA tab/radiogroup focus handoff and AA contrast on `.nav.on .nav-n` and `.split-resizer:focus-visible`.
3. **Pass 2 Final Verification:**
   - Desktop unit tests: **85 / 85 PASS** (`contrast.test.ts`, `identity.test.ts`, `accessible-rows.test.ts`, `routes.test.ts`, `bundle-config.test.ts`, `ipc.test.ts`, etc.).
   - First-run browser smoke: **12 / 12 PASS**.
   - Visual route alignment & overflow check: **100 / 100 views PASS (25 routes × 2 themes × 2 widths, 0 misses)**.
   - Final native Windows binary capture (`shots-after-final`): **45 / 45 screenshots, 0 errors, 0 right-edge clipped controls, 0 unnamed interactive controls**.
   - Public repository hygiene: **PASS**.

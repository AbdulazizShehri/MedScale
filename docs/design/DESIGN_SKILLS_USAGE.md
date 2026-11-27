# Design Skills Discovery, Verification & Application Log

This document records the exact external and local design skills inspected and applied during the MedScale Desktop UX & Visual Execution mission. All skills were used strictly as **development-time design and engineering guidance** — none were added as MedScale runtime dependencies, and no unsafe scripts or remote services were introduced.

---

## 1. Mandatory Design Skills

| Skill | Verified Source & License | How Principles Were Applied to MedScale Desktop |
|---|---|---|
| **TasteSkill** | `https://github.com/Leonxlnx/taste-skill` (`skills/taste-skill/SKILL.md`, MIT) | Enforced anti-generic-interface discipline: eliminated SaaS card bloat and decorative borders in favor of a quiet architectural 1px-rule hierarchy, hairline `.metric-rail`, restrained monochrome + sky-blue/orange accents, crisp tabular figures (`tnum`), and zero unrequested visual clutter. |
| **Algorithmic Art** | `https://github.com/anthropics/skills/tree/main/skills/algorithmic-art` (`SKILL.md`, Apache-2.0) | Applied strictly where deterministic mathematical structure improves comprehension without distracting from clinical work: refined the `Welcome` screen's seeded `38×20` Gaussian-harmonic topographic dot field with subtle sky-blue crest highlights, plus deterministic geometric SVGs (`TimelineChart`, `CanvasArgumentMap`, `AudioSegmentTimeline`, `EffectSafetyDiagram`) with zero external canvas/CDN libraries. |
| **Meg to Skill → `MengTo/Skills`** | Verified exact identity: `https://github.com/MengTo/Skills` (MIT, created by Meng To; inspected `audit-ai-design-slop`, `no-ai-design-slop`, `operational-enterprise-ai`, `split-layout-technical`) | Audited and removed "AI design slop" patterns (raw `JSON.stringify` dumps in FHIR support matrix, repetitive generic plane descriptions, mismatched button heights, thick colored notice bars). Applied `split-layout-technical` & `operational-enterprise-ai` ergonomics: pointer-captured `ResizableSplit`, collapsible icon-rail sidebar (`Ctrl/Cmd+B`), responsive inspector drawer, and crisp inline operational telemetry (`LifecycleStepper`, `SegmentedBar`, `ProgressBar`). |
| **Frontend Design** | `https://github.com/anthropics/skills/tree/main/skills/frontend-design` (`SKILL.md`, Apache-2.0) | Guided typography pairing (Inter Variable UI + JetBrains Mono NL exact identifiers/digests), intentional whitespace cadence, cohesive Dark/Light surface layering (`--bg-app` → `--bg-surface` → `--bg-raised`), and clear visual hierarchy on every route. |
| **UI/UX Pro Max** | `https://github.com/nextlevelbuilder/ui-ux-pro-max-skill` (`.claude/skills/ui-ux-pro-max/SKILL.md`, MIT) | Guided desktop interaction polish: instant zero-latency Command Palette opening, `SearchField` clear controls, 1-click `CopyButton` with live feedback, `ConfirmPanel` destructive-action guardrails, empty/error recovery actions, and responsive breakpoints (`1920×1080`, `1440×900`, `1180×720`, `<1180px`). |

---

## 2. Complementary Verified Local Skills Applied

| Skill | Origin & License | Concrete Application |
|---|---|---|
| **Impeccable** (`v4.8.0`) | Verified local installation (`SKILL.md`, `reference/craft-floor.md`, `reference/operate.md`, Apache-2.0) | Shaped clinical workstation density, zero-truncation table sizing (`Patients`, `Documents`, `Models`), honest non-clinical boundary messaging, and plain-language microcopy (removing developer path jargon and raw FHIR dotted keys). |
| **Emil Kowalski Design Engineering** | Verified local skill (`emil-design-eng/SKILL.md`) | Applied subtle fine-pointer tactile button response (`scale(0.985)` on `:active`, `110ms`), zero entrance animation on high-frequency keyboard surfaces (Command Palette), and quiet themed native scrollbars. |
| **Apple Design** | Verified local skill (`apple-design/SKILL.md`) | Guided desktop navigation ergonomics (interactive breadcrumbs, platform-aware `MOD` keycaps for Windows/Linux `Ctrl` vs macOS `⌘`, sticky Settings scroll-spy, and clean disclosure controls). |
| **Accessibility Review & Design Critique** | Verified local skills (`accessibility-review`, `design-critique`) | Guided both the independent Pass-1 Design & Accessibility Critic review and our WCAG AA/1.4.11 remediation: fixing `.nav.on .nav-n`, `.sel .st.weak`, `.split-resizer:focus-visible`, WAI-ARIA tab/radiogroup roving tabindex + focus movement, `aria-valuetext`, and eliminating all 5 unnamed Windows UIA groups in `Settings`. |

---

## 3. Identity Precedence Guarantee

Whenever generic design-skill guidance conflicted with MedScale's canonical `DESIGN.md` or founder identity decisions, **MedScale's brand and safety specifications always took precedence**:
- Founder mark SVG geometry and pure `#000000` / `#FFFFFF` monochrome fills remained byte-identical.
- Sky-blue (`#70B8C7` / `#256B7B`) and orange (`#FB905A` / `#B4541F`) remained strictly bounded to interaction and human-review attention semantics.
- Every visual indicator remained backed by literal Rust Core data — zero speculative charts, zero fabricated clinical risk/confidence scores, and zero remote assets.

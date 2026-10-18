---
name: MedScale
description: Approved black/white shared system with restrained sky-blue interaction and orange attention accents, for synthetic-only Tauri presentation
colors:
  dark-canvas: "#080808"
  dark-sidebar: "#0d0d0d"
  dark-surface: "#111111"
  dark-raised: "#171717"
  dark-selected: "#242424"
  dark-hover: "#1d1d1d"
  dark-line: "#282828"
  dark-line-strong: "#3c3c3c"
  dark-text: "#f6f6f4"
  dark-secondary: "#a3a3a0"
  dark-muted: "#939390"
  dark-inverse: "#090909"
  dark-focus: "#70b8c7"
  dark-accent: "#70b8c7"
  dark-attention: "#fb905a"
  light-canvas: "#f8f8f6"
  light-sidebar: "#ffffff"
  light-surface: "#ffffff"
  light-raised: "#f4f4f2"
  light-selected: "#ebebe8"
  light-hover: "#f2f2ef"
  light-line: "#e5e5e1"
  light-line-strong: "#cececa"
  light-text: "#0a0a0a"
  light-secondary: "#575754"
  light-muted: "#666662"
  light-inverse: "#ffffff"
  light-focus: "#256b7b"
  light-accent: "#70b8c7"
  light-accent-text: "#256b7b"
  light-attention-text: "#b4541f"
typography:
  title:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "28px"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.03em"
  section:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "-0.02em"
  body:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "13px"
    lineHeight: 1.6
  label:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "12px"
  caption:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 500
    lineHeight: 1.5
  exact-identity:
    fontFamily: '"JetBrains Mono NL", monospace'
    fontSize: "11px"
rounded:
  sm: "4px"
  md: "6px"
spacing:
  space-1: "4px"
  space-2: "8px"
  space-3: "12px"
  space-4: "16px"
  space-6: "24px"
  space-8: "32px"
components:
  button:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-text}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "0 {spacing.space-4}"
  button-hover:
    backgroundColor: "{colors.dark-hover}"
  button-active:
    backgroundColor: "{colors.dark-selected}"
  button-light:
    backgroundColor: "{colors.light-surface}"
    textColor: "{colors.light-text}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "0 {spacing.space-4}"
  button-light-hover:
    backgroundColor: "{colors.light-hover}"
  button-light-active:
    backgroundColor: "{colors.light-selected}"
  nav-item:
    textColor: "{colors.dark-secondary}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0 10px"
    height: "36px"
    width: "100%"
  nav-item-selected:
    backgroundColor: "{colors.dark-selected}"
    textColor: "{colors.dark-text}"
  nav-item-light:
    textColor: "{colors.light-secondary}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0 10px"
    height: "36px"
    width: "100%"
  nav-item-light-selected:
    backgroundColor: "{colors.light-selected}"
    textColor: "{colors.light-text}"
  search-unavailable:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-secondary}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "0 {spacing.space-3}"
    width: "min(100%, 420px)"
  search-unavailable-light:
    backgroundColor: "{colors.light-surface}"
    textColor: "{colors.light-secondary}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "0 {spacing.space-3}"
    width: "min(100%, 420px)"
---

# MedScale design system

**Current authority:** Spec 095 and the founder-approved identity supplied 2026-09-29. See [productization](docs/planning/PRODUCTIZATION_PROGRAM.md); launch UI is not frozen.

## Identity

The locked mark derives from paired equilateral triangles forming an abstract M, with two peaks, center contact and lower negative space. Only pure black #000000 or white #FFFFFF. No stroke, rounding, circle, gradient or healthcare/AI decoration. assets/brand/mark-black.svg is the master; inverse/native variants share its path. docs/brand/LOGO_SPEC.md defines size, clear space and lockups.

MedScale is a **Clinical Intelligence OS**. **Evidence first. Action second.** is an operating principle, not a readiness claim. Typography and clear relationships carry the identity; triangles belong primarily to the logo.

## Tokens and typography

The preserved Slint source is crates/medscale-desktop/ui/theme.slint. The 096 migration prototype expresses the same approved brand roles in apps/desktop-tauri/src/theme.css; its tokens and native rendering remain unqualified until evidence is captured. Inter handles UI/wordmark; JetBrains Mono NL handles exact IDs, hashes, timestamps and scientific operators without code ligatures. Minimal admitted OFL assets are bundled locally and distributed with their licenses. Full roles: docs/brand/TYPOGRAPHY_SYSTEM.md.

Light uses white/near-white; dark uses black/near-black layers. Neutrals express hierarchy. Black/white MedScale foundation with restrained sky-blue (#70B8C7) interaction accents and orange (#FB905A) attention accents (founder decision 2026-10-07, `docs/design/BRAND_ACCENTS_2026-10-07.md`). Status semantics remain shape- and text-backed and never rely on color alone. Blue marks focus, selection, interaction and data bars; orange marks attention and review. Light mode uses darker accessible variants for accent text and glyphs.

## Product grammar

Prefer panes, readable tables, lists, inspectors, timelines and thin separators. Cards require a grouping purpose. No marketing hero, invented metrics, decorative tiles or perpetual animation. Target 1440×900, minimum 1100×720. Compact changes spacing/control geometry while retaining readable type. Navigation scrolls independently and retains all 25 real routes.

Home's Command Center shows existing synthetic patient context, coverage uncertainty, actual runtime truth and clear links to clinical/evidence/research work. It is not a live roster or clinical risk ranking. Navigation and route qualification proceed through Specs 095–100.

## States and evidence

Current, reviewed, unknown, stale, conflicting, partial, denied, unavailable, unsupported, unmeasured and corrupt retain distinct text and accessible meaning. Unknown never defaults to success. StateBadge supplies a literal state, explanation and distinct monochrome marker. Evidence discloses available source/revision, relationships, uncertainty, freshness/review and lineage. The existing Evidence route is a pinned comparative proof ledger, with no implied live literature service.

## Interaction and qualification

Controls retain Enter/Space, visible focus and exposed role/label/action. Targets are at least 36px normally and 32px compact. Default motion/elevation is zero; future transitions must explain state and respect reduced motion. Measure contrast from actual tokens; inspect actual native pixels. Record keyboard, assistive technology and WCAG claims separately. Compilation/source assertions do not qualify visuals.

Rust Core retains all authority. PHI is unauthorized, egress defaults deny, MESC is separate, and release/privacy/multi-client readiness stays false. CLI machine contracts remain stable. References and figure styles do not create capabilities.

## History

- Pre-068 identity: SUPERSEDED_BY_SPEC_068.
- Spec 068 active visuals: SUPERSEDED_BY_SPEC_073.
- Spec 073 circular Shelf-M, Instrument Sans/Source Serif and Mist Blue active visuals: SUPERSEDED_BY_SPEC_095.
- Closed Specs 068–094 and their evidence remain intact; new identity replaces forward.

## Overview

**Creative North Star: "The work is the interface"**

The following shared-system layer records the approved Tauri shell and Spec 097
Patients preparation. Frontmatter values are normative for this presentation
scope and are extracted from `apps/desktop-tauri/src/theme.css`, not inferred
from a generic preset. The CSS remains the implementation source; use its
theme variables rather than literal theme colors in new components. The
preceding sections and historical source references are retained.

The paired-M mark, one sidebar, quiet black/white hierarchy, Inter and exact
data in JetBrains Mono NL remain the approved world. Light mode uses the same
component relationships with its own neutral layers. This documentation does
not qualify a populated clinical surface, privacy, accessibility or release.

**Key Characteristics:**

- Clinical context and literal evidence states carry the interface.
- Thin separators, readable tables and bounded inspection organize the work.
- Shared geometry and first-party outline icons connect the two themes.

## Colors

The palette is black/white with two restrained accents (sky blue for interaction, orange for attention); see `docs/design/BRAND_ACCENTS_2026-10-07.md`. `dark-*` maps to `:root` variables and `light-*` maps
to their `data-theme=light` overrides; the suffix retains the source role.
Canvas, sidebar, surface and raised establish layers. Selected and hover convey
interaction; line and line-strong convey separation. Text, secondary and muted
carry hierarchy, with inverse reserved for inverse content and selection.
Focus is an explicit theme role. Logo black/white remains governed by Identity
and is not recolored by these surface roles. No additional accent is introduced.

## Typography

The frontmatter records the shared title, section/empty-state heading, body,
label, caption/status and exact-identity roles. Title measurements come from
the Patients heading; section weight and line height come from the shared empty
state heading; body line height comes from shared explanatory copy. Label has
no universal weight or line height: buttons, navigation and tables set their
own values in source. Locally bundled licensed fonts remain required.

Existing Home, wordmark, palette and historical display metrics still have
separate declarations. This layer does not claim all typography is centralized.

## Layout

Use the extracted spacing scale for new shared primitives. The source shell
has a sidebar beside one flexible workspace; navigation and main content
scroll independently. The contextual header uses `--header-height` (66px).
The sidebar is 234px, changing to 207px at the 1180px source breakpoint and
180px at 850px. Other inherited layout metrics remain explicit CSS values.

The shared table has a 40px header, prospective 52px rows and a 680px minimum
width inside its own scrolling region. The current four-column allocation is
specific to Patients, not a global table rule. Explanatory copy uses the source
65ch measure. Configured desktop targets remain those in Product grammar;
smaller CSS breakpoints are implementation behavior, not mobile qualification.

## Elevation & Depth

Shared clinical tables and controls use tonal layers and separators. The
existing command palette alone uses the theme shadow recorded in the sidecar.
Navigation retains its source 120ms background/color transition; reduced-motion
media rules disable transitions and animations. No extra motion is introduced.

## Shapes

Shared controls use the small radius; navigation and theme grouping use the
medium radius. The icon family is first-party geometry: 24-unit view box,
18px display size, two-unit outline, square ends, miter joins, no fill and
current text color. Icons are decorative alongside labels; they do not replace
accessible names. This icon grammar does not alter the filled paired-M logo.

## Components

`components/MedScale.tsx` supplies Sidebar, NavItem, Header, Search, Status,
Table, Row, EmptyState, Section, Button and RuntimeState; `MedScaleIcon.tsx`
supplies the icon family. Use these primitives for supported new surfaces.

- **Navigation:** one labeled sidebar; selected route has `aria-current=page`,
  stronger text and a trailing sky-blue indicator. Hover uses the hover role.
- **Buttons:** native button semantics, source minimum height of 36px, strong
  border, hover/active layers and visible theme focus. Disabled is explicit.
- **Search:** the present patient search is a disabled search input with a
  literal Unavailable label and explanation. An enabled roster search is absent.
- **Status:** literal text plus a shape marker (orange only reinforces review/attention); marker geometry supplements
  the text. Unknown, unavailable, unsupported and other declared states stay
  distinct even when they share a marker.
- **Table/Row:** semantic caption, column headers and row headers, wrapped exact
  identifiers, a labeled keyboard-focusable scroll region. Its authored focus
  outline uses the existing 2px focus role inset by 2px to avoid clipping.
- **EmptyState/Section:** inline heading and constrained explanation, separated
  by the same spacing and line grammar. No record facts are supplied by styling.
- **Header/RuntimeState:** current route, session appearance and literal local
  preview/workspace availability; workspace access does not imply sign-in.

Tabs await the supported Patient Detail slice. The row primitive exists but
the Patients screen renders no records. The sidecar samples show implemented
unavailable presentation and controls, with no invented clinical row.

## Do's and Don'ts

### Do:

- **Do** use shared primitives and theme variables for new supported surfaces.
- **Do** keep literal states, accessible labels and a visible sky-blue focus ring.
- **Do** keep exact identifiers in the locally bundled data font.
- **Do** preserve the paired-M identity and the same relationships in light mode.

### Don't:

- **Don't** substitute fixture patients or successful empty states for an unread roster.
- **Don't** introduce an accent palette, competing sidebar or decorative clinical metrics.
- **Don't** infer privacy, WCAG, native platform visual or release readiness from this token record.

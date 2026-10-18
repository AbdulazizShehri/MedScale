# Shared system documentation handoff — Spec 097

## Scope and authority

The founder approved the existing native Tauri shell, its paired-M monochrome
identity, a first-party icon family and formal shared primitives/central tokens.
This is an English-only documentation merge for that implemented system.
`DESIGN.md` retains its existing prose, source sections and historical record;
the frontmatter and appended canonical sections add the scoped shared layer.
`.impeccable/design.json` extends that layer with geometry, focus, motion,
breakpoints, palette shadow and six framework-independent component samples.

No implementation, identity replacement, palette, dependency, capability,
Core path, Slint retirement or readiness change is made by this handoff.

## Extracted source

Read `PRODUCT.md`, existing `DESIGN.md`, `surface-brief.md` and the admitted
pinned `target/productization/impeccable-document.md` reference. The source
extraction used:

- `apps/desktop-tauri/src/theme.css`: exact authored dark/light color roles,
  font families, shared type sizes, six spacing steps, two radii, control,
  header, icon and table geometry, focus, responsive and reduced-motion rules.
- `components/MedScale.tsx`: actual Sidebar, NavItem, Header, Search, Status,
  Table, Row, EmptyState, Section, Button and RuntimeState semantics.
- `components/MedScaleIcon.tsx`: first-party paths on the 24-unit grid, 18px
  rendering, two-unit outline, square ends and miter joins.
- `patients/PatientIndex.tsx`: actual unknown total, unavailable roster,
  disabled search, four columns, inline disclosure and Return to Home.

The frontmatter carries both theme role sets without inventing ramps or accent
colors. Typography records observed roles rather than a new display scale.
CSS theme variables remain the implementation source. Some inherited Home,
palette, wordmark and layout metrics remain hardcoded, so complete system
centralization is not claimed. No inactive Tabs component is documented;
Tabs await supported Patient Detail. No patient row is populated or sampled.

## Evidence inspected and limits

Read the existing native evidence README, build receipt and capture metadata
under `evidence/097-clinical-surfaces/native/4641cbb`, plus current `findings.md`.
These records bind nine original Windows frames in both themes at configured
sizes and an inferred intermediate size to source
`4641cbb57d27edcd4abf661689cb46a60a45b09c`. This documenter did not rerun the
detector, browser, capture or provider review, or independently remeasure pixels.

The parent supplied a fresh live state snapshot: draft PR 175 at that head,
three successful native build/test/package jobs in run 36830696375, a separate
failed inherited glib dependency-policy job, five successful canonical jobs
with Windows Rust still pending at that snapshot. Those build results do not
establish macOS/Linux visual evidence. Local source includes the subsequent
2px table-region focus outline with a -2px inset. Exact-head native capture and
the fix verdict for that change were pending at this handoff.

Core roster/adapter, populated clinical row, Patient Detail/Evidence,
predecessor closure, complete canonical CI and privacy qualification remain
open. Jev and provider-backed Alibaba OCR remain `BLOCKED_BY_ZERO_COST`.
No provider PASS, merge, `CLOSED_CANONICAL`, WCAG or release claim follows.

## Preserved pre-existing drift

The older DESIGN token paragraph still calls its Tauri rendering unqualified
until evidence capture. That retained historical prose is contextualized by
this scoped addition; it was not rewritten. PRODUCT retains its earlier
historical productization metadata. The immutable native build receipt says
`native_visual_evidence=NOT_CAPTURED` at package build time. The later capture
metadata and README add the subsequent frames. These are distinct evidence
events; the receipt is preserved rather than rewritten after capture.

## Files and review

Only `DESIGN.md`, `.impeccable/design.json` and this file are changed by the
documenter. Existing approved identity, one sidebar, light/dark relationships,
literal evidence states and locally bundled fonts are retained. The sidecar
uses source CSS variables with source-value fallbacks and inline first-party
SVG; it adds no framework dependency or shipping raster. The table sample is
unavailable presentation rather than fictional clinical data.

This is a documentation extraction. JSON parsing and the final diff are to be
checked by the parent alongside the same-head focus correction. No additional
tests or qualification runs are claimed by this role.

## Later evidence recheck — 2026-10-01

After the initial handoff, inspected the new README, immutable build receipt
and capture metadata under `evidence/097-clinical-surfaces/native/7146551`,
and the later `VerdictPass` section in `finish-review.md`. These records bind
twelve original Windows JPEG frames to source
`71465513c440e61a08d3e4c3af30bf0d3b4b084c` and tree
`900543aec748a3d6fcf673373e8fb0d2be5b2098`. The README records successful
size/SHA256 verification of all 1,027 package inventory entries before launch;
the receipt and capture metadata agree on executable SHA256
`ee0f251175973672a063fe113a5d3a4a6f81d116f07950d716626e1db609bfb3`.
The receipt's build-time `NOT_CAPTURED` value remains an earlier event; the
later metadata adds the capture rather than rewriting that immutable receipt.

Nine frames repeat the original matrix and three show table focus. The
capture record and same finish reviewer report a visible, unclipped inset
monochrome ring in light at both configured targets and dark at the minimum
target. The reviewer resolved the previously scored focus correction, observed
no introduced regressions in the supplied comparisons, and returned **ship**
only for that correction. The original pending statement above is retained
as the initial snapshot; this later evidence resolves its capture/verdict
condition. It does not qualify patient records, Core connectivity or the
whole product.

Verified the documentation against current source: `theme.css` still authors
`outline:2px solid var(--focus); outline-offset:-2px` on the table region.
`DESIGN.md` still records the dark/light focus roles and inset treatment, and
the sidecar's `extensions.focus.tableRegion` matches that rule. Those shared
system files remain unchanged. The sidecar's pending `sourceState` describes
the initial handoff; this dated entry carries the later evidence state without
changing the token record.

At the supplied CI snapshot, all three native build/Clippy/test/package jobs
in run 36834231752 succeeded; the separate dependency policy still failed on
the inherited glib advisory. Canonical run 36834231751 had five successful
jobs and Windows Rust in progress. No complete canonical CI or merge readiness
is asserted. Jev and provider-backed Alibaba OCR remain `BLOCKED_BY_ZERO_COST`.
The build receipt remains `UNQUALIFIED_PREVIEW`, Core-unconnected, privacy
unqualified and release-unready.

This recheck read evidence records and compared authored source/documentation.
It did not rerun capture, detector, build, browser or provider review, inspect
additional pixels, or fabricate measurements. Configured/inferred client sizes
remain the capture record's basis; no OS client-rectangle measurement, other
platform visual acceptance, WCAG or provider PASS claim is added. Only this
handoff record was edited for the recheck; the initial history is preserved.

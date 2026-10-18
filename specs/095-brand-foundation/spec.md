# Spec 095 — Approved MedScale brand foundation

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`

**Date:** 2026-09-29

**Base:** `1e2b7d94e970256b38bda15fa91f62bc397e825a`

**Branch:** `spec/095-brand-foundation`

**Program:** `docs/planning/PRODUCTIZATION_PROGRAM.md`

## Outcome

The real native desktop uses the founder-approved paired M, licensed zero-cost typography and one monochrome design system. Home is the first real application of the system. Actual native light/dark and minimum-window renders support qualification.

## Requirements

- R095-01: one vector construction derived from two equilateral triangles, preserving both peaks, center contact and lower triangular negative space; exact black/white inverse assets, standalone icon, horizontal/stacked lockups, favicon and application variants.
- R095-02: visible mark height at least 16px; h/2 clear space; documented proportions, inverse use and misuse; no stroke, rounding, added symbol or decorative color.
- R095-03: evaluate Geist, Inter, Instrument Sans and IBM Plex Sans plus the three requested mono candidates. Choose Inter and JetBrains Mono for native tables/metadata; explicitly admit minimal unmodified OFL binaries for deterministic offline rendering, with revision/path/hash and redistributed license/NOTICE.
- R095-04: actual tokens for typography roles, neutral colors, functional states, spacing, radius, borders, focus, density, elevation and motion; light/dark work independently. Brand/focus was monochrome at 095 approval. Superseded in part by the founder decision of 2026-10-07 (`docs/design/BRAND_ACCENTS_2026-10-07.md`): the logo stays black/white; sky blue is the interaction/focus accent and orange the attention accent; status never relies on color alone.
- R095-05: current, reviewed, unknown, stale, conflicting, partial, denied, unavailable, unsupported, unmeasured and corrupt retain distinct text/accessible meaning. Unknown is not implicit success.
- R095-06: apply the foundation to the real Home header/state presentation and all common primitives; retain all routes, Core authority and exact capability boundaries.
- R095-07: bounded native snapshot path selects actual route/theme/window dimensions, captures the Slint rendered frame and exits. Reject malformed capture arguments before writes. Use isolated synthetic runtime storage. Capture Home light/dark at 1440×900 and 1100×720; record platform, binary/source digest, dimensions and limitations.
- R095-08: deterministic contrast/state/asset qualification, relevant legacy presentation guard updates under new authority, exact-range scope, all six required exact-head and post-main CI jobs. No claim of WCAG or product release readiness.

## Non-goals

No Core/storage/network/model authority change, real PHI, MESC change, reopened historical spec, new runtime crate, paid font, bitmap logo tracing, uncontrolled route redesign, new clinical claim or launch freeze. Navigation/palette fixes are owned by 096.

## Distribution policy for fonts

This spec explicitly permits and requires exactly the native type files needed to render the selected UI and exact metadata locally without installed host fonts or network requests. Unmodified Inter Variable and JetBrains Mono NL Regular binaries are admitted only with their official SIL OFL texts, copyright notices and immutable provenance. Font binaries are not under the project Apache license; portable packages must include their licenses and attribution in the checksummed inventory. No additional weights, italics, webfonts or duplicate formats are required. Old embedded font assets remain historical; active imports move forward.

## Completion

Close only after requirements are evidenced, material findings resolved, normal merge and post-main verification recorded. Follow 096 immediately. A passing compiler or source-string test cannot establish visual acceptance.

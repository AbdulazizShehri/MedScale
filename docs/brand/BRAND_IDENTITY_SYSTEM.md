# MedScale brand identity

**Status:** CANONICAL — founder-supplied mark, 2026-11-05. It supersedes Logo 12 (2026-10-19) and the Spec 095 paired-M identity for all active branding.

## Central idea

One filled rounded trapezoid with three triangular counters. The symbol is fixed. Its discipline carries into precise typography, monochrome hierarchy and whitespace. MedScale is a Clinical Intelligence OS. **Evidence first. Action second.** describes how the product operates, not a readiness claim.

## Primary logo

- **Canonical:** white `#FFFFFF` symbol and wordmark on pure black `#000000` (`assets/brand/medscale-logo-canonical.svg`).
- **Inverse:** black on white (`medscale-logo-inverse.svg`).
- **Lockups:**
  - Primary vertical lockup: `medscale-stacked-*`. The symbol is centred above the wordmark.
  - Horizontal lockup: `medscale-horizontal-*`.
  - Standalone symbol: `mark-*`.
- **Wordmark (founder-approved 2026-10-22):** exactly `MedScale`. Filled geometric letterforms from Plus Jakarta Sans at weight 590, with custom spacing. In the horizontal lockup the symbol is 1.45× the cap height and centred on it. No bold "Scale", no weight contrast, no tagline in the primary logo. The Logo 12 monoline wordmark is superseded.
- No gradients, blue, shadows, glows, embossing, 3D, outlines-of-outlines or decorative effects. The logo uses only `#000000` and `#FFFFFF`; application icons sit on the founder's near-black `#080808` tile. Product UI accents never recolour it.

Construction, sizes and misuse rules: [LOGO_SPEC.md](LOGO_SPEC.md). Provenance and trace method: [`third_party/provenance/medscale-logo-2026-11-05.md`](../../third_party/provenance/medscale-logo-2026-11-05.md).

## Applications

- **Desktop (Tauri):**
  - `Mark`, `Wordmark` and `Logo` in `apps/desktop-tauri/src/components/brand.tsx` draw the master geometry in `currentColor`.
  - They are used in the sidebar, splash, Welcome and Access, Home, and About.
  - Bundle icons: `src-tauri/icons/`.
- **README and docs:** `assets/brand/png/medscale-readme-dark.png` and `-light.png` (horizontal lockup at documentation scale), or the SVG masters.
- **Favicon:** `assets/brand/medscale-favicon.svg` (optical small master), with PNG fallbacks in `assets/brand/icons/`.
- **Legacy Slint desktop:** `crates/medscale-desktop/ui/assets/`.

## Typography and UI

Inter is the UI typeface and JetBrains Mono NL is used for exact data and metadata. Both fonts are admitted with recorded hashes in [`third_party/provenance/095-native-typography.md`](../../third_party/provenance/095-native-typography.md). The logo wordmark is outline artwork drawn from Plus Jakarta Sans (see `LOGO_SPEC.md`). Surfaces render it from path data, never as live text. Other system rules are unchanged:

- four-pixel spacing base
- thin borders
- restrained radii
- no decorative shadow or motion
- source relationships, exact states and uncertainty stay visible

See TYPOGRAPHY_SYSTEM.md and PRODUCT_UI_GRAMMAR.md.

## Voice

Use factual labels and explain uncertainty near claims. State denied, unsupported, unmeasured or unavailable exactly. No clinical promise, real PHI, invented count or comparative superiority without evidence. Technical content is in English.

## Authority and history

The Logo 12 symbol (2026-10-19) is SUPERSEDED_BY_MARK_2026_11_05, and the paired-M identity (Spec 095, 2026-09-29) was superseded by Logo 12 before it. Their provenance records, specs, evidence snapshots and closed 068–101 artifacts remain intact as history.

The symbol is a hand-fitted vector trace of the founder's raster (symbol IoU 0.9986). The 2026-10-22 wordmark is generated, not reconstructed, so it has no IoU. Rendered review does not claim pixel equivalence, and compilation is not visual acceptance.

# MedScale brand identity

**Status:** CANONICAL — founder-approved Logo 12, 2026-10-19. It supersedes the Spec 095 paired-M identity for all active branding.

## Central idea

Three overlapping, upright frames: layered evidence and records that stay separate and inspectable, joined by one continuous lower stroke. The symbol is fixed. Its discipline carries into precise typography, monochrome hierarchy and whitespace. MedScale is a Clinical Intelligence OS. **Evidence first. Action second.** describes how the product operates, not a readiness claim.

## Primary logo

- **Canonical:** white `#FFFFFF` symbol and wordmark on pure black `#000000` (`assets/brand/medscale-logo-canonical.svg`).
- **Inverse:** black on white (`medscale-logo-inverse.svg`).
- **Lockups:**
  - Primary vertical lockup: `medscale-stacked-*`. It follows the approved reference, with the symbol centred above the wordmark.
  - Horizontal lockup: `medscale-horizontal-*`.
  - Standalone symbol: `mark-*`.
- **Wordmark (founder-approved 2026-10-22):** exactly `MedScale`. Filled geometric letterforms from Plus Jakarta Sans at weight 590, with custom spacing. In the horizontal lockup the symbol is 2.05× the cap height. No bold "Scale", no weight contrast, no tagline in the primary logo. The Logo 12 monoline wordmark is superseded.
- No gradients, blue, shadows, glows, embossing, 3D, outlines-of-outlines or decorative effects. The logo uses only `#000000` and `#FFFFFF`. Product UI accents never recolour it.

Construction, sizes and misuse rules: [LOGO_SPEC.md](LOGO_SPEC.md). Provenance and reconstruction method: [`third_party/provenance/logo-12-approved-identity.md`](../../third_party/provenance/logo-12-approved-identity.md).

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

The paired-M identity (Spec 095, 2026-09-29) is SUPERSEDED_BY_LOGO_12 for active branding. Its provenance record, specs, evidence snapshots and closed 068–101 artifacts remain intact as history.

The Logo 12 symbol is a measured reconstruction of the approved raster (symbol IoU 0.925). The 2026-10-22 wordmark is generated, not reconstructed, so it has no IoU. Rendered review does not claim pixel equivalence, and compilation is not visual acceptance.

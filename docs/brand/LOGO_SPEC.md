# MedScale logo specification

**Status:** CANONICAL — FOUNDER_SUPPLIED_MARK_2026_11_05 symbol with the FOUNDER_APPROVED_WORDMARK (2026-10-22, direction A). Both are locked. Do not reinterpret or redesign either.

## Symbol construction

One filled rounded trapezoid with three triangular counters: an inverted triangle in the centre and an upright triangle on each side. It is one path with fill rule `evenodd`, not strokes. The mark is wider than tall (about 1.53 : 1). The trace keeps the founder's art as it is, including its small left-right differences, so do not mirror one side onto the other.

The path, the founder's tile and the lockup proportions are in `assets/brand/source/geometry.json`. `assets/brand/mark-black.svg` is the master. The symbol group is `id="medscale-symbol"`, and every asset reuses its path data unchanged. Provenance: [`third_party/provenance/medscale-logo-2026-11-05.md`](../../third_party/provenance/medscale-logo-2026-11-05.md).

## Wordmark

Exactly `MedScale` (`id="medscale-wordmark"`), approved by the founder on 2026-10-22 after a four-direction comparison. Provenance: [`third_party/provenance/medscale-wordmark-2026-10-22.md`](../../third_party/provenance/medscale-wordmark-2026-10-22.md).

- **Letterforms:** filled outlines drawn from Plus Jakarta Sans at weight 590, one path per letter.
- **Spacing:** the font's pair kerning, tracking −0.018 em, and −0.012 em extra before "S".
- **Artwork, not text:** every surface draws the path data. No font file is embedded or loaded to render the logo.

The earlier monoline wordmark is SUPERSEDED_BY_WORDMARK_2026_10_22 and kept as history in `evidence/brand-logo-12/`.

## Lockups and colour

- **Vertical (primary):** the filled symbol at 2.2 × the wordmark cap height, centred above the wordmark, with 0.55 × cap height between the symbol and the cap line.
- **Horizontal:** symbol height = 1.45 × the wordmark cap height, with no stroke. Gap = 0.5 × cap height. The symbol is centred on the cap height.
- **Colour:**
  - The logo uses only `#000000` and `#FFFFFF`: white on dark (canonical), black on light.
  - The one exception is the application icon tile, `#080808`, sampled from the founder's art.
  - Transparent masters carry no background.
  - The canonical and inverse files include their solid background.

## Size and clear space

- Keep clear space of at least 0.25 × lockup width around a full lockup, and at least 0.5 × symbol width around the standalone symbol.
- **Minimum symbol height:** 16 px on screen. Prefer 20 px or larger.
- **Optical masters keep the counters open on small pixel grids.** They change only how tightly the tile crops the mark, never the geometry:
  - `medscale-icon-small-dark.svg` (16–32 px; the mark fills 92% of the tile width)
  - `medscale-icon-medium-dark.svg` (48–64 px; 84%)
- **16 px slots** (favicons, title bars) are a platform-size limit. The trapezoid and its counters remain recognisable, but the bars between counters are close to one pixel wide. Use 24 px or larger wherever the platform allows.
- **Application icons:**
  - White mark on the founder's near-black `#080808` tile, radius 215 of 1024, with transparent corners. The mark keeps the founder's proportion (77.8% of the tile width) and placement.
  - `medscale-app-icon-light.svg` is the black-on-white counterpart for light window chrome.
  - macOS uses the 824/1024 rounded tile.
  - The ICO carries separately rasterized 16–256 px bitmaps.

## Misuse

Never:

- redraw, stretch, squeeze, rotate or re-slant the symbol
- straighten, mirror or re-round the trapezoid, or move, reshape or fill a counter
- outline the symbol or draw it as strokes
- change the wordmark's weight, or bold part of the word
- retype the wordmark as live text in any font, Inter included
- re-space the letters or change the lockup proportions
- add a tagline to the primary logo
- use colour, gradients, blue, shadows, glows, 3D, outlines or animation
- use medical crosses, hearts, DNA, shields, sparkles or AI stars
- place it on busy imagery without a solid field

Raster screenshots are not vector masters.

## History

- The Logo 12 three-frame symbol (2026-10-19) is SUPERSEDED_BY_MARK_2026_11_05. Its record, [`logo-12-approved-identity.md`](../../third_party/provenance/logo-12-approved-identity.md), and evidence remain intact.
- The Logo 12 monoline wordmark (2026-10-19) is SUPERSEDED_BY_WORDMARK_2026_10_22. Its reconstruction record and evidence remain intact.
The Spec 095 paired-M mark and the Spec 073 "Shelf" are SUPERSEDED_BY_LOGO_12 for active branding. Their commits, specs, provenance records and evidence remain intact.

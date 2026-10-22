# MedScale logo specification

**Status:** CANONICAL — FOUNDER_APPROVED_LOGO_12 symbol (2026-10-19) with the FOUNDER_APPROVED_WORDMARK (2026-10-22, direction A). Both are locked. Do not reinterpret or redesign either.

## Symbol construction

Three upright frames with rounded corners, sheared so their top edges rise to the right. They are drawn as one uniform stroke with round joins and butt ends. All frames overlap visibly and nothing is knocked out.

- **Frame A** (back) is closed.
- **Frames B and C** (middle and front) form one continuous open stroke:
  - It starts at the foot of C's descender, rises along C's left side, crosses C's top and descends C's right side.
  - The shared lower stroke then runs straight from C's lower-right corner to B's lower-left corner.
  - From there it rises along B's left side, crosses B's top, and ends where B's right side meets the shared lower stroke.
  - C's left side continues below the lower stroke as the descender, which is the lowest point of the symbol.

The exact parameters, in reference pixels, are in `assets/brand/source/geometry.json`:

- stroke 26.5
- corner radius 51
- top slope −0.4542; closed bottom slope −0.4767
- frame offsets B = A + (97.5, 50.5) and C = A + (196, 106.5)

`assets/brand/mark-black.svg` is the master. The symbol group is `id="medscale-frames"`, and every asset reuses its path data unchanged.

## Wordmark

Exactly `MedScale` (`id="medscale-wordmark"`), approved by the founder on 2026-10-22 after a four-direction comparison. Provenance: [`third_party/provenance/medscale-wordmark-2026-10-22.md`](../../third_party/provenance/medscale-wordmark-2026-10-22.md).

- **Letterforms:** filled outlines drawn from Plus Jakarta Sans at weight 590, one path per letter.
- **Spacing:** the font's pair kerning, tracking −0.018 em, and −0.012 em extra before "S".
- **Artwork, not text:** every surface draws the path data. No font file is embedded or loaded to render the logo.

The earlier monoline wordmark is SUPERSEDED_BY_WORDMARK_2026_10_22 and kept as history in `evidence/brand-logo-12/`.

## Lockups and colour

- **Vertical (primary):** the symbol is centred above the wordmark. The gap and proportions match the approved reference (symbol height ≈ 4.0 × wordmark height).
- **Vertical gap:** 58 reference px between the symbol and the cap height, unchanged from Logo 12.
- **Horizontal:** symbol height = 2.05 × the wordmark cap height, drawn with a 1.5× optical stroke so the frames match the letter weight. Gap = 0.5 × cap height. The symbol is centred on the cap height and nudged down 4% of its height.
- **Colour:**
  - The logo uses only `#000000` and `#FFFFFF`: white on dark (canonical), black on light.
  - Transparent masters carry no background.
  - The canonical and inverse files include their solid background.

## Size and clear space

- Keep clear space of at least 0.25 × lockup width around a full lockup, and at least 0.5 × symbol width around the standalone symbol.
- **Minimum symbol height:** 24 px on screen. Prefer 32 px or larger.
- **Optical masters keep the frames legible on small pixel grids.** They change only stroke weight and tile fill, never the geometry:
  - `medscale-icon-small-dark.svg` (16–32 px; stroke × 2.4)
  - `medscale-icon-medium-dark.svg` (48–64 px; stroke × 1.5)
  - In the desktop UI, `Mark` below 48 px uses stroke × 1.6.
- **16 px slots** (favicons, title bars) are a platform-size exception. The symbol reads as a compact frame cluster but the individual frames are not distinct. Use 24 px or larger wherever the platform allows.
- **Application icons:**
  - The symbol is centred on a solid square, at 64% of the tile height.
  - macOS uses the 824/1024 rounded tile.
  - The ICO carries separately rasterized 16–256 px bitmaps.

## Misuse

Never:

- redraw, stretch, squeeze, rotate or re-slant the symbol
- move one frame on its own, close C's descender, or separate the shared lower stroke
- fill the frames
- add a fourth frame
- change stroke weights within the symbol, change the wordmark's weight, or bold part of the word
- retype the wordmark as live text in any font, Inter included
- re-space the letters or change the lockup proportions
- add a tagline to the primary logo
- use colour, gradients, blue, shadows, glows, 3D, outlines or animation
- use medical crosses, hearts, DNA, shields, sparkles or AI stars
- place it on busy imagery without a solid field

Raster screenshots are not vector masters.

## History

- The Logo 12 monoline wordmark (2026-10-19) is SUPERSEDED_BY_WORDMARK_2026_10_22. Its reconstruction record and evidence remain intact.
The Spec 095 paired-M mark and the Spec 073 "Shelf" are SUPERSEDED_BY_LOGO_12 for active branding. Their commits, specs, provenance records and evidence remain intact.

# Logo 12 approved MedScale identity provenance

**Operation:** `USER_SUPPLIED_UI` vector reconstruction of founder-approved artwork.

**Authority:** the MedScale founder approved Logo Variant #12 on 2026-10-19 and directed that it replace the Spec 095 paired-M identity across the repository ("OFFICIAL BRAND IDENTITY REPLACEMENT").

**Supersedes for active branding:** `third_party/provenance/095-approved-medscale-identity.md` (kept unchanged as history).

## Approved references

The handoff files are visual authority only. They are cropped, upscaled rasters, not vector sources, and they do not identify a font. They are not committed as source; their digests are recorded here, and `evidence/brand-logo-12/reference-vs-reconstruction.png` shows the reference beside the reconstruction.

| Reference | SHA-256 |
|---|---|
| `MedScale_Approved_Logo_12_Handoff.zip` | `442b6941ca2b3edfa0340bfeaabd5c99386daefe9c0341349eb8ad9ace701abd` |
| `MedScale_Approved_Logo_12_Reference.png` (1304×1180, full lockup) | `cc312a097bc4e277b728a56f9266eab5113b23a6306813ef4b59c6876474c0be` |
| `MedScale_Approved_Symbol_Reference.png` | `a4600a309d448c15442474323967cdc8c6969da3b77709d037385f89915f79a9` |
| `medscale_logo_variations_grid.png` (variant 12 is lower right) | `82408032fca91efd9a09dc4f98c0001ff71be9d34bd01ec74a671f23b3e735bd` |
| `MEDSCALE_APPROVED_LOGO_HANDOFF.txt` | `d20621bcc4d127dd64d9c4a50176f1d559ed14412e8a07c45c4facae97139502` |

## Reconstruction method

The scripts in `assets/brand/source/` measure the reference PNG and rebuild it as stroked vector geometry. Each candidate is rendered in headless Microsoft Edge at the reference size and scored by intersection-over-union (IoU) against the thresholded reference. Parameters are then fitted by coordinate descent. `assets/brand/source/geometry.json` records the final parameters in reference pixels.

- **Symbol:** three sheared, rounded-corner frames drawn as a single uniform stroke (26.5 reference px) with round joins.
  - Frame A, at the back, is closed.
  - Frames B and C are one open stroke. The shared lower stroke descends from C's lower-right corner straight to B's lower-left corner. B's right side ends on that stroke, and C's left side continues below it as the descender.
  - This is the "distinctive descending overlaps and lower strokes" requirement in the handoff.
  - Top edges have slope −0.4542 and the closed bottom edge −0.4767. Corners are quadratic curves with radius 51 reference px.
  - **Symbol IoU: 0.925.**
- **Wordmark:** first-party monoline letterforms built from geometry.
  - **No font is embedded, outlined, licensed, or claimed.** The reference's typeface is not identified, and none is guessed.
  - Every letter is a centre-line path drawn with one stroke width (23 reference px), which matches the measured bar (21.5) and stem (24.5) thickness. There is no weight contrast between "Med" and "Scale".
  - The M's stems turn inward along a short top stroke before the diagonals drop to the V, as in the reference.
  - **Wordmark IoU: 0.911.**
- Residual IoU loss comes from anti-aliasing and from the reference raster's soft, upscaled edges. Measured overlays show no remaining structural difference. Pixel equivalence is not claimed.

## Asset set

- **SVG masters (`assets/brand/`):**
  - `mark-white.svg` and `mark-black.svg`: transparent symbol.
  - `medscale-wordmark-*`.
  - `medscale-stacked-*`: primary vertical lockup, reference composition.
  - `medscale-horizontal-*`.
  - `medscale-logo-canonical.svg`: white on black, canonical.
  - `medscale-logo-inverse.svg`: black on white.
  - `medscale-app-icon-dark.svg` and `medscale-app-icon-light.svg`.
  - `medscale-app-icon-macos.svg`: rounded tile on the 824/1024 grid.
  - Optical-size masters `medscale-icon-small-dark.svg` (16–32 px) and `medscale-icon-medium-dark.svg` (48–64 px).
  - `medscale-favicon.svg`.
- **PNG exports:** high-resolution transparent PNGs in `assets/brand/png/`; icon sizes 16–1024, ICO, ICNS, favicons and the apple-touch icon in `assets/brand/icons/`.
- **Tauri bundle icons:** `apps/desktop-tauri/src-tauri/icons/`. The ICO holds a separately rasterized bitmap for each of 16, 24, 32, 48, 64, 128 and 256 px.
- **Desktop UI:** `apps/desktop-tauri/src/components/brand.tsx` carries the same path data as the masters, and `crates/medscale-core/tests/brand_foundation_095.rs` enforces this.
- **Legacy Slint desktop:** copies in `crates/medscale-desktop/ui/assets/`.

## Small-size qualification

Readability was checked at 16, 24, 32, 64, 128 and 256 px (`assets/brand/png/icon-readability-sheet.png`).

- At 24 px and above the three frames read distinctly, using the optical masters below 128 px.
- At 16 px the symbol reads as a compact frame cluster, but the individual frames are not distinct. This is an explicit platform-size limitation of a three-outline symbol, not a redesign.
- Prefer slots of 24 px or larger.

## Rights

This is MedScale artwork authorized by its founder. The reconstruction contains no third-party font, image or vector data.

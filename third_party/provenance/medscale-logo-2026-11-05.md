# MedScale logo provenance (founder-supplied 2026-11-05)

**Operation:** `USER_SUPPLIED_UI` vector trace of founder-supplied artwork.

**Authority:** the MedScale founder supplied this mark on 2026-11-05 and directed that it replace the logo across the repository.

**Supersedes for active branding:** the Logo 12 three-frame symbol in [`logo-12-approved-identity.md`](logo-12-approved-identity.md), which stays unchanged as history. The wordmark is unchanged: [`medscale-wordmark-2026-10-22.md`](medscale-wordmark-2026-10-22.md) remains its authority, except that its horizontal lockup proportions no longer apply to the new, wider symbol.

## Supplied references

The PNGs are visual authority only. They are not committed as source; their digests are recorded here.

| Reference | SHA-256 |
|---|---|
| `1.png` (1024×1024 master: white mark on a near-black rounded tile) | `7d8139bc35eb34d7ad03db37d3b49636c9898a68df55b854da98b7c2698d02d0` |
| `2.png` (256×256 small version) | `f6854f4ee03e1ac21b85909679fe1047a27305ff7738e1293a80a650812eac4a` |

## Trace method

The symbol is one hand-fitted SVG path, stored in `assets/brand/source/geometry.json`: a rounded trapezoid and three sharp triangular counters, fill rule `evenodd`. Starting from points measured on `1.png`, the coordinates were refined against headless Microsoft Edge renders, with each corner kept tangent to its edges. Counter edges that are flat in the image stay flat. Nothing was mirrored, because the art is not symmetric: the top-left shoulder differs from the top-right.

- **Verification:** the final path, rendered in headless Edge at 1024×1024 on the tile colour and thresholded at 50%, has **symbol IoU 0.9986** against `1.png` thresholded the same way. Rendering `medscale-app-icon-dark.svg` itself, tile included, gives the same comparison. Pixel equivalence is not claimed.
- **Tile sampled from `1.png`:** `#080808`, corner radius 215 of 1024, transparent corners. The mark sits at offset (111.07, 247.1) in that tile at 77.8% of the tile width, and the application icons keep that placement.

## Lockups

The old horizontal rule (symbol 2.05× the cap height, 1.5× stroke, drop) was built for a tall stroked mark. The new proportions were judged on rendered comparison sheets at display and real UI sizes, on black and on white:

- **Horizontal:** symbol 1.45× the cap height, centred on the cap height, gap 0.5× the cap height.
- **Vertical:** symbol 2.2× the cap height, centred above the word, 0.55× the cap height between the symbol and the cap line.

These proportions were chosen in engineering and have not been separately reviewed by the founder.

## Small-size qualification

Readability was checked at 16, 24, 32, 64, 128 and 256 px (`assets/brand/png/icon-readability-sheet.png`). Sizes of 32 px and below use `medscale-icon-small-dark.svg`, the same path cropped tighter on the tile. At 16 px the trapezoid and its counters remain recognisable; the bars between the counters are close to one pixel wide.

## Rights

This is MedScale artwork supplied by its founder. The trace contains no third-party image or vector data, and no font is embedded in the symbol. This record does not claim trademark or other legal clearance.

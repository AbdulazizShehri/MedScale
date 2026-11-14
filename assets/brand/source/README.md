# MedScale logo source

Design tooling that regenerates the MedScale logo: the founder-supplied symbol (2026-11-05) and the founder-approved wordmark (2026-10-22). It is not part of the product build.

- `geometry.json` holds the symbol as one filled path (fill rule `evenodd`), the founder's tile, the wordmark parameters and the lockup and icon proportions. The path was hand-fitted to the founder's PNG; it is data here, and nothing regenerates or adjusts it.
- `wordmark.py` draws `MedScale` from Plus Jakarta Sans outlines (weight 590) with the font's kerning and custom spacing, and returns filled paths. See `third_party/provenance/medscale-wordmark-2026-10-22.md`.
- `build_brand.py` writes the SVG masters and `apps/desktop-tauri/src/components/brand.tsx` from `geometry.json`. The wordmark did not change with the symbol: the build checks that `wordmark.py` still produces the outlines in the committed wordmark masters, then copies those masters byte for byte. Their `<metadata>` therefore still describes the symbol of their date.
- `export.py` and `raster.py` produce the PNG, ICO and ICNS exports through headless Microsoft Edge.

The reference PNGs are founder-supplied and not committed; their SHA-256 digests and the trace's IoU are in [`third_party/provenance/medscale-logo-2026-11-05.md`](../../../third_party/provenance/medscale-logo-2026-11-05.md). Requirements: Python 3 with Pillow and fontTools, and Microsoft Edge. Set `MEDSCALE_EDGE` if Edge is installed somewhere else.

```bash
python build_brand.py out
python export.py out
```

`build_brand.py` reproduces the committed SVG masters and `brand.tsx` byte for byte. Copy the outputs into `assets/brand/`, `assets/brand/png/`, `assets/brand/icons/`, `apps/desktop-tauri/src-tauri/icons/`, `crates/medscale-desktop/ui/assets/` and `web/public/brand/`, plus `icon.ico` to `web/public/favicon.ico` and `medscale-favicon.svg` to `apps/desktop-tauri/public/favicon.svg`. The brand tests in `crates/medscale-core/tests/brand_foundation_095.rs` and `apps/desktop-tauri/src/identity.test.ts` check that all copies use the same geometry.

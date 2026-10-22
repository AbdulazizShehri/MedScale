# MedScale logo source

Design tooling that regenerates the MedScale logo: the founder-approved Logo 12 symbol (2026-10-19) and the founder-approved wordmark (2026-10-22). It is not part of the product build.

- `symbol_fit.py` holds the symbol construction: three stroked frames, with B and C interlocked. It also renders and scores candidates against the reference (IoU).
- `optimize.py` fits the parameters by coordinate descent.
- `wordmark.py` draws `MedScale` from Plus Jakarta Sans outlines (weight 590) with the font's kerning and custom spacing, and returns filled paths. See `third_party/provenance/medscale-wordmark-2026-10-22.md`.
- `build_brand.py` writes the SVG masters and `apps/desktop-tauri/src/components/brand.tsx` from `geometry.json`.
- `export.py` and `raster.py` produce the PNG, ICO and ICNS exports through headless Microsoft Edge.

The reference PNG is founder-supplied and not committed; its SHA-256 is in [`third_party/provenance/logo-12-approved-identity.md`](../../../third_party/provenance/logo-12-approved-identity.md). Requirements: Python 3 with Pillow, NumPy and fontTools, and Microsoft Edge. Set `MEDSCALE_EDGE` if Edge is installed somewhere else.

```bash
export MEDSCALE_LOGO_REFERENCE=/path/to/MedScale_Approved_Logo_12_Reference.png
python build_brand.py out
python export.py out
```

`build_brand.py` reproduces the committed SVG masters and `brand.tsx` byte for byte. Copy the outputs into `assets/brand/`, `assets/brand/png/`, `assets/brand/icons/`, `apps/desktop-tauri/src-tauri/icons/` and `crates/medscale-desktop/ui/assets/`. The brand tests in `crates/medscale-core/tests/brand_foundation_095.rs` check that all copies use the same geometry.

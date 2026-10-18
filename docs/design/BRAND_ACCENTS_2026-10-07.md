# Brand accents: sky blue and orange (founder decision, 2026-10-07)

The founder supplied a reference image and directed that its colors be used in the desktop alongside black and white. The logo geometry is unchanged and is placed more often.

| Token | Value | Role |
|---|---|---|
| `--brand-blue` / `--accent` | `#70B8C7` | interaction: active navigation, selected row and tab, filter selection, primary action, focus ring, data bars, provenance path |
| `--brand-orange` / `--signal` | `#FB905A` | attention: needs-review glyphs, the synthetic-workspace badge, the review node |
| `--brand-white` | `#FEF8F5` | primary ink and inverse surface in dark mode (warm white from the reference) |
| black | `#000000` | dock, window void (unchanged) |

## Rules

- Status meaning is still carried by glyph shape (square, circle, triangle; solid, dashed, hatched). Color only reinforces it and is never the sole signal.
- Fills keep the source colors with black text: blue 9.4:1, orange 9.2:1.
- In dark mode, the raw colors on the surface are 8.1:1 (blue) and 8.0:1 (orange).
- In light mode, text and glyph variants are darkened because the raw colors are about 2.2:1 on white:
  - blue text `#256B7B` (6.1:1);
  - orange text `#B4541F` (5.0:1);
  - review glyph `#C2612A` (4.2:1, above the 3:1 graphics minimum).
- Logo placements added: the loading and runtime-disconnected screen, the Home header, and a blue/orange brand rule under the Welcome headline and the Home mark. Mark geometry: `M0 76 50 0 100 76Z M60 76 110 0 160 76Z` (unchanged).

Implementation: `apps/desktop-tauri/src/styles/medscale.css`, section 16, layered over the shared canvas tokens so the base system stays intact. No accessibility certification is claimed. `PHYSICAL_KEYBOARD_QUALIFICATION` and `SCREEN_READER_QUALIFICATION` remain `PENDING_EXTERNAL_MEASUREMENT`.

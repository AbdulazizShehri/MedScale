# Spec 095 approved MedScale identity provenance

**Operation:** `USER_SUPPLIED_UI` vector reconstruction and licensed font artwork.

**Authority:** founder-approved images and the explicit 2026-09-29 productization request.

**Owning spec:** `specs/095-brand-foundation`.

**Repository base:** `1e2b7d94e970256b38bda15fa91f62bc397e825a`.

## Approved references

The supplied images remain visual authority; raster screenshots are not production logo assets.

| Reference | SHA-256 |
|---|---|
| `1a104ea0-a6c4-44ce-86b5-b48d058ca016.png` | `d864397d7d2112dacd1815b52ea4b2ce4a9070886fd9aa7e46a24db7db5c047c` |
| `ebf2c641-15bc-475c-bad6-30ee712cd41d.png` | `bfb02112f4b442612214aeadd22a2741dd7e32677ce5697590423344c5bcfaac` |

This is MedScale artwork authorized by its founder. Pentagram and other brands are reference-only research; none of their artwork, proprietary typefaces or layouts enters these assets.

## Canonical construction

Triangle side `s=100`; height `h=sqrt(3)*50=86.6025403784`; horizontal offset `d=60`.

- Left equilateral primitive: `(0,h), (50,0), (100,h)`.
- Right equilateral primitive: `(60,h), (110,0), (160,h)`.
- Remove their common overlap from both primitives.
- Shared center contact: `(80,0.6h)`.
- Lower negative space: `(80,0.6h), (60,h), (100,h)`.
- Tight symbol bounds: `160 x h`; SVG coordinates use six decimal places.

The two resulting four-point filled lobes preserve both peaks, the center contact and intentional lower void. Separate full triangles would change the supplied silhouette.

**Reconstruction limitation:** the raster panels illustrate approximately 2:1 proportions and are not metrically exact equilateral vectors. This strict construction has aspect ratio approximately `1.847521:1`, with contact near 60 percent of mark height. It is a mathematical reconstruction, not a claim of pixel-identical tracing. Reference-overlay and native rendering remain separate visual qualification steps.

## Size, clear space and lockups

- `h` is visible mark height, excluding padding.
- Standalone clear space is at least `h/2` on every side; ordinary digital placements require at least `16px` visible mark height.
- Tight SVGs exclude padding; consumers supply clear space in layout.
- Horizontal internal gap equals `h/2` for its 40-unit-high mark.
- Stacked mark width is 74 percent of wordmark width; internal vertical gap is `0.18h`, following the approved arrangement.
- A complete lockup receives `h/2` clear space outside its composite bounds. Its internal relationship is fixed artwork.
- Application variants use a 128-unit square and 40-unit visible mark height. Horizontal margins are approximately `27.049583`, vertical margins `44`; both exceed the required 20 units. An operating-system mask applies to the square surface.
- Favicon master: 64-unit square, 22-unit visible mark height, at least 11 units of padding.

**Small favicon exception:** a visible 16px-high mark with h/2 clear space needs a square container at least approximately `45.561px` wide. A 16px square favicon cannot meet both rules. Those platform slots retain the complete mark with an explicit exception below the ordinary minimum; optical qualification remains required. The exception does not apply to ordinary headers or application marks.

## Outlined Inter wordmark

| Field | Bound value |
|---|---|
| Upstream | https://github.com/rsms/inter |
| Immutable revision | `e3a3d4c57d5ecc01453a575621882a384c1995a3` |
| Original path | `docs/font-files/InterVariable.ttf` |
| Exact source | https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/docs/font-files/InterVariable.ttf |
| Existing admitted local source | `assets/brand/fonts/InterVariable.ttf` |
| SHA-256 | `4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf` |
| License | SIL OFL 1.1; `assets/brand/fonts/Inter-OFL.txt` |
| Axes | `wght=600`, `opsz=14` |
| Layout | Font GPOS kerning, `-0.025em` tracking between glyphs |
| Output | 100 units per em, tight normalized bounds, three-decimal path coordinates |
| Exporter | Existing installed `opentype.js 2.0.0` bundled with Cursor |
| Method | In-memory transformed glyph outlines and advance widths using gvar/HVAR variable-font support |
| Variation check | Weight 400 and weight 600 M outline data differ |

The exporter is a development tool only. No exporter code, added dependency, generated static font or additional font binary is committed. Font redistribution and the checksummed license inventory belong to the separate Spec 095 font-admission record. The OFL states that its font-license requirement does not apply to documents created using the font software. These outlined artwork assets have no runtime font lookup or network dependency.

## Local targets and maintenance

- `assets/brand/mark-{black,white}.svg`: tight standalone symbol and exact inversion.
- `assets/brand/medscale-{wordmark,horizontal,stacked}-{black,white}.svg`: outlined wordmark and fixed lockups.
- `assets/brand/medscale-favicon.svg`, `medscale-app-icon-{light,dark}.svg`: platform masters.
- `assets/brand/medscale-identity-proof.svg`: scalable review sheet; neutral guide lines and labels are not logo artwork.
- `crates/medscale-desktop/ui/assets/medscale-mark.svg`, `medscale-mark-white.svg`: native relative-import copies.
- `crates/medscale-desktop/ui/assets/medscale-app-icon.svg`, `medscale-app-icon-light.svg`: native icon copies.

Native copies support Slint's relative imports. Raster exports are omitted until an actual packaging consumer requires them. Asset checks should measure finite bounds, equilateral construction, both peaks, center contact, open lower void, exact inversion path identity, application padding and XML validity. They do not establish native visual acceptance or small favicon legibility.

The assets are presentation-only, contain no patient data or external resource references, and create no storage, permission, broker or clinical authority. Updates regenerate all targets from the same canonical geometry and immutable font source, retaining axes and layout parameters. An exporter may be replaced when equivalent measured artwork is produced. Removal requires replacing every native and packaging consumer; future raster exports are derived from these vectors and limited to actual platform requirements.

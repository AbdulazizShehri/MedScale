"""Generate the MedScale SVG masters and the desktop brand component.

Symbol: one filled path with three counters, stored in geometry.json. It is a
hand-fitted vector trace of the founder-supplied mark of 2026-11-05 (see
third_party/provenance/medscale-logo-2026-11-05.md). Every asset places the same
path data with a translate/scale transform, so the geometry is byte-identical
across lockups, icons and inverses. Nothing here redraws or adjusts the mark.

Wordmark: filled outlines from wordmark.wordmark_paths (founder-approved
2026-10-22). The committed wordmark masters are copied through untouched; the
generated outlines must match them exactly or the build stops.
"""
import json
import re
import sys
from pathlib import Path

from wordmark import DEFAULT, wordmark_paths

HERE = Path(__file__).resolve().parent
BRAND = HERE.parent
OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "out")
OUT.mkdir(parents=True, exist_ok=True)
GEOMETRY = json.loads((HERE / "geometry.json").read_text(encoding="utf-8"))

SYMBOL = GEOMETRY["symbol"]["path"]
SW_, SH_ = GEOMETRY["symbol"]["box"]
TILE = GEOMETRY["tile"]
CAP = DEFAULT["cap"]

WM, WW_, WH_, CAP_TOP = wordmark_paths(DEFAULT)
if [WW_, WH_] != GEOMETRY["wordmark_box"] or CAP_TOP != GEOMETRY["wordmark_cap_top"]:
    raise SystemExit("wordmark.py box differs from geometry.json")
print("symbol", SW_, SH_, "wordmark", WW_, WH_)

STATEMENT = (
    "MedScale identity. Symbol: founder-supplied mark of 2026-11-05, first-party vector trace as one "
    "filled path. Wordmark: founder-approved 2026-10-22, filled outlines drawn from Plus Jakarta Sans "
    "(SIL OFL 1.1) with custom spacing. No font file is embedded."
)
SHAPE = "One filled rounded trapezoid with three triangular counters."


def symbol_group(color, tx, ty, s):
    return (
        f'<g id="medscale-symbol" transform="translate({tx:.3f} {ty:.3f}) scale({s:.6f})" fill="{color}" '
        f'fill-rule="evenodd"><path d="{SYMBOL}"/></g>'
    )


def wordmark_group(color, tx, ty, s):
    paths = "".join(f'<path d="{d}"/>' for d in WM)
    return (
        f'<g id="medscale-wordmark" transform="translate({tx:.3f} {ty:.3f}) scale({s:.6f})" fill="{color}">{paths}</g>'
    )


def doc(w, h, title, desc, body, bg=None):
    rect = f'<rect width="{w:g}" height="{h:g}" fill="{bg}"/>' if bg else ""
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:g} {h:g}" role="img" aria-labelledby="title description">\n'
        f'  <title id="title">{title}</title>\n'
        f'  <desc id="description">{desc}</desc>\n'
        f"  <metadata>{STATEMENT}</metadata>\n"
        f"  {rect}{body}\n</svg>\n"
    )


FG = {"white": "#FFFFFF", "black": "#000000"}
BG = {"white": "#000000", "black": "#FFFFFF"}
files = {}

# Symbol masters (transparent), padded by nothing: the artwork bounds are the viewBox.
for c in FG:
    files[f"mark-{c}.svg"] = doc(SW_, SH_, f"MedScale symbol - {c}", SHAPE, symbol_group(FG[c], 0, 0, 1))

# Stacked (vertical) lockup: symbol centered above the wordmark.
ST = GEOMETRY["stacked_lockup"]
sts = ST["symbol_to_cap"] * CAP / SH_
stsw, stsh = SW_ * sts, SH_ * sts
gap = ST["gap_to_cap"] * CAP - CAP_TOP
st_w = max(stsw, WW_)
st_h = stsh + gap + WH_
for c in FG:
    body = symbol_group(FG[c], (st_w - stsw) / 2, 0, sts) + wordmark_group(FG[c], (st_w - WW_) / 2, stsh + gap, 1)
    files[f"medscale-stacked-{c}.svg"] = doc(
        round(st_w, 2), round(st_h, 2), f"MedScale vertical lockup - {c}",
        f"Vertical lockup: the filled symbol at {ST['symbol_to_cap']} times the cap height, centered above the wordmark.",
        body)

# Horizontal lockup: the filled symbol centered on the wordmark's cap height.
HZ = GEOMETRY["horizontal_lockup"]
H_RATIO, H_GAP = HZ["symbol_to_cap"], HZ["gap_to_cap"]
hs = H_RATIO * CAP / SH_
hsw, hsh = SW_ * hs, SH_ * hs
hgap = H_GAP * CAP
hw = hsw + hgap + WW_
cap_y = (hsh - CAP) / 2  # y of the cap top, symbol top at 0
for c in FG:
    body = symbol_group(FG[c], 0, 0, hs) + wordmark_group(FG[c], hsw + hgap, cap_y - CAP_TOP, 1)
    files[f"medscale-horizontal-{c}.svg"] = doc(
        round(hw, 2), round(hsh, 2), f"MedScale horizontal lockup - {c}",
        f"Horizontal lockup: the filled symbol at {H_RATIO} times the cap height, centered on the cap height, left of the "
        f"wordmark, with a gap of {H_GAP} times the cap height.",
        body)

# Canonical presentations on their backgrounds (white-on-black is canonical).
for c in FG:
    pad = 0.25 * st_w
    w, h = st_w + 2 * pad, st_h + 2 * pad
    body = symbol_group(FG[c], pad + (st_w - stsw) / 2, pad, sts) + wordmark_group(FG[c], pad + (st_w - WW_) / 2, pad + stsh + gap, 1)
    name = "canonical" if c == "white" else "inverse"
    files[f"medscale-logo-{name}.svg"] = doc(
        round(w, 2), round(h, 2), f"MedScale logo - {'white on black (canonical)' if c == 'white' else 'black on white (inverse)'}",
        "Vertical lockup on its approved solid background.", body, BG[c])


# Application icons. The founder's tile: 1024 units with rounded corners and transparent corners,
# the mark at the founder's own scale and offset. `width_fraction` tightens the crop around the same
# geometry (same relative offset from the tile centre) for small pixel grids.
def placement(width_fraction, size=1024):
    s = width_fraction * size / SW_
    ox, oy = TILE["symbol_offset"]
    cx = size / 2 + (ox + SW_ / 2 - TILE["size"] / 2) * s
    cy = size / 2 + (oy + SH_ / 2 - TILE["size"] / 2) * s
    return cx - SW_ * s / 2, cy - SH_ * s / 2, s


def tile_rect(fill):
    return f'<rect width="{TILE["size"]}" height="{TILE["size"]}" rx="{TILE["radius"]}" fill="{fill}"/>'


FOUNDER_FRACTION = SW_ / TILE["size"]


def icon(title, desc, ink, tile, width_fraction=FOUNDER_FRACTION):
    return doc(1024, 1024, title, desc, tile_rect(tile) + symbol_group(ink, *placement(width_fraction)))


files["medscale-app-icon-dark.svg"] = icon(
    "MedScale application icon - dark",
    "The white symbol on the near-black rounded tile, at the founder-supplied proportion, with transparent corners.",
    "#FFFFFF", TILE["color"])
files["medscale-app-icon-light.svg"] = icon(
    "MedScale application icon - light",
    "The black symbol on a white rounded tile, at the founder-supplied proportion, with transparent corners.",
    "#000000", "#FFFFFF")
# Optical size masters: the same path, cropped tighter so the counters stay open on low-resolution
# pixel grids (small: 16-32 px, medium: 48-64 px). The path itself never changes.
OPTICAL = GEOMETRY["optical_icons"]
files["medscale-icon-small-dark.svg"] = icon(
    "MedScale small-size icon - dark",
    "Optical small master (16 to 32 px): the same symbol path on the near-black rounded tile with a tighter crop.",
    "#FFFFFF", TILE["color"], OPTICAL["small_width_fraction"])
files["medscale-icon-medium-dark.svg"] = icon(
    "MedScale medium-size icon - dark",
    "Optical medium master (48 to 64 px): the same symbol path on the near-black rounded tile with a tighter crop.",
    "#FFFFFF", TILE["color"], OPTICAL["medium_width_fraction"])
files["medscale-favicon.svg"] = files["medscale-icon-small-dark.svg"].replace(
    "MedScale small-size icon - dark", "MedScale favicon")

# macOS application icon: Apple's 824/1024 rounded tile on a transparent canvas, with the mark at the
# same proportion of the tile as the founder's art.
MAC = 824 / TILE["size"]
_ox, _oy = TILE["symbol_offset"]
files["medscale-app-icon-macos.svg"] = doc(
    1024, 1024, "MedScale application icon - macOS",
    "The white symbol on a near-black rounded tile following the macOS icon grid (824 of 1024 units).",
    f'<rect x="100" y="100" width="824" height="824" rx="185" fill="{TILE["color"]}"/>'
    + symbol_group("#FFFFFF", 100 + _ox * MAC, 100 + _oy * MAC, MAC))

for name, text in files.items():
    (OUT / name).write_text(text, encoding="utf-8", newline="\n")

# The wordmark masters do not change: copy their bytes, after proving the outlines still match.
for c in FG:
    name = f"medscale-wordmark-{c}.svg"
    master = (BRAND / name).read_bytes()
    if re.findall(r'<path d="([^"]+)"', master.decode("utf-8")) != WM:
        raise SystemExit(f"wordmark.py output differs from the committed {name}")
    if (OUT / name).resolve() != (BRAND / name).resolve():
        (OUT / name).write_bytes(master)

with open(OUT / "geometry.json", "w", encoding="utf-8", newline="\r\n") as handle:
    handle.write(json.dumps(GEOMETRY, indent=1))
print("wrote", len(files) + 2, "files to", OUT)

# Desktop brand component: the same normalized path data, drawn with currentColor.
tsx = f'''// MedScale identity: the founder-supplied symbol of 2026-11-05 and the
// founder-approved wordmark of 2026-10-22. Generated from the brand
// masters in assets/brand; the path data must stay identical to them
// (crates/medscale-core/tests/brand_foundation_095.rs enforces this).
// The wordmark is filled outline artwork; no font is loaded to draw it.

const SYMBOL = {json.dumps(SYMBOL)};
const WORDMARK = {json.dumps(WM, indent=2)};
const SYMBOL_BOX = [{SW_}, {SH_}] as const;
const WORDMARK_BOX = [{WW_}, {WH_}] as const;

/** The filled symbol at a given height; it is wider than it is tall. */
export function Mark({{ height = 22, className, title }}: {{ height?: number; className?: string; title?: string }}) {{
  const [w, h] = SYMBOL_BOX;
  return (
    <svg height={{height}} width={{Math.round((height * w) / h)}} viewBox={{`0 0 ${{w}} ${{h}}`}} className={{className}}
      role={{title ? "img" : undefined}} aria-label={{title}} aria-hidden={{title ? undefined : true}}>
      <path d={{SYMBOL}} fill="currentColor" fillRule="evenodd" />
    </svg>
  );
}}

/** The MedScale wordmark at a given box height (overshoots included). */
export function Wordmark({{ height = 16, className, title = "MedScale" }}: {{ height?: number; className?: string; title?: string }}) {{
  const [w, h] = WORDMARK_BOX;
  return (
    <svg height={{height}} width={{Math.round((height * w) / h)}} viewBox={{`0 0 ${{w}} ${{h}}`}} className={{className}} role="img" aria-label={{title}}>
      <g fill="currentColor">
        {{WORDMARK.map((d) => <path key={{d}} d={{d}} />)}}
      </g>
    </svg>
  );
}}

/** Horizontal lockup at a given symbol height, with the approved proportions. */
export function Logo({{ height = 22, className }}: {{ height?: number; className?: string }}) {{
  const cap = height / {H_RATIO};
  return (
    <span className={{className}} style={{{{ display: "inline-flex", alignItems: "flex-start", gap: cap * {H_GAP} }}}}>
      <Mark height={{height}} />
      <span style={{{{ marginTop: height * {(cap_y - CAP_TOP) / hsh:.4f} }}}}><Wordmark height={{cap * {WH_ / CAP:.4f}}} /></span>
    </span>
  );
}}
'''
(OUT / "brand.tsx").write_text(tsx, encoding="utf-8", newline="\n")
print("wrote brand.tsx")

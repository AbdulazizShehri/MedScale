"""Generate the MedScale Logo 12 SVG masters from the fitted geometry.

Symbol: three stroked, sheared rounded frames (symbol_fit.symbol_paths).
Wordmark: filled outlines from wordmark.wordmark_paths (founder-approved 2026-10-22).
Both are normalized so each artwork's own geometry starts at (0, 0); every
asset places the same path data with a translate/scale transform, so the
geometry is byte-identical across lockups, icons and inverses.
"""
import json
import re
import sys
from pathlib import Path

import numpy as np

from symbol_fit import render, svg as sym_svg, symbol_paths
from wordmark import DEFAULT, wordmark_paths

OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "out")
OUT.mkdir(parents=True, exist_ok=True)
Q = (json.loads(Path(sys.argv[2]).read_text()) if len(sys.argv) > 2
     else json.loads(Path(__file__).with_name("geometry.json").read_text())["symbol_params_reference_px"])
SYM_SW = Q["sw"]
CAP = DEFAULT["cap"]


def shift(d, dx, dy):
    """Translate an absolute M/L/Q path (coordinate pairs only) by (-dx, -dy)."""
    out, is_x = "", True
    for t in re.findall(r"[A-Za-z]|-?\d+(?:\.\d+)?", d):
        if t.isalpha():
            out += t
            is_x = True
            continue
        v = f"{float(t) - (dx if is_x else dy):.2f}".rstrip("0").rstrip(".")
        is_x = not is_x
        out += v if out[-1].isalpha() else " " + v
    return out


# Measure the true rendered bounding boxes (stroke included) from the reference-frame render.
def bbox(mask):
    ys, xs = np.nonzero(mask)
    return xs.min(), ys.min(), xs.max() + 1, ys.max() + 1


sym_mask = render(sym_svg(Q), "bbox_sym")
sx0, sy0, sx1, sy1 = bbox(sym_mask)
SYM = [shift(d, sx0, sy0) for d in symbol_paths(Q)]
# The wordmark comes out of wordmark.py already normalized; its box is exact, not rasterized.
WM, WW_, WH_, CAP_TOP = wordmark_paths(DEFAULT)
SW_, SH_ = int(sx1 - sx0), int(sy1 - sy0)
print("symbol", SW_, SH_, "wordmark", WW_, WH_)

STATEMENT = (
    "MedScale identity. Symbol: Logo 12 three-frame mark, founder-approved 2026-10-19, first-party vector "
    "reconstruction. Wordmark: founder-approved 2026-10-22, filled outlines drawn from Plus Jakarta Sans "
    "(SIL OFL 1.1) with custom spacing. No font file is embedded."
)


def symbol_group(color, tx, ty, s, sw=SYM_SW, gid="medscale-frames"):
    paths = "".join(f'<path d="{d}"/>' for d in SYM)
    return (
        f'<g id="{gid}" transform="translate({tx:.3f} {ty:.3f}) scale({s:.6f})" fill="none" stroke="{color}" '
        f'stroke-width="{sw}" stroke-linejoin="round" stroke-linecap="butt">{paths}</g>'
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
    files[f"mark-{c}.svg"] = doc(
        SW_, SH_, f"MedScale symbol - {c}",
        "Three overlapping rounded frames; the front frame's left side descends below its open lower stroke.",
        symbol_group(FG[c], 0, 0, 1))

# Wordmark masters.
for c in FG:
    files[f"medscale-wordmark-{c}.svg"] = doc(
        WW_, WH_, f"MedScale wordmark - {c}",
        "MedScale in filled geometric letterforms with custom spacing.",
        wordmark_group(FG[c], 0, 0, 1))

# Stacked (vertical) lockup: symbol centered above the wordmark. The gap keeps the Logo 12
# reference spacing (58 reference px between the symbol and the cap height).
gap = 58 - CAP_TOP
st_w = max(SW_, WW_)
st_h = SH_ + gap + WH_
for c in FG:
    body = symbol_group(FG[c], (st_w - SW_) / 2, 0, 1) + wordmark_group(FG[c], (st_w - WW_) / 2, SH_ + gap, 1)
    files[f"medscale-stacked-{c}.svg"] = doc(
        st_w, st_h, f"MedScale vertical lockup - {c}",
        "Primary vertical lockup: the symbol centered above the wordmark, matching the approved reference proportions.",
        body)

# Horizontal lockup (founder-approved 2026-10-22): symbol 2.05x the cap height with a 1.5x optical
# stroke so the frames match the letter weight; gap 0.5x the cap height. The symbol is centered on
# the cap height and nudged down 4% of its height, because the descender reads as weight below.
H_RATIO, H_STROKE, H_GAP, H_DROP = 2.05, 1.5, 0.5, 0.04
hs = H_RATIO * CAP / SH_
hsw, hsh = SW_ * hs, SH_ * hs
hgap = H_GAP * CAP
hw = hsw + hgap + WW_
cap_y = (hsh - CAP) / 2 - H_DROP * hsh  # y of the cap top, symbol top at 0
for c in FG:
    body = symbol_group(FG[c], 0, 0, hs, sw=round(SYM_SW * H_STROKE, 2)) + wordmark_group(FG[c], hsw + hgap, cap_y - CAP_TOP, 1)
    files[f"medscale-horizontal-{c}.svg"] = doc(
        hw, hsh, f"MedScale horizontal lockup - {c}",
        "Horizontal lockup: the symbol at 2.05 times the cap height, left of the wordmark, with a gap of half the cap height.",
        body)

# Canonical presentations on their backgrounds (white-on-black is canonical).
for c in FG:
    pad = 0.25 * st_w
    w, h = st_w + 2 * pad, st_h + 2 * pad
    body = symbol_group(FG[c], pad + (st_w - SW_) / 2, pad, 1) + wordmark_group(FG[c], pad + (st_w - WW_) / 2, pad + SH_ + gap, 1)
    name = "canonical" if c == "white" else "inverse"
    files[f"medscale-logo-{name}.svg"] = doc(
        w, h, f"MedScale logo - {'white on black (canonical)' if c == 'white' else 'black on white (inverse)'}",
        "Vertical lockup on its approved solid background.", body, BG[c])


# Application icon: symbol centered on a 1024 square; symbol height 64 percent of the tile.
def icon(c, size=1024, frac=0.64, sw=SYM_SW):
    s = frac * size / SH_
    tx, ty = (size - SW_ * s) / 2, (size - SH_ * s) / 2
    return doc(size, size, f"MedScale application icon - {'dark' if c == 'white' else 'light'}",
               "The approved symbol centered on a solid square. The operating system may apply its own mask.",
               symbol_group(FG[c], tx, ty, s, sw), BG[c])


files["medscale-app-icon-dark.svg"] = icon("white")
files["medscale-app-icon-light.svg"] = icon("black")
# Optical size masters: same geometry, heavier strokes and a larger fill of the tile so the frames
# stay distinguishable on low-resolution pixel grids (small: 16-32 px, medium: 48-64 px).
def optical(name, label, sizes, frac, k):
    text = icon("white", frac=frac, sw=round(SYM_SW * k, 1))
    files[name] = text.replace("MedScale application icon - dark", f"MedScale {label} icon - dark").replace(
        "The approved symbol centered", f"Optical {label} master ({sizes}): the approved geometry with a heavier stroke, centered")


optical("medscale-icon-small-dark.svg", "small-size", "16 to 32 px", 0.9, 2.4)
optical("medscale-icon-medium-dark.svg", "medium-size", "48 to 64 px", 0.76, 1.5)
# macOS application icon: Apple's 824/1024 rounded tile on a transparent canvas.
_s = 0.62 * 824 / SH_
files["medscale-app-icon-macos.svg"] = doc(
    1024, 1024, "MedScale application icon - macOS",
    "The approved symbol centered on a black rounded tile following the macOS icon grid (824 of 1024 units).",
    '<rect x="100" y="100" width="824" height="824" rx="185" fill="#000000"/>'
    + symbol_group("#FFFFFF", (1024 - SW_ * _s) / 2, (1024 - SH_ * _s) / 2, _s))
files["medscale-favicon.svg"] = files["medscale-icon-small-dark.svg"].replace(
    "MedScale small-size icon - dark", "MedScale favicon")

for name, text in files.items():
    (OUT / name).write_text(text, encoding="utf-8", newline="\n")
(OUT / "geometry.json").write_text(json.dumps({
    "symbol_params_reference_px": Q, "wordmark_params": DEFAULT,
    "symbol_bbox_reference_px": [int(sx0), int(sy0), int(sx1), int(sy1)],
    "wordmark_box": [WW_, WH_], "wordmark_cap_top": CAP_TOP,
    "horizontal_lockup": {"symbol_to_cap": H_RATIO, "symbol_stroke": H_STROKE, "gap_to_cap": H_GAP, "symbol_drop": H_DROP},
}, indent=1), encoding="utf-8")
print("wrote", len(files), "files to", OUT)

# Desktop brand component: the same normalized path data, drawn with currentColor.
SMALL_SW = round(SYM_SW * 1.6, 1)  # UI marks below 48 px (no tile)
tsx = f'''// MedScale identity: Logo 12 symbol (founder-approved 2026-10-19) and the
// founder-approved wordmark of 2026-10-22. Generated from the brand
// masters in assets/brand; the path data must stay identical to them
// (crates/medscale-core/tests/brand_foundation_095.rs enforces this).
// The wordmark is filled outline artwork; no font is loaded to draw it.

const FRAMES = {json.dumps(SYM, indent=2)};
const WORDMARK = {json.dumps(WM, indent=2)};
const SYMBOL_BOX = [{SW_}, {SH_}] as const;
const WORDMARK_BOX = [{WW_}, {WH_}] as const;

/** The three-frame symbol. Below 48 px it uses the small-size optical stroke. */
export function Mark({{ height = 32, className, title, weight }}: {{ height?: number; className?: string; title?: string; weight?: number }}) {{
  const [w, h] = SYMBOL_BOX;
  return (
    <svg height={{height}} width={{Math.round((height * w) / h)}} viewBox={{`0 0 ${{w}} ${{h}}`}} className={{className}}
      role={{title ? "img" : undefined}} aria-label={{title}} aria-hidden={{title ? undefined : true}}>
      <g fill="none" stroke="currentColor" strokeWidth={{weight ? {SYM_SW} * weight : height < 48 ? {SMALL_SW} : {SYM_SW}}} strokeLinejoin="round" strokeLinecap="butt">
        {{FRAMES.map((d) => <path key={{d}} d={{d}} />)}}
      </g>
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
export function Logo({{ height = 32, className }}: {{ height?: number; className?: string }}) {{
  const cap = height / {H_RATIO};
  return (
    <span className={{className}} style={{{{ display: "inline-flex", alignItems: "flex-start", gap: cap * {H_GAP} }}}}>
      <Mark height={{height}} weight={{{H_STROKE}}} />
      <span style={{{{ marginTop: height * {(cap_y - CAP_TOP) / hsh:.4f} }}}}><Wordmark height={{cap * {WH_ / CAP:.4f}}} /></span>
    </span>
  );
}}
'''
(OUT / "brand.tsx").write_text(tsx, encoding="utf-8", newline="\n")
print("wrote brand.tsx")

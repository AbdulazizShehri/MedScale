"""MedScale wordmark (founder-approved 2026-10-22, direction A).

"MedScale" is drawn from Plus Jakarta Sans outlines (SIL OFL 1.1, instanced at
weight 590), with the font's own pair kerning, tracking -0.018 em and a tighter
"d"-"S" pair. The result is filled vector artwork: one path per letter. No font
file is embedded in or referenced by any brand asset.
"""
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

FONT = Path(__file__).resolve().parents[1] / "fonts" / "PlusJakartaSans-Variable.ttf"

DEFAULT = {
    "word": "MedScale",
    "font": "assets/brand/fonts/PlusJakartaSans-Variable.ttf",
    "instance": {"wght": 590},
    # Letter spacing in em, plus extra pair spacing (em) applied before the given letter index.
    "tracking": -0.018,
    "kern": {"3": -0.012},
    # Output units: cap height (baseline to flat cap top) in reference px.
    "cap": 180,
}


def _num(v):
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def _pair_kern(font, left, right):
    """The font's GPOS pair adjustment (XAdvance) for two glyphs, in font units."""
    if "GPOS" not in font:
        return 0
    for lookup in font["GPOS"].table.LookupList.Lookup:
        for sub in lookup.SubTable:
            if lookup.LookupType == 9:
                sub = sub.ExtSubTable
            if type(sub).__name__ != "PairPos":
                continue
            glyphs = sub.Coverage.glyphs
            if left not in glyphs:
                continue
            if sub.Format == 1:
                for pair in sub.PairSet[glyphs.index(left)].PairValueRecord:
                    if pair.SecondGlyph == right and pair.Value1 is not None:
                        return getattr(pair.Value1, "XAdvance", 0) or 0
            else:
                c1 = sub.ClassDef1.classDefs.get(left, 0)
                c2 = sub.ClassDef2.classDefs.get(right, 0)
                value = sub.Class1Record[c1].Class2Record[c2].Value1
                if value is not None and getattr(value, "XAdvance", 0):
                    return value.XAdvance
    return 0


def wordmark_paths(g=DEFAULT):
    """Return (paths, width, height, cap_top): one filled path per letter, normalized to (0, 0).

    cap_top is the distance from the box top to the flat cap height (round
    overshoots sit above it).
    """
    font = instancer.instantiateVariableFont(TTFont(FONT), g["instance"])
    upm = font["head"].unitsPerEm
    cap_units = font["OS/2"].sCapHeight
    s = g["cap"] / cap_units
    cmap, glyphs = font.getBestCmap(), font.getGlyphSet()
    names = [cmap[ord(ch)] for ch in g["word"]]
    letters, x = [], 0.0
    for i, name in enumerate(names):
        if i:
            x += _pair_kern(font, names[i - 1], name)
        x += g["kern"].get(str(i), 0) * upm
        rec = RecordingPen()
        glyphs[name].draw(rec)
        letters.append((x, rec.value))
        x += glyphs[name].width + g["tracking"] * upm
    bounds = BoundsPen(glyphs)
    for x, rec in letters:
        pen = TransformPen(bounds, (s, 0, 0, -s, x * s, cap_units * s))
        for op, args in rec:
            getattr(pen, op)(*args)
    x0, y0, x1, y1 = bounds.bounds
    paths = []
    for x, rec in letters:
        svg = SVGPathPen(glyphs, ntos=_num)
        pen = TransformPen(svg, (s, 0, 0, -s, x * s - x0, cap_units * s - y0))
        for op, args in rec:
            getattr(pen, op)(*args)
        paths.append(svg.getCommands())
    return paths, round(x1 - x0, 2), round(y1 - y0, 2), round(-y0, 2)

"""MedScale wordmark (founder amendment 2026-10-22).

The word is drawn from Inter Display outlines (the repository's admitted
Inter Variable, opsz 32, weight 480), spaced by hand, then modified: the flat
ascender tops of "d" and "l" are cut at the symbol's frame slope so the word
rises to the right like the frames do. The result is filled vector artwork.
No font file is embedded in or referenced by any brand asset.
"""
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

FONT = Path(__file__).resolve().parents[1] / "fonts" / "InterVariable.ttf"

DEFAULT = {
    "word": "MedScale",
    "font": "assets/brand/fonts/InterVariable.ttf",
    "instance": {"opsz": 32, "wght": 480},
    # Letter spacing in em; extra pair spacing (em) applied before the given letter index.
    "tracking": -0.02,
    "kern": {"1": -0.008, "4": -0.01},
    # Ascender tops cut at the Logo 12 frame top slope (geometry.json "ts").
    "cut": ["d", "l"],
    "cut_slope": 0.4542,
    # Output units: cap height (baseline to flat cap top) in reference px.
    "cap": 180,
}


def _num(v):
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def _cut(rec, slope):
    """Lower the left point of the flat ascender top so the edge rises to the right."""
    pts = [(i, v[1][-1]) for i, v in enumerate(rec) if v[0] in ("moveTo", "lineTo")]
    top = max(p[1] for _, p in pts)
    tops = sorted([(i, p) for i, p in pts if abs(p[1] - top) < 0.5], key=lambda t: t[1][0])
    assert len(tops) == 2, f"expected one flat ascender top, found {tops}"
    (il, pl), (_, pr) = tops
    rec = list(rec)
    rec[il] = (rec[il][0], ((pl[0], top - slope * (pr[0] - pl[0])),))
    return rec


def wordmark_paths(g=DEFAULT):
    """Return (paths, width, height, cap_top): one filled path per letter, normalized to (0, 0).

    cap_top is the distance from the box top to the flat cap height (the round
    overshoot of S sits above it).
    """
    font = instancer.instantiateVariableFont(TTFont(FONT), g["instance"])
    upm = font["head"].unitsPerEm
    cap_units = font["OS/2"].sCapHeight
    s = g["cap"] / cap_units
    cmap, glyphs = font.getBestCmap(), font.getGlyphSet()
    letters, x = [], 0.0
    for i, ch in enumerate(g["word"]):
        x += g["kern"].get(str(i), 0) * upm
        name = cmap[ord(ch)]
        rec = RecordingPen()
        glyphs[name].draw(rec)
        rec = rec.value
        if ch in g["cut"]:
            rec = _cut(rec, g["cut_slope"])
        letters.append((x, rec))
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

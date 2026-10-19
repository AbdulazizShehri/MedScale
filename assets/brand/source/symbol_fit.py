"""Fit the approved Variant 12 symbol as stroked vector geometry.

Renders candidate SVGs with headless Edge at the reference size and scores
them by intersection-over-union with the thresholded reference PNG.
Local design tooling only; not part of the product.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
# Render work directory, and the founder-supplied reference PNG (SHA-256 in
# third_party/provenance/logo-12-approved-identity.md; not committed).
HERE = Path(os.environ.get("MEDSCALE_LOGO_WORK", "logo-work"))
HERE.mkdir(parents=True, exist_ok=True)
REF = np.array(Image.open(os.environ["MEDSCALE_LOGO_REFERENCE"]).convert("L")) > 128
W, H = 1304, 1180
SLOPE = -0.4642


def corner(path, p_prev, p, p_next, r):
    """Line toward corner p, then a quadratic arc with control p."""
    def toward(a, b, d):
        v = np.array(b) - np.array(a)
        n = np.linalg.norm(v)
        return np.array(b) - v / n * d
    a = toward(p_prev, p, r)
    b = toward(p_next, p, r)
    path.append(f"L{a[0]:.2f} {a[1]:.2f}Q{p[0]:.2f} {p[1]:.2f} {b[0]:.2f} {b[1]:.2f}")


def closed_frame(x, y, w, h, r, ts=SLOPE, bs=SLOPE):
    tl, tr = (x, y), (x + w, y + ts * w)
    br, bl = (x + w, y + bs * w + h), (x, y + h)
    start = (x, y + h / 2)
    p = [f"M{start[0]:.2f} {start[1]:.2f}"]
    corner(p, start, tl, tr, r)
    corner(p, tl, tr, br, r)
    corner(p, tr, br, bl, r)
    corner(p, br, bl, tl, r)
    p.append("Z")
    return "".join(p)


def front_frame(x, y, w, h, r, descender_end, ts=SLOPE, bs=SLOPE):
    """Open frame: the left side runs on below the bottom edge (descender)."""
    tl, tr = (x, y), (x + w, y + ts * w)
    br = (x + w, y + bs * w + h)
    junction = (x, y + h)
    p = [f"M{x:.2f} {descender_end:.2f}"]
    corner(p, (x, descender_end), tl, tr, r)
    corner(p, tl, tr, br, r)
    corner(p, tr, br, junction, r)
    p.append(f"L{junction[0]:.2f} {junction[1]:.2f}")
    return "".join(p)


def interlocked_pair(bx, by, bw, bh, cx, cy, cw, ch, r, desc, ts):
    """Frames B and C drawn as one stroke, as in the approved reference.

    C's descender runs up its left side, across the top and down the right
    side; the shared lower stroke then descends from C's lower-right corner
    straight to B's lower-left corner, rises along B's left side, crosses B's
    top and ends where B's right side meets the shared lower stroke.
    """
    c_tl, c_tr = (cx, cy), (cx + cw, cy + ts * cw)
    c_br = (cx + cw, cy + ts * cw + ch)
    b_bl, b_tl, b_tr = (bx, by + bh), (bx, by), (bx + bw, by + ts * bw)
    t = (bx + bw - b_bl[0]) / (c_br[0] - b_bl[0])
    b_end = (bx + bw, b_bl[1] + t * (c_br[1] - b_bl[1]))
    p = [f"M{cx:.2f} {desc:.2f}"]
    corner(p, (cx, desc), c_tl, c_tr, r)
    corner(p, c_tl, c_tr, c_br, r)
    corner(p, c_tr, c_br, b_bl, r)
    corner(p, c_br, b_bl, b_tl, r)
    corner(p, b_bl, b_tl, b_tr, r)
    corner(p, b_tl, b_tr, b_end, r)
    p.append(f"L{b_end[0]:.2f} {b_end[1]:.2f}")
    return "".join(p)


def symbol_paths(q):
    ts, bs = q.get("ts", SLOPE), q.get("bs", SLOPE)
    a = closed_frame(q["ax"], q["ay"], q["aw"], q["ah"], q["r"], ts, bs)
    bc = interlocked_pair(q["ax"] + q["bdx"], q["ay"] + q["bdy"], q["bw"], q["bh"],
                          q["ax"] + q["cdx"], q["ay"] + q["cdy"], q["cw"], q["ch"], q["r"], q["desc"], ts)
    return [a, bc]


def svg(q, extra=""):
    paths = "".join(f'<path d="{d}"/>' for d in symbol_paths(q))
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
        f'<rect width="{W}" height="{H}" fill="#000"/>'
        f'<g fill="none" stroke="#fff" stroke-width="{q["sw"]}" stroke-linejoin="round" stroke-linecap="butt">{paths}</g>'
        f"{extra}</svg>"
    )


def render(svg_text, name):
    html = HERE / f"{name}.html"
    png = HERE / f"{name}.png"
    html.write_text(
        f"<!doctype html><html><body style='margin:0;background:#000'>{svg_text}</body></html>",
        encoding="utf-8",
    )
    subprocess.run(
        [EDGE, "--headless", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
         f"--window-size={W},{H}", f"--screenshot={png}", html.as_uri()],
        check=True, capture_output=True, timeout=60,
    )
    return np.array(Image.open(png).convert("L"))[:H, :W] > 128


def score(mask, region=(150, 900)):
    a = mask[region[0]:region[1]]
    b = REF[region[0]:region[1]]
    return (a & b).sum() / max((a | b).sum(), 1)


if __name__ == "__main__":
    q = json.loads(Path(sys.argv[1]).read_text())
    m = render(svg(q), "candidate")
    print(f"IoU symbol = {score(m):.4f}")

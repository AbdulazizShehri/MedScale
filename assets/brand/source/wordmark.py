"""First-party monoline wordmark "MedScale" (uniform stroke, squared forms).

Centerline paths in reference-image coordinates. Built from geometry, not a
font: no typeface is claimed or embedded.
"""


def rounded(points, r, closed=False):
    """Polyline through points with rounded corners (quadratic, control at the vertex)."""
    import math

    def along(a, b, d):
        dx, dy = b[0] - a[0], b[1] - a[1]
        n = math.hypot(dx, dy)
        return (b[0] - dx / n * d, b[1] - dy / n * d)

    out = [f"M{points[0][0]:.2f} {points[0][1]:.2f}"]
    for i in range(1, len(points) - 1):
        a, p, b = points[i - 1], points[i], points[i + 1]
        if r <= 0:
            out.append(f"L{p[0]:.2f} {p[1]:.2f}")
            continue
        # Clamp so neighbouring corners never overlap on short segments.
        rr = min(r, math.dist(a, p) / 2, math.dist(p, b) / 2)
        s = along(a, p, rr)
        e = along(b, p, rr)
        out.append(f"L{s[0]:.2f} {s[1]:.2f}Q{p[0]:.2f} {p[1]:.2f} {e[0]:.2f} {e[1]:.2f}")
    out.append(f"L{points[-1][0]:.2f} {points[-1][1]:.2f}")
    if closed:
        out.append("Z")
    return "".join(out)


def wordmark_paths(g):
    """g: geometry dict (stroke sw, radius r, metrics). Returns path strings."""
    sw, r = g["sw"], g["r"]
    h = sw / 2
    base = g["base"]            # outer baseline (stroke bottom)
    cap = g["cap"]              # outer cap height top
    asc = g["asc"]              # outer ascender top (d, l)
    xh = g["xh"]                # outer x-height top
    yb, yc, ya, yx = base - h, cap + h, asc + h, xh + h
    p = []

    # M
    x0, x1 = g["M"][0] + h, g["M"][1] - h
    vy, k = g["M_v"], g["M_k"]
    # Each stem turns inward along a short top stroke before its diagonal drops to the V.
    p.append(rounded([(x0, base), (x0, yc), (x0 + k, yc), ((x0 + x1) / 2, vy),
                      (x1 - k, yc), (x1, yc), (x1, base)], g.get("r_M", r)))

    def e(xa, xb):
        a, b = xa + h, xb - h
        mid = g["e_mid"]
        # top bar, right side to the middle bar, middle bar back to the left,
        # then the left side down and the open bottom bar.
        return rounded([(a, mid), (b, mid), (b, yx), (a, yx), (a, yb), (xb, yb)], r)

    p.append(e(*g["e"]))

    # d: bowl (left, top, bottom) joined to the right stem
    a, b = g["d"][0] + h, g["d"][1] - h
    p.append(rounded([(b, yx), (a, yx), (a, yb), (b, yb)], r))
    p.append(f"M{b:.2f} {asc:.2f}L{b:.2f} {base:.2f}")

    # S
    a, b = g["S"][0] + h, g["S"][1] - h
    mid = g["S_mid"]
    p.append(rounded([(g["S"][1], yc), (a, yc), (a, mid), (b, mid), (b, yb), (g["S"][0], yb)], r))

    # c
    a = g["c"][0] + h
    p.append(rounded([(g["c"][1], yx), (a, yx), (a, yb), (g["c"][1], yb)], r))

    # a: top bar into the right stem; bowl from the stem
    a0, b0 = g["a"][0] + h, g["a"][1] - h
    amid = g["a_mid"]
    p.append(rounded([(g["a"][0], yx), (b0, yx), (b0, base)], r))
    p.append(rounded([(b0, amid), (a0, amid), (a0, yb), (b0, yb)], r))

    # l
    xl = (g["l"][0] + g["l"][1]) / 2
    p.append(f"M{xl:.2f} {asc:.2f}L{xl:.2f} {base:.2f}")

    p.append(e(*g["e2"]))
    return p


DEFAULT = {
    "sw": 23, "r": 18, "base": 1121, "cap": 944, "asc": 941, "xh": 988,
    "M": (97, 284), "M_v": 1110, "M_k": 22.7, "r_M": 26, "e": (315, 438), "e_mid": 1054, "d": (465, 587),
    "S": (619, 753), "S_mid": 1031, "c": (782, 889), "a": (912, 1031), "a_mid": 1052,
    "l": (1061, 1086), "e2": (1116, 1238),
}

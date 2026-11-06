"""Rasterize an SVG file to PNG with headless Edge (exact pixel size, optional transparency)."""
import atexit
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

EDGE = os.environ.get("MEDSCALE_EDGE", r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")
# A private profile, so a running Edge cannot stall the headless render.
PROFILE = tempfile.mkdtemp(prefix="medscale-edge-")
atexit.register(shutil.rmtree, PROFILE, ignore_errors=True)


def raster(svg_path, png_path, width, height=None, transparent=True):
    svg_path, png_path = Path(svg_path).resolve(), Path(png_path).resolve()
    text = svg_path.read_text(encoding="utf-8")
    vb = [float(v) for v in text.split('viewBox="', 1)[1].split('"', 1)[0].split()]
    if height is None:
        height = round(width * vb[3] / vb[2])
    html = png_path.with_suffix(".html")
    bg = "transparent" if transparent else "#000"
    html.write_text(
        f"<!doctype html><html><head><style>html,body{{margin:0;background:{bg}}}img{{display:block;width:{width}px;height:{height}px}}</style></head>"
        f"<body><img src='{svg_path.resolve().as_uri()}'></body></html>",
        encoding="utf-8",
    )
    tmp = png_path.with_name(png_path.stem + ".full.png")
    args = [EDGE, "--headless", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
            "--allow-file-access-from-files", f"--user-data-dir={PROFILE}", f"--window-size={max(width, 64)},{max(height, 64)}",
            f"--screenshot={tmp}"]
    if transparent:
        args.append("--default-background-color=00000000")
    # Headless Edge occasionally hangs on a screenshot; a fresh attempt succeeds.
    for attempt in range(4):
        try:
            subprocess.run(args + [html.resolve().as_uri()], check=True, capture_output=True, timeout=30)
            break
        except subprocess.TimeoutExpired:
            if attempt == 3:
                raise
    Image.open(tmp).convert("RGBA").crop((0, 0, width, height)).save(png_path, optimize=True)
    tmp.unlink()
    html.unlink()
    return png_path


if __name__ == "__main__":
    raster(sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]) if len(sys.argv) > 4 else None)

"""Raster exports and platform icons from the SVG masters in out/."""
import sys
from pathlib import Path

from PIL import Image

from raster import raster

OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "out")
PNG = OUT / "png"
ICONS = OUT / "icons"
PNG.mkdir(exist_ok=True)
ICONS.mkdir(exist_ok=True)

# High-resolution transparent PNGs of the masters.
for name, width in [
    ("mark-white", 1024), ("mark-black", 1024),
    ("medscale-wordmark-white", 2048), ("medscale-wordmark-black", 2048),
    ("medscale-stacked-white", 2048), ("medscale-stacked-black", 2048),
    ("medscale-horizontal-white", 2400), ("medscale-horizontal-black", 2400),
    ("medscale-logo-canonical", 2400), ("medscale-logo-inverse", 2400),
]:
    raster(OUT / f"{name}.svg", PNG / f"{name}.png", width)
# README/doc scale (dark and light rendering contexts).
raster(OUT / "medscale-horizontal-white.svg", PNG / "medscale-readme-dark.png", 640)
raster(OUT / "medscale-horizontal-black.svg", PNG / "medscale-readme-light.png", 640)

# Square icon sizes: the small-size optical master at 48 px and below.
sizes = [16, 24, 32, 48, 64, 128, 256, 512, 1024]
icon = {}
for size in sizes:
    master = ("medscale-icon-small-dark" if size <= 32 else "medscale-icon-medium-dark" if size <= 64
              else "medscale-app-icon-dark")
    icon[size] = raster(OUT / f"{master}.svg", ICONS / f"medscale-icon-{size}.png", size, size)
for size in [16, 32, 128, 256, 512, 1024]:
    raster(OUT / "medscale-app-icon-macos.svg", ICONS / f"macos-{size}.png", size, size)

images = {size: Image.open(path).convert("RGBA") for size, path in icon.items()}
# Windows ICO: every size is its own correctly rasterized bitmap.
ico_sizes = [16, 24, 32, 48, 64, 128, 256]
images[256].save(ICONS / "icon.ico", format="ICO", sizes=[(s, s) for s in ico_sizes],
                 append_images=[images[s] for s in ico_sizes if s != 256])
# macOS ICNS from the rounded-tile variant.
mac = {s: Image.open(ICONS / f"macos-{s}.png").convert("RGBA") for s in [16, 32, 128, 256, 512, 1024]}
mac[1024].save(ICONS / "icon.icns", format="ICNS", append_images=[mac[s] for s in [16, 32, 128, 256, 512]])
# Tauri bundle PNGs.
images[32].save(ICONS / "32x32.png")
images[128].save(ICONS / "128x128.png")
images[256].save(ICONS / "128x128@2x.png")
images[512].save(ICONS / "icon.png")
# Favicon PNG fallbacks.
for s in [16, 32, 48]:
    images[s].save(ICONS / f"favicon-{s}.png")
raster(OUT / "medscale-app-icon-dark.svg", ICONS / "apple-touch-icon.png", 180, 180)

# Readability sheet: 16/24/32/64/128/256 at 1:1 on dark and light, plus 4x nearest-neighbour zoom.
row = [16, 24, 32, 64, 128, 256]
light = {s: Image.open(raster(OUT / ("medscale-icon-small-dark.svg" if s <= 32 else "medscale-icon-medium-dark.svg" if s <= 64 else "medscale-app-icon-light.svg"),
                              ICONS / f"tmp-light-{s}.png", s, s)).convert("RGBA") for s in row}
W = sum(row) + 20 * (len(row) + 1)
sheet = Image.new("RGBA", (W, 256 * 2 + 60 + 4 * 64 + 40), (128, 128, 128, 255))
x = 20
for s in row:
    sheet.paste(images[s], (x, 20), images[s])
    sheet.paste(light[s], (x, 296), light[s])
    x += s + 20
x = 20
for s in [16, 24, 32, 64]:
    z = images[s].resize((s * 4, s * 4), Image.NEAREST)
    sheet.paste(z, (x, 572), z)
    x += s * 4 + 20
sheet.save(PNG / "icon-readability-sheet.png")
for p in ICONS.glob("tmp-light-*.png"):
    p.unlink()
print("exported", len(list(PNG.iterdir())), "png and", len(list(ICONS.iterdir())), "icon files")

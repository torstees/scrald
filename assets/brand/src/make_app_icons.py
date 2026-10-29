"""Build the Tauri app icons from the illuminated-initial art.

Sizes up to 48 px use the simplified art (just the gold S); 64 px and up use the full page.
Run with:  python make_app_icons.py
Writes to crates/scrald-app/icons/ and assets/icon-source.png.
"""
import io
import os
import cairosvg
from PIL import Image
import icon_initial

from paths import APP_ICONS as OUT, REPO

FULL, SMALL = icon_initial.svg(1), icon_initial.svg(0)


def render(size):
    svg = SMALL if size <= 48 else FULL
    png = cairosvg.svg2png(bytestring=svg.encode("utf-8"), output_width=size, output_height=size)
    return Image.open(io.BytesIO(png)).convert("RGBA")


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, size in (("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128),
                       ("128x128@2x.png", 256), ("icon.png", 512)):
        render(size).save(os.path.join(OUT, name))

    ico = {s: render(s) for s in (16, 24, 32, 48, 64, 128, 256)}
    ico[256].save(os.path.join(OUT, "icon.ico"), sizes=[(s, s) for s in ico],
                  append_images=[ico[s] for s in (16, 24, 32, 48, 64, 128)])

    icns = {s: render(s) for s in (16, 32, 64, 128, 256, 512, 1024)}
    icns[1024].save(os.path.join(OUT, "icon.icns"),
                    append_images=[icns[s] for s in (16, 32, 64, 128, 256, 512)])

    render(1024).save(os.path.join(REPO, "assets", "icon-source.png"))
    print("icons written to", OUT)


if __name__ == "__main__":
    main()

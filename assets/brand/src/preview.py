"""Render an icon SVG at several sizes side by side on light and dark strips."""
import sys, io
import cairosvg
from PIL import Image


def render(svg, size):
    return Image.open(io.BytesIO(cairosvg.svg2png(bytestring=svg.encode(), output_width=size, output_height=size))).convert("RGBA")


def sheet(svgs, out, sizes=(512, 128, 64, 32, 16)):
    """svgs: list of svg strings (or (big_svg, small_svg) pairs, small used <=48px)."""
    pad = 24
    rowh = sizes[0] + pad * 2
    width = sum(sizes) + pad * (len(sizes) + 1)
    img = Image.new("RGBA", (width, rowh * len(svgs) * 2), "white")
    for r, s in enumerate(svgs):
        big, small = s if isinstance(s, tuple) else (s, s)
        for k, bg in enumerate(("#f6f8fa", "#0d1117")):
            y0 = (r * 2 + k) * rowh
            img.paste(Image.new("RGBA", (width, rowh), bg), (0, y0))
            x = pad
            for sz in sizes:
                im = render(small if sz <= 48 else big, sz)
                img.alpha_composite(im, (x, y0 + pad + (sizes[0] - sz) // 2))
                x += sz + pad
    img.save(out)


if __name__ == "__main__":
    mod = __import__(sys.argv[1])
    sheet([(mod.svg(1.0), mod.svg(0.0))], sys.argv[2])

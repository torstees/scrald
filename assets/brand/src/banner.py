"""README banner: the Skald toasting beside the Scrald wordmark. Light and dark variants."""
import math, random
import os
from wood import *
from textpath import text_path
from skald import figure

W, H = 1280, 400
TAGLINE = "A themed Markdown reader for docs, sagas & study notes"


def frame(fg, bg):
    g = [f'<rect x="12" y="12" width="{W - 24}" height="{H - 24}" rx="18" fill="none" stroke="{fg}" stroke-width="10"/>',
         f'<rect x="28" y="28" width="{W - 56}" height="{H - 56}" rx="8" fill="none" stroke="{fg}" stroke-width="3"/>']
    # corner diamonds
    for (x, y) in ((28, 28), (W - 28, 28), (28, H - 28), (W - 28, H - 28)):
        g.append(f'<rect x="{x - 11}" y="{y - 11}" width="22" height="22" transform="rotate(45 {x} {y})" '
                 f'fill="{GOLD}" stroke="{fg}" stroke-width="4"/>')
    return "".join(g)


def grain(color, seed=5):
    rnd = random.Random(seed)
    out = []
    for i in range(26):
        y = 50 + i * 12.5 + rnd.uniform(-4, 4)
        x0 = rnd.uniform(420, 1150)
        out.append(path(gouge((x0, y), (x0 + rnd.uniform(60, 180), y + rnd.uniform(-3, 3)), rnd.uniform(1.5, 3)), color))
    return "".join(out)


def roundel(cx, cy, r, fill, ray, ring):
    g = [f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}" stroke="{ring}" stroke-width="6"/>']
    n = 36
    for i in range(n):
        a = 2 * math.pi * i / n
        r0 = r * (0.30 if i % 2 else 0.42)
        p0 = (cx + r0 * math.cos(a), cy + r0 * math.sin(a))
        p1 = (cx + (r - 14) * math.cos(a), cy + (r - 14) * math.sin(a))
        g.append(path(wedge(p1, p0, 16 if i % 2 == 0 else 9), ray))
    g.append(f'<circle cx="{cx}" cy="{cy}" r="{r - 16}" fill="none" stroke="{ring}" stroke-width="2.5"/>')
    return "".join(g)


def banner(dark=False):
    bg = "#1d212b" if dark else PAPER
    fg = PAPER if dark else INK
    sub = "#c9c1ae" if dark else "#4a4f5c"
    g = [f'<rect width="{W}" height="{H}" rx="22" fill="{bg}"/>']
    g.append(grain("#262b36" if dark else PAPER_SHADE))
    # clip everything inside the inner frame
    g.append(f'<clipPath id="inner"><rect x="30" y="30" width="{W - 60}" height="{H - 60}" rx="6"/></clipPath>')
    inner = []
    rcx, rcy = 262, 196
    if dark:
        inner.append(roundel(rcx, rcy, 168, PAPER, PAPER_SHADE, PAPER))
    else:
        inner.append(roundel(rcx, rcy, 168, "#f3d99a", GOLD_LT, INK))
    k = 0.74
    inner.append(f'<g transform="translate({70},{H - 30 - 500 * k:.1f}) scale({k})">{figure()}</g>')
    g.append(f'<g clip-path="url(#inner)">{"".join(inner)}</g>')

    # wordmark with a carved gold offset
    x0, base = 520, 222
    d, w = text_path("Scrald", "unifrakturmaguntia", 190, x0, base)
    g.append(f'<path d="{d}" fill="{GOLD}" transform="translate(7,7)"/>')
    g.append(f'<path d="{d}" fill="{fg}"/>')
    # rule with a centre ornament
    ry = 262
    _, tw = text_path(TAGLINE, "im-fell-english", 28, 0, 0)
    g.append(f'<line x1="{x0}" y1="{ry}" x2="{x0 + tw}" y2="{ry}" stroke="{fg}" stroke-width="3"/>')
    g.append(f'<line x1="{x0}" y1="{ry + 7}" x2="{x0 + tw}" y2="{ry + 7}" stroke="{fg}" stroke-width="1.5"/>')
    ox = x0 + tw / 2
    g.append(f'<rect x="{ox - 10}" y="{ry - 6}" width="20" height="20" transform="rotate(45 {ox} {ry + 4})" '
             f'fill="{RED}" stroke="{fg}" stroke-width="3"/>')
    # tagline
    d, tw = text_path(TAGLINE, "im-fell-english", 28, x0, 316)
    g.append(f'<path d="{d}" fill="{sub}"/>')
    # Skål!
    d, sw = text_path("Skål!", "im-fell-english", 34, x0 + tw, 120, anchor="end")
    g.append(f'<path d="{d}" fill="{RED if not dark else GOLD_LT}" transform="rotate(-6 {x0 + 600} 110)"/>')

    g.append(frame(fg, bg))
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">' \
           f'<title>Scrald: a themed Markdown reader</title>{"".join(g)}</svg>'


if __name__ == "__main__":
    from paths import BRAND, write_png, write_svg
    for dark in (False, True):
        name = "banner-dark" if dark else "banner-light"
        s = banner(dark)
        write_svg(os.path.join(BRAND, name + ".svg"), s)
        write_png(os.path.join(BRAND, name + ".png"), s, W * 2)

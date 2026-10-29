"""The longship of pages: hull of a bound book, a manuscript sail, scroll-end prow and stern."""
import math, random
import cairosvg
from wood import *
from textpath import text_path
from skald import skald, raised_arm_with_horn
from vignettes import shield, raven

W, H = 960, 400
WATER = 296


def keel(x, x0=150, x1=822, depth=104):
    t = min(max((x - x0) / (x1 - x0), 0), 1)
    return 232 + depth * math.sin(math.pi * t) ** 0.6


def hull():
    g = []
    xs = [150 + i * 6 for i in range(113)]
    top = [(x, 232) for x in xs]
    bot = [(x, keel(x)) for x in xs]
    hull_d = poly(top + bot[::-1])
    # lower half: the leather cover; upper half: the page block
    mid = [(x, 232 + (keel(x) - 232) * 0.52) for x in xs]
    g.append(path(poly(mid + bot[::-1]), RED))
    g.append(path(poly(top + mid[::-1]), PAPER))
    # page edges: many fine lines following the hull
    for f in [0.065 * i for i in range(1, 8)]:
        pts = [(x, 232 + (keel(x) - 232) * f) for x in xs[2:-2]]
        g.append(f'<path d="{poly(pts, False)}" fill="none" stroke="{PAPER_SHADE}" stroke-width="2.2"/>')
    # gold tooling on the cover
    pts = [(x, 232 + (keel(x) - 232) * 0.74) for x in xs[4:-4]]
    g.append(f'<path d="{poly(pts, False)}" fill="none" stroke="{GOLD_LT}" stroke-width="3" stroke-dasharray="2 9" stroke-linecap="round"/>')
    pts = [(x, 232 + (keel(x) - 232) * 0.62) for x in xs[3:-3]]
    g.append(f'<path d="{poly(pts, False)}" fill="none" stroke="{GOLD}" stroke-width="4"/>')
    # carved shading under the hull
    for x in range(190, 800, 22):
        k = keel(x)
        g.append(path(wedge((x, k - 1), (x + 14, k - 22), 7), INK))
    g.append(f'<path d="{poly(mid, False)}" fill="none" stroke="{INK}" stroke-width="4"/>')
    g.append(path(hull_d, "none", INK, 8))
    return "".join(g)


def scroll_post(x0, y0, sgn):
    """Prow/stern: the hull rises into a rolled scroll end (a curl)."""
    cx, cy = x0 + sgn * 42, y0 - 140
    d = (f"M {x0},{y0} C {x0 + sgn * 30},{y0 - 20} {x0 + sgn * 52},{y0 - 70} {cx + sgn * 26},{cy + 30} "
         f"C {cx + sgn * 34},{cy - 6} {cx + sgn * 6},{cy - 36} {cx - sgn * 20},{cy - 20} "
         f"C {cx - sgn * 40},{cy - 6} {cx - sgn * 30},{cy + 26} {cx - sgn * 6},{cy + 22} "
         f"C {cx + sgn * 10},{cy + 18} {cx + sgn * 8},{cy - 2} {cx - sgn * 4},{cy}")
    return (f'<path d="{d}" fill="none" stroke="{INK}" stroke-width="30" stroke-linecap="round"/>'
            f'<path d="{d}" fill="none" stroke="{RED}" stroke-width="18" stroke-linecap="round"/>'
            f'<path d="{d}" fill="none" stroke="{GOLD_LT}" stroke-width="3" stroke-dasharray="2 10" stroke-linecap="round"/>')


def sail():
    g = []
    g.append(f'<line x1="472" y1="242" x2="472" y2="28" stroke="{INK}" stroke-width="16" stroke-linecap="round"/>')
    g.append(f'<line x1="472" y1="242" x2="472" y2="28" stroke="#8a5a34" stroke-width="8" stroke-linecap="round"/>')
    # rigging
    for (x2, y2) in ((180, 232), (800, 232)):
        g.append(f'<line x1="472" y1="34" x2="{x2}" y2="{y2}" stroke="{INK}" stroke-width="3"/>')
    sail_d = "M 352,58 L 592,58 C 620,110 622,170 598,214 L 346,214 C 322,170 324,110 352,58 Z"
    g.append(path(sail_d, PAPER, INK, 8))
    # red stripes at the edges, as on a Viking sail
    g.append(f'<clipPath id="sailclip"><path d="{sail_d}"/></clipPath>')
    stripes = (f'<rect x="320" y="50" width="48" height="170" fill="{RED}"/>'
               f'<rect x="576" y="50" width="48" height="170" fill="{RED}"/>')
    shade = "".join(path(wedge((604, 70 + i * 16), (566, 78 + i * 16), 7), INK) for i in range(9))
    g.append(f'<g clip-path="url(#sailclip)">{stripes}{shade}</g>')
    # the page: an illuminated initial and lines of text
    g.append(f'<rect x="384" y="78" width="52" height="52" rx="4" fill="{GOLD}" stroke="{INK}" stroke-width="5"/>')
    d, _ = text_path("S", "unifrakturmaguntia", 50, 410, 122, anchor="middle")
    g.append(f'<path d="{d}" fill="{RED}" stroke="{INK}" stroke-width="2"/>')
    rnd = random.Random(2)
    rows = [(446, 90, 104), (446, 106, 96), (446, 122, 100), (384, 146, 168), (384, 162, 160), (384, 178, 168), (384, 194, 110)]
    for (x, y, L) in rows:
        pts = [(x + L * k / 6, y + rnd.uniform(-1.5, 1.5)) for k in range(7)]
        g.append(f'<path d="{poly(pts, False)}" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>')
    g.append(path(sail_d, "none", INK, 8))
    # yard
    g.append(f'<line x1="336" y1="58" x2="608" y2="58" stroke="{INK}" stroke-width="14" stroke-linecap="round"/>')
    g.append(f'<line x1="336" y1="58" x2="608" y2="58" stroke="#8a5a34" stroke-width="6" stroke-linecap="round"/>')
    return "".join(g)


def shields():
    cols = [(GOLD, RED, INK, PAPER), (BLUE, PAPER, GOLD, INK), (RED, GOLD, INK, PAPER), (PAPER, BLUE, RED, INK)]
    out = []
    for i, x in enumerate(range(206, 790, 64)):
        f, p, boss, ring = cols[i % 4]
        out.append(shield(x, 226, 23, f, p, boss, ring, rot=i * 17))
    return "".join(out)


def oars():
    g = []
    for x in range(238, 780, 64):
        y0 = keel(x) - 20
        g.append(f'<line x1="{x}" y1="{y0:.0f}" x2="{x - 46}" y2="{WATER + 50}" stroke="{INK}" stroke-width="9" stroke-linecap="round"/>')
        g.append(f'<line x1="{x}" y1="{y0:.0f}" x2="{x - 46}" y2="{WATER + 50}" stroke="#8a5a34" stroke-width="4" stroke-linecap="round"/>')
    return "".join(g)


def sea_back():
    g = [f'<rect x="0" y="{WATER}" width="{W}" height="{H - WATER}" fill="{BLUE}"/>']
    rnd = random.Random(9)
    for row in range(6):
        y = WATER + 12 + row * 16
        for i in range(10):
            x = rnd.uniform(0, W)
            g.append(path(gouge((x, y), (x + rnd.uniform(40, 120), y + rnd.uniform(-2, 2)), 3.4), "#b9cbe0"))
    return "".join(g)


def wave_row(y, amp, period, fill, phase=0.0, crest=True):
    """A row of woodcut waves with curled crests."""
    pts = []
    d = f"M -20,{H + 10} L -20,{y}"
    x = -20 + phase
    while x < W + period:
        d += (f" C {x + period * 0.25},{y - amp} {x + period * 0.55},{y - amp * 1.15} {x + period * 0.68},{y - amp * 0.7}"
              f" C {x + period * 0.74},{y - amp * 0.3} {x + period * 0.64},{y - amp * 0.2} {x + period * 0.6},{y - amp * 0.42}"
              f" C {x + period * 0.66},{y - amp * 0.05} {x + period * 0.82},{y + 4} {x + period},{y}")
        pts.append(x)
        x += period
    d += f" L {W + 20},{H + 10} Z"
    g = [path(d, fill, INK, 6)]
    if crest:
        for x in pts:
            for k in range(3):
                a = (x + period * (0.2 + k * 0.1), y - amp * (0.2 + k * 0.15) + 14)
                b = (x + period * (0.42 + k * 0.08), y - amp * (0.55 + k * 0.1) + 10)
                g.append(path(gouge(a, b, 4), "#d7e2ee"))
    return "".join(g)


def sky():
    g = []
    # sun with carved rays, top left
    cx, cy = 724, 104
    for i in range(24):
        a = 2 * math.pi * i / 24
        p0 = (cx + 50 * math.cos(a), cy + 50 * math.sin(a))
        p1 = (cx + (86 if i % 2 else 104) * math.cos(a), cy + (86 if i % 2 else 104) * math.sin(a))
        g.append(path(wedge(p0, p1, 12 if i % 2 == 0 else 8), GOLD_LT))
    g.append(f'<circle cx="{cx}" cy="{cy}" r="44" fill="{GOLD_LT}" stroke="{INK}" stroke-width="6"/>')
    g.append(f'<path d="M {cx - 18},{cy + 6} Q {cx},{cy + 22} {cx + 18},{cy + 6}" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>')
    for ex in (cx - 14, cx + 14):
        g.append(f'<path d="M {ex - 7},{cy - 6} Q {ex},{cy - 13} {ex + 7},{cy - 6}" fill="none" stroke="{INK}" stroke-width="4" stroke-linecap="round"/>')
    # curly clouds
    for (x, y, s) in ((196, 52, 0.85),):
        c = (f"M {x},{y} c 10,-26 44,-26 52,-6 c 14,-18 44,-10 44,12 c 18,0 22,24 6,30 "
             f"l -110,0 c -18,-4 -16,-30 8,-36 z")
        g.append(f'<g transform="translate({x * (1 - s)},{y * (1 - s)}) scale({s})">' + path(c, PAPER, INK, 5) +
                 path(gouge((x + 4, y + 26), (x + 80, y + 26), 3), PAPER_SHADE) + "</g>")
    # Huginn and Muninn, far off
    g.append(raven(636, 52, 0.2, -6, 10))
    g.append(raven(668, 80, 0.16, -2, 0))
    return "".join(g)


def frame():
    g = [f'<rect x="8" y="8" width="{W - 16}" height="{H - 16}" rx="18" fill="none" stroke="{INK}" stroke-width="10"/>',
         f'<rect x="22" y="22" width="{W - 44}" height="{H - 44}" rx="8" fill="none" stroke="{INK}" stroke-width="3"/>']
    for (x, y) in ((22, 22), (W - 22, 22), (22, H - 22), (W - 22, H - 22)):
        g.append(f'<rect x="{x - 10}" y="{y - 10}" width="20" height="20" transform="rotate(45 {x} {y})" '
                 f'fill="{GOLD}" stroke="{INK}" stroke-width="4"/>')
    return "".join(g)


def longship():
    inner = []
    inner.append(f'<rect width="{W}" height="{H}" fill="{PAPER}"/>')
    rnd = random.Random(6)
    for i in range(18):
        y = 30 + i * 14
        x0 = rnd.uniform(140, 860)
        inner.append(path(gouge((x0, y), (x0 + rnd.uniform(50, 140), y + rnd.uniform(-3, 3)), rnd.uniform(1.5, 3)), PAPER_SHADE))
    inner.append(sky())
    inner.append(sea_back())
    inner.append(wave_row(WATER + 6, 22, 120, BLUE, phase=40, crest=False))
    inner.append(sail())
    # the Skald at the stern, toasting the voyage (drawn before the hull so the gunwale hides his body)
    inner.append(f'<g transform="translate(236,118) scale(0.24)">{skald(eyes="happy", mouth="sing", uid="ls")}'
                 f'{raised_arm_with_horn()}</g>')
    inner.append(scroll_post(156, 238, -1))
    inner.append(scroll_post(816, 238, 1))
    inner.append(oars())
    inner.append(hull())
    inner.append(shields())
    inner.append(wave_row(WATER + 40, 30, 150, BLUE_DK, phase=-60))
    inner.append(wave_row(WATER + 78, 26, 130, BLUE, phase=10))
    body = (f'<clipPath id="lsclip"><rect x="24" y="24" width="{W - 48}" height="{H - 48}" rx="6"/></clipPath>'
            f'<rect x="8" y="8" width="{W - 16}" height="{H - 16}" rx="18" fill="{PAPER}"/>'
            f'<g clip-path="url(#lsclip)">{"".join(inner)}</g>{frame()}')
    return tidy(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">'
            f'<title>The longship of pages: a ship whose hull is a bound book and whose sail is a manuscript page</title>'
            f'{body}</svg>')


if __name__ == "__main__":
    import os
    from paths import DOCS_ART, UI_ART, write_svg
    for folder in (DOCS_ART, UI_ART):
        write_svg(os.path.join(folder, "longship.svg"), longship())

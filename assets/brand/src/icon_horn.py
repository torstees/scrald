"""Icon concept A: the quill-horn - a drinking horn whose tip is a pen nib, scrawling a line."""
import math
from wood import *

S = 1024


def tile(inner, carve_border=True):
    """Woodcut tile background: ink squircle with a carved paper border line."""
    out = [f'<rect x="40" y="40" width="944" height="944" rx="200" fill="{INK}"/>',
           f'<rect x="{78 if carve_border else 70}" y="{78 if carve_border else 70}" width="{868 if carve_border else 884}" '
           f'height="{868 if carve_border else 884}" rx="{164 if carve_border else 172}" fill="{PAPER}"/>']
    if carve_border:
        out.append(woodgrain())
        out.append(f'<rect x="112" y="112" width="800" height="800" rx="134" fill="none" '
                   f'stroke="{INK}" stroke-width="8"/>')
    out.append(inner)
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {S} {S}">{"".join(out)}</svg>'


def woodgrain():
    """Faint carved grain lines, as uninked areas of a block often print."""
    import random
    rnd = random.Random(7)
    g = []
    for i in range(14):
        y = 150 + i * 55 + rnd.uniform(-10, 10)
        x0 = rnd.uniform(140, 420)
        x1 = x0 + rnd.uniform(90, 260)
        g.append(path(gouge((x0, y), (x1, y + rnd.uniform(-8, 8)), rnd.uniform(3, 6), rnd.uniform(-4, 4)), PAPER_SHADE))
    return f'<g opacity="0.9">{"".join(g)}</g>'


def horn(detail=1.0):
    C = ((800, 285), (590, 215), (300, 400), (262, 760))
    W0, W1 = 250, 58
    T_NIB = 0.83

    def w(t):
        if t <= T_NIB:
            return W0 + (W1 - W0) * (t / T_NIB) ** 0.85
        return W1 * (1 - (t - T_NIB) / (1 - T_NIB)) ** 0.75

    g = []
    # --- nib (metal), drawn first so the horn overlaps it
    nib = ribbon(C, 0, 0, n=30, t0=T_NIB - 0.02, t1=1.0, wfn=w)
    g.append(path(nib, PAPER, INK, 16))
    # nib slit + breather hole
    a = pt(C, T_NIB + 0.06)
    b = pt(C, 0.995)
    g.append(f'<line x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}" '
             f'stroke="{INK}" stroke-width="9" stroke-linecap="round"/>')
    g.append(f'<circle cx="{a[0]:.1f}" cy="{a[1]:.1f}" r="10" fill="{INK}"/>')

    # --- horn body
    body = ribbon(C, 0, 0, n=80, t0=0.0, t1=T_NIB, wfn=w)
    g.append(path(body, GOLD))
    # carved shading on the lower (shadow) side: ink wedges from the edge inward
    n_cuts = int(26 * detail)
    for i in range(n_cuts):
        t = 0.04 + (T_NIB - 0.08) * i / (n_cuts - 1)
        ww = w(t)
        depth = ww * (0.55 - 0.25 * (i % 2))
        edge = pt(C, t - 0.004, -ww / 2 - 2)
        inner = pt(C, t + 0.03, -ww / 2 + depth)
        g.append(path(wedge(edge, inner, ww * 0.12 + 6), INK))
    # highlight gouges on the light side
    for i in range(int(9 * detail)):
        t0 = 0.06 + i * 0.075
        if t0 + 0.06 > T_NIB - 0.04:
            break
        a = pt(C, t0, w(t0) * 0.22)
        b = pt(C, t0 + 0.06, w(t0 + 0.06) * 0.22)
        g.append(path(gouge(a, b, 10 + 6 * (1 - t0)), GOLD_LT))
    g.append(path(body, "none", INK, 16))

    # --- bands (mouth rim and nib collar)
    for (ta, tb) in ((0.05, 0.12), (T_NIB - 0.045, T_NIB)):
        band = ribbon(C, 0, 0, n=12, t0=ta, t1=tb, wfn=lambda t: w(t) + 16)
        g.append(path(band, PAPER_SHADE, INK, 14))
        tm = (ta + tb) / 2
        for k in (-0.3, 0, 0.3):
            p = pt(C, tm, w(tm) * k)
            g.append(f'<circle cx="{p[0]:.1f}" cy="{p[1]:.1f}" r="{7 if ta < .5 else 4.5}" fill="{INK}"/>')

    # --- mouth: opening ellipse aimed along the tangent, with mead and foam
    mx, my = C[0]
    dx, dy = bez_d(*C, 0)
    ang = math.degrees(math.atan2(dy, dx))
    g.append(f'<g transform="translate({mx},{my}) rotate({ang + 180:.1f})">'
             f'<ellipse rx="44" ry="{W0 / 2 + 4}" fill="{GOLD_LT}" stroke="{INK}" stroke-width="16"/>'
             f'<ellipse cx="-4" rx="26" ry="{W0 / 2 - 22}" fill="{GOLD}"/>'
             f'</g>')
    # foam spilling over the rim
    foam = ("M 752,160 C 764,110 826,100 842,138 C 876,110 932,134 918,182 "
            "C 952,200 940,256 902,258 C 916,300 880,328 852,310 "
            "C 854,346 832,366 816,352 C 810,312 828,274 812,246 "
            "C 794,224 760,224 752,200 Z")
    g.append(path(foam, PAPER, INK, 14))
    # a drip running down the horn
    g.append(path("M 792,400 C 784,432 784,458 794,474 C 806,488 822,474 818,456 "
                  "C 814,438 808,424 804,400 Z", PAPER, INK, 12))
    for (cx, cy, r) in ((866, 186, 9), (888, 226, 7), (832, 160, 6)):
        g.append(f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{INK}" stroke-width="6"/>')
    return "".join(g)


def scrawl(detail=1.0):
    # the "scrawled" line leaving the nib tip: loops of a hand-written flourish
    d = ("M 262,768 C 274,818 316,836 358,812 C 400,788 388,746 362,756 "
         "C 332,768 358,834 422,834 C 486,834 516,782 494,766 "
         "C 470,750 454,816 528,830 C 610,846 720,800 830,808")
    if detail < 1:
        d = "M 262,768 C 300,860 420,840 470,800 C 520,770 560,840 830,808"
    # tapered: draw as a ribbon-ish stroke with a thinner tail
    return (f'<path d="{d}" fill="none" stroke="{INK}" stroke-width="{24 if detail >= 1 else 48}" '
            f'stroke-linecap="round" stroke-linejoin="round"/>')


def svg(detail=1.0):
    k = 0.86 if detail >= 1 else 0.98
    return tile(f'<g transform="translate({512-512*k-18:.0f},{512-512*k+26:.0f}) scale({k})">' + scrawl(detail) + horn(detail) + '</g>', carve_border=detail >= 1)


if __name__ == "__main__":
    from paths import write_icon
    write_icon("horn", svg)

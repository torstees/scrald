"""Icon concept B: an illuminated manuscript page with a Fraktur 'S' initial."""
import random
from wood import *
from textpath import text_path
from icon_horn import tile

S = 1024


def svg(detail=1.0, with_horn=False):
    g = []
    rnd = random.Random(3)
    full = detail >= 1
    # gilded initial box
    bx, by, bs = (210, 200, 380) if full else (150, 150, 724)
    g.append(f'<rect x="{bx}" y="{by}" width="{bs}" height="{bs}" rx="18" fill="{GOLD}" stroke="{INK}" stroke-width="16"/>')
    if full:
        # carved shading: a narrow band of parallel cuts inside the bottom and right edges
        for i in range(14):
            x = bx + 30 + i * 25
            g.append(path(wedge((x, by + bs - 10), (x + 16, by + bs - 34), 8), INK))
        for i in range(13):
            y = by + 40 + i * 25
            g.append(path(wedge((bx + bs - 10, y), (bx + bs - 34, y + 16), 8), INK))
        # vine flourish in blue curling out of the box
        vine = f"M {bx+bs-30},{by+30} C {bx+bs+60},{by-40} {bx+bs+150},{by+40} {bx+bs+110},{by+110} " \
               f"C {bx+bs+80},{by+160} {bx+bs+30},{by+120} {bx+bs+60},{by+90}"
        g.append(f'<path d="{vine}" fill="none" stroke="{INK}" stroke-width="30" stroke-linecap="round"/>')
        g.append(f'<path d="{vine}" fill="none" stroke="{BLUE}" stroke-width="14" stroke-linecap="round"/>')
        for (lx, ly, a) in ((bx + bs + 40, by - 2, -30), (bx + bs + 140, by + 50, 40)):
            g.append(f'<ellipse cx="{lx}" cy="{ly}" rx="34" ry="16" transform="rotate({a} {lx} {ly})" '
                     f'fill="{BLUE}" stroke="{INK}" stroke-width="10"/>')
    # the letter
    size = 350 if full else 700
    d, w = text_path("S", "unifrakturmaguntia", size, bx + bs / 2 - (6 if full else 10), by + bs * (0.79 if full else 0.80), anchor="middle")
    g.append(f'<path d="{d}" fill="{PAPER}" stroke="{INK}" stroke-width="{26 if full else 34}" stroke-linejoin="round" paint-order="stroke"/>')
    g.append(f'<path d="{d}" fill="{RED}"/>' if not full else f'<path d="{d}" fill="{RED}" transform="translate(0,0)"/>')
    if full:
        # scrawled text lines: wobbly carved strokes to the right of and under the initial
        if with_horn:
            rows = [(650, 430, 210), (650, 510, 180), (210, 650, 370), (210, 740, 280), (210, 830, 240)]
        else:
            rows = [(650, 430, 210), (650, 510, 180), (210, 650, 600), (210, 740, 560), (210, 830, 330)]
        for (x, y, L) in rows:
            pts = []
            n = 9
            for k in range(n + 1):
                pts.append((x + L * k / n, y + rnd.uniform(-7, 7)))
            dd = "M" + " L".join(f"{px:.0f},{py:.0f}" for px, py in pts)
            g.append(f'<path d="{dd}" fill="none" stroke="{INK}" stroke-width="22" stroke-linecap="round" stroke-linejoin="round"/>')
    if full and with_horn:
        from icon_horn import horn
        k = 0.46
        tip = (482, 846)  # nib rests at the end of the last line
        g.append(f'<g transform="translate({tip[0] - 262 * k:.1f},{tip[1] - 760 * k:.1f}) scale({k})">{horn(1.0)}</g>')
    return tile("".join(g), carve_border=full)


if __name__ == "__main__":
    from paths import write_icon
    write_icon("initial", svg)

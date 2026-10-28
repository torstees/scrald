"""Tiny geometry kit for woodcut-style SVG: beziers, tapered ribbons, carved hatching."""
import math

INK = "#232733"
PAPER = "#ede5d0"
PAPER_SHADE = "#d9ceb3"
BLUE = "#5e80a8"
BLUE_DK = "#3f5f86"
GOLD = "#c8912f"
GOLD_LT = "#e3b862"
RED = "#a3412e"


def bez(p0, p1, p2, p3, t):
    u = 1 - t
    return (u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
            u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1])


def bez_d(p0, p1, p2, p3, t):
    u = 1 - t
    return (3 * u * u * (p1[0] - p0[0]) + 6 * u * t * (p2[0] - p1[0]) + 3 * t * t * (p3[0] - p2[0]),
            3 * u * u * (p1[1] - p0[1]) + 6 * u * t * (p2[1] - p1[1]) + 3 * t * t * (p3[1] - p2[1]))


def normal(c, t):
    dx, dy = bez_d(*c, t)
    l = math.hypot(dx, dy) or 1
    return (-dy / l, dx / l)


def pt(c, t, off=0.0):
    x, y = bez(*c, t)
    nx, ny = normal(c, t)
    return (x + nx * off, y + ny * off)


def poly(points, close=True):
    s = "M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in points)
    return s + (" Z" if close else "")


def ribbon(c, w0, w1, n=60, t0=0.0, t1=1.0, wfn=None):
    """Tapered filled stroke along cubic c, width w0 -> w1 (or wfn(t))."""
    left, right = [], []
    for i in range(n + 1):
        t = t0 + (t1 - t0) * i / n
        w = wfn(t) if wfn else w0 + (w1 - w0) * (i / n)
        left.append(pt(c, t, w / 2))
        right.append(pt(c, t, -w / 2))
    return poly(left + right[::-1])


def gouge(a, b, w, bulge=0.0, n=14):
    """A carved sliver from a to b: pointed at both ends, max width w in the middle."""
    ax, ay = a
    bx, by = b
    dx, dy = bx - ax, by - ay
    l = math.hypot(dx, dy) or 1
    nx, ny = -dy / l, dx / l
    mx, my = (ax + bx) / 2 + nx * bulge, (ay + by) / 2 + ny * bulge
    c = (a, (ax + (mx - ax) * 1.3, ay + (my - ay) * 1.3), (bx + (mx - bx) * 1.3, by + (my - by) * 1.3), b)
    return ribbon(c, 0, 0, n=n, wfn=lambda t: w * math.sin(math.pi * t) ** 0.8)


def wedge(a, b, w, n=10):
    """Carved stroke that starts wide at a and tapers to a point at b."""
    c = (a, lerp(a, b, 0.33), lerp(a, b, 0.66), b)
    return ribbon(c, w, 0, n=n, wfn=lambda t: w * (1 - t) ** 0.9)


def lerp(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def path(d, fill=INK, stroke=None, sw=0, extra=""):
    s = f'<path d="{d}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{sw}" stroke-linejoin="round" stroke-linecap="round"'
    return s + f" {extra}/>"


def tidy(svg_text):
    """Round every number with decimals to one place: far smaller files, no visible change."""
    import re

    def r(m):
        v = round(float(m.group(0)), 1)
        s = f"{v:.1f}".rstrip("0").rstrip(".")
        return "0" if s in ("-0", "") else s

    return re.sub(r' d="([^"]*)"', lambda m: ' d="' + re.sub(r"-?\d+\.\d+", r, m.group(1)) + '"', svg_text)

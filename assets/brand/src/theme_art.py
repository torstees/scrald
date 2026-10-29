"""Art for the Skald theme: the section divider (a carved rule with a round shield at its centre)."""
import os
from wood import *
from vignettes import shield

from paths import SKALD_THEME, write_svg

W, H = 640, 64


def divider():
    cy = H / 2
    g = []
    for sgn in (-1, 1):
        x_in, x_out = W / 2 + sgn * 46, W / 2 + sgn * 300
        # the carved rule: thick by the shield, tapering to a point
        g.append(path(wedge((x_in, cy), (x_out, cy), 9, n=24), INK))
        g.append(path(gouge((x_in + sgn * 20, cy), (x_in + sgn * 150, cy), 2.2), PAPER))
        # a lozenge and a dot along each arm
        lx = W / 2 + sgn * 88
        g.append(f'<rect x="{lx - 8}" y="{cy - 8}" width="16" height="16" transform="rotate(45 {lx} {cy})" '
                 f'fill="{GOLD}" stroke="{INK}" stroke-width="3.5"/>')
        g.append(f'<circle cx="{W / 2 + sgn * 128}" cy="{cy}" r="4.5" fill="{RED}" stroke="{INK}" stroke-width="2.5"/>')
    g.append(f'<g transform="translate({W / 2},{cy}) scale(0.5)">' + shield(0, 0, 54, RED, GOLD, PAPER, INK, rot=12) + "</g>")
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">'
            f'<title>A carved rule with a round shield at its centre</title>{"".join(g)}</svg>')


if __name__ == "__main__":
    write_svg(os.path.join(SKALD_THEME, "assets", "divider.svg"), tidy(divider()))

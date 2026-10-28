"""Future 'rune' package icon: a runestone carved with sowilo (S) inside a serpent band."""
from wood import *
from icon_horn import tile

STONE = "#8a8f98"
STONE_DK = "#626873"


def svg(detail=1.0):
    full = detail >= 1
    g = []
    stone = ("M 300,880 C 250,700 230,420 300,250 C 350,140 470,110 540,120 "
             "C 640,130 730,190 760,300 C 810,470 790,700 740,880 Z")
    g.append(path(stone, STONE, INK, 18))
    if full:
        # carved shadow on the right flank
        for i in range(14):
            y = 260 + i * 44
            g.append(path(wedge((786 - abs(i - 6) * 4, y), (700, y + 24), 18), STONE_DK))
        # serpent band
        band = ("M 330,860 C 300,640 300,420 350,300 C 400,190 500,170 540,172 "
                "C 620,176 690,220 712,320 C 750,480 740,680 712,860")
        g.append(f'<path d="{band}" fill="none" stroke="{INK}" stroke-width="64" stroke-linecap="round"/>')
        g.append(f'<path d="{band}" fill="none" stroke="{RED}" stroke-width="40" stroke-linecap="round"/>')
        for i in range(0, 22):
            pass
        g.append(f'<path d="{band}" fill="none" stroke="{INK}" stroke-width="5" stroke-dasharray="4 28"/>')
        # serpent head
        g.append(path("M 712,860 C 740,830 790,840 800,870 C 770,890 730,890 712,860 Z", RED, INK, 10))
        g.append(f'<circle cx="772" cy="862" r="6" fill="{INK}"/>')
        g.append(f'<line x1="200" y1="882" x2="840" y2="882" stroke="{INK}" stroke-width="18" stroke-linecap="round"/>')
    # sowilo rune
    sow = "M 590,330 L 450,520 L 580,560 L 440,760" if full else "M 610,250 L 420,520 L 600,560 L 420,830"
    g.append(f'<path d="{sow}" fill="none" stroke="{INK}" stroke-width="{80 if full else 120}" stroke-linecap="round" stroke-linejoin="round"/>')
    g.append(f'<path d="{sow}" fill="none" stroke="{RED if not full else PAPER}" stroke-width="{44 if full else 70}" stroke-linecap="round" stroke-linejoin="round"/>')
    return tile("".join(g), carve_border=full)


if __name__ == "__main__":
    from paths import write_icon
    write_icon("rune", svg)

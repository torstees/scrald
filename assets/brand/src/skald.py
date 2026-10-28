"""The Skald: an original, friendly bearded poet drawn as a woodcut. Local frame 500x500."""
import math, random
from wood import *
from icon_horn import horn

STEEL = "#9aa0a8"
STEEL_DK = "#6b717b"
CLOAK = RED
TUNIC = BLUE
BEARD = GOLD
SKIN = PAPER


def outline(d, fill, sw=7, extra=""):
    return path(d, fill, INK, sw, extra)


def strands(rnd, region_pts, n, w=5, color=INK):
    """Carved strands: list of (a,b) pairs -> gouges."""
    return "".join(path(gouge(a, b, w, rnd.uniform(-6, 6)), color) for a, b in region_pts[:n])


def skald(eyes="happy", mouth="sing", glasses=False, uid="a", body=True):
    """The bust. eyes: happy|open|down|wink|squint; mouth: sing|smile|tongue|o|grin."""
    rnd = random.Random(11)
    g = []

    # ---------------- cloak & body ----------------
    if not body:
        pass
    cloak = "M 20,500 C 30,420 70,350 140,330 C 200,316 300,316 360,330 C 430,350 470,420 480,500 Z"
    g.append(outline(cloak, CLOAK, 8))
    # carved folds in cloak
    for (a, b) in [((60, 500), (95, 400)), ((90, 500), (115, 420)), ((440, 500), (405, 400)), ((410, 500), (388, 430))]:
        g.append(path(wedge(a, b, 14), INK))
    tunic = "M 150,500 C 158,430 175,372 205,345 L 295,345 C 325,372 342,430 350,500 Z"
    g.append(outline(tunic, TUNIC, 7))
    # tunic hatching (right side shadow), clipped to the tunic
    hatch = "".join(path(wedge((352, 372 + i * 17), (302, 380 + i * 17), 9), BLUE_DK) for i in range(8))
    g.append(f'<clipPath id="tunicclip{uid}"><path d="{tunic}"/></clipPath><g clip-path="url(#tunicclip{uid})">{hatch}</g>')
    g.append(path(tunic, "none", INK, 7))
    # gold zig-zag trim along neckline
    zz = "M 196,352 " + " ".join(f"L {200 + i * 10},{352 + (8 if i % 2 else 0)}" for i in range(11))
    g.append(f'<path d="{zz}" fill="none" stroke="{GOLD_LT}" stroke-width="5" stroke-linejoin="round"/>')
    # brooch on left shoulder
    g.append(f'<circle cx="150" cy="352" r="22" fill="{GOLD}" stroke="{INK}" stroke-width="7"/>')
    g.append(f'<circle cx="150" cy="352" r="9" fill="{RED}" stroke="{INK}" stroke-width="4"/>')
    g.append(f'<line x1="118" y1="372" x2="184" y2="330" stroke="{INK}" stroke-width="6" stroke-linecap="round"/>')

    # ---------------- beard (behind face parts below the mouth) ----------------
    beard = ("M 168,262 C 160,320 178,388 214,420 C 228,432 240,446 250,452 "
             "C 260,446 272,432 286,420 C 322,388 340,320 332,262 "
             "C 310,300 280,318 250,318 C 220,318 190,300 168,262 Z")
    # braids hang from the beard
    for bx in (226, 274):
        for k in range(4):
            cy = 440 + k * 17
            g.append(f'<ellipse cx="{bx + (k % 2) * 3 - 1.5}" cy="{cy}" rx="{11 - k}" ry="11" '
                     f'fill="{BEARD}" stroke="{INK}" stroke-width="5"/>')
        g.append(f'<rect x="{bx - 9}" y="{440 + 4 * 17 - 6}" width="18" height="14" rx="4" '
                 f'fill="{GOLD_LT}" stroke="{INK}" stroke-width="5"/>')
    g.append(outline(beard, BEARD, 8))
    # beard strands
    pairs = []
    for i in range(11):
        x = 182 + i * 13.6
        top = 300 + abs(x - 250) * -0.3 + 20
        bot = 410 - abs(x - 250) * 0.9
        if bot - top < 30:
            continue
        sx = (x - 250) * 0.15
        pairs.append(((x, top + rnd.uniform(0, 14)), (x - sx + rnd.uniform(-4, 4), bot)))
    g.append(strands(rnd, pairs, 20, w=6))

    # ---------------- face ----------------
    face = "M 172,214 C 168,272 200,312 250,312 C 300,312 332,272 328,214 Z"
    g.append(outline(face, SKIN, 8))
    # shading on the right cheek
    for i in range(6 if eyes in ("happy", "down") else 4):
        y = (228 if eyes in ("happy", "down") else 256) + i * 13
        g.append(path(wedge((326 - i * 1.5, y), (300, y + 8), 7), INK))
    # rosy cheeks
    for cx in (200, 300):
        g.append(f'<ellipse cx="{cx}" cy="272" rx="17" ry="11" fill="{RED}" opacity="0.55"/>')
    g.append(eyes_svg(eyes))
    # bushy brows (carved tufts) peeking under the helmet rim
    for cx, sgn in ((212, -1), (288, 1)):
        for k in range(4):
            a = (cx - 18 + k * 11, 226)
            b = (cx - 18 + k * 11 + sgn * 6, 214)
            g.append(path(gouge(a, b, 8), BEARD))
        g.append(f'<path d="M {cx - 22},{226} Q {cx},{214} {cx + 22},{226}" fill="none" stroke="{INK}" stroke-width="4"/>')

    g.append(mouth_svg(mouth))
    # moustache: two curls
    for sgn in (-1, 1):
        mx = 250
        d = (f"M {mx},300 C {mx + sgn * 30},292 {mx + sgn * 62},300 {mx + sgn * 70},322 "
             f"C {mx + sgn * 74},338 {mx + sgn * 58},344 {mx + sgn * 52},332 "
             f"C {mx + sgn * 50},322 {mx + sgn * 60},320 {mx + sgn * 62},326 "
             f"C {mx + sgn * 48},312 {mx + sgn * 24},314 {mx},318 Z")
        g.append(outline(d, BEARD, 6))
    # nose
    g.append(outline("M 250,250 C 266,256 274,276 268,290 C 262,302 238,302 232,290 C 226,276 234,256 250,250 Z", SKIN, 7))
    g.append(f'<path d="M 256,272 Q 264,280 258,290" fill="none" stroke="{INK}" stroke-width="4"/>')

    if glasses:
        g.append(spectacles())
    # ---------------- helmet ----------------
    dome = "M 160,222 C 158,150 196,104 250,100 C 304,104 342,150 340,222 Z"
    g.append(outline(dome, STEEL, 8))
    # spangen ribs
    g.append(f'<path d="M 250,100 L 250,214" stroke="{INK}" stroke-width="16"/>')
    g.append(f'<path d="M 250,102 L 250,212" stroke="{GOLD}" stroke-width="7"/>')
    # carved highlights on dome (left-lit)
    for i in range(3):
        a = (184 + i * 14, 192 - i * 4)
        b = (204 + i * 12, 146 - i * 6)
        g.append(path(gouge(a, b, 6, -5), PAPER))
    for i in range(5):
        y = 150 + i * 14
        g.append(path(wedge((338 - i * 0.5, y + 10), (306, y + 2), 9), STEEL_DK))
    # brow band with rivets
    g.append(outline("M 154,206 C 200,196 300,196 346,206 L 346,230 C 300,220 200,220 154,230 Z", GOLD, 7))
    for x in range(170, 340, 22):
        g.append(f'<circle cx="{x}" cy="{215}" r="4" fill="{INK}"/>')
    # nasal guard
    g.append(outline("M 242,222 L 258,222 L 256,262 C 254,268 246,268 244,262 Z", STEEL, 6))
    # top knob
    g.append(outline("M 240,104 C 240,88 260,88 260,104 Z", GOLD, 6))

    return "".join(g)


def eyes_svg(kind):
    g = []
    for i, cx in enumerate((212, 288)):
        k = kind
        if kind == "wink":
            k = "happy" if i == 0 else "open"
        if k == "happy":
            g.append(f'<path d="M {cx - 15},246 Q {cx},232 {cx + 15},246" fill="none" stroke="{INK}" '
                     f'stroke-width="7" stroke-linecap="round"/>')
        elif k == "open":
            g.append(f'<ellipse cx="{cx}" cy="244" rx="8" ry="10" fill="{INK}"/>')
            g.append(f'<circle cx="{cx - 3}" cy="240" r="2.8" fill="{PAPER}"/>')
        elif k == "up":
            g.append(f'<ellipse cx="{cx}" cy="244" rx="10" ry="12" fill="{PAPER}" stroke="{INK}" stroke-width="4"/>')
            g.append(f'<circle cx="{cx + 1}" cy="238" r="6" fill="{INK}"/>')
        elif k == "down":
            # lids lowered, looking down at something held in front
            g.append(f'<path d="M {cx - 14},242 Q {cx},236 {cx + 14},242" fill="none" stroke="{INK}" stroke-width="6" stroke-linecap="round"/>')
            g.append(f'<path d="M {cx - 9},245 Q {cx},253 {cx + 9},245 Z" fill="{INK}"/>')
        elif k == "squint":
            g.append(f'<path d="M {cx - 14},240 L {cx + 12},246 L {cx - 14},250" fill="none" stroke="{INK}" '
                     f'stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>' if i == 0 else
                     f'<path d="M {cx + 14},240 L {cx - 12},246 L {cx + 14},250" fill="none" stroke="{INK}" '
                     f'stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>')
    return "".join(g)


def mouth_svg(kind):
    if kind == "sing":
        return (outline("M 228,322 C 230,354 270,354 272,322 C 258,316 242,316 228,322 Z", "#3a1f1c", 6)
                + f'<ellipse cx="250" cy="340" rx="12" ry="6" fill="{RED}"/>')
    if kind == "grin":
        return (outline("M 226,322 C 232,346 268,346 274,322 C 258,318 242,318 226,322 Z", "#3a1f1c", 6)
                + f'<path d="M 230,324 L 270,324 L 266,331 L 234,331 Z" fill="{PAPER}"/>')
    if kind == "o":
        return outline("M 242,330 C 242,316 258,316 258,330 C 258,344 242,344 242,330 Z", "#3a1f1c", 5)
    if kind == "tongue":
        return (f'<path d="M 232,332 Q 250,346 270,332" fill="none" stroke="{INK}" stroke-width="6" stroke-linecap="round"/>'
                + outline("M 246,336 C 246,358 224,362 222,340 C 228,334 238,334 246,336 Z", RED, 5))
    # smile
    return f'<path d="M 230,325 Q 250,342 270,325" fill="none" stroke="{INK}" stroke-width="6" stroke-linecap="round"/>'


def spectacles():
    g = []
    for cx in (212, 288):
        g.append(f'<circle cx="{cx}" cy="246" r="21" fill="{PAPER}" fill-opacity="0.25" stroke="{INK}" stroke-width="5"/>')
    g.append(f'<path d="M 233,244 Q 250,236 267,244" fill="none" stroke="{INK}" stroke-width="5"/>')
    return "".join(g)


def sleeve(c, w0=48, w1=40, color=None):
    """An arm: a sleeve ribbon along cubic c (shoulder -> wrist) with a gold cuff."""
    color = color or TUNIC
    d = ribbon(c, w0, w1, n=30)
    cuff = ribbon(c, w1 + 4, w1 + 4, n=4, t0=0.9, t1=1.0)
    return path(d, color, INK, 7) + path(cuff, GOLD, INK, 6)


def fist(x, y, rot=0, s=1.0):
    g = [f'<g transform="translate({x},{y}) rotate({rot}) scale({s})">',
         path("M -24,4 C -28,-20 -10,-32 6,-28 C 26,-24 30,-6 24,10 C 16,22 -20,22 -24,4 Z", SKIN, INK, 7)]
    for k in range(3):
        yy = -16 + k * 11
        g.append(f'<path d="M 0,{yy} Q 14,{yy + 2} 22,{yy + 8}" fill="none" stroke="{INK}" stroke-width="4"/>')
    g.append("</g>")
    return "".join(g)


def open_hand(x, y, rot=0, s=1.0):
    """A raised open palm, fingers up (in the hand's frame), wrist at (0,0)."""
    g = [f'<g transform="translate({x},{y}) rotate({rot}) scale({s})">']
    for (fx, h, a) in ((-17, 38, -12), (-6, 46, -4), (6, 46, 4), (17, 38, 12)):
        g.append(f'<rect x="{fx - 6.5}" y="{-30 - h}" width="13" height="{h + 8}" rx="6.5" '
                 f'transform="rotate({a} {fx} -30)" fill="{SKIN}" stroke="{INK}" stroke-width="6"/>')
    g.append(path("M -26,-36 C -30,-10 -20,4 0,4 C 20,4 30,-10 26,-36 C 12,-42 -12,-42 -26,-36 Z", SKIN, INK, 7))
    g.append(path("M 22,-12 C 34,-20 46,-36 52,-30 C 58,-24 44,-6 26,4 Z", SKIN, INK, 6))
    g.append(f'<path d="M -14,-14 Q 0,-6 12,-16" fill="none" stroke="{INK}" stroke-width="3.5"/>')
    g.append("</g>")
    return "".join(g)


def raised_arm_with_horn(scale=0.30):
    """Right arm (viewer's right) raising the quill-horn in a toast."""
    g = []
    # horn: reuse the icon's horn, rotated so the mouth is up and the nib points down
    grip_icon = (472, 430)  # roughly mid-horn in the icon's frame
    hand = (420, 222)
    g.append(f'<g transform="translate({hand[0]},{hand[1]}) rotate(-38) scale({scale}) '
             f'translate({-grip_icon[0]},{-grip_icon[1]})">{horn(1.0)}</g>')
    # sleeve from shoulder to hand
    sleeve = "M 372,346 C 400,330 432,300 436,250 L 400,240 C 396,280 380,300 350,320 Z"
    g.append(path(sleeve, TUNIC, INK, 7))
    g.append(f'<path d="M 398,244 L 438,254" stroke="{GOLD_LT}" stroke-width="6"/>')
    # fist wrapped around horn
    fist = "M 398,236 C 394,212 412,198 430,202 C 450,206 456,224 448,240 C 440,252 404,252 398,236 Z"
    g.append(path(fist, SKIN, INK, 7))
    for k in range(3):
        y = 212 + k * 11
        g.append(f'<path d="M 424,{y} Q 438,{y + 2} 446,{y + 8}" fill="none" stroke="{INK}" stroke-width="4"/>')
    return "".join(g)


def open_book():
    """An open book held against the chest (viewer's left)."""
    g = []
    # cover
    g.append(path("M 46,420 L 120,388 L 214,410 L 218,476 L 124,458 L 50,488 Z", RED, INK, 7))
    # pages
    left = "M 58,420 C 80,404 104,398 124,404 L 126,456 C 104,450 82,456 62,474 Z"
    right = "M 124,404 C 148,398 180,402 206,416 L 206,470 C 180,458 150,452 126,456 Z"
    g.append(path(left, PAPER, INK, 6))
    g.append(path(right, PAPER, INK, 6))
    for k in range(4):
        y = 420 + k * 10
        g.append(f'<path d="M 70,{y + 8 - k * 0.5} Q 96,{y - 4} 118,{y}" fill="none" stroke="{INK}" stroke-width="3"/>')
        g.append(f'<path d="M 132,{y} Q 160,{y - 4} 196,{y + 8}" fill="none" stroke="{INK}" stroke-width="3"/>')
    # a tiny illuminated initial
    g.append(f'<rect x="134" y="410" width="12" height="12" fill="{GOLD}" stroke="{INK}" stroke-width="2"/>')
    # forearm coming up from below, hand cradling the spine
    g.append(path("M 96,500 C 104,486 118,474 134,466 L 170,480 C 160,490 150,496 146,500 Z", TUNIC, INK, 7))
    g.append(path("M 118,470 C 120,452 152,450 166,458 C 176,466 170,482 156,484 C 140,486 120,484 118,470 Z", SKIN, INK, 6))
    for k in range(3):
        x = 132 + k * 11
        g.append(f'<path d="M {x},{458} Q {x + 3},{470} {x},{482}" fill="none" stroke="{INK}" stroke-width="3.5"/>')
    return "".join(g)


def figure():
    return skald() + open_book() + raised_arm_with_horn()


if __name__ == "__main__":
    import os
    from paths import BRAND, write_png, write_svg
    # The mascot on a transparent background, cropped to the figure.
    s = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="-20 30 540 470" width="540" height="470">'
         f"<title>The Skald, Scrald's mascot</title>{figure()}</svg>")
    write_svg(os.path.join(BRAND, "skald.svg"), s)
    write_png(os.path.join(BRAND, "skald.png"), s, 1080)

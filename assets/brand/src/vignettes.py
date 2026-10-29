"""Docs spot art: round woodcut plates with the Skald in different poses."""
import math, random
from wood import *
from textpath import text_path
from icon_horn import horn
from skald import skald, sleeve, fist, open_hand, open_book, SKIN, TUNIC, STEEL, STEEL_DK, outline

import os
from paths import DOCS_ART, UI_ART, write_svg

P = 400  # plate size
R = 186  # plate radius


def plate(scene, title, uid, bg_extra="", label=None):
    """A round paper plate with a double ink rim; the scene is clipped to the inner circle."""
    c = P / 2
    rnd = random.Random(len(uid))
    grain = []
    for i in range(16):
        y = 40 + i * 21 + rnd.uniform(-5, 5)
        x0 = rnd.uniform(40, 260)
        grain.append(path(gouge((x0, y), (x0 + rnd.uniform(40, 110), y + rnd.uniform(-3, 3)), rnd.uniform(1.5, 3)), PAPER_SHADE))
    body = (f'<clipPath id="plate-{uid}"><circle cx="{c}" cy="{c}" r="{R - 8}"/></clipPath>'
            f'<circle cx="{c}" cy="{c}" r="{R}" fill="{PAPER}"/>'
            f'<g clip-path="url(#plate-{uid})">{"".join(grain)}{bg_extra}{scene}</g>'
            f'<circle cx="{c}" cy="{c}" r="{R - 8}" fill="none" stroke="{INK}" stroke-width="2.5"/>'
            f'<circle cx="{c}" cy="{c}" r="{R}" fill="none" stroke="{INK}" stroke-width="9"/>')
    if label:
        body += ribbon_label(label, uid)
    return tidy(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {P} {P}" width="{P}" height="{P}">'
                f'<title>{title}</title>{body}</svg>')


def ribbon_label(text, uid, y=346):
    """A banderole across the bottom of the plate, as on old prints."""
    d, w = text_path(text, "im-fell-english", 26, P / 2, y + 9, anchor="middle")
    half = w / 2 + 28
    x0, x1 = P / 2 - half, P / 2 + half
    g = [path(f"M {x0 - 26},{y - 12} L {x0 + 4},{y - 14} L {x0 + 4},{y + 18} L {x0 - 26},{y + 20} L {x0 - 14},{y + 3} Z", PAPER_SHADE, INK, 5),
         path(f"M {x1 + 26},{y - 12} L {x1 - 4},{y - 14} L {x1 - 4},{y + 18} L {x1 + 26},{y + 20} L {x1 + 14},{y + 3} Z", PAPER_SHADE, INK, 5),
         path(f"M {x0},{y - 20} Q {P / 2},{y - 28} {x1},{y - 20} L {x1},{y + 14} Q {P / 2},{y + 6} {x0},{y + 14} Z", PAPER, INK, 5),
         f'<path d="{d}" fill="{INK}"/>']
    return "".join(g)


def rays(cx, cy, r0, r1, n=28, color=PAPER_SHADE):
    g = []
    for i in range(n):
        a = 2 * math.pi * i / n
        p0 = (cx + r0 * math.cos(a), cy + r0 * math.sin(a))
        p1 = (cx + r1 * math.cos(a), cy + r1 * math.sin(a))
        g.append(path(wedge(p1, p0, 14 if i % 2 == 0 else 8), color))
    return "".join(g)


def place(inner, x=0, y=0, s=1.0):
    return f'<g transform="translate({x},{y}) scale({s})">{inner}</g>'


BUST = dict(x=0, y=-14, s=0.8)  # 500-frame bust placed in the plate (head centred)


def bust(*parts, **kw):
    b = dict(BUST)
    b.update(kw)
    return place("".join(parts), b["x"], b["y"], b["s"])


# ------------------------------------------------------------------ scenes

def getting_started():
    """Waving hello, horn in the other hand."""
    g = []
    g.append(skald(eyes="open", mouth="sing", uid="gs"))
    # left arm (viewer's left) holding the horn low across the body
    g.append(f'<g transform="translate(118,380) rotate(-80) scale(0.24) translate(-472,-430)">{horn(1.0)}</g>')
    g.append(sleeve(((120, 350), (90, 390), (95, 430), (120, 440))))
    g.append(fist(126, 430, -100, 1.0))
    # right arm raised, waving
    g.append(sleeve(((368, 352), (404, 330), (414, 300), (410, 262))))
    g.append(open_hand(410, 266, 14, 1.1))
    # motion marks beside the hand
    for r in (88, 106):
        a0, a1 = math.radians(-100), math.radians(-40)
        cx, cy = 410, 266
        g.append(f'<path d="M {cx + r * math.cos(a0):.1f},{cy + r * math.sin(a0):.1f} A {r} {r} 0 0 1 '
                 f'{cx + r * math.cos(a1):.1f},{cy + r * math.sin(a1):.1f}" fill="none" stroke="{INK}" '
                 f'stroke-width="5" stroke-linecap="round" transform="rotate(14 {cx} {cy})"/>')
    scene = bust("".join(g))
    return plate(scene, "The Skald waves hello", "gs", rays(200, 170, 60, 200), label="Heill!")


def reading():
    """Spectacles on, absorbed in a big book (we see its cover)."""
    g = [skald(eyes="down", mouth="o", glasses=True, uid="rd")]
    # the big book: cover toward us
    cover = "M 92,352 L 408,352 L 412,500 L 88,500 Z"
    g.append(path(cover, RED, INK, 8))
    # page block peeking at the top edge
    g.append(path("M 100,352 C 160,340 230,338 250,348 C 270,338 340,340 400,352 L 400,360 L 100,360 Z", PAPER, INK, 6))
    # carved border and gold bosses
    g.append(f'<rect x="112" y="372" width="276" height="140" fill="none" stroke="{GOLD_LT}" stroke-width="5"/>')
    for (x, y) in ((112, 372), (388, 372)):
        g.append(f'<circle cx="{x}" cy="{y}" r="12" fill="{GOLD}" stroke="{INK}" stroke-width="5"/>')
    # cover medallion: a gold lozenge with a red S
    g.append(f'<rect x="215" y="390" width="70" height="70" transform="rotate(45 250 425)" fill="{GOLD}" stroke="{INK}" stroke-width="6"/>')
    d, _ = text_path("S", "unifrakturmaguntia", 64, 250, 446, anchor="middle")
    g.append(f'<path d="{d}" fill="{RED}" stroke="{INK}" stroke-width="2"/>')
    # shading on the cover
    for i in range(8):
        y = 372 + i * 16
        g.append(path(wedge((410, y), (380, y + 10), 9), INK))
    # hands gripping the top corners, fingers over the edge
    for x, sgn in ((96, 1), (404, -1)):
        g.append(path(f"M {x - sgn * 4},{376} C {x - sgn * 4},{350} {x + sgn * 30},{340} {x + sgn * 36},{354} "
                      f"C {x + sgn * 40},{370} {x + sgn * 24},{386} {x + sgn * 8},{388} Z", SKIN, INK, 6))
        for k in range(3):
            fx = x + sgn * (10 + k * 9)
            g.append(f'<path d="M {fx},{350 + k} L {fx},{364}" stroke="{INK}" stroke-width="3.5" stroke-linecap="round"/>')
    scene = bust("".join(g))
    return plate(scene, "The Skald reads a big book", "rd", rays(200, 170, 60, 200))


def editing():
    """Writing with the quill-horn, tongue out in concentration."""
    g = [skald(eyes="down", mouth="tongue", uid="ed")]
    # desk and the sheet he is writing on
    g.append(path("M 0,440 L 500,430 L 500,520 L 0,520 Z", "#8a5a34", INK, 8))
    for i in range(7):
        x = 20 + i * 70
        g.append(path(gouge((x, 470), (x + 54, 467), 4), "#6e4527"))
    g.append(path("M 96,378 L 384,370 L 396,452 L 88,460 Z", PAPER, INK, 7))
    for k, L in enumerate((250, 236, 244, 150)):
        y = 394 + k * 15
        g.append(f'<path d="M 110,{y} q {L * 0.25},-4 {L * 0.5},0 t {L * 0.5},0" fill="none" '
                 f'stroke="{INK}" stroke-width="3.5" stroke-linecap="round"/>')
    # left hand holding the page down
    g.append(sleeve(((128, 352), (88, 380), (92, 420), (120, 432)), 46, 38))
    g.append(fist(130, 428, -70, 0.95))
    # right arm, then the horn held like a pen (nib at the end of the fresh line), then the fist
    nib = (268, 446)
    g.append(sleeve(((372, 352), (416, 386), (380, 432), (322, 438)), 48, 40))
    g.append(f'<g transform="translate({nib[0]},{nib[1]}) rotate(-12) scale(0.30) translate(-262,-760)">{horn(1.0)}</g>')
    g.append(fist(306, 418, 160, 0.95))
    scene = bust("".join(g))
    return plate(scene, "The Skald writes with his quill-horn", "ed", rays(200, 170, 60, 200))


SCENES = {
    "getting-started": getting_started,
    "reading": reading,
    "editing": editing,
}


def scroll(x, y, rot, seal, s=1.0):
    """A rolled scroll with a wax seal, centred at (x,y)."""
    g = [f'<g transform="translate({x},{y}) rotate({rot}) scale({s})">',
         path("M -40,-14 L 40,-14 L 40,14 L -40,14 Z", PAPER, INK, 6),
         f'<ellipse cx="-40" cy="0" rx="8" ry="16" fill="{PAPER_SHADE}" stroke="{INK}" stroke-width="6"/>',
         f'<ellipse cx="40" cy="0" rx="8" ry="16" fill="{PAPER_SHADE}" stroke="{INK}" stroke-width="6"/>',
         f'<path d="M -24,-14 L -24,14 M 24,-14 L 24,14" stroke="{INK}" stroke-width="3"/>',
         f'<path d="M -6,14 L -10,30 L -2,26 L 2,32 L 4,14 Z" fill="{seal}" stroke="{INK}" stroke-width="3"/>',
         f'<circle cx="0" cy="8" r="12" fill="{seal}" stroke="{INK}" stroke-width="5"/>',
         "</g>"]
    return "".join(g)


def shield(x, y, r, field, paint, boss, ring, rot=0):
    """A round Viking shield painted in a theme's colours."""
    g = [f'<g transform="translate({x},{y}) rotate({rot})">',
         f'<circle r="{r}" fill="{field}"/>']
    for a0 in (0, 180):
        a1 = a0 + 90
        p0 = (r * math.cos(math.radians(a0)), r * math.sin(math.radians(a0)))
        p1 = (r * math.cos(math.radians(a1)), r * math.sin(math.radians(a1)))
        g.append(f'<path d="M 0,0 L {p0[0]:.1f},{p0[1]:.1f} A {r} {r} 0 0 1 {p1[0]:.1f},{p1[1]:.1f} Z" fill="{paint}"/>')
    for k in (-0.5, 0, 0.5):  # planks
        g.append(f'<line x1="{k * r:.1f}" y1="{-r * math.sqrt(1 - k * k):.1f}" x2="{k * r:.1f}" y2="{r * math.sqrt(1 - k * k):.1f}" '
                 f'stroke="{INK}" stroke-width="2" opacity="0.35"/>')
    g.append(f'<circle r="{r - 7}" fill="none" stroke="{ring}" stroke-width="5"/>')
    g.append(f'<circle r="{r}" fill="none" stroke="{INK}" stroke-width="7"/>')
    g.append(f'<circle r="{r * 0.26:.1f}" fill="{boss}" stroke="{INK}" stroke-width="6"/>')
    g.append(f'<path d="M {-r * 0.1:.1f},{-r * 0.12:.1f} q {r * 0.06:.1f},{-r * 0.06:.1f} {r * 0.14:.1f},{-r * 0.02:.1f}" '
             f'fill="none" stroke="{PAPER}" stroke-width="3" stroke-linecap="round"/>')
    for a in range(0, 360, 45):
        g.append(f'<circle cx="{(r - 7) * math.cos(math.radians(a)):.1f}" cy="{(r - 7) * math.sin(math.radians(a)):.1f}" r="2.6" fill="{INK}"/>')
    g.append("</g>")
    return "".join(g)


# theme colours: (background, foreground, accent, link)
THEMES = {
    "scrald-light": ("#fbf8f1", "#2b2a27", "#8a5a2b", "#2f5f8a"),
    "scrald-dark": ("#22211f", "#e6e1d6", "#d6a463", "#8fb6d9"),
    "sepia": ("#f4ecd8", "#433422", "#895028", "#b9935e"),
    "technical": ("#ffffff", "#1f2328", "#0965d1", "#0969da"),
    "technical-dark": ("#0d1117", "#e6edf3", "#4493f8", "#3d444d"),
}


def printing():
    """Proudly holding up a fresh print; an ink brayer in the other hand."""
    g = [skald(eyes="happy", mouth="grin", uid="pr")]
    g.append('<ellipse cx="300" cy="286" rx="13" ry="6" fill="#232733" opacity="0.8" transform="rotate(-20 300 286)"/>')
    # left hand: brayer (ink roller)
    g.append(sleeve(((128, 352), (90, 390), (96, 430), (126, 440)), 46, 38))
    g.append(f'<line x1="126" y1="434" x2="70" y2="380" stroke="{INK}" stroke-width="16" stroke-linecap="round"/>')
    g.append(f'<line x1="126" y1="434" x2="70" y2="380" stroke="#8a5a34" stroke-width="7" stroke-linecap="round"/>')
    g.append(f'<path d="M 70,380 L 52,398 M 70,380 L 88,362" stroke="{INK}" stroke-width="6"/>')
    g.append(f'<rect x="10" y="352" width="44" height="74" rx="18" transform="rotate(45 32 389)" fill="{INK}"/>')
    g.append(f'<path d="M 22,372 L 44,394" stroke="{STEEL}" stroke-width="4" stroke-linecap="round"/>')
    g.append(fist(130, 436, -60, 0.95))
    # right hand holding up the print
    g.append(sleeve(((372, 352), (414, 336), (420, 300), (404, 270)), 48, 40))
    sheet = "M 316,96 L 482,108 L 470,300 L 304,286 Z"
    g.append(path(sheet, PAPER, INK, 7))
    # the print on the sheet: a little woodcut of the initial
    g.append(f'<g transform="translate(330,118) rotate(4) scale(0.13)">'
             f'<rect x="210" y="200" width="380" height="380" fill="{GOLD}" stroke="{INK}" stroke-width="30"/></g>')
    d, _ = text_path("S", "unifrakturmaguntia", 46, 357, 182, anchor="middle")
    g.append(f'<path d="{d}" fill="{RED}" transform="rotate(4 357 160)"/>')
    for k, L in enumerate((70, 64, 140, 132, 120, 80)):
        y = 150 + k * 20 if k < 2 else 210 + (k - 2) * 20
        x = 400 if k < 2 else 330
        g.append(f'<path d="M {x},{y + (x - 330) * 0.07:.1f} l {L},{L * 0.07:.1f}" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>')
    g.append(f'<path d="M 470,300 L 304,286 L 302,296 L 468,310 Z" fill="{PAPER_SHADE}" stroke="{INK}" stroke-width="4"/>')
    g.append(fist(398, 276, -140, 0.95))
    # sparkle
    for (cx, cy) in ((290, 88), (488, 330)):
        g.append(f'<path d="M {cx},{cy - 16} L {cx + 4},{cy - 4} L {cx + 16},{cy} L {cx + 4},{cy + 4} L {cx},{cy + 16} '
                 f'L {cx - 4},{cy + 4} L {cx - 16},{cy} L {cx - 4},{cy - 4} Z" fill="{GOLD}" stroke="{INK}" stroke-width="3"/>')
    return plate(bust("".join(g)), "The Skald shows off a fresh print", "pr", rays(200, 170, 60, 200))


def themes():
    """Behind a row of shields painted as the built-in themes, brush in hand."""
    g = [skald(eyes="open", mouth="grin", uid="th")]
    # brush raised in the right hand
    g.append(sleeve(((372, 352), (414, 330), (424, 290), (414, 250)), 48, 40))
    g.append(f'<line x1="410" y1="262" x2="452" y2="120" stroke="{INK}" stroke-width="18" stroke-linecap="round"/>')
    g.append(f'<line x1="410" y1="262" x2="452" y2="120" stroke="#8a5a34" stroke-width="9" stroke-linecap="round"/>')
    g.append(path("M 444,138 L 462,142 L 470,96 C 466,82 456,80 452,94 Z", "#d9c7a0", INK, 5))
    g.append(path("M 452,94 C 456,80 466,82 470,96 L 467,112 L 454,108 Z", RED, INK, 4))
    g.append(f'<path d="M 472,112 C 480,124 478,134 472,138" fill="none" stroke="{RED}" stroke-width="5" stroke-linecap="round"/>')
    g.append(fist(412, 258, -150, 0.95))
    # shields: back pair, then front three
    T = THEMES
    pos = [("scrald-dark", 74, 376, 56), ("technical-dark", 426, 376, 56),
           ("sepia", 154, 410, 60), ("scrald-light", 250, 420, 62), ("technical", 346, 410, 60)]
    for name, x, y, r in pos:
        bg, fg, acc, link = T[name]
        g.append(shield(x, y, r, bg, fg, acc, link, rot=-20 + x * 0.08))
    return plate(bust("".join(g)), "The Skald with shields painted in Scrald's themes", "th", rays(200, 170, 60, 200))


def front_matter():
    """Proclaiming a document's particulars from an unrolled scroll, like a herald."""
    g = [skald(eyes="open", mouth="sing", uid="fm")]
    # the scroll sheet hanging between two rollers
    g.append(path("M 106,356 L 394,356 L 394,520 L 106,520 Z", PAPER, INK, 7))
    for x in (106, 394):
        g.append(f'<rect x="{x - 8}" y="356" width="16" height="190" fill="{PAPER_SHADE}" stroke="{INK}" stroke-width="6"/>')
    g.append(f'<rect x="88" y="344" width="324" height="20" rx="10" fill="#8a5a34" stroke="{INK}" stroke-width="7"/>')
    for x in (88, 412):
        g.append(f'<circle cx="{x}" cy="354" r="13" fill="{GOLD}" stroke="{INK}" stroke-width="6"/>')
    # the YAML fence and keys
    g.append(f'<path d="M 170,376 L 214,376" stroke="{INK}" stroke-width="6" stroke-linecap="round" stroke-dasharray="10 7"/>')
    for k, key in enumerate(("title:", "author:", "tags:")):
        y = 406 + k * 30
        d, w = text_path(key, "im-fell-english", 27, 170, y)
        g.append(f'<path d="{d}" fill="{RED}"/>')
        g.append(f'<path d="M {180 + w},{y - 8} q 40,-5 80,0 t {100 - k * 25},0" fill="none" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>')
    # hands holding the top roller
    g.append(sleeve(((128, 352), (100, 376), (96, 358), (104, 334)), 46, 38))
    g.append(fist(104, 342, -170, 0.9))
    g.append(sleeve(((372, 352), (400, 376), (404, 358), (396, 334)), 48, 40))
    g.append(fist(396, 342, 170, 0.9))
    return plate(bust("".join(g)), "The Skald proclaims a document's front matter", "fm", rays(200, 170, 60, 200))


def markdown():
    """Juggling three sealed scrolls: one for each flavor."""
    g = [skald(eyes="up", mouth="o", uid="md")]
    # juggling arcs
    g.append(f'<path d="M 110,300 C 120,60 380,60 390,300" fill="none" stroke="{INK}" stroke-width="4" stroke-dasharray="2 14" stroke-linecap="round"/>')
    g.append(sleeve(((128, 352), (90, 340), (84, 320), (100, 296)), 46, 38))
    g.append(open_hand(100, 300, -20, 0.9))
    g.append(sleeve(((372, 352), (410, 340), (416, 320), (400, 296)), 48, 40))
    g.append(open_hand(400, 300, 20, 0.9))
    g.append(scroll(130, 150, -40, BLUE))
    g.append(scroll(250, 88, 10, "#6b4a8a"))
    g.append(scroll(372, 150, 50, RED))
    return plate(bust("".join(g)), "The Skald juggles three scrolls, one per Markdown flavor", "md", rays(200, 170, 60, 200))


def your_data():
    """Guarding a locked chest, key on a cord, with a wink."""
    g = [skald(eyes="wink", mouth="smile", uid="dt")]
    # the chest in front
    lid = "M 70,380 C 70,330 430,330 430,380 L 430,400 L 70,400 Z"
    box = "M 70,400 L 430,400 L 430,520 L 70,520 Z"
    g.append(path(box, "#8a5a34", INK, 8))
    g.append(path(lid, "#9a6a40", INK, 8))
    for i in range(5):
        y = 420 + i * 22
        g.append(path(gouge((90, y), (200, y + 3), 4), "#6e4527"))
        g.append(path(gouge((300, y), (410, y - 2), 4), "#6e4527"))
    for x in (110, 390):  # iron bands
        g.append(path(f"M {x - 14},346 L {x + 14},344 L {x + 14},520 L {x - 14},520 Z", STEEL_DK, INK, 6))
        for y in (370, 430, 490):
            g.append(f'<circle cx="{x}" cy="{y}" r="4" fill="{INK}"/>')
    # lock plate and keyhole
    g.append(path("M 220,388 L 280,388 L 280,452 C 280,466 220,466 220,452 Z", GOLD, INK, 7))
    g.append(path("M 250,408 C 262,408 262,424 254,426 L 258,446 L 242,446 L 246,426 C 238,424 238,408 250,408 Z", INK))
    # arms resting on the lid
    g.append(sleeve(((128, 352), (110, 370), (130, 372), (170, 368)), 46, 38))
    g.append(fist(180, 364, 100, 0.9))
    g.append(sleeve(((372, 352), (390, 370), (370, 372), (330, 368)), 48, 40))
    g.append(fist(320, 364, -100, 0.9))
    # key on a cord round his neck
    g.append(f'<path d="M 214,350 C 220,380 250,392 290,374" fill="none" stroke="{INK}" stroke-width="4"/>')
    g.append(f'<g transform="translate(290,374) rotate(-30)"><circle r="12" fill="none" stroke="{INK}" stroke-width="10"/>'
             f'<circle r="12" fill="none" stroke="{GOLD}" stroke-width="5"/>'
             f'<path d="M 12,0 L 52,0 M 40,0 L 40,12 M 50,0 L 50,10" stroke="{INK}" stroke-width="9" stroke-linecap="round"/>'
             f'<path d="M 12,0 L 52,0 M 40,0 L 40,12 M 50,0 L 50,10" stroke="{GOLD}" stroke-width="4" stroke-linecap="round"/></g>')
    return plate(bust("".join(g)), "The Skald guards a locked chest", "dt", rays(200, 170, 60, 200))


SCENES.update({
    "printing": printing,
    "themes": themes,
    "front-matter": front_matter,
    "markdown": markdown,
    "data": your_data,
})


def keycap(x, y, label, s=1.0, rot=0):
    d, w = text_path(label, "im-fell-english", 30, 0, 10, anchor="middle")
    return (f'<g transform="translate({x},{y}) rotate({rot}) scale({s})">'
            + path("M -44,-30 L 44,-30 L 52,34 L -52,34 Z", PAPER_SHADE, INK, 6)
            + f'<rect x="-40" y="-36" width="80" height="58" rx="10" fill="{PAPER}" stroke="{INK}" stroke-width="6"/>'
            + f'<path d="{d}" fill="{INK}"/>'
            + "</g>")


def raven(x, y, s=1.0, rot=0, flap=0):
    """A woodcut raven in flight, facing left: black with carved feather lines."""
    g = [f'<g transform="translate({x},{y}) rotate({rot}) scale({s})">']
    # far wing (behind)
    far = (f"M 10,-10 C 30,-70 70,-{110 - flap} 120,-{120 - flap} C 112,-{100 - flap} 118,-{88 - flap} 108,-{78 - flap} "
           f"C 112,-{66 - flap} 104,-{58 - flap} 96,-{50 - flap} C 96,-40 80,-30 60,-16 Z")
    g.append(path(far, "#3a3f4b", INK, 5))
    # tail
    g.append(path("M 60,6 L 132,-6 L 126,8 L 136,18 L 124,24 L 130,36 L 60,26 Z", INK))
    # body
    g.append(path("M -40,0 C -30,-26 20,-30 66,-8 C 80,0 80,18 66,26 C 30,40 -20,32 -40,0 Z", INK))
    # head and beak
    g.append(path("M -38,-6 C -50,-30 -86,-30 -92,-10 C -94,4 -76,16 -46,14 Z", INK))
    g.append(path("M -88,-16 L -124,-6 L -88,0 Z", INK))
    g.append(f'<path d="M -124,-6 L -94,-8" stroke="{STEEL}" stroke-width="2"/>')
    g.append(f'<circle cx="-70" cy="-14" r="6" fill="{PAPER}"/><circle cx="-71" cy="-14" r="3" fill="{INK}"/>')
    # carved feather lines on the body
    for i in range(6):
        g.append(path(gouge((-20 + i * 14, 4 + (i % 2) * 6), (-4 + i * 14, 18 + (i % 2) * 6), 3.2), PAPER))
    for i in range(4):
        g.append(path(gouge((76 + i * 12, 2 + i * 6), (124, 2 + i * 7), 2.6), "#5a6070"))
    # near wing (front), with carved primaries
    near = (f"M 0,-4 C 10,-60 40,-{118 - flap} 92,-{140 - flap} C 86,-{120 - flap} 94,-{108 - flap} 82,-{96 - flap} "
            f"C 90,-{84 - flap} 80,-{74 - flap} 72,-{66 - flap} C 76,-{54 - flap} 66,-46 56,-38 C 50,-24 34,-12 20,-2 Z")
    g.append(path(near, INK))
    for i in range(5):
        t = i / 5
        a = (18 + 40 * t, -16 - 40 * t)
        b = (34 + 52 * t, -60 - (70 - flap * 0.6) * t)
        g.append(path(gouge(a, b, 4), "#7a8090"))
    # talons
    g.append(f'<path d="M 8,28 L 4,46 M 24,30 L 26,48" stroke="{INK}" stroke-width="6" stroke-linecap="round"/>')
    g.append("</g>")
    return "".join(g)


def shortcuts():
    """Huginn and Muninn, Odin's ravens, carrying keycaps."""
    g = []
    g.append(raven(118, 104, 0.55, -6, flap=10))
    g.append(f'<path d="M 118,126 L 116,142" stroke="{INK}" stroke-width="3"/>')
    g.append(keycap(116, 160, "Esc", 0.6, -4))
    g.append(raven(232, 214, 1.0, -4))
    # the key hangs from the big raven's talons on a short cord
    g.append(f'<path d="M 240,262 L 238,282" stroke="{INK}" stroke-width="4"/>')
    g.append(keycap(236, 312, "Ctrl", 1.0, -3))
    # speed lines
    for (x, y, L) in ((330, 160, 60), (346, 196, 44), (334, 244, 50), (210, 104, 34)):
        g.append(f'<path d="M {x},{y} l {L},0" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>')
    return plate("".join(g), "Odin's ravens carry keyboard keys", "sc", rays(200, 200, 70, 200))


def anvil_scene():
    """A shield on the anvil: hammered into shape, sparks flying."""
    g = []
    # stump
    g.append(path("M 110,300 L 290,300 L 300,400 L 100,400 Z", "#8a5a34", INK, 8))
    for i in range(4):
        g.append(path(gouge((124 + i * 42, 316), (128 + i * 42, 392), 5), "#6e4527"))
    g.append(f'<ellipse cx="200" cy="300" rx="92" ry="12" fill="#9a6a40" stroke="{INK}" stroke-width="7"/>')
    # anvil
    anvil = ("M 70,214 L 240,214 C 280,214 316,206 346,196 C 330,226 300,240 268,244 "
             "L 258,262 C 268,272 270,286 278,294 L 136,294 C 144,286 148,272 158,262 L 150,244 C 110,244 82,234 70,214 Z")
    g.append(path(anvil, STEEL_DK, INK, 8))
    g.append(path("M 70,214 L 240,214 C 280,214 316,206 346,196 L 340,206 C 310,216 280,222 240,222 L 78,222 Z", STEEL, INK, 4))
    for i in range(5):
        g.append(path(wedge((164 + i * 16, 290), (176 + i * 16, 254), 7), INK))
    # the shield lying on the anvil, seen at an angle
    g.append('<g transform="translate(176,190) scale(1,0.42)">'
             + shield(0, 0, 74, "#fbf8f1", "#2b2a27", GOLD, BLUE) + "</g>")
    # hammer mid-swing
    g.append(f'<line x1="312" y1="44" x2="232" y2="128" stroke="{INK}" stroke-width="18" stroke-linecap="round"/>')
    g.append(f'<line x1="312" y1="44" x2="232" y2="128" stroke="#8a5a34" stroke-width="9" stroke-linecap="round"/>')
    g.append(path("M 206,104 L 236,74 L 278,116 L 248,146 Z", STEEL, INK, 7))
    g.append(path("M 206,104 L 236,74 L 228,66 L 198,96 Z", STEEL_DK, INK, 5))
    # sparks
    rnd = random.Random(4)
    for i in range(9):
        a = math.radians(-160 + i * 20)
        r0, r1 = 40, 70 + rnd.uniform(0, 30)
        cx, cy = 210, 172
        p0 = (cx + r0 * math.cos(a), cy + r0 * math.sin(a) * 0.8)
        p1 = (cx + r1 * math.cos(a), cy + r1 * math.sin(a) * 0.8)
        if p1[1] > 190:
            continue
        g.append(path(wedge(p0, p1, 7), GOLD))
        g.append(path(wedge(p0, p1, 7), "none", INK, 2))
    return plate("".join(g), "A shield hammered out on the anvil", "tr", rays(200, 180, 60, 200))


def about():
    """The banner's pose on a plate: horn raised, book in hand. Skål!"""
    from skald import figure
    return plate(bust(figure(), x=40, y=74, s=0.64), "The Skald raises his horn: Skål!", "ab", rays(200, 170, 60, 200), label="Skål!")


SCENES.update({"shortcuts": shortcuts, "theme-reference": anvil_scene})


def render_all(names=None, outdir=None):
    import os
    outdir = outdir or DOCS_ART
    out = []
    for name, fn in SCENES.items():
        if name == "about" or (names and name not in names):
            continue
        s = fn()
        write_svg(os.path.join(outdir, name + ".svg"), s)
        out.append(name)
    return out


def contact_sheet(names, outdir=None, out="sheet.png", size=360):
    """A preview of the plates in `outdir`, rendered from their SVGs, three to a row."""
    import io
    import cairosvg
    from PIL import Image
    outdir = outdir or DOCS_ART
    ims = []
    for n in names:
        with open(os.path.join(outdir, n + ".svg"), encoding="utf-8") as f:
            png = cairosvg.svg2png(bytestring=f.read().encode("utf-8"), output_width=size, output_height=size)
        ims.append(Image.open(io.BytesIO(png)))
    cols = min(3, len(ims))
    rows = (len(ims) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * (size + 20) + 20, rows * (size + 20) + 20), "#f6f8fa")
    for i, im in enumerate(ims):
        sheet.alpha_composite(im.convert("RGBA"), (20 + (i % cols) * (size + 20), 20 + (i // cols) * (size + 20)))
    sheet.save(out)


if __name__ == "__main__":
    import sys
    render_all(sys.argv[1:] or None)
    # The app's own pictures (start screen, About).
    write_svg(os.path.join(UI_ART, "skald-hello.svg"), getting_started())
    write_svg(os.path.join(UI_ART, "skald-skal.svg"), about())

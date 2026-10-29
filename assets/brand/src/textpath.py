"""Convert text to SVG path data using fontTools (so SVGs need no fonts)."""
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen

import os
FONTS = os.environ.get("SCRALD_BRAND_FONTS", os.path.join(os.path.dirname(__file__), "fonts")) + "/"
_cache = {}


def font(name):
    if name not in _cache:
        _cache[name] = TTFont(FONTS + name + ".woff2")
    return _cache[name]


def text_path(text, fontname, size, x=0, y=0, anchor="start", tracking=0.0):
    """Return (path_d, width). y is the baseline."""
    f = font(fontname)
    upm = f["head"].unitsPerEm
    cmap = f.getBestCmap()
    gs = f.getGlyphSet()
    hmtx = f["hmtx"]
    s = size / upm
    kern = {}
    names = [cmap.get(ord(c), ".notdef") for c in text]
    adv = [hmtx[n][0] * s + tracking * size for n in names]
    width = sum(adv) - (tracking * size if names else 0)
    if anchor == "middle":
        x -= width / 2
    elif anchor == "end":
        x -= width
    pen = SVGPathPen(gs)
    cx = x
    for n, a in zip(names, adv):
        tp = TransformPen(pen, (s, 0, 0, -s, cx, y))
        gs[n].draw(tp)
        cx += a
    return pen.getCommands(), width

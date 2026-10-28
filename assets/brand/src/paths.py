"""Where the scripts write their output, found from this folder, so they run from any working directory."""
import os

SRC = os.path.dirname(os.path.abspath(__file__))
BRAND = os.path.dirname(SRC)                                  # assets/brand
REPO = os.path.normpath(os.path.join(BRAND, "..", ".."))
BRAND_ICONS = os.path.join(BRAND, "icons")                    # icon art and concepts
APP_ICONS = os.path.join(REPO, "crates", "scrald-app", "icons")
DOCS_ART = os.path.join(REPO, "docs", "art")
UI_ART = os.path.join(REPO, "ui", "src", "assets", "art")
SKALD_THEME = os.path.join(REPO, "assets", "themes", "skald")


def write_svg(path, svg):
    """Write SVG text as UTF-8 with LF line endings, whatever the platform's default encoding is."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(svg)
    print("wrote", os.path.relpath(path, REPO))


def write_png(path, svg, width, height=None):
    """Render SVG text to a PNG `width` pixels wide (and `height` high, if given)."""
    import cairosvg
    os.makedirs(os.path.dirname(path), exist_ok=True)
    cairosvg.svg2png(bytestring=svg.encode("utf-8"), write_to=path, output_width=width, output_height=height)
    print("wrote", os.path.relpath(path, REPO))


def write_icon(name, svg):
    """An icon concept: icons/icon-<name>.svg, its simplified -small.svg for 16-48 px, and a 1024 px PNG."""
    full = svg(1.0)
    write_svg(os.path.join(BRAND_ICONS, f"icon-{name}.svg"), full)
    write_svg(os.path.join(BRAND_ICONS, f"icon-{name}-small.svg"), svg(0.0))
    write_png(os.path.join(BRAND_ICONS, f"icon-{name}-1024.png"), full, 1024, 1024)

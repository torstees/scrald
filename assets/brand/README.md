# Scrald brand art

Woodcut / manuscript style: ink on paper, carved hatching, a little gold.

| Role | Hex |
|---|---|
| Ink | `#232733` |
| Paper | `#ede5d0` (shade `#d9ceb3`) |
| Mead gold | `#c8912f` (light `#e3b862`) |
| Madder red | `#a3412e` |
| Woad blue | `#5e80a8` (dark `#3f5f86`) |

Type: **UnifrakturMaguntia** (wordmark) and **IM Fell English** (taglines), both OFL. All text in the SVGs is converted to paths, so nothing depends on installed fonts.

## Files

- `banner-light.svg` / `banner-dark.svg`: README hero (1280×400), with PNG renders at 2×.
- `skald.svg`: the Skald mascot on a transparent background.
- `icons/`: **the app icon is the illuminated initial** (`icon-initial.svg`). Also here: the quill-horn concept (kept for other uses) and the future *rune* package icon. `*-small.svg` are simplified versions for 16–48 px.

## Docs art

`docs/art/` holds a round woodcut plate for each docs page (the Skald in a pose per topic; ravens for shortcuts; an anvil for the theme reference) and the **longship of pages** for the docs home. The plates sit on their own paper, so they read on light and dark pages alike. Each page uses one tag, which floats right in Scrald (`class="right"`), docsify and GitHub (`align="right"`):

```html
<img src="art/reading.svg" alt="The Skald reads a big book" width="200" align="right" class="right">
```

## Regenerating

The art is drawn in code (`src/`). It needs Python 3 with `fonttools`, `brotli`, `cairosvg` and `Pillow`. The SVGs need only the first two; the PNGs need `cairosvg`, which on Windows also needs the Cairo library (`libcairo-2.dll`, for example from the GTK runtime or MSYS2) on the `PATH`. The two fonts ship unmodified in `src/fonts/` (from Fontsource, OFL 1.1); set `SCRALD_BRAND_FONTS` to use another folder.

Each script writes into the repository (the folders are in `src/paths.py`), so it can run from any folder:

```sh
python assets/brand/src/banner.py          # banner-light/dark .svg + .png
python assets/brand/src/skald.py           # skald.svg + .png, the mascot on a transparent background
python assets/brand/src/icon_initial.py    # icons/icon-initial.svg, -small.svg, -1024.png (also icon_horn.py, icon_rune.py)
python assets/brand/src/make_app_icons.py  # crates/scrald-app/icons/* and assets/icon-source.png
python assets/brand/src/vignettes.py       # docs/art/*.svg and the app's start-screen and About pictures
python assets/brand/src/longship.py        # docs/art/longship.svg and ui/src/assets/art/longship.svg
python assets/brand/src/theme_art.py       # assets/themes/skald/assets/divider.svg
```

Use `make_app_icons.py` for the app icons rather than `npm run tauri icon`, which would replace the simplified small sizes with plain downscales. The next build picks up new icons (`crates/scrald-app/build.rs` tells Cargo to watch `icons/`).

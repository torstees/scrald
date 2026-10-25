# theme.toml reference

A theme is a folder in Scrald's `themes` folder (see [Themes](themes.md)):

```
my-theme/
├── theme.toml     the settings (required)
├── theme.css      extra styles (optional)
└── assets/        fonts and images (optional)
```

Only `[meta]` is required. Anything else you leave out gets a default that suits a light or dark theme, so a theme can be as short as this:

```toml
[meta]
name = "Fjord"
schema = 1
appearance = "dark"

[colors]
background = "#1b2430"
foreground = "#d8dee9"
accent     = "#88c0d0"
```

Misspelled settings and values out of range are errors, each reported with its line, and Scrald lists them all at once.

## Meta

| Key in `[meta]` | Value |
|---|---|
| `name` | The name shown in the switcher. Required. |
| `schema` | Always `1`. Required. |
| `appearance` | `"light"` or `"dark"`. Required. Chooses the defaults for anything left out, and whether printing starts with white paper. |
| `author` | Optional. |

## Colors

Colors are hex: `#rgb`, `#rrggbb`, or with transparency, `#rgba` and `#rrggbbaa`.

| Key in `[colors]` | Used for |
|---|---|
| `background` | The page |
| `foreground` | Text |
| `muted` | Secondary text, such as captions and code comments |
| `accent` | Tags, the keyboard focus outline, the reading progress line |
| `link` | Links |
| `selection` | Selected text |
| `border` | Rules, table lines, panel edges |
| `code_bg` | Code blocks and inline code |
| `quote_bar` | The bar beside quotes |
| `highlight_bg` | `==highlighted==` text (usually translucent) |
| `ui_background` | The sidebars and status bar |

Every color can be left out. Missing ones are worked out from the background, text, and accent colors.

## Palette

The `[palette]` section holds the 16 terminal colors: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, and the same with `bright_` in front (`bright_red`, ...). They color code (syntax highlighting) and callouts. Missing ones get defaults.

## Fonts

The `[fonts]` section has four slots: `body`, `heading`, `mono` (code), and `ui` (sidebars and status bar). Each is a table:

```toml
[fonts]
body    = { family = "Literata", source = "google", weights = [400, 700], italic = true }
heading = { family = "Cormorant Garamond", source = "google", weights = [600] }
mono    = { family = "JetBrains Mono", fallback = ["Cascadia Code", "Consolas"] }
```

| Key | Value |
|---|---|
| `family` | The font's name. Required. |
| `source` | `"system"` (installed, the default), `"google"` (downloaded once from Google Fonts), or `"theme"` (files in the theme folder). |
| `fallback` | Fonts to use if this one isn't available. |
| `weights` | For Google fonts, the weights to download. Defaults to 400 and 700. |
| `italic` | For Google fonts, whether to download the italics too. |
| `files` | For theme fonts, the files: `[{ path = "assets/MyFont.woff2", weight = 400, italic = false }]`. |

## Layout

| Key in `[layout]` | Default | Range | Meaning |
|---|---|---|---|
| `measure` | `66` | 20–200 | Characters per line |
| `font_size` | `18` | 6–96 | Text size in pixels at 100% zoom |
| `line_height` | `1.6` | 0.8–3 | Line spacing, as a multiple of the text size |
| `paragraph_spacing` | `1.0` | 0–5 | Space between paragraphs, in lines of text (em) |
| `text_sizing` | `"fixed"` | | `"fixed"` or `"fit"`: the text sizing a document starts with (see [Reading](reading.md)). Your own setting wins. |
| `min_font_size` | `14` | 6–96 | The smallest size "fit" will use |
| `max_font_size` | `32` | 6–96 | The largest size "fit" will use |

## Elements

The `[elements]` sections set how quotes, dividers, and headings look, and what shows before the text:

```toml
[elements.blockquote]
style = "pull-quote"   # "bar", "indent", "boxed", or "pull-quote"
italic = true

[elements.hr]
style = "ornament"     # "line", "dots", "ornament", or "image"
glyph = "❧"            # for "ornament": 1 to 8 characters
# image = "assets/divider.svg"   # for "image": a file in the theme folder

[elements.headings]
numbering = true       # number headings 1, 1.1, 1.1.1 (below the title)
h1_rule = false        # a rule under the title

[elements.front]
cover = true           # show the front matter's cover image above the text
title = true           # without a cover, show a title the text doesn't start with
```

Both `[elements.front]` keys are on unless a theme turns them off. The user's settings can override them either way (see "The cover and the title" in [Reading](reading.md)).

## Syntax

In the `[syntax]` section, `source = "palette"` is the default and only choice so far: code colors come from `[palette]`.

## Extra CSS

```toml
[css]
file = "theme.css"
```

A stylesheet in the theme folder, for anything the settings can't express. It's added after Scrald's own styles, and can be up to 512 KB. For safety, `@import` is removed, and `url()` can only point to files in the theme's folder (or `data:` URLs).

The settings above reach the page as CSS variables, which `theme.css` can use:

- `--sk-color-background`, `--sk-color-foreground`, and the other colors, with `-` for `_` (`--sk-color-code-bg`);
- `--sk-palette-red`, `--sk-palette-bright-red`, and so on;
- `--sk-font-body`, `--sk-font-heading`, `--sk-font-mono`, `--sk-font-ui`;
- `--sk-measure`, `--sk-font-size`, `--sk-line-height`, `--sk-paragraph-spacing`.

The document's text is inside elements with the class `sk-block`, for example:

```css
.sk-block h2 {
  font-variant: small-caps;
  letter-spacing: 0.05em;
}
```

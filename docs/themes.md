# Themes

A theme sets a document's colors, fonts, text size, line length, and spacing, and how quotes, dividers, and headings look.

## Built-in themes

| Theme | Id | Look |
|---|---|---|
| Scrald Light | `scrald-light` | Warm paper and a classic serif, 66 characters to a line |
| Scrald Dark | `scrald-dark` | Scrald Light's type on a dark page, in soft contrast |
| Sepia | `sepia` | A book page: narrower lines, ornamental breaks, italic pull quotes |
| Technical | `technical` | Crisp sans-serif text on white, wider lines, numbered headings |
| Technical Dark | `technical-dark` | Technical on a dark page |

The id is what you write in front matter or a folder's `.scrald.toml`.

## Choosing a theme

`Ctrl+T`, or the theme's name in the status bar, opens the theme switcher.

- `Up` and `Down` preview each theme on the document as you go.
- `Enter` or **Use for this document** keeps the theme for this document.
- `Esc` puts the previous theme back.
- **Make default** uses the theme for every document without a theme of its own.
- **Save to document** writes the theme into the document's front matter (`scrald-theme: sepia`), so it travels with the file. Like any edit, it needs saving.
- **Reset to document default** forgets the choice you made in Scrald, so the document's front matter, folder, or the default applies again.

Until you choose a default, Scrald follows Windows' light or dark mode, using Scrald Light or Scrald Dark. Settings can change that.

## Which theme a document gets

Scrald uses the first of these that names a theme:

1. The theme you chose for the document in Scrald.
2. `scrald-theme` in the document's front matter.
3. `theme` in a `.scrald.toml` file in the document's folder or any folder above it.
4. Your default theme.

A `.scrald.toml` applies to every document under its folder:

```toml
theme = "sepia"
flavor = "obsidian"
```

The nearest file wins. An unknown theme id is skipped, so a typo falls through to the next step.

## Your own themes

Your themes live in Scrald's `themes` folder, one folder each, holding a `theme.toml`:

```
%APPDATA%\io.github.torstees.scrald\themes\my-theme\theme.toml
```

The folder's name is the theme's id. A theme with the same id as a built-in theme replaces it.

Scrald notices changes to this folder while it runs, so a saved `theme.toml` shows at once. A theme with mistakes is still listed in the switcher, with what's wrong and the line it's on.

See the [theme.toml reference](theme-reference.md) for every setting.

### The theme editor

Built-in themes can't be changed, but **Duplicate to customize** in the switcher copies one into your themes folder. For your own themes, **Edit…** opens the theme editor beside the document:

- the name and whether the theme is light or dark;
- the colors, each with a color picker. An empty color is worked out from the others, and the box shows what it would be;
- the fonts for body text, headings, code, and the interface, from the fonts installed on your computer or from Google Fonts;
- line length, text size, line height, paragraph spacing, and text sizing;
- the quote style, the divider, heading numbers, and the rule under the title.

Each change shows on the document at once, and nothing is written until you **Save** (or press `Ctrl+Enter`). `Esc` or **Cancel** puts the theme back.

The editor changes only the settings you touch: comments and the order of `theme.toml` stay as they were. Theme fonts in the theme's own folder and image dividers are shown, but are changed in `theme.toml`.

## Importing a terminal theme

**Import terminal theme…** in the switcher makes a Scrald theme from a terminal's color scheme. It reads:

- Windows Terminal: a `settings.json`, or a scheme copied from it;
- iTerm2 (`.itermcolors`);
- Alacritty (`.toml`);
- Ghostty;
- kitty (`.conf`);
- base16 and base24 (`.yaml`).

A file with several schemes asks which to import. Each becomes a new theme in your themes folder. Scrald works out the colors the scheme doesn't have, such as links, muted text, and the selection, choosing shades that are easy to read on its background.

## Contrast

All built-in themes meet the WCAG AA guidelines for text contrast. **Check contrast** in the switcher checks the selected theme: each pair of colors that's too close is listed with its contrast ratio and a sample. The theme editor checks as you edit, marking each color involved with ⚠.

These are warnings: a theme below the guideline still works.

## Fonts

A font can come from three places:

- **Installed on your computer** (the default).
- **Google Fonts.** Scrald downloads the font once, the first time a theme uses it, and keeps it, so it works offline afterward. Until it arrives, the theme's fallback fonts show. This is the only time Scrald contacts a server by itself.
- **The theme's own folder**, for fonts that come with a theme.

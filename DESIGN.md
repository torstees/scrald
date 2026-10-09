# Scrald: Design Document

Scrald is a themeable Markdown reader with lightweight editing. The name plays on *scrawled*, *skald* (the Norse poet), and *skål* (the Viking toast). It is built for anything from technical documentation and Obsidian vault notes to full-length novels, and its defining promise is that **large documents (100K+ words) open fast and read beautifully**, with an integrated table of contents.

This document describes the architecture and the decisions behind it. GitHub issues hold the build order (indexed in `TODO.md`), and `AGENTS.md` holds the working conventions for coding agents.

---

## 1. Goals and non-goals

### Goals

1. **Reader first.** The default experience is a polished, rendered reading view. Editing never forces you out of that view.
2. **Fast on large files.** A 100K-word document should show its first screen in well under a second and scroll smoothly from then on.
3. **Common Markdown flavors.** GitHub-flavored Markdown (GFM) is required. The pure-Markdown parts of Obsidian syntax and a useful subset of Pandoc extensions are strongly desired.
4. **Themes are first-class.** Bundled themes, user-created themes, import from terminal color schemes, local and Google fonts, and theme-owned assets.
5. **Remembered state.** Scrald remembers each document's theme, flavor, and reading position.
6. **Native desktop integration.** Double-click or "Open with" from the file manager, plus a start screen listing recent documents.

### Non-goals for v1

- Running Obsidian plugins or arbitrary JavaScript embedded in documents.
- Full Pandoc compatibility (citations, raw LaTeX, grid tables, and so on).
- Vault-wide features such as a graph view, backlinks, or full-vault search.
- Sync, collaboration, or any cloud features.
- A WYSIWYG editor. Editing is source-based, scoped to a block or the whole raw file.

---

## 2. Technology stack

| Layer | Choice | Rationale |
|---|---|---|
| Shell | **Tauri 2** | Small footprint; uses WebView2 on Windows; Rust backend. |
| Markdown parser | **comrak** | CommonMark/GFM compliant, broad extension set, full AST with source positions (needed for transforms and editing). |
| Front matter | **serde-saphyr** (into `serde_json::Value` with `preserve_order`) | Maintained, serde-based, built on the saphyr parser. `serde_yaml` is archived and `serde_norway` is unmaintained. The saphyr parser's spans can drive minimal edits later (§8.3). |
| Theme files | **TOML** via the `toml` crate | Idiomatic in Rust, comments allowed, friendlier to hand-edit than JSON. |
| Syntax highlighting | **syntect**, emitting CSS classes rather than inline colors | Colors then come from the theme; highlighting can run per block on demand. |
| HTML sanitizing | **ammonia** | Raw HTML in Markdown must never run scripts. |
| File watching | **notify** (with a debouncer) | Live reload on external changes. |
| App state | **SQLite** via `rusqlite` (bundled feature) | Per-document memory, recent files, settings. |
| System fonts | **fontdb** | Enumerate installed font families. |
| Frontend | **TypeScript + Svelte 5 + Vite** | Small runtime, simple reactivity; the hot rendering paths are hand-written anyway. |
| Raw editor | **CodeMirror 6** | Handles very large files because it renders only the visible viewport. |
| Math | **KaTeX** (frontend, loaded lazily) | Fast, no JS engine needed in Rust. |
| Diagrams | **Mermaid** (frontend, loaded lazily, only if a document uses it) | Heavy library, so it should never load unless needed. |

Agents should use the latest stable versions at scaffolding time and check current APIs, since several of these crates evolve quickly.

---

## 3. Architecture overview

```
scrald/
├── Cargo.toml                 # workspace
├── crates/
│   ├── scrald-core/           # pure Rust, no Tauri/GUI dependencies
│   │   ├── src/
│   │   │   ├── document.rs    # DocumentModel, Block, SourceRange
│   │   │   ├── frontmatter.rs # split + parse + minimal-edit YAML
│   │   │   ├── flavor.rs      # Flavor enum, detection heuristics
│   │   │   ├── parse.rs       # comrak options per flavor, parse to AST
│   │   │   ├── transform/     # AST passes: attributes, callouts, wikilinks, ...
│   │   │   ├── render.rs      # per-block HTML rendering + sanitizing
│   │   │   ├── highlight.rs   # syntect, class-based output
│   │   │   ├── toc.rs         # table of contents
│   │   │   ├── search.rs      # search over source text, mapped to blocks
│   │   │   ├── assets.rs      # image path resolution
│   │   │   ├── edit.rs        # splice edits into source by range
│   │   │   └── theme/         # theme schema, loading, terminal import
│   │   └── tests/fixtures/    # sample docs + snapshot tests
│   └── scrald-app/            # Tauri application (thin layer)
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands.rs    # Tauri commands exposed to the frontend
│       │   ├── protocol.rs    # custom URI schemes for doc + theme assets
│       │   ├── state.rs       # SQLite persistence
│       │   ├── watcher.rs     # file watching
│       │   └── fonts.rs       # system + Google font handling
│       └── tauri.conf.json
└── ui/                        # Svelte + TypeScript frontend
    └── src/
        ├── reader/            # block renderer, scroll anchoring, zoom
        ├── toc/
        ├── frontmatter/       # properties panel
        ├── editor/            # block editor + raw mode (CodeMirror)
        ├── themes/            # switcher, CSS variable application
        └── start/             # recent documents screen
```

**Key principle:** `scrald-core` contains all logic that can be tested without a window. It is also the best place to learn Rust, because its functions have clear inputs and outputs. `scrald-app` stays thin: it wires core functionality to Tauri commands, the file system, and persistence.

### Data flow when opening a document

1. Read the file as bytes. Strip and remember a UTF-8 BOM if present. Record the dominant line ending (LF or CRLF) so saves preserve it.
2. Split off front matter (YAML between `---` fences at the very start of the file).
3. Determine the flavor (section 5).
4. Parse the body with comrak, using that flavor's options, with source positions enabled.
5. Run AST transform passes (section 5.4).
6. Render each **top-level block** to sanitized HTML independently.
7. Build the TOC, word count, and a block index for search.
8. Send a `DocumentModel` to the frontend.

```rust
// Illustrative shape, not a final API.
pub struct DocumentModel {
    pub path: PathBuf,
    pub front_matter: FrontMatter,
    pub flavor: Flavor,
    pub blocks: Vec<Block>,
    pub toc: Vec<TocEntry>,
    pub word_count: usize,
    pub line_ending: LineEnding,
    pub features: FeatureFlags, // e.g. has_math, has_mermaid: tells the UI what to lazy-load
}

pub struct Block {
    pub id: u32,                // stable within one parse
    pub kind: BlockKind,        // Heading { level }, Paragraph, List, Code, Table, ...
    pub html: String,
    pub source: SourceRange,    // byte range in the ORIGINAL file, including front matter offset
    pub hash: u64,              // used to re-render only changed blocks after edits
    pub section: u32,           // index of the heading section this block belongs to
}
```

comrak reports positions as line/column, so `scrald-core` builds a line-start table once per parse and converts to **byte offsets in the original file**. This conversion must be correct for CRLF files and multi-byte UTF-8, and it needs dedicated tests.

Block range rules (as implemented):

- comrak's columns are 1-based **byte** columns with inclusive ends, and CRLF lines are reported exactly like LF lines, so `source::LineIndex` is the single conversion point.
- A block's range starts at **column 1 of its first line**, so leading indentation (an indented code block's four spaces) belongs to the block.
- Blocks are listed in **source order**. comrak moves footnote definitions to the end of the document; Scrald re-sorts them back to where they're written, and the UI is responsible for presenting them as endnotes.
- Between blocks there is only blank space or link reference definitions (which comrak keeps out of the AST). The round-trip test enforces this, so concatenating gaps and blocks reproduces the file exactly.
- Word count counts whitespace-separated runs containing a letter or digit, in prose and inline code; code blocks, raw HTML, and math are excluded.

---

## 4. Large-document performance

Parsing is not the bottleneck: comrak parses a 100K-word file (~600–700 KB) in tens of milliseconds. The cost is the size of the DOM and the browser's layout work. The strategy, from cheapest to most involved:

1. **Section containment.** Group blocks into sections by heading. Each section element gets `content-visibility: auto` and a `contain-intrinsic-size` estimate, so the browser skips layout and paint for offscreen sections.
2. **Progressive mount.** Render the sections around the initial scroll position immediately, then mount the rest in `requestIdleCallback` chunks.
3. **Lazy heavy work.** Syntax highlighting, KaTeX, and Mermaid run only when a block approaches the viewport (`IntersectionObserver`). Code blocks arrive from Rust already highlighted only if they are cheap. Otherwise the UI requests highlighting per block.
4. **Lazy images.** `loading="lazy"`, with known dimensions reserved when available (read image headers in Rust) to prevent scroll jumps.
5. **Virtualization (fallback only).** If profiling shows containment is not enough, mount only nearby sections and use placeholders with measured or estimated heights. This adds complexity, so it is deferred until measurements justify it.

**Scroll anchoring.** Reading position is stored as `(block_id_source_offset, fraction_into_block)` rather than pixels, so it survives theme changes, zoom, window resizes, live reloads, and small edits.

**Performance budget** (measured on the generated 100K-word fixture, release build):

| Metric | Target |
|---|---|
| File open to first rendered screen | < 500 ms |
| Full document mounted | < 2 s, without blocking scrolling |
| Theme switch | < 300 ms |
| Re-render after a single-block edit | < 100 ms |

A fixture generator in `scrald-core` produces 100K, 250K, and 500K-word documents with headings, lists, tables, code, images, and math, for benchmarks and manual testing.

---

## 5. Markdown flavors

### 5.1 Approach: a superset with profiles

The three flavors are **mostly additive**. Wikilinks, callouts, fenced divs, attributes, and highlights can coexist in a single parser configuration. Only a few constructs genuinely conflict, for example:

- `~text~` is strikethrough in GFM but subscript in Pandoc.
- `$...$` math rules differ slightly between Obsidian, GitHub, and Pandoc.
- Link resolution differs: Obsidian resolves `[[Note]]` vault-wide, while the others use plain relative paths.

So a **flavor is a profile**: the shared superset of extensions, plus resolutions for those few conflicts.

```rust
pub enum Flavor { Gfm, Obsidian, Pandoc }
```

### 5.2 Flavor resolution order

1. Per-document override the user chose in the app (stored in the state database).
2. `scrald-flavor` in front matter.
3. Automatic detection.
4. Default: `Gfm`.

**Detection** is a quick scan, effectively instant even at 500K words. It scores signals and picks the highest:

- Obsidian: `[[wikilinks]]`, `![[embeds]]`, `> [!type]` callouts with Obsidian-specific types, `%%comments%%`, `==highlight==`, a `.obsidian/` folder in an ancestor directory (strong signal).
- Pandoc: `:::` fenced divs, `{.class}` or `{#id}` attribute blocks, `^[inline footnotes]`, Pandoc-specific front matter keys (`bibliography`, `csl`, `abstract`), a `% Title` block.
- GFM: absence of the above.

Because detection is instant, the detected result does not need to be stored. Only explicit user overrides are remembered. The UI shows the active flavor in the status bar with a dropdown to change it.

### 5.3 Supported features

| Feature | GFM | Obsidian | Pandoc | Notes |
|---|---|---|---|---|
| Tables, task lists, strikethrough, autolinks | ✓ | ✓ | ✓ | comrak built-in |
| Footnotes | ✓ | ✓ | ✓ | Rendered as hover popovers plus an endnotes section |
| Inline footnotes `^[...]` | | | ✓ | Transform pass |
| GitHub alerts `> [!NOTE]` | ✓ | ✓ | | Styled like callouts |
| Callouts `> [!type]+/-` (foldable) | | ✓ | | Transform pass |
| Math `$...$`, `$$...$$` | ✓ | ✓ | ✓ | KaTeX, lazy |
| Mermaid code blocks | ✓ | ✓ | | Lazy |
| Wikilinks `[[Note]]`, `[[Note\|alias]]`, `[[Note#Heading]]` | | ✓ | | Resolved within the vault root (nearest ancestor with `.obsidian/`) or the document's folder |
| Image embeds `![[img.png]]`, `![[img.png\|300]]` | | ✓ | | Width/height syntax supported |
| Note embeds `![[Other note]]` | | ✓ | | v1: rendered as a link card that opens the note; post-v1: transclusion |
| `==highlight==` | | ✓ | | Transform pass if comrak lacks it |
| `%%comment%%` | | ✓ | | Hidden in reading view |
| Inline `#tags` | | ✓ | | Styled as chips |
| Block references `^block-id` | | ✓ | | Rendered as hidden anchors |
| Fenced divs `::: {.class}` | | | ✓ | Transform pass |
| Bracketed spans `[text]{.class}` | | | ✓ | Transform pass |
| Attributes on images, headings, code `{#id .class key=val}` | | | ✓ | Transform pass; enables image layout (section 6) |
| Definition lists | | | ✓ | comrak built-in |
| Sub/superscript `~x~`, `^x^` | | | ✓ | Pandoc profile only for `~x~` |
| Smart punctuation | opt | opt | ✓ | Setting |
| Raw HTML | ✓ | ✓ | ✓ | Always sanitized |

### 5.4 Transform passes

Transforms operate on the comrak AST after parsing and are each independently testable. Order matters and is fixed:

1. Strip `%%comments%%` (Obsidian profile).
2. Attribute blocks: attach `{...}` attributes to the preceding image, heading, code block, or span.
3. Fenced divs and bracketed spans.
4. Callouts and alerts.
5. Wikilinks and embeds (resolution only; unresolved links render with a distinct "missing" style).
6. Image path resolution (section 6).
7. Highlights, inline tags, block-reference anchors.

Where comrak lacks a construct, the pass works on text nodes or adjacent nodes. Each pass must preserve source ranges for any node it rewrites.

---

## 6. Images and layout

### 6.1 Path resolution

For a relative image path, Scrald tries in order:

1. The document's asset directory from front matter (`assets:`), resolved relative to the document.
2. For Obsidian profile: the vault's attachment folder if configured in `.obsidian/app.json`, then a vault-wide filename search (cached per vault).
3. The document's own directory.

Absolute paths and `file://` URLs are used directly. Remote `http(s)` images are **blocked by default** with a per-document "load remote images" toggle, for privacy.

Missing images render as a placeholder showing the filename, with the attempted paths in a tooltip.

### 6.2 Serving images

Images are served through a custom URI scheme (`scrald-asset://`) registered in `scrald-app`, never via `file://`. The handler only serves files the current document legitimately resolved, preventing a document from reading arbitrary files. Theme assets use a separate scheme (`scrald-theme://<theme-id>/...`).

### 6.3 Layout

Layout uses classes and attributes, with the theme defining what they look like:

```markdown
![A map of the fjord](fjord.png){.right width=40%}
![[portrait.jpg|300]]
![Diagram](flow.svg){.center}
![Figure](wide.png){.full-bleed}
```

| Class | Effect |
|---|---|
| `.left`, `.right` | Float with text wrapping; collapses to centered on narrow windows |
| `.center` | Centered block, no wrapping |
| `.full-bleed` | Wider than the text column, up to the window width |
| `.inline` | Inline with text (icons, small glyphs) |

An image with a title or alt text and the `.figure` class (or alone in a paragraph, configurable) renders as `<figure>` with a caption. Tables accept the same `.center` and `.full-bleed` classes, and wide tables scroll horizontally inside their own container.

---

## 7. Themes

### 7.1 Theme package format

A theme is a folder:

```
nordic-night/
├── theme.toml        # required: tokens
├── theme.css         # optional: advanced overrides
└── assets/           # optional: fonts, divider images, textures, ...
```

Themes live in two places: bundled themes (read-only, shipped with the app) and the user theme directory (`themes/` under the platform's app config directory, as resolved by Tauri's path API: `%APPDATA%\Scrald\themes\` on Windows). A bundled theme can be duplicated into the user directory as a starting point. User themes hot-reload when their files change.

### 7.2 `theme.toml` schema (draft)

```toml
[meta]
name = "Nordic Night"
author = "You"
schema = 1
appearance = "dark"            # "light" | "dark": used for derived defaults and UI chrome

[colors]
background   = "#2e3440"
foreground   = "#d8dee9"
muted        = "#8a93a5"
accent       = "#88c0d0"
link         = "#81a1c1"
selection    = "#434c5e"
border       = "#3b4252"
code_bg      = "#3b4252"
quote_bar    = "#5e81ac"
highlight_bg = "#ebcb8b44"

[palette]                      # 16 ANSI colors; seeds syntax highlighting and callout colors
black = "#3b4252"
red = "#bf616a"
# ... green, yellow, blue, magenta, cyan, white, bright_* variants

[fonts]
body    = { family = "Literata", source = "google", weights = [400, 700], italic = true }
heading = { family = "Cormorant Garamond", source = "google", weights = [600] }
mono    = { family = "JetBrains Mono", source = "system", fallback = ["Cascadia Code", "Consolas"] }

[layout]
measure = 66                   # characters per line
font_size = 18                 # px at 100% zoom
line_height = 1.6
paragraph_spacing = 1.0        # em
text_sizing = "fixed"          # "fixed" | "fit": default only; the user's setting wins (see 7.4)
min_font_size = 14             # px; lower bound for "fit"
max_font_size = 32             # px; upper bound for "fit"

[elements.blockquote]
style = "bar"                  # "bar" | "indent" | "boxed" | "pull-quote"
italic = false

[elements.hr]
style = "ornament"             # "line" | "dots" | "ornament" | "image"
glyph = "❦"                    # for "ornament"
# image = "assets/divider.svg" # for "image"

[elements.headings]
numbering = false
h1_rule = true

[syntax]
source = "palette"             # "palette" derives from [palette]; or name a bundled syntect theme

[css]
file = "theme.css"
```

Tokens become CSS custom properties (`--sk-color-background`, `--sk-font-body`, `--sk-measure`, and so on). Bundled themes and user themes go through the same path, so a theme can be built entirely from TOML, and `theme.css` exists only for things tokens can't express.

### 7.3 Importing terminal themes

Scrald can create a new theme from a terminal color scheme. Supported import formats, in order of priority:

1. Windows Terminal scheme JSON
2. iTerm2 `.itermcolors` (plist)
3. Alacritty (TOML) and Ghostty config
4. kitty `.conf`
5. base16/base24 YAML

The importer maps the 16 ANSI colors into `[palette]`, maps foreground/background/selection/cursor directly, and derives the remaining semantic colors (`muted`, `accent`, `link`, `code_bg`, `quote_bar`) with simple rules: for example, `muted` blends foreground toward background, and `link` uses blue. The user can then adjust the result. The output is a normal theme folder, so imported themes need no special handling afterwards.

### 7.4 Line length, narrow windows, and zoom

The text column is `max-width: min(var(--sk-measure) * 1ch, 100%)`, centered. The default measure is **66 characters**, the classic typographic ideal within the 45–75 range. When the window is narrower than the measure, the column narrows to fit the window and the text rewraps, the way most readers behave.

**Text sizing modes.** A user setting (with an optional default in the theme's `text_sizing`) chooses how font size relates to window width:

- `fixed` (default): font size comes from the theme and zoom only. Wider windows add margin around the column; narrower windows rewrap the text.
- `fit`: font size is derived from the window width so the column always holds roughly the measure: `font_size ≈ available_width / measure`, adjusted for the font's average character width. It works in **both directions**. On a large monitor, text grows to fill the column comfortably, which is especially helpful for readers who want larger type without constantly adjusting zoom. On a narrow window it shrinks, down to `min_font_size`, after which the text rewraps as in `fixed`. Growth is capped at `max_font_size`. Zoom still applies on top as a multiplier.

Font size changes in `fit` mode are applied with the same anchoring as zoom, so resizing the window never loses the reader's place.

**Per-document memory.** Text sizing mode and zoom level are remembered per document, alongside window size and position (section 11). Changing either one updates that document's stored values only; a "make this the default" action promotes the current settings to the global default. Resolution order for text sizing: the per-document stored value, then the user's global setting, then the theme's `text_sizing` suggestion.

A per-user **"fill window"** toggle ignores the measure entirely, letting the text run the full window width at the current font size.

**Zoom** (Ctrl+wheel, Ctrl+plus/minus, trackpad pinch, Ctrl+0 to reset) scales a single `--sk-zoom` variable that multiplies the base font size. Changes animate smoothly over ~120 ms, and the block under the cursor (or at the top of the viewport for keyboard zoom) stays anchored in place.

### 7.5 Fonts

- **System fonts:** enumerated with `fontdb` and offered in the theme editor and font pickers.
- **Google Fonts:** downloaded once as `woff2` files into the app's cache directory and served locally from then on. This keeps the app working offline and avoids contacting Google on every launch. A theme referencing an uncached Google font triggers a one-time download, with the fallback stack shown meanwhile.
- **Theme fonts:** font files inside a theme's `assets/` folder are loaded through `scrald-theme://`.

### 7.6 Theme selection and memory

A theme switcher (Ctrl+T, plus a status bar control) shows themes with live previews applied to the current document. Theme resolution for a document:

1. The user's per-document choice (state database).
2. `scrald-theme` in front matter.
3. A folder-level config (`.scrald.toml` in the document's directory or an ancestor).
4. The global default theme.

The switcher offers "Save to document," which writes `scrald-theme` into the front matter, and "Reset to document default," which clears the stored override.

---

## 8. Front matter

### 8.1 Recognized properties

Scrald reads common keys at the top level so documents stay compatible with Obsidian, Hugo, Jekyll, and Pandoc. App-specific settings use flat, top-level keys with a `scrald-` prefix.

Flat keys were chosen over a nested `scrald:` map for two reasons. Obsidian's Properties panel only handles flat values (text, lists, numbers, checkboxes, dates), so a nested map can't be edited there (confirmed manually, 2026-10-08: the nested map was shown read-only, the flat keys as ordinary editable text properties). Flat keys are also plain top-level scalars, so the minimal-edit YAML engine (section 8.3) never has to edit inside a nested map.

```yaml
---
title: The Long Winter
author: A. Writer
summary: A family survives a hard season on the northern coast.
tags: [novel, draft, norse]
notes: Chapter 12 needs a rewrite.
source: https://example.com/original
assets: ../images/long-winter
scrald-theme: nordic-night
scrald-flavor: obsidian
---
```

| Key | Aliases accepted | Meaning |
|---|---|---|
| `title` | | Document title (window title, start screen) |
| `author` | `authors` | String or list |
| `summary` | `synopsis`, `description`, `abstract` | Short description |
| `tags` | `tag`, `keywords` | String or list; `#` prefix stripped |
| `notes` | | Freeform notes |
| `source` | `url` | Where the content came from |
| `assets` | `asset_dir`, `attachments` | Base directory for relative image paths |
| `scrald-theme` | `scrald.theme` (read only) | Theme ID |
| `scrald-flavor` | `scrald.flavor` (read only) | `gfm`, `obsidian`, or `pandoc` |

Unknown keys are preserved and shown in the properties panel as generic fields.

For leniency, Scrald also *reads* the nested form (`scrald: { theme, flavor }`), but it always *writes* the flat keys. If both forms are present, the flat key wins. When Scrald writes a flat key to a document that only has the nested form, it leaves the nested map untouched rather than restructuring it.

### 8.2 Properties panel

A collapsible panel at the top of the document, collapsed by default when empty or when the user prefers. It shows recognized properties with suitable editors: tag chips with add and remove, a multiline field for notes and summary, a folder picker for assets, and dropdowns for theme and flavor.

### 8.3 Editing YAML without damaging it

Re-serializing YAML through a parser loses comments, key order, and quoting style. Scrald instead performs **minimal text edits** on the front matter block: it locates the line range for a given key and rewrites only that key's value. Supported edit types are scalar values, flow lists (`[a, b]`), and block lists (`- a`). If a key's existing formatting is too complex to edit safely, such as anchors or multi-document YAML, the panel shows it read-only with an "edit in raw mode" link.

**Interaction with Obsidian.** Obsidian does *not* edit minimally: when a property is changed in its Properties panel, it rewrites the whole front matter block. Observed 2026-10-08: comments were dropped and the flow list `tags: [a, b]` became a block list, while key order and the (read-only) nested map were preserved. So Scrald's minimal edits only protect files from Scrald itself, and files from Obsidian vaults will commonly use block lists with no comments. The YAML editor must handle both list styles equally well, and Scrald's own edits should keep whichever style a key already uses.

---

## 9. Editing

### 9.1 Block editing

Double-clicking a block in the reading view replaces it with a small CodeMirror editor containing that block's Markdown source. Clicking outside, pressing Escape, or pressing Ctrl+Enter commits the edit and returns to the rendered view.

- The editable unit is a **top-level block**. Editing a single list item edits the whole list, and editing inside a blockquote edits the whole blockquote. Finer granularity can come later.
- On commit, the new text is spliced into the source at the block's byte range (`scrald-core::edit`). The whole document is then reparsed, which is cheap and avoids bugs from Markdown's non-local constructs such as reference links and footnotes. Only blocks whose hashes changed are re-rendered in the DOM.
- Undo and redo operate on the whole document's edit history, so Ctrl+Z works predictably across blocks.

### 9.2 Raw mode

Ctrl+E toggles between the reading view and a full-document CodeMirror 6 editor showing the raw Markdown, including front matter. Scroll position carries over in both directions via source offsets. Raw mode gets basic Markdown syntax highlighting, search and replace, and soft wrap at the theme's measure.

### 9.3 Saving and external changes

- Unsaved changes are marked in the title bar. Ctrl+S saves. Closing a window (or quitting) with unsaved changes asks whether to save, discard, or cancel.
- Autosave after a few seconds of inactivity is a setting, **off by default for v1**. The reasons: the edit and save path is new code, and manual save keeps a human check between any bug and the user's file; reading is the primary mode, so accidental edits (a stray double-click and keystroke) are likely and shouldn't become permanent on their own; documents often live in folders watched by Obsidian, git, or sync tools, where frequent writes cause churn and conflicts; and changing the default from off to on later is painless, while the reverse is not. Revisit the default after v1 once the edit path has proven reliable.
- **Crash recovery.** While a document has unsaved changes, Scrald periodically writes them to a recovery file in the app data directory (never next to the document). Recovery files are deleted on save or discard. On launch, or when reopening a document that has a recovery file, Scrald offers to restore or discard it. This is what protects against crashes and power loss, independent of the autosave setting.
- Saves are **atomic** (write a temp file in the same directory, then rename), preserve the original line endings and BOM, and never touch bytes outside edited ranges.
- The file watcher reloads the document when it changes externally and there are no unsaved local edits, preserving scroll position. If there are unsaved edits, Scrald shows a banner offering to keep local changes, load the disk version, or compare them in raw mode.

---

## 10. Navigation and reading features

- **Table of contents:** a sidebar built from headings, with collapsible levels, the current section highlighted while scrolling, and click-to-jump. Toggle with Ctrl+\\. For novels, a setting to show only H1/H2 keeps the list manageable.
- **Search in document:** Ctrl+F searches the source text in Rust and maps hits back to blocks; the UI highlights matches in rendered blocks and scrolls to each.
- **Footnotes:** hover popovers in reading view, plus the endnotes section.
- **Status bar:** word count, estimated reading time, current section, flavor, theme, zoom.
- **Links:** external links open in the default browser. Links to other Markdown files open in Scrald (same window, with back and forward navigation).
- **Post-v1:** print/export to PDF using the active theme, and a reading-progress indicator.

---

## 11. Startup and windows

- **File associations** for `.md`, `.markdown`, and `.mdown`, registered by the Tauri bundler. The installer should not take over the default `.md` association without the user's consent; Windows offers it through "Open with."
- **Single instance:** using Tauri's single-instance plugin, opening another file while Scrald is running opens it in a new window of the existing process.
- **Command line:** `scrald path/to/file.md` opens the file directly.
- **Start screen:** launching with no file shows recent documents (title from front matter, path, last opened, theme swatch), pinned documents, an Open button, and a drop zone for drag and drop.
- **Window state:** size, position, and maximized state are remembered per document, together with its text sizing mode and zoom. On restore, the window is clamped to a currently connected monitor, so a document last opened on a now-disconnected display still appears on screen. New documents open at the size of the most recently used window.

### 11.1 Platform support

Scrald is developed and tested on Windows first. macOS and Linux support is the **final milestone of v1** (milestone M10). Until then, code stays portable: no Windows-only APIs outside a clearly isolated module, and paths come from Tauri's path API rather than hard-coded locations.

`scrald-core` is the exception: it has no GUI dependencies, so it is supported on Windows, macOS, and Linux **from the start**. CI runs its tests on all three platforms from M0 onward, which catches portability slips (hard-coded `\` separators, case-sensitive file names, path normalization) as they're written rather than at M10.

Known differences to plan for when that milestone starts:

- **Webviews.** macOS uses WKWebView and Linux uses WebKitGTK, not Chromium-based WebView2. The large-document strategy in section 4 relies on `content-visibility: auto` and `requestIdleCallback`, so their availability and behavior on each engine must be checked, with fallbacks where needed (for example a `setTimeout`-based idle scheduler). Performance must be re-measured against the section 4 budget on each platform.
- **Opening files.** On macOS, files opened from Finder arrive as an "opened" app event, not as command-line arguments, and the single-instance plugin behaves differently. File associations are declared differently on each platform (Info.plist on macOS, `.desktop` files and MIME types on Linux).
- **Custom protocols.** Tauri formats custom-scheme URLs differently per platform, so `scrald-asset://` and `scrald-theme://` URLs must be built in one place rather than as hard-coded strings.
- **Keyboard shortcuts.** Ctrl-based shortcuts map to Cmd on macOS.
- **Packaging.** `.dmg` for macOS (signing and notarization needed for smooth installs), AppImage and/or `.deb` for Linux.

---

## 12. Persistence

A SQLite database in the app data directory:

| Table | Contents |
|---|---|
| `documents` | Canonical path, file identity hint (size + first-KB hash, used to reconnect moved files), theme override, flavor override, scroll anchor, text sizing mode, zoom, window geometry (size, position, maximized), last opened, pinned |
| `settings` | Global settings key/value |
| `font_cache` | Downloaded Google font families and file paths |

The database stores only metadata, never document contents. The one place Scrald keeps document text outside the document itself is crash-recovery files (section 9.3), stored as separate files in a `recovery/` folder in the app data directory and deleted once the changes are saved or discarded.

---

## 13. Security

- All rendered HTML passes through `ammonia`, with an allowlist that covers what Markdown and the transforms produce (including KaTeX and Mermaid output containers). `<script>`, event handler attributes, `javascript:` URLs, and `<iframe>` are always stripped.
- A strict Tauri content security policy: no remote scripts; images only from `scrald-asset:`, `scrald-theme:`, and `data:` (plus remote origins when the user enables remote images for a document).
- The asset protocol only serves files resolved for the currently open documents and theme assets.
- `theme.css` can style the page but cannot run code. Theme CSS `url()` references are limited to the theme's own assets and cached fonts.

---

## 14. Testing strategy

- **Core unit tests:** each transform pass, flavor detection, source-range conversion (including CRLF and multi-byte UTF-8), YAML minimal edits, edit splicing, theme parsing, and terminal-theme import.
- **Snapshot tests** using `insta`: fixture Markdown files in `crates/scrald-core/tests/fixtures/` rendered to HTML, with snapshots reviewed on change.
- **Round-trip tests:** applying a no-op edit to any block must reproduce the original file byte-for-byte.
- **Benchmarks** (`criterion`): parse and render times on the generated 100K, 250K, and 500K-word fixtures.
- **Frontend tests:** Vitest for logic such as scroll anchoring and theme variable application. Manual performance checks using WebView2 DevTools on the large fixtures.

---

## 15. Open questions

1. ~~**Front matter namespacing:** Is `scrald: { theme, flavor }` acceptable, or should these be flat keys such as `scrald-theme`?~~ **Resolved (2026-10-08):** flat `scrald-*` keys; the nested form is read but never written. See section 8.1.
2. ~~**Autosave:** off by default for v1, or on?~~ **Resolved (2026-10-08):** off by default, with a prompt on close and crash-recovery files covering lost work. See section 9.3.
3. ~~**Note transclusion** (`![[Other note]]`): needed for v1, or post-v1?~~ **Resolved (2026-10-08):** post-v1. In v1, note embeds render as a link card that opens the target note (section 5.3). Things to design when it's picked up: cycle detection and a depth limit, heading and block-reference embeds (`![[Note#Heading]]`, `![[Note#^id]]`), source ranges for blocks that come from another file, whether embedded content is editable in place, and live reload when the embedded file changes.
4. ~~**Platform scope:** Windows-only for v1, or keep macOS and Linux builds working from the start?~~ **Resolved (2026-10-08):** `scrald-core` is tested on all three platforms from the start; the app is Windows first, with macOS and Linux app support as the final milestone of v1. See section 11.1.

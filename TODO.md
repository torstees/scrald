# Scrald: TODO

Milestones are ordered so that each one leaves a usable, testable app. Check items off as they're completed, and add follow-ups where they come up. See `DESIGN.md` for details on each area (section numbers in parentheses).

---

## M0: Scaffolding

- [ ] Cargo workspace with `crates/scrald-core` and `crates/scrald-app`
- [ ] Tauri 2 app in `scrald-app` with Svelte 5 + TypeScript + Vite frontend in `ui/`
- [x] `.gitattributes` (LF everywhere in repo and working tree; fixtures stored byte-for-byte with `-text`) and `.editorconfig`
- [ ] `.gitignore`, `rustfmt.toml` (`newline_style = "Unix"`), clippy config, TypeScript strict mode; Prettier `endOfLine: "lf"` if Prettier is added
- [ ] Confirm all commands in `AGENTS.md` work on Windows; update the table to match reality
- [ ] Minimal "hello" window that receives a file path from the command line
- [ ] Enable `tracing` logging in the app
- [ ] CI (GitHub Actions): `fmt`, `clippy`, and `test` for `scrald-core` on Windows, macOS, and Linux; full workspace on Windows

## M1: Core parsing and rendering (§3, §5)

- [ ] `SourceRange`, `Block`, `BlockKind`, `DocumentModel` types
- [ ] Read file: detect and preserve BOM and line endings
- [ ] Front matter split + parse into `FrontMatter` with aliases (§8.1); unknown keys preserved
- [ ] Read `scrald-theme` / `scrald-flavor`, falling back to nested `scrald: { theme, flavor }`; flat key wins; test both forms
- [ ] Manual check: how the installed Obsidian version shows a nested `scrald:` map in Properties (confirms the §8.1 rationale)
- [ ] Parse body with comrak (GFM options), source positions enabled
- [ ] Line-start table and line/column → byte offset conversion; tests for CRLF and multi-byte UTF-8
- [ ] Render each top-level block to HTML independently; sanitize with `ammonia`
- [ ] Group blocks into heading sections
- [ ] TOC extraction with stable heading IDs (slugs, deduplicated)
- [ ] Word count and `FeatureFlags` (has_math, has_mermaid, has_code)
- [ ] Snapshot test harness with `insta` and a starter fixture set (GFM basics, tables, footnotes, front matter, raw HTML)
- [ ] Round-trip test: reconstructing the file from block ranges + gaps reproduces original bytes
- [ ] Large fixture generator (100K / 250K / 500K words) and a `criterion` benchmark

## M2: Reader shell (§4, §6, §10, §11)

- [ ] Tauri command `open_document(path) -> DocumentModel`
- [ ] Reader component rendering sections with `content-visibility: auto` and intrinsic size estimates
- [ ] Progressive mount around the initial position
- [ ] TOC sidebar: click-to-jump, current section highlight, collapse levels, toggle with Ctrl+\
- [ ] `scrald-asset://` protocol with restricted serving; image path resolution order (§6.1)
- [ ] Missing-image placeholder with attempted paths
- [ ] Image dimensions read in Rust to reserve layout space
- [ ] Remote images blocked by default, per-document toggle
- [ ] External links open in browser; Markdown links open in Scrald with back/forward
- [ ] File watching with debounce; live reload preserving scroll anchor
- [ ] Scroll anchor model (block source offset + fraction)
- [ ] Status bar: word count, reading time, current section
- [ ] Measure performance on large fixtures against the budget in §4; record results here

## M3: Themes, typography, and memory (§7, §12)

- [ ] SQLite state store: `documents`, `settings` tables; recent documents tracked
- [ ] Theme schema structs + TOML loading with clear validation errors
- [ ] Theme tokens → CSS custom properties; reader styles use only `--sk-*` variables
- [ ] Three or four bundled themes (e.g. light serif, dark, sepia, technical/sans)
- [ ] User theme directory, duplicate-bundled-theme action, hot reload
- [ ] `scrald-theme://` protocol for theme assets
- [ ] Measure (default 66ch) with rewrap on narrow windows; "fill window" toggle
- [ ] `fit` text sizing mode: font scales up and down with window width, clamped to min/max, anchored on resize
- [ ] Smooth zoom (Ctrl+wheel, Ctrl+plus/minus, Ctrl+0, pinch) anchored at cursor/viewport
- [ ] System font enumeration with `fontdb`; fonts from theme assets
- [ ] Theme switcher (Ctrl+T) with live preview
- [ ] Theme resolution order: per-document override → front matter → folder `.scrald.toml` → global default
- [ ] Remember per-document theme, scroll position, text sizing mode, zoom, and window geometry (size, position, maximized)
- [ ] Clamp restored windows to a connected monitor; new documents open at the last-used window size
- [ ] "Make this the default" action for text sizing and zoom
- [ ] Blockquote and horizontal rule styles from theme (`[elements.*]`)

## M4: Flavors and rich content (§5, §6.3)

- [ ] `Flavor` enum, comrak options per profile, conflict resolution (`~x~`, math rules)
- [ ] Flavor detection heuristics with tests on representative fixtures
- [ ] Flavor resolution order and status bar flavor switcher; store only explicit overrides
- [ ] Transform: attribute blocks `{#id .class key=val}`
- [ ] Image layout classes: `.left`, `.right`, `.center`, `.full-bleed`, `.inline`; figures with captions
- [ ] Transform: fenced divs and bracketed spans
- [ ] Transform: Obsidian callouts (incl. foldable) and GitHub alerts
- [ ] Transform: wikilinks and image embeds with size syntax; vault root detection; unresolved link style
- [ ] Note embeds `![[Other note]]` render as a link card that opens the note (v1 stand-in for transclusion)
- [ ] Transform: `%%comments%%`, `==highlight==`, inline `#tags`, block-reference anchors
- [ ] Inline footnotes `^[...]`; footnote hover popovers
- [ ] Syntax highlighting via syntect with class output; palette-derived syntax colors
- [ ] KaTeX, lazily loaded and rendered on approach to viewport
- [ ] Mermaid, lazily loaded only when present
- [ ] Wide tables scroll in their own container; table layout classes

## M5: Front matter panel (§8)

- [ ] Collapsible properties panel (read-only first)
- [ ] Minimal-edit YAML engine: scalars, flow lists, block lists; tests preserving comments and formatting
- [ ] Editable fields: title, author, summary, notes, source, assets (folder picker)
- [ ] Tag chips with add/remove
- [ ] Theme and flavor dropdowns writing flat `scrald-theme` / `scrald-flavor` keys ("Save to document")
- [ ] Read-only fallback for complex YAML with "edit in raw mode" link

## M6: Editing (§9)

- [ ] Document buffer with dirty state and document-level undo/redo
- [ ] Atomic save preserving line endings and BOM; Ctrl+S
- [ ] Raw mode toggle (Ctrl+E) with CodeMirror 6; scroll position carried both ways
- [ ] Block editing: double-click to edit, commit on blur / Esc / Ctrl+Enter
- [ ] Edit splicing in core; full reparse; re-render only changed blocks by hash
- [ ] External change while dirty: banner with keep / reload / compare options
- [ ] Prompt on window close / app quit with unsaved changes (save / discard / cancel)
- [ ] Crash recovery: periodic recovery files in app data `recovery/`, deleted on save/discard; offer restore on launch or reopen
- [ ] Autosave setting (off by default)

## M7: Startup and integration (§11)

- [ ] Start screen: recent and pinned documents, Open button, drag and drop
- [ ] File associations in the bundler config (`.md`, `.markdown`, `.mdown`) without forcing default
- [ ] Single-instance plugin: new files open as new windows in the running process
- [ ] Windows installer build and smoke test (open from Explorer, "Open with")

## M8: Fonts and theme creation (§7.3, §7.5)

- [ ] Google Fonts download and local cache; `font_cache` table; fallback while downloading
- [ ] Terminal theme import: Windows Terminal JSON
- [ ] Terminal theme import: iTerm2, Alacritty, Ghostty, kitty, base16/base24
- [ ] Derived semantic colors from imported palettes
- [ ] Simple in-app theme editor (colors, fonts, measure, element styles) writing `theme.toml`

## M9: Polish

- [ ] In-document search (Ctrl+F) in core, mapped to blocks, with highlighted matches
- [ ] Settings UI
- [ ] Keyboard shortcut reference
- [ ] Accessibility pass (focus order, contrast checks for bundled themes, reduced-motion support for zoom animation)
- [ ] Print / export to PDF with the active theme
- [ ] Reading progress indicator

## M10: macOS and Linux (§11.1) — final v1 milestone

- [ ] Extend CI to build and test the full workspace (including `scrald-app`) on macOS and Linux
- [ ] Check `content-visibility` and `requestIdleCallback` on WKWebView and WebKitGTK; add fallbacks where missing
- [ ] Re-measure the §4 performance budget on each platform; record results here
- [ ] macOS: handle files opened from Finder (opened app event); verify single-instance behavior
- [ ] File associations: Info.plist (macOS), `.desktop` + MIME types (Linux)
- [ ] Verify custom protocol URLs (`scrald-asset://`, `scrald-theme://`) on both platforms
- [ ] Cmd instead of Ctrl for shortcuts on macOS; update the shortcut reference
- [ ] Packaging: `.dmg` with signing and notarization (macOS), AppImage and/or `.deb` (Linux); smoke test each

## Later / ideas

- [ ] Note transclusion `![[Other note]]` (open design points listed in DESIGN §15 Q3)
- [ ] Finer-grained block editing (list items, blockquote children)
- [ ] Virtualized rendering, only if profiling shows it's needed
- [ ] Pandoc grid tables and line blocks
- [ ] Revisit autosave default once the edit/save path has proven reliable in real use

---

## Decisions log

Record notable decisions made during implementation here (date, decision, reason), so `DESIGN.md` can be updated to match.

| Date | Decision | Reason |
|---|---|---|
| 2026-10-08 | Front matter settings use flat `scrald-theme` / `scrald-flavor` keys; nested `scrald:` map is read but never written (DESIGN §8.1, §15 Q1) | Obsidian's Properties panel can't display or edit nested maps; flat keys keep the minimal-edit YAML engine to top-level scalars |
| 2026-10-08 | Autosave off by default for v1; add close prompt and crash-recovery files in app data (DESIGN §9.3, §12, §15 Q2) | New edit/save path shouldn't write to user files unattended; recovery files cover crashes without touching the document |
| 2026-10-08 | Note transclusion is post-v1; v1 renders `![[Other note]]` as a link card (DESIGN §5.3, §15 Q3) | Keeps v1 scope down; transclusion raises cross-file source-range, editing, and cycle questions worth designing properly |
| 2026-10-08 | Windows first; macOS and Linux support is the final v1 milestone, M10 (DESIGN §11.1, §15 Q4) | Focus early work on one platform while keeping code portable so the port is mostly verification and packaging |
| 2026-10-08 | LF line endings for all repo files, enforced by `.gitattributes` (`eol=lf`) and `.editorconfig`; test fixtures exempt (`-text`) | Maintainer preference; also keeps diffs clean across platforms. Fixtures must keep CRLF/BOM bytes to test preservation |
| 2026-10-08 | `scrald-core` is tested on Windows, macOS, and Linux in CI from M0 (DESIGN §11.1) | Core has no GUI dependencies, so cross-platform CI is nearly free and catches portability slips early |

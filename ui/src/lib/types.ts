import type { ScrollAnchor } from "../reader/anchor";
import type { TextSizing } from "../typography/typography";

// TypeScript mirrors of the Tauri command payloads (crates/scrald-app/src/commands.rs)
// and the core document model (crates/scrald-core/src/document.rs and friends).
// Keep these in sync with the Rust types; all use serde `rename_all = "camelCase"`.

/** Mirrors `LaunchInfo`. */
export interface LaunchInfo {
  /** Absolute path from the command line, or null if none was given. */
  path: string | null;
  /** Display title for the window and page. */
  title: string;
  /** Whether `path` existed when the app started. */
  exists: boolean;
}

/** Mirrors `SourceRange`: a half-open byte range into the original file. */
export interface SourceRange {
  start: number;
  end: number;
}

/** Mirrors `BlockKind` (serde `tag = "type"`). */
export type BlockKind =
  | { type: "heading"; level: number }
  | { type: "paragraph" }
  | { type: "list"; ordered: boolean }
  | { type: "blockQuote" }
  | { type: "codeBlock"; language: string | null }
  | { type: "table" }
  | { type: "thematicBreak" }
  | { type: "html" }
  | { type: "footnoteDefinition" }
  | { type: "descriptionList" }
  | { type: "other" };

/** Mirrors `Block`. */
export interface Block {
  id: number;
  kind: BlockKind;
  /** Sanitized HTML produced by core. The only HTML the UI may insert. */
  html: string;
  source: SourceRange;
  hash: number;
  section: number;
  /** Words in the block; they add up to `DocumentModel.wordCount`. */
  words: number;
}

/** Mirrors `Section`. */
export interface Section {
  id: number;
  headingBlock: number | null;
  level: number;
  firstBlock: number;
  blockCount: number;
}

/** Mirrors `TocEntry`. */
export interface TocEntry {
  level: number;
  text: string;
  slug: string;
  /** The block holding the heading: the heading itself, or a fenced div or HTML container around it. */
  blockId: number;
  /** Byte offset of the heading in the file. */
  offset: number;
  /** Outline number like "2.1"; null for the document title. */
  number: string | null;
}

/** Mirrors `FeatureFlags`. */
export interface FeatureFlags {
  hasMath: boolean;
  hasMermaid: boolean;
  hasCode: boolean;
  /** ABC music notation (rendered with abcjs). */
  hasAbc: boolean;
}

/** Mirrors `FrontMatter`. */
export interface FrontMatter {
  range: SourceRange;
  title: string | null;
  authors: string[];
  summary: string | null;
  tags: string[];
  notes: string | null;
  source: string | null;
  assets: string | null;
  theme: string | null;
  flavor: string | null;
  /** A cover image path as written (`cover`, `image`, `heroImage`, ...); its key is also in `extra`. */
  cover: string | null;
  /** Unrecognized keys, in file order. Values are arbitrary YAML data. */
  extra: Record<string, unknown>;
  /** The key each property was read from (`author` or `authors`, ...). */
  keys: PropertyKeys;
  /** Every top-level key in file order, and whether it can be edited. */
  fields: FieldInfo[];
  error: string | null;
}

/** Mirrors `PropertyKeys`. */
export interface PropertyKeys {
  title: string | null;
  authors: string | null;
  summary: string | null;
  tags: string | null;
  notes: string | null;
  source: string | null;
  assets: string | null;
  theme: string | null;
  flavor: string | null;
}

/** Mirrors `FieldInfo` (crates/scrald-core/src/yaml_edit.rs). */
export interface FieldInfo {
  key: string;
  editable: boolean;
  /** Why it can't be edited in the panel, when it can't. */
  reason: string | null;
  /** Byte offset in the original file where the key's line starts. */
  offset: number;
}

/** Mirrors `ImageAsset`: a local image file the document uses. */
export interface ImageAsset {
  id: number;
  path: string;
  width: number | null;
  height: number | null;
}

/** Mirrors `Flavor` (crates/scrald-core/src/flavor.rs). */
export type Flavor = "gfm" | "obsidian" | "pandoc";

/** Mirrors `FlavorSource`: why a document has its flavor. */
export type FlavorSource = "document" | "frontMatter" | "folder" | "detected" | "default";

/** Mirrors `Footnote`: one footnote, rendered, for popovers and endnotes. */
export interface Footnote {
  /** References link to `#fn-<name>`; the first reference has id `fnref-<name>`. */
  name: string;
  /** The number shown at its references. */
  number: number;
  /** Sanitized HTML of the footnote's contents (no back-links). */
  html: string;
  /** The defining block, or null for an inline (`^[...]`) footnote. */
  blockId: number | null;
}

/** Mirrors `LineEnding` (serde `rename_all = "lowercase"`). */
export type LineEnding = "lf" | "crlf";

/** Mirrors `DocumentModel`. */
export interface DocumentModel {
  path: string;
  hasBom: boolean;
  lineEnding: LineEnding;
  frontMatter: FrontMatter | null;
  blocks: Block[];
  sections: Section[];
  toc: TocEntry[];
  wordCount: number;
  features: FeatureFlags;
  /** Local images; block HTML refers to them as `<img data-asset="id">`. */
  images: ImageAsset[];
  /** Number of remote (http/https) images in the document. */
  remoteImages: number;
  /** Whether remote images were allowed to load in this parse. */
  remoteImagesAllowed: boolean;
  flavor: Flavor;
  flavorSource: FlavorSource;
  /** Every footnote in numbered order, including inline (`^[...]`) ones. */
  footnotes: Footnote[];
}

/** Mirrors `LinkTarget` (crates/scrald-core/src/links.rs, serde `tag = "kind"`). */
export type LinkTarget =
  | { kind: "external"; url: string }
  | { kind: "document"; path: string; fragment: string | null }
  | { kind: "fragment"; id: string }
  | { kind: "localFile"; path: string }
  | { kind: "unsupported"; href: string };

/** Mirrors `DocumentMemory` (crates/scrald-app/src/state.rs). */
export interface DocumentMemory {
  /** Where the reader left off, if the document was opened before. */
  anchor: ScrollAnchor | null;
  /** Whether the user allowed remote images for this document. */
  remoteImages: boolean;
  /** Theme the user picked for this document in Scrald, if any. */
  theme: string | null;
  /** Text sizing mode chosen for this document, if any. */
  textSizing: TextSizing | null;
  /** Zoom chosen for this document, if any. */
  zoom: number | null;
  /** Flavor chosen for this document, if any. */
  flavor: Flavor | null;
}

/** Mirrors `TypographyDefaults` (crates/scrald-app/src/state.rs). */
export interface TypographyDefaults {
  textSizing: TextSizing | null;
  zoom: number | null;
  fillWindow: boolean;
}

/** Mirrors `ThemeLayout` (crates/scrald-app/src/commands.rs). */
export interface ThemeLayout {
  measure: number;
  fontSize: number;
  minFontSize: number;
  maxFontSize: number;
  textSizing: TextSizing;
}

/** Mirrors `Appearance` (crates/scrald-core/src/theme/schema.rs). */
export type Appearance = "light" | "dark";

/** Mirrors `ThemeSource`: where a document's theme choice came from. */
export type ThemeSource = "document" | "frontMatter" | "folder" | "default";

/** Mirrors `ResolvedTheme` (crates/scrald-app/src/commands.rs). */
export interface ResolvedTheme {
  id: string;
  source: ThemeSource;
}

/** Mirrors `ThemeChange` (crates/scrald-core/src/theme/edit.rs): set `key`, or remove it with null. */
export interface ThemeChange {
  key: string;
  value: string | number | boolean | (string | number)[] | null;
}

/** Mirrors `ColorValue`: what the file sets, and what the theme uses. */
export interface ThemeColorValue {
  key: string;
  value: string | null;
  resolved: string;
}

/** Mirrors `FontValue`; `family` is null when the theme leaves the slot out. */
export interface ThemeFontValue {
  slot: "body" | "heading" | "mono" | "ui";
  family: string | null;
  source: "system" | "google" | "theme";
  weights: number[];
  italic: boolean;
  fallback: string[];
}

/** Mirrors `EditorValues` (crates/scrald-core/src/theme/edit.rs). */
export interface ThemeEditorValues {
  name: string;
  appearance: "light" | "dark";
  colors: ThemeColorValue[];
  fonts: ThemeFontValue[];
  measure: number;
  fontSize: number;
  lineHeight: number;
  paragraphSpacing: number;
  textSizing: "fixed" | "fit";
  blockquoteStyle: "bar" | "indent" | "boxed" | "pull-quote";
  blockquoteItalic: boolean;
  hrStyle: "line" | "dots" | "ornament" | "image";
  hrGlyph: string | null;
  numbering: boolean;
  h1Rule: boolean;
}

/** Mirrors `ThemeDetails` (crates/scrald-app/src/commands.rs). */
export interface ThemeDetails {
  id: string;
  editable: boolean;
  folder: string | null;
  values: ThemeEditorValues;
  /** Color pairs below WCAG AA, worst first. */
  contrast: ContrastProblem[];
}

/** Mirrors `ContrastProblem` (crates/scrald-core/src/theme/contrast.rs). */
export interface ContrastProblem {
  /** What is drawn on what, e.g. "muted text on the sidebar". */
  what: string;
  ratio: number;
  minimum: number;
  /** Theme color keys involved: "muted", "ui_background", "palette.green", ... */
  colors: string[];
  /** The two colors as drawn, for a sample. */
  fg: string;
  bg: string;
}

/** Mirrors `ThemeEditPreview` (crates/scrald-app/src/commands.rs). */
export interface ThemeEditPreview {
  style: ThemeStyle;
  values: ThemeEditorValues;
  contrast: ContrastProblem[];
}

/** Mirrors `TerminalThemeFile` (crates/scrald-app/src/commands.rs). */
export interface TerminalThemeFile {
  path: string;
  /** The color schemes in the file, in file order. */
  names: string[];
}

/** Mirrors `ThemeSummary` (crates/scrald-core/src/theme/mod.rs). */
export interface ThemeSummary {
  id: string;
  name: string;
  author: string | null;
  appearance: Appearance | null;
  bundled: boolean;
  background: string;
  foreground: string;
  accent: string;
  /** Why a user theme couldn't be loaded, if it couldn't. */
  error: string | null;
}

/** Mirrors `ThemeStyle` (crates/scrald-app/src/commands.rs). */
export interface ThemeStyle {
  id: string;
  name: string;
  appearance: Appearance;
  css: string;
  numbering: boolean;
  layout: ThemeLayout;
}

/** Mirrors `RecentDocument` (crates/scrald-app/src/state.rs). */
export interface RecentDocument {
  path: string;
  /** The title it had when last opened or saved (null for older entries). */
  title: string | null;
  /** The theme it was last shown in, for the swatch. */
  theme: string | null;
  /** The front matter's summary, on one line. */
  summary: string | null;
  /** The cover image, served at the asset URL for `${token}-${id}`; null if none. */
  cover: { token: number; id: number } | null;
  /** Unix time in milliseconds. */
  lastOpened: number;
  pinned: boolean;
  /** Whether the file is still there. */
  exists: boolean;
}

/** Mirrors `Hits` (crates/scrald-core/src/search.rs): matches in one block or footnote. */
export interface SearchHits {
  /** Block id, or index in `DocumentModel.footnotes`. */
  index: number;
  count: number;
}

/** Mirrors `SearchReply` (crates/scrald-app/src/commands.rs): matches in model `model`. */
export interface SearchReply {
  model: number;
  total: number;
  /** Blocks with matches, in order. */
  blocks: SearchHits[];
  /** Footnotes with matches, in order (shown as endnotes after the last block). */
  footnotes: SearchHits[];
}

/** Mirrors `OpenedDocument` (crates/scrald-app/src/commands.rs). */
export interface OpenedDocument {
  /** Image `id` is served at the asset URL for `${assetToken}-${id}`. */
  assetToken: number;
  document: DocumentModel;
  /** What Scrald remembers about this document from earlier sessions. */
  memory: DocumentMemory;
  /** The theme this document uses, and where that choice came from. */
  theme: ResolvedTheme;
  /** Whether the window holds edits that aren't saved yet. */
  dirty: boolean;
  /** Unsaved changes left behind when Scrald stopped (only when opening). */
  recovered: RecoveredChanges | null;
  /** This model's number. */
  model: number;
  /**
   * The model this window must be showing to fill in left-out block HTML;
   * null when every block has its HTML (see `ModelTracker`).
   */
  base: number | null;
  /** For each block, the block in `base` whose HTML it has, or null if its HTML is included. */
  htmlFrom: (number | null)[];
}

/** Mirrors `RecoveredChanges` (crates/scrald-app/src/commands.rs): unsaved changes from a recovery file. */
export interface RecoveredChanges {
  id: string;
  path: string;
  /** The title in the recovered text. */
  title: string;
  /** When the changes were last written, in milliseconds since the epoch. */
  savedAt: number;
  /** The file changed after these changes were made. */
  fileChanged: boolean;
  /** The file can't be read any more; the changes can only be copied. */
  fileMissing: boolean;
}

/** Mirrors `Change` (crates/scrald-core/src/yaml_edit.rs): a new value for a front matter key. */
export type PropertyChange =
  | { kind: "text"; value: string }
  /** A number or boolean typed by the user, kept as that type when it is one. */
  | { kind: "value"; value: string }
  | { kind: "list"; value: string[] }
  | { kind: "remove" };

/** Mirrors `RawText`: the whole document text (no BOM) and its version. */
export interface RawText {
  text: string;
  version: number;
}

/** Mirrors `RawSync`: after replacing the text from raw mode. */
export interface RawSync {
  version: number;
  dirty: boolean;
}

/** Mirrors `DiskState` (crates/scrald-app/src/session.rs). */
export type DiskState = "unchanged" | "changed" | "missing";

/**
 * Mirrors `SaveOutcome` (crates/scrald-app/src/session.rs): "conflict" means
 * the file changed on disk and nothing was written.
 */
export type SaveOutcome = "saved" | "conflict";

/** Where the properties panel goes (DESIGN.md §8.2); mirrors `PropertiesPlacement` in state.rs. */
export type PropertiesPlacement = "top" | "left" | "right";

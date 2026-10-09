import type { ScrollAnchor } from "../reader/anchor";

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
  blockId: number;
}

/** Mirrors `FeatureFlags`. */
export interface FeatureFlags {
  hasMath: boolean;
  hasMermaid: boolean;
  hasCode: boolean;
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
  /** Unrecognized keys, in file order. Values are arbitrary YAML data. */
  extra: Record<string, unknown>;
  error: string | null;
}

/** Mirrors `ImageAsset`: a local image file the document uses. */
export interface ImageAsset {
  id: number;
  path: string;
  width: number | null;
  height: number | null;
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
}

/** Mirrors `RecentDocument` (crates/scrald-app/src/state.rs). */
export interface RecentDocument {
  path: string;
  /** Unix time in milliseconds. */
  lastOpened: number;
  pinned: boolean;
}

/** Mirrors `OpenedDocument` (crates/scrald-app/src/commands.rs). */
export interface OpenedDocument {
  /** Image `id` is served at the asset URL for `${assetToken}-${id}`. */
  assetToken: number;
  document: DocumentModel;
  /** What Scrald remembers about this document from earlier sessions. */
  memory: DocumentMemory;
}

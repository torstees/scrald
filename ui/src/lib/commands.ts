// Typed wrappers around the Tauri commands, so components never call
// `invoke` with raw strings.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ScrollAnchor } from "../reader/anchor";
import type {
  LaunchInfo,
  LinkTarget,
  OpenedDocument,
  PropertyChange,
  DiskState,
  SaveOutcome,
  RawSync,
  RawText,
  RecentDocument,
  RecoveredChanges,
  ThemeStyle,
  ThemeSummary,
  TypographyDefaults,
  Flavor,
} from "./types";
import type { TextSizing } from "../typography/typography";
import { ModelTracker } from "./models";

// The document model this window shows, so the app can leave out the HTML
// of blocks that didn't change.
const models = new ModelTracker();

/** Fills in block HTML the app left out (fetching the whole model if needed). */
async function complete(opened: OpenedDocument): Promise<OpenedDocument> {
  const filled = models.receive(opened);
  if (filled) return filled;
  // It builds on a model this window doesn't have: ask for every block.
  const full = await invoke<OpenedDocument>("reparse_document", { full: true });
  return models.receive(full) ?? full;
}

export function launchInfo(): Promise<LaunchInfo> {
  return invoke<LaunchInfo>("launch_info");
}

/**
 * Reads and parses a document, with what Scrald remembers about it (reading
 * position, remote images choice). Rejects with an error message string.
 */
export async function openDocument(path: string): Promise<OpenedDocument> {
  return complete(await invoke<OpenedDocument>("open_document", { path }));
}

/** Parses the window's document again from memory, keeping unsaved edits. */
export async function reparseDocument(): Promise<OpenedDocument> {
  return complete(await invoke<OpenedDocument>("reparse_document", { full: false }));
}

/** Changes one front matter key in memory; the file changes on save. */
export async function editFrontMatter(key: string, change: PropertyChange): Promise<OpenedDocument> {
  return complete(await invoke<OpenedDocument>("edit_front_matter", { key, change }));
}

/** A block's Markdown source (its `source` range), for the block editor. */
export function blockSource(start: number, end: number): Promise<string> {
  return invoke<string>("block_source", { start, end });
}

/** Replaces a block's source in memory; refused if the document changed since `original` was read. */
export async function editBlock(start: number, end: number, original: string, text: string): Promise<OpenedDocument> {
  return complete(await invoke<OpenedDocument>("edit_block", { start, end, original, text }));
}

/** Undoes (or with `redo`, redoes) one edit; null if there's nothing to do. */
export async function stepHistory(redo: boolean): Promise<OpenedDocument | null> {
  const opened = await invoke<OpenedDocument | null>("step_history", { redo });
  return opened && complete(opened);
}

/** The whole document text and its version, for raw mode. */
export function documentText(): Promise<RawText> {
  return invoke<RawText>("document_text");
}

/** Replaces the whole text from raw mode; refused if `version` is out of date. */
export function setDocumentText(version: number, text: string): Promise<RawSync> {
  return invoke<RawSync>("set_document_text", { version, text });
}

/** Saves the window's document atomically. */
export function saveDocument(): Promise<SaveOutcome> {
  return invoke<SaveOutcome>("save_document");
}

/** The changed disk version's text, for comparing (after `checkDisk` found one). */
export function diskText(): Promise<string> {
  return invoke<string>("disk_text");
}

/** "Keep mine": the next save may replace the changed disk version. */
export function keepLocalVersion(): Promise<void> {
  return invoke("keep_local_version");
}

/** "Load disk version": replaces the text with it (undoable), re-parsed. */
export async function loadDiskVersion(): Promise<OpenedDocument> {
  return complete(await invoke<OpenedDocument>("load_disk_version"));
}

/** Whether documents save themselves after a pause in editing. */
export function autosave(): Promise<boolean> {
  return invoke<boolean>("autosave");
}

/** Turns autosave on or off, for every window. */
export function setAutosave(enabled: boolean): Promise<void> {
  return invoke<void>("set_autosave", { enabled });
}

/** Calls `handler` when autosave is turned on or off (in any window). */
export function onAutosaveChanged(handler: (enabled: boolean) => void): Promise<UnlistenFn> {
  return listen<boolean>("autosave-changed", (event) => handler(event.payload));
}

/** Unsaved changes left behind when Scrald stopped, newest first. */
export function recoveredChanges(): Promise<RecoveredChanges[]> {
  return invoke<RecoveredChanges[]>("recovered_changes");
}

/** Restores recovered changes into the open document (undoable), re-parsed. */
export async function restoreRecovered(id: string): Promise<OpenedDocument> {
  return complete(await invoke<OpenedDocument>("restore_recovered", { id }));
}

/** Deletes recovered changes the user chose not to restore. */
export function discardRecovered(id: string): Promise<void> {
  return invoke<void>("discard_recovered", { id });
}

/** The text of recovered changes, for copying. */
export function recoveredText(id: string): Promise<string> {
  return invoke<string>("recovered_text", { id });
}

/** How the file on disk compares with what this window last read or saved. */
export function checkDisk(): Promise<DiskState> {
  return invoke<DiskState>("check_disk");
}

/** Asks for the assets folder; returns it relative to the document, or null if cancelled. */
export function pickAssetsFolder(document: string): Promise<string | null> {
  return invoke<string | null>("pick_assets_folder", { document });
}

/** Remembers the reading position in a document. Never throws. */
export function saveReadingPosition(path: string, anchor: ScrollAnchor): void {
  invoke("save_reading_position", { path, anchor }).catch((e: unknown) => {
    console.warn("could not save reading position", e);
  });
}

/** Remembers whether remote images may load for a document. */
export function setRemoteImages(path: string, allowed: boolean): Promise<void> {
  return invoke("set_remote_images", { path, allowed });
}

/** Every theme, including broken user themes (with `error` set). */
export function listThemes(): Promise<ThemeSummary[]> {
  return invoke<ThemeSummary[]>("list_themes");
}

/** CSS and details for one theme. */
export function themeStyle(id: string): Promise<ThemeStyle> {
  return invoke<ThemeStyle>("theme_style", { id });
}

/** Sets a document's theme; `null` returns it to its default. */
export function setDocumentTheme(path: string, id: string | null): Promise<void> {
  return invoke("set_document_theme", { path, id });
}

export function setDefaultTheme(id: string): Promise<void> {
  return invoke("set_default_theme", { id });
}

/** Copies a theme into the user theme folder; resolves to the new theme's id. */
export function duplicateTheme(id: string): Promise<string> {
  return invoke<string>("duplicate_theme", { id });
}

/** Where user themes live, or null if there's no such folder. */
export function userThemeFolder(): Promise<string | null> {
  return invoke<string | null>("user_theme_folder");
}

/** Calls `handler` whenever a file in the user theme folder changes. */
export function onThemesChanged(handler: () => void): Promise<UnlistenFn> {
  return listen("themes-changed", () => handler());
}

/** Remembers a document's text sizing mode and zoom; `null` clears either. Never throws. */
export function setDocumentTypography(path: string, textSizing: TextSizing | null, zoom: number | null): void {
  invoke("set_document_typography", { path, textSizing, zoom }).catch((e: unknown) => {
    console.warn("could not save typography", e);
  });
}

/** Sets a document's flavor; `null` returns to front matter, folder config, or detection. */
export function setDocumentFlavor(path: string, flavor: Flavor | null): Promise<void> {
  return invoke("set_document_flavor", { path, flavor });
}

/** Whether the properties panel starts open (global preference). */
export function propertiesOpen(): Promise<boolean> {
  return invoke<boolean>("properties_open");
}

export function setPropertiesOpen(open: boolean): Promise<void> {
  return invoke("set_properties_open", { open });
}

export function typographyDefaults(): Promise<TypographyDefaults> {
  return invoke<TypographyDefaults>("typography_defaults");
}

export function setTypographyDefaults(defaults: TypographyDefaults): Promise<void> {
  return invoke("set_typography_defaults", { defaults });
}

/** Recently opened documents, newest (and pinned) first. */
export function recentDocuments(limit: number): Promise<RecentDocument[]> {
  return invoke<RecentDocument[]>("recent_documents", { limit });
}

/**
 * URL of a document image served by the `scrald-asset` protocol. Tauri
 * formats custom-scheme URLs differently per platform (DESIGN.md §11.1), so
 * this is the one place they are built.
 */
export function assetUrl(assetToken: number, imageId: number): string {
  return convertFileSrc(`${assetToken}-${imageId}`, "scrald-asset");
}

/** Classifies a link clicked in the document at `documentPath`. */
export function resolveLink(documentPath: string, href: string): Promise<LinkTarget> {
  return invoke<LinkTarget>("resolve_link", { document: documentPath, href });
}

/** Opens an http(s) or mailto link in the system's default handler. */
export function openExternal(url: string): Promise<void> {
  return invoke("open_external", { url });
}

/** Calls `handler` with the path whenever this window's document changes on disk. */
export function onDocumentChanged(handler: (path: string) => void): Promise<UnlistenFn> {
  return listen<string>("document-changed", (event) => handler(event.payload));
}

/** Sends a frontend timing to the backend log. Never throws. */
export function reportTiming(name: string, ms: number): void {
  invoke("report_timing", { name, ms }).catch(() => {
    // Timing is best-effort diagnostics; losing one measurement is fine.
  });
}

// Typed wrappers around the Tauri commands, so components never call
// `invoke` with raw strings.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ScrollAnchor } from "../reader/anchor";
import type {
  FrontSettings,
  LaunchInfo,
  LinkTarget,
  OpenedDocument,
  PropertiesPlacement,
  PropertyChange,
  DiskState,
  SaveOutcome,
  RawSync,
  RawText,
  RecentDocument,
  ResolvedTheme,
  RecoveredChanges,
  SearchReply,
  TerminalThemeFile,
  ThemeChange,
  ThemeDetails,
  ThemeEditPreview,
  ContrastProblem,
  NewDocumentOptions,
  NewDocumentRequest,
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

/**
 * Searches the document the window shows (DESIGN.md §10). Null while
 * another search of this window is running, or if the matches are for a
 * model other than the one shown (a newer one arrived): ask again.
 */
export async function searchDocument(query: string, matchCase: boolean, wholeWord: boolean): Promise<SearchReply | null> {
  const reply = await invoke<SearchReply | null>("search_document", { query, matchCase, wholeWord });
  return reply && reply.model === models.current ? reply : null;
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

/** The file's text as it is on disk now, for comparing recovered changes with. */
export function fileText(): Promise<string> {
  return invoke<string>("file_text");
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

/** Sets the global default theme; null follows the system's light or dark mode again. */
export function setDefaultTheme(id: string | null): Promise<void> {
  return invoke("set_default_theme", { id });
}

/** The global default theme's id, or null when it follows the system. */
export function defaultTheme(): Promise<string | null> {
  return invoke<string | null>("default_theme");
}

/** Copies a theme into the user theme folder; resolves to the new theme's id. */
export function duplicateTheme(id: string): Promise<string> {
  return invoke<string>("duplicate_theme", { id });
}

/** What the theme editor shows for theme `id`. */
export function themeDetails(id: string): Promise<ThemeDetails> {
  return invoke<ThemeDetails>("theme_details", { id });
}

/** Theme `id`'s color pairs below WCAG AA, worst first. */
export function themeContrast(id: string): Promise<ContrastProblem[]> {
  return invoke<ContrastProblem[]>("theme_contrast", { id });
}

/** Theme `id` as it would look with `changes`, without saving. */
export function previewThemeEdit(id: string, changes: ThemeChange[]): Promise<ThemeEditPreview> {
  return invoke<ThemeEditPreview>("preview_theme_edit", { id, changes });
}

/** Saves `changes` into theme `id`'s theme.toml. */
export function saveThemeEdit(id: string, changes: ThemeChange[]): Promise<void> {
  return invoke<void>("save_theme_edit", { id, changes });
}

/** Installed font family names, sorted. */
export function systemFonts(): Promise<string[]> {
  return invoke<string[]>("system_fonts");
}

/** Asks for a terminal theme file and lists its schemes; null if cancelled. */
export function pickTerminalTheme(): Promise<TerminalThemeFile | null> {
  return invoke<TerminalThemeFile | null>("pick_terminal_theme");
}

/** Creates a theme for each named scheme in the file; returns the new ids. */
export function importTerminalTheme(path: string, names: string[]): Promise<string[]> {
  return invoke<string[]>("import_terminal_theme", { path, names });
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

export function propertiesPlacement(): Promise<PropertiesPlacement> {
  return invoke<PropertiesPlacement>("properties_placement");
}

/** Moves the properties panel, in every window. */
export function setPropertiesPlacement(placement: PropertiesPlacement): Promise<void> {
  return invoke("set_properties_placement", { placement });
}

export function typographyDefaults(): Promise<TypographyDefaults> {
  return invoke<TypographyDefaults>("typography_defaults");
}

export function setTypographyDefaults(defaults: TypographyDefaults): Promise<void> {
  return invoke("set_typography_defaults", { defaults });
}

/** The deepest heading level the contents sidebar lists (1–6). */
export function contentsDepth(): Promise<number> {
  return invoke<number>("contents_depth");
}

export function setContentsDepth(depth: number): Promise<void> {
  return invoke("set_contents_depth", { depth });
}

/** Whether documents show their cover and a missing title (#189). */
export function frontSettings(): Promise<FrontSettings> {
  return invoke<FrontSettings>("front_settings");
}

export function setFrontSettings(settings: FrontSettings): Promise<void> {
  return invoke("set_front_settings", { settings });
}

/** A global setting another window (or this one) changed. */
export type SettingName = "theme.default" | "typography" | "contents.depth" | "properties.placement" | "reader.front";

/** Calls `handler` with the setting's name whenever a global setting changes, in any window. */
export function onSettingsChanged(handler: (name: SettingName) => void): Promise<UnlistenFn> {
  return listen<SettingName>("settings-changed", (event) => handler(event.payload));
}

/** Recently opened documents: pinned first, then newest first. */
export function recentDocuments(limit: number): Promise<RecentDocument[]> {
  return invoke<RecentDocument[]>("recent_documents", { limit });
}

/** Pins a document to the top of the start screen, or unpins it. */
export function setDocumentPinned(path: string, pinned: boolean): Promise<void> {
  return invoke("set_document_pinned", { path, pinned });
}

/** Forgets everything remembered about a document ("Remove from list"). */
export function forgetDocument(path: string): Promise<void> {
  return invoke("forget_document", { path });
}

/** Asks for a file to open, starting near `near`; null if cancelled. */
export function pickDocument(near: string | null): Promise<string | null> {
  return invoke<string | null>("pick_document", { near });
}

/** The theme for a window without a document (the start screen). */
export function startTheme(): Promise<ResolvedTheme> {
  return invoke<ResolvedTheme>("start_theme");
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

/** Opens the system's print dialog for this window (DESIGN.md §10). */
export function printWindow(): Promise<void> {
  return invoke("print_window");
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

/** Sends the web features this webview lacks to the backend log. Never throws. */
export function reportWebview(missing: string[]): void {
  invoke("report_webview", { userAgent: navigator.userAgent, missing }).catch(() => {
    // Diagnostics only.
  });
}

/** Folders, templates, and defaults for the New document dialog (DESIGN.md §11). */
export function newDocumentOptions(): Promise<NewDocumentOptions> {
  return invoke<NewDocumentOptions>("new_document_options");
}

/** The system folder dialog, starting in `near`; null if cancelled. */
export function pickFolder(near: string | null): Promise<string | null> {
  return invoke<string | null>("pick_folder", { near });
}

/** Creates a new document and returns its path. Rejects with a message to show. */
export function createDocument(request: NewDocumentRequest): Promise<string> {
  return invoke<string>("create_document", { request });
}

export function rememberNewDocumentFolder(path: string): Promise<void> {
  return invoke<void>("remember_new_document_folder", { path });
}

export function forgetNewDocumentFolder(path: string): Promise<void> {
  return invoke<void>("forget_new_document_folder", { path });
}

/** Whether "Remember this folder" starts checked. */
export function rememberNewFolders(): Promise<boolean> {
  return invoke<boolean>("remember_new_folders");
}

export function setRememberNewFolders(remember: boolean): Promise<void> {
  return invoke<void>("set_remember_new_folders", { remember });
}

/** Opens the templates folder in the system file manager. */
export function openTemplatesFolder(): Promise<void> {
  return invoke<void>("open_templates_folder");
}

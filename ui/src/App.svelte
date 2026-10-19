<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import {
    launchInfo,
    onDocumentChanged,
    openDocument,
    openExternal,
    reportTiming,
    resolveLink,
    saveReadingPosition,
    setRemoteImages,
    setDocumentFlavor,
    listThemes,
    themeStyle,
    setDocumentTheme,
    setDefaultTheme,
    duplicateTheme,
    pickTerminalTheme,
    themeDetails,
    previewThemeEdit,
    themeContrast,
    saveThemeEdit,
    systemFonts,
    importTerminalTheme,
    userThemeFolder,
    onThemesChanged,
    propertiesOpen as propertiesOpenSetting,
    recentDocuments,
    setDocumentPinned,
    forgetDocument,
    pickDocument,
    newDocumentOptions,
    pickFolder,
    createDocument,
    openTemplatesFolder,
    rememberNewDocumentFolder,
    forgetNewDocumentFolder,
    setRememberNewFolders,
    startTheme,
    setPropertiesOpen,
    propertiesPlacement as propertiesPlacementSetting,
    setPropertiesPlacement,
    reparseDocument,
    editFrontMatter,
    saveDocument,
    checkDisk,
    diskText,
    fileText,
    keepLocalVersion,
    loadDiskVersion,
    recoveredChanges,
    restoreRecovered,
    discardRecovered,
    recoveredText,
    autosave as autosaveSetting,
    setAutosave,
    onAutosaveChanged,
    defaultTheme as defaultThemeSetting,
    contentsDepth as contentsDepthSetting,
    typographyDefaults,
    setContentsDepth,
    onSettingsChanged,
    type SettingName,
    pickAssetsFolder,
    blockSource,
    editBlock,
    stepHistory,
    documentText,
    setDocumentText,
  } from "./lib/commands";
  import { afterPaint } from "./lib/idle";
  import { launchMessage } from "./lib/launch";
  import type {
    DocumentModel,
    RecentDocument,
    Flavor,
    OpenedDocument,
    PropertiesPlacement,
    PropertyChange,
    RecoveredChanges,
    ResolvedTheme,
    TerminalThemeFile,
    ThemeChange,
    ThemeDetails,
    ThemeEditorValues,
    ContrastProblem,
    ThemeSummary,
    NewDocumentOptions,
    NewDocumentRequest,
  } from "./lib/types";
  import SchemeChooser from "./themes/SchemeChooser.svelte";
  import ThemeEditor from "./themes/ThemeEditor.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import UnsavedDialog, { type UnsavedChoice } from "./editor/UnsavedDialog.svelte";
  import type { PropertyEditor } from "./frontmatter/properties";
  import type { BlockEditing } from "./editor/blocks";
  import RawEditor from "./editor/RawEditor.svelte";
  import { applyTheme } from "./themes/apply";
  import { refreshRichForTheme } from "./render/renderers";
  import ThemeSwitcher from "./themes/ThemeSwitcher.svelte";
  import { NavHistory } from "./nav/history";
  import { TOP_ANCHOR, type ScrollAnchor } from "./reader/anchor";
  import Reader from "./reader/Reader.svelte";
  import ConflictBanner from "./editor/ConflictBanner.svelte";
  import RecoveryBanner from "./editor/RecoveryBanner.svelte";
  import RecoveredList from "./editor/RecoveredList.svelte";
  import StartScreen from "./start/StartScreen.svelte";
  import { isMarkdownPath } from "./start/recent";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { AUTOSAVE_DELAY_MS, Autosaver } from "./editor/autosave";
  import TocSidebar from "./toc/TocSidebar.svelte";
  import PropertiesSidebar from "./sidebar/PropertiesSidebar.svelte";
  import SidebarTabs from "./sidebar/SidebarTabs.svelte";
  import { chooseTab, isShown, layout, toggle, type Panel, type SidebarState } from "./sidebar/layout";
  import StatusBar from "./status/StatusBar.svelte";
  import { TypographyController } from "./typography/controller.svelte";
  import TypographyPanel from "./typography/TypographyPanel.svelte";
  import SearchBar from "./search/SearchBar.svelte";
  import { SearchController } from "./search/controller.svelte";
  import ShortcutsDialog from "./shortcuts/ShortcutsDialog.svelte";
  import { opensSettings, opensShortcuts, togglesPanel } from "./shortcuts/shortcuts";
  import SettingsDialog from "./settings/SettingsDialog.svelte";
  import NewDocumentDialog from "./newdoc/NewDocumentDialog.svelte";
  import { readingProgress, wordsBefore, type ReadingPosition } from "./status/progress";

  let doc = $state<DocumentModel | null>(null);
  let assetToken = $state(0);
  let initialAnchor = $state<ScrollAnchor>(TOP_ANCHOR);
  let message = $state<string | null>(null);
  let notice = $state<string | null>(null);
  // The left sidebar: Contents, or its Properties tab (DESIGN.md §8.2, §10).
  let tocVisible = $state(true);
  // The TOC entry for the reading position, reported by the reader.
  let currentEntry = $state<number | null>(null);
  let reader: Reader | undefined = $state();
  const search = new SearchController(() => reader);
  let searchBar: SearchBar | undefined = $state();
  let shortcutsOpen = $state(false);
  let settingsOpen = $state(false);
  // The New document dialog's folders and templates while it's open (DESIGN.md §11).
  let newDocument = $state<NewDocumentOptions | null>(null);
  // Remembered folders and their setting, for the settings dialog.
  let newDocumentSettings = $state<NewDocumentOptions | null>(null);
  // Global settings shown in the settings dialog (the typography defaults
  // live in `typography`, autosave in `autosave`).
  let defaultThemeId = $state<string | null>(null);
  let contentsDepth = $state(6);
  // Reading progress (DESIGN.md §10), in words above the top of the screen.
  let readingPosition = $state<ReadingPosition>({ blockIndex: 0, fraction: 0, atEnd: false });
  const blockWordsBefore = $derived(doc ? wordsBefore(doc.blocks) : [0]);
  const progress = $derived(readingProgress(blockWordsBefore, readingPosition));
  let canGoBack = $state(false);
  let canGoForward = $state(false);

  // The document's theme, and what's on screen (they differ while previewing).
  let theme = $state<ResolvedTheme | null>(null);
  let themeName = $state<string | null>(null);
  let numbering = $state(false);
  let switcherOpen = $state(false);
  let themeList = $state<ThemeSummary[]>([]);
  let themeFolder = $state<string | null>(null);

  let mainElement: HTMLElement | undefined = $state();
  let typographyOpen = $state(false);
  const typography = new TypographyController({
    documentPath: () => doc?.path ?? null,
    // Before the reader exists, estimate from the main area minus the reader's padding.
    textAreaWidth: () => reader?.textAreaWidth() || (mainElement?.clientWidth ?? 800) - 48,
    viewportTop: () => reader?.viewportTop() ?? 0,
    captureAnchor: (offsetY) => reader?.captureAnchor(offsetY) ?? null,
    stableAnchor: () => reader?.stableAnchor() ?? null,
    previewScale: (scale, offsetY, ms) => reader?.previewScale(scale, offsetY, ms) ?? Promise.resolve(),
    clearPreview: () => reader?.clearPreview(),
    restoreAnchor: (anchor, offsetY) => reader?.scrollToAnchor(anchor, offsetY) ?? Promise.resolve(),
  });

  // Fit mode follows the width of the reading area (window resizes, TOC
  // toggles). The <main> element only exists once a document is open.
  $effect(() => {
    const element = mainElement;
    if (!element) return;
    const observer = new ResizeObserver(() => typography.onResize());
    observer.observe(element);
    return () => observer.disconnect();
  });

  const history = new NavHistory();
  // A heading slug to scroll to once a newly opened document is on screen.
  let pendingFragment: string | null = null;
  // When the open started, for the first-screen and full-mount timings.
  let openStarted = 0;
  let noticeTimer = 0;

  const sectionTitle = $derived(currentEntry === null ? null : (doc?.toc[currentEntry]?.text ?? null));

  // Whether the properties panel is open: a global preference (DESIGN.md §8.2).
  // In a sidebar, it's whether the properties are shown.
  let propertiesOpen = $state(true);
  let propertiesPlacement = $state<PropertiesPlacement>("top");
  // The sidebar opened last, which a narrow window keeps when both don't fit.
  let lastPanel = $state<Panel>("contents");
  // Wide enough for Contents and properties side by side with the reading column.
  const wideQuery = window.matchMedia("(min-width: 64rem)");
  let wide = $state(wideQuery.matches);
  const sidebarState = $derived<SidebarState>({ leftOpen: tocVisible, propertiesOpen, last: lastPanel });
  const shown = $derived(layout(propertiesPlacement, sidebarState, wide));

  function setSidebars(next: SidebarState): void {
    tocVisible = next.leftOpen;
    lastPanel = next.last;
    if (next.propertiesOpen !== propertiesOpen) togglePropertiesPanel(next.propertiesOpen);
  }

  function togglePanel(panel: Panel): void {
    setSidebars(toggle(panel, propertiesPlacement, sidebarState, wide));
  }

  /** Moves the properties panel, in every window, and shows it in this one. */
  async function placeProperties(placement: PropertiesPlacement): Promise<void> {
    await changeSetting("where the properties go", () => setPropertiesPlacement(placement));
    propertiesPlacement = placement;
    if (!isShown("properties", placement, sidebarState, wide)) {
      setSidebars(toggle("properties", placement, sidebarState, wide));
    }
  }

  // Unsaved edits (DESIGN.md §9.3): the backend keeps the edited text; the
  // file changes only on Ctrl+S.
  let dirty = $state(false);
  // The open "save changes?" question, answered through `resolve`.
  let unsavedPrompt = $state<{ title: string; action: string; resolve: (choice: UnsavedChoice) => void } | null>(
    null,
  );

  function documentTitle(d: DocumentModel): string {
    return d.frontMatter?.title ?? d.path.split(/[\\/]/).pop() ?? d.path;
  }

  /** Asks the user what to do with unsaved changes. */
  function askUnsaved(action: string): Promise<UnsavedChoice> {
    if (!doc) return Promise.resolve("discard");
    const title = documentTitle(doc);
    return new Promise((resolve) => {
      unsavedPrompt = {
        title,
        action,
        resolve: (choice) => {
          unsavedPrompt = null;
          resolve(choice);
        },
      };
    });
  }

  /**
   * Before leaving the document (opening another, going back, closing):
   * true to go ahead. With unsaved changes, asks first, saving if chosen.
   */
  async function confirmLeave(action: string): Promise<boolean> {
    if (rawMode) await syncRaw();
    await pendingEdits;
    if (!dirty) return true;
    // With autosave on, leaving saves; if that fails (say, the file changed
    // on disk), ask as usual.
    if (autosave && (await save())) return true;
    const choice = await askUnsaved(action);
    if (choice === "cancel") return false;
    if (choice === "save") return save();
    return true;
  }

  // Edits in flight, in order: each waits for the one before, and saving
  // waits for all of them, so an edit committed by Ctrl+S itself is saved.
  let pendingEdits: Promise<unknown> = Promise.resolve();

  function queueEdit(run: () => Promise<void>): Promise<void> {
    const next = pendingEdits.then(run, run);
    pendingEdits = next.catch(() => {});
    return next;
  }

  // Raw mode (DESIGN.md §9.2): the whole file in CodeMirror. Its text is
  // sent to the app shortly after typing stops; the reader re-parses when
  // raw mode closes.
  let rawMode = $state(false);
  let raw = $state<{
    text: string;
    version: number;
    startByte: number;
    resetKey: number;
    /** The disk version shown beside the text (compare view), or null. */
    compareWith: string | null;
    /** Changes when the editor must be built again (compare view on or off). */
    mount: number;
  } | null>(null);
  let rawEditor: RawEditor | undefined = $state();
  let rawPending: string | null = null;
  let rawTimer = 0;
  const RAW_SYNC_MS = 400;

  function onRawChange(): void {
    rawPending = "changed";
    noteEdit();
    window.clearTimeout(rawTimer);
    rawTimer = window.setTimeout(() => void syncRaw(), RAW_SYNC_MS);
  }

  /** Sends raw mode's latest text to the app (queued like any edit). */
  function syncRaw(): Promise<void> {
    window.clearTimeout(rawTimer);
    return queueEdit(async () => {
      if (rawPending === null || !raw) return;
      const text = rawEditor?.currentText() ?? null;
      if (text === null) return;
      rawPending = null;
      try {
        const result = await setDocumentText(raw.version, text);
        raw.version = result.version;
        dirty = result.dirty;
      } catch (e) {
        showNotice(`Could not apply the raw edit: ${String(e)}`);
      }
    });
  }

  /**
   * Opens raw mode at the reading position, or at `atOffset` (a byte offset
   * in the file, as core's ranges are) when given.
   */
  async function enterRawMode(atOffset?: number): Promise<void> {
    if (!doc || rawMode) return;
    // Raw mode has its own search (CodeMirror's, on the same keys).
    search.close();
    await pendingEdits;
    const offset = atOffset ?? (reader?.captureAnchor() ?? TOP_ANCHOR).offset;
    try {
      const current = await documentText();
      const bom = doc.hasBom ? 3 : 0;
      raw = { ...current, startByte: Math.max(0, offset - bom), resetKey: 0, compareWith: null, mount: 0 };
      rawMode = true;
      showNotice("Raw mode: Ctrl+E returns to reading");
    } catch (e) {
      showNotice(`Could not open raw mode: ${String(e)}`);
    }
  }

  async function exitRawMode(): Promise<void> {
    if (!doc || !rawMode) return;
    await syncRaw();
    await pendingEdits;
    const bom = doc.hasBom ? 3 : 0;
    const top = rawEditor?.topByte() ?? 0;
    rawMode = false;
    raw = null;
    comparing = false;
    if (recoveryComparing) {
      // Leaving the merge view keeps the text on the right, as Done does.
      recoveryComparing = false;
      recovered = null;
    }
    try {
      const opened = await reparseDocument();
      await applyOpened(opened, { offset: top + bom, fraction: 0 });
    } catch (e) {
      showNotice(`Could not show the document: ${String(e)}`);
    }
  }

  // True while an edit's result is being shown: the reader updates in place.
  let keepPosition = $state(false);

  /** Shows the result of an edit, undo, or redo without moving the reader. */
  async function applyEdit(opened: OpenedDocument): Promise<void> {
    keepPosition = true;
    try {
      await applyOpened(opened, reader?.captureAnchor() ?? TOP_ANCHOR);
    } finally {
      keepPosition = false;
    }
  }

  const blockEditing: BlockEditing = {
    load: (block) => blockSource(block.source.start, block.source.end),
    commit: (block, original, text) =>
      queueEdit(async () => {
        try {
          const started = performance.now();
          await applyEdit(await editBlock(block.source.start, block.source.end, original, text));
          noteEdit();
          // DESIGN.md §4 budget: re-render after a single-block edit < 100 ms.
          void afterPaint().then(() => reportTiming("block_edit", performance.now() - started));
        } catch (e) {
          showNotice(`Could not apply the edit: ${String(e)}`);
        }
      }),
  };

  /** Undo (or redo) one edit to the document. */
  function stepEdits(redo: boolean): Promise<void> {
    return queueEdit(async () => {
      try {
        const opened = await stepHistory(redo);
        if (opened) {
          await applyEdit(opened);
          noteEdit();
        }
        else showNotice(redo ? "Nothing to redo" : "Nothing to undo");
      } catch (e) {
        showNotice(`Could not ${redo ? "redo" : "undo"}: ${String(e)}`);
      }
    });
  }

  /** Saves the document; false (with a notice) if that failed. */
  async function save(savedNotice = "Saved"): Promise<boolean> {
    if (!doc) return false;
    if (rawMode) await syncRaw();
    await pendingEdits;
    try {
      if ((await saveDocument()) === "conflict") {
        conflict = "changed";
        showNotice("Not saved: the file changed on disk. Choose what to keep first.");
        return false;
      }
      dirty = false;
      showNotice(savedNotice);
      return true;
    } catch (e) {
      showNotice(`Could not save: ${String(e)}`);
      return false;
    }
  }

  /** Changes a front matter property (in memory) and shows the result. */
  function editProperty(key: string, change: PropertyChange): Promise<void> {
    return queueEdit(async () => {
      try {
        await applyEdit(await editFrontMatter(key, change));
        noteEdit();
      } catch (e) {
        showNotice(`Could not change ${key}: ${String(e)}`);
      }
    });
  }

  const propertyEditor = $derived<PropertyEditor | null>(
    doc === null
      ? null
      : {
          themes: themeList,
          edit: editProperty,
          pickFolder: () => (doc ? pickAssetsFolder(doc.path) : Promise.resolve(null)),
        },
  );
  // Raw mode holds the text being typed, so a sidebar shows the properties
  // read-only there: the front matter is edited in the text instead.
  const sidebarEditor = $derived(rawMode ? null : propertyEditor);

  // External changes while there are unsaved edits (DESIGN.md §9.3): the
  // banner offers to keep mine, load the disk version, or compare.
  let conflict = $state<"changed" | "missing" | null>(null);
  let comparing = $state(false);
  let conflictBusy = $state(false);

  /**
   * The document changed on disk. Scrald's own saves (and touches that
   * change nothing) are ignored; with unsaved edits the file isn't reloaded,
   * so nothing is lost, and the conflict banner asks what to keep.
   */
  async function onDiskChanged(): Promise<void> {
    if (rawMode) await syncRaw();
    await pendingEdits;
    const state = await checkDisk().catch(() => "changed" as const);
    if (state === "unchanged") {
      // Changed back (or to the version already kept): nothing to decide.
      if (conflict) await endConflict();
      return;
    }
    if (dirty || conflict) {
      // The conflict banner takes over; the restored changes stay.
      if (recoveryComparing) await finishRecoveryCompare();
      conflict = state;
      // Changed again while comparing: compare with the new version.
      if (comparing) {
        if (state === "changed") await startCompare();
        else await stopComparing();
      }
      return;
    }
    if (state === "changed") {
      await reload();
      if (rawMode && raw) {
        const current = await documentText();
        raw = { ...raw, ...current, startByte: rawEditor?.topByte() ?? 0, resetKey: raw.resetKey + 1 };
      }
    }
  }

  /** Closes the banner (and the compare view, staying in raw mode). */
  async function endConflict(): Promise<void> {
    conflict = null;
    if (comparing) await stopComparing();
  }

  async function keepMine(): Promise<void> {
    conflictBusy = true;
    try {
      if (rawMode) await syncRaw();
      await keepLocalVersion();
      const wasChanged = conflict === "changed";
      await endConflict();
      if (wasChanged) showNotice("Keeping your version: Ctrl+S replaces the file on disk");
    } catch (e) {
      showNotice(`Could not keep your version: ${String(e)}`);
    } finally {
      conflictBusy = false;
    }
  }

  async function loadDisk(): Promise<void> {
    conflictBusy = true;
    if (rawMode) await syncRaw();
    await queueEdit(async () => {
      try {
        const opened = await loadDiskVersion();
        conflict = null;
        if (rawMode && raw) {
          const current = await documentText();
          raw = {
            ...current,
            startByte: rawEditor?.topByte() ?? 0,
            resetKey: 0,
            compareWith: null,
            mount: raw.mount + 1,
          };
        }
        comparing = false;
        await applyEdit(opened);
        // In raw mode, Ctrl+Z is the editor's own undo, which starts afresh.
        showNotice(
          rawMode
            ? "Loaded the disk version. After Ctrl+E, Ctrl+Z brings your changes back."
            : "Loaded the disk version. Ctrl+Z brings your changes back.",
        );
      } catch (e) {
        showNotice(`Could not load the disk version: ${String(e)}`);
      }
    });
    conflictBusy = false;
  }

  /** Shows the disk version beside the document's text, in raw mode. */
  async function startCompare(): Promise<void> {
    if (!doc) return;
    conflictBusy = true;
    try {
      if (rawMode) await syncRaw();
      await pendingEdits;
      const disk = await diskText();
      const current = await documentText();
      const bom = doc.hasBom ? 3 : 0;
      const startByte =
        rawMode && rawEditor ? rawEditor.topByte() : Math.max(0, (reader?.captureAnchor() ?? TOP_ANCHOR).offset - bom);
      raw = { ...current, startByte, resetKey: 0, compareWith: disk, mount: (raw?.mount ?? 0) + 1 };
      rawMode = true;
      comparing = true;
    } catch (e) {
      showNotice(`Could not compare: ${String(e)}`);
    } finally {
      conflictBusy = false;
    }
  }

  /** Back from the compare view to plain raw mode. */
  async function stopComparing(): Promise<void> {
    comparing = false;
    if (!rawMode || !raw) return;
    await syncRaw();
    const current = await documentText();
    raw = { ...current, startByte: rawEditor?.topByte() ?? 0, resetKey: 0, compareWith: null, mount: raw.mount + 1 };
  }

  // Autosave (DESIGN.md §9.3): off by default; when on, the document saves
  // itself once editing pauses. Only the user's own edits start the wait;
  // loading the disk version or restoring recovered changes doesn't.
  let autosave = $state(false);
  const autosaver = new Autosaver(AUTOSAVE_DELAY_MS, autosaveNow);

  /** Called after each edit by the user. */
  function noteEdit(): void {
    if (autosave) autosaver.touch();
  }

  async function autosaveNow(): Promise<void> {
    // A conflict or a comparison of recovered changes waits for the user's
    // decision; a "save changes?" prompt is already handling the changes.
    if (!autosave || !doc || conflict || recoveryComparing || unsavedPrompt) return;
    if (rawMode) await syncRaw();
    await pendingEdits;
    if (dirty) await save("Autosaved");
  }

  async function changeAutosave(enabled: boolean): Promise<void> {
    try {
      await setAutosave(enabled);
      applyAutosave(enabled);
    } catch (e) {
      showNotice(`Could not change autosave: ${String(e)}`);
    }
  }

  function applyAutosave(enabled: boolean): void {
    autosave = enabled;
    if (!enabled) autosaver.cancel();
    else if (dirty) autosaver.touch();
  }

  // Crash recovery (DESIGN.md §9.3): unsaved changes Scrald found from a
  // run that stopped before they were saved. Offered for the open document,
  // or listed when Scrald starts without one.
  let recovered = $state<RecoveredChanges | null>(null);
  let recoveredList = $state<RecoveredChanges[]>([]);
  let recoveryBusy = $state(false);

  /** Replaces the document's text with recovered changes (undoable). */
  async function restoreRecovery(id: string): Promise<void> {
    recoveryBusy = true;
    if (rawMode) await syncRaw();
    await queueEdit(async () => {
      try {
        const opened = await restoreRecovered(id);
        recovered = null;
        if (rawMode && raw) {
          const current = await documentText();
          raw = { ...raw, ...current, startByte: rawEditor?.topByte() ?? 0, resetKey: raw.resetKey + 1 };
        }
        await applyEdit(opened);
        showNotice(
          rawMode
            ? "Restored your unsaved changes. Ctrl+S saves them."
            : "Restored your unsaved changes. Ctrl+S saves them; Ctrl+Z goes back to the file.",
        );
      } catch (e) {
        showNotice(`Could not restore the changes: ${String(e)}`);
      }
    });
    recoveryBusy = false;
  }

  /** Deletes recovered changes; the banner then offers any others for this document. */
  async function discardRecovery(id: string): Promise<void> {
    recoveryBusy = true;
    try {
      await discardRecovered(id);
      const others = await recoveredChanges();
      recoveredList = doc === null ? others : [];
      recovered = doc === null ? null : (others.find((r) => r.path === doc?.path) ?? null);
    } catch (e) {
      showNotice(`Could not discard the changes: ${String(e)}`);
    } finally {
      recoveryBusy = false;
    }
  }

  // Comparing recovered changes with the file: they're restored (an undoable
  // edit, as with Restore) and shown in raw mode's merge view, with the file
  // on the left as it is on disk now.
  let recoveryComparing = $state(false);
  let recoveryFileText = "";

  async function compareRecovery(id: string): Promise<void> {
    if (!doc) return;
    recoveryBusy = true;
    try {
      if (rawMode) await syncRaw();
      await pendingEdits;
      const file = await fileText();
      const bom = doc.hasBom ? 3 : 0;
      const startByte =
        rawMode && rawEditor ? rawEditor.topByte() : Math.max(0, (reader?.captureAnchor() ?? TOP_ANCHOR).offset - bom);
      await queueEdit(async () => {
        await applyEdit(await restoreRecovered(id));
      });
      const current = await documentText();
      recoveryFileText = file;
      raw = { ...current, startByte, resetKey: 0, compareWith: file, mount: (raw?.mount ?? 0) + 1 };
      rawMode = true;
      comparing = true;
      recoveryComparing = true;
    } catch (e) {
      showNotice(`Could not compare the changes: ${String(e)}`);
    } finally {
      recoveryBusy = false;
    }
  }

  /** "Done": the text on the right stays as unsaved changes, in raw mode. */
  async function finishRecoveryCompare(): Promise<void> {
    recoveryComparing = false;
    recovered = null;
    await stopComparing();
  }

  async function keepComparedRecovery(): Promise<void> {
    recoveryBusy = true;
    try {
      await finishRecoveryCompare();
      // Like an edit of the user's own: autosave may save it now.
      noteEdit();
      showNotice("Kept the recovered changes as unsaved changes. Ctrl+S saves them.");
    } finally {
      recoveryBusy = false;
    }
  }

  /** "Use the file": back to the file's text, as an undoable edit. */
  async function useFileOverRecovery(): Promise<void> {
    recoveryBusy = true;
    try {
      await syncRaw();
      await queueEdit(async () => {
        if (!raw) return;
        const result = await setDocumentText(raw.version, recoveryFileText);
        raw.version = result.version;
        dirty = result.dirty;
      });
      await finishRecoveryCompare();
      showNotice("Back to the file's text. After Ctrl+E, Ctrl+Z brings the recovered changes back.");
    } catch (e) {
      showNotice(`Could not go back to the file: ${String(e)}`);
    } finally {
      recoveryBusy = false;
    }
  }

  /** From the list shown without a document: open it, then restore. */
  async function restoreFromList(item: RecoveredChanges): Promise<void> {
    recoveryBusy = true;
    try {
      await navigate(item.path);
    } finally {
      recoveryBusy = false;
    }
    if (doc) await restoreRecovery(item.id);
  }

  /** Copies recovered text (for changes whose file is gone). */
  async function copyRecovered(id: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(await recoveredText(id));
      message = "Copied the recovered text.";
    } catch (e) {
      message = `Could not copy the text: ${String(e)}`;
    }
  }

  function togglePropertiesPanel(open: boolean): void {
    propertiesOpen = open;
    void setPropertiesOpen(open).catch(() => {});
  }

  // Settings (DESIGN.md §10). Each change is stored at once; the app tells
  // every window (this one too), and `applySetting` brings each up to date.
  async function openSettings(): Promise<void> {
    settingsOpen = true;
    try {
      [themeList, defaultThemeId, newDocumentSettings] = await Promise.all([
        listThemes(),
        defaultThemeSetting(),
        newDocumentOptions(),
      ]);
    } catch (e) {
      showNotice(`Could not read the settings: ${String(e)}`);
    }
  }

  async function changeSetting(name: string, change: () => Promise<void>): Promise<void> {
    try {
      await change();
    } catch (e) {
      showNotice(`Could not change ${name}: ${String(e)}`);
    }
  }

  async function applySetting(name: SettingName): Promise<void> {
    try {
      if (name === "typography") {
        await typography.applyDefaults(await typographyDefaults());
      } else if (name === "contents.depth") {
        contentsDepth = await contentsDepthSetting();
      } else if (name === "properties.placement") {
        propertiesPlacement = await propertiesPlacementSetting();
      } else if (name === "theme.default") {
        defaultThemeId = await defaultThemeSetting();
        await followDefaultTheme();
      }
    } catch (e) {
      console.warn(`could not apply the ${name} setting`, e);
    }
  }

  /** The default theme changed: a window showing it switches to the new one. */
  async function followDefaultTheme(): Promise<void> {
    if (!doc) {
      await showStartScreen();
      return;
    }
    if (theme?.source !== "default") return;
    // The default level alone resolves exactly as on the start screen.
    const resolved = await startTheme();
    if (resolved.id === theme.id) return;
    theme = { id: resolved.id, source: "default" };
    if (!switcherOpen && !themeEditing) await showTheme(resolved.id);
  }

  onMount(() => {
    void refreshThemeList().catch(() => {});
    // Closing with unsaved changes asks first (DESIGN.md §9.3).
    const unlistenClose = getCurrentWindow().onCloseRequested(async (event) => {
      if (!dirty) return;
      event.preventDefault();
      if (await confirmLeave("closing")) {
        dirty = false;
        await getCurrentWindow().destroy();
      }
    });
    void propertiesOpenSetting()
      .then((open) => (propertiesOpen = open))
      .catch(() => {});
    void propertiesPlacementSetting()
      .then((placement) => (propertiesPlacement = placement))
      .catch(() => {});
    const onWideChange = (event: MediaQueryListEvent) => (wide = event.matches);
    wideQuery.addEventListener("change", onWideChange);
    void autosaveSetting()
      .then(applyAutosave)
      .catch(() => {});
    const unlistenAutosave = onAutosaveChanged(applyAutosave);
    void contentsDepthSetting()
      .then((depth) => (contentsDepth = depth))
      .catch(() => {});
    const unlistenSettings = onSettingsChanged((name) => void applySetting(name));
    // Drag and drop (DESIGN.md §11): Tauri hands over the dropped files'
    // paths; the page itself never sees the files.
    const unlistenDrop = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") dropping = true;
      else if (payload.type === "leave") dropping = false;
      else {
        dropping = false;
        void openDropped(payload.paths);
      }
    });
    void typography.loadDefaults().then(start);
    // Live reload (DESIGN.md §9.3), unless it's our own save or there are
    // unsaved edits.
    const unlisten = onDocumentChanged((path) => {
      if (doc && path === doc.path) void onDiskChanged();
    });
    // Theme hot reload (DESIGN.md §7.1): re-apply when a user theme changes.
    const unlistenThemes = onThemesChanged(() => {
      if (theme && !switcherOpen && !themeEditing) void showTheme(theme.id);
      if (switcherOpen) void refreshThemeList();
      if (!doc) void showStartScreen();
    });
    // Other windows open, close and recover documents meanwhile, so the start
    // screen re-reads its lists whenever its window comes to the front.
    const unlistenFocus = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused && !doc) void refreshStartLists();
    });
    // Ctrl+wheel and trackpad pinch zoom the text, not the whole webview.
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey || !doc) return;
      event.preventDefault();
      typography.wheel(event);
    };
    window.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      void unlisten.then((stop) => stop());
      void unlistenClose.then((stop) => stop());
      void unlistenThemes.then((stop) => stop());
      void unlistenAutosave.then((stop) => stop());
      void unlistenSettings.then((stop) => stop());
      wideQuery.removeEventListener("change", onWideChange);
      void unlistenDrop.then((stop) => stop());
      void unlistenFocus.then((stop) => stop());
      window.removeEventListener("wheel", onWheel);
    };
  });

  // The start screen (DESIGN.md §11), shown while no document is open.
  const RECENT_LIMIT = 50;
  let recentList = $state<RecentDocument[]>([]);
  let dropping = $state(false);

  /** Fills the start screen: its theme, recent documents, recovered changes. */
  async function showStartScreen(): Promise<void> {
    try {
      const [resolved, recent, themes, recoveredNow] = await Promise.all([
        startTheme(),
        recentDocuments(RECENT_LIMIT),
        listThemes(),
        recoveredChanges().catch(() => []),
      ]);
      await showTheme(resolved.id, false);
      recentList = recent;
      themeList = themes;
      recoveredList = recoveredNow;
    } catch (e) {
      message = `Could not list recent documents: ${String(e)}`;
    }
  }

  /** Re-reads the start screen's recent documents and recovered changes. */
  async function refreshStartLists(): Promise<void> {
    try {
      const [recent, recoveredNow] = await Promise.all([
        recentDocuments(RECENT_LIMIT),
        recoveredChanges().catch(() => []),
      ]);
      // A document may have opened while the lists were read.
      if (doc) return;
      recentList = recent;
      recoveredList = recoveredNow;
    } catch {
      // The lists shown stay as they were.
    }
  }

  /** "Open…" (Ctrl+O): a file dialog, starting in the open document's folder. */
  // New document (DESIGN.md §11).
  async function openNewDocument(): Promise<void> {
    try {
      newDocument = await newDocumentOptions();
    } catch (e) {
      report(`Could not start a new document: ${String(e)}`);
    }
  }

  /**
   * Creates the document and opens it in raw mode, ready to type. Asks about
   * unsaved changes first, so cancelling that never leaves a new file
   * behind; if creating fails, the dialog stays open with the reason.
   */
  async function createNewDocument(request: NewDocumentRequest): Promise<void> {
    if (!(await confirmLeave("creating a new document"))) return;
    const path = await createDocument(request);
    newDocument = null;
    await navigate(path, null, true);
    // Past the end: the cursor goes after the starting text.
    if (doc) await enterRawMode(Number.MAX_SAFE_INTEGER);
  }

  /** Remembered folders, from the settings (DESIGN.md §11). */
  async function changeNewDocumentFolders(change: () => Promise<unknown>): Promise<void> {
    try {
      await change();
      newDocumentSettings = await newDocumentOptions();
    } catch (e) {
      showNotice(`Could not change the folders: ${String(e)}`);
    }
  }

  async function addNewDocumentFolder(): Promise<void> {
    const picked = await pickFolder(newDocumentSettings?.defaultFolder ?? null);
    if (picked) await changeNewDocumentFolders(() => rememberNewDocumentFolder(picked));
  }

  async function pickAndOpen(): Promise<void> {
    try {
      const path = await pickDocument(doc?.path ?? null);
      if (path) await navigate(path);
    } catch (e) {
      report(`Could not open: ${String(e)}`);
    }
  }

  /** Opens the first Markdown file among dropped files. */
  async function openDropped(paths: string[]): Promise<void> {
    const path = paths.find(isMarkdownPath);
    if (path) await navigate(path);
    else if (paths.length > 0) report("Scrald opens Markdown files (.md, .markdown, .mdown, .txt).");
  }

  async function openRecent(item: RecentDocument): Promise<void> {
    await navigate(item.path);
    // Opening can fail (say, the file just went away); show the list as it is now.
    if (!doc) await showStartScreen();
  }

  async function pinRecent(item: RecentDocument, pinned: boolean): Promise<void> {
    try {
      await setDocumentPinned(item.path, pinned);
      recentList = await recentDocuments(RECENT_LIMIT);
    } catch (e) {
      message = `Could not change the pin: ${String(e)}`;
    }
  }

  async function forgetRecent(item: RecentDocument): Promise<void> {
    try {
      await forgetDocument(item.path);
      recentList = await recentDocuments(RECENT_LIMIT);
    } catch (e) {
      message = `Could not remove it from the list: ${String(e)}`;
    }
  }

  /** A notice in the status bar, or the start screen's message line. */
  function report(text: string): void {
    if (doc) showNotice(text);
    else message = text;
  }

  async function start(): Promise<void> {
    try {
      const info = await launchInfo();
      if (info.path === null || !info.exists) {
        message = launchMessage(info);
        await showStartScreen();
        return;
      }
      await navigate(info.path);
    } catch (e) {
      message = `Could not reach the Scrald backend: ${String(e)}`;
    }
  }

  /**
   * Loads a document into the reader at `anchor`, or where the reader left
   * off last time if `anchor` is "remembered". Returns false (and shows why)
   * if it couldn't be opened; the current document stays on screen then.
   */
  async function load(path: string, anchor: ScrollAnchor | "remembered"): Promise<boolean> {
    openStarted = performance.now();
    try {
      const opened = await openDocument(path);
      reportTiming("ipc_open_document", performance.now() - openStarted);
      autosaver.cancel();
      // Another document never inherits raw mode: the editor would still
      // hold the previous document's text. (Its pending typing was sent by
      // `confirmLeave`, which runs before every switch.)
      const another = doc !== null && opened.document.path !== doc.path;
      await applyOpened(opened, anchor);
      recovered = opened.recovered;
      conflict = null;
      recoveryComparing = false;
      if (another && rawMode) {
        rawMode = false;
        raw = null;
        comparing = false;
      }
      if (comparing) {
        comparing = false;
        rawMode = false;
        raw = null;
      }
      message = null;
      return true;
    } catch (e) {
      if (doc) showNotice(`Could not open ${path}: ${String(e)}`);
      else message = `Could not open ${path}: ${String(e)}`;
      return false;
    }
  }

  /** Shows an opened (or re-parsed, or edited) document. */
  async function applyOpened(opened: OpenedDocument, anchor: ScrollAnchor | "remembered"): Promise<void> {
    // Apply the theme before the document renders, so it never flashes in
    // the previous document's theme.
    theme = opened.theme;
    typography.setDocument(opened.memory);
    // After an edit the theme is usually the one already shown; applying it
    // again would restyle the page and redraw every diagram.
    if (opened.theme.id !== shownThemeId) await showTheme(opened.theme.id, false);
    initialAnchor = anchor === "remembered" ? (opened.memory.anchor ?? TOP_ANCHOR) : anchor;
    assetToken = opened.assetToken;
    doc = opened.document;
    dirty = opened.dirty;
  }

  /** Re-parses the document from memory (keeping unsaved edits) in place. */
  async function reparse(): Promise<void> {
    if (!doc) return;
    const opened = await reparseDocument();
    await applyOpened(opened, reader?.captureAnchor() ?? TOP_ANCHOR);
  }

  /**
   * Opens a document as a new history entry: at `fragment` if given,
   * otherwise where the reader left off last time.
   */
  /** Opens `path` in this window; `confirmed` when unsaved changes were already asked about. */
  async function navigate(path: string, fragment: string | null = null, confirmed = false): Promise<void> {
    if (!confirmed && !(await confirmLeave("opening another document"))) return;
    rememberPosition();
    pendingFragment = fragment;
    if (await load(path, fragment === null ? "remembered" : TOP_ANCHOR)) {
      history.push({ path, anchor: TOP_ANCHOR });
      syncHistory();
    }
  }

  async function goBack(): Promise<void> {
    if (!history.canGoBack || !(await confirmLeave("going back"))) return;
    rememberPosition();
    const entry = history.back();
    if (entry) await load(entry.path, entry.anchor);
    syncHistory();
  }

  async function goForward(): Promise<void> {
    if (!history.canGoForward || !(await confirmLeave("going forward"))) return;
    rememberPosition();
    const entry = history.forward();
    if (entry) await load(entry.path, entry.anchor);
    syncHistory();
  }

  function syncHistory(): void {
    canGoBack = history.canGoBack;
    canGoForward = history.canGoForward;
  }

  /** Re-parses the current document in place, keeping the reading position. */
  async function reload(): Promise<void> {
    if (!doc) return;
    await load(doc.path, reader?.captureAnchor() ?? TOP_ANCHOR);
  }

  /** Saves the current reading position, in history and in the state database. */
  function rememberPosition(): void {
    if (!doc || !reader) return;
    const anchor = reader.captureAnchor();
    history.updateAnchor(anchor);
    saveReadingPosition(doc.path, anchor);
  }

  /** Re-parses the document in another flavor (null: back to automatic). */
  async function chooseFlavor(flavor: Flavor | null): Promise<void> {
    if (!doc) return;
    try {
      await setDocumentFlavor(doc.path, flavor);
      await reparse();
    } catch (e) {
      showNotice(`Could not change flavor: ${String(e)}`);
    }
  }

  /** Loads (or blocks again) the document's remote images, remembered for it (DESIGN.md §6.1). */
  async function setRemote(allowed: boolean): Promise<void> {
    if (!doc) return;
    try {
      await setRemoteImages(doc.path, allowed);
      await reparse();
    } catch (e) {
      showNotice(`Could not ${allowed ? "load" : "block"} remote images: ${String(e)}`);
    }
  }

  async function onLink(href: string): Promise<void> {
    if (!doc) return;
    const target = await resolveLink(doc.path, href);
    switch (target.kind) {
      case "external":
        try {
          await openExternal(target.url);
        } catch (e) {
          showNotice(`Could not open link: ${String(e)}`);
        }
        break;
      case "document":
        await navigate(target.path, target.fragment);
        break;
      case "fragment":
        await reader?.scrollToId(target.id);
        break;
      case "localFile":
        showNotice(`Scrald only opens Markdown files: ${target.path}`);
        break;
      case "unsupported":
        showNotice(`Unsupported link: ${target.href}`);
        break;
    }
  }

  function onFirstScreen(): void {
    reportTiming("first_screen", performance.now() - openStarted);
    const fragment = pendingFragment;
    pendingFragment = null;
    if (fragment !== null) void reader?.scrollToId(fragment);
  }

  // The theme whose styles are on the page.
  let shownThemeId: string | null = null;

  /**
   * Puts theme `id` on screen. With `keepPlace`, the reading position is
   * restored afterwards, since a new font or size reflows the text.
   */
  async function showTheme(id: string, keepPlace = true): Promise<void> {
    const started = performance.now();
    const anchor = keepPlace ? reader?.captureAnchor() : undefined;
    try {
      const style = await themeStyle(id);
      applyTheme(style);
      shownThemeId = id;
      // Diagrams and music carry the old theme's colors; redraw them.
      void refreshRichForTheme();
      themeName = style.name;
      numbering = style.numbering;
      typography.setTheme(style.layout);
    } catch (e) {
      showNotice(`Could not load theme ${id}: ${String(e)}`);
      return;
    }
    if (anchor) {
      await afterPaint();
      await reader?.scrollToAnchor(anchor);
      reportTiming("theme_switch", performance.now() - started);
    }
  }

  async function refreshThemeList(): Promise<void> {
    themeList = await listThemes();
  }

  async function openSwitcher(): Promise<void> {
    if (!doc || !theme) return;
    await refreshThemeList();
    themeFolder = await userThemeFolder();
    switcherOpen = true;
  }

  function closeSwitcher(): void {
    switcherOpen = false;
    // Next time, the switcher opens on the document's theme, not the import.
    importedTheme = null;
  }

  /** Keeps `id` as this document's theme. */
  async function chooseTheme(id: string): Promise<void> {
    if (!doc) return;
    closeSwitcher();
    try {
      await setDocumentTheme(doc.path, id);
      theme = { id, source: "document" };
      await showTheme(id);
    } catch (e) {
      showNotice(`Could not save theme: ${String(e)}`);
    }
  }

  async function makeDefaultTheme(id: string): Promise<void> {
    closeSwitcher();
    try {
      await setDefaultTheme(id);
      if (theme?.source === "default") theme = { id, source: "default" };
      showNotice(
        theme?.id === id
          ? `${themeName ?? id} is now the default theme`
          : `Default theme set; this document keeps its own theme`,
      );
      if (theme) await showTheme(theme.id);
    } catch (e) {
      showNotice(`Could not set default theme: ${String(e)}`);
    }
  }

  /** Forgets this document's own theme choice and re-resolves it. */
  async function resetTheme(): Promise<void> {
    if (!doc) return;
    closeSwitcher();
    await setDocumentTheme(doc.path, null);
    await reparse();
  }

  /**
   * "Save to document": writes `scrald-theme` into the front matter (an
   * unsaved edit, like any other) and clears the per-document choice, so the
   * document's own setting is the one in effect (DESIGN.md §7.6).
   */
  async function saveThemeToDocument(id: string): Promise<void> {
    closeSwitcher();
    await editProperty("scrald-theme", { kind: "text", value: id });
  }

  async function duplicate(id: string): Promise<void> {
    try {
      const newId = await duplicateTheme(id);
      await refreshThemeList();
      showNotice(`Created theme "${newId}"${themeFolder ? ` in ${themeFolder}` : ""}`);
    } catch (e) {
      showNotice(`Could not duplicate theme: ${String(e)}`);
    }
  }

  // Importing terminal themes (DESIGN.md §7.3): a file with several schemes
  // asks which ones; the first new theme is then selected and previewed.
  let schemeChoice = $state<TerminalThemeFile | null>(null);
  let importedTheme = $state<string | null>(null);

  async function importTerminal(): Promise<void> {
    try {
      const file = await pickTerminalTheme();
      if (!file) return;
      if (file.names.length === 1) await finishImport(file.path, file.names);
      else schemeChoice = file;
    } catch (e) {
      showNotice(`Could not import: ${String(e)}`);
    }
  }

  async function finishImport(path: string, names: string[]): Promise<void> {
    schemeChoice = null;
    try {
      const ids = await importTerminalTheme(path, names);
      await refreshThemeList();
      importedTheme = ids[0] ?? null;
      // Named after the new theme, which may be numbered ("Tango Dark 2").
      const first = themeList.find((t) => t.id === ids[0])?.name ?? names[0];
      const what = ids.length === 1 ? `"${first}"` : `${ids.length} themes`;
      showNotice(`Imported ${what}${themeFolder ? ` into ${themeFolder}` : ""}`);
    } catch (e) {
      showNotice(`Could not import: ${String(e)}`);
    }
  }

  // The theme editor (DESIGN.md §7.2): a panel beside the document. It
  // previews on the document as it goes; closing it puts the document's own
  // theme back (the saved version, if the edited theme is that one).
  let themeEditing = $state<{ details: ThemeDetails; fonts: string[] } | null>(null);
  // The theme editor takes the right edge while it's open.
  const showRight = $derived(shown.right && themeEditing === null);
  let installedFonts: string[] | null = null;

  async function editTheme(id: string): Promise<void> {
    try {
      const details = await themeDetails(id);
      installedFonts ??= await systemFonts().catch(() => []);
      closeSwitcher();
      themeEditing = { details, fonts: installedFonts };
    } catch (e) {
      showNotice(`Could not edit theme: ${String(e)}`);
    }
  }

  async function previewEdit(
    changes: ThemeChange[],
  ): Promise<{ values: ThemeEditorValues; contrast: ContrastProblem[] }> {
    if (!themeEditing) throw new Error("not editing");
    const preview = await previewThemeEdit(themeEditing.details.id, changes);
    applyTheme(preview.style);
    themeName = preview.style.name;
    numbering = preview.style.numbering;
    typography.setTheme(preview.style.layout);
    return { values: preview.values, contrast: preview.contrast };
  }

  async function saveEdit(changes: ThemeChange[]): Promise<void> {
    if (!themeEditing) return;
    const { id, values } = themeEditing.details;
    await saveThemeEdit(id, changes);
    themeEditing = null;
    await refreshThemeList();
    if (theme) await showTheme(theme.id);
    showNotice(`Saved theme "${themeList.find((t) => t.id === id)?.name ?? values.name}"`);
  }

  function cancelEdit(): void {
    themeEditing = null;
    if (theme) void showTheme(theme.id);
  }

  function cancelSwitcher(): void {
    closeSwitcher();
    if (theme) void showTheme(theme.id);
  }

  function showNotice(text: string): void {
    notice = text;
    window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = null), 6000);
  }

  /** `Ctrl+F`: opens the find bar, with the selected text if it's short and on one line. */
  function openSearch(): void {
    const selected = window.getSelection()?.toString().trim() ?? "";
    const text = selected !== "" && selected.length <= 200 && !selected.includes("\n") ? selected : null;
    search.show(text);
    void tick().then(() => searchBar?.focus());
  }

  // Search again when the reader shows a new model: after an edit (same
  // path), keeping the current match; for another document, from the top.
  let searchedPath: string | null = null;
  $effect(() => {
    const current = doc;
    untrack(() => {
      if (!current) {
        search.close();
      } else {
        search.documentChanged(searchedPath !== null && searchedPath !== current.path);
      }
      searchedPath = current?.path ?? null;
    });
  });

  function onKeydown(event: KeyboardEvent): void {
    if (switcherOpen || unsavedPrompt) return;
    if (opensShortcuts(event, (event.target as HTMLElement | null)?.closest(".cm-editor") != null)) {
      event.preventDefault();
      settingsOpen = false;
      shortcutsOpen = !shortcutsOpen;
      return;
    }
    if (opensSettings(event)) {
      event.preventDefault();
      shortcutsOpen = false;
      if (settingsOpen) settingsOpen = false;
      else void openSettings();
      return;
    }
    // The reference, settings, and New document are modal: other shortcuts
    // wait until they close (the New document dialog handles its own Escape).
    if (newDocument) return;
    if (shortcutsOpen || settingsOpen) {
      if (event.key === "Escape") shortcutsOpen = settingsOpen = false;
      return;
    }
    if (event.ctrlKey && !event.shiftKey && doc && !rawMode && (event.key === "f" || event.key === "F")) {
      event.preventDefault();
      openSearch();
      return;
    }
    if (event.key === "F3" && search.open) {
      event.preventDefault();
      search.step(event.shiftKey ? -1 : 1);
      return;
    }
    if (event.ctrlKey && !event.shiftKey && (event.key === "o" || event.key === "O")) {
      event.preventDefault();
      void pickAndOpen();
      return;
    }
    if (event.ctrlKey && !event.shiftKey && (event.key === "n" || event.key === "N")) {
      event.preventDefault();
      void openNewDocument();
      return;
    }
    if (event.ctrlKey && (event.key === "s" || event.key === "S")) {
      event.preventDefault();
      // Commit a property or block being edited first: both commit on blur,
      // and `save` waits for the edit to land. Raw mode sends its text
      // instead, and keeps the cursor where it was.
      if (!rawMode) (document.activeElement as HTMLElement | null)?.blur();
      void tick()
        .then(() => (rawMode ? syncRaw() : undefined))
        .then(() => pendingEdits)
        .then(() => (dirty ? save() : undefined));
      return;
    }
    if (event.ctrlKey && doc && (event.key === "e" || event.key === "E")) {
      event.preventDefault();
      void (rawMode ? exitRawMode() : enterRawMode());
      return;
    }
    // Document undo/redo, except in text fields and editors, which have
    // their own undo for what's being typed.
    const target = event.target as HTMLElement | null;
    const inEditor = target?.closest("input, textarea, [contenteditable='true'], .cm-editor") != null;
    if (event.ctrlKey && doc && !inEditor) {
      const key = event.key.toLowerCase();
      if (key === "z" && !event.shiftKey) {
        event.preventDefault();
        void stepEdits(false);
        return;
      }
      if (key === "y" || (key === "z" && event.shiftKey)) {
        event.preventDefault();
        void stepEdits(true);
        return;
      }
    }
    const panel = togglesPanel(event);
    if (panel) {
      event.preventDefault();
      togglePanel(panel);
      return;
    }
    if (event.ctrlKey && doc && (event.key === "=" || event.key === "+")) {
      event.preventDefault();
      typography.zoomIn();
    } else if (event.ctrlKey && doc && (event.key === "-" || event.key === "_")) {
      event.preventDefault();
      typography.zoomOut();
    } else if (event.ctrlKey && doc && event.key === "0") {
      event.preventDefault();
      typography.resetZoom();
    } else if (event.ctrlKey && (event.key === "t" || event.key === "T")) {
      event.preventDefault();
      void openSwitcher();
    } else if (event.altKey && event.key === "ArrowLeft") {
      event.preventDefault();
      void goBack();
    } else if (event.altKey && event.key === "ArrowRight") {
      event.preventDefault();
      void goForward();
    }
  }

  /** Mouse side buttons: 3 is Back, 4 is Forward. */
  function onMouseUp(event: MouseEvent): void {
    if (event.button === 3) {
      event.preventDefault();
      void goBack();
    } else if (event.button === 4) {
      event.preventDefault();
      void goForward();
    }
  }

  function selectTocEntry(index: number): void {
    const entry = doc?.toc[index];
    if (!entry || !doc) return;
    if (rawMode) {
      const bom = doc.hasBom ? 3 : 0;
      void rawEditor?.scrollToByte(entry.offset - bom);
      return;
    }
    void reader?.scrollToHeading(entry);
  }
</script>

<svelte:window onkeydown={onKeydown} onmouseup={onMouseUp} />

{#snippet sidebarTabs()}
  <SidebarTabs tab={shown.left ?? "contents"} ontab={(tab) => setSidebars(chooseTab(tab))} />
{/snippet}

<div class="sk-app" class:with-left={shown.left !== null && doc !== null} class:with-right={showRight && doc !== null}>
  {#if doc}
    {#if shown.left === "contents"}
      <TocSidebar
        toc={doc.toc}
        current={currentEntry}
        onselect={selectTocEntry}
        {numbering}
        depth={contentsDepth}
        ondepth={(depth) => changeSetting("the contents depth", () => setContentsDepth(depth))}
        tabs={propertiesPlacement === "left" ? sidebarTabs : undefined}
      />
    {:else if shown.left === "properties"}
      <PropertiesSidebar
        side="left"
        frontMatter={doc.frontMatter}
        editor={sidebarEditor}
        onlink={onLink}
        onraw={(offset) => void enterRawMode(offset)}
        onplacement={placeProperties}
        tabs={sidebarTabs}
      />
    {/if}
    <main class="sk-main" class:sk-beside-editor={themeEditing !== null} bind:this={mainElement}>
      {#if search.open && !rawMode}
        <SearchBar
          bind:this={searchBar}
          query={search.query}
          matchCase={search.matchCase}
          wholeWord={search.wholeWord}
          label={search.label}
          hasMatches={(search.results?.total ?? 0) > 0}
          onchange={(query, matchCase, wholeWord) => search.change(query, matchCase, wholeWord)}
          onnext={() => search.step(1)}
          onprevious={() => search.step(-1)}
          onclose={() => search.close()}
        />
      {/if}
      {#if conflict}
        <ConflictBanner
          state={conflict}
          comparing={comparing && !recoveryComparing}
          busy={conflictBusy}
          onkeep={keepMine}
          onload={loadDisk}
          oncompare={startCompare}
        />
      {/if}
      {#if recovered}
        <RecoveryBanner
          changes={recovered}
          comparing={recoveryComparing}
          busy={recoveryBusy}
          onrestore={() => recovered && restoreRecovery(recovered.id)}
          ondiscard={() => recovered && discardRecovery(recovered.id)}
          oncompare={() => recovered && compareRecovery(recovered.id)}
          ondone={keepComparedRecovery}
          onusefile={useFileOverRecovery}
        />
      {/if}
      {#if rawMode && raw}
        {#key raw.mount}
          <RawEditor
            bind:this={rawEditor}
            text={raw.text}
            lineEnding={doc.lineEnding}
            startByte={raw.startByte}
            resetKey={raw.resetKey}
            fontSize={typography.fontSize}
            onchange={onRawChange}
            compareWith={raw.compareWith}
          />
        {/key}
      {:else}
        <Reader
          bind:this={reader}
          {doc}
          {assetToken}
          {initialAnchor}
          onentrychange={(e) => (currentEntry = e)}
          onfirstscreen={onFirstScreen}
          onfullymounted={() => reportTiming("full_mount", performance.now() - openStarted)}
          onlink={onLink}
          onscrollsettled={rememberPosition}
          onposition={(p) => (readingPosition = p)}
          fontSize={typography.fontSize}
          fillWindow={typography.fillWindow}
          {propertiesOpen}
          propertiesAtTop={propertiesPlacement === "top"}
          onpropertiesplacement={placeProperties}
          {propertyEditor}
          {blockEditing}
          {keepPosition}
          onpropertiestoggle={togglePropertiesPanel}
          onrawat={(offset) => void enterRawMode(offset)}
        />
      {/if}
    </main>
    {#if showRight}
      <PropertiesSidebar
        side="right"
        frontMatter={doc.frontMatter}
        editor={sidebarEditor}
        onlink={onLink}
        onraw={(offset) => void enterRawMode(offset)}
        onplacement={placeProperties}
        onclose={() => togglePanel("properties")}
      />
    {/if}
    <StatusBar
      wordCount={doc.wordCount}
      section={sectionTitle}
      {notice}
      {canGoBack}
      {canGoForward}
      onback={goBack}
      onforward={goForward}
      remoteImages={doc.remoteImages}
      remoteImagesAllowed={doc.remoteImagesAllowed}
      onremoteimages={setRemote}
      {themeName}
      onthemeclick={openSwitcher}
      tocVisible={shown.left === "contents"}
      ontoggletoc={() => togglePanel("contents")}
      propertiesShown={propertiesPlacement === "right" ? showRight : null}
      ontoggleproperties={() => togglePanel("properties")}
      flavor={doc.flavor}
      flavorSource={doc.flavorSource}
      onflavor={chooseFlavor}
      typography={typography.summary}
      ontypographyclick={() => (typographyOpen = !typographyOpen)}
      {autosave}
      onsettings={openSettings}
      {progress}
      onshortcuts={() => (shortcutsOpen = true)}
    />
    {#if typographyOpen}
      <TypographyPanel
        mode={typography.mode}
        zoom={typography.zoom}
        fillWindow={typography.fillWindow}
        onmode={(mode) => typography.setMode(mode)}
        onzoomin={() => typography.zoomIn()}
        onzoomout={() => typography.zoomOut()}
        onzoomreset={() => typography.resetZoom()}
        onfillwindow={(fill) => void typography.setFillWindow(fill)}
        onmakedefault={async () => {
          await typography.makeDefault();
          showNotice(`Default text size: ${typography.summary}`);
        }}
        onclose={() => (typographyOpen = false)}
      />
    {/if}
    {#if switcherOpen && theme}
      <ThemeSwitcher
        themes={themeList}
        current={theme}
        folder={themeFolder}
        onpreview={(id) => void showTheme(id)}
        onchoose={chooseTheme}
        onmakedefault={makeDefaultTheme}
        onreset={resetTheme}
        onduplicate={duplicate}
        onimport={importTerminal}
        onedit={(id) => void editTheme(id)}
        oncontrast={themeContrast}
        highlight={importedTheme}
        onsavetodocument={saveThemeToDocument}
        oncancel={cancelSwitcher}
      />
    {/if}
    {#if themeEditing}
      <ThemeEditor
        details={themeEditing.details}
        fonts={themeEditing.fonts}
        onpreview={previewEdit}
        onsave={saveEdit}
        oncancel={cancelEdit}
      />
    {/if}
    {#if schemeChoice}
      <SchemeChooser
        path={schemeChoice.path}
        names={schemeChoice.names}
        onimport={(names) => schemeChoice && void finishImport(schemeChoice.path, names)}
        oncancel={() => (schemeChoice = null)}
      />
    {/if}
    {#if unsavedPrompt}
      <UnsavedDialog title={unsavedPrompt.title} action={unsavedPrompt.action} onchoose={unsavedPrompt.resolve} />
    {/if}
  {:else}
    <main class="sk-main">
      <StartScreen
        documents={recentList}
        themes={themeList}
        {message}
        onpick={pickAndOpen}
        onnew={openNewDocument}
        onopen={openRecent}
        onpin={pinRecent}
        onforget={forgetRecent}
        onshortcuts={() => (shortcutsOpen = true)}
        onsettings={openSettings}
      >
        {#if recoveredList.length > 0}
          <RecoveredList
            items={recoveredList}
            busy={recoveryBusy}
            onrestore={restoreFromList}
            oncopy={(item) => copyRecovered(item.id)}
            ondiscard={(item) => discardRecovery(item.id)}
          />
        {/if}
      </StartScreen>
    </main>
  {/if}
  {#if shortcutsOpen}
    <ShortcutsDialog onclose={() => (shortcutsOpen = false)} />
  {/if}
  {#if settingsOpen}
    <SettingsDialog
      themes={themeList}
      defaultTheme={defaultThemeId}
      typography={typography.defaults}
      {contentsDepth}
      {propertiesOpen}
      {propertiesPlacement}
      {autosave}
      ondefaulttheme={(id) => changeSetting("the default theme", () => setDefaultTheme(id))}
      ontypography={(defaults) => changeSetting("the text size", () => typography.applyDefaults(defaults, true))}
      oncontentsdepth={(depth) => changeSetting("the contents depth", () => setContentsDepth(depth))}
      onpropertiesopen={togglePropertiesPanel}
      onpropertiesplacement={(placement) => void placeProperties(placement)}
      onautosave={changeAutosave}
      newDocument={newDocumentSettings}
      onremembernewfolders={(remember) => void changeNewDocumentFolders(() => setRememberNewFolders(remember))}
      onforgetfolder={(path) => void changeNewDocumentFolders(() => forgetNewDocumentFolder(path))}
      onaddfolder={() => void addNewDocumentFolder()}
      onclose={() => (settingsOpen = false)}
    />
  {/if}
  {#if newDocument}
    <NewDocumentDialog
      options={newDocument}
      oncreate={createNewDocument}
      onpickfolder={pickFolder}
      ontemplatesfolder={() => void openTemplatesFolder().catch((e) => showNotice(`Could not open the templates folder: ${String(e)}`))}
      oncancel={() => (newDocument = null)}
    />
  {/if}
  {#if dropping}
    <div class="sk-drop" aria-hidden="true"><span>Drop to open</span></div>
  {/if}
</div>

<style>
  .sk-app {
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 1fr auto;
    height: 100vh;
  }

  .sk-app.with-left {
    grid-template-columns: minmax(12rem, 18rem) 1fr;
  }

  .sk-app.with-right {
    grid-template-columns: 1fr minmax(14rem, 20rem);
  }

  .sk-app.with-left.with-right {
    grid-template-columns: minmax(12rem, 18rem) 1fr minmax(14rem, 20rem);
  }

  .sk-app :global(.sk-status) {
    grid-column: 1 / -1;
  }

  /* The theme editor panel sits at the right; the document makes room. */
  .sk-main.sk-beside-editor {
    margin-right: min(23rem, 92vw);
  }

  .sk-main {
    /* The find bar floats over the top right of the reader. */
    position: relative;
    min-width: 0;
    min-height: 0;
    /* The conflict banner, when shown, sits above the reader or editor. */
    display: flex;
    flex-direction: column;
  }

  .sk-main > :global(:last-child) {
    flex: 1;
    min-height: 0;
  }

  /* Shown while files are dragged over the window. */
  .sk-drop {
    position: fixed;
    inset: 0.5rem;
    display: grid;
    place-items: center;
    border: 2px dashed var(--sk-color-accent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--sk-color-background) 80%, transparent);
    pointer-events: none;
    z-index: 100;
  }

  .sk-drop span {
    font-family: var(--sk-font-ui);
    font-size: 1.2rem;
    color: var(--sk-color-foreground);
  }
</style>

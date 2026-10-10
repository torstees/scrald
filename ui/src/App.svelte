<script lang="ts">
  import { onMount, tick } from "svelte";
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
    userThemeFolder,
    onThemesChanged,
    propertiesOpen as propertiesOpenSetting,
    setPropertiesOpen,
    reparseDocument,
    editFrontMatter,
    saveDocument,
    checkDisk,
    pickAssetsFolder,
    blockSource,
    editBlock,
    stepHistory,
  } from "./lib/commands";
  import { afterPaint } from "./lib/idle";
  import { launchMessage } from "./lib/launch";
  import type {
    DocumentModel,
    Flavor,
    OpenedDocument,
    PropertyChange,
    ResolvedTheme,
    ThemeSummary,
  } from "./lib/types";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import UnsavedDialog, { type UnsavedChoice } from "./editor/UnsavedDialog.svelte";
  import type { PropertyEditor } from "./frontmatter/properties";
  import type { BlockEditing } from "./editor/blocks";
  import { applyTheme } from "./themes/apply";
  import { refreshRichForTheme } from "./render/renderers";
  import ThemeSwitcher from "./themes/ThemeSwitcher.svelte";
  import { NavHistory } from "./nav/history";
  import { TOP_ANCHOR, type ScrollAnchor } from "./reader/anchor";
  import Reader from "./reader/Reader.svelte";
  import TocSidebar from "./toc/TocSidebar.svelte";
  import { entryForHeadingBlock } from "./toc/tree";
  import StatusBar from "./status/StatusBar.svelte";
  import { TypographyController } from "./typography/controller.svelte";
  import TypographyPanel from "./typography/TypographyPanel.svelte";

  let doc = $state<DocumentModel | null>(null);
  let assetToken = $state(0);
  let initialAnchor = $state<ScrollAnchor>(TOP_ANCHOR);
  let message = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let tocVisible = $state(true);
  let currentSection = $state(0);
  let reader: Reader | undefined = $state();
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

  const currentEntry = $derived.by(() => {
    if (!doc) return null;
    const section = doc.sections[currentSection];
    return entryForHeadingBlock(doc.toc, section?.headingBlock ?? null);
  });
  const sectionTitle = $derived(currentEntry === null ? null : (doc?.toc[currentEntry]?.text ?? null));

  // Whether the properties panel is open: a global preference (DESIGN.md §8.2).
  let propertiesOpen = $state(true);

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
    if (!dirty) return true;
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
        if (opened) await applyEdit(opened);
        else showNotice(redo ? "Nothing to redo" : "Nothing to undo");
      } catch (e) {
        showNotice(`Could not ${redo ? "redo" : "undo"}: ${String(e)}`);
      }
    });
  }

  /** Saves the document; false (with a notice) if that failed. */
  async function save(): Promise<boolean> {
    if (!doc) return false;
    await pendingEdits;
    try {
      await saveDocument();
      dirty = false;
      showNotice("Saved");
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

  /**
   * The document changed on disk. Scrald's own saves (and touches that
   * change nothing) are ignored; with unsaved edits the file isn't reloaded,
   * so nothing is lost (the full conflict banner comes with M6).
   */
  async function onDiskChanged(): Promise<void> {
    const state = await checkDisk().catch(() => "changed" as const);
    if (state === "unchanged") return;
    if (dirty) {
      showNotice(
        state === "missing"
          ? "The file was moved or deleted. Your unsaved changes are kept; saving writes it again."
          : "The file changed on disk. Your unsaved changes are kept; saving will replace the disk version.",
      );
      return;
    }
    if (state === "changed") await reload();
  }

  function togglePropertiesPanel(open: boolean): void {
    propertiesOpen = open;
    void setPropertiesOpen(open).catch(() => {});
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
    void typography.loadDefaults().then(start);
    // Live reload (DESIGN.md §9.3), unless it's our own save or there are
    // unsaved edits.
    const unlisten = onDocumentChanged((path) => {
      if (doc && path === doc.path) void onDiskChanged();
    });
    // Theme hot reload (DESIGN.md §7.1): re-apply when a user theme changes.
    const unlistenThemes = onThemesChanged(() => {
      if (theme && !switcherOpen) void showTheme(theme.id);
      if (switcherOpen) void refreshThemeList();
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
      window.removeEventListener("wheel", onWheel);
    };
  });

  async function start(): Promise<void> {
    try {
      const info = await launchInfo();
      if (info.path === null || !info.exists) {
        message = launchMessage(info);
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
      await applyOpened(opened, anchor);
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
    await showTheme(opened.theme.id, false);
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
  async function navigate(path: string, fragment: string | null = null): Promise<void> {
    if (!(await confirmLeave("opening another document"))) return;
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

  async function allowRemoteImages(): Promise<void> {
    if (!doc) return;
    try {
      await setRemoteImages(doc.path, true);
      await reparse();
    } catch (e) {
      showNotice(`Could not load remote images: ${String(e)}`);
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

  function cancelSwitcher(): void {
    closeSwitcher();
    if (theme) void showTheme(theme.id);
  }

  function showNotice(text: string): void {
    notice = text;
    window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = null), 6000);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (switcherOpen || unsavedPrompt) return;
    if (event.ctrlKey && (event.key === "s" || event.key === "S")) {
      event.preventDefault();
      // Commit a property or block being edited first: both commit on blur,
      // and `save` waits for the edit to land.
      (document.activeElement as HTMLElement | null)?.blur();
      void tick()
        .then(() => pendingEdits)
        .then(() => (dirty ? save() : undefined));
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
    } else if (event.ctrlKey && (event.key === "\\" || event.code === "Backslash")) {
      // Match the physical key too: on many non-US layouts the backslash isn't on it.
      event.preventDefault();
      tocVisible = !tocVisible;
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
    if (entry) void reader?.scrollToBlock(entry.blockId);
  }
</script>

<svelte:window onkeydown={onKeydown} onmouseup={onMouseUp} />

<div class="sk-app" class:with-toc={tocVisible && doc !== null}>
  {#if doc}
    {#if tocVisible}
      <TocSidebar toc={doc.toc} current={currentEntry} onselect={selectTocEntry} {numbering} />
    {/if}
    <main class="sk-main" bind:this={mainElement}>
      <Reader
        bind:this={reader}
        {doc}
        {assetToken}
        {initialAnchor}
        onsectionchange={(s) => (currentSection = s)}
        onfirstscreen={onFirstScreen}
        onfullymounted={() => reportTiming("full_mount", performance.now() - openStarted)}
        onlink={onLink}
        onscrollsettled={rememberPosition}
        fontSize={typography.fontSize}
        fillWindow={typography.fillWindow}
        {propertiesOpen}
        {propertyEditor}
        {blockEditing}
        {keepPosition}
        onpropertiestoggle={togglePropertiesPanel}
      />
    </main>
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
      onallowremote={allowRemoteImages}
      {themeName}
      onthemeclick={openSwitcher}
      {tocVisible}
      ontoggletoc={() => (tocVisible = !tocVisible)}
      flavor={doc.flavor}
      flavorSource={doc.flavorSource}
      onflavor={chooseFlavor}
      typography={typography.summary}
      ontypographyclick={() => (typographyOpen = !typographyOpen)}
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
        onsavetodocument={saveThemeToDocument}
        oncancel={cancelSwitcher}
      />
    {/if}
    {#if unsavedPrompt}
      <UnsavedDialog title={unsavedPrompt.title} action={unsavedPrompt.action} onchoose={unsavedPrompt.resolve} />
    {/if}
  {:else}
    <main class="sk-main sk-message">
      <h1>Scrald</h1>
      {#if message}<p>{message}</p>{/if}
    </main>
  {/if}
</div>

<style>
  .sk-app {
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 1fr auto;
    height: 100vh;
  }

  .sk-app.with-toc {
    grid-template-columns: minmax(12rem, 18rem) 1fr;
  }

  .sk-app.with-toc :global(.sk-status) {
    grid-column: 1 / -1;
  }

  .sk-main {
    min-width: 0;
    min-height: 0;
  }

  .sk-message {
    max-width: calc(var(--sk-measure) * 1ch);
    margin: 0 auto;
    padding: 3rem 1.5rem;
  }

  .sk-message p {
    color: var(--sk-color-muted);
    font-family: var(--sk-font-ui);
    font-size: 0.9rem;
    overflow-wrap: anywhere;
  }
</style>

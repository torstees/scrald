<script lang="ts">
  import { onMount } from "svelte";
  import {
    launchInfo,
    onDocumentChanged,
    openDocument,
    openExternal,
    reportTiming,
    resolveLink,
    saveReadingPosition,
    setRemoteImages,
  } from "./lib/commands";
  import { launchMessage } from "./lib/launch";
  import type { DocumentModel } from "./lib/types";
  import { NavHistory } from "./nav/history";
  import { TOP_ANCHOR, type ScrollAnchor } from "./reader/anchor";
  import Reader from "./reader/Reader.svelte";
  import TocSidebar from "./toc/TocSidebar.svelte";
  import { entryForHeadingBlock } from "./toc/tree";
  import StatusBar from "./status/StatusBar.svelte";

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

  onMount(() => {
    void start();
    // Live reload (DESIGN.md §9.3). There is no editing yet, so a change on
    // disk always reloads; M6 adds the "unsaved changes" banner.
    const unlisten = onDocumentChanged((path) => {
      if (doc && path === doc.path) void reload();
    });
    return () => void unlisten.then((stop) => stop());
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
      initialAnchor = anchor === "remembered" ? (opened.memory.anchor ?? TOP_ANCHOR) : anchor;
      assetToken = opened.assetToken;
      doc = opened.document;
      message = null;
      return true;
    } catch (e) {
      if (doc) showNotice(`Could not open ${path}: ${String(e)}`);
      else message = `Could not open ${path}: ${String(e)}`;
      return false;
    }
  }

  /**
   * Opens a document as a new history entry: at `fragment` if given,
   * otherwise where the reader left off last time.
   */
  async function navigate(path: string, fragment: string | null = null): Promise<void> {
    rememberPosition();
    pendingFragment = fragment;
    if (await load(path, fragment === null ? "remembered" : TOP_ANCHOR)) {
      history.push({ path, anchor: TOP_ANCHOR });
      syncHistory();
    }
  }

  async function goBack(): Promise<void> {
    rememberPosition();
    const entry = history.back();
    if (entry) await load(entry.path, entry.anchor);
    syncHistory();
  }

  async function goForward(): Promise<void> {
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

  async function allowRemoteImages(): Promise<void> {
    if (!doc) return;
    try {
      await setRemoteImages(doc.path, true);
      await reload();
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

  function showNotice(text: string): void {
    notice = text;
    window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = null), 6000);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.ctrlKey && event.key === "\\") {
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
      <TocSidebar toc={doc.toc} current={currentEntry} onselect={selectTocEntry} />
    {/if}
    <main class="sk-main">
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
    />
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

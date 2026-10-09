<script lang="ts">
  import { onMount } from "svelte";
  import { launchInfo, openDocument, reportTiming } from "./lib/commands";
  import { launchMessage } from "./lib/launch";
  import type { DocumentModel } from "./lib/types";
  import Reader from "./reader/Reader.svelte";
  import TocSidebar from "./toc/TocSidebar.svelte";
  import { entryForHeadingBlock } from "./toc/tree";
  import StatusBar from "./status/StatusBar.svelte";

  let doc = $state<DocumentModel | null>(null);
  let message = $state<string | null>(null);
  let tocVisible = $state(true);
  let currentSection = $state(0);
  let reader: Reader | undefined = $state();

  // When the open started, for the first-screen and full-mount timings.
  let openStarted = 0;

  const currentEntry = $derived.by(() => {
    if (!doc) return null;
    const section = doc.sections[currentSection];
    return entryForHeadingBlock(doc.toc, section?.headingBlock ?? null);
  });
  const sectionTitle = $derived(currentEntry === null ? null : (doc?.toc[currentEntry]?.text ?? null));

  onMount(async () => {
    try {
      const info = await launchInfo();
      if (info.path === null || !info.exists) {
        message = launchMessage(info);
        return;
      }
      await open(info.path);
    } catch (e) {
      message = `Could not reach the Scrald backend: ${String(e)}`;
    }
  });

  async function open(path: string): Promise<void> {
    openStarted = performance.now();
    try {
      doc = await openDocument(path);
      reportTiming("ipc_open_document", performance.now() - openStarted);
      currentSection = 0;
      message = null;
    } catch (e) {
      message = `Could not open ${path}: ${String(e)}`;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.ctrlKey && event.key === "\\") {
      event.preventDefault();
      tocVisible = !tocVisible;
    }
  }

  function selectTocEntry(index: number): void {
    const entry = doc?.toc[index];
    if (entry) void reader?.scrollToBlock(entry.blockId);
  }

  function onLink(href: string): void {
    // External and cross-document links arrive in M2 (#34).
    console.info("link not handled yet:", href);
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="sk-app" class:with-toc={tocVisible && doc !== null}>
  {#if doc}
    {#if tocVisible}
      <TocSidebar toc={doc.toc} current={currentEntry} onselect={selectTocEntry} />
    {/if}
    <main class="sk-main">
      <Reader
        bind:this={reader}
        {doc}
        onsectionchange={(s) => (currentSection = s)}
        onfirstscreen={() => reportTiming("first_screen", performance.now() - openStarted)}
        onfullymounted={() => reportTiming("full_mount", performance.now() - openStarted)}
        onlink={onLink}
      />
    </main>
    <StatusBar wordCount={doc.wordCount} section={sectionTitle} />
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

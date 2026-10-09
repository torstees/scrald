<script lang="ts">
  // The reading view (DESIGN.md §4). Blocks are grouped into heading
  // sections; each mounted section uses `content-visibility: auto` so the
  // browser skips layout and paint for off-screen sections. Sections are
  // mounted outward from the initial position, the rest in idle time.
  import { tick, untrack } from "svelte";
  import type { DocumentModel } from "../lib/types";
  import { assetUrl } from "../lib/commands";
  import { afterPaint, whenIdle, type CancelIdle } from "../lib/idle";
  import { batchByBlocks, estimateSectionHeights, mountOrder, sectionBlocks } from "./layout";
  import { blockIndexAtOffset, firstBoxBelow, fractionInto, TOP_ANCHOR, type ScrollAnchor } from "./anchor";

  interface Props {
    doc: DocumentModel;
    /** Token for this document's images in the asset protocol. */
    assetToken: number;
    initialAnchor?: ScrollAnchor;
    /** Called when the section at the top of the viewport changes. */
    onsectionchange?: (sectionId: number) => void;
    /** Called once the first screen has been painted. */
    onfirstscreen?: () => void;
    /** Called once every section is mounted. */
    onfullymounted?: () => void;
    /** Called for clicks on links that aren't in-document anchors. */
    onlink?: (href: string) => void;
  }

  let { doc, assetToken, initialAnchor = TOP_ANCHOR, onsectionchange, onfirstscreen, onfullymounted, onlink }: Props =
    $props();

  /** Blocks per idle-time mount batch: enough to finish quickly, small enough not to jank. */
  const BLOCKS_PER_BATCH = 250;

  let scroller: HTMLElement | undefined = $state();
  let mounted = $state<boolean[]>([]);
  const heights = $derived(estimateSectionHeights(doc));

  let cancelIdle: CancelIdle | null = null;
  // Bumped for every new document, so work for an old one stops.
  let generation = 0;
  let currentSection = -1;
  let scrollFrame = 0;

  // (Re)start mounting whenever the document changes.
  $effect(() => {
    const d = doc;
    const anchor = initialAnchor;
    // `untrack` keeps the mount work from subscribing this effect to
    // `mounted`, which it writes.
    untrack(() => void startMount(d, anchor));
    return () => cancelIdle?.();
  });

  async function startMount(d: DocumentModel, anchor: ScrollAnchor): Promise<void> {
    const gen = ++generation;
    cancelIdle?.();
    currentSection = -1;

    const startBlock = d.blocks[blockIndexAtOffset(d.blocks, anchor.offset)];
    const startSection = startBlock?.section ?? 0;
    const batches = batchByBlocks(mountOrder(d.sections.length, startSection), d.sections, BLOCKS_PER_BATCH);

    const flags = d.sections.map(() => false);
    for (const i of batches.shift() ?? []) flags[i] = true;
    mounted = flags;

    await tick();
    if (gen !== generation) return;
    await scrollToAnchor(anchor);
    await afterPaint();
    if (gen !== generation) return;
    onfirstscreen?.();
    updateCurrentSection();

    const step = () => {
      if (gen !== generation) return;
      const batch = batches.shift();
      if (batch === undefined) {
        cancelIdle = null;
        onfullymounted?.();
        return;
      }
      // Sections above the viewport grow as they mount; the browser's
      // scroll anchoring (overflow-anchor) keeps the visible text still.
      for (const i of batch) mounted[i] = true;
      cancelIdle = whenIdle(step);
    };
    cancelIdle = whenIdle(step);
  }

  function sectionElements(): HTMLElement[] {
    return scroller ? Array.from(scroller.querySelectorAll<HTMLElement>(":scope .sk-section")) : [];
  }

  /** Viewport-relative extent of an element. */
  function extentOf(el: Element | undefined): { top: number; height: number } {
    if (!el) return { top: 0, height: 0 };
    const r = el.getBoundingClientRect();
    return { top: r.top, height: r.height };
  }

  /** The current reading position. */
  export function captureAnchor(): ScrollAnchor {
    if (!scroller) return TOP_ANCHOR;
    const viewTop = scroller.getBoundingClientRect().top;
    const sections = sectionElements();
    const s = firstBoxBelow(sections.length, (i) => extentOf(sections[i]), viewTop);
    const blockEls = sections[s]?.querySelectorAll<HTMLElement>(":scope > .sk-block");
    const section = doc.sections[s];
    if (!blockEls || blockEls.length === 0 || !section) {
      const first = section ? doc.blocks[section.firstBlock] : undefined;
      return { offset: first?.source.start ?? 0, fraction: 0 };
    }
    const b = firstBoxBelow(blockEls.length, (i) => extentOf(blockEls[i]), viewTop);
    const { top, height } = extentOf(blockEls[b]);
    const block = doc.blocks[section.firstBlock + b];
    return { offset: block?.source.start ?? 0, fraction: fractionInto(viewTop, top, height) };
  }

  /** Scrolls so the anchored position is at the top of the viewport. */
  export async function scrollToAnchor(anchor: ScrollAnchor): Promise<void> {
    if (!scroller) return;
    const block = doc.blocks[blockIndexAtOffset(doc.blocks, anchor.offset)];
    if (!block) return;
    if (!mounted[block.section]) {
      mounted[block.section] = true;
      await tick();
    }
    const el = scroller.querySelector<HTMLElement>(`[data-block="${block.id}"]`);
    if (!el) return;
    const { top, height } = extentOf(el);
    scroller.scrollTop += top - scroller.getBoundingClientRect().top + anchor.fraction * height;
    updateCurrentSection();
  }

  /** Scrolls a block to the top of the viewport. */
  export function scrollToBlock(blockId: number): Promise<void> {
    const block = doc.blocks[blockId];
    return block ? scrollToAnchor({ offset: block.source.start, fraction: 0 }) : Promise.resolve();
  }

  function updateCurrentSection(): void {
    if (!scroller) return;
    const sections = sectionElements();
    // A little below the top edge, so a heading counts once it's in view.
    const y = scroller.getBoundingClientRect().top + 8;
    const s = firstBoxBelow(sections.length, (i) => extentOf(sections[i]), y);
    if (s !== currentSection && doc.sections[s]) {
      currentSection = s;
      onsectionchange?.(s);
    }
  }

  function onScroll(): void {
    if (scrollFrame) return;
    scrollFrame = requestAnimationFrame(() => {
      scrollFrame = 0;
      updateCurrentSection();
    });
  }

  /**
   * Svelte action: points a block's local images at the asset protocol.
   * Core emits `<img data-asset="id">` without a `src` (DESIGN.md §6.2).
   */
  function assetImages(node: HTMLElement, token: number) {
    const apply = (t: number) => {
      for (const img of node.querySelectorAll<HTMLImageElement>("img[data-asset]")) {
        img.src = assetUrl(t, Number(img.dataset.asset));
      }
    };
    apply(token);
    return { update: apply };
  }

  /** Finds the block whose rendered HTML defines `id` (headings, footnotes). */
  function blockWithId(id: string): number | null {
    const entry = doc.toc.find((t) => t.slug === id);
    if (entry) return entry.blockId;
    const needle = `id="${id}"`;
    const block = doc.blocks.find((b) => b.html.includes(needle));
    return block ? block.id : null;
  }

  async function onClick(event: MouseEvent): Promise<void> {
    const link = (event.target as Element | null)?.closest("a");
    const href = link?.getAttribute("href");
    if (!link || href === null || href === undefined) return;
    // Never let a click navigate the webview away from the app.
    event.preventDefault();
    if (href.startsWith("#")) {
      const id = decodeURIComponent(href.slice(1));
      const blockId = blockWithId(id);
      if (blockId === null) return;
      await scrollToBlock(blockId);
      scroller?.querySelector(`[id="${CSS.escape(id)}"]`)?.scrollIntoView({ block: "start" });
      return;
    }
    onlink?.(href);
  }
</script>

<!-- The click handler only intercepts clicks on links (delegation), and links are
     keyboard-activatable themselves, so the container needs no key handler. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<div class="sk-scroller" bind:this={scroller} onscroll={onScroll} onclick={onClick} role="document">
  <article class="sk-column">
    {#each doc.sections as section, i (section.id)}
      {#if mounted[i]}
        <section
          class="sk-section"
          data-section={section.id}
          style:contain-intrinsic-size={`auto ${heights[i] ?? 0}px`}
        >
          {#each sectionBlocks(doc, section) as block (block.id)}
            <!-- Block.html is sanitized by core (ammonia); it's the only HTML we insert. -->
            <div class="sk-block" data-block={block.id} use:assetImages={assetToken}>{@html block.html}</div>
          {/each}
        </section>
      {:else}
        <section class="sk-section sk-placeholder" data-section={section.id} style:height={`${heights[i] ?? 0}px`}></section>
      {/if}
    {/each}
  </article>
</div>

<style>
  .sk-scroller {
    height: 100%;
    overflow-y: auto;
    overflow-anchor: auto;
    padding: 0 1.5rem;
    box-sizing: border-box;
  }

  /* The text column: at most --sk-measure characters wide (DESIGN.md §7.4). */
  .sk-column {
    max-width: min(calc(var(--sk-measure) * 1ch), 100%);
    margin: 0 auto;
    padding: 2.5rem 0 40vh;
  }

  .sk-section {
    content-visibility: auto;
  }

  .sk-placeholder {
    content-visibility: visible;
  }
</style>
